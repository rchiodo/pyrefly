/*
 * Unit tests for TSP search_for_type_attribute request handler
 *
 * These tests verify the search_for_type_attribute request parameter construction and validation by:
 * 1. Testing TSP SearchForTypeAttributeParams construction with various parameter combinations
 * 2. Validating proper handling of different Type parameter variations (start_type)
 * 3. Testing attribute name handling and edge cases
 * 4. Testing access flags combinations (NONE, SKIP_INSTANCE_ATTRIBUTES, etc.)
 * 5. Testing optional parameters (expression_node, instance_type)
 * 6. Testing snapshot validation logic
 * 7. Testing TypeHandle variations and module name handling
 * 8. Testing serialization/deserialization for LSP protocol compliance
 *
 * These unit tests complement the integration tests by focusing on parameter validation
 * and edge cases without requiring full TSP message protocol flows.
 */

use lsp_types::{Position, Range, Url};
use serde_json;

use crate::tsp;

#[test]
fn test_search_for_type_attribute_params_construction() {
    // Test basic parameter construction
    let start_type = tsp::Type {
        handle: tsp::TypeHandle::String("test_class_type".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["mymodule".to_string()],
        }),
        name: "MyClass".to_string(),
        category_flags: 0,
        decl: Some(serde_json::json!({
            "kind": "class",
            "name": "MyClass"
        })),
    };

    let params = tsp::SearchForTypeAttributeParams {
        start_type: start_type.clone(),
        attribute_name: "my_method".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 42,
    };

    // Verify parameter construction
    assert_eq!(params.snapshot, 42);
    assert_eq!(params.attribute_name, "my_method");
    assert_eq!(params.start_type.name, "MyClass");
    assert_eq!(params.start_type.category, tsp::TypeCategory::CLASS);
    match &params.start_type.handle {
        tsp::TypeHandle::String(s) => assert_eq!(s, "test_class_type"),
        _ => panic!("Expected String handle"),
    }
    assert!(params.expression_node.is_none());
    assert!(params.instance_type.is_none());
}

#[test]
fn test_search_for_type_attribute_different_start_types() {
    // Test with CLASS category (expected case)
    let class_type = tsp::Type {
        handle: tsp::TypeHandle::String("class_handle".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["models".to_string()],
        }),
        name: "User".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_class = tsp::SearchForTypeAttributeParams {
        start_type: class_type,
        attribute_name: "name".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with FUNCTION category (should not have attributes typically)
    let function_type = tsp::Type {
        handle: tsp::TypeHandle::String("function_handle".to_string()),
        category: tsp::TypeCategory::FUNCTION,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["utils".to_string()],
        }),
        name: "helper_function".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_function = tsp::SearchForTypeAttributeParams {
        start_type: function_type,
        attribute_name: "__name__".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with OVERLOADED category
    let overloaded_type = tsp::Type {
        handle: tsp::TypeHandle::String("overloaded_handle".to_string()),
        category: tsp::TypeCategory::OVERLOADED,
        flags: tsp::TypeFlags::new(),
        module_name: None,
        name: "overloaded_func".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_overloaded = tsp::SearchForTypeAttributeParams {
        start_type: overloaded_type,
        attribute_name: "__call__".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Verify different categories
    assert_eq!(params_class.start_type.category, tsp::TypeCategory::CLASS);
    assert_eq!(params_function.start_type.category, tsp::TypeCategory::FUNCTION);
    assert_eq!(params_overloaded.start_type.category, tsp::TypeCategory::OVERLOADED);
}

#[test]
fn test_search_for_type_attribute_attribute_name_variants() {
    let base_type = tsp::Type {
        handle: tsp::TypeHandle::String("test_type".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_string()],
        }),
        name: "TestClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    // Test with regular method name
    let params_method = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "regular_method".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with dunder method
    let params_dunder = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "__init__".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with property
    let params_property = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "my_property".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with private attribute
    let params_private = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "_private_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with mangled attribute
    let params_mangled = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "__very_private".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Verify attribute names
    assert_eq!(params_method.attribute_name, "regular_method");
    assert_eq!(params_dunder.attribute_name, "__init__");
    assert_eq!(params_property.attribute_name, "my_property");
    assert_eq!(params_private.attribute_name, "_private_attr");
    assert_eq!(params_mangled.attribute_name, "__very_private");
}

#[test]
fn test_search_for_type_attribute_access_flags() {
    let base_type = tsp::Type {
        handle: tsp::TypeHandle::String("test_type".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_string()],
        }),
        name: "TestClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    // Test with NONE flags
    let params_none = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with SKIP_INSTANCE_ATTRIBUTES flag
    let params_skip_instance = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::SKIP_INSTANCE_ATTRIBUTES,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with SKIP_TYPE_BASE_CLASS flag
    let params_skip_base = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::SKIP_TYPE_BASE_CLASS,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Test with multiple flags combined
    let combined_flags = tsp::AttributeAccessFlags(
        tsp::AttributeAccessFlags::SKIP_INSTANCE_ATTRIBUTES.0 
        | tsp::AttributeAccessFlags::SKIP_TYPE_BASE_CLASS.0
    );
    let params_combined = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: combined_flags,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Verify flags - we can check the inner values
    assert_eq!(params_none.access_flags.0, 0);
    assert_eq!(params_skip_instance.access_flags.0, 1);
    assert_eq!(params_skip_base.access_flags.0, 2);
    assert_eq!(params_combined.access_flags.0, 3); // 1 | 2 = 3
}

#[test]
fn test_search_for_type_attribute_optional_parameters() {
    let base_type = tsp::Type {
        handle: tsp::TypeHandle::String("test_type".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_string()],
        }),
        name: "TestClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    // Test with expression_node provided
    let test_uri = Url::parse("file:///test.py").unwrap();
    let expression_node = tsp::Node {
        uri: test_uri.clone(),
        range: Range {
            start: Position { line: 10, character: 5 },
            end: Position { line: 10, character: 15 },
        },
    };

    let params_with_node = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: Some(expression_node.clone()),
        instance_type: None,
        snapshot: 1,
    };

    // Test with instance_type provided
    let instance_type = tsp::Type {
        handle: tsp::TypeHandle::Integer(123),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new().with_instance(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_string()],
        }),
        name: "TestClassInstance".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_with_instance = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: Some(instance_type.clone()),
        snapshot: 1,
    };

    // Test with both optional parameters
    let params_with_both = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: Some(expression_node.clone()),
        instance_type: Some(instance_type.clone()),
        snapshot: 1,
    };

    // Verify optional parameters
    assert!(params_with_node.expression_node.is_some());
    assert!(params_with_node.instance_type.is_none());
    
    assert!(params_with_instance.expression_node.is_none());
    assert!(params_with_instance.instance_type.is_some());
    
    assert!(params_with_both.expression_node.is_some());
    assert!(params_with_both.instance_type.is_some());

    // Verify node details
    let node = params_with_node.expression_node.as_ref().unwrap();
    assert_eq!(node.uri, test_uri);
    assert_eq!(node.range.start.line, 10);
    assert_eq!(node.range.start.character, 5);

    // Verify instance type details
    let instance = params_with_instance.instance_type.as_ref().unwrap();
    assert_eq!(instance.name, "TestClassInstance");
    match &instance.handle {
        tsp::TypeHandle::Integer(i) => assert_eq!(*i, 123),
        _ => panic!("Expected Integer handle"),
    }
}

#[test]
fn test_search_for_type_attribute_type_handle_variants() {
    // Test with String handle
    let string_handle_type = tsp::Type {
        handle: tsp::TypeHandle::String("string_handle_class".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["models".to_string()],
        }),
        name: "StringHandleClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_string = tsp::SearchForTypeAttributeParams {
        start_type: string_handle_type,
        attribute_name: "string_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 5,
    };

    // Test with Integer handle
    let integer_handle_type = tsp::Type {
        handle: tsp::TypeHandle::Integer(789),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["models".to_string()],
        }),
        name: "IntegerHandleClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params_integer = tsp::SearchForTypeAttributeParams {
        start_type: integer_handle_type,
        attribute_name: "integer_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 5,
    };

    // Verify handle types
    match &params_string.start_type.handle {
        tsp::TypeHandle::String(s) => assert_eq!(s, "string_handle_class"),
        _ => panic!("Expected String handle"),
    }

    match &params_integer.start_type.handle {
        tsp::TypeHandle::Integer(i) => assert_eq!(*i, 789),
        _ => panic!("Expected Integer handle"),
    }
}

#[test]
fn test_search_for_type_attribute_module_name_variants() {
    // Test with simple module name
    let simple_module = tsp::ModuleName {
        leading_dots: 0,
        name_parts: vec!["simple".to_string()],
    };

    // Test with nested module name
    let nested_module = tsp::ModuleName {
        leading_dots: 0,
        name_parts: vec!["package".to_string(), "submodule".to_string()],
    };

    // Test with relative import (leading dots)
    let relative_module = tsp::ModuleName {
        leading_dots: 2,
        name_parts: vec!["relative".to_string()],
    };

    // Test with no module name (None)
    let type_no_module = tsp::Type {
        handle: tsp::TypeHandle::String("no_module".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: None,
        name: "BuiltinClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type_simple = tsp::Type {
        handle: tsp::TypeHandle::String("simple".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(simple_module.clone()),
        name: "SimpleClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type_nested = tsp::Type {
        handle: tsp::TypeHandle::String("nested".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(nested_module.clone()),
        name: "NestedClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type_relative = tsp::Type {
        handle: tsp::TypeHandle::String("relative".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(relative_module.clone()),
        name: "RelativeClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    // Create params
    let params_no_module = tsp::SearchForTypeAttributeParams {
        start_type: type_no_module,
        attribute_name: "builtin_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    let params_simple = tsp::SearchForTypeAttributeParams {
        start_type: type_simple,
        attribute_name: "simple_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    let params_nested = tsp::SearchForTypeAttributeParams {
        start_type: type_nested,
        attribute_name: "nested_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    let params_relative = tsp::SearchForTypeAttributeParams {
        start_type: type_relative,
        attribute_name: "relative_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 1,
    };

    // Verify module name handling
    assert!(params_no_module.start_type.module_name.is_none());

    let simple_mod = params_simple.start_type.module_name.as_ref().unwrap();
    assert_eq!(simple_mod.leading_dots, 0);
    assert_eq!(simple_mod.name_parts, vec!["simple"]);

    let nested_mod = params_nested.start_type.module_name.as_ref().unwrap();
    assert_eq!(nested_mod.leading_dots, 0);
    assert_eq!(nested_mod.name_parts, vec!["package", "submodule"]);

    let relative_mod = params_relative.start_type.module_name.as_ref().unwrap();
    assert_eq!(relative_mod.leading_dots, 2);
    assert_eq!(relative_mod.name_parts, vec!["relative"]);
}

#[test]
fn test_search_for_type_attribute_snapshot_validation() {
    let base_type = tsp::Type {
        handle: tsp::TypeHandle::String("snapshot_test".to_string()),
        category: tsp::TypeCategory::CLASS,
        flags: tsp::TypeFlags::new(),
        module_name: Some(tsp::ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_string()],
        }),
        name: "SnapshotClass".to_string(),
        category_flags: 0,
        decl: None,
    };

    // Test with zero snapshot
    let params_zero = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 0,
    };

    // Test with positive snapshot
    let params_positive = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: 12345,
    };

    // Test with negative snapshot (should be valid in parameter construction)
    let params_negative = tsp::SearchForTypeAttributeParams {
        start_type: base_type.clone(),
        attribute_name: "test_attr".to_string(),
        access_flags: tsp::AttributeAccessFlags::NONE,
        expression_node: None,
        instance_type: None,
        snapshot: -1,
    };

    // Verify snapshot values
    assert_eq!(params_zero.snapshot, 0);
    assert_eq!(params_positive.snapshot, 12345);
    assert_eq!(params_negative.snapshot, -1);
}

#[test]
fn test_search_for_type_attribute_serialization_deserialization() {
    // Test that parameters can be properly serialized and deserialized
    let test_uri = Url::parse("file:///test_serialization.py").unwrap();
    let original_params = tsp::SearchForTypeAttributeParams {
        start_type: tsp::Type {
            handle: tsp::TypeHandle::String("serialization_test".to_string()),
            category: tsp::TypeCategory::CLASS,
            flags: tsp::TypeFlags::new().with_instantiable(),
            module_name: Some(tsp::ModuleName {
                leading_dots: 1,
                name_parts: vec!["serialization".to_string(), "test".to_string()],
            }),
            name: "SerializableClass".to_string(),
            category_flags: 2,
            decl: Some(serde_json::json!({
                "kind": "class",
                "name": "SerializableClass",
                "methods": ["__init__", "serialize", "deserialize"]
            })),
        },
        attribute_name: "serialize_method".to_string(),
        access_flags: tsp::AttributeAccessFlags::SKIP_INSTANCE_ATTRIBUTES,
        expression_node: Some(tsp::Node {
            uri: test_uri.clone(),
            range: Range {
                start: Position { line: 5, character: 10 },
                end: Position { line: 5, character: 25 },
            },
        }),
        instance_type: Some(tsp::Type {
            handle: tsp::TypeHandle::Integer(456),
            category: tsp::TypeCategory::CLASS,
            flags: tsp::TypeFlags::new().with_instance(),
            module_name: Some(tsp::ModuleName {
                leading_dots: 0,
                name_parts: vec!["instance".to_string()],
            }),
            name: "InstanceType".to_string(),
            category_flags: 0,
            decl: None,
        }),
        snapshot: 999,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&original_params).expect("Failed to serialize");
    
    // Deserialize back from JSON
    let deserialized_params: tsp::SearchForTypeAttributeParams = 
        serde_json::from_str(&json_str).expect("Failed to deserialize");

    // Verify round-trip serialization
    assert_eq!(deserialized_params.snapshot, original_params.snapshot);
    assert_eq!(deserialized_params.attribute_name, original_params.attribute_name);
    assert_eq!(deserialized_params.start_type.name, original_params.start_type.name);
    assert_eq!(deserialized_params.start_type.category, original_params.start_type.category);
    assert_eq!(deserialized_params.start_type.category_flags, original_params.start_type.category_flags);
    assert_eq!(deserialized_params.access_flags.0, original_params.access_flags.0);

    match (&deserialized_params.start_type.handle, &original_params.start_type.handle) {
        (tsp::TypeHandle::String(d), tsp::TypeHandle::String(o)) => assert_eq!(d, o),
        _ => panic!("Handle type mismatch"),
    }

    let orig_module = original_params.start_type.module_name.as_ref().unwrap();
    let deser_module = deserialized_params.start_type.module_name.as_ref().unwrap();
    assert_eq!(deser_module.leading_dots, orig_module.leading_dots);
    assert_eq!(deser_module.name_parts, orig_module.name_parts);

    // Verify optional parameters
    assert!(deserialized_params.expression_node.is_some());
    assert!(deserialized_params.instance_type.is_some());

    let deser_node = deserialized_params.expression_node.as_ref().unwrap();
    let orig_node = original_params.expression_node.as_ref().unwrap();
    assert_eq!(deser_node.uri, orig_node.uri);
    assert_eq!(deser_node.range.start.line, orig_node.range.start.line);

    let deser_instance = deserialized_params.instance_type.as_ref().unwrap();
    let orig_instance = original_params.instance_type.as_ref().unwrap();
    assert_eq!(deser_instance.name, orig_instance.name);
}

