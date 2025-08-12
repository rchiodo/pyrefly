/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for the getSymbolsForFile TSP request

use crate::test::tsp::util::build_tsp_test_server;
use crate::tsp;
use crate::tsp::protocol::{Range, Position};

#[test]
fn test_get_symbols_for_file_params_construction() {
    let (_handle, uri, _state) = build_tsp_test_server();
    let uri_str = uri.to_string();

    let params = tsp::GetSymbolsForFileParams { uri: uri_str.clone(), snapshot: 1 };

    // Just test that we can construct the parameters correctly
    assert_eq!(params.snapshot, 1);
    assert_eq!(params.uri, uri_str);
}

#[test]
fn test_get_symbols_for_file_serialization_deserialization() {
    let (_handle, uri, _state) = build_tsp_test_server();
    let uri_str = uri.to_string();

    let params = tsp::GetSymbolsForFileParams { uri: uri_str.clone(), snapshot: 1 };

    // Test serialization
    let serialized = serde_json::to_string(&params).expect("Should serialize");

    // Test deserialization
    let deserialized: tsp::GetSymbolsForFileParams =
        serde_json::from_str(&serialized).expect("Should deserialize");

    assert_eq!(deserialized.uri, params.uri);
    assert_eq!(deserialized.snapshot, params.snapshot);
}

#[test]
fn test_file_symbol_info_construction() {
    let (_handle, uri, _state) = build_tsp_test_server();
    let uri_str = uri.to_string();

    let symbols = vec![tsp::Symbol {
        node: tsp::Node {
            uri: uri_str.clone(),
            range: Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 5 } },
        },
        name: "test_symbol".to_owned(),
        decls: vec![],
        synthesized_types: vec![],
    }];

    let file_symbol_info = tsp::FileSymbolInfo { uri: uri_str.clone(), symbols: symbols.clone() };

    assert_eq!(file_symbol_info.uri, uri_str);
    assert_eq!(file_symbol_info.symbols.len(), 1);
    assert_eq!(file_symbol_info.symbols[0].name, "test_symbol");
}
