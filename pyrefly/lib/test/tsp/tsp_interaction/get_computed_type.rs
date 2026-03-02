/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP getComputedType request

use lsp_server::RequestId;
use lsp_types::Url;
use tempfile::TempDir;

use crate::lsp::non_wasm::protocol::Message;
use crate::test::tsp::tsp_interaction::object_model::TspInteraction;

/// Getting the computed type of a simple variable annotated as `int` should
/// return a valid TSP Type whose string representation contains "int".
#[test]
fn test_tsp_get_computed_type_basic() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "x: int = 42\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let uri = Url::from_file_path(&test_file).unwrap().to_string();
    // Position 0:0 = the `x` identifier
    tsp.server.get_computed_type(&uri, 0, 0, 1);

    let msg = tsp.client.receive_any_message();
    match msg {
        Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::from(2));
            assert!(
                resp.error.is_none(),
                "Expected success but got error: {:?}",
                resp.error
            );
            let result = resp.result.expect("Expected result in response");
            // The result should be a TSP Type JSON object with a "kind" field
            let kind = result
                .get("kind")
                .and_then(|v| v.as_str())
                .expect("Expected 'kind' field in Type response");
            // For `int`, we expect a ClassType with a declaration
            assert_eq!(
                kind, "Class",
                "Expected Class kind for int type, got: {kind}"
            );
            // Should have a declaration with the class name
            let decl = result
                .get("declaration")
                .expect("Expected 'declaration' field in ClassType");
            let decl_name = decl.get("name").and_then(|v| v.as_str());
            assert_eq!(
                decl_name,
                Some("int"),
                "Expected declaration name 'int'"
            );
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

/// Requesting a computed type with a stale snapshot should return an error.
#[test]
fn test_tsp_get_computed_type_snapshot_outdated() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "x: int = 42\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let uri = Url::from_file_path(&test_file).unwrap().to_string();
    // Use snapshot 0, but current snapshot is 1 after didOpen
    tsp.server.get_computed_type(&uri, 0, 0, 0);

    let msg = tsp.client.receive_any_message();
    match msg {
        Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::from(2));
            let err = resp.error.expect("Expected error for stale snapshot");
            // ServerCancelled = -32802
            assert_eq!(err.code, -32802, "Expected ServerCancelled error code");
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

/// Getting the computed type of a `None` literal should return a BuiltInType
/// with name "none".
#[test]
fn test_tsp_get_computed_type_none() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "x = None\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let uri = Url::from_file_path(&test_file).unwrap().to_string();
    // Position 0:0 = the `x` identifier; its computed type is `None`
    tsp.server.get_computed_type(&uri, 0, 0, 1);

    let msg = tsp.client.receive_any_message();
    match msg {
        Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::from(2));
            assert!(
                resp.error.is_none(),
                "Expected success but got error: {:?}",
                resp.error
            );
            let result = resp.result.expect("Expected result in response");
            let kind = result
                .get("kind")
                .and_then(|v| v.as_str())
                .expect("Expected 'kind' field");
            assert_eq!(kind, "BuiltIn", "Expected BuiltIn type for None");
            let name = result
                .get("name")
                .and_then(|v| v.as_str())
                .expect("Expected 'name' field");
            assert_eq!(name, "none", "Expected name 'none' for None type");
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}
