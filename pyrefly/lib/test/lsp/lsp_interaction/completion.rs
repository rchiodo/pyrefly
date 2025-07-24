/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use lsp_server::Message;
use lsp_server::Notification;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::CompletionItem;
use lsp_types::CompletionItemKind;
use lsp_types::Url;
use pyrefly_python::keywords::get_keywords;

use crate::commands::lsp::IndexingMode;
use crate::config::environment::environment::PythonEnvironment;
use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::build_did_open_notification;
use crate::test::lsp::lsp_interaction::util::get_test_files_root;
use crate::test::lsp::lsp_interaction::util::run_test_lsp;

fn get_all_builtin_completions() -> Vec<CompletionItem> {
    get_keywords(
        PythonEnvironment::get_default_interpreter_env()
            .python_version
            .unwrap(),
    )
    .into_iter()
    .map(|kw| CompletionItem {
        label: (*kw).to_owned(),
        kind: Some(CompletionItemKind::KEYWORD),
        sort_text: Some("0".to_owned()),
        ..Default::default()
    })
    .collect()
}

/// Creates a completion response message sorting the completion_items.
/// completion_items is a Vec of CompletionItem to include in the response
pub fn make_and_sort_completion_result(
    request_id: i32,
    completion_items: Vec<CompletionItem>,
) -> Message {
    let mut all_items = get_all_builtin_completions();
    all_items.extend(completion_items);
    all_items.sort_by(|item1, item2| {
        item1
            .sort_text
            .cmp(&item2.sort_text)
            .then_with(|| item1.label.cmp(&item2.label))
            .then_with(|| item1.detail.cmp(&item2.detail))
    });

    let items_json: Vec<serde_json::Value> = all_items
        .into_iter()
        .map(|item| serde_json::to_value(item).unwrap())
        .collect();

    Message::Response(Response {
        id: RequestId::from(request_id),
        result: Some(serde_json::json!({
            "isIncomplete": false,
            "items": items_json,
        })),
        error: None,
    })
}

#[test]
fn test_completion() {
    let root = get_test_files_root();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(root.path().join("foo.py"))),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root.path().join("foo.py")).unwrap().to_string(),
                        "languageId": "python",
                        "version": 2
                    },
                    "contentChanges": [{
                        "range": {
                            "start": {"line": 10, "character": 0},
                            "end": {"line": 12, "character": 0}
                        },
                        "text": format!("\n{}\n", "Ba")
                    }],
                }),
            }),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root.path().join("foo.py")).unwrap().to_string()
                    },
                    "position": {
                        "line": 11,
                        "character": 1
                    }
                }),
            }),
            Message::from(Request {
                id: RequestId::from(3),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root.path().join("foo.py")).unwrap().to_string()
                    },
                    "position": {
                        "line": 11,
                        "character": 2
                    }
                }),
            }),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root.path().join("foo.py")).unwrap().to_string(),
                        "languageId": "python",
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": format!("{}\n{}", std::fs::read_to_string(root.path().join("foo.py")).unwrap(), "Sequenc")
                    }],
                }),
            }),
        ],
        expected_messages_from_language_server: vec![
            make_and_sort_completion_result(
                2,
                vec![CompletionItem {
                    label: "Bar".to_owned(),
                    detail: Some("type[Bar]".to_owned()),
                    kind: Some(CompletionItemKind::VARIABLE),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                }],
            ),
            make_and_sort_completion_result(
                3,
                vec![CompletionItem {
                    label: "Bar".to_owned(),
                    detail: Some("type[Bar]".to_owned()),
                    kind: Some(CompletionItemKind::VARIABLE),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                }],
            ),
        ],
        ..Default::default()
    });
}

#[test]
fn test_completion_with_autoimport() {
    let root = get_test_files_root();
    let root_path = root.path().join("tests_requiring_config");
    let scope_uri = Url::from_file_path(root_path.clone()).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(root_path.join("foo.py"))),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root_path.join("foo.py")).unwrap().to_string(),
                        "languageId": "python",
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": format!("{}\n{}", std::fs::read_to_string(root_path.join("foo.py")).unwrap(), "this_is_a_very_long_function_name_so_we_can")
                    }],
                }),
            }),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root_path.join("foo.py")).unwrap().to_string()
                    },
                    "position": {
                        "line": 11,
                        "character": 43
                    }
                }),
            }),
        ],
        expected_messages_from_language_server: vec![make_and_sort_completion_result(
            2,
            vec![
                CompletionItem {
                    label: "Bar".to_owned(),
                    detail: Some("type[Bar]".to_owned()),
                    kind: Some(CompletionItemKind::VARIABLE),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                },
                CompletionItem {
                    label: "this_is_a_very_long_function_name_so_we_can_deterministically_test_autoimport_with_fuzzy_search".to_owned(),
                    detail: Some("from autoimport_provider import this_is_a_very_long_function_name_so_we_can_deterministically_test_autoimport_with_fuzzy_search\n".to_owned()),
                    kind: Some(CompletionItemKind::FUNCTION),
                    sort_text: Some("3".to_owned()),
                    additional_text_edits: Some(vec![lsp_types::TextEdit {
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                            end: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                        },
                        new_text: "from autoimport_provider import this_is_a_very_long_function_name_so_we_can_deterministically_test_autoimport_with_fuzzy_search\n".to_owned(),
                    }]),
                    ..Default::default()
                },
            ],
        )],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: Some(vec![("test".to_owned(), scope_uri)]),
        ..Default::default()
    });
}

#[test]
fn test_completion_with_autoimport_in_defined_module() {
    let root = get_test_files_root();
    let root_path = root.path().join("tests_requiring_config");
    let scope_uri = Url::from_file_path(root_path.clone()).unwrap();
    let file = root_path.join("autoimport_provider.py");

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(file.clone())),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&file).unwrap().to_string(),
                        "languageId": "python",
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": format!("{}\n{}", std::fs::read_to_string(&file).unwrap(), "this_is_a_very_long_function_name_so_we_can")
                    }],
                }),
            }),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&file).unwrap().to_string()
                    },
                    "position": {
                        "line": 12,
                        "character": 95
                    }
                }),
            }),
        ],
        // This response should contain no textedits because it's defined locally in the module
        expected_messages_from_language_server: vec![make_and_sort_completion_result(
            2,
            vec![
                CompletionItem {
                    label: "this_is_a_very_long_function_name_so_we_can_deterministically_test_autoimport_with_fuzzy_search".to_owned(),
                    detail: Some("() -> None".to_owned()),
                    kind: Some(CompletionItemKind::FUNCTION),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                },
            ],
        )],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: Some(vec![("test".to_owned(), scope_uri)]),
        ..Default::default()
    });
}

#[test]
fn test_completion_with_autoimport_duplicates() {
    let root = get_test_files_root();
    let root_path = root.path().join("duplicate_export_test");
    let scope_uri = Url::from_file_path(root_path.clone()).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(root_path.join("foo.py"))),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(root_path.join("foo.py")).unwrap().to_string()
                    },
                    "position": {
                        "line": 5,
                        "character": 14
                    }
                }),
            }),
        ],
        expected_messages_from_language_server: vec![make_and_sort_completion_result(
            2,
            vec![
                CompletionItem {
                    label: "MutableMappingUnrelatedAfter".to_owned(),
                    detail: Some("from typing import MutableMappingUnrelatedAfter\n".to_owned()),
                    kind: Some(CompletionItemKind::CLASS),
                    sort_text: Some("3".to_owned()),
                    additional_text_edits: Some(vec![lsp_types::TextEdit {
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                            end: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                        },
                        new_text: "from typing import MutableMappingUnrelatedAfter\n".to_owned(),
                    }]),
                    ..Default::default()
                },
                CompletionItem {
                    label: "MutableMapping".to_owned(),
                    detail: Some("from typing import MutableMapping\n".to_owned()),
                    kind: Some(CompletionItemKind::CLASS),
                    sort_text: Some("3".to_owned()),
                    additional_text_edits: Some(vec![lsp_types::TextEdit {
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                            end: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                        },
                        new_text: "from typing import MutableMapping\n".to_owned(),
                    }]),
                    ..Default::default()
                },
                CompletionItem {
                    label: "MutableMappingUnrelatedBefore".to_owned(),
                    detail: Some("from typing import MutableMappingUnrelatedBefore\n".to_owned()),
                    kind: Some(CompletionItemKind::CLASS),
                    sort_text: Some("3".to_owned()),
                    additional_text_edits: Some(vec![lsp_types::TextEdit {
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                            end: lsp_types::Position {
                                line: 5,
                                character: 0,
                            },
                        },
                        new_text: "from typing import MutableMappingUnrelatedBefore\n".to_owned(),
                    }]),
                    ..Default::default()
                },
            ],
        )],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: Some(vec![("test".to_owned(), scope_uri)]),
        ..Default::default()
    });
}

#[test]
fn test_module_completion() {
    let root = get_test_files_root();
    let foo = root.path().join("tests_requiring_config").join("foo.py");

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(foo.clone())),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(foo).unwrap().to_string()
                    },
                    "position": {
                        "line": 5,
                        "character": 10
                    }
                }),
            }),
        ],
        expected_messages_from_language_server: vec![make_and_sort_completion_result(
            2,
            vec![CompletionItem {
                label: "bar".to_owned(),
                detail: Some("bar".to_owned()),
                kind: Some(CompletionItemKind::MODULE),
                sort_text: Some("0".to_owned()),
                ..Default::default()
            }],
        )],
        ..Default::default()
    });
}

// TODO: Handle relative import (via ModuleName::new_maybe_relative)
#[test]
fn test_relative_module_completion() {
    let root = get_test_files_root();
    let foo = root.path().join("relative_test").join("relative_import.py");

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(foo.clone())),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(foo).unwrap().to_string()
                    },
                    "position": {
                        "line": 5,
                        "character": 10
                    }
                }),
            }),
        ],
        expected_messages_from_language_server: vec![make_and_sort_completion_result(2, vec![])],
        ..Default::default()
    });
}

#[test]
fn test_empty_filepath_file_completion() {
    let root = get_test_files_root();
    let empty_filename = root.path().join("empty_file.py");

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(Notification {
                method: "textDocument/didOpen".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&empty_filename).unwrap().to_string(),
                        "languageId": "python",
                        "version": 1,
                        "text": String::default(),
                    }
                }),
            }),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&empty_filename).unwrap().to_string(),
                        "languageId": "python",
                        "version": 2
                    },
                    "contentChanges": [{
                        "text": format!("{}\n{}\n", std::fs::read_to_string(root.path().join("notebook.py")).unwrap(), "t")
                    }],
                }),
            }),
            Message::from(Request {
                id: RequestId::from(2),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&empty_filename).unwrap().to_string()
                    },
                    "position": {
                        "line": 9,
                        "character": 1
                    }
                }),
            }),
            Message::from(Notification {
                method: "textDocument/didChange".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&empty_filename).unwrap().to_string(),
                        "languageId": "python",
                        "version": 3
                    },
                    "contentChanges": [{
                        "text": format!("{}\n{}", std::fs::read_to_string(root.path().join("notebook.py")).unwrap(), "t")
                    }],
                }),
            }),
            Message::from(Request {
                id: RequestId::from(3),
                method: "textDocument/completion".to_owned(),
                params: serde_json::json!({
                    "textDocument": {
                        "uri": Url::from_file_path(&empty_filename).unwrap().to_string()
                    },
                    "position": {
                        "line": 10,
                        "character": 1
                    }
                }),
            }),
        ],
        expected_messages_from_language_server: vec![
            make_and_sort_completion_result(
                2,
                vec![CompletionItem {
                    label: "tear".to_owned(),
                    detail: Some("(a: int, b: int, c: str) -> int".to_owned()),
                    kind: Some(CompletionItemKind::FUNCTION),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                }],
            ),
            make_and_sort_completion_result(
                3,
                vec![CompletionItem {
                    label: "tear".to_owned(),
                    detail: Some("(a: int, b: int, c: str) -> int".to_owned()),
                    kind: Some(CompletionItemKind::FUNCTION),
                    sort_text: Some("0".to_owned()),
                    ..Default::default()
                }],
            ),
        ],
        ..Default::default()
    });
}
