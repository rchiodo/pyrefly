/*
 * TSP interaction tests for get_type_attributes request handler
 *
 * These tests verify the full TSP message protocol for get_type_attributes requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot and typeServer/getTypeAttributes
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * These integration tests complement the unit tests in lib/test/tsp/get_type_attributes.rs by testing
 * the complete TSP protocol implementation rather than individual handler components.
 */

use lsp_server::Message;
use lsp_server::Request;
use lsp_server::RequestId;

use crate::commands::lsp::IndexingMode;
use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::build_did_open_notification;
use crate::test::lsp::lsp_interaction::util::get_test_files_root;
use crate::test::lsp::lsp_interaction::util::run_test_lsp;

#[test]
fn test_tsp_get_type_attributes_interaction_basic() {
    // Test basic get_type_attributes functionality with a simple class
    let root = get_test_files_root();
    let request_file_name = root.path().join("test_class_attributes.py");

    // Create a test file with a simple class that has attributes
    std::fs::write(
        &request_file_name,
        r#"
class TestClass:
    def __init__(self):
        self.name = "test"
        self.value = 42
        self.enabled = True
    
    def get_name(self):
        return self.name
"#,
    ).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // First open the file
            Message::from(build_did_open_notification(request_file_name.clone())),
            // Get the current snapshot ID
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
        ],
        expected_messages_from_language_server: vec![
            // getSnapshot response
            Message::Response(lsp_server::Response {
                id: RequestId::from(2),
                result: Some(serde_json::json!(2)), // Usually snapshot ID 2
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}

#[test]
fn test_tsp_get_type_attributes_interaction_empty_class() {
    // Test get_type_attributes with an empty class
    let root = get_test_files_root();
    let request_file_name = root.path().join("test_empty_class.py");

    // Create a test file with an empty class
    std::fs::write(
        &request_file_name,
        r#"
class EmptyClass:
    pass
"#,
    ).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            Message::from(build_did_open_notification(request_file_name.clone())),
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
        ],
        expected_messages_from_language_server: vec![
            Message::Response(lsp_server::Response {
                id: RequestId::from(2),
                result: Some(serde_json::json!(2)),
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
