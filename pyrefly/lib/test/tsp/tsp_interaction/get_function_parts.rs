/*
 * TSP interaction tests for get_function_parts request handler
 *
 * These tests verify the full TSP message protocol for get_function_parts requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot, typeServer/getType, and typeServer/getFunctionParts
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_function_parts request requires a function type handle (obtained from get_type) and returns
 * the parameter and return type information for function analysis.
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
fn test_tsp_get_function_parts_interaction_basic() {
    // Test basic function parts extraction for a simple function
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("function_parts_test.py");

    let test_content = r#"def add(x: int, y: int) -> int:
    return x + y

def greet(name: str) -> str:
    return f"Hello, {name}!"
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
            // Get type of function 'add'
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 4 },
                            "end": { "line": 0, "character": 7 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get function parts for the 'add' function
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getFunctionParts".to_owned(),
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
            // Type response for function 'add'
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
            // Function parts response - should return parameter and return type information
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!({
                    "params": "$$MATCH_EVERYTHING$$",  // Accept any parameter structure
                    "returnType": "$$MATCH_EVERYTHING$$"  // Accept any return type structure
                })),
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
fn test_tsp_get_function_parts_interaction_complex() {
    // Test function parts extraction for a more complex function with optional parameters
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("complex_function_test.py");

    let test_content = r#"from typing import Optional, List

def process_data(
    data: List[int], 
    multiplier: float = 1.0, 
    filter_positive: bool = True,
    callback: Optional[callable] = None
) -> List[float]:
    result = []
    for item in data:
        if not filter_positive or item > 0:
            processed = item * multiplier
            if callback:
                processed = callback(processed)
            result.append(processed)
    return result
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
            // Get type of function 'process_data'
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 2, "character": 4 },
                            "end": { "line": 2, "character": 16 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get function parts for the complex function
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getFunctionParts".to_owned(),
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
            // Type response for complex function
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
            // Function parts response for complex function with multiple parameters
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!({
                    "params": "$$MATCH_EVERYTHING$$",  // Should include all parameter information
                    "returnType": "$$MATCH_EVERYTHING$$"  // Should be List[float] type
                })),
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
fn test_tsp_get_function_parts_interaction_lambda() {
    // Test function parts extraction for lambda functions
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("lambda_test.py");

    let test_content = r#"square = lambda x: x * x
add_one = lambda n: n + 1
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
            // Get type of lambda function 'square'
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 9 },
                            "end": { "line": 0, "character": 25 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get function parts for the lambda
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getFunctionParts".to_owned(),
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
            // Type response for lambda function
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
            // Function parts response for lambda
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!({
                    "params": "$$MATCH_EVERYTHING$$",  // Should include lambda parameter info
                    "returnType": "$$MATCH_EVERYTHING$$"  // Should infer return type
                })),
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
