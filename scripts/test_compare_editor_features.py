#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

import unittest

from compare_editor_features import (
    FeatureResult,
    code_action_edit_texts,
    completion_labels,
    flatten_strings,
    range_for,
    render_report,
    source_position,
    workspace_edit_texts,
)


class TestCompareEditorFeatures(unittest.TestCase):
    def test_source_position_uses_occurrence(self) -> None:
        source = "value = 1\nprint(value)\n"
        self.assertEqual(source_position(source, "value", 2), {"line": 1, "character": 6})
        self.assertEqual(
            range_for(source, "value", 2),
            {
                "start": {"line": 1, "character": 6},
                "end": {"line": 1, "character": 11},
            },
        )

    def test_flatten_strings_ignores_payload_shape(self) -> None:
        payload = {"contents": {"kind": "markdown", "value": "def greet() -> str"}}
        self.assertIn("def greet() -> str", flatten_strings(payload))

    def test_completion_labels_support_list_and_completion_list(self) -> None:
        items = [{"label": "upper"}, {"label": "lower"}]
        self.assertEqual(completion_labels(items), {"upper", "lower"})
        self.assertEqual(completion_labels({"items": items}), {"upper", "lower"})

    def test_workspace_edit_texts_support_both_lsp_shapes(self) -> None:
        edit = {
            "changes": {"file:///a.py": [{"newText": "first"}]},
            "documentChanges": [{"edits": [{"newText": "second"}]}],
        }
        self.assertEqual(workspace_edit_texts(edit), ["first", "second"])
        self.assertEqual(code_action_edit_texts([{"edit": edit}]), ["first", "second"])

    def test_report_scores_semantic_results(self) -> None:
        results = [
            FeatureResult("quick fixes", "pass", "import"),
            FeatureResult("hover", "pass", "typed"),
            FeatureResult("completion", "fail", "missing"),
            FeatureResult("navigation", "pass", "target"),
            FeatureResult("rename", "pass", "edits"),
        ]
        report = render_report([("Checker", "1", results)])
        self.assertIn("| Checker | 1 | pass | pass | fail | pass | pass | 4/5 |", report)
        self.assertIn("**Completion:** fail", report)


if __name__ == "__main__":
    unittest.main()
