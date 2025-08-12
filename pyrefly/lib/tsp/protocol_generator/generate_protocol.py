#!/usr/bin/env python3
"""
Type Server Protocol Generator

This script generates a Rust protocol.rs file from the Type Server Protocol
JSON definitions using the lsprotocol library's generator infrastructure.
"""

import json
import pathlib
import sys
import os
from typing import Dict, Any

# Import the lsprotocol generator modules
import generator.model as model
from generator.plugins.rust import generate as rust_generate


def load_json_schema(file_path: str) -> Dict[str, Any]:
    """Load and parse a JSON file."""
    with open(file_path, 'r', encoding='utf-8') as f:
        return json.load(f)


def convert_json_to_model(tsp_json: Dict[str, Any]) -> model.LSPModel:
    """
    Convert our TSP JSON format to the lsprotocol internal model format.
    
    This function maps our JSON structure to the Python dataclasses used
    by the lsprotocol generator.
    """
    
    # Extract metadata
    metadata = tsp_json["metaData"]
    
    # Convert enumerations
    enumerations = []
    for enum_def in tsp_json.get("enumerations", []):
        values = []
        for value_def in enum_def["values"]:
            values.append(model.EnumItem(
                name=value_def["name"],
                value=value_def["value"],
                documentation=value_def.get("documentation")
            ))
        
        # Determine enumeration base type
        first_value = enum_def["values"][0]["value"] if enum_def["values"] else ""
        if isinstance(first_value, int):
            enum_type = model.EnumValueType(kind="base", name="integer")
        else:
            enum_type = model.EnumValueType(kind="base", name="string")
        
        enumerations.append(model.Enum(
            name=enum_def["name"],
            type=enum_type,
            values=values,
            documentation=enum_def.get("documentation"),
            supportsCustomValues=enum_def.get("supportsCustomValues", False)
        ))
    
    # Convert structures
    structures = []
    for struct_def in tsp_json.get("structures", []):
        properties = []
        for prop_def in struct_def["properties"]:
            prop_type = convert_type_reference(prop_def["type"])
            properties.append(model.Property(
                name=prop_def["name"],
                type=prop_type,
                optional=prop_def.get("optional", False),
                documentation=prop_def.get("documentation")
            ))
        
        structures.append(model.Structure(
            name=struct_def["name"],
            properties=properties,
            documentation=struct_def.get("documentation")
        ))
    
    # Convert type aliases
    type_aliases = []
    for alias_def in tsp_json.get("typeAliases", []):
        alias_type = convert_type_reference(alias_def["type"])
        type_aliases.append(model.TypeAlias(
            name=alias_def["name"],
            type=alias_type,
            documentation=alias_def.get("documentation")
        ))
    
    # Convert requests
    requests = []
    for req_def in tsp_json.get("requests", []):
        # Convert parameters
        params = None
        if req_def.get("params"):
            params = convert_type_reference(req_def["params"])
        
        # Convert result type - make sure it exists
        if "result" not in req_def:
            print(f"Warning: Request {req_def.get('method')} missing result field, skipping")
            continue
            
        result = convert_type_reference(req_def["result"])
        
        requests.append(model.Request(
            method=req_def["method"],
            params=params,
            result=result,
            messageDirection=req_def["messageDirection"],
            documentation=req_def.get("documentation"),
            typeName=req_def.get("typeName")
        ))
    
    # Convert notifications
    notifications = []
    for notif_def in tsp_json.get("notifications", []):
        # Convert parameters
        params = None
        if notif_def.get("params"):
            params = convert_type_reference(notif_def["params"])
        
        notifications.append(model.Notification(
            method=notif_def["method"],
            params=params,
            messageDirection=notif_def["messageDirection"],
            documentation=notif_def.get("documentation"),
            typeName=notif_def.get("typeName")
        ))
    
    return model.LSPModel(
        metaData=metadata,
        enumerations=enumerations,
        structures=structures,
        typeAliases=type_aliases,
        requests=requests,
        notifications=notifications
    )


def convert_type_reference(type_def: Dict[str, Any]) -> model.LSP_TYPE_SPEC:
    """Convert a type definition from JSON to the model format."""
    kind = type_def["kind"]
    
    if kind == "base":
        return model.BaseType(kind="base", name=type_def["name"])
    elif kind == "reference":
        return model.ReferenceType(kind="reference", name=type_def["name"])
    elif kind == "array":
        element_type = convert_type_reference(type_def["element"])
        return model.ArrayType(kind="array", element=element_type)
    elif kind == "or":
        items = [convert_type_reference(item) for item in type_def["items"]]
        return model.OrType(kind="or", items=items)
    elif kind == "and":
        items = [convert_type_reference(item) for item in type_def["items"]]
        return model.AndType(kind="and", items=items)
    elif kind == "literal":
        # Handle structure literals
        properties = []
        for prop_def in type_def["value"]["properties"]:
            prop_type = convert_type_reference(prop_def["type"])
            properties.append(model.Property(
                name=prop_def["name"],
                type=prop_type,
                optional=prop_def.get("optional", False),
                documentation=prop_def.get("documentation")
            ))
        
        literal_value = model.LiteralValue(properties=properties)
        return model.LiteralType(kind="literal", value=literal_value)
    elif kind == "stringLiteral":
        return model.StringLiteralType(kind="stringLiteral", value=type_def["value"])
    elif kind == "integerLiteral":
        return model.IntegerLiteralType(kind="integerLiteral", value=type_def["value"])
    elif kind == "booleanLiteral":
        return model.BooleanLiteralType(kind="booleanLiteral", value=type_def["value"])
    else:
        raise ValueError(f"Unsupported type kind: {kind}")


def generate_rust_protocol(tsp_json_path: str, output_dir: str) -> None:
    """Generate the Rust protocol.rs file from TSP JSON."""
    
    # Load the TSP JSON
    print(f"Loading TSP definition from: {tsp_json_path}")
    tsp_json = load_json_schema(tsp_json_path)
    
    # Convert to internal model
    print("Converting TSP JSON to internal model...")
    lsp_model = convert_json_to_model(tsp_json)
    
    # Generate Rust code
    print(f"Generating Rust code to: {output_dir}")
    
    # Create output directory
    output_path = pathlib.Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    # Generate the Rust code using lsprotocol's generator
    # Since we don't have a test directory, we'll pass an empty string
    rust_generate(lsp_model, str(output_path), "")
    
    # The generator creates a 'lsprotocol' subdirectory, let's copy the lib.rs
    # to protocol.rs in our main directory
    generated_lib = output_path / "lsprotocol" / "src" / "lib.rs"
    target_protocol = output_path / "protocol.rs"
    
    if generated_lib.exists():
        print(f"Copying generated lib.rs to protocol.rs...")
        content = generated_lib.read_text(encoding='utf-8')
        
        # Update the header comment to reflect that this is for Type Server Protocol
        content = content.replace(
            "Language Server Protocol types for Rust generated from LSP specification.",
            "Type Server Protocol types for Rust generated from TSP specification."
        ).replace(
            "// Steps to generate:\n// 1. Checkout https://github.com/microsoft/lsprotocol\n// 2. Install nox: `python -m pip install nox`\n// 3. Run command: `python -m nox --session build_lsp`",
            "// Steps to generate:\n// 1. Create tsp.json and tsp.schema.json from typeServerProtocol.ts\n// 2. Install lsprotocol generator: `pip install git+https://github.com/microsoft/lsprotocol.git`\n// 3. Run: `python generate_protocol.py`"
        )
        
        target_protocol.write_text(content, encoding='utf-8')
        print(f"Successfully generated: {target_protocol}")
    else:
        print(f"Warning: Generated lib.rs not found at {generated_lib}")


def main():
    """Main entry point."""
    if len(sys.argv) != 1:
        print("Usage: python generate_protocol.py")
        print("Example: python generate_protocol.py")
        sys.exit(1)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    tsp_json_path = os.path.join(script_dir, "tsp.json")
    output_dir = os.path.abspath(os.path.join(script_dir, "../"))

    try:
        generate_rust_protocol(tsp_json_path, output_dir)
        print("✅ Protocol generation completed successfully!")
    except Exception as e:
        print(f"❌ Error generating protocol: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
