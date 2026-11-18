#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

import unittest

from compare_typechecker_behavior import parse_probes, render_report, score_probes


class TestCompareTypecheckerBehavior(unittest.TestCase):
    def test_parse_probes(self) -> None:
        probes = parse_probes(
            'good()  # benchmark: members/accept\n'
            'bad()  # benchmark: members/error\n'
        )
        self.assertEqual(len(probes), 2)
        self.assertEqual(probes[0].line, 1)
        self.assertEqual(probes[0].category, "members")
        self.assertEqual(probes[0].expected, "accept")
        self.assertEqual(probes[0].source, "good()")
        self.assertTrue(probes[0].scored)

    def test_score_probes(self) -> None:
        probes = parse_probes(
            'good()  # benchmark: errors/accept\n'
            'bad()  # benchmark: errors/error\n'
        )
        scores = score_probes(
            probes,
            [{"line": 2, "severity": "error"}],
        )
        self.assertEqual(scores["errors"]["correct"], 2)
        self.assertEqual(scores["errors"]["total"], 2)

    def test_permissive_checker_does_not_get_error_credit(self) -> None:
        probes = parse_probes('bad()  # benchmark: specificity/error\n')
        scores = score_probes(probes, [])
        self.assertEqual(scores["specificity"]["correct"], 0)

    def test_comparison_probe_is_not_scored(self) -> None:
        probes = parse_probes('policy()  # comparison: specificity/accept\n')
        scores = score_probes(probes, [{"line": 1, "severity": "error"}])
        self.assertEqual(scores["specificity"]["total"], 0)
        self.assertFalse(probes[0].scored)

    def test_report_lists_only_disagreements(self) -> None:
        probes = parse_probes(
            'good()  # benchmark: overloads/accept\n'
            'bad()  # benchmark: overloads/error\n'
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


if __name__ == "__main__":
    unittest.main()
