/*
 * TSP interaction tests for get_definition request handler
 *
 * These tests verify the full TSP message protocol for get_definition requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot and typeServer/getDefinition
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_definition request requires a node location and returns definition information
 * for symbols, including location and context.
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
fn test_tsp_get_definition_interaction_function() {
    // Test definition lookup for a function usage
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("definition_test.py");

    let test_content = r#"def calculate_square(x: int) -> int:
    """Calculate the square of a number."""
    return x * x

def main():
    result = calculate_square(5)
    print(result)
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
            // Get definition for function call 'calculate_square' in main()
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getDefinition".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 5, "character": 13 },
                            "end": { "line": 5, "character": 29 }
                        }
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
            // Definition response - should point to the function definition
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "uri": file_uri.to_string(),
                    "range": "$$MATCH_EVERYTHING$$"  // Accept any range pointing to definition
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
fn test_tsp_get_definition_interaction_variable() {
    // Test definition lookup for variable usage
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("variable_definition_test.py");

    let test_content = r#"PI = 3.14159
GRAVITY = 9.81

def calculate_circumference(radius: float) -> float:
    return 2 * PI * radius

def calculate_fall_time(height: float) -> float:
    import math
    return math.sqrt(2 * height / GRAVITY)
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
            // Get definition for PI variable usage in calculate_circumference
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getDefinition".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 4, "character": 15 },
                            "end": { "line": 4, "character": 17 }
                        }
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
            // Definition response for PI variable
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "uri": file_uri.to_string(),
                    "range": "$$MATCH_EVERYTHING$$"  // Should point to PI = 3.14159 line
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
fn test_tsp_get_definition_interaction_class_method() {
    // Test definition lookup for class method usage
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("class_method_definition_test.py");

    let test_content = r#"class Calculator:
    def __init__(self):
        self.history = []
    
    def add(self, x: int, y: int) -> int:
        result = x + y
        self.history.append(f"Added {x} + {y} = {result}")
        return result
    
    def get_history(self):
        return self.history

def main():
    calc = Calculator()
    result = calc.add(10, 20)
    history = calc.get_history()
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
            // Get definition for method call 'add' in main()
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getDefinition".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 14, "character": 18 },
                            "end": { "line": 14, "character": 21 }
                        }
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
            // Definition response for method 'add'
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "uri": file_uri.to_string(),
                    "range": "$$MATCH_EVERYTHING$$"  // Should point to the add method definition
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
fn test_tsp_get_definition_interaction_import() {
    // Test definition lookup for imported symbols
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("import_definition_test.py");

    let test_content = r#"import math
from typing import List, Dict
from collections import defaultdict

def calculate_distance(x1: float, y1: float, x2: float, y2: float) -> float:
    return math.sqrt((x2 - x1)**2 + (y2 - y1)**2)

def group_items(items: List[str]) -> Dict[str, List[str]]:
    groups = defaultdict(list)
    return dict(groups)
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
            // Get definition for math.sqrt usage
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getDefinition".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 5, "character": 11 },
                            "end": { "line": 5, "character": 20 }
                        }
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
            // Definition response for math.sqrt - may point to standard library or import
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!({
                    "uri": "$$MATCH_EVERYTHING$$",  // Could be different file/module
                    "range": "$$MATCH_EVERYTHING$$"
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
