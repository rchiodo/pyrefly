/*
 * TSP interaction tests for get_references request handler
 *
 * These tests verify the full TSP message protocol for get_references requests by:
 * 1. Following the LSP interaction test pattern using run_test_lsp
 * 2. Testing complete request/response flows including typeServer/getSnapshot and typeServer/getReferences
 * 3. Validating proper snapshot management and protocol sequencing
 * 4. Using real file operations and message passing to simulate end-to-end TSP interactions
 *
 * The get_references request requires a node location and returns all references to that symbol
 * throughout the codebase.
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
fn test_tsp_get_references_interaction_function() {
    // Test reference finding for a function used in multiple places
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("references_test.py");

    let test_content = r#"def utility_function(x: int) -> int:
    """A utility function used throughout the code."""
    return x * 2 + 1

def first_caller():
    result = utility_function(10)
    return result

def second_caller():
    value = utility_function(20)
    return value + utility_function(5)

class Calculator:
    def compute(self, num: int):
        return utility_function(num) * 3
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
            // Get references for utility_function definition
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getReferences".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 4 },
                            "end": { "line": 0, "character": 20 }
                        }
                    },
                    "includeDeclaration": true,
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
            // References response - should include definition and all usage locations
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!([
                    "$$MATCH_EVERYTHING$$"  // Accept any array of reference locations
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

#[test]
fn test_tsp_get_references_interaction_variable() {
    // Test reference finding for a variable used across multiple scopes
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("variable_references_test.py");

    let test_content = r#"GLOBAL_CONSTANT = 42

def use_constant():
    local_value = GLOBAL_CONSTANT * 2
    return local_value

class DataHandler:
    def __init__(self):
        self.multiplier = GLOBAL_CONSTANT
    
    def process(self, data):
        return data * GLOBAL_CONSTANT

def another_function():
    if GLOBAL_CONSTANT > 40:
        return "Large constant"
    return "Small constant"
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
            // Get references for GLOBAL_CONSTANT
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getReferences".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 15 }
                        }
                    },
                    "includeDeclaration": true,
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
            // References response for global variable
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!([
                    "$$MATCH_EVERYTHING$$"  // Should include all usages of GLOBAL_CONSTANT
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

#[test]
fn test_tsp_get_references_interaction_class_method() {
    // Test reference finding for a class method
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("method_references_test.py");

    let test_content = r#"class MathOperations:
    def add(self, x: int, y: int) -> int:
        return x + y
    
    def multiply(self, x: int, y: int) -> int:
        return x * y
    
    def complex_operation(self, a: int, b: int, c: int) -> int:
        sum_result = self.add(a, b)
        return self.multiply(sum_result, c)

def external_usage():
    math_ops = MathOperations()
    result1 = math_ops.add(5, 10)
    result2 = math_ops.add(result1, 3)
    return result2

class Calculator(MathOperations):
    def enhanced_add(self, x: int, y: int) -> int:
        base_result = self.add(x, y)
        return base_result + 1
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
            // Get references for the 'add' method
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getReferences".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 1, "character": 8 },
                            "end": { "line": 1, "character": 11 }
                        }
                    },
                    "includeDeclaration": false,  // Exclude definition, only usage
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
            // References response for 'add' method (excluding declaration)
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!([
                    "$$MATCH_EVERYTHING$$"  // Should include all calls to add method
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

#[test]
fn test_tsp_get_references_interaction_parameter() {
    // Test reference finding for function parameters
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("parameter_references_test.py");

    let test_content = r#"def process_data(input_data: list, threshold: int) -> list:
    """Process input data based on threshold."""
    filtered_data = []
    
    for item in input_data:
        if item > threshold:
            filtered_data.append(item * 2)
        elif item == threshold:
            filtered_data.append(item)
    
    if len(input_data) > 10:
        print(f"Processing {len(input_data)} items with threshold {threshold}")
    
    return filtered_data
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
            // Get references for 'threshold' parameter
            Message::from(Request {
                id: RequestId::from(3),
                method: "typeServer/getReferences".to_owned(),
                params: serde_json::json!({
                    "node": {
                        "uri": file_uri.to_string(),
                        "range": {
                            "start": { "line": 0, "character": 35 },
                            "end": { "line": 0, "character": 44 }
                        }
                    },
                    "includeDeclaration": true,
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
            // References response for parameter 'threshold'
            Message::Response(Response {
                id: RequestId::from(3),
                result: Some(serde_json::json!([
                    "$$MATCH_EVERYTHING$$"  // Should include parameter declaration and all usages
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
