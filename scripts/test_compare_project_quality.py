#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

"""Unit tests for compare_project_quality.py."""

import unittest

from compare_project_quality import (
    aggregate_checker_runs,
    aggregate_project_runs,
    diagnostic_fingerprint,
    diagnostics_overlap,
)
from projects import Project


def diagnostic(
    file: str = "src/example.py",
    line: int = 10,
    message: str = "bad assignment",
) -> dict[str, object]:
    return {
        "file": file,
        "line": line,
        "col": 1,
        "end_line": line,
        "end_col": 5,
        "kind": "assignment",
        "message": message,
        "severity": "error",
    }


def successful_run(
    diagnostics: list[dict[str, object]], duration: float
) -> dict[str, object]:
    return {
        "status": "ok",
        "duration_sec": duration,
        "returncode": 1,
        "diagnostics": diagnostics,
    }


class TestAggregateCheckerRuns(unittest.TestCase):
    def test_aggregates_runtime_duplicates_and_stability(self) -> None:
        stable = diagnostic()
        unstable = diagnostic(line=20, message="only once")
        metrics = aggregate_checker_runs(
            [
                successful_run([stable, stable, unstable], 3.0),
                successful_run([stable], 1.0),
                successful_run([stable], 2.0),
            ]
        )
        self.assertEqual(metrics["runtime_sec"], {"min": 1.0, "median": 2.0, "max": 3.0})
        self.assertEqual(metrics["diagnostic_count"], {"min": 1, "max": 3})
        self.assertEqual(
            metrics["duplicate_diagnostics"], {"total": 1, "max_per_run": 1}
        )
        self.assertEqual(metrics["stable_diagnostic_count"], 1)
        self.assertEqual(metrics["unstable_diagnostic_count"], 1)

    def test_counts_timeouts_and_crashes_without_treating_them_as_empty_runs(self) -> None:
        metrics = aggregate_checker_runs(
            [
                successful_run([diagnostic()], 1.5),
                {"status": "timeout", "duration_sec": 10.0, "diagnostics": []},
                {"status": "crash", "duration_sec": 0.1, "diagnostics": []},
            ]
        )
        self.assertEqual(metrics["successful_runs"], 1)
        self.assertEqual(metrics["timeouts"], 1)
        self.assertEqual(metrics["crashes"], 1)
        self.assertEqual(metrics["stable_diagnostic_count"], 1)

    def test_fingerprint_distinguishes_message_changes(self) -> None:
        self.assertNotEqual(
            diagnostic_fingerprint(diagnostic(message="first")),
            diagnostic_fingerprint(diagnostic(message="second")),
        )


class TestCrossCheckerComparison(unittest.TestCase):
    def test_overlap_uses_file_and_line_range(self) -> None:
        left = diagnostic(line=10)
        right = diagnostic(line=10, message="different checker wording")
        self.assertTrue(diagnostics_overlap(left, right))
        self.assertFalse(diagnostics_overlap(left, diagnostic(line=11)))
        self.assertFalse(diagnostics_overlap(left, diagnostic(file="other.py")))

    def test_unmatched_diagnostics_are_candidates_not_false_positive_findings(
        self,
    ) -> None:
        project = Project(
            location="https://example.com/project",
            name_override="project",
            mypy_cmd="",
            pyright_cmd="{pyright}",
            pyrefly_cmd="{pyrefly} .",
        )
        shared_pyrefly = diagnostic(line=10, message="pyrefly wording")
        shared_pyright = diagnostic(line=10, message="pyright wording")
        pyrefly_only = diagnostic(line=20)
        result = aggregate_project_runs(
            project,
            {
                "pyrefly": [
                    successful_run([shared_pyrefly, pyrefly_only], 1.0),
                    successful_run([shared_pyrefly, pyrefly_only], 1.1),
                ],
                "pyright": [
                    successful_run([shared_pyright], 2.0),
                    successful_run([shared_pyright], 2.1),
                ],
            },
        )
        adjudication = result["semantic_adjudication"]
        self.assertEqual(len(adjudication["pyrefly_only_candidates"]), 1)
        self.assertEqual(len(adjudication["pyright_only_candidates"]), 0)
        self.assertIn("not automated false-positive", adjudication["note"])


if __name__ == "__main__":
    unittest.main()
