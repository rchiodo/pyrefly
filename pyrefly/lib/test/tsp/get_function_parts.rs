/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP getFunctionParts request parameter construction

use super::util::build_tsp_test_server;
use crate::tsp::GetFunctionPartsParams;
use crate::tsp::Type;
use crate::tsp::TypeCategory;
use crate::tsp::TypeFlags;
use crate::tsp::TypeHandle;
use crate::tsp::TypeReprFlags;

#[test]
fn test_get_function_parts_params_construction() {
    // Build test server
    let (_handle, _uri, _state) = build_tsp_test_server();

    // Test basic parameter construction
    let type_handle = TypeHandle::Integer(42);
    let tsp_type = Type {
        handle: type_handle.clone(),
        category: TypeCategory::FUNCTION,
        flags: TypeFlags::new().with_callable(),
        module_name: None,
        name: "test_function".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params = GetFunctionPartsParams {
        type_param: tsp_type.clone(),
        flags: TypeReprFlags::NONE,
        snapshot: 123,
    };

    // Verify parameter construction
    if let TypeHandle::Integer(handle_value) = &params.type_param.handle {
        assert_eq!(*handle_value, 42);
    } else {
        panic!("Expected integer type handle");
    }
    assert_eq!(params.snapshot, 123);
    assert_eq!(params.type_param.name, "test_function");

    // Test with different flags
    let params_with_flags = GetFunctionPartsParams {
        type_param: tsp_type.clone(),
        flags: TypeReprFlags::EXPAND_TYPE_ALIASES,
        snapshot: 456,
    };

    if let TypeHandle::Integer(handle_value) = &params_with_flags.type_param.handle {
        assert_eq!(*handle_value, 42);
    } else {
        panic!("Expected integer type handle");
    }
    assert!(params_with_flags.flags.has_expand_type_aliases());
    assert_eq!(params_with_flags.snapshot, 456);

    // Test parameter serialization/deserialization
    let json_str = serde_json::to_string(&params).unwrap();
    let deserialized: GetFunctionPartsParams = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.type_param.name, params.type_param.name);
    assert_eq!(deserialized.snapshot, params.snapshot);
}

#[test]
fn test_get_function_parts_params_with_different_types() {
    // Build test server
    let (_handle, _uri, _state) = build_tsp_test_server();

    // Test parameter construction for different function types
    let type1 = Type {
        handle: TypeHandle::Integer(100),
        category: TypeCategory::FUNCTION,
        flags: TypeFlags::new().with_callable(),
        module_name: None,
        name: "simple_func".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type2 = Type {
        handle: TypeHandle::String("async_func_handle".to_string()),
        category: TypeCategory::FUNCTION,
        flags: TypeFlags::new().with_callable().with_instance(),
        module_name: None,
        name: "async_func".to_string(),
        category_flags: 0,
        decl: None,
    };

    let type3 = Type {
        handle: TypeHandle::Integer(300),
        category: TypeCategory::OVERLOADED,
        flags: TypeFlags::new().with_callable(),
        module_name: None,
        name: "overloaded_func".to_string(),
        category_flags: 0,
        decl: None,
    };

    let params1 = GetFunctionPartsParams {
        type_param: type1,
        flags: TypeReprFlags::NONE,
        snapshot: 1,
    };

    let params2 = GetFunctionPartsParams {
        type_param: type2,
        flags: TypeReprFlags::CONVERT_TO_INSTANCE_TYPE,
        snapshot: 2,
    };

    let params3 = GetFunctionPartsParams {
        type_param: type3,
        flags: TypeReprFlags::PRINT_TYPE_VAR_VARIANCE,
        snapshot: 3,
    };

    // Verify each parameter set is distinct
    assert_eq!(params1.type_param.name, "simple_func");
    assert_eq!(params2.type_param.name, "async_func");
    assert_eq!(params3.type_param.name, "overloaded_func");

    assert_eq!(params1.snapshot, 1);
    assert_eq!(params2.snapshot, 2);
    assert_eq!(params3.snapshot, 3);

    // Test different flag combinations
    assert!(!params1.flags.has_expand_type_aliases());
    assert!(params2.flags.has_convert_to_instance_type());
    assert!(params3.flags.has_print_type_var_variance());
}
