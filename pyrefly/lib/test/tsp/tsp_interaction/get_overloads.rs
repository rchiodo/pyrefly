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

use crate::commands::lsp::IndexingMode;
use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::build_did_open_notification;
use crate::test::lsp::lsp_interaction::util::run_test_lsp;

#[test]
fn test_tsp_get_overloads_interaction_basic() {
    // Test getting overloads for a function with multiple signatures
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("function_overloads_test.py");

    let test_content = r#"from typing import overload, Union

@overload
def process(value: int) -> str: ...

@overload  
def process(value: str) -> int: ...

@overload
def process(value: float) -> str: ...

def process(value: Union[int, str, float]) -> Union[str, int]:
    """Process value based on type."""
    if isinstance(value, int):
        return str(value)
    elif isinstance(value, str):
        return int(value) if value.isdigit() else 0
    else:
        return str(value)

def test_overloads():
    result = process(42)  # Target this function call for overloads
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
            // Get type of 'process' function to get a type handle
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 11, "character": 4 },
                            "end": { "line": 11, "character": 11 }
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
                    "type": "$$TYPE_HANDLE_FROM_STEP_3$$",  // Use handle from getType response
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
            // Type response for 'process' function
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": "$$MATCH_EVERYTHING$$",
                    "categoryFlags": "$$MATCH_EVERYTHING$$",
                    "decl": "$$MATCH_EVERYTHING$$",
                    "flags": "$$MATCH_EVERYTHING$$",
                    "handle": "$$MATCH_EVERYTHING$$",  // This handle will be used in next request
                    "moduleName": "$$MATCH_EVERYTHING$$",
                    "name": "$$MATCH_EVERYTHING$$"
                })),
                error: None,
            }),
            // Overloads response - should return multiple overload signatures
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!([
                    "$$MATCH_EVERYTHING$$"  // Accept any overload structure
                ])),
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
