#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from compare_typechecker_behavior import (
    QUALITY_TOPICS,
    contains_type,
    parse_corpus_probes,
    parse_probes,
    render_report,
    score_diagnostic_quality,
    score_probes,
)


class TestCompareTypecheckerBehavior(unittest.TestCase):
    def test_type_fragments_support_required_and_alternative_forms(self) -> None:
        text = "literal['bad'] is not assignable to int"
        self.assertTrue(contains_type(text, "str/Literal['bad'],int"))
        self.assertFalse(contains_type(text, "str/Literal['bad'],float"))

    def test_parse_probes(self) -> None:
        probes = parse_probes(
            "good()  # benchmark: members/accept\nbad()  # benchmark: members/error\n"
        )
        self.assertEqual(len(probes), 2)
        self.assertEqual(probes[0].line, 1)
        self.assertEqual(probes[0].file, "<memory>")
        self.assertEqual(probes[0].category, "members")
        self.assertEqual(probes[0].expected, "accept")
        self.assertEqual(probes[0].source, "good()")
        self.assertTrue(probes[0].scored)

    def test_parse_quality_oracle(self) -> None:
        probes = parse_probes(
            'takes_int("bad")  # benchmark: errors/error'
            ' | quality: category=call; span="bad"; actual-type=str;'
            " expected-type=int; name=value; hint=assignable\n"
        )
        self.assertEqual(probes[0].quality.category, "call")
        self.assertEqual(probes[0].quality.actual_type, "str")
        self.assertEqual(probes[0].span_columns, (11, 16))

    def test_quality_span_preserves_indentation_columns(self) -> None:
        probes = parse_probes(
            '    return "bad"  # benchmark: errors/error'
            ' | quality: span="bad"\n'
        )
        self.assertEqual(probes[0].source, 'return "bad"')
        self.assertEqual(probes[0].span_columns, (12, 17))

    def test_parse_directory_corpus_with_file_identity(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory) / "corpus"
            root.mkdir()
            (root / "first.py").write_text(
                'bad: int = "x"  # benchmark: errors/error\n',
                encoding="utf-8",
            )
            (root / "second.py").write_text(
                "good: int = 1  # benchmark: errors/accept\n",
                encoding="utf-8",
            )
            probes = parse_corpus_probes(root)
        self.assertEqual(
            [(probe.file, probe.line) for probe in probes],
            [("corpus/first.py", 1), ("corpus/second.py", 1)],
        )

    def test_same_line_in_different_files_does_not_cross_match(self) -> None:
        first = parse_probes(
            "bad()  # benchmark: errors/error\n", "corpus/first.py"
        )
        second = parse_probes(
            "good()  # benchmark: errors/accept\n", "corpus/second.py"
        )
        scores = score_probes(
            [*first, *second],
            [{"file": "corpus/first.py", "line": 1, "severity": "error"}],
        )
        self.assertEqual(scores["errors"], {
            "correct": 2,
            "total": 2,
            "accepts": 1,
            "errors": 1,
        })

    def test_checked_in_corpus_has_broad_quality_coverage(self) -> None:
        corpus = Path(__file__).with_name("typechecker_behavior_corpus")
        probes = parse_corpus_probes(corpus)
        quality_probes = [probe for probe in probes if probe.quality is not None]
        self.assertEqual(len(quality_probes), 64)
        self.assertEqual(
            {probe.quality.topic for probe in quality_probes},
            set(QUALITY_TOPICS),
        )

    def test_score_probes(self) -> None:
        probes = parse_probes(
            "good()  # benchmark: errors/accept\nbad()  # benchmark: errors/error\n"
        )
        scores = score_probes(
            probes,
            [{"line": 2, "severity": "error"}],
        )
        self.assertEqual(scores["errors"]["correct"], 2)
        self.assertEqual(scores["errors"]["total"], 2)

    def test_permissive_checker_does_not_get_error_credit(self) -> None:
        probes = parse_probes("bad()  # benchmark: specificity/error\n")
        scores = score_probes(probes, [])
        self.assertEqual(scores["specificity"]["correct"], 0)

    def test_comparison_probe_is_not_scored(self) -> None:
        probes = parse_probes("policy()  # comparison: specificity/accept\n")
        scores = score_probes(probes, [{"line": 1, "severity": "error"}])
        self.assertEqual(scores["specificity"]["total"], 0)
        self.assertFalse(probes[0].scored)

    def test_report_lists_only_disagreements(self) -> None:
        probes = parse_probes(
            "good()  # benchmark: overloads/accept\n"
            "bad()  # benchmark: overloads/error\n"
        )
        report = render_report(
            probes,
            [
                ("Pyrefly", "1", [{"line": 2, "severity": "error"}]),
                ("Pyright", "2", [{"line": 1, "severity": "error"}]),
            ],
        )
        self.assertIn("| Pyrefly | 1 |", report)
        self.assertIn("`good()`", report)
        self.assertIn("`bad()`", report)

    def test_score_diagnostic_quality_without_full_message_equality(self) -> None:
        probes = parse_probes(
            'takes_int("bad")  # benchmark: errors/error'
            ' | quality: category=call; span="bad"; actual-type=str;'
            " expected-type=int; name=value; hint=assignable\n"
        )
        scores = score_diagnostic_quality(
            probes,
            [
                {
                    "line": 1,
                    "col": 11,
                    "end_line": 1,
                    "end_col": 16,
                    "kind": "bad-argument-type",
                    "message": (
                        "Argument `str` is not assignable to parameter `value` "
                        "with type `int`"
                    ),
                    "severity": "error",
                    "related": [],
                }
            ],
        )
        for dimension in (
            "range",
            "category",
            "actual type",
            "expected type",
            "name",
            "actionable hint",
        ):
            self.assertEqual(scores[dimension], {"correct": 1, "total": 1})

    def test_related_candidate_location_is_scored_structurally(self) -> None:
        probes = parse_probes(
            "choose(value)  # benchmark: overloads/error"
            " | quality: related-candidates=true\n"
        )
        scores = score_diagnostic_quality(
            probes,
            [
                {
                    "line": 1,
                    "severity": "error",
                    "related": [{"file": "sample.py", "range": {}}],
                }
            ],
        )
        self.assertEqual(scores["related candidates"], {"correct": 1, "total": 1})


if __name__ == "__main__":
    unittest.main()
