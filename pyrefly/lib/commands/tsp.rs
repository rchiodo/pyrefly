use serde::{Deserialize, Serialize};

pub enum GetTypeRequest {}

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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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
    pub end: i32,
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
pub struct GetPythonSearchPathsParams {
    pub uri: String,          // File URI to determine which config to use
    pub snapshot: i32,        // Snapshot version
}

#[derive(Serialize, Deserialize)]
pub struct GetSnapshotParams {
    // No parameters needed for getting snapshot
}

impl lsp_types::request::Request for GetTypeRequest {
    type Params = GetTypeParams;
    type Result = Type;
    const METHOD: &'static str = "typeServer/getType";
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

fn convert_module_name(pyrefly_module: &crate::module::module_name::ModuleName) -> ModuleName {
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