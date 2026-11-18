#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Score checker behavior and diagnostic quality against structured oracles."""

import argparse
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

from compare_typecheckers import parse_full_errors_pyrefly, parse_full_errors_pyright

MARKER = re.compile(
    r"# (benchmark|comparison): ([a-z-]+)/(accept|error)"
    r"(?: \| quality: (.*))?\s*$"
)
CATEGORIES = ("errors", "specificity", "members", "overloads")
QUALITY_DIMENSIONS = (
    "range",
    "category",
    "actual type",
    "expected type",
    "name",
    "related candidates",
    "actionable hint",
)

CATEGORY_SIGNALS = {
    "assignment": ("bad-assignment", "reportassignmenttype", "declared type"),
    "call": ("argument", "parameter", "reportargumenttype"),
    "member": ("attribute", "member", "reportattributeaccessissue"),
    "overload": ("overload", "reportcallissue"),
    "protocol": ("protocol",),
    "typed-dict": ("typeddict", "typed-dict", "key"),
    "generic": ("type variable", "bound", "constraint"),
}

HINT_SIGNALS = {
    "assignable": ("assignable", "expected"),
    "candidate-overloads": ("overload", "argument"),
    "missing-key": ("missing", "required"),
    "missing-member": ("missing", "attribute", "member", "not present"),
}


@dataclass(frozen=True)
class QualityExpectation:
    category: str | None = None
    span: str | None = None
    actual_type: str | None = None
    expected_type: str | None = None
    name: str | None = None
    related_candidates: bool = False
    hint: str | None = None


@dataclass(frozen=True)
class Probe:
    line: int
    category: str
    expected: str
    source: str
    scored: bool
    quality: QualityExpectation | None = None
    span_columns: tuple[int, int] | None = None


def parse_quality(
    raw: str | None, source: str
) -> tuple[QualityExpectation | None, tuple[int, int] | None]:
    if raw is None:
        return None, None
    fields = {}
    for item in raw.split(";"):
        key, separator, value = item.strip().partition("=")
        if not separator:
            raise ValueError(f"Invalid quality field: {item}")
        fields[key.replace("-", "_")] = value.strip()
    unknown = set(fields) - {
        "category",
        "span",
        "actual_type",
        "expected_type",
        "name",
        "related_candidates",
        "hint",
    }
    if unknown:
        raise ValueError(f"Unknown quality fields: {sorted(unknown)}")
    category = fields.get("category")
    if category is not None and category not in CATEGORY_SIGNALS:
        raise ValueError(f"Unknown diagnostic category: {category}")
    hint = fields.get("hint")
    if hint is not None and hint not in HINT_SIGNALS:
        raise ValueError(f"Unknown actionable hint: {hint}")
    span = fields.get("span")
    span_columns = None
    if span is not None:
        start = source.index(span) + 1
        span_columns = (start, start + len(span))
    related = fields.get("related_candidates", "false").lower()
    if related not in {"true", "false"}:
        raise ValueError("related-candidates must be true or false")
    return (
        QualityExpectation(
            category=category,
            span=span,
            actual_type=fields.get("actual_type"),
            expected_type=fields.get("expected_type"),
            name=fields.get("name"),
            related_candidates=related == "true",
            hint=hint,
        ),
        span_columns,
    )


def parse_probes(source: str) -> list[Probe]:
    probes = []
    for line_number, line in enumerate(source.splitlines(), 1):
        match = MARKER.search(line)
        if match:
            probe_source = line[: match.start()].strip()
            quality, span_columns = parse_quality(match.group(4), probe_source)
            probes.append(
                Probe(
                    line=line_number,
                    category=match.group(2),
                    expected=match.group(3),
                    source=probe_source,
                    scored=match.group(1) == "benchmark",
                    quality=quality,
                    span_columns=span_columns,
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


def diagnostic_text(diagnostic: dict[str, object]) -> str:
    return f"{diagnostic.get('kind', '')} {diagnostic.get('message', '')}".lower()


def contains_type(text: str, expected: str) -> bool:
    return all(
        any(option.strip().lower() in text for option in part.split("/"))
        for part in expected.split(",")
    )


def range_matches(probe: Probe, diagnostic: dict[str, object]) -> bool:
    if probe.span_columns is None:
        return False
    start_line = int(diagnostic.get("line", 0))
    end_line = int(diagnostic.get("end_line", start_line))
    if not start_line <= probe.line <= end_line:
        return False
    expected_start, expected_end = probe.span_columns
    diagnostic_start = int(diagnostic.get("col", 0)) if start_line == probe.line else 1
    diagnostic_end = (
        int(diagnostic.get("end_col", diagnostic_start))
        if end_line == probe.line
        else len(probe.source) + 1
    )
    return diagnostic_start < expected_end and expected_start < diagnostic_end


def diagnostic_quality_checks(
    probe: Probe, diagnostics: list[dict[str, object]]
) -> dict[str, bool]:
    expectation = probe.quality
    if expectation is None:
        return {}
    candidates = [
        diagnostic
        for diagnostic in diagnostics
        if diagnostic.get("severity") == "error"
        and int(diagnostic.get("line", 0))
        <= probe.line
        <= int(diagnostic.get("end_line", diagnostic.get("line", 0)))
    ]
    texts = [diagnostic_text(diagnostic) for diagnostic in candidates]
    checks: dict[str, bool] = {}
    if expectation.span is not None:
        checks["range"] = any(
            range_matches(probe, diagnostic) for diagnostic in candidates
        )
    if expectation.category is not None:
        signals = CATEGORY_SIGNALS[expectation.category]
        checks["category"] = any(
            any(signal in text for signal in signals) for text in texts
        )
    if expectation.actual_type is not None:
        checks["actual type"] = any(
            contains_type(text, expectation.actual_type) for text in texts
        )
    if expectation.expected_type is not None:
        checks["expected type"] = any(
            contains_type(text, expectation.expected_type) for text in texts
        )
    if expectation.name is not None:
        checks["name"] = any(expectation.name.lower() in text for text in texts)
    if expectation.related_candidates:
        checks["related candidates"] = any(
            bool(diagnostic.get("related")) for diagnostic in candidates
        )
    if expectation.hint is not None:
        signals = HINT_SIGNALS[expectation.hint]
        checks["actionable hint"] = any(
            any(signal in text for signal in signals) for text in texts
        )
    return checks


def score_diagnostic_quality(
    probes: list[Probe], diagnostics: list[dict[str, object]]
) -> dict[str, dict[str, int]]:
    scores = {dimension: {"correct": 0, "total": 0} for dimension in QUALITY_DIMENSIONS}
    for probe in probes:
        for dimension, correct in diagnostic_quality_checks(probe, diagnostics).items():
            scores[dimension]["total"] += 1
            scores[dimension]["correct"] += int(correct)
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

    lines.extend(
        [
            "",
            "## Diagnostic quality",
            "",
            "Each cell scores one structured property. Message wording may vary freely.",
            "",
            "| Checker | " + " | ".join(QUALITY_DIMENSIONS) + " | Total |",
            "|---|" + "---:|" * (len(QUALITY_DIMENSIONS) + 1),
        ]
    )
    for name, version, diagnostics in checker_results:
        scores = score_diagnostic_quality(probes, diagnostics)
        total_correct = sum(score["correct"] for score in scores.values())
        total = sum(score["total"] for score in scores.values())
        cells = [
            f"{scores[dimension]['correct']}/{scores[dimension]['total']}"
            for dimension in QUALITY_DIMENSIONS
        ]
        lines.append(
            f"| {name} {version} | {' | '.join(cells)} | {total_correct}/{total} |"
        )

    lines.extend(["", "### Missing diagnostic qualities", ""])
    lines.append(
        "| Probe | "
        + " | ".join(name for name, _version, _diagnostics in checker_results)
        + " |"
    )
    lines.append("|---|" + "---|" * len(checker_results))
    for probe in probes:
        if probe.quality is None:
            continue
        missing = {
            name: [
                dimension
                for dimension, correct in diagnostic_quality_checks(
                    probe, diagnostics
                ).items()
                if not correct
            ]
            for name, _version, diagnostics in checker_results
        }
        if not any(missing.values()):
            continue
        lines.append(
            f"| `{probe.source}` | "
            + " | ".join(", ".join(missing[name]) or "-" for name in missing)
            + " |"
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
    lines.append(
        "| Category | Expected | Probe | " + " | ".join(diagnostics_by_checker) + " |"
    )
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
            f"`{probe.source}` | " + " | ".join(outcomes.values()) + " |"
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
