# TSP Message Implementation Plan

## Overview

The Type Server Protocol (TSP) defines 7 request types and 1 notification. Currently
**2 requests** are implemented (`getSupportedProtocolVersion`, `getSnapshot`). This plan
covers the remaining **5 requests** and **1 notification**.

### Current State

| Message | Direction | Status |
|---------|-----------|--------|
| `typeServer/getSupportedProtocolVersion` | client → server | ✅ Done |
| `typeServer/getSnapshot` | client → server | ✅ Done |
| `typeServer/getPythonSearchPaths` | client → server | ❌ Not implemented |
| `typeServer/resolveImport` | client → server | ❌ Not implemented |
| `typeServer/getComputedType` | client → server | ❌ Not implemented |
| `typeServer/getDeclaredType` | client → server | ❌ Not implemented |
| `typeServer/getExpectedType` | client → server | ❌ Not implemented |
| `typeServer/snapshotChanged` | server → client | ❌ Not implemented |

---

## Implementation Order

The messages are ordered from simplest (least coupling to pyrefly internals) to
most complex. Each milestone can be shipped independently.

### Milestone 1: `snapshotChanged` Notification

**Complexity:** Low
**Coupling:** Minimal — only touches `TspServer::process_event` and adds a
notification send.

#### What It Does
When the TSP server increments its snapshot counter (on `RecheckFinished` or
`DidChangeTextDocument`), it sends a `typeServer/snapshotChanged` notification
to the client so the client knows its cached types are stale.

#### Implementation Steps

1. **Add a helper to build the notification** in `pyrefly/lib/tsp/server.rs`
   (or a new file `pyrefly/lib/tsp/notifications/snapshot_changed.rs`)
   - Construct a `SnapshotChangedNotification` from `tsp_types::protocol`
   - Serialize to `Message::Notification` and send via `self.inner.sender()`

2. **Wire into `process_event`** — after the snapshot is incremented, send
   the notification.

#### Unit Tests (`crates/tsp_types/tests/`)
- Serialization round-trip for `SnapshotChangedNotification`
- Verify JSON wire format matches `{"jsonrpc":"2.0","method":"typeServer/snapshotChanged","params":null}`

#### Integration Tests (`pyrefly/lib/test/tsp/tsp_interaction/`)
- `test_tsp_snapshot_changed_notification_on_recheck` — open file, wait for
  recheck, assert client receives the notification
- `test_tsp_snapshot_changed_notification_on_did_change` — send didChange,
  assert client receives the notification
- `test_tsp_snapshot_changed_notification_contains_no_params` — verify params
  is null or absent

---

### Milestone 2: `getPythonSearchPaths`

**Complexity:** Low–Medium
**Coupling:** Needs access to `ConfigFile` from inner server state.

#### What It Does
Returns the list of directories Python searches for modules (stdlib, site-packages,
project paths) for a given root URI.

#### Protocol Types
- **Params:** `GetPythonSearchPathsParams { from_uri: String, snapshot: i32 }`
- **Response:** `Vec<String>` (list of path URIs)

#### Implementation Steps

1. **Extend `TspInterface`** with a method to query search paths:
   ```rust
   fn get_python_search_paths(&self, from_uri: &str) -> Vec<String>;
   ```
   Implement on `Server` by:
   - Converting `from_uri` to a `Handle`
   - Calling `state.get_config(&handle)` to get `ConfigFile`
   - Collecting `config.search_path()` and `config.site_package_path()`
   - Converting to URI strings

2. **Add handler file** `pyrefly/lib/tsp/requests/get_python_search_paths.rs`
   - Parse `GetPythonSearchPathsParams` from the request
   - Validate snapshot (return `snapshot_outdated_error()` if stale)
   - Call `self.inner.get_python_search_paths(params.from_uri)`
   - Return `Vec<String>` response

3. **Wire into dispatcher** in `TspServer::handle_tsp_request` match arm

#### Unit Tests (`crates/tsp_types/tests/`)
- `GetPythonSearchPathsParams` serialization/deserialization round-trip
- Verify `from_uri` and `snapshot` fields serialize to camelCase

#### Integration Tests (`pyrefly/lib/test/tsp/tsp_interaction/`)
- `test_tsp_get_python_search_paths_basic` — set up project with a venv,
  request search paths, assert result contains expected directories
- `test_tsp_get_python_search_paths_snapshot_outdated` — send stale snapshot,
  assert `ServerCancelled` error
- `test_tsp_get_python_search_paths_unknown_uri` — request for a URI not in
  any workspace, assert reasonable error

---

### Milestone 3: `resolveImport`

**Complexity:** Medium
**Coupling:** Needs access to import resolution via `Transaction::import_handle`
or `find_import`.

#### What It Does
Resolves a Python import (given as `ModuleName`) to its file-system URI.

#### Protocol Types
- **Params:** `ResolveImportParams { module_descriptor: ModuleName, snapshot: i32, source_uri: String }`
- **Response:** `String` (URI of the resolved module file)

#### Implementation Steps

1. **Extend `TspInterface`** with:
   ```rust
   fn resolve_import(
       &self,
       source_uri: &str,
       module_descriptor: &tsp_types::ModuleName,
   ) -> Result<String, ResponseError>;
   ```
   Implement on `Server` by:
   - Converting `source_uri` to a `Handle`
   - Converting `tsp_types::ModuleName` to pyrefly's internal `ModuleName`:
     `ModuleName` with `leading_dots` (relative import depth) and `name_parts`
   - Calling `find_import(config, module_name, origin, phantom_paths)` or
     `transaction.import_handle(handle, module_name, None)`
   - Converting result `ModulePath` to a URI string

2. **Add handler file** `pyrefly/lib/tsp/requests/resolve_import.rs`
   - Parse `ResolveImportParams`
   - Validate snapshot
   - Call `self.inner.resolve_import(...)` (the new trait method)
   - Return the URI string or error

3. **Add `ModuleName` conversion** — a small helper to convert between
   `tsp_types::ModuleName` and pyrefly's internal `ModuleName` type. Keep this
   in the handler file or a shared conversion module (e.g.,
   `pyrefly/lib/tsp/conversions.rs`).

4. **Wire into dispatcher**

#### Unit Tests (`crates/tsp_types/tests/`)
- `ResolveImportParams` serialization round-trip
- `ModuleName` with various `leading_dots` and `name_parts` combinations
- `ResolveImportOptions` default values (all `Some(false)`)

#### Integration Tests (`pyrefly/lib/test/tsp/tsp_interaction/`)
- `test_tsp_resolve_import_stdlib` — resolve `os.path`, assert URI points to
  stdlib
- `test_tsp_resolve_import_relative` — create a package with `__init__.py`,
  resolve a relative import (`from . import sub`), assert correct URI
- `test_tsp_resolve_import_not_found` — resolve a non-existent module, assert
  appropriate error
- `test_tsp_resolve_import_snapshot_outdated` — stale snapshot → error

---

### Milestone 4: `getComputedType`

**Complexity:** High
**Coupling:** Needs access to `Transaction::get_type_at` and type-to-TSP
conversion.

#### What It Does
Returns the **computed (inferred) type** at a given position in a file. This is
the type the type checker determines based on code flow (e.g., after narrowing).

#### Protocol Types
- **Params:** Currently `serde_json::Value` in `TSPRequests` — need to define
  concrete params type (likely `{ uri: String, position: Position, snapshot: i32 }`)
- **Response:** `Type` (the TSP protocol's rich type representation)

#### Implementation Steps

1. **Define concrete params type.** The generated protocol has
   `GetComputedTypeRequest` with `params: serde_json::Value`. We need to
   either:
   - Add a `GetComputedTypeParams` struct to `tsp_types::protocol` (by updating
     the generator/JSON schema), or
   - Define it manually in `tsp_types::common` as a compatibility shim.

   Fields needed:
   ```rust
   pub struct GetComputedTypeParams {
       pub uri: String,
       pub position: Position,
       pub snapshot: i32,
   }
   ```

2. **Build the pyrefly Type → TSP Type converter.** This is the biggest piece
   of work. Create `pyrefly/lib/tsp/type_conversion.rs` (or similar) that maps
   pyrefly's internal `Type` representation to `tsp_types::protocol::Type`.
   
   Key mappings:
   - pyrefly `Type::ClassType` → `tsp_types::ClassType`
   - pyrefly `Type::FunctionType` → `tsp_types::FunctionType`
   - pyrefly `Type::Union` → `tsp_types::UnionType`
   - pyrefly `Type::Module` → `tsp_types::ModuleType`
   - pyrefly `Type::Any`, `Type::Never`, `Type::Unknown` → `tsp_types::BuiltInType`
   - etc.
   
   This converter should be shared across `getComputedType`, `getDeclaredType`,
   and `getExpectedType`.

3. **Extend `TspInterface`** with:
   ```rust
   fn get_computed_type(
       &self,
       uri: &str,
       position: tsp_types::Position,
   ) -> Result<Option<tsp_types::Type>, ResponseError>;
   ```
   Implement on `Server` using the hover pattern:
   - `make_handle_if_enabled(uri)`
   - `transaction.get_module_info(&handle)` → convert position
   - `transaction.get_type_at(&handle, text_position)`
   - Convert pyrefly `Type` → TSP `Type` using the converter

4. **Add handler file** `pyrefly/lib/tsp/requests/get_computed_type.rs`

5. **Wire into dispatcher**

#### Unit Tests
- **Type conversion tests** — for each type variant, construct a pyrefly type
  and verify the TSP output matches expected JSON structure
- `GetComputedTypeParams` serialization round-trip

#### Integration Tests
- `test_tsp_get_computed_type_simple_variable` — `x = 42`, get type at `x`,
  expect an `int` class type
- `test_tsp_get_computed_type_narrowed` — `x: int | str`, after `isinstance`
  guard, expect narrowed type
- `test_tsp_get_computed_type_function` — `def foo(x: int) -> str`, get type at
  `foo`, expect a function type
- `test_tsp_get_computed_type_no_type` — position with no meaningful type
  (blank line), expect null/error
- `test_tsp_get_computed_type_snapshot_outdated` — stale snapshot → error

---

### Milestone 5: `getDeclaredType`

**Complexity:** High (but shares converter with Milestone 4)
**Coupling:** Needs `Transaction::find_definition` + type converter

#### What It Does
Returns the **declared type** at a position — the type explicitly annotated in
the source, not the narrowed/inferred type.

#### Protocol Types
- **Params:** Same shape as `getComputedType` params
- **Response:** `Type`

#### Implementation Steps

1. **Define `GetDeclaredTypeParams`** (same as `GetComputedTypeParams`)

2. **Extend `TspInterface`** with:
   ```rust
   fn get_declared_type(
       &self,
       uri: &str,
       position: tsp_types::Position,
   ) -> Result<Option<tsp_types::Type>, ResponseError>;
   ```
   Implement by:
   - Using `transaction.find_definition(handle, position, ...)`
   - Getting the type at the definition location
   - Converting via the shared type converter

3. **Add handler file** `pyrefly/lib/tsp/requests/get_declared_type.rs`

4. **Wire into dispatcher**

#### Unit Tests
- `GetDeclaredTypeParams` serialization round-trip

#### Integration Tests
- `test_tsp_get_declared_type_annotated_variable` — `x: int = 42`, declared
  type of `x` is `int`
- `test_tsp_get_declared_type_function_param` — `def foo(a: int | str)`,
  declared type of `a` is `int | str`
- `test_tsp_get_declared_type_unannotated` — `x = 42`, declared type may be
  absent or inferred
- `test_tsp_get_declared_type_snapshot_outdated` — stale snapshot → error

---

### Milestone 6: `getExpectedType`

**Complexity:** High
**Coupling:** Needs contextual type inference

#### What It Does
Returns the **expected type** at a position — the type that the surrounding
context expects at that location (e.g., the parameter type at a call site).

#### Protocol Types
- **Params:** Same shape as `getComputedType` params
- **Response:** `Type`

#### Implementation Steps

1. **Define `GetExpectedTypeParams`**

2. **Determine the contextual type resolution strategy.** This is the most
   complex message because "expected type" requires understanding context:
   - At a function call argument: the corresponding parameter's type
   - At a return statement: the function's return type
   - At an assignment target: the annotation type
   - At a comparison: the other operand's type
   
   Options:
   - **Option A:** Build on `identifier_at()` + context analysis to derive
     expected types from the surrounding AST
   - **Option B:** If pyrefly already computes expected types internally
     (e.g., for autocomplete), expose that directly
   - **Option C:** Start with a subset (function argument expected types only)
     and expand later

3. **Extend `TspInterface`** with:
   ```rust
   fn get_expected_type(
       &self,
       uri: &str,
       position: tsp_types::Position,
   ) -> Result<Option<tsp_types::Type>, ResponseError>;
   ```

4. **Add handler file** `pyrefly/lib/tsp/requests/get_expected_type.rs`

5. **Wire into dispatcher**

#### Unit Tests
- `GetExpectedTypeParams` serialization round-trip

#### Integration Tests
- `test_tsp_get_expected_type_function_argument` — `def foo(a: int): ...`,
  at call `foo(▌)`, expected type is `int`
- `test_tsp_get_expected_type_return_statement` — `def foo() -> int: return ▌`,
  expected type is `int`
- `test_tsp_get_expected_type_assignment` — `x: int = ▌`, expected type is
  `int`
- `test_tsp_get_expected_type_no_context` — position with no expected type,
  assert null/error
- `test_tsp_get_expected_type_snapshot_outdated` — stale snapshot → error

---

## Shared Infrastructure

### Type Converter (`pyrefly/lib/tsp/type_conversion.rs`)

This module is needed for Milestones 4–6. It converts pyrefly's internal `Type`
to the TSP wire format (`tsp_types::Type`). Key considerations:

- **Cycle detection:** Use `id` fields and `TypeReferenceType` to break cycles
- **ID generation:** Maintain a monotonic counter for type IDs
- **Flags mapping:** Map pyrefly type characteristics to `TypeFlags`
- **Declaration mapping:** Convert pyrefly source locations to `Node` with
  `Range` and URI
- **Stub generation for SynthesizedType:** For types without source
  declarations, generate valid `.pyi` stub content

### `TspInterface` Extensions

Each milestone that needs server state will add methods to the `TspInterface`
trait. To minimize churn, consider batching trait additions. An alternative
design is to add a single `fn state(&self) -> &State` accessor, but this
exposes too much internals. Prefer purpose-specific methods.

### Snapshot Validation

All request handlers (except `getSnapshot` and `getSupportedProtocolVersion`)
accept a `snapshot: i32` parameter. Add a shared validation helper:
```rust
fn validate_snapshot(&self, snapshot: i32) -> Result<(), ResponseError> {
    let current = self.get_snapshot();
    if snapshot != current {
        Err(snapshot_outdated_error())
    } else {
        Ok(())
    }
}
```

### Position Conversion

TSP uses its own `Position` type (from `tsp_types::protocol`). Need a helper to
convert to pyrefly's internal position representation (via `TextSize` from
`text_size` crate, same conversion used in LSP):
```rust
fn tsp_position_to_text_size(module_info: &ModuleInfo, pos: tsp_types::Position) -> TextSize
```

---

## File Structure After All Milestones

```
pyrefly/lib/tsp/
├── mod.rs
├── server.rs                           # TspServer, process_event, dispatcher
├── type_conversion.rs                  # pyrefly Type → TSP Type converter
├── notifications/
│   ├── mod.rs
│   └── snapshot_changed.rs
└── requests/
    ├── mod.rs
    ├── get_computed_type.rs
    ├── get_declared_type.rs
    ├── get_expected_type.rs
    ├── get_python_search_paths.rs
    ├── get_snapshot.rs                 # (existing)
    ├── get_supported_protocol_version.rs  # (existing)
    └── resolve_import.rs

crates/tsp_types/
├── src/
│   ├── common.rs                       # + new params types, conversion helpers
│   ├── lib.rs
│   └── protocol.rs                     # (generated, do not edit)
└── tests/
    ├── mod.rs
    ├── get_snapshot.rs                 # (existing)
    ├── protocol_types.rs              # (existing)
    ├── get_python_search_paths.rs
    ├── resolve_import.rs
    ├── get_computed_type.rs
    ├── get_declared_type.rs
    └── get_expected_type.rs

pyrefly/lib/test/tsp/tsp_interaction/
├── mod.rs
├── object_model.rs                     # (existing) + new helper methods
├── get_snapshot.rs                     # (existing)
├── get_supported_protocol_version.rs   # (existing)
├── snapshot_changed.rs
├── get_python_search_paths.rs
├── resolve_import.rs
├── get_computed_type.rs
├── get_declared_type.rs
└── get_expected_type.rs
```

---

## Risk & Open Questions

1. **Params types for type requests.** The generated protocol uses
   `serde_json::Value` for `getComputedType`, `getDeclaredType`, and
   `getExpectedType` params. Need to either update the protocol generator or
   add manual params types. **Recommendation:** Add manual params types in
   `tsp_types::common` for now; update generator later.

2. **Type converter fidelity.** Pyrefly's internal types are rich and may not
   map 1:1 to TSP types. Start with best-effort mapping and iterate.

3. **Expected type complexity.** `getExpectedType` may require new internal
   analysis in pyrefly. Consider shipping Milestones 1–5 first and treating
   Milestone 6 as a follow-up.

4. **Snapshot semantics.** Current snapshot increments on `DidChangeTextDocument`
   and `RecheckFinished`. Verify this matches what TSP clients expect. The
   `snapshotChanged` notification should only fire when types actually change,
   not just on file edits — but the current design increments eagerly.
