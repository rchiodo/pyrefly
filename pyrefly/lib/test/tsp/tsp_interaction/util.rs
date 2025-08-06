/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use lsp_server::Message;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::TextDocumentItem;
use lsp_types::Url;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::Notification;
use tempfile::TempDir;

use crate::commands::lsp::IndexingMode;
// Re-use the LSP interaction test infrastructure since TSP requests
// go through the same server message handling logic
use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::run_test_lsp;

pub struct TspTestCase {
    pub messages_from_language_client: Vec<Message>,
    pub expected_messages_from_language_server: Vec<Message>,
}

pub fn get_test_files_root() -> TempDir {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file_path = temp_dir.path().join("test_file.py");
    std::fs::write(
        test_file_path,
        r#"
# Test file for TSP interaction tests
x = 42
y = "hello"
def func(a: int) -> str:
    return str(a)

class MyClass:
    def __init__(self):
        self.value = 123
"#,
    )
    .unwrap();
    temp_dir
}

pub fn build_did_open_notification(file_path: std::path::PathBuf) -> lsp_server::Notification {
    let content = std::fs::read_to_string(&file_path).unwrap();
    let uri = Url::from_file_path(file_path).unwrap();

    lsp_server::Notification {
        method: DidOpenTextDocument::METHOD.to_owned(),
        params: serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "python".to_owned(),
                version: 1,
                text: content,
            },
        })
        .unwrap(),
    }
}

pub fn run_test_tsp(test_case: TspTestCase) {
    // Convert TspTestCase to TestCase and delegate to LSP infrastructure
    // since TSP requests go through the same server
    run_test_lsp(TestCase {
        messages_from_language_client: test_case.messages_from_language_client,
        expected_messages_from_language_server: test_case.expected_messages_from_language_server,
        indexing_mode: IndexingMode::LazyBlocking, // Use blocking mode for deterministic testing
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
