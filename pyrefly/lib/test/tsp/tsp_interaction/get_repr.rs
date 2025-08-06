/*
 * TSP interaction tests for get_repr request handler
 *
 * These tests verify the full TSP message protocol for get_repr requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot, typeServer/getType, and typeServer/getRepr
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_repr request requires a type handle (obtained from get_type) and returns a string
 * representation of the type for display purposes.
 */

use lsp_server::Message;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::Url;
use tempfile::TempDir;

use crate::commands::lsp::IndexingMode;
use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::build_did_open_notification;
use crate::test::lsp::lsp_interaction::util::run_test_lsp;

#[test]
fn test_tsp_get_repr_interaction_basic() {
    // Test basic repr functionality by first getting a type, then getting its representation
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("repr_test.py");

    let test_content = r#"x = 42
y = "hello"
z = [1, 2, 3]
"#;

    std::fs::write(&test_file_path, test_content).unwrap();
    let file_uri = Url::from_file_path(&test_file_path).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // Open the test file
            Message::from(build_did_open_notification(test_file_path.clone())),
            // Get snapshot
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // First get the type of variable 'x' to obtain a type handle
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 1 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Then get the string representation of that type
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getRepr".to_owned(),
                params: serde_json::json!({
                    "type": {
                        "category": 0,
                        "categoryFlags": 0,
                        "decl": null,
                        "flags": 8,
                        "handle": "$$TYPE_HANDLE_FROM_STEP_3$$",  // Use handle from getType response
                        "moduleName": null,
                        "name": "$$MATCH_EVERYTHING$$"
                    },
                    "flags": 0,
                    "snapshot": 2
                }),
            }),
        ],
        expected_messages_from_language_server: vec![
            // Snapshot response
            Message::Response(Response {
                id: RequestId::from(2),
                result: Some(serde_json::json!(2)),
                error: None,
            }),
            // Type response for variable 'x'
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": 0,
                    "categoryFlags": 0,
                    "decl": null,
                    "flags": 8,
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",  // Capture this handle for next request
                    "moduleName": null,
                    "name": "$$MATCH_EVERYTHING$$"
                })),
                error: None,
            }),
            // Repr response - should return string representation of the type
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::Value::String("$$MATCH_EVERYTHING$$".to_owned())),  // Accept any string result
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
fn test_tsp_get_repr_interaction_with_flags() {
    // Test repr with different formatting flags
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("repr_flags_test.py");

    let test_content = r#"def func(param: int) -> str:
    return str(param)
"#;

    std::fs::write(&test_file_path, test_content).unwrap();
    let file_uri = Url::from_file_path(&test_file_path).unwrap();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // Open the test file
            Message::from(build_did_open_notification(test_file_path.clone())),
            // Get snapshot
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // Get type of function
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 4 },
                            "end": { "line": 0, "character": 8 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get repr with expand type aliases flag
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getRepr".to_owned(),
                params: serde_json::json!({
                    "type": {
                        "category": 0,
                        "categoryFlags": 0,
                        "decl": null,
                        "flags": 8,
                        "handle": "$$TYPE_HANDLE_FROM_STEP_3$$",
                        "moduleName": null,
                        "name": "$$MATCH_EVERYTHING$$"
                    },
                    "flags": 1,  // EXPAND_TYPE_ALIASES flag
                    "snapshot": 2
                }),
            }),
        ],
        expected_messages_from_language_server: vec![
            // Snapshot response
            Message::Response(Response {
                id: RequestId::from(2),
                result: Some(serde_json::json!(2)),
                error: None,
            }),
            // Type response for function
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": 0,
                    "categoryFlags": 0,
                    "decl": null,
                    "flags": 8,
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",
                    "moduleName": null,
                    "name": "$$MATCH_EVERYTHING$$"
                })),
                error: None,
            }),
            // Repr response with flags applied
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::Value::String("$$MATCH_EVERYTHING$$".to_owned())),
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
