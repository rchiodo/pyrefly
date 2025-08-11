/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Utility functions for TSP testing

use lsp_types::Position;
use lsp_types::Url;

use crate::state::handle::Handle;
use crate::state::state::State;
use crate::test::util::extract_cursors_for_test;
use crate::test::util::mk_multi_file_state_assert_no_errors;

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
pub fn extract_cursor_location(content: &str, _uri: &Url) -> Position {
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
