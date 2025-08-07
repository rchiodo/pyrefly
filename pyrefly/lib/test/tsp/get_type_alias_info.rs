/*
 * Unit tests for get_type_alias_info request handler
 *
 * These tests verify the get_type_alias_info functionality by:
 * 1. Testing parameter construction and validation
 * 2. Testing TypeAliasInfo struct creation
 * 3. Following the same pattern as other TSP request tests
 */

use super::util::build_tsp_test_server;
use crate::tsp::GetTypeAliasInfoParams;
use crate::tsp::ModuleName;
use crate::tsp::Type;
use crate::tsp::TypeAliasInfo;
use crate::tsp::TypeCategory;
use crate::tsp::TypeFlags;
use crate::tsp::TypeHandle;

#[test]
fn test_get_type_alias_info_params_construction() {
    // Build test server
    let (_handle, _uri, _state) = build_tsp_test_server();

    // Test basic parameter construction
    let type_handle = TypeHandle::Integer(42);
    let tsp_type = Type {
        handle: type_handle.clone(),
        category: TypeCategory::CLASS,
        flags: TypeFlags::new().with_from_alias(), // Use FROM_ALIAS flag to indicate type alias
        module_name: None,
        name: "MyTypeAlias".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params = GetTypeAliasInfoParams {
        type_param: tsp_type.clone(),
        snapshot: 123,
    };

    // Verify parameter construction
    if let TypeHandle::Integer(handle_value) = &params.type_param.handle {
        assert_eq!(*handle_value, 42);
    } else {
        panic!("Expected integer type handle");
    }
    assert_eq!(params.snapshot, 123);
    assert_eq!(params.type_param.name, "MyTypeAlias");

    // Note: We can't directly test the FROM_ALIAS flag due to private fields,
    // but we can verify the type was constructed correctly
    assert_eq!(params.type_param.category, TypeCategory::CLASS);
}

#[test]
fn test_get_type_alias_info_params_different_handles() {
    // Test with different handle types
    let (_handle, _uri, _state) = build_tsp_test_server();

    // Test with string handle
    let string_type = Type {
        handle: TypeHandle::String("alias_handle".to_string()),
        category: TypeCategory::CLASS,
        flags: TypeFlags::new().with_from_alias(),
        module_name: Some(ModuleName {
            leading_dots: 0,
            name_parts: vec!["typing".to_string()],
        }),
        name: "List[str]".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params = GetTypeAliasInfoParams {
        type_param: string_type,
        snapshot: 456,
    };

    assert_eq!(params.snapshot, 456);
    assert_eq!(params.type_param.name, "List[str]");
    if let Some(module_name) = &params.type_param.module_name {
        assert_eq!(module_name.name_parts, vec!["typing"]);
        assert_eq!(module_name.leading_dots, 0);
    }
}

#[test]
fn test_type_alias_info_creation() {
    // Test TypeAliasInfo struct creation and validation
    let str_type = Type {
        handle: TypeHandle::Integer(1),
        category: TypeCategory::CLASS,
        flags: TypeFlags::new(),
        module_name: None,
        name: "str".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type_alias_info = TypeAliasInfo {
        name: "MyList".to_string(),
        type_args: Some(vec![str_type]),
    };

    assert_eq!(type_alias_info.name, "MyList");
    assert!(type_alias_info.type_args.is_some());
    assert_eq!(type_alias_info.type_args.unwrap().len(), 1);
}

#[test]
fn test_type_alias_info_no_type_arguments() {
    // Test TypeAliasInfo for non-generic type alias
    let type_alias_info = TypeAliasInfo {
        name: "SimpleAlias".to_string(),
        type_args: None,
    };

    assert_eq!(type_alias_info.name, "SimpleAlias");
    assert!(type_alias_info.type_args.is_none());
}

#[test]
fn test_type_alias_info_multiple_type_arguments() {
    // Test TypeAliasInfo with multiple type arguments
    let str_type = Type {
        handle: TypeHandle::Integer(1),
        category: TypeCategory::CLASS,
        flags: TypeFlags::new(),
        module_name: None,
        name: "str".to_string(),
        category_flags: 0,
        decl: None,
    };

    let int_type = Type {
        handle: TypeHandle::Integer(2),
        category: TypeCategory::CLASS,
        flags: TypeFlags::new(),
        module_name: None,
        name: "int".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type_alias_info = TypeAliasInfo {
        name: "MyDict".to_string(),
        type_args: Some(vec![str_type, int_type]),
    };

    assert_eq!(type_alias_info.name, "MyDict");
    assert!(type_alias_info.type_args.is_some());
    let type_args = type_alias_info.type_args.unwrap();
    assert_eq!(type_args.len(), 2);
    assert_eq!(type_args[0].name, "str");
    assert_eq!(type_args[1].name, "int");
}

#[test]
fn test_params_serialization_structure() {
    // Test that the params structure matches expected JSON structure
    let (_handle, _uri, _state) = build_tsp_test_server();

    let type_handle = TypeHandle::String("test_handle".to_string());
    let tsp_type = Type {
        handle: type_handle,
        category: TypeCategory::CLASS,
        flags: TypeFlags::new().with_from_alias(),
        module_name: None,
        name: "TestAlias".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params = GetTypeAliasInfoParams {
        type_param: tsp_type,
        snapshot: 789,
    };

    // Basic validation that the structure is correct
    assert_eq!(params.snapshot, 789);
    assert_eq!(params.type_param.name, "TestAlias");

    // Test serialization doesn't panic
    let _serialized = serde_json::to_string(&params).expect("Should serialize");
}
