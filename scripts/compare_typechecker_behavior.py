#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Score Pyrefly and Pyright against oracle-labeled behavioral probes."""

import argparse
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

from compare_typecheckers import parse_full_errors_pyrefly, parse_full_errors_pyright

MARKER = re.compile(r"# (benchmark|comparison): ([a-z-]+)/(accept|error)\s*$")
CATEGORIES = ("errors", "specificity", "members", "overloads")


@dataclass(frozen=True)
class Probe:
    line: int
    category: str
    expected: str
    source: str
    scored: bool


def parse_probes(source: str) -> list[Probe]:
    probes = []
    for line_number, line in enumerate(source.splitlines(), 1):
        match = MARKER.search(line)
        if match:
            probes.append(
                Probe(
                    line=line_number,
                    category=match.group(2),
                    expected=match.group(3),
                    source=MARKER.sub("", line).strip(),
                    scored=match.group(1) == "benchmark",
                )
            )
    return probes


def score_probes(
    probes: list[Probe], diagnostics: list[dict[str, object]]
) -> dict[str, dict[str, int]]:
    error_lines = {
        int(diagnostic["line"])
        for diagnostic in diagnostics
        if diagnostic.get("severity") == "error"
    }
    scores = {
        category: {"correct": 0, "total": 0, "accepts": 0, "errors": 0}
        for category in CATEGORIES
    }
    for probe in probes:
        if not probe.scored:
            continue
        has_error = probe.line in error_lines
        correct = has_error == (probe.expected == "error")
        category = scores[probe.category]
        category["total"] += 1
        category["correct"] += int(correct)
        category["errors" if probe.expected == "error" else "accepts"] += int(correct)
    return scores


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, capture_output=True, text=True)


def pyrefly_diagnostics(
    command: list[str],
    corpus: Path,
    python_version: str = "3.11",
    min_severity: str = "error",
) -> tuple[str, list[dict[str, object]]]:
    version = run([*command, "--version"], corpus.parent).stdout.strip()
    result = run(
        [
            *command,
            "check",
            str(corpus),
            "--output-format=json",
            "--summary=none",
            "--progress-bar=no",
            "--preset=default",
            "--color=never",
            "--threads=1",
            "--python-version",
            python_version,
            f"--min-severity={min_severity}",
        ],
        corpus.parent,
    )
    return version, parse_full_errors_pyrefly(result.stdout)


def pyright_diagnostics(
    command: list[str], corpus: Path, python_version: str = "3.11"
) -> tuple[str, list[dict[str, object]]]:
    version = run([*command, "--version"], corpus.parent).stdout.strip()
    result = run(
        [*command, str(corpus), "--outputjson", "--pythonversion", python_version],
        corpus.parent,
    )
    return version, parse_full_errors_pyright(result.stdout, str(corpus.parent))


def render_report(
    probes: list[Probe],
    checker_results: list[tuple[str, str, list[dict[str, object]]]],
) -> str:
    lines = [
        "# Type checker behavioral comparison",
        "",
        "Each probe has an explicit accept/error oracle. Paired positive and negative",
        "probes prevent permissive `Any` behavior from scoring as precision.",
        "Compatibility-policy comparisons are reported but excluded from totals.",
        "",
        "| Checker | Version | Errors | Specificity | Members | Overloads | Total |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    for name, version, diagnostics in checker_results:
        scores = score_probes(probes, diagnostics)
        total_correct = sum(score["correct"] for score in scores.values())
        total = sum(score["total"] for score in scores.values())
        cells = [
            f"{scores[category]['correct']}/{scores[category]['total']}"
            for category in CATEGORIES
        ]
        lines.append(
            f"| {name} | {version} | {' | '.join(cells)} | {total_correct}/{total} |"
        )

    lines.extend(["", "## Differences", ""])
    diagnostics_by_checker = {
        name: {
            int(diagnostic["line"])
            for diagnostic in diagnostics
            if diagnostic.get("severity") == "error"
        }
        for name, _version, diagnostics in checker_results
    }
    lines.append("| Category | Expected | Probe | " + " | ".join(diagnostics_by_checker) + " |")
    lines.append("|---|---|---|" + "---|" * len(diagnostics_by_checker))
    for probe in probes:
        outcomes = {
            name: ("error" if probe.line in error_lines else "accepted")
            for name, error_lines in diagnostics_by_checker.items()
        }
        if len(set(outcomes.values())) == 1:
            continue
        lines.append(
            f"| {probe.category} | "
            f"{'scored' if probe.scored else 'comparison'} {probe.expected} | "
            f"`{probe.source}` | "
            + " | ".join(outcomes.values())
            + " |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pyrefly-command", nargs="+", default=["pyrefly"])
    parser.add_argument("--pyright-command", nargs="+", default=["pyright"])
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path(__file__).with_name("typechecker_behavior_cases.py"),
    )
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    corpus = args.corpus.resolve()
    source = corpus.read_text()
    probes = parse_probes(source)
    unknown_categories = {probe.category for probe in probes} - set(CATEGORIES)
    if unknown_categories:
        raise ValueError(f"Unknown probe categories: {sorted(unknown_categories)}")

    results = [
        ("Pyrefly", *pyrefly_diagnostics(args.pyrefly_command, corpus, "3.13")),
        ("Pyright", *pyright_diagnostics(args.pyright_command, corpus, "3.13")),
    ]
    report = render_report(probes, results)
    if args.output:
        args.output.write_text(report)
    else:
        print(report, end="")


if __name__ == "__main__":
    main()
