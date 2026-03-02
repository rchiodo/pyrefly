/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP getPythonSearchPaths request

use lsp_server::RequestId;
use lsp_types::Url;
use tempfile::TempDir;

use crate::lsp::non_wasm::protocol::Response;
use crate::test::tsp::tsp_interaction::object_model::TspInteraction;

#[test]
fn test_tsp_get_python_search_paths_basic() {
    // getPythonSearchPaths should return at least one path for a valid project
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("test.py");
    std::fs::write(&test_file_path, "x = 1\n").unwrap();

    let pyproject = r#"[project]
name = "test-project"
version = "1.0.0"
"#;
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("test.py");
    // Wait for diagnostics / RecheckFinished and the snapshotChanged notification
    tsp.client.expect_notification("typeServer/snapshotChanged");

    // Use snapshot=1 (after first recheck)
    let from_uri = Url::from_file_path(&test_file_path).unwrap().to_string();
    tsp.server.get_python_search_paths(&from_uri, 1);

    // Expect a response with an array of path strings
    let msg = tsp.client.receive_any_message();
    match msg {
        crate::lsp::non_wasm::protocol::Message::Response(resp) => {
            assert_eq!(resp.id, RequestId::from(2));
            assert!(
                resp.error.is_none(),
                "Expected success but got error: {:?}",
                resp.error
            );
            let result = resp.result.expect("Expected result in response");
            let paths: Vec<String> = serde_json::from_value(result).unwrap();
            // Should contain at least one path (the project root or import root)
            assert!(!paths.is_empty(), "Expected at least one search path");
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

#[test]
fn test_tsp_get_python_search_paths_snapshot_outdated() {
    // Sending a stale snapshot should return a ServerCancelled error
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("test.py");
    std::fs::write(&test_file_path, "x = 1\n").unwrap();

    let pyproject = r#"[project]
name = "test-project"
version = "1.0.0"
"#;
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("test.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    // Use snapshot=0 which is now stale (current is 1)
    let from_uri = Url::from_file_path(&test_file_path).unwrap().to_string();
    tsp.server.get_python_search_paths(&from_uri, 0);

    tsp.client.expect_response(Response {
        id: RequestId::from(2),
        result: None,
        error: Some(lsp_server::ResponseError {
            code: lsp_server::ErrorCode::ServerCancelled as i32,
            message: "Snapshot outdated".to_owned(),
            data: None,
        }),
    });

    tsp.shutdown();
}
