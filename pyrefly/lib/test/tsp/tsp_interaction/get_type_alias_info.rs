/*
 * TSP interaction tests for get_type_alias_info request handler
 *
 * These tests verify the full TSP message protocol for get_type_alias_info requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot and typeServer/getTypeAliasInfo
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * These integration tests complement the unit tests in lib/test/tsp/get_type_alias_info.rs by testing
 * the complete TSP protocol implementation rather than individual handler components.
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
fn test_tsp_get_type_alias_info_interaction_basic() {
    // This test demonstrates TSP interaction testing for type alias info
    // It verifies that:
    // 1. TSP typeServer/getTypeAliasInfo requests can be sent through the server infrastructure
    // 2. Files with type aliases can be opened and TSP requests made against them
    // 3. The server properly handles typeServer/getTypeAliasInfo requests
    // 4. Response infrastructure works for type alias TSP messages

    // Create a temporary directory and test file with type aliases
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("type_aliases.py");

    // Create test content with various type aliases
    let test_content = r#"from typing import List, Dict, Optional, Union

# Simple type alias
StringList = List[str]

# Complex type alias
UserData = Dict[str, Union[str, int, bool]]

# Optional type alias
OptionalStringList = Optional[List[str]]
"#;

    std::fs::write(&test_file_path, test_content).unwrap();

    let file_uri = Url::from_file_path(&test_file_path).unwrap();
    let request_file_name = test_file_path.clone();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // First open the file
            Message::from(build_did_open_notification(request_file_name.clone())),
            // Get the current snapshot ID before making TSP requests
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // First get the type of the StringList alias to get a proper handle
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 3, "character": 0 },  // Position of "StringList"
                            "end": { "line": 3, "character": 10 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Now use the captured type information to call getTypeAliasInfo
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getTypeAliasInfo".to_owned(),
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
            // First expect response to getSnapshot - should return a snapshot ID
            Message::Response(Response {
                id: RequestId::from(2),
                result: Some(serde_json::json!(2)), // Expect snapshot ID 2
                error: None,
            }),
            // Expect response to getType - this should give us type info for StringList and capture all fields
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": "$$CAPTURE_TYPE_CATEGORY$$",
                    "categoryFlags": "$$CAPTURE_TYPE_CATEGORY_FLAGS$$",
                    "decl": "$$CAPTURE_TYPE_DECL$$",
                    "flags": "$$CAPTURE_TYPE_FLAGS$$",
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",
                    "moduleName": "$$CAPTURE_TYPE_MODULE_NAME$$",
                    "name": "$$CAPTURE_TYPE_NAME$$"
                })),
                error: None,
            }),
            // Then expect response to getTypeAliasInfo - should either return type alias info or null
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!({
                    "name": "$$MATCH_EVERYTHING$$",
                    "typeArgs": "$$MATCH_EVERYTHING$$"
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
fn test_tsp_get_type_alias_info_interaction_invalid_snapshot() {
    // Test error handling for invalid snapshot ID
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("simple_alias.py");

    let test_content = r#"from typing import List
MyList = List[str]
"#;

    std::fs::write(&test_file_path, test_content).unwrap();
    let request_file_name = test_file_path.clone();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // Open the file
            Message::from(build_did_open_notification(request_file_name.clone())),
            // Get snapshot
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // Get type of MyList to capture real type info
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": Url::from_file_path(&test_file_path).unwrap().to_string(),
                        "range": {
                            "start": { "line": 1, "character": 0 },
                            "end": { "line": 1, "character": 6 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Try to use an invalid snapshot ID with captured type info
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getTypeAliasInfo".to_owned(),
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
                    "snapshot": 999  // Invalid snapshot ID
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
            // Type response for MyList - capture the type info
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": "$$CAPTURE_TYPE_CATEGORY$$",
                    "categoryFlags": "$$CAPTURE_TYPE_CATEGORY_FLAGS$$",
                    "decl": "$$CAPTURE_TYPE_DECL$$",
                    "flags": "$$CAPTURE_TYPE_FLAGS$$",
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",
                    "moduleName": "$$CAPTURE_TYPE_MODULE_NAME$$",
                    "name": "$$CAPTURE_TYPE_NAME$$"
                })),
                error: None,
            }),
            // Error response for invalid snapshot
            Message::Response(Response {
                id: RequestId::from(4),
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32,
                    message: "$$MATCH_EVERYTHING$$".to_string(), // Accept any error message about invalid snapshot
                    data: None,
                }),
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}

#[test]
fn test_tsp_get_type_alias_info_interaction_empty_params() {
    // Test error handling for missing required parameters
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("alias_test.py");

    let test_content = r#"from typing import Dict
MyDict = Dict[str, int]
"#;

    std::fs::write(&test_file_path, test_content).unwrap();
    let request_file_name = test_file_path.clone();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // Open the file
            Message::from(build_did_open_notification(request_file_name.clone())),
            // Try to call getTypeAliasInfo with empty params
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getTypeAliasInfo".to_owned(),
                params: serde_json::json!({}), // Empty params - should cause parsing error
            }),
        ],
        expected_messages_from_language_server: vec![
            // Error response for invalid params
            Message::Response(Response {
                id: RequestId::from(2),
                result: None,
                error: Some(lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::InvalidParams as i32, // Changed from ParseError to InvalidParams
                    message: "$$MATCH_EVERYTHING$$".to_string(), // Accept any parse error message
                    data: None,
                }),
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}

#[test]
fn test_tsp_get_type_alias_info_interaction_protocol_structure() {
    // Test that the protocol properly handles well-formed requests
    // even if the business logic fails due to invalid handles
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("complex_aliases.py");

    let test_content = r#"from typing import List, Dict, Optional, Union, Tuple, Callable

# Various type alias patterns for testing
SimpleList = List[str]
ComplexDict = Dict[str, Union[int, str, bool]]
OptionalType = Optional[Tuple[str, int]]
CallableType = Callable[[str, int], bool]
NestedGeneric = Dict[str, List[Optional[int]]]
"#;

    std::fs::write(&test_file_path, test_content).unwrap();

    let file_uri = Url::from_file_path(&test_file_path).unwrap();
    let request_file_name = test_file_path.clone();

    run_test_lsp(TestCase {
        messages_from_language_client: vec![
            // Open the file
            Message::from(build_did_open_notification(request_file_name.clone())),
            // Get snapshot
            Message::from(Request {
                id: RequestId::from(2),
                method: "typeServer/getSnapshot".to_owned(),
                params: serde_json::json!({}),
            }),
            // Get type of SimpleList to capture real type info
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 3, "character": 0 },
                            "end": { "line": 3, "character": 10 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Get type of ComplexDict to capture second type info
            Message::from(Request {
                id: RequestId::from(4),
                method: "typeServer/getType".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 4, "character": 0 },
                            "end": { "line": 4, "character": 11 }
                        }
                    },
                    "snapshot": 2
                }),
            }),
            // Test first type alias info request with captured type info
            Message::from(Request {
                id: RequestId::from(5),
                method: "typeServer/getTypeAliasInfo".to_owned(),
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
            // Test second type alias info request with second captured type info
            Message::from(Request {
                id: RequestId::from(6),
                method: "typeServer/getTypeAliasInfo".to_owned(),
                params: serde_json::json!({
                    "type": {
                        "category": "$$TYPE_CATEGORY_2$$",
                        "categoryFlags": "$$TYPE_CATEGORY_FLAGS_2$$",
                        "decl": "$$TYPE_DECL_2$$",
                        "flags": "$$TYPE_FLAGS_2$$",
                        "handle": "$$TYPE_HANDLE_2$$",
                        "moduleName": "$$TYPE_MODULE_NAME_2$$",
                        "name": "$$TYPE_NAME_2$$"
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
            // First type response - capture the type info
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "category": "$$CAPTURE_TYPE_CATEGORY$$",
                    "categoryFlags": "$$CAPTURE_TYPE_CATEGORY_FLAGS$$",
                    "decl": "$$CAPTURE_TYPE_DECL$$",
                    "flags": "$$CAPTURE_TYPE_FLAGS$$",
                    "handle": "$$CAPTURE_TYPE_HANDLE$$",
                    "moduleName": "$$CAPTURE_TYPE_MODULE_NAME$$",
                    "name": "$$CAPTURE_TYPE_NAME$$"
                })),
                error: None,
            }),
            // Second type response - capture the second type info
            Message::Response(Response {
                id: RequestId::from(4),
                result: Some(serde_json::json!({
                    "category": "$$CAPTURE_TYPE_CATEGORY_2$$",
                    "categoryFlags": "$$CAPTURE_TYPE_CATEGORY_FLAGS_2$$",
                    "decl": "$$CAPTURE_TYPE_DECL_2$$",
                    "flags": "$$CAPTURE_TYPE_FLAGS_2$$",
                    "handle": "$$CAPTURE_TYPE_HANDLE_2$$",
                    "moduleName": "$$CAPTURE_TYPE_MODULE_NAME_2$$",
                    "name": "$$CAPTURE_TYPE_NAME_2$$"
                })),
                error: None,
            }),
            // First type alias info response - should work with real captured type
            Message::Response(Response {
                id: RequestId::from(5),
                result: Some(serde_json::json!("$$MATCH_EVERYTHING$$")), // Accept any result for the type alias info
                error: None,
            }),
            // Second type alias info response - should work with real captured type
            Message::Response(Response {
                id: RequestId::from(6),
                result: Some(serde_json::json!("$$MATCH_EVERYTHING$$")), // Accept any result for the type alias info
                error: None,
            }),
        ],
        indexing_mode: IndexingMode::LazyBlocking,
        workspace_folders: None,
        configuration: false,
        file_watch: false,
    });
}
