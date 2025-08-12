/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Common utilities and helper Functions for TSP request handling

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use lsp_server::Request;
use serde::de::DeserializeOwned;

use crate::tsp;

/// Handle TypeServer Protocol (TSP) requests that don't implement the LSP Request trait
pub fn as_tsp_request<T>(x: &Request, method_name: &str) -> Option<Result<T, serde_json::Error>>
where
    T: DeserializeOwned,
{
    if x.method == method_name {
        match serde_json::from_value(x.params.clone()) {
            Ok(request) => Some(Ok(request)),
            Err(err) => Some(Err(err)),
        }
    } else {
        None
    }
}

/// LSP debug logging that can be disabled in release builds
#[cfg(debug_assertions)]
macro_rules! tsp_debug {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}

#[cfg(not(debug_assertions))]
macro_rules! tsp_debug {
    ($($arg:tt)*) => {};
}

// Re-export the macro for use in TSP request modules
pub(crate) use tsp_debug;

/// Creates a snapshot outdated error
#[allow(dead_code)]
pub(crate) fn snapshot_outdated_error() -> ResponseError {
    ResponseError {
        code: ErrorCode::ServerCancelled as i32,
        message: "Snapshot outdated".to_owned(),
        data: None,
    }
}

/// Creates a common error response for internal errors
#[allow(dead_code)]
pub(crate) fn create_internal_error(message: &str) -> ResponseError {
    ResponseError {
        code: ErrorCode::InternalError as i32,
        message: message.to_owned(),
        data: None,
    }
}

/// Creates a common error response for language services being disabled
#[allow(dead_code)]
pub(crate) fn language_services_disabled_error() -> ResponseError {
    ResponseError {
        code: ErrorCode::RequestFailed as i32,
        message: "Language services disabled".to_owned(),
        data: None,
    }
}

/// Create a default type for a declaration when we can't determine the exact type
pub fn create_default_type_for_declaration(decl: &tsp::Declaration) -> tsp::Type {
    let (category, flags) = match decl.category {
        tsp::DeclarationCategory::Function => (
            tsp::TypeCategory::Function,
            tsp::TypeFlags::new().with_callable(),
        ),
        tsp::DeclarationCategory::Class => (
            tsp::TypeCategory::Class,
            tsp::TypeFlags::new().with_instantiable(),
        ),
        tsp::DeclarationCategory::Import => (tsp::TypeCategory::Module, tsp::TypeFlags::new()),
        tsp::DeclarationCategory::TypeAlias => (
            tsp::TypeCategory::Any,
            tsp::TypeFlags::new().with_from_alias(),
        ),
        tsp::DeclarationCategory::TypeParam => {
            (tsp::TypeCategory::TypeVar, tsp::TypeFlags::new())
        }
        _ => (tsp::TypeCategory::Any, tsp::TypeFlags::new()),
    };

    tsp::Type {
        alias_name: None,
        handle: decl.handle.clone(),
        category,
        flags,
        module_name: Some(decl.module_name.clone()),
        name: decl.name.clone(),
        category_flags: 0,
        decl: None,
    }
}

/// Convert a pyrefly Type to a TSP Type
pub fn convert_to_tsp_type(py_type: crate::types::types::Type) -> tsp::Type {
    use crate::types::types::Type as PyType;

    tsp::Type {
        handle: tsp::TypeHandle::String(format!("{:p}", &py_type as *const _)),
        category: match &py_type {
            PyType::Any(_) => tsp::TypeCategory::Any,
            PyType::Function(_) | PyType::Callable(_) => tsp::TypeCategory::Function,
            PyType::Overload(_) => tsp::TypeCategory::Overloaded,
            PyType::ClassType(_) | PyType::ClassDef(_) => tsp::TypeCategory::Class,
            PyType::Module(_) => tsp::TypeCategory::Module,
            PyType::Union(_) => tsp::TypeCategory::Union,
            PyType::TypeVar(_) => tsp::TypeCategory::TypeVar,
            _ => tsp::TypeCategory::Any,
        },
        flags: calculate_type_flags(&py_type),
        module_name: extract_module_name(&py_type),
        name: py_type.to_string(),
        category_flags: 0,
        decl: None,
        alias_name: None
    }
}

/// Calculate type flags for a pyrefly Type
pub fn calculate_type_flags(py_type: &crate::types::types::Type) -> tsp::TypeFlags {
    use crate::types::types::Type as PyType;

    let mut flags = tsp::TypeFlags::new();

    match py_type {
        PyType::ClassDef(_) => flags = flags.with_instantiable(),
        PyType::ClassType(_) => flags = flags.with_instance(),
        PyType::Function(_) | PyType::Callable(_) => flags = flags.with_callable(),
        PyType::Literal(_) => flags = flags.with_literal(),
        PyType::TypeAlias(_) => flags = flags.with_from_alias(),
        _ => {}
    }

    flags
}

/// Extract module name from a pyrefly Type
pub fn extract_module_name(py_type: &crate::types::types::Type) -> Option<tsp::ModuleName> {
    use crate::types::types::Type as PyType;

    match py_type {
        PyType::ClassType(ct) => Some(convert_module_name(ct.qname().module_name())),
        PyType::ClassDef(cd) => Some(convert_module_name(cd.qname().module_name())),
        PyType::Module(m) => Some(convert_module_name_from_string(&m.to_string())),
        _ => None,
    }
}

/// Convert a pyrefly ModuleName to a TSP ModuleName
pub fn convert_module_name(
    pyrefly_module: pyrefly_python::module_name::ModuleName,
) -> tsp::ModuleName {
    tsp::ModuleName {
        leading_dots: 0, // pyrefly modules don't have leading dots in this context
        name_parts: pyrefly_module
            .as_str()
            .split('.')
            .map(|s| s.to_owned())
            .collect(),
    }
}

/// Convert TSP ModuleName back to pyrefly ModuleName
pub fn convert_tsp_module_name_to_pyrefly(
    tsp_module: &tsp::ModuleName,
) -> pyrefly_python::module_name::ModuleName {
    let module_str = tsp_module.name_parts.join(".");

    // Normalize __builtins__ to builtins so that the builtins module can be found
    let normalized_module_str = if module_str == "__builtins__" {
        "builtins".to_owned()
    } else {
        module_str
    };

    pyrefly_python::module_name::ModuleName::from_str(&normalized_module_str)
}

/// Convert a module string to a TSP ModuleName
pub fn convert_module_name_from_string(module_str: &str) -> tsp::ModuleName {
    tsp::ModuleName {
        leading_dots: 0,
        name_parts: module_str.split('.').map(|s| s.to_owned()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_get_snapshot_params_deserialization() {
        // Test null case
        let null_json = serde_json::Value::Null;
        let result: Result<tsp::GetSnapshotParams, _> = serde_json::from_value(null_json);
        assert!(result.is_ok());

        // Test empty object case
        let empty_obj_json = serde_json::json!({});
        let result: Result<tsp::GetSnapshotParams, _> = serde_json::from_value(empty_obj_json);
        assert!(result.is_ok());

        // Test object with unknown fields (should be ignored)
        let obj_with_fields = serde_json::json!({"unknown_field": "value"});
        let result: Result<tsp::GetSnapshotParams, _> = serde_json::from_value(obj_with_fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_supported_protocol_version_params_deserialization() {
        // Test null case
        let null_json = serde_json::Value::Null;
        let result: Result<tsp::GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(null_json);
        assert!(result.is_ok());

        // Test empty object case
        let empty_obj_json = serde_json::json!({});
        let result: Result<tsp::GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(empty_obj_json);
        assert!(result.is_ok());

        // Test object with unknown fields (should be ignored)
        let obj_with_fields = serde_json::json!({"unknown_field": "value"});
        let result: Result<tsp::GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(obj_with_fields);
        assert!(result.is_ok());
    }
}
