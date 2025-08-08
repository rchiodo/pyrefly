/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Unit tests for get_type_attributes TSP request handler

use serde_json;

use crate::tsp::GetTypeAttributesParams;
use crate::tsp::ModuleName;
use crate::tsp::Type;
use crate::tsp::TypeCategory;
use crate::tsp::TypeFlags;
use crate::tsp::TypeHandle;

#[test]
fn test_get_type_attributes_params_construction() {
    // Test creating GetTypeAttributesParams with valid data
    let handle = TypeHandle::String("test_handle_123".to_string());
    let test_type = Type {
        category: TypeCategory::CLASS,
        name: "TestClass".to_string(),
        handle: handle.clone(),
        flags: TypeFlags::new(),
        category_flags: 0,
        decl: None,
        module_name: None,
    };

    let params = GetTypeAttributesParams {
        snapshot: 42,
        type_param: test_type.clone(),
    };

    assert_eq!(params.snapshot, 42);
    assert_eq!(params.type_param.name, "TestClass");
    assert_eq!(params.type_param.category, TypeCategory::CLASS);
    // Note: handle comparison skipped as TypeHandle doesn't implement PartialEq
}

#[test]
fn test_get_type_attributes_params_serialization() {
    // Test JSON serialization/deserialization of GetTypeAttributesParams
    let handle = TypeHandle::String("handle_456".to_string());
    let test_type = Type {
        category: TypeCategory::FUNCTION,
        name: "test_function".to_string(),
        handle: handle.clone(),
        flags: TypeFlags::new().with_callable(),
        category_flags: 16,
        decl: None,
        module_name: Some(ModuleName {
            leading_dots: 0,
            name_parts: vec!["test_module".to_string()],
        }),
    };

    let original_params = GetTypeAttributesParams {
        snapshot: 123,
        type_param: test_type,
    };

    // Serialize to JSON and back
    let json = serde_json::to_string(&original_params).expect("Should serialize");
    let deserialized: GetTypeAttributesParams =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.snapshot, original_params.snapshot);
    assert_eq!(
        deserialized.type_param.name,
        original_params.type_param.name
    );
    assert_eq!(
        deserialized.type_param.category,
        original_params.type_param.category
    );
    // Note: handle, flags, and module_name comparisons skipped as they don't implement PartialEq
    assert_eq!(
        deserialized.type_param.category_flags,
        original_params.type_param.category_flags
    );
}

#[test]
fn test_get_type_attributes_params_json_format() {
    // Test that JSON format matches expected TSP structure
    let handle = TypeHandle::String("json_test_handle".to_string());
    let test_type = Type {
        category: TypeCategory::CLASS,
        name: "JsonTestClass".to_string(),
        handle,
        flags: TypeFlags::new().with_instantiable(),
        category_flags: 24,
        decl: None,
        module_name: Some(ModuleName {
            leading_dots: 0,
            name_parts: vec!["json_module".to_string()],
        }),
    };

    let params = GetTypeAttributesParams {
        snapshot: 789,
        type_param: test_type,
    };

    let json = serde_json::to_value(&params).expect("Should serialize to JSON");

    // Verify JSON structure has the expected fields
    assert!(json.get("snapshot").is_some());
    assert!(json.get("type").is_some()); // Should serialize as "type", not "type_param"

    let type_json = json.get("type").unwrap();
    assert!(type_json.get("category").is_some());
    assert!(type_json.get("name").is_some());
    assert!(type_json.get("handle").is_some());
    assert!(type_json.get("flags").is_some());
    assert!(type_json.get("categoryFlags").is_some());
    assert!(type_json.get("moduleName").is_some());
}

#[test]
fn test_get_type_attributes_params_function_type() {
    // Test GetTypeAttributesParams with function type to verify our enhancement
    let handle = TypeHandle::String("function_handle_123".to_string());
    let function_type = Type {
        category: TypeCategory::FUNCTION,
        name: "test_function".to_string(),
        handle: handle.clone(),
        flags: TypeFlags::new().with_callable(),
        category_flags: 0,
        decl: None,
        module_name: Some(ModuleName {
            leading_dots: 0,
            name_parts: vec!["test_module".to_string()],
        }),
    };

    let params = GetTypeAttributesParams {
        snapshot: 100,
        type_param: function_type.clone(),
    };

    assert_eq!(params.snapshot, 100);
    assert_eq!(params.type_param.name, "test_function");
    assert_eq!(params.type_param.category, TypeCategory::FUNCTION);

    // Test serialization for function type
    let json = serde_json::to_string(&params).expect("Should serialize function type params");
    let deserialized: GetTypeAttributesParams =
        serde_json::from_str(&json).expect("Should deserialize function type params");

    assert_eq!(deserialized.snapshot, params.snapshot);
    assert_eq!(deserialized.type_param.name, params.type_param.name);
    assert_eq!(deserialized.type_param.category, TypeCategory::FUNCTION);
}

#[test]
fn test_get_type_attributes_params_callable_type() {
    // Test GetTypeAttributesParams with callable type (also maps to FUNCTION category)
    let handle = TypeHandle::Integer(999);
    let callable_type = Type {
        category: TypeCategory::FUNCTION, // Callable types use FUNCTION category
        name: "Callable[[int, str], bool]".to_string(),
        handle,
        flags: TypeFlags::new().with_callable(),
        category_flags: 0,
        decl: None,
        module_name: None,
    };

    let params = GetTypeAttributesParams {
        snapshot: 200,
        type_param: callable_type.clone(),
    };

    assert_eq!(params.snapshot, 200);
    assert_eq!(params.type_param.name, "Callable[[int, str], bool]");
    assert_eq!(params.type_param.category, TypeCategory::FUNCTION);

    // Test JSON round-trip for callable type
    let json_value = serde_json::to_value(&params).expect("Should serialize callable type params");
    let json_str = serde_json::to_string(&json_value).expect("Should convert to string");
    let deserialized: GetTypeAttributesParams =
        serde_json::from_str(&json_str).expect("Should deserialize callable type params");

    assert_eq!(deserialized.snapshot, params.snapshot);
    assert_eq!(deserialized.type_param.name, params.type_param.name);
    assert_eq!(deserialized.type_param.category, TypeCategory::FUNCTION);
}
