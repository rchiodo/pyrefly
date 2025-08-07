/*
 * TSP interaction tests for get_symbol request handler
 *
 * These tests verify the full TSP message protocol for get_symbol requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot and typeServer/getSymbol
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_symbol request requires a node position and returns symbol information
 * including declarations, types, and other metadata for the symbol at that location.
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
fn test_tsp_get_symbol_interaction_basic() {
    // Test basic symbol lookup functionality through the TSP protocol
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("symbols.py");

    let test_content = r#"# Test file for symbol tests
def my_function(param: int) -> str:
    return str(param)

class MyClass:
    def __init__(self):
        self.value = 42

x = 10
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
            // Get symbol information for function 'my_function' at line 1
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getSymbol".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 1, "character": 4 },
                            "end": { "line": 1, "character": 15 }
                        }
                    },
                    "name": null,
                    "skipUnreachableCode": false,
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
            // Symbol response for 'my_function'
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "decls": [{
                        "category": 5,
                        "flags": 0,
                        "handle": "$$MATCH_EVERYTHING$$",
                        "moduleName": {
                            "leadingDots": 0,
                            "nameParts": ["__unknown__"]
                        },
                        "name": "my_function",
                        "node": {
                            "range": {
                                "end": {"character": 15, "line": 1},
                                "start": {"character": 4, "line": 1}
                            },
                            "uri": "$$MATCH_EVERYTHING$$"
                        },
                        "uri": "$$MATCH_EVERYTHING$$"
                    }],
                    "name": "my_function",
                    "node": {
                        "range": {
                            "end": {"character": 15, "line": 1},
                            "start": {"character": 4, "line": 1}
                        },
                        "uri": "$$MATCH_EVERYTHING$$"
                    },
                    "synthesizedTypes": [{
                        "category": 1,
                        "categoryFlags": 0,
                        "decl": null,
                        "flags": 4,
                        "handle": "$$MATCH_EVERYTHING$$",
                        "moduleName": null,
                        "name": "(param: int) -> str"
                    }]
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
fn test_tsp_get_symbol_interaction_with_name() {
    // Test symbol lookup with explicit name parameter
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("named_symbols.py");

    let test_content = r#"variable_name = "test"
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
            // Get symbol with explicit name
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getSymbol".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 13 }
                        }
                    },
                    "name": "variable_name",
                    "skipUnreachableCode": false,
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
            // Symbol response with specific name
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "decls": [{
                        "category": 1,
                        "flags": 1,
                        "handle": "$$MATCH_EVERYTHING$$",
                        "moduleName": {
                            "leadingDots": 0,
                            "nameParts": ["__unknown__"]
                        },
                        "name": "variable_name",
                        "node": {
                            "range": {
                                "end": {"character": 13, "line": 0},
                                "start": {"character": 0, "line": 0}
                            },
                            "uri": "$$MATCH_EVERYTHING$$"
                        },
                        "uri": "$$MATCH_EVERYTHING$$"
                    }],
                    "name": "variable_name",
                    "node": {
                        "range": {
                            "end": {"character": 13, "line": 0},
                            "start": {"character": 0, "line": 0}
                        },
                        "uri": "$$MATCH_EVERYTHING$$"
                    },
                    "synthesizedTypes": [{
                        "category": 0,
                        "categoryFlags": 0,
                        "decl": null,
                        "flags": 8,
                        "handle": "$$MATCH_EVERYTHING$$",
                        "moduleName": null,
                        "name": "Literal['test']"
                    }]
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
