use serde::{Deserialize, Serialize};

pub enum GetTypeRequest {}

pub enum GetSymbolRequest {}

pub enum GetPythonSearchPathsRequest {}

pub enum GetSnapshotRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Type {
    pub handle: TypeHandle,
    pub category: TypeCategory,
    pub flags: TypeFlags,
    #[serde(rename = "moduleName")]
    pub module_name: Option<ModuleName>,
    pub name: String,
    #[serde(rename = "categoryFlags")]
    pub category_flags: i32,
    pub decl: Option<serde_json::Value>, // Generic object for declaration info
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TypeHandle {
    String(String),
    Integer(i32),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct TypeCategory(i32);

impl TypeCategory {
    pub const ANY: TypeCategory = TypeCategory(0);
    pub const FUNCTION: TypeCategory = TypeCategory(1);
    pub const OVERLOADED: TypeCategory = TypeCategory(2);
    pub const CLASS: TypeCategory = TypeCategory(3);
    pub const MODULE: TypeCategory = TypeCategory(4);
    pub const UNION: TypeCategory = TypeCategory(5);
    pub const TYPE_VAR: TypeCategory = TypeCategory(6);
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TypeFlags(i32);

impl TypeFlags {
    pub const NONE: TypeFlags = TypeFlags(0);
    pub const INSTANTIABLE: TypeFlags = TypeFlags(1);
    pub const INSTANCE: TypeFlags = TypeFlags(2);
    pub const CALLABLE: TypeFlags = TypeFlags(4);
    pub const LITERAL: TypeFlags = TypeFlags(8);
    pub const INTERFACE: TypeFlags = TypeFlags(16);
    pub const GENERIC: TypeFlags = TypeFlags(32);
    pub const FROM_ALIAS: TypeFlags = TypeFlags(64);
    
    pub fn new() -> Self {
        TypeFlags(0)
    }
    
    pub fn with_instantiable(mut self) -> Self {
        self.0 |= Self::INSTANTIABLE.0;
        self
    }
    
    pub fn with_instance(mut self) -> Self {
        self.0 |= Self::INSTANCE.0;
        self
    }
    
    pub fn with_callable(mut self) -> Self {
        self.0 |= Self::CALLABLE.0;
        self
    }
    
    pub fn with_literal(mut self) -> Self {
        self.0 |= Self::LITERAL.0;
        self
    }
    
    pub fn with_from_alias(mut self) -> Self {
        self.0 |= Self::FROM_ALIAS.0;
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModuleName {
    #[serde(rename = "leadingDots")]
    pub leading_dots: i32,
    #[serde(rename = "nameParts")]
    pub name_parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub uri: String,
    pub start: i32,
    pub length: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub name: String,
    #[serde(rename = "type")]
    pub type_info: Type,
    pub owner: Option<Type>,
    #[serde(rename = "boundType")]
    pub bound_type: Option<Type>,
    pub flags: AttributeFlags,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct DeclarationCategory(i32);

impl DeclarationCategory {
    pub const INTRINSIC: DeclarationCategory = DeclarationCategory(0);
    pub const VARIABLE: DeclarationCategory = DeclarationCategory(1);
    pub const PARAM: DeclarationCategory = DeclarationCategory(2);
    pub const TYPE_PARAM: DeclarationCategory = DeclarationCategory(3);
    pub const TYPE_ALIAS: DeclarationCategory = DeclarationCategory(4);
    pub const FUNCTION: DeclarationCategory = DeclarationCategory(5);
    pub const CLASS: DeclarationCategory = DeclarationCategory(6);
    pub const IMPORT: DeclarationCategory = DeclarationCategory(7);
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct DeclarationFlags(i32);

impl DeclarationFlags {
    pub const NONE: DeclarationFlags = DeclarationFlags(0);
    pub const CLASS_MEMBER: DeclarationFlags = DeclarationFlags(1 << 0);   // Method defined within a class
    pub const CONSTANT: DeclarationFlags = DeclarationFlags(1 << 1);       // Variable that cannot be changed
    pub const FINAL: DeclarationFlags = DeclarationFlags(1 << 2);          // Final variable/class
    pub const IS_DEFINED_BY_SLOTS: DeclarationFlags = DeclarationFlags(1 << 3); // Class uses __slots__
    pub const USES_LOCAL_NAME: DeclarationFlags = DeclarationFlags(1 << 4);     // Import uses 'as' alias
    pub const UNRESOLVED_IMPORT: DeclarationFlags = DeclarationFlags(1 << 5);   // Import is unresolved
    
    pub fn new() -> Self {
        DeclarationFlags(0)
    }
    
    pub fn with_class_member(mut self) -> Self {
        self.0 |= Self::CLASS_MEMBER.0;
        self
    }
    
    pub fn with_constant(mut self) -> Self {
        self.0 |= Self::CONSTANT.0;
        self
    }
    
    pub fn with_final(mut self) -> Self {
        self.0 |= Self::FINAL.0;
        self
    }
    
    pub fn with_defined_by_slots(mut self) -> Self {
        self.0 |= Self::IS_DEFINED_BY_SLOTS.0;
        self
    }
    
    pub fn with_local_name(mut self) -> Self {
        self.0 |= Self::USES_LOCAL_NAME.0;
        self
    }
    
    pub fn with_unresolved_import(mut self) -> Self {
        self.0 |= Self::UNRESOLVED_IMPORT.0;
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Declaration {
    pub handle: TypeHandle, // Unique identifier for the declaration
    pub category: DeclarationCategory, // Category of the symbol
    pub flags: DeclarationFlags, // Extra information about the declaration
    pub node: Option<Node>, // Parse node associated with the declaration
    #[serde(rename = "moduleName")]
    pub module_name: ModuleName, // Dot-separated import name for the file
    pub name: String, // Symbol name as the user sees it
    pub uri: String, // File that contains the declaration
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub node: Node,
    pub name: String,
    pub decls: Vec<Declaration>,
    #[serde(rename = "synthesizedTypes")]
    pub synthesized_types: Vec<Type>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct AttributeFlags(i32);

impl AttributeFlags {
    pub const NONE: AttributeFlags = AttributeFlags(0);
    pub const IS_ARGS_LIST: AttributeFlags = AttributeFlags(1);
    pub const IS_KWARGS_DICT: AttributeFlags = AttributeFlags(2);
}

#[derive(Serialize, Deserialize)]
pub struct GetTypeParams {
    pub node: Node,           // Location in the file
    pub snapshot: i32,        // Snapshot version
}

#[derive(Serialize, Deserialize)]
pub struct GetSymbolParams {
    pub node: Node,           // Location in the file
    pub name: Option<String>, // Optional symbol name
    #[serde(rename = "skipUnreachableCode")]
    pub skip_unreachable_code: bool, // Whether to skip unreachable code
    pub snapshot: i32,        // Snapshot version
}

#[derive(Serialize, Deserialize)]
pub struct GetPythonSearchPathsParams {
    #[serde(rename = "fromUri")]
    pub from_uri: String,          // File URI to determine which config to use
    pub snapshot: i32,        // Snapshot version
}

#[derive(Serialize)]
pub struct GetSnapshotParams {
    // No parameters needed for getting snapshot, but we need an empty struct
    // to handle both {} and null parameter cases
}

impl Default for GetSnapshotParams {
    fn default() -> Self {
        Self {}
    }
}

// Custom deserializer that handles both null and empty object
impl<'de> serde::Deserialize<'de> for GetSnapshotParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct GetSnapshotParamsVisitor;

        impl<'de> Visitor<'de> for GetSnapshotParamsVisitor {
            type Value = GetSnapshotParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("null or an empty object")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(GetSnapshotParams {})
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(GetSnapshotParams {})
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                // Read any fields in the map but ignore them since we don't need any params
                while let Some((key, _value)) = map.next_entry::<String, serde_json::Value>()? {
                    // Ignore unknown fields for flexibility
                    let _ = key;
                }
                Ok(GetSnapshotParams {})
            }
        }

        deserializer.deserialize_any(GetSnapshotParamsVisitor)
    }
}

impl lsp_types::request::Request for GetTypeRequest {
    type Params = GetTypeParams;
    type Result = Type;
    const METHOD: &'static str = "typeServer/getType";
}

impl lsp_types::request::Request for GetSymbolRequest {
    type Params = GetSymbolParams;
    type Result = Symbol;
    const METHOD: &'static str = "typeServer/getSymbol";
}

impl lsp_types::request::Request for GetPythonSearchPathsRequest {
    type Params = GetPythonSearchPathsParams;
    type Result = Vec<String>;
    const METHOD: &'static str = "typeServer/getPythonSearchPaths";
}

impl lsp_types::request::Request for GetSnapshotRequest {
    type Params = GetSnapshotParams;
    type Result = i32;
    const METHOD: &'static str = "typeServer/getSnapshot";
}


pub fn convert_to_tsp_type(py_type: crate::types::types::Type) -> Type {
    use crate::types::types::Type as PyType;
    
    Type {
        handle: TypeHandle::String(format!("{:p}", &py_type as *const _)),
        category: match &py_type {
            PyType::Any(_) => TypeCategory::ANY,
            PyType::Function(_) | PyType::Callable(_) => TypeCategory::FUNCTION,
            PyType::Overload(_) => TypeCategory::OVERLOADED,
            PyType::ClassType(_) | PyType::ClassDef(_) => TypeCategory::CLASS,
            PyType::Module(_) => TypeCategory::MODULE,
            PyType::Union(_) => TypeCategory::UNION,
            PyType::TypeVar(_) => TypeCategory::TYPE_VAR,
            _ => TypeCategory::ANY,
        },
        flags: calculate_type_flags(&py_type),
        module_name: extract_module_name(&py_type),
        name: py_type.to_string(),
        category_flags: 0,
        decl: None,
    }
}

fn calculate_type_flags(py_type: &crate::types::types::Type) -> TypeFlags {
    use crate::types::types::Type as PyType;
    
    let mut flags = TypeFlags::new();
    
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

fn extract_module_name(py_type: &crate::types::types::Type) -> Option<ModuleName> {
    use crate::types::types::Type as PyType;
    
    match py_type {
        PyType::ClassType(ct) => Some(convert_module_name(&ct.qname().module_name())),
        PyType::ClassDef(cd) => Some(convert_module_name(&cd.qname().module_name())),
        PyType::Module(m) => Some(convert_module_name_from_string(&m.to_string())), // Fixed this line
        _ => None,
    }
}

fn convert_module_name(pyrefly_module: &pyrefly_python::module_name::ModuleName) -> ModuleName {
    ModuleName {
        leading_dots: 0, // pyrefly modules don't have leading dots in this context
        name_parts: pyrefly_module.as_str().split('.').map(|s| s.to_string()).collect(),
    }
}

// Add this new function to handle Module objects
fn convert_module_name_from_string(module_str: &str) -> ModuleName {
    ModuleName {
        leading_dots: 0,
        name_parts: module_str.split('.').map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_get_snapshot_params_deserialization() {
        // Test null case
        let null_json = serde_json::Value::Null;
        let result: Result<GetSnapshotParams, _> = serde_json::from_value(null_json);
        assert!(result.is_ok());

        // Test empty object case
        let empty_obj_json = serde_json::json!({});
        let result: Result<GetSnapshotParams, _> = serde_json::from_value(empty_obj_json);
        assert!(result.is_ok());

        // Test object with unknown fields (should be ignored)
        let obj_with_fields = serde_json::json!({"unknown_field": "value"});
        let result: Result<GetSnapshotParams, _> = serde_json::from_value(obj_with_fields);
        assert!(result.is_ok());
    }
}

// Request/Response types for resolveImportDeclaration
#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveImportOptions {
    #[serde(rename = "resolveLocalNames")]
    pub resolve_local_names: Option<bool>,
    #[serde(rename = "allowExternallyHiddenAccess")]
    pub allow_externally_hidden_access: Option<bool>,
    #[serde(rename = "skipFileNeededCheck")]
    pub skip_file_needed_check: Option<bool>,
}

impl Default for ResolveImportOptions {
    fn default() -> Self {
        Self {
            resolve_local_names: Some(false),
            allow_externally_hidden_access: Some(false),
            skip_file_needed_check: Some(false),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolveImportDeclarationParams {
    pub decl: Declaration,
    pub options: ResolveImportOptions,
    pub snapshot: i32,
}

// LSP request type for resolveImportDeclaration
pub enum ResolveImportDeclarationRequest {}

impl lsp_types::request::Request for ResolveImportDeclarationRequest {
    type Params = ResolveImportDeclarationParams;
    type Result = Option<Declaration>;
    const METHOD: &'static str = "typeServer/resolveImportDeclaration";
}

// Request/Response types for getTypeOfDeclaration
#[derive(Debug, Serialize, Deserialize)]
pub struct GetTypeOfDeclarationParams {
    pub decl: Declaration,
    pub snapshot: i32,
}

// LSP request type for getTypeOfDeclaration
pub enum GetTypeOfDeclarationRequest {}

impl lsp_types::request::Request for GetTypeOfDeclarationRequest {
    type Params = GetTypeOfDeclarationParams;
    type Result = Type;
    const METHOD: &'static str = "typeServer/getTypeOfDeclaration";
}

// Flags that control how type representations are formatted
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TypeReprFlags(i32);

impl TypeReprFlags {
    pub const NONE: TypeReprFlags = TypeReprFlags(0);
    pub const EXPAND_TYPE_ALIASES: TypeReprFlags = TypeReprFlags(1 << 0);
    pub const PRINT_TYPE_VAR_VARIANCE: TypeReprFlags = TypeReprFlags(1 << 1);
    pub const CONVERT_TO_INSTANCE_TYPE: TypeReprFlags = TypeReprFlags(1 << 2);
    
    pub fn has_expand_type_aliases(&self) -> bool {
        self.0 & Self::EXPAND_TYPE_ALIASES.0 != 0
    }
    
    pub fn has_print_type_var_variance(&self) -> bool {
        self.0 & Self::PRINT_TYPE_VAR_VARIANCE.0 != 0
    }
    
    pub fn has_convert_to_instance_type(&self) -> bool {
        self.0 & Self::CONVERT_TO_INSTANCE_TYPE.0 != 0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetReprParams {
    #[serde(rename = "type")]
    pub type_param: Type,
    pub flags: TypeReprFlags,
    pub snapshot: i32,
}

// LSP request type for getRepr
pub enum GetReprRequest {}

impl lsp_types::request::Request for GetReprRequest {
    type Params = GetReprParams;
    type Result = String;
    const METHOD: &'static str = "typeServer/getRepr";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDocstringParams {
    #[serde(rename = "type")]
    pub type_param: Option<Type>,
    pub decl: Declaration,
    #[serde(rename = "boundObjectOrClass")]
    pub bound_object_or_class: Option<Type>,
    pub snapshot: i32,
}

// LSP request type for getDocstring
pub enum GetDocstringRequest {}

impl lsp_types::request::Request for GetDocstringRequest {
    type Params = GetDocstringParams;
    type Result = Option<String>;
    const METHOD: &'static str = "typeServer/getDocstring";
}