/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests for the CinderX type report.

use pretty_assertions::assert_eq;

use crate::report::cinderx::collect::collect_module_types;
use crate::report::cinderx::types::StructuredType;
use crate::report::cinderx::write_results;
use crate::state::require::Require;
use crate::test::util::TestEnv;

/// Create a type-checked state from a single module's Python source.
fn create_state(module_name: &str, python_code: &str) -> crate::state::state::State {
    let mut test_env = TestEnv::new();
    test_env.add(module_name, python_code);
    let (state, _) = test_env
        .with_default_require_level(Require::Everything)
        .to_state();
    state
}

/// Find the handle for a module by name in a transaction.
fn get_handle(
    module_name: &str,
    transaction: &crate::state::state::Transaction,
) -> pyrefly_build::handle::Handle {
    transaction
        .handles()
        .into_iter()
        .find(|h| h.module().as_str() == module_name)
        .unwrap_or_else(|| panic!("module `{module_name}` not found"))
}

#[test]
fn test_simple_variable() {
    let state = create_state("test", "x: int = 42");
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // We should have at least one located type (the variable `x`)
    assert!(
        !data.locations.is_empty(),
        "expected at least one located type"
    );
    assert!(
        !data.entries.is_empty(),
        "expected at least one type table entry"
    );

    // The type table should contain a `builtins.int` class entry
    let has_int = data.entries.iter().any(|entry| {
        matches!(&entry.ty, StructuredType::Class { qname, args, .. } if qname == "builtins.int" && args.is_empty())
    });
    assert!(
        has_int,
        "expected `builtins.int` in the type table, got: {:#?}",
        data.entries
    );
}

#[test]
fn test_function_types() {
    let state = create_state(
        "test",
        r#"
def foo(x: int) -> str:
    return str(x)

y = foo(42)
"#,
    );
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // Should have `int`, `str` class entries and a callable entry for `foo`
    let has_str = data.entries.iter().any(
        |entry| matches!(&entry.ty, StructuredType::Class { qname, .. } if qname == "builtins.str"),
    );
    let has_callable = data
        .entries
        .iter()
        .any(|entry| matches!(&entry.ty, StructuredType::Callable { .. }));
    assert!(has_str, "expected `builtins.str` in the type table");
    assert!(has_callable, "expected a Callable entry for `foo`");

    // Every location should reference a valid type table index
    for loc in &data.locations {
        assert!(
            loc.type_index < data.entries.len(),
            "location references out-of-bounds type index {} (table has {} entries)",
            loc.type_index,
            data.entries.len(),
        );
    }
}

#[test]
fn test_optional_type() {
    let state = create_state(
        "test",
        r#"
x: int | None = None
"#,
    );
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // Should have a typing.Optional wrapping int
    let has_optional = data.entries.iter().any(|entry| {
        matches!(&entry.ty, StructuredType::OtherForm { qname, args } if qname == "typing.Optional" && args.len() == 1)
    });
    assert!(
        has_optional,
        "expected `typing.Optional` in the type table, got: {:#?}",
        data.entries,
    );
}

#[test]
fn test_literal_type() {
    let state = create_state("test", "x = 42");
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // The literal value `42` should produce a Literal entry
    let has_literal = data
        .entries
        .iter()
        .any(|entry| matches!(&entry.ty, StructuredType::Literal { value } if value == "42"));
    assert!(
        has_literal,
        "expected Literal(42) in the type table, got: {:#?}",
        data.entries,
    );
}

#[test]
fn test_class_with_type_args() {
    let state = create_state(
        "test",
        r#"
x: list[int] = [1, 2, 3]
"#,
    );
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // Should have `builtins.list` with an `builtins.int` arg
    let has_list = data.entries.iter().any(|entry| {
        matches!(&entry.ty, StructuredType::Class { qname, args, .. } if qname == "builtins.list" && args.len() == 1)
    });
    assert!(
        has_list,
        "expected `builtins.list` with type arg in the type table, got: {:#?}",
        data.entries,
    );
}

#[test]
fn test_deduplication() {
    let state = create_state(
        "test",
        r#"
x: int = 1
y: int = 2
"#,
    );
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // `builtins.int` should appear only once in the type table despite being used twice
    let int_count = data.entries.iter().filter(|entry| {
        matches!(&entry.ty, StructuredType::Class { qname, args, .. } if qname == "builtins.int" && args.is_empty())
    }).count();
    assert_eq!(
        int_count, 1,
        "expected exactly one `builtins.int` entry (deduplication)"
    );
}

#[test]
fn test_json_serialization() {
    let state = create_state("test", "x: int = 42");
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // Verify the data can be serialized to JSON without errors
    let entries_json =
        serde_json::to_string_pretty(&data.entries).expect("type table should be serializable");
    let locations_json =
        serde_json::to_string_pretty(&data.locations).expect("locations should be serializable");

    // Basic sanity: JSON should contain expected strings
    assert!(
        entries_json.contains("\"kind\""),
        "entries JSON should have 'kind' field"
    );
    assert!(
        locations_json.contains("\"loc\""),
        "locations JSON should have 'loc' field"
    );
    assert!(
        locations_json.contains("\"type\""),
        "locations JSON should have 'type' field"
    );
}

#[test]
fn test_mro_collection() {
    let state = create_state(
        "test",
        r#"
class Base:
    pass

class Child(Base):
    pass

x: Child = Child()
"#,
    );
    let transaction = state.transaction();
    let handle = get_handle("test", &transaction);

    let data = collect_module_types(&transaction, &handle).expect("should collect types");

    // The classes list should contain the classes we referenced
    assert!(
        !data.classes.is_empty(),
        "expected at least one class in the classes list"
    );
}

#[test]
fn test_mro_in_report() {
    let state = create_state(
        "test",
        r#"
class Base:
    pass

class Child(Base):
    pass

x: Child = Child()
"#,
    );
    let transaction = state.transaction();
    let handles: Vec<_> = transaction.handles();

    let output_dir = tempfile::tempdir().expect("should create temp dir");
    write_results(output_dir.path(), &transaction, &handles).expect("should write results");

    // Read and parse class_metadata.json
    let metadata_json = std::fs::read_to_string(output_dir.path().join("class_metadata.json"))
        .expect("class_metadata.json should exist");
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_json).expect("class_metadata.json should be valid JSON");

    let entries = metadata["entries"]
        .as_array()
        .expect("entries should be an array");

    // Find the Child class entry
    let child_entry = entries
        .iter()
        .find(|e| e["qname"].as_str() == Some("test.Child"));
    assert!(
        child_entry.is_some(),
        "expected entry for test.Child, got entries: {metadata_json}"
    );

    let ancestors = child_entry.unwrap()["ancestors"]
        .as_array()
        .expect("ancestors should be an array");
    let ancestor_names: Vec<&str> = ancestors
        .iter()
        .map(|a| a.as_str().expect("ancestor should be a string"))
        .collect();
    assert!(
        ancestor_names.contains(&"test.Base"),
        "expected test.Base in Child's MRO, got: {ancestor_names:?}"
    );

    // Non-protocol classes should have no tags
    let child_tags = child_entry.unwrap().get("tags");
    assert!(
        child_tags.is_none() || child_tags.unwrap().as_array().unwrap().is_empty(),
        "expected no tags on non-protocol class"
    );
}

#[test]
fn test_protocol_tags() {
    let state = create_state(
        "test",
        r#"
from typing import Protocol

class MyProto(Protocol):
    def method(self) -> int: ...

class Impl(MyProto):
    def method(self) -> int:
        return 42

x: MyProto = Impl()
y: Impl = Impl()
"#,
    );
    let transaction = state.transaction();
    let handles: Vec<_> = transaction.handles();

    let output_dir = tempfile::tempdir().expect("should create temp dir");
    write_results(output_dir.path(), &transaction, &handles).expect("should write results");

    let metadata_json = std::fs::read_to_string(output_dir.path().join("class_metadata.json"))
        .expect("class_metadata.json should exist");
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_json).expect("class_metadata.json should be valid JSON");

    let entries = metadata["entries"]
        .as_array()
        .expect("entries should be an array");

    // MyProto should have the "protocol" tag
    let proto_entry = entries
        .iter()
        .find(|e| e["qname"].as_str() == Some("test.MyProto"));
    assert!(
        proto_entry.is_some(),
        "expected entry for test.MyProto, got entries: {metadata_json}"
    );
    let proto_tags: Vec<&str> = proto_entry.unwrap()["tags"]
        .as_array()
        .expect("tags should be an array")
        .iter()
        .map(|t| t.as_str().expect("tag should be a string"))
        .collect();
    assert!(
        proto_tags.contains(&"protocol"),
        "expected 'protocol' tag on MyProto, got: {proto_tags:?}"
    );

    // Impl should have the "inherits_protocol" tag
    let impl_entry = entries
        .iter()
        .find(|e| e["qname"].as_str() == Some("test.Impl"));
    assert!(
        impl_entry.is_some(),
        "expected entry for test.Impl, got entries: {metadata_json}"
    );
    let impl_tags: Vec<&str> = impl_entry.unwrap()["tags"]
        .as_array()
        .expect("tags should be an array")
        .iter()
        .map(|t| t.as_str().expect("tag should be a string"))
        .collect();
    assert!(
        impl_tags.contains(&"inherits_protocol"),
        "expected 'inherits_protocol' tag on Impl, got: {impl_tags:?}"
    );
}
