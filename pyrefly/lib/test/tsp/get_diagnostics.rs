/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for the getDiagnostics TSP request handler

use serde_json;

use crate::tsp;

#[test]
fn test_get_diagnostics_params_construction() {
    let params = tsp::GetDiagnosticsParams { uri: "file:///test.py".to_owned(), snapshot: 1 };

    assert_eq!(params.uri, "file:///test.py");
    assert_eq!(params.snapshot, 1);
}

#[test]
fn test_get_diagnostics_params_serialization() {
    let params = tsp::GetDiagnosticsParams { uri: "file:///test.py".to_owned(), snapshot: 1 };

    let serialized = serde_json::to_string(&params).expect("Failed to serialize params");
    let expected = r#"{"uri":"file:///test.py","snapshot":1}"#;
    assert_eq!(serialized, expected);

    let deserialized: tsp::GetDiagnosticsParams =
        serde_json::from_str(&serialized).expect("Failed to deserialize params");
    assert_eq!(deserialized.uri, params.uri);
    assert_eq!(deserialized.snapshot, params.snapshot);
}
