use crate::test::tsp::util::build_tsp_test_server;
use crate::test::tsp::util::extract_cursor_location;
use crate::test::util::mk_multi_file_state_assert_no_errors;
use crate::tsp::requests::get_symbol::extract_symbol_name;
use tsp_types::{GetSymbolParams, Node, Position, Range};

#[test]
fn test_get_symbol_params_construction() {
    let (_handle, uri, state) = build_tsp_test_server();
    let _transaction = state.transaction();

    let content = r#"
def my_function():
#   ^
    pass

my_function()
"#;

    let position = extract_cursor_location(content, &uri);

    let params = GetSymbolParams {
        node: Node {
            uri: uri.to_string(),
            range: Range {
                start: Position {
                    line: position.line,
                    character: position.character,
                },
                end: Position {
                    line: position.line,
                    character: position.character,
                },
            },
        },
        name: None,
        skip_unreachable_code: false,
        snapshot: 1,
    };

    // Just test that we can construct the parameters correctly
    assert_eq!(params.snapshot, 1);
    assert!(!params.skip_unreachable_code);
    assert!(params.name.is_none());
}

// Standalone function tests for the core logic

#[test]
fn test_extract_symbol_name_with_provided_name() {
    let (_handles, state) = mk_multi_file_state_assert_no_errors(&[(
        "test.py",
        r#"def my_function():
    pass
"#,
    )]);

    let handles: std::collections::HashMap<&str, crate::state::handle::Handle> = _handles;
    let handle = handles.get("test.py").unwrap();
    let transaction = state.transaction();
    let module_info = transaction.get_module_info(handle).unwrap();

    let node = Node {
        uri: "file:///test.py".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 4,
            },
            end: Position {
                line: 0,
                character: 15,
            },
        },
    };

    // Test with provided name
    let result = extract_symbol_name(Some("provided_name".to_owned()), &node, &module_info);
    assert_eq!(result, "provided_name");
}

#[test]
fn test_extract_symbol_name_from_node_range() {
    let (_handles, state) = mk_multi_file_state_assert_no_errors(&[(
        "test.py",
        r#"def my_function():
    pass
"#,
    )]);

    let handles: std::collections::HashMap<&str, crate::state::handle::Handle> = _handles;
    let handle = handles.get("test.py").unwrap();
    let transaction = state.transaction();
    let module_info = transaction.get_module_info(handle).unwrap();

    let node = Node {
        uri: "file:///test.py".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 4,
            }, // Start of "my_function"
            end: Position {
                line: 0,
                character: 15,
            }, // End of "my_function"
        },
    };

    // Test without provided name - should extract from node range
    let result = extract_symbol_name(None, &node, &module_info);
    assert_eq!(result, "my_function");
}

#[test]
fn test_extract_symbol_name_variable() {
    let (_handles, state) = mk_multi_file_state_assert_no_errors(&[(
        "test.py",
        r#"x = 42
y = "hello"
"#,
    )]);

    let handles: std::collections::HashMap<&str, crate::state::handle::Handle> = _handles;
    let handle = handles.get("test.py").unwrap();
    let transaction = state.transaction();
    let module_info = transaction.get_module_info(handle).unwrap();

    let node = Node {
        uri: "file:///test.py".to_string(),
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            }, // Start of "x"
            end: Position {
                line: 0,
                character: 1,
            }, // End of "x"
        },
    };

    // Test extracting variable name
    let result = extract_symbol_name(None, &node, &module_info);
    assert_eq!(result, "x");
}
