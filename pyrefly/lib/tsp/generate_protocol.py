#!/usr/bin/env python3
"""
Generate Rust protocol definitions from TypeScript protocol files.
This script parses TypeScript interface definitions and generates corresponding Rust structs.
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

@dataclass
class EnumField:
    """Represents a field in an enum."""
    name: str
    value: Optional[str] = None

@dataclass
class StructField:
    """Represents a field in a struct."""
    name: str
    type_: str
    optional: bool = False
    rename: Optional[str] = None

@dataclass
class EnumDefinition:
    """Represents an enum definition."""
    name: str
    fields: List[EnumField]

@dataclass
class InterfaceDefinition:
    """Represents an interface definition."""
    name: str
    fields: List[StructField]
    extends: Optional[str] = None

@dataclass
class RequestDefinition:
    """Represents a request definition."""
    name: str
    method: str
    params_type: Optional[str] = None
    result_type: Optional[str] = None

class TypeScriptParser:
    def __init__(self):
        self.enums: List[EnumDefinition] = []
        self.interfaces: List[InterfaceDefinition] = []
        self.requests: List[RequestDefinition] = []
        self.constants: Dict[str, str] = {}
        self.type_aliases: Dict[str, str] = {}

    def parse_file(self, file_path: str):
        """Parse a TypeScript file and extract type definitions."""
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        self._parse_content(content)

    def _parse_content(self, content: str):
        """Parse TypeScript content."""
        # Remove comments and normalize whitespace
        content = self._remove_comments(content)
        
        # Parse enums
        self._parse_enums(content)
        
        # Parse interfaces
        self._parse_interfaces(content)
        
        # Parse request definitions (special pattern)
        self._parse_requests(content)

    def _remove_comments(self, content: str) -> str:
        """Remove TypeScript comments."""
        # Remove single-line comments
        content = re.sub(r'//.*$', '', content, flags=re.MULTILINE)
        
        # Remove multi-line comments
        content = re.sub(r'/\*.*?\*/', '', content, flags=re.DOTALL)
        
        return content

    def _parse_enums(self, content: str):
        """Parse enum definitions."""
        enum_pattern = r'export\s+(?:const\s+)?enum\s+(\w+)\s*\{([^}]+)\}'
        
        for match in re.finditer(enum_pattern, content, re.MULTILINE | re.DOTALL):
            enum_name = match.group(1)
            enum_body = match.group(2)
            
            fields = []
            for field_match in re.finditer(r'(\w+)\s*=\s*([^,\n}]+)', enum_body):
                field_name = field_match.group(1)
                field_value = field_match.group(2).strip().rstrip(',')
                fields.append(EnumField(field_name, field_value))
            
            # If no explicit values found, parse simple enum fields
            if not fields:
                for field_match in re.finditer(r'(\w+)\s*(?:,|\s*$)', enum_body):
                    field_name = field_match.group(1)
                    fields.append(EnumField(field_name))
            
            self.enums.append(EnumDefinition(enum_name, fields))

    def _parse_interfaces(self, content: str):
        """Parse interface definitions."""
        # Match export interface with optional extends clause
        interface_pattern = r'export\s+interface\s+(\w+)(?:\s+extends\s+([^{]+))?\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}'
        
        for match in re.finditer(interface_pattern, content, re.MULTILINE | re.DOTALL):
            interface_name = match.group(1)
            extends = match.group(2).strip() if match.group(2) else None
            interface_body = match.group(3)
            
            fields = self._parse_interface_fields(interface_body)
            self.interfaces.append(InterfaceDefinition(interface_name, fields, extends))

    def _parse_interface_fields(self, body: str) -> List[StructField]:
        """Parse fields within an interface body."""
        fields = []
        
        # Handle simple field patterns
        field_patterns = [
            r'(\w+)\s*\?\s*:\s*([^;,\n}]+)',  # optional field
            r'(\w+)\s*:\s*([^;,\n}]+)',       # required field
        ]
        
        for pattern in field_patterns:
            for field_match in re.finditer(pattern, body):
                field_name = field_match.group(1)
                field_type = field_match.group(2).strip().rstrip(',;')
                optional = '?' in field_match.group(0)
                
                # Skip if already found (optional pattern matches required too)
                if any(f.name == field_name for f in fields):
                    continue
                
                fields.append(StructField(field_name, field_type, optional))
        
        return fields

    def _parse_requests(self, content: str):
        """Parse request definitions from TypeScript protocols."""
        # Look for patterns like: export namespace GetSnapshotRequest
        namespace_pattern = r'export\s+namespace\s+(\w+)\s*\{([^}]+(?:\{[^}]*\}[^}]*)*)\}'
        
        for match in re.finditer(namespace_pattern, content, re.MULTILINE | re.DOTALL):
            namespace_name = match.group(1)
            namespace_body = match.group(2)
            
            # Extract method name
            method_match = re.search(r'method:\s*[\'"]([^\'"]+)[\'"]', namespace_body)
            method = method_match.group(1) if method_match else f"typeServer/{namespace_name}"
            
            # Extract param and result types
            params_type = None
            result_type = None
            
            type_match = re.search(r'ProtocolRequestType(?:0)?<([^>]+)>', namespace_body)
            if type_match:
                params = [p.strip() for p in type_match.group(1).split(',')]
                if len(params) >= 1 and params[0] != 'never' and params[0] != 'void':
                    params_type = params[0]
                if len(params) >= 2 and params[1] != 'never' and params[1] != 'void':
                    result_type = params[1]
            
            # Only add if this looks like a request (ends with "Request")
            if namespace_name.endswith('Request'):
                self.requests.append(RequestDefinition(namespace_name, method, params_type, result_type))

class RustCodeGenerator:
    def __init__(self, parser: TypeScriptParser):
        self.parser = parser
        self.type_mappings = {
            'string': 'String',
            'number': 'i32',
            'boolean': 'bool',
            'Range': 'Range',
            'Url': 'Url',
            'Diagnostic': 'Diagnostic',
            'Diagnostic[]': 'Vec<Diagnostic>',
            'string[]': 'Vec<String>',
            'string | number': 'TypeHandle',
            'Type[]': 'Vec<Type>',
            'Declaration[]': 'Vec<Declaration>',
            'Attribute[]': 'Vec<Attribute>',
            'Symbol[]': 'Vec<Symbol>',
        }
    
    def generate_protocol_rs(self) -> str:
        """Generate the complete protocol.rs file."""
        lines = []
        
        # Add file header
        lines.extend(self._generate_header())
        
        # Add constants
        lines.extend(self._generate_constants())
        
        # Add request type definitions
        lines.extend(self._generate_request_types())
        
        # Add type definitions
        lines.extend(self._generate_enums())
        lines.extend(self._generate_structs())
        
        # Add request implementations
        lines.extend(self._generate_request_implementations())
        
        return '\n'.join(lines)
    
    def _generate_header(self) -> List[str]:
        """Generate the file header with imports."""
        return [
            "use lsp_types::Diagnostic;",
            "use lsp_types::Range;",
            "use lsp_types::Url;",
            "use serde::Deserialize;",
            "use serde::Serialize;",
            "",
            "// Re-export common utilities",
            "pub use super::common::*;",
            "",
            "// Type alias for string | number union",
            "#[derive(Serialize, Deserialize, Debug, Clone)]",
            "#[serde(untagged)]",
            "pub enum TypeHandle {",
            "    String(String),",
            "    Number(i32),",
            "}",
            "",
        ]
    
    def _generate_constants(self) -> List[str]:
        """Generate constant definitions."""
        lines = []
        
        # Add protocol version
        if 'TypeServerVersion' in [enum.name for enum in self.parser.enums]:
            version_enum = next(enum for enum in self.parser.enums if enum.name == 'TypeServerVersion')
            for field in version_enum.fields:
                if field.name == 'current':
                    version = field.value.strip('\'"')
                    lines.append(f'// TSP Protocol Version')
                    lines.append(f'pub const TSP_PROTOCOL_VERSION: &str = "{version}";')
                    lines.append('')
        
        # Add other constants
        lines.append('pub const RETURN_ATTRIBUTE_NAME: &str = "__return__";')
        lines.append('pub const INVALID_HANDLE: i32 = -1;')
        lines.append('')
        
        return lines
    
    def _generate_request_types(self) -> List[str]:
        """Generate request type definitions."""
        lines = []
        
        # Collect interface names to avoid duplicates
        interface_names = {interface.name for interface in self.parser.interfaces}
        
        # Generate parameter structs for requests that have complex parameters
        # but only if there's no corresponding interface already
        for request in self.parser.requests:
            if request.params_type and request.params_type not in ['void', 'null', 'undefined']:
                param_struct_name = request.name.replace('Request', 'Params')
                
                # Only generate if no interface already exists with this name
                if param_struct_name not in interface_names:
                    lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone)]")
                    lines.append(f"pub struct {param_struct_name} {{")
                    
                    # For now, we'll use generic JSON value since we don't have detailed parameter info
                    # This can be improved when we have the actual TypeScript parameter definitions
                    lines.append(f"    #[serde(flatten)]")
                    lines.append(f"    pub data: serde_json::Value,")
                    
                    lines.append("}")
                    lines.append("")
        
        # Generate request type structs
        for request in self.parser.requests:
            lines.append(f"#[derive(Debug)]")
            lines.append(f"pub struct {request.name};")
            lines.append("")
        
        return lines
    
    def _generate_enums(self) -> List[str]:
        """Generate enum definitions."""
        lines = []
        
        for enum_def in self.parser.enums:
            # Skip TypeServerVersion enum since we extract constants from it
            if enum_def.name == 'TypeServerVersion':
                continue
                
            # Check if it's a bit flags enum
            is_bitflags = any(field.value and ('<<' in field.value or '1 << ' in field.value) for field in enum_def.fields)
            
            if is_bitflags:
                lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone, Copy)]")
                lines.append(f"pub struct {enum_def.name}(i32);")
                lines.append("")
                lines.append(f"impl {enum_def.name} {{")
                
                for field in enum_def.fields:
                    const_name = self._to_screaming_snake_case(field.name)
                    if field.value:
                        # Handle bit shift operations
                        value = field.value.replace('<<', ' << ')
                        lines.append(f"    pub const {const_name}: {enum_def.name} = {enum_def.name}({value});")
                    else:
                        lines.append(f"    pub const {const_name}: {enum_def.name} = {enum_def.name}(0);")
                
                # Add common methods for flag enums
                lines.append("")
                lines.append(f"    pub fn new() -> Self {{")
                lines.append(f"        {enum_def.name}(0)")
                lines.append(f"    }}")
                lines.append("")
                lines.append(f"    pub fn has(self, flag: {enum_def.name}) -> bool {{")
                lines.append(f"        (self.0 & flag.0) != 0")
                lines.append(f"    }}")
                lines.append("")
                lines.append(f"    pub fn with(self, flag: {enum_def.name}) -> Self {{")
                lines.append(f"        {enum_def.name}(self.0 | flag.0)")
                lines.append(f"    }}")
                
                lines.append("}")
            else:
                lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]")
                lines.append(f"pub struct {enum_def.name}(i32);")
                lines.append("")
                lines.append(f"impl {enum_def.name} {{")
                
                for i, field in enumerate(enum_def.fields):
                    const_name = self._to_screaming_snake_case(field.name)
                    value = field.value if field.value else str(i)
                    lines.append(f"    pub const {const_name}: {enum_def.name} = {enum_def.name}({value});")
                
                lines.append("}")
            
            lines.append("")
        
        return lines
    
    def _generate_structs(self) -> List[str]:
        """Generate struct definitions."""
        lines = []
        
        for interface in self.parser.interfaces:
            if interface.name in ['RequestSender', 'NotificationSender', 'RequestReceiver', 'NotificationReceiver', 'TypeHandle']:
                continue  # Skip these utility interfaces and TypeHandle (handled separately)
                
            lines.append(f"#[derive(Serialize, Deserialize, Debug, Clone)]")
            lines.append(f"pub struct {interface.name} {{")
            
            for field in interface.fields:
                rust_type = self._map_typescript_type(field.type_)
                field_name = field.rename or self._to_snake_case(field.name)
                
                # Add serde rename if needed
                if field.rename or field.name != field_name:
                    original_name = field.name
                    lines.append(f'    #[serde(rename = "{original_name}")]')
                
                # Handle optionality - either from TypeScript optional field (?) or from | undefined
                is_optional = field.optional or rust_type.startswith('__OPTIONAL__')
                if rust_type.startswith('__OPTIONAL__'):
                    rust_type = rust_type[12:]  # Remove __OPTIONAL__ prefix
                
                if is_optional:
                    rust_type = f"Option<{rust_type}>"
                
                lines.append(f"    pub {field_name}: {rust_type},")
            
            lines.append("}")
            lines.append("")
        
        return lines
    
    def _generate_request_implementations(self) -> List[str]:
        """Generate LSP request implementations."""
        lines = []
        
        for request in self.parser.requests:
            # Determine params and result types
            params_type = "serde_json::Value"  # Default to generic JSON
            result_type = "serde_json::Value"  # Default to generic JSON
            
            if request.params_type:
                params_type = self._map_request_params_type(request.params_type, request.name)
            if request.result_type:
                mapped_result = self._map_typescript_type(request.result_type)
                # Clean up __OPTIONAL__ markers for result types
                if mapped_result.startswith('__OPTIONAL__'):
                    result_type = f"Option<{mapped_result[12:]}>"
                else:
                    result_type = mapped_result
            
            # Generate the implementation
            lines.append(f"impl lsp_types::request::Request for {request.name} {{")
            lines.append(f"    type Params = {params_type};")
            lines.append(f"    type Result = {result_type};")
            lines.append(f'    const METHOD: &\'static str = "{request.method}";')
            lines.append("}")
            lines.append("")
        
        return lines
    
    def _map_request_params_type(self, ts_type: str, request_name: str) -> str:
        """Map TypeScript parameter types to Rust parameter struct names."""
        # Handle inline object types
        if ts_type.startswith('{') and ts_type.endswith('}'):
            # Generate a params struct name
            base_name = request_name.replace('Request', '')
            return f"{base_name}Params"
        
        # Handle known interface types
        return self._map_typescript_type(ts_type)
    
    def _map_typescript_type(self, ts_type: str) -> str:
        """Map TypeScript types to Rust types."""
        # Clean up the type
        ts_type = ts_type.strip()
        
        # Handle union with undefined (make it optional) - but return info about optionality
        if ' | undefined' in ts_type:
            inner_type = ts_type.replace(' | undefined', '').strip()
            # Return a special marker that this should be optional
            mapped_inner = self._map_typescript_type(inner_type)
            return f"__OPTIONAL__{mapped_inner}"
        
        # Handle string | number union specifically (common in handle types)
        if ts_type == 'string | number':
            return 'TypeHandle'
        
        # Handle other union types by defaulting to the first type
        if ' | ' in ts_type:
            types = [t.strip() for t in ts_type.split(' | ')]
            # Use the first non-undefined type
            for t in types:
                if t != 'undefined':
                    return self._map_typescript_type(t)
        
        # Handle arrays
        if ts_type.endswith('[]'):
            element_type = ts_type[:-2]
            return f"Vec<{self._map_typescript_type(element_type)}>"
        
        # Handle TypeServerProtocol namespace references
        if ts_type.startswith('TypeServerProtocol.'):
            return ts_type.replace('TypeServerProtocol.', '')
        
        # Use direct mappings
        if ts_type in self.type_mappings:
            return self.type_mappings[ts_type]
        
        # Default: assume it's a struct name that exists
        return ts_type
    
    def _to_snake_case(self, name: str) -> str:
        """Convert camelCase to snake_case."""
        # Handle reserved keywords
        if name == 'type':
            return 'type_'
        
        # Insert underscore before uppercase letters
        result = re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', name)
        return result.lower()
    
    def _to_screaming_snake_case(self, name: str) -> str:
        """Convert camelCase to SCREAMING_SNAKE_CASE."""
        return self._to_snake_case(name).upper()

def main():
    if len(sys.argv) != 2:
        print("Usage: python generate_protocol.py <input_typescript_file>")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = "./protocol.rs"
    
    if not Path(input_file).exists():
        print(f"Error: Input file '{input_file}' not found")
        sys.exit(1)
    
    # Parse TypeScript file
    parser = TypeScriptParser()
    parser.parse_file(input_file)
    
    # Generate Rust code
    generator = RustCodeGenerator(parser)
    rust_code = generator.generate_protocol_rs()
    
    # Write output
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(rust_code)
    
    print(f"Generated {output_file} from {input_file}")
    print(f"Found {len(parser.enums)} enums, {len(parser.interfaces)} interfaces, {len(parser.requests)} requests")

if __name__ == "__main__":
    main()
