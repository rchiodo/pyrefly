#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

import tempfile
import unittest
from pathlib import Path

from compare_typechecker_conformance import (
    ConformanceProbe,
    parse_conformance_probes,
    score_conformance,
    unexpected_diagnostics,
)


class TestCompareTypecheckerConformance(unittest.TestCase):
    def test_parse_probes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overloads_sample.py"
            path.write_text(
                "from typing import assert_type\n"
                "bad()  # E: required\n"
                "maybe()  # E?: optional\n"
                "left()  # E[group]\n"
                "right()  # E[group]\n"
                "assert_type(value, int)\n"
            )
            probes = parse_conformance_probes(Path(directory))
        self.assertEqual(len(probes), 4)
        self.assertEqual(probes[0].expected, "error")
        self.assertEqual(probes[1].expected, "optional")
        self.assertEqual(probes[2].expected, "accept")
        self.assertEqual(probes[3].lines, (4, 5))
        self.assertTrue(all(probe.category == "overloads" for probe in probes))

    def test_parse_stub_probes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overloads_sample.pyi"
            path.write_text("@overload  # E[group]\ndef sample(): ...  # E[group]\n")
            probes = parse_conformance_probes(Path(directory))
        self.assertEqual(len(probes), 1)
        self.assertEqual(probes[0].lines, (1, 2))

    def test_score_probes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.py"
            path.write_text(
                "from typing import assert_type\n"
                "bad()  # E: required\n"
                "assert_type(value, int)\n"
            )
            probes = parse_conformance_probes(Path(directory))
        scores, failures, unexpected = score_conformance(
            probes,
            [{"file": "sample.py", "line": 2, "severity": "error"}],
        )
        self.assertEqual(scores["errors"]["correct"], 1)
        self.assertEqual(scores["specificity"]["correct"], 1)
        self.assertEqual(failures, [])
        self.assertEqual(unexpected, 0)

    def test_warning_satisfies_required_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.py"
            path.write_text("deprecated()  # E: required\n")
            probes = parse_conformance_probes(Path(directory))
        scores, failures, unexpected = score_conformance(
            probes,
            [{"file": "sample.py", "line": 1, "severity": "warn"}],
        )
        self.assertEqual(scores["errors"]["correct"], 1)
        self.assertEqual(failures, [])
        self.assertEqual(unexpected, 0)

    def test_permitted_overload_any_results_are_excluded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overloads_evaluation.py"
            path.write_text(
                "assert_type(ret2, Any)\n"
                "assert_type(example5(b), Any)\n"
                "assert_type(ret3, Any)\n"
                "assert_type(required, int)\n"
            )
            probes = parse_conformance_probes(Path(directory))
        self.assertEqual(len(probes), 4)
        self.assertEqual(
            [probe.expected for probe in probes],
            ["optional", "optional", "optional", "accept"],
        )
        self.assertEqual(probes[-1].source, "assert_type(required, int)")

    def test_unexpected_diagnostic(self) -> None:
        scores, failures, unexpected = score_conformance(
            [],
            [{"file": "sample.py", "line": 3, "severity": "error"}],
        )
        self.assertEqual(sum(score["total"] for score in scores.values()), 0)
        self.assertEqual(failures, [])
        self.assertEqual(unexpected, 1)

    def test_optional_diagnostic_is_not_unexpected_or_scored(self) -> None:
        probes = [
            ConformanceProbe(
                file="sample.py",
                lines=(3,),
                category="errors",
                expected="optional",
                source="optional()",
            )
        ]
        scores, failures, unexpected = score_conformance(
            probes,
            [{"file": "sample.py", "line": 3, "severity": "error"}],
        )
        self.assertEqual(sum(score["total"] for score in scores.values()), 0)
        self.assertEqual(failures, [])
        self.assertEqual(unexpected, 0)

    def test_classifies_multiline_location_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.py"
            path.write_text("call(\n    bad,  # E: required\n)\n")
            probes = parse_conformance_probes(Path(directory))
        unexpected = unexpected_diagnostics(
            probes,
            [
                {
                    "file": "sample.py",
                    "line": 1,
                    "end_line": 2,
                    "severity": "error",
                }
            ],
        )
        self.assertEqual(unexpected[0][1], "multiline range reaches an oracle line")


if __name__ == "__main__":
    unittest.main()
