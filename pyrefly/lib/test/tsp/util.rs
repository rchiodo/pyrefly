/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Utility functions for TSP testing

use lsp_types::{Range, Position, Url};
use ruff_text_size::TextSize;

use crate::state::state::State;
use crate::state::handle::Handle;
use crate::test::util::{mk_multi_file_state_assert_no_errors, extract_cursors_for_test};
use crate::tsp;

/// Create a test server and return it along with a test handle and URI
pub fn build_tsp_test_server() -> (Handle, Url, State) {
    let files = [("test.py", "")];
    let (handles, state) = mk_multi_file_state_assert_no_errors(&files);
    let handle = handles["test.py"].clone();
    
    // Create a simple test URI instead of using file path
    let uri = Url::parse("file:///test.py").expect("Failed to create test URI");
    
    (handle, uri, state)
}

/// Extract cursor location from test content
pub fn extract_cursor_location(content: &str, uri: &Url) -> Position {
    let cursors = extract_cursors_for_test(content);
    if cursors.is_empty() {
        panic!("No cursor found in test content");
    }
    
    // Convert TextSize to LSP Position
    let cursor_pos = cursors[0];
    let lines: Vec<&str> = content.lines().collect();
    let mut char_offset = 0;
    let mut line_number = 0;
    
    for (line_idx, line) in lines.iter().enumerate() {
        let line_end = char_offset + line.len() + 1; // +1 for newline
        if cursor_pos.to_usize() <= char_offset + line.len() {
            line_number = line_idx;
            break;
        }
        char_offset = line_end;
    }
    
    let character = cursor_pos.to_usize() - char_offset;
    Position {
        line: line_number as u32,
        character: character as u32,
    }
}

/// Helper to create a TSP Node from a position
pub fn make_tsp_node(uri: Url, position: Position, text_length: u32) -> tsp::Node {
    tsp::Node {
        uri,
        range: Range {
            start: position,
            end: Position {
                line: position.line,
                character: position.character + text_length,
            },
        },
    }
}

/// Helper to create a TSP Node from a cursor position and handle
pub fn make_tsp_node_from_cursor(handle: &Handle, cursor_pos: TextSize, text_length: u32) -> tsp::Node {
    // Create a simple test URI instead of using file path
    let uri = Url::parse("file:///test.py").expect("Failed to create test URI");
    
    // Create a simple position at line 0, character position based on cursor
    let position = Position {
        line: 0,
        character: cursor_pos.to_usize() as u32,
    };
    
    make_tsp_node(uri, position, text_length)
}

/// Helper to format TSP test results for comparison
pub fn format_symbol_result(result: Option<tsp::Symbol>) -> String {
    match result {
        Some(symbol) => {
            let mut output = String::new();
            output.push_str(&format!("Symbol: {}\n", symbol.name));
            output.push_str(&format!("Declarations: {}\n", symbol.decls.len()));
            
            for (i, decl) in symbol.decls.iter().enumerate() {
                output.push_str(&format!("  Decl {}: {:?} in module {}\n", 
                    i, decl.category, decl.module_name.name_parts.join(".")));
                if let Some(ref node) = decl.node {
                    output.push_str(&format!("    Range: {}:{}-{}:{}\n",
                        node.range.start.line, node.range.start.character,
                        node.range.end.line, node.range.end.character));
                }
            }
            
            output.push_str(&format!("Types: {}\n", symbol.synthesized_types.len()));
            for (i, typ) in symbol.synthesized_types.iter().enumerate() {
                output.push_str(&format!("  Type {}: {:?} - {}\n", 
                    i, typ.category, typ.name));
            }
            
            output
        }
        None => "No symbol found\n".to_string(),
    }
}

/// Helper to format type result for comparison
pub fn format_type_result(result: Option<tsp::Type>) -> String {
    match result {
        Some(typ) => {
            format!("Type: {:?} - {} (flags: {:?})\n", 
                typ.category, typ.name, typ.flags)
        }
        None => "No type found\n".to_string(),
    }
}

/// Helper to format overloads result
pub fn format_overloads_result(result: Vec<tsp::Type>) -> String {
    if result.is_empty() {
        "No overloads found\n".to_string()
    } else {
        let mut output = format!("Found {} overloads:\n", result.len());
        for (i, typ) in result.iter().enumerate() {
            output.push_str(&format!("  Overload {}: {:?} - {}\n", 
                i, typ.category, typ.name));
        }
        output
    }
}

/// Helper to format function parts result
pub fn format_function_parts_result(result: Option<tsp::FunctionParts>) -> String {
    match result {
        Some(parts) => {
            let mut output = String::new();
            output.push_str(&format!("Function Parts:\n"));
            
            if !parts.params.is_empty() {
                output.push_str("  Parameters:\n");
                for (i, param) in parts.params.iter().enumerate() {
                    // Assume params are just strings for now - adjust based on actual TSP structure
                    output.push_str(&format!("    {}: {}\n", i, param));
                }
            }
            
            if !parts.return_type.is_empty() {
                output.push_str(&format!("  Return Type: {}\n", parts.return_type));
            }
            
            output
        }
        None => "No function parts found\n".to_string(),
    }
}

/// Helper to run a TSP test and capture any panics as error messages
pub fn run_tsp_test_safe<F, R>(test_fn: F) -> Result<R, String>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(test_fn) {
        Ok(result) => Ok(result),
        Err(panic_info) => {
            let panic_msg = if let Some(msg) = panic_info.downcast_ref::<String>() {
                msg.clone()
            } else if let Some(&msg) = panic_info.downcast_ref::<&str>() {
                msg.to_string()
            } else {
                "Unknown panic occurred".to_string()
            };
            Err(format!("Test panicked: {}", panic_msg))
        }
    }
}

/// Format TSP search for type attribute result
pub fn format_search_for_type_attribute_result(result: Option<tsp::Type>) -> String {
    match result {
        Some(res) => format!("Search results: {:?}", res),
        None => "No search results found".to_string(),
    }
}

/// Format TSP Python search paths result
pub fn format_search_paths_result(result: Vec<Url>) -> String {
    if result.is_empty() {
        "No search paths found".to_string()
    } else {
        format!("Search Paths: {} paths found", result.len())
    }
}

/// Format TSP import declaration result
pub fn format_import_declaration_result(result: Option<tsp::Declaration>) -> String {
    match result {
        Some(res) => format!("Import Resolution: {:?}", res),
        None => "No resolution found".to_string(),
    }
}

/// Format TSP type of declaration result
pub fn format_type_of_declaration_result(result: tsp::Type) -> String {
    format!("Type of Declaration: {:?} - {}", result.category, result.name)
}

/// Format TSP docstring result
pub fn format_docstring_result(result: Option<String>) -> String {
    match result {
        Some(res) => format!("Docstring: {}", res),
        None => "No docstring found".to_string(),
    }
}

/// Format TSP diagnostics version result
pub fn format_diagnostics_version_result(result: u32) -> String {
    format!("Diagnostics Version: {}", result)
}
