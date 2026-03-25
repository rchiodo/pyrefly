/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Tests documenting current Pyrefly behavior for degenerate generic callable
//! types — cases where a type variable appears only in the return type and
//! cannot be inferred from arguments.
//!
//! Each test uses two variables:
//! - `out_a`: assigned then immediately revealed (degenerate pin → Unknown)
//! - `out_b`: flows into a real expression that pins the partial type, then revealed
//!
//! See paramspec_overload_design.md, "Forall degeneracy" section.

use crate::testcase;

// Unbounded return-only type var: produces a partial type.
// Degenerate pin gives Unknown; typed assignment pins to int.
testcase!(
    test_unsolved_typevar_unbounded,
    r#"
from typing import reveal_type
def f[T]() -> T: ...
reveal_type(f())  # E: revealed type: @_
out_a = f()
reveal_type(out_a)  # E: revealed type: Unknown
out_b: int = f()
reveal_type(out_b)  # E: revealed type: int
"#,
);

// Bounded return-only type var: same partial behavior as unbounded.
// The bound does not serve as a fallback for the degenerate pin.
testcase!(
    test_unsolved_typevar_bounded,
    r#"
from typing import reveal_type
def f[T: int]() -> T: ...
reveal_type(f())  # E: revealed type: @_
out_a = f()
reveal_type(out_a)  # E: revealed type: Unknown
out_b: int = f()
reveal_type(out_b)  # E: revealed type: int
"#,
);

// Defaulted return-only type var: default IS used, no partial type.
testcase!(
    test_unsolved_typevar_with_default,
    r#"
from typing import reveal_type
def f[T = int]() -> T: ...
reveal_type(f())  # E: revealed type: int
out_a = f()
reveal_type(out_a)  # E: revealed type: int
out_b: int = f()
reveal_type(out_b)  # E: revealed type: int
"#,
);

// Unsolved type var nested in a container: partial type propagates into list.
// Calling .append(42) pins T to int.
testcase!(
    test_unsolved_typevar_in_container,
    r#"
from typing import reveal_type
def f[T]() -> list[T]: ...
reveal_type(f())  # E: revealed type: list[@_]
out_a = f()
reveal_type(out_a)  # E: revealed type: list[Unknown]
out_b = f()
out_b.append(42)
reveal_type(out_b)  # E: revealed type: list[int]
"#,
);
