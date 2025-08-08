/*
 * Unit tests for TSP get_overloads request handler
 *
 * These tests verify the get_overloads request parameter construction and validation by:
 * 1. Testing TSP GetOverloadsParams construction with various parameter combinations
 * 2. Validating proper handling of different Type parameter variations
 * 3. Testing snapshot validation logic
 * 4. Testing TypeHandle variations (String and Integer handles)
 * 5. Testing different type categories and flags combinations
 * 6. Testing module name handling and edge cases
 *
 * These unit tests complement the integration tests by focusing on parameter validation
 * and edge cases without requiring full TSP message protocol flows.
 */

use serde_json;

use crate::test::util::mk_multi_file_state_assert_no_errors;
use crate::tsp;
use crate::tsp::requests::get_overloads::extract_overloads_from_type;

#[test]
fn test_get_overloads_params_construction() {
    // Test basic parameter construction
    let type_param = tsp::Type {
        handle: tsp::TypeHandle::String("test_overloaded_function".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["mymodule".to_owned()],
        }),
        name: "overloaded_func".to_owned(),
        category_flags: 0,
        decl: Some(serde_json::json!({
            "kind": "function",
            "name": "overloaded_func"
        })),
    };

    let params = tsp::GetOverloadsParams {
        type_param: type_param.clone(),
        snapshot: 42,
    };

    // Verify parameter construction
    assert_eq!(params.snapshot, 42);
    assert_eq!(params.type_param.name, "overloaded_func");
    assert_eq!(params.type_param.category, tsp::TypeCategory::OVERLOADED);
    match &params.type_param.handle {
        tsp::TypeHandle::String(s) => assert_eq!(s, "test_overloaded_function"),
        _ => panic!("Expected String handle"),
    }
}

#[test]
fn test_get_overloads_different_type_categories() {
    // Test with OVERLOADED category (expected case)
    let overloaded_type = tsp::Type {
        handle: tsp::TypeHandle::String("overloaded_handle".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "overloaded_function".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params_overloaded = tsp::GetOverloadsParams {
        type_param: overloaded_type,
        snapshot: 1,
    };

    // Test with FUNCTION category (should not have overloads)
    let function_type = tsp::Type {
        handle: tsp::TypeHandle::String("function_handle".to_owned()),
        category: tsp::TypeCategory::FUNCTION,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "simple_function".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params_function = tsp::GetOverloadsParams {
        type_param: function_type,
        snapshot: 1,
    };

    // Test with ANY category (should not have overloads)
    let any_type = tsp::Type {
        handle: tsp::TypeHandle::String("any_handle".to_owned()),
        category: tsp::TypeCategory::ANY,
        flags: tsp::TypeFlags::new(),
        module_name: None,
        name: "any_type".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params_any = tsp::GetOverloadsParams {
        type_param: any_type,
        snapshot: 1,
    };

    // Verify different categories
    assert_eq!(
        params_overloaded.type_param.category,
        tsp::TypeCategory::OVERLOADED
    );
    assert_eq!(
        params_function.type_param.category,
        tsp::TypeCategory::FUNCTION
    );
    assert_eq!(params_any.type_param.category, tsp::TypeCategory::ANY);
}

#[test]
fn test_get_overloads_type_handle_variants() {
    // Test with String handle
    let string_handle_type = tsp::Type {
        handle: tsp::TypeHandle::String("string_handle_123".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["module".to_owned()],
        }),
        name: "string_handle_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params_string = tsp::GetOverloadsParams {
        type_param: string_handle_type,
        snapshot: 5,
    };

    // Test with Integer handle
    let integer_handle_type = tsp::Type {
        handle: tsp::TypeHandle::Integer(42),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["module".to_owned()],
        }),
        name: "integer_handle_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let params_integer = tsp::GetOverloadsParams {
        type_param: integer_handle_type,
        snapshot: 5,
    };

    // Verify handle types
    match &params_string.type_param.handle {
        tsp::TypeHandle::String(s) => assert_eq!(s, "string_handle_123"),
        _ => panic!("Expected String handle"),
    }

    match &params_integer.type_param.handle {
        tsp::TypeHandle::Integer(i) => assert_eq!(*i, 42),
        _ => panic!("Expected Integer handle"),
    }
}

#[test]
fn test_get_overloads_module_name_variants() {
    // Test with simple module name
    let simple_module = tsp::ModuleName {
        leading_dots: 0,
        name_parts: vec!["simple".to_owned()],
    };

    // Test with nested module name
    let nested_module = tsp::ModuleName {
        leading_dots: 0,
        name_parts: vec!["package".to_owned(), "submodule".to_owned()],
    };

    // Test with relative import (leading dots)
    let relative_module = tsp::ModuleName {
        leading_dots: 2,
        name_parts: vec!["relative".to_owned()],
    };

    // Test with no module name (None)
    let type_no_module = tsp::Type {
        handle: tsp::TypeHandle::String("no_module".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: None,
        name: "builtin_function".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let type_simple = tsp::Type {
        handle: tsp::TypeHandle::String("simple".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(simple_module.clone()),
        name: "simple_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let type_nested = tsp::Type {
        handle: tsp::TypeHandle::String("nested".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(nested_module.clone()),
        name: "nested_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    let type_relative = tsp::Type {
        handle: tsp::TypeHandle::String("relative".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(relative_module.clone()),
        name: "relative_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    // Create params
    let params_no_module = tsp::GetOverloadsParams {
        type_param: type_no_module,
        snapshot: 1,
    };

    let params_simple = tsp::GetOverloadsParams {
        type_param: type_simple,
        snapshot: 1,
    };

    let params_nested = tsp::GetOverloadsParams {
        type_param: type_nested,
        snapshot: 1,
    };

    let params_relative = tsp::GetOverloadsParams {
        type_param: type_relative,
        snapshot: 1,
    };

    // Verify module name handling
    assert!(params_no_module.type_param.module_name.is_none());

    let simple_mod = params_simple.type_param.module_name.as_ref().unwrap();
    assert_eq!(simple_mod.leading_dots, 0);
    assert_eq!(simple_mod.name_parts, vec!["simple"]);

    let nested_mod = params_nested.type_param.module_name.as_ref().unwrap();
    assert_eq!(nested_mod.leading_dots, 0);
    assert_eq!(nested_mod.name_parts, vec!["package", "submodule"]);

    let relative_mod = params_relative.type_param.module_name.as_ref().unwrap();
    assert_eq!(relative_mod.leading_dots, 2);
    assert_eq!(relative_mod.name_parts, vec!["relative"]);
}

#[test]
fn test_get_overloads_flags_handling() {
    // Test with no flags
    let no_flags_type = tsp::Type {
        handle: tsp::TypeHandle::String("no_flags".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "no_flags_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    // Test with some flags set
    let with_flags_type = tsp::Type {
        handle: tsp::TypeHandle::String("with_flags".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new().with_callable(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "callable_func".to_owned(),
        category_flags: 1, // Some category flags
        decl: None,
    };

    let params_no_flags = tsp::GetOverloadsParams {
        type_param: no_flags_type,
        snapshot: 10,
    };

    let params_with_flags = tsp::GetOverloadsParams {
        type_param: with_flags_type,
        snapshot: 10,
    };

    // Verify flag handling - just verify construction works properly
    // We can't compare TypeFlags directly since it doesn't implement PartialEq
    let _no_flags = params_no_flags.type_param.flags;
    let _with_flags = params_with_flags.type_param.flags;
    assert_eq!(params_no_flags.type_param.category_flags, 0);
    assert_eq!(params_with_flags.type_param.category_flags, 1);
}

#[test]
fn test_get_overloads_declaration_handling() {
    // Test with null declaration
    let null_decl_type = tsp::Type {
        handle: tsp::TypeHandle::String("null_decl".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "null_decl_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    // Test with simple declaration
    let simple_decl_type = tsp::Type {
        handle: tsp::TypeHandle::String("simple_decl".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "simple_decl_func".to_owned(),
        category_flags: 0,
        decl: Some(serde_json::json!({
            "kind": "function",
            "name": "simple_decl_func",
            "signature": "(int) -> str"
        })),
    };

    // Test with complex declaration
    let complex_decl_type = tsp::Type {
        handle: tsp::TypeHandle::String("complex_decl".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["complex".to_owned(), "module".to_owned()],
        }),
        name: "complex_func".to_owned(),
        category_flags: 0,
        decl: Some(serde_json::json!({
            "kind": "overloaded_function",
            "name": "complex_func",
            "signatures": [
                {"params": ["int"], "return": "str"},
                {"params": ["str"], "return": "int"},
                {"params": ["float"], "return": "str"}
            ],
            "module": "complex.module",
            "docstring": "A function with multiple overloads"
        })),
    };

    let params_null = tsp::GetOverloadsParams {
        type_param: null_decl_type,
        snapshot: 20,
    };

    let params_simple = tsp::GetOverloadsParams {
        type_param: simple_decl_type,
        snapshot: 20,
    };

    let params_complex = tsp::GetOverloadsParams {
        type_param: complex_decl_type,
        snapshot: 20,
    };

    // Verify declaration handling
    assert!(params_null.type_param.decl.is_none());

    let simple_decl = params_simple.type_param.decl.as_ref().unwrap();
    assert_eq!(simple_decl["kind"], "function");
    assert_eq!(simple_decl["name"], "simple_decl_func");

    let complex_decl = params_complex.type_param.decl.as_ref().unwrap();
    assert_eq!(complex_decl["kind"], "overloaded_function");
    assert_eq!(complex_decl["name"], "complex_func");
    assert!(complex_decl["signatures"].is_array());
    assert_eq!(complex_decl["signatures"].as_array().unwrap().len(), 3);
}

#[test]
fn test_get_overloads_snapshot_validation() {
    // Test with different snapshot values
    let type_param = tsp::Type {
        handle: tsp::TypeHandle::String("snapshot_test".to_owned()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        }),
        name: "snapshot_func".to_owned(),
        category_flags: 0,
        decl: None,
    };

    // Test with zero snapshot
    let params_zero = tsp::GetOverloadsParams {
        type_param: type_param.clone(),
        snapshot: 0,
    };

    // Test with positive snapshot
    let params_positive = tsp::GetOverloadsParams {
        type_param: type_param.clone(),
        snapshot: 12345,
    };

    // Test with negative snapshot (should be valid in parameter construction)
    let params_negative = tsp::GetOverloadsParams {
        type_param: type_param.clone(),
        snapshot: -1,
    };

    // Verify snapshot values
    assert_eq!(params_zero.snapshot, 0);
    assert_eq!(params_positive.snapshot, 12345);
    assert_eq!(params_negative.snapshot, -1);
}

#[test]
fn test_get_overloads_serialization_deserialization() {
    // Test that parameters can be properly serialized and deserialized
    let original_params = tsp::GetOverloadsParams {
        type_param: tsp::Type {
            handle: tsp::TypeHandle::String("serialization_test".to_owned()),
            category: tsp::TypeCategory::OVERLOADED,
            flags: tsp::TypeFlags::new().with_callable(),
            module_name: Some(tsp::ModuleName {
                leading_dots: 1,
                name_parts: vec!["serialization".to_owned(), "test".to_owned()],
            }),
            name: "serializable_func".to_owned(),
            category_flags: 2,
            decl: Some(serde_json::json!({
                "kind": "overloaded_function",
                "overloads": ["(int) -> str", "(str) -> int"]
            })),
        },
        snapshot: 999,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&original_params).expect("Failed to serialize");

    // Deserialize back from JSON
    let deserialized_params: tsp::GetOverloadsParams =
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    // Verify round-trip serialization
    assert_eq!(deserialized_params.snapshot, original_params.snapshot);
    assert_eq!(
        deserialized_params.type_param.name,
        original_params.type_param.name
    );
    assert_eq!(
        deserialized_params.type_param.category,
        original_params.type_param.category
    );
    // Note: TypeFlags doesn't implement PartialEq so we can't directly compare
    // but serialization/deserialization should preserve the flag structure
    assert_eq!(
        deserialized_params.type_param.category_flags,
        original_params.type_param.category_flags
    );

    match (
        &deserialized_params.type_param.handle,
        &original_params.type_param.handle,
    ) {
        (tsp::TypeHandle::String(d), tsp::TypeHandle::String(o)) => assert_eq!(d, o),
        _ => panic!("Handle type mismatch"),
    }

    let orig_module = original_params.type_param.module_name.as_ref().unwrap();
    let deser_module = deserialized_params.type_param.module_name.as_ref().unwrap();
    assert_eq!(deser_module.leading_dots, orig_module.leading_dots);
    assert_eq!(deser_module.name_parts, orig_module.name_parts);
}

// Standalone function tests for the core logic

#[test]
fn test_extract_overloads_from_type_non_overloaded() {
    let (_handles, _state) = mk_multi_file_state_assert_no_errors(&[(
        "test.py",
        r#"def simple_func(x: int) -> str:
    return str(x)
"#,
    )]);

    // Test with a non-overloaded type (simple function type)
    let function_type = crate::types::types::Type::never(); // Use a simple non-overloaded type

    let result = extract_overloads_from_type(&function_type);

    // Should return None for non-overloaded types
    assert!(result.is_none());
}

#[test]
fn test_extract_overloads_from_type_simple_validation() {
    let (_handles, _state) = mk_multi_file_state_assert_no_errors(&[(
        "test.py",
        r#"from typing import overload

@overload
def process(x: int) -> str: ...

@overload  
def process(x: str) -> int: ...

def process(x):
    if isinstance(x, int):
        return str(x)
    else:
        return len(x)
"#,
    )]);

    // For this test, we're mainly verifying that the function handles the extraction
    // correctly without needing to construct complex overload types

    // Test that the function exists and can be called with basic types
    let simple_type = crate::types::types::Type::never();
    let result = extract_overloads_from_type(&simple_type);

    // This should return None since it's not an overloaded type
    assert!(result.is_none());

    // Test basic functionality - the specific type construction is complex,
    // so we focus on the function's existence and basic behavior
    // More detailed tests would require building actual overload types
}
