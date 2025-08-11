use lsp_types::Diagnostic;
use lsp_types::Range;
use lsp_types::Url;
use serde::Deserialize;
use serde::Serialize;

// Re-export common utilities
pub use super::common::*;

// Type alias for string | number union
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TypeHandle {
    String(String),
    Number(i32),
}

// TSP Protocol Version
pub const TSP_PROTOCOL_VERSION: &str = "0.1.0";

pub const RETURN_ATTRIBUTE_NAME: &str = "__return__";
pub const INVALID_HANDLE: i32 = -1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetSnapshotParams {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug)]
pub struct GetSnapshotRequest;

#[derive(Debug)]
pub struct GetDiagnosticsRequest;

#[derive(Debug)]
pub struct GetDiagnosticsVersionRequest;

#[derive(Debug)]
pub struct GetTypeRequest;

#[derive(Debug)]
pub struct GetBuiltinTypeRequest;

#[derive(Debug)]
pub struct GetTypeArgsRequest;

#[derive(Debug)]
pub struct SearchForTypeAttributeRequest;

#[derive(Debug)]
pub struct GetTypeAttributesRequest;

#[derive(Debug)]
pub struct GetOverloadsRequest;

#[derive(Debug)]
pub struct GetMatchingOverloadsRequest;

#[derive(Debug)]
pub struct GetMetaclassRequest;

#[derive(Debug)]
pub struct GetTypeOfDeclarationRequest;

#[derive(Debug)]
pub struct GetSymbolRequest;

#[derive(Debug)]
pub struct GetSymbolsForFileRequest;

#[derive(Debug)]
pub struct GetFunctionPartsRequest;

#[derive(Debug)]
pub struct GetReprRequest;

#[derive(Debug)]
pub struct GetDocStringRequest;

#[derive(Debug)]
pub struct ResolveImportDeclarationRequest;

#[derive(Debug)]
pub struct ResolveImportRequest;

#[derive(Debug)]
pub struct GetTypeAliasInfoRequest;

#[derive(Debug)]
pub struct CombineTypesRequest;

#[derive(Debug)]
pub struct CreateInstanceTypeRequest;

#[derive(Debug)]
pub struct GetPythonSearchPathsRequest;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const INSTANTIABLE: TypeFlags = TypeFlags(1  <<  0);
    pub const INSTANCE: TypeFlags = TypeFlags(1  <<  1);
    pub const CALLABLE: TypeFlags = TypeFlags(1  <<  2);
    pub const LITERAL: TypeFlags = TypeFlags(1  <<  3);
    pub const INTERFACE: TypeFlags = TypeFlags(1  <<  4);
    pub const GENERIC: TypeFlags = TypeFlags(1  <<  5);
    pub const FROM_ALIAS: TypeFlags = TypeFlags(1  <<  6);

    pub fn new() -> Self {
        TypeFlags(0)
    }

    pub fn has(self, flag: TypeFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: TypeFlags) -> Self {
        TypeFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct FunctionFlags(i32);

impl FunctionFlags {
    pub const NONE: FunctionFlags = FunctionFlags(0);
    pub const ASYNC: FunctionFlags = FunctionFlags(1  <<  0);
    pub const GENERATOR: FunctionFlags = FunctionFlags(1  <<  1);
    pub const ABSTRACT: FunctionFlags = FunctionFlags(1  <<  2);
    pub const STATIC: FunctionFlags = FunctionFlags(1  <<  3);

    pub fn new() -> Self {
        FunctionFlags(0)
    }

    pub fn has(self, flag: FunctionFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: FunctionFlags) -> Self {
        FunctionFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ClassFlags(i32);

impl ClassFlags {
    pub const NONE: ClassFlags = ClassFlags(0);
    pub const ENUM: ClassFlags = ClassFlags(1  <<  0);
    pub const TYPED_DICT: ClassFlags = ClassFlags(1  <<  1);

    pub fn new() -> Self {
        ClassFlags(0)
    }

    pub fn has(self, flag: ClassFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: ClassFlags) -> Self {
        ClassFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TypeVarFlags(i32);

impl TypeVarFlags {
    pub const NONE: TypeVarFlags = TypeVarFlags(0);
    pub const IS_PARAM_SPEC: TypeVarFlags = TypeVarFlags(1  <<  0);

    pub fn new() -> Self {
        TypeVarFlags(0)
    }

    pub fn has(self, flag: TypeVarFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: TypeVarFlags) -> Self {
        TypeVarFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct AttributeFlags(i32);

impl AttributeFlags {
    pub const NONE: AttributeFlags = AttributeFlags(0);
    pub const IS_ARGS_LIST: AttributeFlags = AttributeFlags(1  <<  0);
    pub const IS_KWARGS_DICT: AttributeFlags = AttributeFlags(1  <<  1);

    pub fn new() -> Self {
        AttributeFlags(0)
    }

    pub fn has(self, flag: AttributeFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: AttributeFlags) -> Self {
        AttributeFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct AttributeAccessFlags(i32);

impl AttributeAccessFlags {
    pub const NONE: AttributeAccessFlags = AttributeAccessFlags(0);
    pub const SKIP_INSTANCE_ATTRIBUTES: AttributeAccessFlags = AttributeAccessFlags(1  <<  0);
    pub const SKIP_TYPE_BASE_CLASS: AttributeAccessFlags = AttributeAccessFlags(1  <<  1);
    pub const SKIP_ATTRIBUTE_ACCESS_OVERRIDES: AttributeAccessFlags = AttributeAccessFlags(1  <<  2);
    pub const GET_BOUND_ATTRIBUTES: AttributeAccessFlags = AttributeAccessFlags(1  <<  3);

    pub fn new() -> Self {
        AttributeAccessFlags(0)
    }

    pub fn has(self, flag: AttributeAccessFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: AttributeAccessFlags) -> Self {
        AttributeAccessFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const CLASS_MEMBER: DeclarationFlags = DeclarationFlags(1  <<  0);
    pub const CONSTANT: DeclarationFlags = DeclarationFlags(1  <<  1);
    pub const FINAL: DeclarationFlags = DeclarationFlags(1  <<  2);
    pub const IS_DEFINED_BY_SLOTS: DeclarationFlags = DeclarationFlags(1  <<  3);
    pub const USES_LOCAL_NAME: DeclarationFlags = DeclarationFlags(1  <<  4);
    pub const UNRESOLVED_IMPORT: DeclarationFlags = DeclarationFlags(1  <<  5);

    pub fn new() -> Self {
        DeclarationFlags(0)
    }

    pub fn has(self, flag: DeclarationFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: DeclarationFlags) -> Self {
        DeclarationFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct TypeReprFlags(i32);

impl TypeReprFlags {
    pub const NONE: TypeReprFlags = TypeReprFlags(0);
    pub const EXPAND_TYPE_ALIASES: TypeReprFlags = TypeReprFlags(1  <<  0);
    pub const PRINT_TYPE_VAR_VARIANCE: TypeReprFlags = TypeReprFlags(1  <<  1);
    pub const CONVERT_TO_INSTANCE_TYPE: TypeReprFlags = TypeReprFlags(1  <<  2);

    pub fn new() -> Self {
        TypeReprFlags(0)
    }

    pub fn has(self, flag: TypeReprFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn with(self, flag: TypeReprFlags) -> Self {
        TypeReprFlags(self.0 | flag.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub uri: String,
    pub range: Range,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModuleName {
    #[serde(rename = "leadingDots")]
    pub leading_dots: i32,
    #[serde(rename = "nameParts")]
    pub name_parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Type {
    #[serde(rename = "aliasName")]
    pub alias_name: Option<String>,
    pub handle: TypeHandle,
    pub category: TypeCategory,
    pub flags: TypeFlags,
    #[serde(rename = "moduleName")]
    pub module_name: Option<ModuleName>,
    pub name: String,
    #[serde(rename = "categoryFlags")]
    pub category_flags: i32,
    pub decl: Option<Declaration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: Type,
    pub owner: Option<Type>,
    #[serde(rename = "boundType")]
    pub bound_type: Option<Type>,
    pub flags: i32,
    pub decls: Vec<Declaration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Declaration {
    pub node: Option<Node>,
    pub handle: TypeHandle,
    pub category: DeclarationCategory,
    pub flags: DeclarationFlags,
    #[serde(rename = "moduleName")]
    pub module_name: ModuleName,
    pub name: String,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub node: Node,
    pub name: String,
    pub decls: Vec<Declaration>,
    #[serde(rename = "synthesizedTypes")]
    pub synthesized_types: Vec<Type>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSymbolInfo {
    pub uri: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveImportOptions {
    #[serde(rename = "resolveLocalNames")]
    pub resolve_local_names: Option<bool>,
    #[serde(rename = "allowExternallyHiddenAccess")]
    pub allow_externally_hidden_access: Option<bool>,
    #[serde(rename = "skipFileNeededCheck")]
    pub skip_file_needed_check: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextDocumentOpenParams {
    #[serde(rename = "chainedFileUri")]
    pub chained_file_uri: Option<String>,
    pub uri: String,
    pub text: String,
    pub version: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextDocumentCloseParams {
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveImportParams {
    #[serde(rename = "sourceUri")]
    pub source_uri: String,
    #[serde(rename = "moduleDescriptor")]
    pub module_descriptor: ModuleName,
    pub snapshot: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchForTypeAttributeParams {
    #[serde(rename = "expressionNode")]
    pub expression_node: Option<Node>,
    #[serde(rename = "instanceType")]
    pub instance_type: Option<Type>,
    #[serde(rename = "startType")]
    pub start_type: Type,
    #[serde(rename = "attributeName")]
    pub attribute_name: String,
    #[serde(rename = "accessFlags")]
    pub access_flags: AttributeAccessFlags,
    pub snapshot: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetTypeAttributesParams {
    #[serde(rename = "type")]
    pub type_: Type,
    pub snapshot: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetSymbolParams {
    pub name: Option<String>,
    pub node: Node,
    #[serde(rename = "skipUnreachableCode")]
    pub skip_unreachable_code: bool,
    pub snapshot: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetBuiltinTypeParams {
    #[serde(rename = "scopingNode")]
    pub scoping_node: Node,
    pub name: String,
    pub snapshot: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypeAliasInfo {
    pub name: String,
    #[serde(rename = "typeArgs")]
    pub type_args: Option<Vec<Type>>,
}

impl lsp_types::request::Request for GetSnapshotRequest {
    type Params = i32;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetSnapshotRequest";
}

impl lsp_types::request::Request for GetDiagnosticsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetDiagnosticsRequest";
}

impl lsp_types::request::Request for GetDiagnosticsVersionRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetDiagnosticsVersionRequest";
}

impl lsp_types::request::Request for GetTypeRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetTypeRequest";
}

impl lsp_types::request::Request for GetBuiltinTypeRequest {
    type Params = GetBuiltinTypeParams;
    type Result = Option<Type>;
    const METHOD: &'static str = "typeServer/GetBuiltinTypeRequest";
}

impl lsp_types::request::Request for GetTypeArgsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetTypeArgsRequest";
}

impl lsp_types::request::Request for SearchForTypeAttributeRequest {
    type Params = SearchForTypeAttributeParams;
    type Result = Option<Attribute>;
    const METHOD: &'static str = "typeServer/SearchForTypeAttributeRequest";
}

impl lsp_types::request::Request for GetTypeAttributesRequest {
    type Params = GetTypeAttributesParams;
    type Result = Option<Vec<Attribute>>;
    const METHOD: &'static str = "typeServer/GetTypeAttributesRequest";
}

impl lsp_types::request::Request for GetOverloadsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetOverloadsRequest";
}

impl lsp_types::request::Request for GetMatchingOverloadsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetMatchingOverloadsRequest";
}

impl lsp_types::request::Request for GetMetaclassRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetMetaclassRequest";
}

impl lsp_types::request::Request for GetTypeOfDeclarationRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetTypeOfDeclarationRequest";
}

impl lsp_types::request::Request for GetSymbolRequest {
    type Params = GetSymbolParams;
    type Result = Option<Symbol>;
    const METHOD: &'static str = "typeServer/GetSymbolRequest";
}

impl lsp_types::request::Request for GetSymbolsForFileRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetSymbolsForFileRequest";
}

impl lsp_types::request::Request for GetFunctionPartsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetFunctionPartsRequest";
}

impl lsp_types::request::Request for GetReprRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetReprRequest";
}

impl lsp_types::request::Request for GetDocStringRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetDocStringRequest";
}

impl lsp_types::request::Request for ResolveImportDeclarationRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/ResolveImportDeclarationRequest";
}

impl lsp_types::request::Request for ResolveImportRequest {
    type Params = ResolveImportParams;
    type Result = Option<String>;
    const METHOD: &'static str = "typeServer/ResolveImportRequest";
}

impl lsp_types::request::Request for GetTypeAliasInfoRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetTypeAliasInfoRequest";
}

impl lsp_types::request::Request for CombineTypesRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/CombineTypesRequest";
}

impl lsp_types::request::Request for CreateInstanceTypeRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/CreateInstanceTypeRequest";
}

impl lsp_types::request::Request for GetPythonSearchPathsRequest {
    type Params = serde_json::Value;
    type Result = serde_json::Value;
    const METHOD: &'static str = "typeServer/GetPythonSearchPathsRequest";
}
