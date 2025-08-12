/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Common utilities and helper Functions for TSP request handling

use lsp_server::ErrorCode;
use lsp_server::Request;
use lsp_server::ResponseError;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::tsp;

// ---------------------------------------------------------------------------
// Backward compatibility shims (manually added)
// ---------------------------------------------------------------------------
// Older code expected a TSP_PROTOCOL_VERSION constant; alias to generated name.
// Reference the generated version constant without editing the generated file.
pub const TSP_PROTOCOL_VERSION: &str = crate::tsp::TYPE_SERVER_VERSION;

// Older handlers referenced GetSupportedProtocolVersionParams even though
// the generator only emits a Request with no params. Provide an empty params
// struct so existing handler signatures (before refactor) can compile or we
// can simplify handlers to omit it. This can be removed once all handlers
// are updated to not expect params.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSupportedProtocolVersionParams {}

// -------------------------------------------------------------------------------------------------
// Compatibility shims for legacy handwritten code expecting older enum shapes / helper builders.
// These adapt the generated protocol.rs API (do NOT modify the generated file).
// Keep this section minimal; remove once all call sites are migrated.
// -------------------------------------------------------------------------------------------------

// Legacy builder-style aliases removed (generator now supplies snake_case methods directly)

// Lightweight debug macro used by request handlers (avoids pulling in tracing for generated-ish code)
#[macro_export]
macro_rules! tsp_debug {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) { eprintln!("[TSP] {}:", module_path!()); eprintln!($($arg)*); }
    }};
}
pub use tsp_debug;

/// Legacy constant-like ALL_CAPS variants for DeclarationCategory (optional)
#[cfg(feature = "legacy_tsp_aliases")]
#[allow(non_upper_case_globals)]
pub mod legacy_decl_category {
    pub use crate::tsp::DeclarationCategory as DC; // alias for brevity inside module
    pub const CLASS: DC = DC::Class;
    pub const VARIABLE: DC = DC::Variable;
    pub const PARAM: DC = DC::Param;
    pub const TYPE_PARAM: DC = DC::TypeParam;
    pub const TYPE_ALIAS: DC = DC::TypeAlias;
    pub const IMPORT: DC = DC::Import;
    pub const INTRINSIC: DC = DC::Intrinsic;
}
#[cfg(feature = "legacy_tsp_aliases")]
pub use legacy_decl_category as decl_cat; // optional re-export

/// Legacy ALL_CAPS for TypeCategory
#[cfg(feature = "legacy_tsp_aliases")]
#[allow(non_upper_case_globals)]
pub mod legacy_type_category {
    pub use crate::tsp::TypeCategory as TC;
    pub const ANY: TC = TC::Any;
    pub const FUNCTION: TC = TC::Function;
    pub const OVERLOADED: TC = TC::Overloaded;
    pub const CLASS: TC = TC::Class;
    pub const MODULE: TC = TC::Module;
    pub const UNION: TC = TC::Union;
    pub const TYPE_VAR: TC = TC::TypeVar;
}

/// Legacy ALL_CAPS for AttributeFlags (old code referenced NONE / PARAMETER / RETURN_TYPE etc.)
/// Only existing flags are mapped; removed flags are intentionally omitted.
#[cfg(feature = "legacy_tsp_aliases")]
#[allow(non_upper_case_globals)]
pub mod legacy_attribute_flags {
    pub use crate::tsp::AttributeFlags as AF;
    pub const NONE: AF = AF::None;
}

/// Legacy ALL_CAPS for TypeFlags if referenced (INSTANCE / CALLABLE etc.)
#[cfg(feature = "legacy_tsp_aliases")]
#[allow(non_upper_case_globals)]
pub mod legacy_type_flags {
    pub use crate::tsp::TypeFlags as TF;
    pub const NONE: TF = TF::None;
    pub const CALLABLE: TF = TF::Callable;
    pub const INSTANCE: TF = TF::Instance;
    pub const INSTANTIABLE: TF = TF::Instantiable;
    pub const LITERAL: TF = TF::Literal;
    pub const FROM_ALIAS: TF = TF::FromAlias;
}

/// Add the query helper methods that legacy code expected on TypeReprFlags
impl tsp::TypeReprFlags {
    #[inline]
    pub fn has_expand_type_aliases(&self) -> bool { self.contains(tsp::TypeReprFlags::EXPAND_TYPE_ALIASES) }
    #[inline]
    pub fn has_print_type_var_variance(&self) -> bool { self.contains(tsp::TypeReprFlags::PRINT_TYPE_VAR_VARIANCE) }
    #[inline]
    pub fn has_convert_to_instance_type(&self) -> bool { self.contains(tsp::TypeReprFlags::CONVERT_TO_INSTANCE_TYPE) }
}

/// Provide a Default implementation shim for ResolveImportOptions (all None)
impl Default for tsp::ResolveImportOptions {
    fn default() -> Self {
        tsp::ResolveImportOptions {
            allow_externally_hidden_access: None,
            resolve_local_names: None,
            skip_file_needed_check: None,
        }
    }
}

/// Helper: convert protocol Position to lsp_types::Position
pub fn to_lsp_position(pos: &tsp::Position) -> lsp_types::Position {
    lsp_types::Position {
        line: pos.line,
        character: pos.character,
    }
}

/// Helper: convert lsp_types::Position to protocol Position
pub fn from_lsp_position(pos: lsp_types::Position) -> tsp::Position {
    tsp::Position {
        line: pos.line,
        character: pos.character,
    }
}

/// Helper: convert protocol Range to lsp_types::Range
pub fn to_lsp_range(r: &tsp::Range) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_position(&r.start),
        end: to_lsp_position(&r.end),
    }
}

/// Helper: convert lsp_types::Range to protocol Range
pub fn from_lsp_range(r: lsp_types::Range) -> tsp::Range {
    tsp::Range {
        start: from_lsp_position(r.start),
        end: from_lsp_position(r.end),
    }
}

/// Handle TypeServer Protocol (TSP) requests that don't implement the LSP Request trait
#[allow(dead_code)]
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

/// Helper to build a JSON-RPC error response for TSP handlers
pub fn error_response(
    id: lsp_server::RequestId,
    code: i32,
    message: String,
) -> lsp_server::Response {
    lsp_server::Response {
        id,
        result: None,
        error: Some(ResponseError {
            code,
            message,
            data: None,
        }),
    }
}

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
        tsp::DeclarationCategory::Function => {
            (tsp::TypeCategory::Function, tsp::TypeFlags::CALLABLE)
        }
    tsp::DeclarationCategory::Class => (tsp::TypeCategory::Class, tsp::TypeFlags::INSTANTIABLE),
    tsp::DeclarationCategory::Import => (tsp::TypeCategory::Module, tsp::TypeFlags::NONE),
    tsp::DeclarationCategory::TypeAlias => (tsp::TypeCategory::Any, tsp::TypeFlags::FROM_ALIAS),
    tsp::DeclarationCategory::TypeParam => (tsp::TypeCategory::TypeVar, tsp::TypeFlags::NONE),
    _ => (tsp::TypeCategory::Any, tsp::TypeFlags::NONE),
    };

    // Convert the declaration handle into a type handle. We just mirror the
    // underlying representation (string or int) so synthesized types remain
    // stable within the snapshot.
    let type_handle = match &decl.handle {
        tsp::DeclarationHandle::String(s) => tsp::TypeHandle::String(s.clone()),
        tsp::DeclarationHandle::Int(i) => tsp::TypeHandle::Int(*i),
    };

    tsp::Type {
        alias_name: None,
        handle: type_handle,
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
        alias_name: None,
    }
}

/// Calculate type flags for a pyrefly Type
pub fn calculate_type_flags(py_type: &crate::types::types::Type) -> tsp::TypeFlags {
    use crate::types::types::Type as PyType;
    match py_type {
    PyType::ClassDef(_) => tsp::TypeFlags::INSTANTIABLE,
    PyType::ClassType(_) => tsp::TypeFlags::INSTANCE,
    PyType::Function(_) | PyType::Callable(_) => tsp::TypeFlags::CALLABLE,
        PyType::Literal(_) => tsp::TypeFlags::LITERAL,
    PyType::TypeAlias(_) => tsp::TypeFlags::FROM_ALIAS,
    _ => tsp::TypeFlags::NONE,
    }
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
    fn test_get_supported_protocol_version_params_deserialization() {
        // Test null case
        let null_json = serde_json::Value::Null;
        let result: Result<GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(null_json);
        assert!(result.is_ok());

        // Test empty object case
        let empty_obj_json = serde_json::json!({});
        let result: Result<GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(empty_obj_json);
        assert!(result.is_ok());

        // Test object with unknown fields (should be ignored)
        let obj_with_fields = serde_json::json!({"unknown_field": "value"});
        let result: Result<GetSupportedProtocolVersionParams, _> =
            serde_json::from_value(obj_with_fields);
        assert!(result.is_ok());
    }
}
