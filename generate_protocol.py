#!/usr/bin/env python3
"""
Unified Protocol Generator for pyrefly TSP

This script generates a complete protocol.rs file from TypeScript protocol definitions.
It creates all necessary structures, enums, and type definitions from scratch.
"""

import re
import sys
import json
from pathlib import Path
from typing import Dict, List, Set, Optional, Tuple
from dataclasses import dataclass

@dataclass
class TypeScriptField:
    name: str
    type_annotation: str
    optional: bool = False
    readonly: bool = False
    serde_rename: Optional[str] = None

@dataclass 
class TypeScriptInterface:
    name: str
    fields: List[TypeScriptField]
    extends: List[str] = None

@dataclass
class TypeScriptEnum:
    name: str
    values: List[Tuple[str, Optional[str]]]  # (name, value)
    is_const: bool = False
    
@dataclass
class RequestInfo:
    name: str
    params_type: str
    result_type: str

class ComprehensiveTypeScriptParser:
    """Complete TypeScript parser that extracts all protocol elements"""
    
    def __init__(self):
        self.interfaces: Dict[str, TypeScriptInterface] = {}
        self.enums: Dict[str, TypeScriptEnum] = {}
        self.requests: List[RequestInfo] = []
        self.type_aliases: Dict[str, str] = {}
        
    def parse_file(self, file_path: str) -> None:
        """Parse a TypeScript file and extract all relevant structures"""
        content = Path(file_path).read_text(encoding='utf-8')
        
        # Clean up content - remove comments and extra whitespace
        content = self._clean_content(content)
        
        # Parse different elements
        self._parse_interfaces(content)
        self._parse_enums(content)
        self._parse_type_aliases(content)
        self._parse_requests(content)
        self._add_derived_types()
        
    def _clean_content(self, content: str) -> str:
        """Remove comments and normalize whitespace"""
        # Remove single-line comments
        content = re.sub(r'//.*$', '', content, flags=re.MULTILINE)
        
        # Remove multi-line comments
        content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
        
        return content
    
    def _parse_interfaces(self, content: str) -> None:
        """Parse TypeScript interfaces"""
        # Pattern to match interface definitions including those in namespaces
        interface_pattern = r'(?:export\s+)?interface\s+(\w+)(?:\s+extends\s+([^{]+))?\s*\{([^}]+)\}'
        
        for match in re.finditer(interface_pattern, content, re.DOTALL):
            name = match.group(1)
            extends_clause = match.group(2)
            body = match.group(3)
            
            # Parse extends clause
            extends = []
            if extends_clause:
                extends = [ext.strip() for ext in extends_clause.split(',')]
            
            # Parse fields
            fields = self._parse_interface_fields(body)
            
            self.interfaces[name] = TypeScriptInterface(
                name=name,
                fields=fields,
                extends=extends
            )
    
    def _parse_interface_fields(self, body: str) -> List[TypeScriptField]:
        """Parse fields from interface body"""
        fields = []
        
        # Split by semicolons and newlines, but be careful about nested types
        field_lines = []
        current_line = ""
        brace_count = 0
        
        for char in body:
            current_line += char
            if char == '{':
                brace_count += 1
            elif char == '}':
                brace_count -= 1
            elif char in ';\n' and brace_count == 0:
                if current_line.strip():
                    field_lines.append(current_line.strip())
                current_line = ""
        
        if current_line.strip():
            field_lines.append(current_line.strip())
        
        for line in field_lines:
            line = line.rstrip(';,').strip()
            if not line or line.startswith('//'):
                continue
                
            field = self._parse_field_line(line)
            if field:
                fields.append(field)
        
        return fields
    
    def _parse_field_line(self, line: str) -> Optional[TypeScriptField]:
        """Parse a single field line"""
        # Handle readonly modifier
        readonly = False
        if line.startswith('readonly '):
            readonly = True
            line = line[9:]
        
        # Pattern for field: name?: type or name: type
        field_pattern = r'^(\w+)(\?)?\s*:\s*(.+)$'
        match = re.match(field_pattern, line)
        
        if not match:
            return None
        
        name = match.group(1)
        optional = match.group(2) == '?'
        type_annotation = match.group(3).strip()
        
        # Determine serde rename if needed
        serde_rename = None
        if self._needs_serde_rename(name):
            serde_rename = name
            name = self._to_snake_case(name)
        
        return TypeScriptField(
            name=name,
            type_annotation=type_annotation,
            optional=optional,
            readonly=readonly,
            serde_rename=serde_rename
        )
    
    def _needs_serde_rename(self, name: str) -> bool:
        """Check if field needs serde rename (camelCase to snake_case)"""
        return any(c.isupper() for c in name[1:])  # Has uppercase after first char
    
    def _to_snake_case(self, name: str) -> str:
        """Convert camelCase to snake_case"""
        result = []
        for i, c in enumerate(name):
            if i > 0 and c.isupper():
                result.append('_')
            result.append(c.lower())
        return ''.join(result)
    
    def _parse_enums(self, content: str) -> None:
        """Parse TypeScript enums"""
        # Pattern for both regular enums and const enums
        enum_pattern = r'export\s+(?:const\s+)?enum\s+(\w+)\s*\{([^}]+)\}'
        
        for match in re.finditer(enum_pattern, content):
            name = match.group(1)
            body = match.group(2)
            is_const = 'const enum' in match.group(0)
            
            values = self._parse_enum_values(body)
            self.enums[name] = TypeScriptEnum(name=name, values=values, is_const=is_const)
    
    def _parse_enum_values(self, body: str) -> List[Tuple[str, Optional[str]]]:
        """Parse enum values"""
        values = []
        
        # Split by both commas and newlines for better parsing
        lines = []
        for line in body.replace(',', '\n').split('\n'):
            line = line.strip()
            if line and not line.startswith('//'):
                lines.append(line)
        
        for line in lines:
            line = line.strip().rstrip(',')
            if not line:
                continue
            
            # Handle NAME = value or just NAME
            if '=' in line:
                name, value = line.split('=', 1)
                name = name.strip()
                value = value.strip().strip('"\'')
                values.append((name, value))
            else:
                values.append((line.strip(), None))
        
        return values
    
    def _parse_type_aliases(self, content: str) -> None:
        """Parse TypeScript type aliases"""
        alias_pattern = r'export\s+type\s+(\w+)\s*=\s*([^;]+);'
        
        for match in re.finditer(alias_pattern, content):
            name = match.group(1)
            type_def = match.group(2).strip()
            self.type_aliases[name] = type_def
    
    def _parse_requests(self, content: str) -> None:
        """Parse request method signatures and inline parameter types"""
        # Look for ProtocolRequestType patterns with inline types
        request_pattern = r'export\s+namespace\s+(\w+).*?new\s+ProtocolRequestType(?:0)?<([^>]+)>'
        
        for match in re.finditer(request_pattern, content, re.DOTALL):
            namespace_name = match.group(1)
            type_params = match.group(2)
            
            # Extract parameter type (first type parameter)
            param_type = type_params.split(',')[0].strip()
            
            # Convert namespace name to request name
            request_name = namespace_name if namespace_name.endswith('Request') else namespace_name + 'Request'
            
            # If parameter type is inline object type, create interface
            if param_type.startswith('{') and param_type.endswith('}'):
                # Extract fields from inline type
                fields = self._parse_inline_type_fields(param_type)
                
                # Create parameter interface name
                params_interface_name = namespace_name.replace('Request', '') + 'Params'
                
                # Create interface
                self.interfaces[params_interface_name] = TypeScriptInterface(
                    name=params_interface_name,
                    fields=fields
                )
                
                self.requests.append(RequestInfo(
                    name=request_name,
                    params_type=params_interface_name,
                    result_type='unknown'
                ))
            else:
                self.requests.append(RequestInfo(
                    name=request_name,
                    params_type=param_type,
                    result_type='unknown'
                ))
    
    def _parse_inline_type_fields(self, inline_type: str) -> List[TypeScriptField]:
        """Parse fields from inline object type like { uri: string; snapshot: number }"""
        # Remove braces and split by semicolon
        body = inline_type.strip('{}').strip()
        fields = []
        
        for field_def in body.split(';'):
            field_def = field_def.strip()
            if not field_def:
                continue
                
            field = self._parse_field_line(field_def)
            if field:
                fields.append(field)
        
        return fields
    
    def _add_derived_types(self) -> None:
        """Add derived types that are needed but not explicitly defined in TypeScript"""
        # Add common parameter structures that might be missing
        if 'GetSnapshotParams' not in self.interfaces:
            self.interfaces['GetSnapshotParams'] = TypeScriptInterface(
                name='GetSnapshotParams',
                fields=[]  # Empty struct
            )
        
        if 'GetSupportedProtocolVersionParams' not in self.interfaces:
            self.interfaces['GetSupportedProtocolVersionParams'] = TypeScriptInterface(
                name='GetSupportedProtocolVersionParams',
                fields=[]  # Empty struct
            )

class ComprehensiveRustGenerator:
    """Generate complete Rust protocol code"""
    
    def __init__(self):
        self.type_mapping = self._create_type_mapping()
        self.builtin_enums = self._create_builtin_enums()
    
    def _create_type_mapping(self) -> Dict[str, str]:
        """Create TypeScript to Rust type mapping"""
        return {
            'string': 'String',
            'number': 'i32',
            'boolean': 'bool',
            'string[]': 'Vec<String>',
            'number[]': 'Vec<i32>',
            'any': 'serde_json::Value',
            'object': 'serde_json::Value',
            'void': '()',
            'undefined': 'Option<()>',
            # VSCode Language Server Protocol types mapped to lsp_types
            'Range': 'lsp_types::Range',
            'Diagnostic': 'lsp_types::Diagnostic',
            'Url': 'lsp_types::Url',
            'CancellationToken': 'lsp_types::CancellationToken',
            'Disposable': 'serde_json::Value',  # Generic for now
            'RequestHandler': 'serde_json::Value',  # Generic for now
            'NotificationHandler': 'serde_json::Value',  # Generic for now
            'MessageDirection': 'serde_json::Value',  # From vscode-languageserver-protocol
            'ProtocolNotificationType': 'serde_json::Value',
            'ProtocolRequestType': 'serde_json::Value',
            'ProtocolRequestType0': 'serde_json::Value',
            # Protocol-specific types
            'Node': 'Node',
            'Type': 'Type',
            'ModuleName': 'ModuleName',
            'Declaration': 'Declaration',
            'Declaration[]': 'Vec<Declaration>',
            'Diagnostic[]': 'Vec<lsp_types::Diagnostic>',
            'Type[]': 'Vec<Type>',
            'Symbol': 'Symbol',
            'Symbol[]': 'Vec<Symbol>',
            'Attribute': 'Attribute',
            'Attribute[]': 'Vec<Attribute>',
            'AttributeAccessFlags': 'AttributeAccessFlags',
            'TypeReprFlags': 'TypeReprFlags',
            'FunctionFlags': 'FunctionFlags',
            'Settings': 'serde_json::Value',
            'FunctionParts': 'FunctionParts',
            'TypeAliasInfo': 'TypeAliasInfo',
            'FileSymbolInfo': 'FileSymbolInfo',
            'ResolveImportOptions': 'ResolveImportOptions',
        }
    
    def _create_builtin_enums(self) -> Dict[str, List[Tuple[str, int]]]:
        """Define built-in enums that need specific values"""
        return {
            'AttributeFlags': [
                ('NONE', 0),
                ('IS_ARGS_LIST', 1),
                ('IS_KWARGS_DICT', 2),
                ('PARAMETER', 4),
                ('RETURN_TYPE', 8),
            ]
        }
    
    def generate_protocol(self, parser: ComprehensiveTypeScriptParser) -> str:
        """Generate complete protocol.rs content"""
        sections = []
        
        # Header with imports
        sections.append(self._generate_header())
        
        # Constants
        sections.append(self._generate_constants())
        
        # Request enums
        sections.append(self._generate_request_enums(parser.requests))
        
        # Core types (Type, TypeHandle, etc.)
        sections.append(self._generate_core_types())
        
        # Generated enums
        sections.append(self._generate_enums(parser.enums))
        
        # Generated interfaces/structs
        sections.append(self._generate_structs(parser.interfaces))
        
        return '\n\n'.join(sections)
    
    def _generate_header(self) -> str:
        """Generate file header with imports"""
        return '''use lsp_types::Diagnostic;
use lsp_types::Range;
use lsp_types::Url;
use serde::Deserialize;
use serde::Serialize;

// Generated TSP Protocol structures
// This file is auto-generated from TypeScript protocol definitions'''
    
    def _generate_constants(self) -> str:
        """Generate protocol constants"""
        return '''// TSP Protocol Version
pub const TSP_PROTOCOL_VERSION: &str = "0.1.0";'''
    
    def _generate_request_enums(self, requests: List[RequestInfo]) -> str:
        """Generate request enum declarations"""
        lines = []
        lines.append("// Request type markers")
        
        request_names = set()
        for request in requests:
            if request.name not in request_names:
                lines.append(f"pub enum {request.name} {{}}")
                request_names.add(request.name)
        
        # Add common request types that might be missing
        common_requests = [
            'GetTypeRequest', 'GetSymbolRequest', 'GetPythonSearchPathsRequest',
            'GetSnapshotRequest', 'GetSupportedProtocolVersionRequest', 'GetDiagnosticsRequest',
            'GetBuiltinTypeRequest', 'GetTypeAttributesRequest', 'GetSymbolsForFileRequest',
            'GetMetaclassRequest', 'GetTypeAliasInfoRequest', 'CombineTypesRequest',
            'CreateInstanceTypeRequest', 'GetDocstringRequest', 'ResolveImportRequest',
            'ResolveImportDeclarationRequest', 'GetTypeOfDeclarationRequest',
            'GetReprRequest', 'SearchForTypeAttributeRequest', 'GetFunctionPartsRequest',
            'GetDiagnosticsVersionRequest', 'GetTypeArgsRequest', 'GetOverloadsRequest',
            'GetMatchingOverloadsRequest'
        ]
        
        for req_name in common_requests:
            if req_name not in request_names:
                lines.append(f"pub enum {req_name} {{}}")
        
        return '\n'.join(lines)
    
    def _generate_core_types(self) -> str:
        """Generate core protocol types"""
        return '''// Core protocol types
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
    #[serde(rename = "aliasName")]
    pub alias_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TypeHandle {
    String(String),
    Integer(i32),
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
    pub range: Range,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Declaration {
    pub handle: TypeHandle,
    pub category: DeclarationCategory,
    pub flags: DeclarationFlags,
    pub node: Option<Node>,
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
pub struct Attribute {
    pub name: String,
    #[serde(rename = "type")]
    pub type_info: Type,
    pub owner: Option<Type>,
    #[serde(rename = "boundType")]
    pub bound_type: Option<Type>,
    pub flags: AttributeFlags,
    pub decls: Vec<Declaration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSymbolInfo {
    pub uri: String,
    pub symbols: Vec<Symbol>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypeAliasInfo {
    pub name: String,
    #[serde(rename = "typeArgs")]
    pub type_args: Option<Vec<Type>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionParts {
    pub params: Vec<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveImportOptions {
    #[serde(rename = "resolveLocalNames")]
    pub resolve_local_names: Option<bool>,
    #[serde(rename = "allowExternallyHiddenAccess")]
    pub allow_externally_hidden_access: Option<bool>,
    #[serde(rename = "skipFileNeededCheck")]
    pub skip_file_needed_check: Option<bool>,
}'''
    
    def _generate_enums(self, enums: Dict[str, TypeScriptEnum]) -> str:
        """Generate enum definitions"""
        lines = []
        lines.append("// Enum definitions")
        
        # First add built-in enums with specific values (these take priority)
        for enum_name, values in self.builtin_enums.items():
            lines.append(self._generate_builtin_enum(enum_name, values))
        
        # Then add other enums from TypeScript (skip if already defined)
        for enum_name, enum_def in enums.items():
            if enum_name not in self.builtin_enums:
                lines.append(self._generate_enum(enum_def))
        
        return '\n\n'.join(lines)
    
    def _generate_enum(self, enum_def: TypeScriptEnum) -> str:
        """Generate a single enum"""
        lines = []
        lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]")
        lines.append(f"pub struct {enum_def.name}(i32);")
        lines.append("")
        lines.append(f"impl {enum_def.name} {{")
        
        for i, (name, value) in enumerate(enum_def.values):
            if value is not None:
                # Handle bit shift expressions like "1 << 0"
                if '<<' in value:
                    try:
                        left, right = value.split('<<')
                        left_val = int(left.strip())
                        right_val = int(right.strip())
                        computed_value = left_val << right_val
                        lines.append(f"    pub const {name}: {enum_def.name} = {enum_def.name}({computed_value});")
                    except (ValueError, IndexError):
                        lines.append(f"    pub const {name}: {enum_def.name} = {enum_def.name}({i});")
                elif value.isdigit():
                    lines.append(f"    pub const {name}: {enum_def.name} = {enum_def.name}({value});")
                else:
                    lines.append(f"    pub const {name}: {enum_def.name} = {enum_def.name}({i});")
            else:
                lines.append(f"    pub const {name}: {enum_def.name} = {enum_def.name}({i});")
        
        lines.append("}")
        
        return '\n'.join(lines)
    
    def _generate_builtin_enum(self, enum_name: str, values: List[Tuple[str, int]]) -> str:
        """Generate a built-in enum with specific values"""
        lines = []
        lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone, Copy)]")
        lines.append(f"pub struct {enum_name}(i32);")
        lines.append("")
        lines.append(f"impl {enum_name} {{")
        
        for name, value in values:
            lines.append(f"    pub const {name}: {enum_name} = {enum_name}({value});")
        
        lines.append("}")
        
        return '\n'.join(lines)
    
    def _generate_structs(self, interfaces: Dict[str, TypeScriptInterface]) -> str:
        """Generate struct definitions"""
        lines = []
        lines.append("// Parameter and data structures")
        
        for interface_name, interface_def in interfaces.items():
            lines.append(self._generate_struct(interface_def))
        
        return '\n\n'.join(lines)
    
    def _generate_struct(self, interface_def: TypeScriptInterface) -> str:
        """Generate a single struct"""
        lines = []
        
        # Determine derive attributes based on struct type
        if not interface_def.fields:  # Empty struct
            lines.append("#[derive(Serialize, Deserialize, Default)]")
        else:
            lines.append("#[derive(Serialize, Deserialize, Debug, Clone)]")
        
        lines.append(f"pub struct {interface_def.name} {{")
        
        for field in interface_def.fields:
            field_line = self._generate_field(field)
            lines.append(f"    {field_line}")
        
        lines.append("}")
        
        return '\n'.join(lines)
    
    def _generate_field(self, field: TypeScriptField) -> str:
        """Generate a single field definition"""
        parts = []
        
        # Add serde rename if needed
        if field.serde_rename:
            parts.append(f'#[serde(rename = "{field.serde_rename}")]')
        
        # Generate field declaration
        rust_type = self._map_type(field.type_annotation)
        
        if field.optional:
            rust_type = f"Option<{rust_type}>"
        
        field_decl = f"pub {field.name}: {rust_type},"
        parts.append(field_decl)
        
        return '\n    '.join(parts)
    
    def _map_type(self, ts_type: str) -> str:
        """Map TypeScript type to Rust type"""
        # Handle array types
        if ts_type.endswith('[]'):
            element_type = ts_type[:-2]
            mapped_element = self.type_mapping.get(element_type, element_type)
            return f"Vec<{mapped_element}>"
        
        # Handle union types (take first type for now)
        if '|' in ts_type:
            ts_type = ts_type.split('|')[0].strip()
        
        # Handle generic types like Array<Type>
        generic_match = re.match(r'(\w+)<(.+)>', ts_type)
        if generic_match:
            container = generic_match.group(1)
            inner_type = generic_match.group(2)
            if container == 'Array':
                mapped_inner = self.type_mapping.get(inner_type, inner_type)
                return f"Vec<{mapped_inner}>"
        
        # Direct mapping
        return self.type_mapping.get(ts_type, ts_type)

def main():
    # Use the updated TypeScript file path
    typescript_file = r"C:\Users\rchiodo\source\repos\pyrx-2\packages\type-server\src\protocol\typeServerProtocol.ts"
    
    if not Path(typescript_file).exists():
        print(f"Error: TypeScript file {typescript_file} not found")
        sys.exit(1)
    
    print(f"Parsing TypeScript file: {typescript_file}")
    
    # Parse TypeScript
    parser = ComprehensiveTypeScriptParser()
    parser.parse_file(typescript_file)
    
    print(f"Found {len(parser.interfaces)} interfaces, {len(parser.enums)} enums, {len(parser.requests)} requests")
    
    # Generate complete protocol
    generator = ComprehensiveRustGenerator()
    rust_code = generator.generate_protocol(parser)
    
    # Write output directly to protocol.rs
    output_file = "pyrefly/lib/tsp/protocol.rs"
    Path(output_file).write_text(rust_code, encoding='utf-8')
    
    print(f"Protocol written to: {output_file}")
    print(f"Generated {len(rust_code.splitlines())} lines of Rust code")
    
    # Show summary
    print("\nGenerated structures:")
    print(f"  Interfaces: {len(parser.interfaces)}")
    print(f"  Enums: {len(parser.enums)}")
    print(f"  Requests: {len(parser.requests)}")
    
    print("\nGeneration complete. Run 'cargo check' to verify compilation.")

if __name__ == "__main__":
    main()
