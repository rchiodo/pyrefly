/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use lsp_types::CompletionItemKind;
use lsp_types::HoverContents;
use pyrefly_python::module::TextRangeWithModule;
use ruff_text_size::TextRange;

use crate::lsp::wasm::hover::get_hover;
use crate::state::lsp::ImportFormat;
use crate::state::require::Require;
use crate::test::util::extract_cursors_for_test;
use crate::test::util::mk_multi_file_state;
use crate::test::util::mk_multi_file_state_assert_no_errors;

#[test]
fn quick_fix_inserts_resolvable_import() {
    let code = "Greeter()\n# ^";
    let (handles, state) = mk_multi_file_state(
        &[("library", "class Greeter:\n    pass\n"), ("main", code)],
        Require::Exports,
        false,
    );
    let handle = handles.get("main").unwrap();
    let position = extract_cursors_for_test(code)[0];
    let actions = state
        .transaction()
        .local_quickfix_code_actions_sorted(
            handle,
            TextRange::new(position, position),
            ImportFormat::Absolute,
            None,
        )
        .unwrap_or_default();

    assert!(
        actions.iter().any(|(_, edits)| edits
            .iter()
            .any(|(_, _, replacement)| replacement.trim() == "from library import Greeter")),
        "expected a quick fix that imports Greeter from library"
    );
}

#[test]
fn hover_exposes_symbol_signature_and_documentation() {
    let code = r#"
class Greeter:
    def greet(self, name: str) -> str:
        """Return a greeting."""
        return f"Hello {name}"

Greeter().greet("World")
#         ^
"#;
    let (handles, state) =
        mk_multi_file_state_assert_no_errors(&[("main", code)], Require::Everything);
    let handle = handles.get("main").unwrap();
    let position = extract_cursors_for_test(code)[0];
    let hover = get_hover(&state.transaction(), handle, position, true)
        .expect("expected hover information for greet");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("expected markup hover contents");
    };

    assert!(markup.value.contains("greet"));
    assert!(markup.value.contains("name: str"));
    assert!(markup.value.contains("-> str"));
    assert!(markup.value.contains("Return a greeting."));
}

#[test]
fn completion_contains_type_appropriate_member() {
    let code = r#"
message = "hello"
message.
#       ^
"#;
    let (handles, state) = mk_multi_file_state(&[("main", code)], Require::Everything, false);
    let handle = handles.get("main").unwrap();
    let position = extract_cursors_for_test(code)[0];
    let completions =
        state
            .transaction()
            .completion(handle, position, ImportFormat::Absolute, true, None);

    assert!(
        completions
            .iter()
            .any(|item| { item.label == "upper" && item.kind == Some(CompletionItemKind::METHOD) })
    );
}

#[test]
fn definition_resolves_to_the_declared_symbol() {
    let library = r#"
class Greeter:
    def greet(self, name: str) -> str:
        return name
"#;
    let main = r#"
from library import Greeter

Greeter().greet("World")
#         ^
"#;
    let (handles, state) = mk_multi_file_state_assert_no_errors(
        &[("library", library), ("main", main)],
        Require::Everything,
    );
    let handle = handles.get("main").unwrap();
    let position = extract_cursors_for_test(main)[0];
    let definitions = state
        .transaction()
        .goto_definition(handle, position)
        .expect("expected definition for greet");

    assert!(
        definitions
            .iter()
            .any(|TextRangeWithModule { module, range }| {
                &module.contents().as_str()[range.start().to_usize()..range.end().to_usize()]
                    == "greet"
            })
    );
}

#[test]
fn rename_selects_only_semantic_references() {
    let code = r#"
message = "hello"
# ^
print(message)
other_message = message
"#;
    let (handles, state) =
        mk_multi_file_state_assert_no_errors(&[("main", code)], Require::Everything);
    let handle = handles.get("main").unwrap();
    let position = extract_cursors_for_test(code)[0];
    let references = state
        .transaction()
        .find_local_references(handle, position, true);
    let selected = references
        .iter()
        .map(|range| &code[range.start().to_usize()..range.end().to_usize()])
        .collect::<Vec<_>>();

    assert_eq!(selected, vec!["message", "message", "message"]);
}
