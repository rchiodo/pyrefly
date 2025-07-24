/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    test_generic_call_happy_case,
    r#"
from typing import Never
def force_error(x: Never) -> None: ...
def f[S, T](x: S, y: T) -> tuple[S, T]: ...
force_error(f(1, "foo"))  # E: Argument `tuple[int, str]` is not assignable to parameter `x`
"#,
);

testcase!(
    test_generic_call_fails_to_solve_output_var_simple,
    r#"
from typing import Never
def force_error(x: Never) -> None: ...
def f[S, T](x: S) -> tuple[S, T]: ...
force_error(f(1))  # E: Argument `tuple[int, @_]` is not assignable to parameter `x`
"#,
);

testcase!(
    test_generic_call_fails_to_solve_output_var_union_case,
    r#"
from typing import Never
def force_error(x: Never) -> None: ...
def f[S, T](x: S, y: list[T] | None) -> tuple[S, T]: ...
force_error(f(1, None))  # E: Argument `tuple[int, @_]` is not assignable to parameter `x`
"#,
);

testcase!(
    bug = "We should specialize `type[Self@A]` to `type[A]` in the call to `A.__new__`. Also, we should not leak the `Self` type when self-specialization fails.",
    test_self_type_subst,
    r#"
from typing import assert_type, Self
class A:
    def __new__(cls) -> Self: ...
class B[T](A): ...
class C[T]: ...
assert_type(A.__new__(A), A)
assert_type(A.__new__(B[int]), B[int])
assert_type(A.__new__(C[int]), Self) # E: Argument `type[C[int]]` is not assignable to parameter `cls` with type `type[Self@A]` in function `A.__new__`

o = A()
assert_type(o.__new__(A), A)
assert_type(o.__new__(B[int]), B[int])
assert_type(o.__new__(C[int]), Self) # E: Argument `type[C[int]]` is not assignable to parameter `cls` with type `type[Self@A]` in function `A.__new__`
    "#,
);

testcase!(
    test_deprecated_call,
    r#"
from warnings import deprecated
@deprecated("function is deprecated")
def old_function() -> None: ...
old_function()  # E: Call to deprecated function `old_function`
    "#,
);

testcase!(
    test_deprecated_method_call,
    r#"
from warnings import deprecated
class C:
    @deprecated("function is deprecated")
    def old_function(self) -> None: ...

c = C()
c.old_function()  # E: Call to deprecated function `C.old_function`
    "#,
);

testcase!(
    test_deprecated_overloaded_call,
    r#"
from typing import overload
from warnings import deprecated

@overload
def f(x: int) -> int: ...
@overload
def f(x: str) -> str: ...
@deprecated("DEPRECATED")
def f(x: int | str) -> int | str:
    return x

f(0)  # E: Call to deprecated function `f`
    "#,
);

testcase!(
    test_deprecated_overloaded_signature,
    r#"
from typing import overload
from warnings import deprecated

@deprecated("DEPRECATED")
@overload
def f(x: int) -> int: ...
@overload
def f(x: str) -> str: ...
def f(x: int | str) -> int | str:
    return x

f(0)  # E: Call to deprecated overload `f`
f("foo") # No error
    "#,
);

testcase!(
    test_reduce_call,
    r#"
from functools import reduce
reduce(max, [1,2])
    "#,
);

testcase!(
    test_union_with_type,
    r#"
from typing import assert_type
class A:
    pass
def identity[T](x: T) -> T:
    return x
def f(condition: bool):
    if condition:
        g = type
    else:
        g = identity
    assert_type(g(A()), type[A] | A)
    "#,
);
