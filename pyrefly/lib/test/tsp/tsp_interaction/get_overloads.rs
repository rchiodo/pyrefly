/*
 * TSP interaction tests for get_overloads request handler
 *
 * These tests verify the full TSP message protocol for get_overloads requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot, typeServer/getType, and typeServer/getOverloads
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_overloads request requires a type handle (obtained from get_type) and returns
 * all available overload signatures for a callable type.
 */

use lsp_server::Message;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::Url;
use tempfile::TempDir;

use crate::test::lsp::lsp_interaction::util::build_did_open_notification;
use crate::test::tsp::tsp_interaction::util::TspTestCase;
use crate::test::tsp::tsp_interaction::util::run_test_tsp_with_capture;

#[test]
fn test_tsp_get_overloads_interaction_basic() {
    // Test getting overloads for a function with multiple signatures
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("function_overloads_test.py");

    let test_content = r#"def simple_func(x: int) -> str:
    return str(x)

y = simple_func(42)
"#;

    std::fs::write(&test_file_path, test_content).unwrap();
    let file_uri = Url::from_file_path(&test_file_path).unwrap();

    run_test_tsp_with_capture(TspTestCase {
        messages_from_language_client: vec![
            // Open the test file
            Message::from(build_did_open_notification(test_file_path.clone())),
            // Get snapshot
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // Get type of 'simple_func' function to get a type handle
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 3, "character": 4 },
                            "end": { "line": 3, "character": 15 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get overloads using the type handle from getType
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getOverloads".to_owned(),
                params: serde_json::json!({
                    "type": {
                        "category": "$$TYPE_CATEGORY$$",
                        "categoryFlags": "$$TYPE_CATEGORY_FLAGS$$",
                        "decl": "$$TYPE_DECL$$",
                        "flags": "$$TYPE_FLAGS$$",
                        "handle": "$$TYPE_HANDLE$$",
                        "moduleName": "$$TYPE_MODULE_NAME$$",
                        "name": "$$TYPE_NAME$$"
                    },
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
            // Type response for 'simple_func' function - capture handle
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": "$$CAPTURE_TYPE_CATEGORY$$",
                    "categoryFlags": "$$CAPTURE_TYPE_CATEGORY_FLAGS$$",
                    "decl": "$$CAPTURE_TYPE_DECL$$",
                    "flags": "$$CAPTURE_TYPE_FLAGS$$",
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",  // Capture this handle for next request
                    "moduleName": "$$CAPTURE_TYPE_MODULE_NAME$$",
                    "name": "$$CAPTURE_TYPE_NAME$$"
                })),
                error: None,
            }),
            // Overloads response - for a non-overloaded function, this returns null
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::Value::Null),
                error: None,
            }),
        ],
    });
}
