// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

// ****** THIS IS A GENERATED FILE, DO NOT EDIT. ******
// Steps to generate:
// 1. Create tsp.json and tsp.schema.json from typeServerProtocol.ts
// 2. Install lsprotocol generator: `pip install git+https://github.com/microsoft/lsprotocol.git`
// 3. Run: `python generate_protocol.py`

use serde::{Serialize, Deserialize};

/// This type allows extending any string enum to support custom values.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum CustomStringEnum<T> {
    /// The value is one of the known enum values.
    Known(T),
    /// The value is custom.
    Custom(String),
}



/// This type allows extending any integer enum to support custom values.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum CustomIntEnum<T> {
    /// The value is one of the known enum values.
    Known(T),
    /// The value is custom.
    Custom(i32),
}



/// This allows a field to have two types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR2<T, U> {
    T(T),
    U(U),
}



/// This allows a field to have three types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR3<T, U, V> {
    T(T),
    U(U),
    V(V),
}



/// This allows a field to have four types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR4<T, U, V, W> {
    T(T),
    U(U),
    V(V),
    W(W),
}



/// This allows a field to have five types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR5<T, U, V, W, X> {
    T(T),
    U(U),
    V(V),
    W(W),
    X(X),
}



/// This allows a field to have six types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR6<T, U, V, W, X, Y> {
    T(T),
    U(U),
    V(V),
    W(W),
    X(X),
    Y(Y),
}



/// This allows a field to have seven types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum OR7<T, U, V, W, X, Y, Z> {
    T(T),
    U(U),
    V(V),
    W(W),
    X(X),
    Y(Y),
    Z(Z),
}



/// This allows a field to always have null or empty value.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum LSPNull {
    None,
}



#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
pub enum TSPRequestMethods{
    #[serde(rename = "typeServer/getSnapshot")]
    TypeServerGetSnapshot,
    #[serde(rename = "typeServer/getSupportedProtocolVersion")]
    TypeServerGetSupportedProtocolVersion,
    #[serde(rename = "typeServer/getDiagnostics")]
    TypeServerGetDiagnostics,
    #[serde(rename = "typeServer/getDiagnosticsVersion")]
    TypeServerGetDiagnosticsVersion,
    #[serde(rename = "typeServer/getType")]
    TypeServerGetType,
    #[serde(rename = "typeServer/getBuiltinType")]
    TypeServerGetBuiltinType,
    #[serde(rename = "typeServer/getTypeArgs")]
    TypeServerGetTypeArgs,
    #[serde(rename = "typeServer/searchForTypeAttribute")]
    TypeServerSearchForTypeAttribute,
    #[serde(rename = "typeServer/getTypeAttributes")]
    TypeServerGetTypeAttributes,
    #[serde(rename = "typeServer/getOverloads")]
    TypeServerGetOverloads,
    #[serde(rename = "typeServer/getMatchingOverloads")]
    TypeServerGetMatchingOverloads,
    #[serde(rename = "typeServer/getMetaclass")]
    TypeServerGetMetaclass,
    #[serde(rename = "typeServer/getTypeOfDeclaration")]
    TypeServerGetTypeOfDeclaration,
    #[serde(rename = "typeServer/getSymbol")]
    TypeServerGetSymbol,
    #[serde(rename = "typeServer/getSymbolsForFile")]
    TypeServerGetSymbolsForFile,
    #[serde(rename = "typeServer/getFunctionParts")]
    TypeServerGetFunctionParts,
    #[serde(rename = "typeServer/getRepr")]
    TypeServerGetRepr,
    #[serde(rename = "typeServer/getDocString")]
    TypeServerGetDocstring,
    #[serde(rename = "typeServer/resolveImportDeclaration")]
    TypeServerResolveImportDeclaration,
    #[serde(rename = "typeServer/resolveImport")]
    TypeServerResolveImport,
    #[serde(rename = "typeServer/getTypeAliasInfo")]
    TypeServerGetTypeAliasInfo,
    #[serde(rename = "typeServer/combineTypes")]
    TypeServerCombineTypes,
    #[serde(rename = "typeServer/createInstanceType")]
    TypeServerCreateInstanceType,
    #[serde(rename = "typeServer/getPythonSearchPaths")]
    TypeServerGetPythonSearchPaths,
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
pub enum TSPNotificationMethods{
    #[serde(rename = "typeServer/snapshotChanged")]
    TypeServerSnapshotChanged,
    #[serde(rename = "typeServer/diagnosticsChanged")]
    TypeServerDiagnosticsChanged,
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
pub enum MessageDirection{
    #[serde(rename = "clientToServer")]
    ClientToServer,
    #[serde(rename = "serverToClient")]
    ServerToClient,
}


/// Represents a category of a type, such as class, function, variable, etc.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum TypeCategory {
    /// Type can be anything
    Any = 0,
    
    /// Callable type
    Function = 1,
    
    /// Functions defined with @overload decorator
    Overloaded = 2,
    
    /// Class definition
    Class = 3,
    
    /// Module instance
    Module = 4,
    
    /// Union of two or more other types
    Union = 5,
    
    /// Type variable
    TypeVar = 6,
    
}
impl Serialize for TypeCategory {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
TypeCategory::Any => serializer.serialize_i32(0),
TypeCategory::Function => serializer.serialize_i32(1),
TypeCategory::Overloaded => serializer.serialize_i32(2),
TypeCategory::Class => serializer.serialize_i32(3),
TypeCategory::Module => serializer.serialize_i32(4),
TypeCategory::Union => serializer.serialize_i32(5),
TypeCategory::TypeVar => serializer.serialize_i32(6),
}
}
}
impl<'de> Deserialize<'de> for TypeCategory {
fn deserialize<D>(deserializer: D) -> Result<TypeCategory, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(TypeCategory::Any),
1 => Ok(TypeCategory::Function),
2 => Ok(TypeCategory::Overloaded),
3 => Ok(TypeCategory::Class),
4 => Ok(TypeCategory::Module),
5 => Ok(TypeCategory::Union),
6 => Ok(TypeCategory::TypeVar),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe the characteristics of a type. These flags can be combined using bitwise operations.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum TypeFlags {
    None = 0,
    
    /// Indicates if the type can be instantiated.
    Instantiable = 1,
    
    /// Indicates if the type represents an instance (as opposed to a class or type itself).
    Instance = 2,
    
    /// Indicates if an instance of the type can be called like a function.
    Callable = 4,
    
    /// Indicates if the instance is a literal (like `42`, `"hello"`, etc.).
    Literal = 8,
    
    /// Indicates if the type is an interface (a type that defines a set of methods and properties).
    Interface = 16,
    
    /// Indicates if the type is a generic type (a type that can be parameterized with other types).
    Generic = 32,
    
    /// Indicates if the type came from an alias (a type that refers to another type).
    FromAlias = 64,
    
}
impl Serialize for TypeFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
TypeFlags::None => serializer.serialize_i32(0),
TypeFlags::Instantiable => serializer.serialize_i32(1),
TypeFlags::Instance => serializer.serialize_i32(2),
TypeFlags::Callable => serializer.serialize_i32(4),
TypeFlags::Literal => serializer.serialize_i32(8),
TypeFlags::Interface => serializer.serialize_i32(16),
TypeFlags::Generic => serializer.serialize_i32(32),
TypeFlags::FromAlias => serializer.serialize_i32(64),
}
}
}
impl<'de> Deserialize<'de> for TypeFlags {
fn deserialize<D>(deserializer: D) -> Result<TypeFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(TypeFlags::None),
1 => Ok(TypeFlags::Instantiable),
2 => Ok(TypeFlags::Instance),
4 => Ok(TypeFlags::Callable),
8 => Ok(TypeFlags::Literal),
16 => Ok(TypeFlags::Interface),
32 => Ok(TypeFlags::Generic),
64 => Ok(TypeFlags::FromAlias),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe the characteristics of a function or method. These flags can be combined using bitwise operations.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum FunctionFlags {
    None = 0,
    
    /// Indicates if the function is asynchronous.
    Async = 1,
    
    /// Indicates if the function is a generator (can yield values).
    Generator = 2,
    
    /// Indicates if the function is abstract (must be implemented in a subclass).
    Abstract = 4,
    
    /// Indicates if the function has a @staticmethod decorator.
    Static = 8,
    
}
impl Serialize for FunctionFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
FunctionFlags::None => serializer.serialize_i32(0),
FunctionFlags::Async => serializer.serialize_i32(1),
FunctionFlags::Generator => serializer.serialize_i32(2),
FunctionFlags::Abstract => serializer.serialize_i32(4),
FunctionFlags::Static => serializer.serialize_i32(8),
}
}
}
impl<'de> Deserialize<'de> for FunctionFlags {
fn deserialize<D>(deserializer: D) -> Result<FunctionFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(FunctionFlags::None),
1 => Ok(FunctionFlags::Async),
2 => Ok(FunctionFlags::Generator),
4 => Ok(FunctionFlags::Abstract),
8 => Ok(FunctionFlags::Static),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe the characteristics of a class. These flags can be combined using bitwise operations.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum ClassFlags {
    None = 0,
    
    /// Indicates if the class is an enum (a special kind of class that defines a set of named values).
    Enum = 1,
    
    /// Indicates if the class is a TypedDict or derived from a TypedDict.
    TypedDict = 2,
    
}
impl Serialize for ClassFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
ClassFlags::None => serializer.serialize_i32(0),
ClassFlags::Enum => serializer.serialize_i32(1),
ClassFlags::TypedDict => serializer.serialize_i32(2),
}
}
}
impl<'de> Deserialize<'de> for ClassFlags {
fn deserialize<D>(deserializer: D) -> Result<ClassFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(ClassFlags::None),
1 => Ok(ClassFlags::Enum),
2 => Ok(ClassFlags::TypedDict),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe the characteristics of a type variable. These flags can be combined using bitwise operations.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum TypeVarFlags {
    None = 0,
    
    /// Indicates if the type variable is a ParamSpec (as defined in PEP 612).
    IsParamSpec = 1,
    
}
impl Serialize for TypeVarFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
TypeVarFlags::None => serializer.serialize_i32(0),
TypeVarFlags::IsParamSpec => serializer.serialize_i32(1),
}
}
}
impl<'de> Deserialize<'de> for TypeVarFlags {
fn deserialize<D>(deserializer: D) -> Result<TypeVarFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(TypeVarFlags::None),
1 => Ok(TypeVarFlags::IsParamSpec),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe extra data about an attribute.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum AttributeFlags {
    None = 0,
    
    /// Indicates if a parameter is an argument list (e.g., `*args`).
    IsArgsList = 1,
    
    /// Indicates if the attribute is a keyword argument dictionary (e.g., `**kwargs`).
    IsKwargsDict = 2,
    
}
impl Serialize for AttributeFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
AttributeFlags::None => serializer.serialize_i32(0),
AttributeFlags::IsArgsList => serializer.serialize_i32(1),
AttributeFlags::IsKwargsDict => serializer.serialize_i32(2),
}
}
}
impl<'de> Deserialize<'de> for AttributeFlags {
fn deserialize<D>(deserializer: D) -> Result<AttributeFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(AttributeFlags::None),
1 => Ok(AttributeFlags::IsArgsList),
2 => Ok(AttributeFlags::IsKwargsDict),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that are used for searching for attributes of a class Type.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum AttributeAccessFlags {
    None = 0,
    
    /// Skip instance attributes when searching for attributes of a type.
    SkipInstanceAttributes = 1,
    
    /// Skip members from the base class of a type when searching for members of a type.
    SkipTypeBaseClass = 2,
    
    /// Skip attribute access overrides when searching for members of a type.
    SkipAttributeAccessOverrides = 4,
    
    /// Look for bound attributes when searching for attributes of a type.
    GetBoundAttributes = 8,
    
}
impl Serialize for AttributeAccessFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
AttributeAccessFlags::None => serializer.serialize_i32(0),
AttributeAccessFlags::SkipInstanceAttributes => serializer.serialize_i32(1),
AttributeAccessFlags::SkipTypeBaseClass => serializer.serialize_i32(2),
AttributeAccessFlags::SkipAttributeAccessOverrides => serializer.serialize_i32(4),
AttributeAccessFlags::GetBoundAttributes => serializer.serialize_i32(8),
}
}
}
impl<'de> Deserialize<'de> for AttributeAccessFlags {
fn deserialize<D>(deserializer: D) -> Result<AttributeAccessFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(AttributeAccessFlags::None),
1 => Ok(AttributeAccessFlags::SkipInstanceAttributes),
2 => Ok(AttributeAccessFlags::SkipTypeBaseClass),
4 => Ok(AttributeAccessFlags::SkipAttributeAccessOverrides),
8 => Ok(AttributeAccessFlags::GetBoundAttributes),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Represents the category of a declaration in the type system.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum DeclarationCategory {
    /// An intrinsic refers to a symbol that has no actual declaration in the source code, such as built-in types or functions.
    Intrinsic = 0,
    
    /// A variable is a named storage location that can hold a value.
    Variable = 1,
    
    /// A parameter is a variable that is passed to a function or method.
    Param = 2,
    
    /// This is for PEP 695 type parameters.
    TypeParam = 3,
    
    /// This is for PEP 695 type aliases.
    TypeAlias = 4,
    
    /// A function is any construct that begins with the `def` keyword and has a body, which can be called with arguments.
    Function = 5,
    
    /// A class is any construct that begins with the `class` keyword and has a body, which can be instantiated.
    Class = 6,
    
    /// An import declaration, which is a reference to another module.
    Import = 7,
    
}
impl Serialize for DeclarationCategory {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
DeclarationCategory::Intrinsic => serializer.serialize_i32(0),
DeclarationCategory::Variable => serializer.serialize_i32(1),
DeclarationCategory::Param => serializer.serialize_i32(2),
DeclarationCategory::TypeParam => serializer.serialize_i32(3),
DeclarationCategory::TypeAlias => serializer.serialize_i32(4),
DeclarationCategory::Function => serializer.serialize_i32(5),
DeclarationCategory::Class => serializer.serialize_i32(6),
DeclarationCategory::Import => serializer.serialize_i32(7),
}
}
}
impl<'de> Deserialize<'de> for DeclarationCategory {
fn deserialize<D>(deserializer: D) -> Result<DeclarationCategory, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(DeclarationCategory::Intrinsic),
1 => Ok(DeclarationCategory::Variable),
2 => Ok(DeclarationCategory::Param),
3 => Ok(DeclarationCategory::TypeParam),
4 => Ok(DeclarationCategory::TypeAlias),
5 => Ok(DeclarationCategory::Function),
6 => Ok(DeclarationCategory::Class),
7 => Ok(DeclarationCategory::Import),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that describe extra information about a declaration.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum DeclarationFlags {
    None = 0,
    
    /// Indicates if the declaration is a method (a function defined within a class).
    ClassMember = 1,
    
    /// Indicates if the declaration is a constant (a variable that cannot be changed).
    Constant = 2,
    
    /// Indicates if the declaration is final variable (a class that cannot be subclassed).
    Final = 4,
    
    /// Indicates if the declaration is defined by slots (a class that uses __slots__).
    IsDefinedBySlots = 8,
    
    /// Indicates if the import declaration uses 'as' with a different name.
    UsesLocalName = 16,
    
    /// Indicates if the import declaration is unresolved (the module or symbol could not be found).
    UnresolvedImport = 32,
    
}
impl Serialize for DeclarationFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
DeclarationFlags::None => serializer.serialize_i32(0),
DeclarationFlags::ClassMember => serializer.serialize_i32(1),
DeclarationFlags::Constant => serializer.serialize_i32(2),
DeclarationFlags::Final => serializer.serialize_i32(4),
DeclarationFlags::IsDefinedBySlots => serializer.serialize_i32(8),
DeclarationFlags::UsesLocalName => serializer.serialize_i32(16),
DeclarationFlags::UnresolvedImport => serializer.serialize_i32(32),
}
}
}
impl<'de> Deserialize<'de> for DeclarationFlags {
fn deserialize<D>(deserializer: D) -> Result<DeclarationFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(DeclarationFlags::None),
1 => Ok(DeclarationFlags::ClassMember),
2 => Ok(DeclarationFlags::Constant),
4 => Ok(DeclarationFlags::Final),
8 => Ok(DeclarationFlags::IsDefinedBySlots),
16 => Ok(DeclarationFlags::UsesLocalName),
32 => Ok(DeclarationFlags::UnresolvedImport),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Flags that control how type representations are formatted.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum TypeReprFlags {
    None = 0,
    
    /// Turn type aliases into their original type.
    ExpandTypeAliases = 1,
    
    /// Print the variance of a type parameter.
    PrintTypeVarVariance = 2,
    
    /// Convert the type into an instance type before printing it.
    ConvertToInstanceType = 4,
    
}
impl Serialize for TypeReprFlags {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer,{
match self {
TypeReprFlags::None => serializer.serialize_i32(0),
TypeReprFlags::ExpandTypeAliases => serializer.serialize_i32(1),
TypeReprFlags::PrintTypeVarVariance => serializer.serialize_i32(2),
TypeReprFlags::ConvertToInstanceType => serializer.serialize_i32(4),
}
}
}
impl<'de> Deserialize<'de> for TypeReprFlags {
fn deserialize<D>(deserializer: D) -> Result<TypeReprFlags, D::Error> where D: serde::Deserializer<'de>,{
let value = i32::deserialize(deserializer)?;
match value {
0 => Ok(TypeReprFlags::None),
1 => Ok(TypeReprFlags::ExpandTypeAliases),
2 => Ok(TypeReprFlags::PrintTypeVarVariance),
4 => Ok(TypeReprFlags::ConvertToInstanceType),
_ => Err(serde::de::Error::custom("Unexpected value"))
}
}
}


/// Unique identifier for a type definition within the snapshot. A handle doesn't need to exist beyond the lifetime of the snapshot.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum TypeHandle
{
    String(String),
    Int(i32),
}



/// Unique identifier for a declaration within the session.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum DeclarationHandle
{
    String(String),
    Int(i32),
}



/// Position in a text document expressed as zero-based line and character offset.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Position
{
    /// Character offset on a line in a document (zero-based).
    pub character: u32,
    
    /// Line position in a document (zero-based).
    pub line: u32,
    
}



/// A range in a text document expressed as (zero-based) start and end positions.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Range
{
    /// The range's end position.
    pub end: Position,
    
    /// The range's start position.
    pub start: Position,
    
}



/// Represents a diagnostic, such as a compiler error or warning.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Diagnostic
{
    /// The diagnostic's code.
    pub code: Option<OR2<i32, String>>,
    
    /// The diagnostic's message.
    pub message: String,
    
    /// The range at which the message applies.
    pub range: Range,
    
    /// The diagnostic's severity.
    pub severity: Option<i32>,
    
    /// A human-readable string describing the source of this diagnostic.
    pub source: Option<String>,
    
}



/// Represents a node in an AST (Abstract Syntax Tree) or similar structure.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node
{
    /// The range of the node in the source file. This is a zero-based range, meaning the start and end positions are both zero-based. The range uses character offsets the same way the LSP does.
    pub range: Range,
    
    /// URI of the source file containing this node.
    pub uri: String,
    
}



/// Represents a module name with optional leading dots for relative imports.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleName
{
    /// The leading dots in the module name. This is used to determine the relative import level.
    pub leading_dots: i32,
    
    /// The parts of the module name, split by dots. For example, for `my_module.sub_module`, this would be `['my_module', 'sub_module']`.
    pub name_parts: Vec<String>,
    
}



/// Represents a type in the type system.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Type
{
    /// The typing module defines aliases for builtin types (e.g. Tuple, List, Dict). This field holds the alias name.
    pub alias_name: Option<String>,
    
    /// Essential classification of the Type.
    pub category: TypeCategory,
    
    /// Flags specific to the category. For example, for a class type, this would be ClassFlags. For a function type, this would be FunctionFlags.
    pub category_flags: i32,
    
    /// Declaration of the type, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decl: Option<Declaration>,
    
    /// Flags describing the type.
    pub flags: TypeFlags,
    
    /// Unique identifier for the type definition within the snapshot. A handle doesn't need to exist beyond the lifetime of the snapshot.
    pub handle: TypeHandle,
    
    /// Name of the module the type comes from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_name: Option<ModuleName>,
    
    /// Simple name of the type. For example, for a class `MyClass` in module `my_module`, this would be `MyClass`.
    pub name: String,
    
}



/// Represents an attribute of a type (e.g., a field, method, or parameter).
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Attribute
{
    /// The type the attribute is bound to, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_type: Option<Type>,
    
    /// The declarations for the attribute.
    pub decls: Vec<Declaration>,
    
    /// Flags describing extra data about an attribute.
    pub flags: i32,
    
    /// The name of the attribute. This is the name used to access the attribute in code.
    pub name: String,
    
    /// The type the attribute came from (can be a class, function, module, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Type>,
    
    /// The type of the attribute.
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Represents a symbol declaration in the type system.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Declaration
{
    /// Category of this symbol (function, variable, etc.).
    pub category: DeclarationCategory,
    
    /// Extra information about the declaration.
    pub flags: DeclarationFlags,
    
    /// Unique identifier for the declaration within the session.
    pub handle: DeclarationHandle,
    
    /// The dot-separated import name for the file that contains the declaration.
    pub module_name: ModuleName,
    
    /// The symbol name for the declaration (as the user sees it)
    pub name: String,
    
    /// Parse node associated with the declaration
    pub node: Option<Node>,
    
    /// The file that contains the declaration. Unless this is an import declaration, then the uri refers to the file the import is referring to.
    pub uri: String,
    
}



/// Symbol information for a node, which includes a list of declarations and potentially synthesized types for those declarations.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Symbol
{
    /// The declarations for the symbol. This can include multiple declarations for the same symbol.
    pub decls: Vec<Declaration>,
    
    /// The name of the symbol found.
    pub name: String,
    
    /// The node for which the declaration information is being requested.
    pub node: Node,
    
    /// Synthesized type information for a declaration that is not directly represented in the source code, but is derived from the declaration.
    pub synthesized_types: Vec<Type>,
    
}



/// Contains symbol information for an entire file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileSymbolInfo
{
    /// The symbols in the file.
    pub symbols: Vec<Symbol>,
    
    /// The URI of the source file.
    pub uri: String,
    
}



/// Options for resolving an import declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportOptions
{
    /// Whether to allow access to members that are hidden by external modules.
    pub allow_externally_hidden_access: Option<bool>,
    
    /// Whether to resolve local names in the import declaration.
    pub resolve_local_names: Option<bool>,
    
    /// Whether to skip checking if the file is needed for the import resolution.
    pub skip_file_needed_check: Option<bool>,
    
}



/// Parameters for resolving an import
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportParams
{
    /// The descriptor of the imported module.
    pub module_descriptor: ModuleName,
    
    /// The snapshot version.
    pub snapshot: i32,
    
    /// The URI of the source file where the import is referenced.
    pub source_uri: String,
    
}



/// Parameters for searching for a type attribute.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchForTypeAttributeParams
{
    /// Flags that control how the attribute is accessed.
    pub access_flags: AttributeAccessFlags,
    
    /// The name of the attribute being requested.
    pub attribute_name: String,
    
    /// Optional: The expression node that the member is being accessed from.
    pub expression_node: Option<Node>,
    
    /// Optional: The type of an instance that the attribute is being accessed from.
    pub instance_type: Option<Type>,
    
    /// The snapshot version of the type server state.
    pub snapshot: i32,
    
    /// The starting point in the type hierarchy to search for the attribute.
    pub start_type: Type,
    
}



/// Parameters for getting type attributes.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAttributesParams
{
    /// The snapshot version of the type server state.
    pub snapshot: i32,
    
    /// The type for which the attributes are being requested.
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting symbol information.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolParams
{
    /// The name of the symbol being requested. This is optional and can be undefined especially when the node is a name node.
    pub name: Option<String>,
    
    /// The node for which the symbol information is being requested.
    pub node: Node,
    
    /// Whether to skip unreachable code when looking for the symbol declaration.
    pub skip_unreachable_code: bool,
    
    /// The snapshot version of the type server state.
    pub snapshot: i32,
    
}



/// Parameters for getting builtin type information.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetBuiltinTypeParams
{
    /// The name of the builtin type being requested.
    pub name: String,
    
    /// The node that is used to scope the builtin type. Every module may have a different set of builtins based on where the module is located.
    pub scoping_node: Node,
    
    /// The snapshot version of the type server state.
    pub snapshot: i32,
    
}



/// Parts of a function, including its parameters and return type. This is used to provide a string representation of a function's signature.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FunctionParts
{
    /// The function parameters as strings.
    pub params: Vec<String>,
    
    /// The return type as a string.
    pub return_type: String,
    
}



/// Information about a type alias.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TypeAliasInfo
{
    /// The original name of the alias.
    pub name: String,
    
    /// The arguments for the type alias, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_args: Option<Vec<Type>>,
    
}



/// Parameters for getting diagnostics for a file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The URI of the file to get diagnostics for
    pub uri: String,
    
}



/// Parameters for getting diagnostics version for a file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsVersionParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The URI of the file
    pub uri: String,
    
}



/// Parameters for getting type information for a node.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeParams
{
    /// The node to get type information for
    pub node: Node,
    
    /// The snapshot version
    pub snapshot: i32,
    
}



/// Parameters for getting type arguments.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeArgsParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to get arguments for
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting function overloads.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetOverloadsParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to get overloads for
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting matching overloads for a call.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMatchingOverloadsParams
{
    /// The call node to get matching overloads for
    pub call_node: Node,
    
    /// The snapshot version
    pub snapshot: i32,
    
}



/// Parameters for getting metaclass of a type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMetaclassParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to get metaclass for
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting type of a declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeOfDeclarationParams
{
    /// The declaration to get type for
    pub decl: Declaration,
    
    /// The snapshot version
    pub snapshot: i32,
    
}



/// Parameters for getting symbols for a file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolsForFileParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The URI of the file
    pub uri: String,
    
}



/// Parameters for getting function parts.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetFunctionPartsParams
{
    /// Formatting flags
    pub flags: TypeReprFlags,
    
    /// The snapshot version
    pub snapshot: i32,
    
    /// The function type
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting type representation.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetReprParams
{
    /// Formatting flags
    pub flags: TypeReprFlags,
    
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to get representation for
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting documentation string.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDocstringParams
{
    /// The bound object or class type
    pub bound_object_or_class: Option<Type>,
    
    /// The declaration to get documentation for
    pub decl: Declaration,
    
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type context
    #[serde(rename = "type")]
    pub type_: Option<Type>,
    
}



/// Parameters for resolving import declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportDeclarationParams
{
    /// The import declaration to resolve
    pub decl: Declaration,
    
    /// Resolution options
    pub options: ResolveImportOptions,
    
    /// The snapshot version
    pub snapshot: i32,
    
}



/// Parameters for getting type alias information.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAliasInfoParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to get alias info for
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for combining types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CombineTypesParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The types to combine
    pub types: Vec<Type>,
    
}



/// Parameters for creating instance type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstanceTypeParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The type to create instance from
    #[serde(rename = "type")]
    pub type_: Type,
    
}



/// Parameters for getting Python search paths.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetPythonSearchPathsParams
{
    /// The URI to get search paths from
    pub from_uri: String,
    
    /// The snapshot version
    pub snapshot: i32,
    
}



/// Parameters for snapshot changed notification.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotChangedParams
{
    /// The new snapshot version
    pub new: i32,
    
    /// The old snapshot version
    pub old: i32,
    
}



/// Parameters for diagnostics changed notification.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticsChangedParams
{
    /// The snapshot version
    pub snapshot: i32,
    
    /// The URI of the file with changed diagnostics
    pub uri: String,
    
    /// The diagnostics version
    pub version: i32,
    
}



/// Notification sent by the server to indicate any outstanding snapshots are invalid.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotChangedNotification
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPNotificationMethods,
    
    pub params: Option<serde_json::Value>,
    
}



/// Notification sent by the server to indicate that diagnostics have changed and the client should re-request diagnostics for the file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticsChangedNotification
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPNotificationMethods,
    
    pub params: Option<serde_json::Value>,
    
}



/// An identifier to denote a specific request.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum LSPId
{
    Int(i32),
    String(String),
}



/// An identifier to denote a specific response.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(untagged)]
pub enum LSPIdOptional
{
    Int(i32),
    String(String),
    None,
}



/// Request from client to get the current snapshot of the type server. A snapshot is a point-in-time representation of the type server's state, including all loaded files and their types.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSnapshotRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<LSPNull>,
    
}



/// Response to the [GetSnapshotRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSnapshotResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    pub result: i32,
    
}



/// Request to get the version of the protocol the type server supports. Returns a string representation of the protocol version (should be semver format).
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSupportedProtocolVersionRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<LSPNull>,
    
}



/// Response to the [GetSupportedProtocolVersionRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSupportedProtocolVersionResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    pub result: String,
    
}



/// Request to get diagnostics for a specific file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetDiagnosticsParams,
    
}



/// Response to the [GetDiagnosticsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Diagnostic>>,
    
}



/// Request to get the version of diagnostics for a specific file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsVersionRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetDiagnosticsVersionParams,
    
}



/// Response to the [GetDiagnosticsVersionRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDiagnosticsVersionResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<i32>,
    
}



/// Request to get the type information for a specific node.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetTypeParams,
    
}



/// Response to the [GetTypeRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to get the type information for a specific builtin type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetBuiltinTypeRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetBuiltinTypeParams,
    
}



/// Response to the [GetBuiltinTypeRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetBuiltinTypeResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to get the collection of subtypes that make up a union type or the types that makes up a generic type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeArgsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetTypeArgsParams,
    
}



/// Response to the [GetTypeArgsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeArgsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Type>>,
    
}



/// Request to find an attribute of a class.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchForTypeAttributeRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: SearchForTypeAttributeParams,
    
}



/// Response to the [SearchForTypeAttributeRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchForTypeAttributeResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Attribute>,
    
}



/// Request to get the attributes of a specific class or the parameters and return value of a specific function.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAttributesRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetTypeAttributesParams,
    
}



/// Response to the [GetTypeAttributesRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAttributesResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Attribute>>,
    
}



/// Request to get all overloads of a function or method. The returned value doesn't include the implementation signature.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetOverloadsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetOverloadsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetOverloadsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Type>>,
    
}



/// Request to get the overloads that a call node matches.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMatchingOverloadsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetMatchingOverloadsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMatchingOverloadsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Type>>,
    
}



/// Request to get the meta class of a type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMetaclassRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetMetaclassRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetMetaclassResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to get the type of a declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeOfDeclarationRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetTypeOfDeclarationRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeOfDeclarationResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to get symbol declaration information for a node.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: GetSymbolParams,
    
}



/// Response to the [GetSymbolRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Symbol>,
    
}



/// Request to get all symbols for a file. This is used to get all symbols in a file.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolsForFileRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetSymbolsForFileRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSymbolsForFileResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<FileSymbolInfo>,
    
}



/// Request to get the string representation of a function's parts, meaning its parameters and return type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetFunctionPartsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetFunctionPartsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetFunctionPartsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<FunctionParts>,
    
}



/// Request to get the string representation of a type in a human-readable format. This may or may not be the same as the type's "name".
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetReprRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetReprRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetReprResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
}



/// Request to get the docstring for a specific declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDocstringRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetDocstringRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDocstringResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
}



/// Request to resolve an import declaration.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportDeclarationRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [ResolveImportDeclarationRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportDeclarationResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Declaration>,
    
}



/// Request to resolve an import. This is used to resolve the import name to its location in the file system.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: ResolveImportParams,
    
}



/// Response to the [ResolveImportRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveImportResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    
}



/// Get information about a type alias.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAliasInfoRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetTypeAliasInfoRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetTypeAliasInfoResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<TypeAliasInfo>,
    
}



/// Request to combine types. This is used to combine multiple types into a single type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CombineTypesRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [CombineTypesRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CombineTypesResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to generate an instance type representation for the provided type.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstanceTypeRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [CreateInstanceTypeRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInstanceTypeResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Type>,
    
}



/// Request to get the search paths that the type server uses for Python modules.
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetPythonSearchPathsRequest
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPId,
    
    pub params: Option<serde_json::Value>,
    
}



/// Response to the [GetPythonSearchPathsRequest].
#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetPythonSearchPathsResponse
{
    /// The version of the JSON RPC protocol.
    pub jsonrpc: String,
    
    /// The method to be invoked.
    pub method: TSPRequestMethods,
    
    /// The request id.
    pub id: LSPIdOptional,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<String>>,
    
}




// Type Server Protocol Constants

/// The version of the Type Server Protocol
pub const TypeServerVersion: &str = "0.1.0";

/// Represents an invalid handle value
pub const InvalidHandle: i32 = -1;

