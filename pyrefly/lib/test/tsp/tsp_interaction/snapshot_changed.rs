/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for TSP snapshotChanged notification

use tempfile::TempDir;

use crate::test::tsp::tsp_interaction::object_model::TspInteraction;

#[test]
fn test_tsp_snapshot_changed_notification_on_recheck() {
    // After opening a file and triggering an initial recheck, the client
    // should receive a snapshotChanged notification.
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

    // Opening a file eventually triggers RecheckFinished which increments the
    // snapshot and sends snap­shotChanged.
    tsp.server.did_open("test.py");

    let params = tsp.client.expect_notification("typeServer/snapshotChanged");
    // The notification params should contain the new snapshot value.
    let snapshot = params["snapshot"].as_i64().expect("snapshot should be an integer");
    assert!(snapshot > 0, "snapshot should be positive after recheck");

    tsp.shutdown();
}

#[test]
fn test_tsp_snapshot_changed_notification_on_did_change() {
    // A didChange notification should also trigger snapshotChanged.
    let temp_dir = TempDir::new().unwrap();
    let test_file_path = temp_dir.path().join("change.py");
    std::fs::write(&test_file_path, "x = 1\n").unwrap();

    let pyproject = r#"[project]
name = "test-project"
version = "1.0.0"
"#;
    std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let mut tsp = TspInteraction::new();
    tsp.set_root(temp_dir.path().to_path_buf());
    tsp.initialize(Default::default());

    tsp.server.did_open("change.py");

    // Consume the first snapshotChanged from the open/recheck
    tsp.client.expect_notification("typeServer/snapshotChanged");

    // Now send a didChange and expect another snapshotChanged
    tsp.server.did_change("change.py", "x = 2\n", 2);

    let params = tsp.client.expect_notification("typeServer/snapshotChanged");
    let snapshot = params["snapshot"].as_i64().expect("snapshot should be an integer");
    assert!(snapshot > 1, "snapshot should be > 1 after second change");

    tsp.shutdown();
}
