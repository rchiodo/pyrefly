#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Compare Pyrefly and Pyright quality metrics on a bounded project corpus."""

import argparse
import json
import logging
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
import venv
from datetime import datetime, timezone
from typing import Any

from compare_typecheckers import (
    _get_package_name,
    _has_pyright_config,
    _remove_pyrefly_section,
    extract_paths_from_cmd,
    get_projects,
    parse_full_errors_pyrefly,
    parse_full_errors_pyright,
    resolve_repo_dir,
    run,
)
from projects import Project


DEFAULT_PROJECTS = ("mypy_primer", "black", "attrs")
CHECKERS = ("pyrefly", "pyright")


def diagnostic_fingerprint(diagnostic: dict[str, object]) -> tuple[object, ...]:
    """Return the exact identity used for duplicate and stability metrics."""
    return (
        str(diagnostic.get("file", "")).replace("\\", "/"),
        diagnostic.get("line", 0),
        diagnostic.get("col", 0),
        diagnostic.get("end_line", diagnostic.get("line", 0)),
        diagnostic.get("end_col", diagnostic.get("col", 0)),
        diagnostic.get("kind", ""),
        diagnostic.get("message", ""),
        diagnostic.get("severity", "error"),
    )


def _error_diagnostics(run_result: dict[str, object]) -> list[dict[str, object]]:
    diagnostics = run_result.get("diagnostics", [])
    if not isinstance(diagnostics, list):
        return []
    return [
        diagnostic
        for diagnostic in diagnostics
        if isinstance(diagnostic, dict) and diagnostic.get("severity") == "error"
    ]


def aggregate_checker_runs(
    run_results: list[dict[str, object]],
) -> dict[str, object]:
    """Aggregate runtime, duplicate, failure, and stability metrics."""
    successful_runs = [run for run in run_results if run.get("status") == "ok"]
    durations = [
        float(run["duration_sec"])
        for run in successful_runs
        if isinstance(run.get("duration_sec"), (int, float))
    ]
    diagnostic_sets = [
        {diagnostic_fingerprint(diagnostic) for diagnostic in _error_diagnostics(run)}
        for run in successful_runs
    ]
    stable_fingerprints = set.intersection(*diagnostic_sets) if diagnostic_sets else set()
    all_fingerprints = set.union(*diagnostic_sets) if diagnostic_sets else set()
    representative_diagnostics: dict[tuple[object, ...], dict[str, object]] = {}
    duplicate_counts = []
    diagnostic_counts = []
    for run in successful_runs:
        diagnostics = _error_diagnostics(run)
        fingerprints = [diagnostic_fingerprint(diagnostic) for diagnostic in diagnostics]
        duplicate_counts.append(len(fingerprints) - len(set(fingerprints)))
        diagnostic_counts.append(len(fingerprints))
        for diagnostic in diagnostics:
            representative_diagnostics.setdefault(
                diagnostic_fingerprint(diagnostic), diagnostic
            )

    status_counts = {
        status: sum(run.get("status") == status for run in run_results)
        for status in ("ok", "timeout", "crash")
    }
    return {
        "runs": len(run_results),
        "successful_runs": len(successful_runs),
        "timeouts": status_counts["timeout"],
        "crashes": status_counts["crash"],
        "runtime_sec": {
            "min": round(min(durations), 3) if durations else None,
            "median": round(statistics.median(durations), 3) if durations else None,
            "max": round(max(durations), 3) if durations else None,
        },
        "diagnostic_count": {
            "min": min(diagnostic_counts) if diagnostic_counts else None,
            "max": max(diagnostic_counts) if diagnostic_counts else None,
        },
        "duplicate_diagnostics": {
            "total": sum(duplicate_counts),
            "max_per_run": max(duplicate_counts, default=0),
        },
        "stable_diagnostic_count": len(stable_fingerprints),
        "unstable_diagnostic_count": len(all_fingerprints - stable_fingerprints),
        "stable_diagnostics": [
            representative_diagnostics[fingerprint]
            for fingerprint in sorted(stable_fingerprints, key=repr)
        ],
    }


def diagnostics_overlap(
    left: dict[str, object], right: dict[str, object]
) -> bool:
    """Return whether two diagnostics cover an overlapping line range."""
    left_file = str(left.get("file", "")).replace("\\", "/")
    right_file = str(right.get("file", "")).replace("\\", "/")
    if left_file != right_file:
        return False
    left_start = int(left.get("line", 0))
    left_end = int(left.get("end_line", left_start))
    right_start = int(right.get("line", 0))
    right_end = int(right.get("end_line", right_start))
    return left_start <= right_end and right_start <= left_end


def unmatched_diagnostics(
    diagnostics: list[dict[str, object]],
    comparison_diagnostics: list[dict[str, object]],
) -> list[dict[str, object]]:
    """Find stable diagnostics with no overlapping diagnostic from the other checker."""
    return [
        diagnostic
        for diagnostic in diagnostics
        if not any(
            diagnostics_overlap(diagnostic, comparison)
            for comparison in comparison_diagnostics
        )
    ]


def aggregate_project_runs(
    project: Project, checker_runs: dict[str, list[dict[str, object]]]
) -> dict[str, object]:
    """Aggregate both checkers and identify diagnostics needing adjudication."""
    checker_metrics = {
        checker: aggregate_checker_runs(checker_runs[checker]) for checker in CHECKERS
    }
    pyrefly_diagnostics = checker_metrics["pyrefly"]["stable_diagnostics"]
    pyright_diagnostics = checker_metrics["pyright"]["stable_diagnostics"]
    assert isinstance(pyrefly_diagnostics, list)
    assert isinstance(pyright_diagnostics, list)
    pyrefly_only = unmatched_diagnostics(pyrefly_diagnostics, pyright_diagnostics)
    pyright_only = unmatched_diagnostics(pyright_diagnostics, pyrefly_diagnostics)
    return {
        "name": project.name,
        "url": project.location,
        "checkers": checker_metrics,
        "semantic_adjudication": {
            "pyrefly_only_candidates": pyrefly_only,
            "pyright_only_candidates": pyright_only,
            "note": (
                "These are stable diagnostics without an overlapping error from the "
                "other checker. They are candidates for manual semantic adjudication, "
                "not automated false-positive classifications."
            ),
        },
        "runs": checker_runs,
    }


def _venv_executable(venv_dir: str, executable: str) -> str:
    directory = "Scripts" if os.name == "nt" else "bin"
    suffix = ".exe" if os.name == "nt" else ""
    return os.path.join(venv_dir, directory, executable + suffix)


def setup_project(project: Project, repo_dir: str, debug: bool) -> list[str]:
    """Clone a project and prepare a reusable dependency environment."""
    if not os.path.exists(os.path.join(repo_dir, ".git")):
        clone = run(
            ["git", "clone", "--depth=1", project.location, repo_dir],
            debug,
            capture_output=True,
            text=True,
        )
        if clone.returncode:
            raise RuntimeError(f"Failed to clone {project.name}: {clone.stderr}")
        if project.revision:
            checkout = run(
                ["git", "checkout", project.revision],
                debug,
                cwd=repo_dir,
                capture_output=True,
                text=True,
            )
            if checkout.returncode:
                raise RuntimeError(
                    f"Failed to check out {project.revision}: {checkout.stderr}"
                )

    venv_dir = os.path.join(repo_dir, ".venv")
    python = _venv_executable(venv_dir, "python")
    if not os.path.exists(python):
        venv.create(venv_dir, with_pip=True)
        if project.deps:
            install = run(
                [python, "-m", "pip", "install", *project.deps],
                debug,
                cwd=repo_dir,
                capture_output=True,
                text=True,
            )
            if install.returncode:
                raise RuntimeError(
                    f"Failed to install dependencies for {project.name}: "
                    f"{install.stderr}"
                )

    runtime_deps_marker = os.path.join(venv_dir, ".runtime-deps-installed")
    if not os.path.exists(runtime_deps_marker):
        install_project = run(
            [python, "-m", "pip", "install", "."],
            debug,
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        if install_project.returncode:
            raise RuntimeError(
                f"Failed to install runtime dependencies for {project.name}: "
                f"{install_project.stderr}"
            )
        package_name = _get_package_name(repo_dir)
        if package_name:
            uninstall_project = run(
                [python, "-m", "pip", "uninstall", "-y", package_name],
                debug,
                cwd=repo_dir,
                capture_output=True,
                text=True,
            )
            if uninstall_project.returncode:
                raise RuntimeError(
                    f"Failed to remove installed source package for {project.name}: "
                    f"{uninstall_project.stderr}"
                )
        with open(runtime_deps_marker, "w", encoding="utf-8"):
            pass

    site_packages = run(
        [
            python,
            "-c",
            "import json, site; print(json.dumps(site.getsitepackages()))",
        ],
        debug,
        capture_output=True,
        text=True,
    )
    if site_packages.returncode:
        raise RuntimeError(
            f"Failed to find site-packages for {project.name}: {site_packages.stderr}"
        )
    return json.loads(site_packages.stdout)


def _project_revision(repo_dir: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=repo_dir,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout.strip()


def _checker_version(command: list[str]) -> str:
    result = subprocess.run(command, capture_output=True, text=True)
    output = (result.stdout or result.stderr).strip()
    return output.splitlines()[0] if output else "unknown"


def _prepare_pyrefly_config(repo_dir: str, pyrefly_bin: str, debug: bool) -> None:
    pyrefly_toml = os.path.join(repo_dir, "pyrefly.toml")
    if os.path.exists(pyrefly_toml):
        os.remove(pyrefly_toml)
    pyproject_toml = os.path.join(repo_dir, "pyproject.toml")
    if os.path.exists(pyproject_toml):
        _remove_pyrefly_section(pyproject_toml)
    command = [pyrefly_bin, "init", "--non-interactive"]
    if _has_pyright_config(repo_dir):
        command.extend(["--migrate-from", "pyright"])
    result = run(
        command,
        debug,
        cwd=repo_dir,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(f"Failed to initialize Pyrefly: {result.stderr}")


def _project_paths(project: Project, repo_dir: str) -> list[str]:
    paths = extract_paths_from_cmd(project.pyrefly_cmd or "")
    existing = [path for path in paths if os.path.exists(os.path.join(repo_dir, path))]
    return existing or ["."]


def _run_checker(
    checker: str,
    project: Project,
    repo_dir: str,
    pyrefly_bin: str,
    site_packages: list[str],
    timeout: int,
    debug: bool,
) -> dict[str, object]:
    paths = _project_paths(project, repo_dir)
    venv_python = _venv_executable(os.path.join(repo_dir, ".venv"), "python")
    if checker == "pyrefly":
        command = [
            pyrefly_bin,
            "check",
            "--project-excludes",
            "**/.venv/**",
            "--output-format",
            "json",
            *paths,
            *[
                argument
                for site_package in site_packages
                for argument in ("--site-package-path", site_package)
            ],
        ]
    else:
        command = [
            "pyright",
            "--pythonpath",
            venv_python,
            "--outputjson",
            *paths,
        ]

    start = time.perf_counter()
    result = run(
        command,
        debug,
        cwd=repo_dir,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    duration = time.perf_counter() - start
    if result.stderr == "CHECKER_TIMEOUT":
        return {
            "status": "timeout",
            "duration_sec": round(duration, 3),
            "returncode": result.returncode,
            "diagnostics": [],
        }
    try:
        json.loads(result.stdout or "")
    except json.JSONDecodeError:
        return {
            "status": "crash",
            "duration_sec": round(duration, 3),
            "returncode": result.returncode,
            "diagnostics": [],
            "stderr": (result.stderr or "")[-4000:],
            "stdout": (result.stdout or "")[-4000:],
        }
    diagnostics = (
        parse_full_errors_pyrefly(result.stdout or "")
        if checker == "pyrefly"
        else parse_full_errors_pyright(result.stdout or "", repo_dir)
    )
    return {
        "status": "ok",
        "duration_sec": round(duration, 3),
        "returncode": result.returncode,
        "diagnostics": diagnostics,
    }


def render_markdown(report: dict[str, object]) -> str:
    """Render the automated metrics and adjudication queue."""
    lines = [
        "# Pyrefly versus Pyright real-project quality comparison",
        "",
        "## Automated metrics",
        "",
        "| Project | Checker | Runs | Median (s) | Errors min-max | Duplicates | Unstable | Timeouts | Crashes |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    projects = report["projects"]
    assert isinstance(projects, list)
    for project in projects:
        assert isinstance(project, dict)
        checkers = project["checkers"]
        assert isinstance(checkers, dict)
        for checker in CHECKERS:
            metrics = checkers[checker]
            assert isinstance(metrics, dict)
            runtime = metrics["runtime_sec"]
            diagnostic_count = metrics["diagnostic_count"]
            duplicates = metrics["duplicate_diagnostics"]
            assert isinstance(runtime, dict)
            assert isinstance(diagnostic_count, dict)
            assert isinstance(duplicates, dict)
            error_range = f"{diagnostic_count['min']}-{diagnostic_count['max']}"
            lines.append(
                f"| {project['name']} | {checker} | {metrics['successful_runs']}/{metrics['runs']} "
                f"| {runtime['median']} | {error_range} | {duplicates['total']} "
                f"| {metrics['unstable_diagnostic_count']} | {metrics['timeouts']} "
                f"| {metrics['crashes']} |"
            )

    lines.extend(
        [
            "",
            "## Diagnostics requiring semantic adjudication",
            "",
            "The counts below are stable, checker-only diagnostics with no overlapping "
            "error from the other checker. They are false-positive candidates, not "
            "automated false-positive findings.",
            "",
            "| Project | Pyrefly-only candidates | Pyright-only candidates |",
            "|---|---:|---:|",
        ]
    )
    for project in projects:
        assert isinstance(project, dict)
        adjudication = project["semantic_adjudication"]
        assert isinstance(adjudication, dict)
        lines.append(
            f"| {project['name']} | "
            f"{len(adjudication['pyrefly_only_candidates'])} | "
            f"{len(adjudication['pyright_only_candidates'])} |"
        )
    return "\n".join(lines) + "\n"


def _run_comparison(args: argparse.Namespace) -> dict[str, object]:
    projects = get_projects(args.project)
    if len(projects) != len(args.project):
        found = {project.name.lower() for project in projects}
        missing = [name for name in args.project if name.lower() not in found]
        raise ValueError(f"Unknown or unsupported projects: {', '.join(missing)}")

    project_reports = []
    revisions = {}
    for project in projects:
        logging.info("Preparing %s", project.name)
        repo_dir = resolve_repo_dir(project, args.cache_dir, None, True)
        site_packages = setup_project(project, repo_dir, args.debug)
        _prepare_pyrefly_config(repo_dir, args.pyrefly, args.debug)
        revisions[project.name] = _project_revision(repo_dir)
        checker_runs = {checker: [] for checker in CHECKERS}
        for run_index in range(args.runs):
            for checker in CHECKERS:
                logging.info(
                    "%s run %d/%d: %s",
                    project.name,
                    run_index + 1,
                    args.runs,
                    checker,
                )
                checker_runs[checker].append(
                    _run_checker(
                        checker,
                        project,
                        repo_dir,
                        args.pyrefly,
                        site_packages,
                        args.timeout,
                        args.debug,
                    )
                )
        project_reports.append(aggregate_project_runs(project, checker_runs))

    return {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "environment": {
            "platform": platform.platform(),
            "python": platform.python_version(),
            "pyrefly": _checker_version([args.pyrefly, "--version"]),
            "pyright": _checker_version(["pyright", "--version"]),
        },
        "configuration": {
            "runs": args.runs,
            "timeout_sec": args.timeout,
            "cache_dir": os.path.abspath(args.cache_dir),
            "project_revisions": revisions,
        },
        "projects": project_reports,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "-k",
        "--project",
        nargs="+",
        default=list(DEFAULT_PROJECTS),
        help="Bounded project set to compare",
    )
    parser.add_argument("--pyrefly", help="Path to the Pyrefly binary")
    parser.add_argument(
        "--cache-dir",
        default=os.path.join(tempfile.gettempdir(), "pyrefly-project-quality"),
        help="Persistent clone and virtualenv cache",
    )
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-markdown")
    parser.add_argument("--debug", action="store_true")
    args = parser.parse_args()
    if args.runs < 2:
        parser.error("--runs must be at least 2 to measure stability")
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    if not shutil.which("pyright"):
        parser.error("pyright not found on PATH; install it with: pip install pyright")
    if not args.pyrefly:
        candidate = os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "target",
            "release",
            "pyrefly.exe" if os.name == "nt" else "pyrefly",
        )
        args.pyrefly = candidate
    args.pyrefly = os.path.abspath(args.pyrefly)
    if not os.path.exists(args.pyrefly):
        parser.error(f"Pyrefly binary not found: {args.pyrefly}")

    logging.basicConfig(
        level=logging.DEBUG if args.debug else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )
    report = _run_comparison(args)
    with open(args.output_json, "w") as output:
        json.dump(report, output, indent=2)
    markdown = render_markdown(report)
    print(markdown)
    if args.output_markdown:
        with open(args.output_markdown, "w") as output:
            output.write(markdown)


if __name__ == "__main__":
    main()
