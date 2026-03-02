# TSP Implementation Agent Guidelines

This document provides instructions for an AI agent implementing TSP (Type
Server Protocol) messages in the pyrefly codebase. Read this document in full
before starting any work. Also read `AGENTS.md` in the repo root — it takes
precedence for general coding style.

## Project Context

Pyrefly is a Python type checker and language server written in Rust. The Type
Server Protocol (TSP) is a parallel protocol that runs alongside LSP, exposing
type-checking capabilities to external clients. The TSP server wraps the LSP
server and delegates non-TSP events to it.

See `TSP_IMPLEMENTATION_PLAN.md` for the full task list and milestone ordering.

---

## Core Principles

### 1. Minimal Changes to Pyrefly Internals

The TSP layer is a **thin adapter** on top of existing pyrefly infrastructure.

- **Do NOT modify** files in `pyrefly/lib/alt/`, `pyrefly/lib/binding/`,
  `pyrefly/lib/solver/`, `pyrefly/lib/export/`, or `pyrefly/lib/error/`.
- **Do NOT modify** `crates/tsp_types/src/protocol.rs` — it is generated.
- **Prefer extending `TspInterface`** (in `pyrefly/lib/lsp/non_wasm/server.rs`)
  with small, purpose-specific methods rather than exposing `State` directly.
  Each new trait method should have a single clear responsibility.
- **Conversion code** (pyrefly types → TSP types) must live in `pyrefly/lib/tsp/`
  — never inside the LSP or core type-checker code.
- When you need data from the inner `Server`, add a method to `TspInterface`
  and implement it on `Server`. Keep the implementation body minimal — call
  existing `Transaction` or `State` methods; do not duplicate logic.

### 2. Zero Clippy Warnings

All new code must pass `cargo clippy` with zero warnings. Specifically:

- **No `#[allow(unused)]` or `#[allow(dead_code)]`** on new code. Every
  function, struct, and field you add must be used or tested. (The existing
  `protocol.rs` has `#![allow(clippy::all)]` because it is generated — do not
  replicate this in handwritten code.)
- **No `unwrap()` in production code.** Use `?`, `.expect("explanation")`, or
  proper error handling. `unwrap()` is acceptable in test code.
- **Use `impl` imports**, not inline qualified paths. E.g., write
  `use tsp_types::ResolveImportParams;` at the top, not
  `tsp_types::ResolveImportParams` inline.
- **Match arms must be exhaustive.** When matching on `TSPRequests`, handle
  every variant explicitly — do not use `_ =>` for known variants.
- **Prefer `&str` over `String`** in function parameters when the function
  does not need ownership.
- Run `cargo clippy --all-targets` after every change and fix all warnings
  before committing. If a clippy lint is a false positive, suppress it with
  `#[expect(clippy::lint_name)]` (not `#[allow(...)]`) and add a comment
  explaining why.

### 3. Good Tests

Every TSP message needs **two layers** of tests:

#### Unit Tests (in `crates/tsp_types/tests/`)

These test the protocol types in isolation — no pyrefly server required.

- Serialization round-trips (struct → JSON → struct)
- Wire format verification (exact JSON field names, camelCase, required/optional)
- Edge cases (null params, empty arrays, missing optional fields)

**Pattern:** See `crates/tsp_types/tests/protocol_types.rs` for examples.

#### Integration Tests (in `pyrefly/lib/test/tsp/tsp_interaction/`)

These spin up a real TSP server over in-memory channels and exercise the full
request/response cycle.

- Use the existing `TspInteraction` test harness from `object_model.rs`
- Each test should: create temp dir → write Python files → initialize →
  send request → assert response → shutdown
- For helpers on `TestTspServer` (e.g., `get_python_search_paths()`), follow
  the pattern of existing helpers like `get_snapshot()` and
  `get_supported_protocol_version()`
- Assert exact JSON responses using `Response { id, result, error }`
- Test both success paths and error paths (stale snapshot, invalid URI, etc.)

**Pattern:** See `pyrefly/lib/test/tsp/tsp_interaction/get_snapshot.rs`.

#### Module Registration

When adding new test files, you must:
1. Add `pub mod <name>;` to the appropriate `mod.rs` file
2. For `crates/tsp_types/tests/`: add to `tests/mod.rs`
3. For `pyrefly/lib/test/tsp/tsp_interaction/`: add to
   `tsp_interaction/mod.rs`

---

## File Locations & Conventions

### Where to put new code

| What | Where |
|------|-------|
| New request handler | `pyrefly/lib/tsp/requests/<message_name>.rs` |
| New notification handler | `pyrefly/lib/tsp/notifications/<message_name>.rs` |
| Type conversion (pyrefly → TSP) | `pyrefly/lib/tsp/type_conversion.rs` |
| Shared TSP helpers (position conversion, snapshot validation) | `pyrefly/lib/tsp/server.rs` or a new `pyrefly/lib/tsp/helpers.rs` |
| Manual params types not in generated code | `crates/tsp_types/src/common.rs` |
| Unit tests for params/types | `crates/tsp_types/tests/<message_name>.rs` |
| Integration tests | `pyrefly/lib/test/tsp/tsp_interaction/<message_name>.rs` |

### New handler file template

```rust
/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the <messageName> TSP request

use tsp_types::SomeParamsType;
use tsp_types::SomeResponseType;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;

impl<T: TspInterface> TspServer<T> {
    /// <Doc comment explaining what this request does>
    pub fn handle_message_name(
        &self,
        params: SomeParamsType,
    ) -> Result<SomeResponseType, lsp_server::ResponseError> {
        // 1. Validate snapshot
        // 2. Call self.inner.<method>()
        // 3. Convert result to TSP response type
        todo!()
    }
}
```

### Dispatcher wiring

In `pyrefly/lib/tsp/server.rs`, method `handle_tsp_request`, add a new match
arm. The match must be **exhaustive** — do not leave a catch-all `_ =>`.
Instead, every `TSPRequests` variant should have its own arm.

### `TspInterface` extensions

When adding a new method to `TspInterface`:

1. Add the method signature to the trait in
   `pyrefly/lib/lsp/non_wasm/server.rs`
2. Add the implementation for `Server` in the `impl TspInterface for Server`
   block (same file, near the bottom)
3. The implementation should be a thin wrapper — typically:
   ```rust
   fn my_method(&self, ...) -> ... {
       let transaction = self.state.transaction();
       // Use existing Transaction/State methods
   }
   ```

---

## Snapshot Validation Pattern

All requests that accept a `snapshot` parameter must validate it:

```rust
let current = self.get_snapshot();
if params.snapshot != current {
    return Err(tsp_types::snapshot_outdated_error());
}
```

Use the existing `snapshot_outdated_error()` helper from `tsp_types::common`.

---

## Error Handling Pattern

- Use `tsp_types::error_response(id, code, message)` for building error
  responses in the dispatcher.
- Use `tsp_types::snapshot_outdated_error()` for stale snapshots.
- Return `lsp_server::ResponseError` from handler methods so the dispatcher
  can convert them to `Response`.
- For internal errors (panics in pyrefly code), catch them at the dispatcher
  level and return `ErrorCode::InternalError`.

---

## Type Conversion Guidelines (Milestones 4–6)

When building the pyrefly Type → TSP Type converter:

- **Check `crates/pyrefly_types/src/`** for existing helpers before
  destructuring a `Type` manually. The `AGENTS.md` explicitly requires this.
- Use a `TypeConverter` struct with a `HashMap<TypeId, i32>` for cycle
  detection. When a type is seen twice, emit a `TypeReferenceType` instead.
- The converter must be **pure** — no mutation of pyrefly state.
- **TypeFlags** should be derived from pyrefly type properties. Start with
  `TypeFlags::None` and set flags based on type characteristics.
- For **SynthesizedType**, generate minimal valid `.pyi` stub content. Keep
  stubs simple — they just need to be parseable Python.
- Write dedicated unit tests for the converter, exercising each type variant
  independently.

---

## Workflow for Each Milestone

1. **Write a failing integration test first** (as per `AGENTS.md`)
2. Add the `TestTspServer` helper method (e.g., `resolve_import()`)
3. Add the handler file in `pyrefly/lib/tsp/requests/`
4. Extend `TspInterface` if needed
5. Wire the match arm in the dispatcher
6. Run `cargo clippy --all-targets` — fix any warnings
7. Run `cargo test <test_name>` — make the test pass
8. Add unit tests for the params/response types
9. Add edge-case integration tests (error paths, empty results)
10. Run `python3 test.py --no-test --no-conformance` for formatting/linting
11. Commit

---

## Commands

```bash
# Run a specific test
cargo test test_tsp_resolve_import_basic

# Run all TSP tests
cargo test tsp

# Clippy check
cargo clippy --all-targets

# Format + lint (required before commit)
python3 test.py --no-test --no-conformance

# Full test suite (run when confident)
python3 test.py
```

---

## Things to Avoid

- **Do not add new dependencies** to `Cargo.toml` unless absolutely necessary.
  The crate already has `serde`, `serde_json`, `lsp-server`, `lsp-types`,
  `tracing`, and `anyhow`.
- **Do not create wrapper types** around existing pyrefly types just for TSP.
  Convert at the boundary, don't wrap.
- **Do not modify the generated `protocol.rs`**. If the generated types are
  insufficient, add shims in `common.rs`.
- **Do not use `println!` or `eprintln!` in production code.** Use `tracing`
  macros (`info!`, `debug!`, `warn!`, `error!`). `eprintln!` is acceptable in
  test code.
- **Do not add `#[cfg(test)]` modules inside handler files.** Put tests in the
  dedicated test directories instead.
- **Do not use `async`**. The TSP server runs synchronously on a dedicated
  thread, matching the existing LSP pattern.
