/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP resolveImport request

use lsp_server::RequestId;
use lsp_types::Url;
use tempfile::TempDir;

use crate::lsp::non_wasm::protocol::Response;
use crate::test::tsp::tsp_interaction::object_model::TspInteraction;

/// Resolving a stdlib module (e.g. `os`) should return a valid file URI.
#[test]
fn test_tsp_resolve_import_stdlib() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "import os\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let source_uri = Url::from_file_path(&test_file).unwrap().to_string();
    tsp.server.resolve_import(&source_uri, 0, &["os"], 1);

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
            let uri: String = serde_json::from_value(result).unwrap();
            // The resolved path should reference the `os` module. For bundled
            // typeshed this may be a relative path rather than a file URI.
            assert!(
                uri.contains("os"),
                "Expected resolved path to reference 'os' module, got: {uri}"
            );
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

/// Resolving a local module in the same project should return a file URI.
#[test]
fn test_tsp_resolve_import_local_module() {
    let temp_dir = TempDir::new().unwrap();
    // Create a package with two modules
    std::fs::create_dir_all(temp_dir.path().join("mypackage")).unwrap();
    std::fs::write(temp_dir.path().join("mypackage/__init__.py"), "").unwrap();
    std::fs::write(
        temp_dir.path().join("mypackage/utils.py"),
        "def helper(): pass\n",
    )
    .unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "from mypackage import utils\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let source_uri = Url::from_file_path(&test_file).unwrap().to_string();
    tsp.server
        .resolve_import(&source_uri, 0, &["mypackage", "utils"], 1);

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
            let uri: String = serde_json::from_value(result).unwrap();
            let expected_uri = Url::from_file_path(temp_dir.path().join("mypackage/utils.py"))
                .unwrap()
                .to_string();
            assert_eq!(uri, expected_uri);
        }
        other => panic!("Expected Response, got: {other:?}"),
    }

    tsp.shutdown();
}

/// Resolving a module that does not exist should return an error.
#[test]
fn test_tsp_resolve_import_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "import nonexistent_module_xyz\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let source_uri = Url::from_file_path(&test_file).unwrap().to_string();
    tsp.server
        .resolve_import(&source_uri, 0, &["nonexistent_module_xyz"], 1);

    tsp.client.expect_response(Response {
        id: RequestId::from(2),
        result: Some(serde_json::Value::Null),
        error: None,
    });

    tsp.shutdown();
}

/// Using a stale snapshot should return a ServerCancelled error.
#[test]
fn test_tsp_resolve_import_snapshot_outdated() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("main.py");
    std::fs::write(&test_file, "import os\n").unwrap();

    let pyproject = "[project]\nname = \"test\"\nversion = \"1.0.0\"\n";
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("main.py");
    tsp.client.expect_notification("typeServer/snapshotChanged");

    let source_uri = Url::from_file_path(&test_file).unwrap().to_string();
    // Use snapshot=0 which is stale (current is 1)
    tsp.server.resolve_import(&source_uri, 0, &["os"], 0);

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
