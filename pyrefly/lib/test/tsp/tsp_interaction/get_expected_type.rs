/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP getExpectedType request

use lsp_server::RequestId;
use lsp_types::Url;
use tempfile::TempDir;

use crate::lsp::non_wasm::protocol::Message;
use crate::test::tsp::tsp_interaction::object_model::TspInteraction;

/// Getting the expected type of a simple annotated variable should succeed.
#[test]
fn test_tsp_get_expected_type_basic() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "x: float = 3.14\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let uri = Url::from_file_path(&test_file).unwrap().to_string();
    tsp.server.get_expected_type(&uri, 0, 0, 1);

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
                .expect("Expected 'kind' field in Type response");
            // For `float`, we expect a ClassType with a declaration
            assert_eq!(
                kind, "Class",
                "Expected Class kind for float type, got: {kind}"
            );
            let decl = result
                .get("declaration")
                .expect("Expected 'declaration' field in ClassType");
            let decl_name = decl.get("name").and_then(|v| v.as_str());
            assert_eq!(
                decl_name,
                Some("float"),
                "Expected declaration name 'float'"
            );
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

/// Stale snapshot should return a ServerCancelled error.
#[test]
fn test_tsp_get_expected_type_snapshot_outdated() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "x: int = 1\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let uri = Url::from_file_path(&test_file).unwrap().to_string();
    tsp.server.get_expected_type(&uri, 0, 0, 0);

    let msg = tsp.client.receive_any_message();
    match msg {
        Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::from(2));
            let err = resp.error.expect("Expected error for stale snapshot");
            assert_eq!(err.code, -32802, "Expected ServerCancelled error code");
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}
