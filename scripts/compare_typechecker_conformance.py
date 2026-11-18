#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Score Pyrefly and Pyright against the official typing conformance corpus."""

import argparse
import re
from dataclasses import dataclass
from pathlib import Path

from compare_typechecker_behavior import (
    CATEGORIES,
    pyrefly_diagnostics,
    pyright_diagnostics,
)

ERROR_MARKER = re.compile(r"# E(?::|$| )")
OPTIONAL_ERROR_MARKER = re.compile(r"# E\?(?::|$| )")
ERROR_GROUP_MARKER = re.compile(r"# E\[([^\]]+)\]")
MEMBER_FILE_PREFIXES = (
    "enums_",
    "literals_interactions",
    "narrowing_",
    "protocols_",
    "typeddicts_",
)
OVERLOAD_ANY_COMPATIBILITY_PROBES = {
    "assert_type(ret2, Any)",
    "assert_type(example5(b), Any)",
    "assert_type(ret3, Any)",
}


@dataclass(frozen=True)
class ConformanceProbe:
    file: str
    lines: tuple[int, ...]
    category: str
    expected: str
    source: str


def category_for(file: str, expected: str) -> str:
    if file.startswith("overloads_") or "overloads" in file:
        return "overloads"
    if file.startswith(MEMBER_FILE_PREFIXES):
        return "members"
    if expected == "accept":
        return "specificity"
    return "errors"


def parse_conformance_probes(root: Path) -> list[ConformanceProbe]:
    probes = []
    for path in sorted(root.glob("*.py")):
        if path.name.startswith("_"):
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        groups: dict[str, list[int]] = {}
        for line_number, line in enumerate(lines, 1):
            code = line.split("#", 1)[0].strip()
            if not code:
                continue
            for group in ERROR_GROUP_MARKER.findall(line):
                groups.setdefault(group, []).append(line_number)
            if OPTIONAL_ERROR_MARKER.search(line) or ERROR_GROUP_MARKER.search(line):
                continue
            if ERROR_MARKER.search(line):
                probes.append(
                    ConformanceProbe(
                        file=path.name,
                        lines=(line_number,),
                        category=category_for(path.name, "error"),
                        expected="error",
                        source=code,
                    )
                )
            elif "assert_type(" in line and ")" in line:
                if (
                    path.name == "overloads_evaluation.py"
                    and code in OVERLOAD_ANY_COMPATIBILITY_PROBES
                ):
                    continue
                probes.append(
                    ConformanceProbe(
                        file=path.name,
                        lines=(line_number,),
                        category=category_for(path.name, "accept"),
                        expected="accept",
                        source=code,
                    )
                )
        for group, group_lines in groups.items():
            probes.append(
                ConformanceProbe(
                    file=path.name,
                    lines=tuple(group_lines),
                    category=category_for(path.name, "error"),
                    expected="error",
                    source=f"error group {group}",
                )
            )
    return probes


def score_conformance(
    probes: list[ConformanceProbe], diagnostics: list[dict[str, object]]
) -> tuple[dict[str, dict[str, int]], list[ConformanceProbe], int]:
    diagnostic_locations = {
        (Path(str(diagnostic["file"])).name, int(diagnostic["line"]))
        for diagnostic in diagnostics
        if diagnostic.get("severity") in {"error", "warn", "warning"}
    }
    probed_locations = {
        (probe.file, line) for probe in probes for line in probe.lines
    }
    scores = {
        category: {"correct": 0, "total": 0}
        for category in CATEGORIES
    }
    failures = []
    for probe in probes:
        has_diagnostic = any(
            (probe.file, line) in diagnostic_locations for line in probe.lines
        )
        correct = has_diagnostic == (probe.expected == "error")
        scores[probe.category]["total"] += 1
        scores[probe.category]["correct"] += int(correct)
        if not correct:
            failures.append(probe)
    unexpected = len(diagnostic_locations - probed_locations)
    return scores, failures, unexpected


def render_report(
    probes: list[ConformanceProbe],
    checker_results: list[tuple[str, str, list[dict[str, object]]]],
) -> str:
    lines = [
        "# Type checker conformance comparison",
        "",
        "Sources: the checked-in official Python typing conformance tests.",
        "Required `# E` markers and error groups are error oracles; single-line",
        "`assert_type` calls without an error marker are specificity oracles.",
        "Optional `# E?` behavior is excluded.",
        "Three overload assertions requiring a printed `Any` are also excluded because",
        "the specification permits a more precise gradual result.",
        "",
        "| Checker | Version | Errors | Specificity | Members | Overloads | Total | Unexpected diagnostics |",
        "|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    failures_by_checker: dict[str, list[ConformanceProbe]] = {}
    for name, version, diagnostics in checker_results:
        scores, failures, unexpected = score_conformance(probes, diagnostics)
        failures_by_checker[name] = failures
        total_correct = sum(score["correct"] for score in scores.values())
        total = sum(score["total"] for score in scores.values())
        cells = [
            f"{scores[category]['correct']}/{scores[category]['total']}"
            for category in CATEGORIES
        ]
        lines.append(
            f"| {name} | {version} | {' | '.join(cells)} | "
            f"{total_correct}/{total} | {unexpected} |"
        )

    lines.extend(["", "## First 100 differing outcomes", ""])
    lines.append("| Category | Expected | Location | Probe | " + " | ".join(failures_by_checker) + " |")
    lines.append("|---|---|---|---|" + "---|" * len(failures_by_checker))
    emitted = 0
    for probe in probes:
        outcomes = {
            name: "failed" if probe in failures else "passed"
            for name, failures in failures_by_checker.items()
        }
        if len(set(outcomes.values())) == 1:
            continue
        location = f"{probe.file}:{','.join(map(str, probe.lines))}"
        lines.append(
            f"| {probe.category} | {probe.expected} | `{location}` | "
            f"`{probe.source}` | "
            + " | ".join(outcomes.values())
            + " |"
        )
        emitted += 1
        if emitted == 100:
            break
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pyrefly-command", nargs="+", default=["pyrefly"])
    parser.add_argument("--pyright-command", nargs="+", default=["pyright"])
    parser.add_argument(
        "--conformance-dir",
        type=Path,
        default=Path(__file__).parents[1] / "conformance" / "third_party",
    )
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    root = args.conformance_dir.resolve()
    probes = parse_conformance_probes(root)
    results = [
        (
            "Pyrefly",
            *pyrefly_diagnostics(
                args.pyrefly_command, root, "3.13", min_severity="warn"
            ),
        ),
        ("Pyright", *pyright_diagnostics(args.pyright_command, root, "3.13")),
    ]
    report = render_report(probes, results)
    if args.output:
        args.output.write_text(report)
    else:
        print(report, end="")


if __name__ == "__main__":
    main()
