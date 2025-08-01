/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    test_context_annassign,
    r#"
class A: ...
class B(A): ...

xs: list[A] = [B()]
"#,
);

testcase!(
    test_context_assign_annotated_binding,
    r#"
class A: ...
class B(A): ...

xs: list[A] = []
xs = [B()]
"#,
);

testcase!(
    test_context_assign_member,
    r#"
class A: ...
class B(A): ...

class C:
    xs: list[A]

o = C()
o.xs = [B()]
"#,
);

testcase!(
    test_context_class_field_init,
    r#"
class A: ...
class B(A): ...

class C:
    xs: list[A] = [B()]
    def __init__(self):
        self.xs = [B()]
"#,
);

testcase!(
    test_context_return_annot,
    r#"
class A: ...
class B(A): ...

def f() -> list[A]:
    return [B()]
"#,
);

testcase!(
    test_context_parameter,
    r#"
class A: ...
class B(A): ...

def posonly(xs: list[A], /): ...
posonly([B()])

def pos(xs: list[A]): ...
pos([B()])
pos(xs=[B()])

def kwonly(*, xs: list[A]): ...
kwonly(xs=[B()])

def vararg(*args: list[A]): ...
vararg([B()], [B()])

def kwarg(**kwargs: list[A]): ...
kwarg(xs=[B()], ys=[B()])
"#,
);

testcase!(
    bug = "Both assignments should be allowed. When decomposing the contextual hint, we eagerly resolve vars to the 'first' branch of the union. Note: due to the union's sorted representation, the first branch is not necessarily the first in source order.",
    test_contextual_typing_against_unions,
    r#"
class A: ...
class B: ...
class B2(B): ...
class C: ...

x: list[A] | list[B] = [B2()] # E: `list[B2]` is not assignable to `list[A] | list[B]`
y: list[B] | list[C] = [B2()]
"#,
);

testcase!(
    bug = "Unpacked assignments do not currently use contextual typing",
    test_context_assign_unpacked_list,
    r#"
class A: ...
class B(A): ...

xs: list[A] = []
[*xs] = [B(), B()]  # E: `list[B]` is not assignable to `list[A]`
"#,
);

testcase!(
    test_context_for,
    r#"
class A: ...
class B(A): ...

xs: list[A] = []
for xs in [[B()]]:
    pass
"#,
);

testcase!(
    test_set_hint,
    r#"
from typing import Iterable, MutableSet, Literal
x1: set[int] = {1}
x2: set[int] = {'oops'}  # E: `set[str]` is not assignable to `set[int]`
x3: set[Literal[1]] = {2}  # E: `set[int]` is not assignable to `set[Literal[1]]`
x4: MutableSet[int] = {1}
x5: MutableSet[int] = {'oops'}  # E: `set[str]` is not assignable to `MutableSet[int]`
x6: Iterable[int] = {1}
x7: object = {1}
x8: list[int] = {1}  # E: `set[int]` is not assignable to `list[int]`
    "#,
);

testcase!(
    test_dict_hint,
    r#"
from typing import Iterable, MutableMapping, Literal
x1: dict[str, int] = {"a": 1}
x2: dict[str, int] = {"a": "oops"}  # E: `dict[str, str]` is not assignable to `dict[str, int]`
x3: dict[str, Literal[1]] = {"a": 2} # E: `dict[str, int]` is not assignable to `dict[str, Literal[1]]`
x4: MutableMapping[str, int] = {"a": 1}
x5: Iterable[str] = {"a": 1}
x6: Iterable[int] = {"oops": 1}  # E: `dict[str, int]` is not assignable to `Iterable[int]`
x7: Iterable[Literal[4]] = {4: "a"}
x8: object = {"a": 1}
x9: list[str] = {"a": 1}  # E: `dict[str, int]` is not assignable to `list[str]`
    "#,
);

testcase!(
    test_call_keyword_arg_is_context_even_for_duplicates,
    r#"
from typing import assert_type, Callable, Any
def f(cb: Callable[[int], int]) -> None: ...
def g(cb: Any) -> None: ...
f(cb = lambda x: assert_type(x, int), cb = lambda x: assert_type(x, int))  # E: Multiple values for argument `cb` # E: Parse error
g(cb = lambda x: assert_type(x, Any), cb = lambda x: assert_type(x, Any))  # E: Multiple values for argument `cb` # E: Parse error
    "#,
);

testcase!(
    test_context_list_comprehension,
    r#"
class A: ...
class B(A): ...
xs: list[A] = [B() for _ in [0]]
"#,
);

testcase!(
    test_context_set_comprehension,
    r#"
class A: ...
class B(A): ...
xs: set[A] = {B() for _ in [0]}
"#,
);

testcase!(
    test_context_dict_comprehension,
    r#"
class A: ...
class B(A): ...
class X: ...
class Y(X): ...
xs: dict[A, X] = {B(): Y() for _ in [0]}
"#,
);

testcase!(
    bug = "We should push context into generator expressions",
    test_context_generator_expr,
    r#"
from typing import Generator, Iterable
class A: ...
class B(A): ...
x0 = ([B()] for _ in [0])
x1a: Generator[list[A], None, None] = x0 # E: `Generator[list[B], None, None]` is not assignable to `Generator[list[A], None, None]`
x1b: Generator[list[A], None, None] = ([B()] for _ in [0])
x2a: Iterable[list[A]] = x0 # E: `Generator[list[B], None, None]` is not assignable to `Iterable[list[A]]`
x2b: Iterable[list[A]] = ([B()] for _ in [0])

# In theory, we should allow this, since the generator expression accepts _any_ send type,
# but both Mypy and Pyright assume that the send type is `None`.
x3: Generator[int, int, None] = (1 for _ in [1]) # E: `Generator[Literal[1], None, None]` is not assignable to `Generator[int, int, None]`

x4: Generator[int, None, int] = (1 for _ in [1]) # E: `Generator[Literal[1], None, None]` is not assignable to `Generator[int, None, int]`
"#,
);

testcase!(
    test_context_if_expr,
    r#"
class A: ...
class B(A): ...
def cond() -> bool: ...
xs: list[A] = [B()] if cond() else [B()]
"#,
);

// Still infer types for unreachable branches (and find errors in them),
// but don't propagate them to the result.
testcase!(
    test_context_if_expr_unreachable,
    r#"
class A: ...
class B(A): ...
def takes_int(x: int) -> None: ...
xs: list[A] = [B()] if True else takes_int("") # E: Argument `Literal['']` is not assignable to parameter `x` with type `int`
ys: list[A] = takes_int("") if False else [B()] # E: Argument `Literal['']` is not assignable to parameter `x` with type `int`
"#,
);

testcase!(
    test_context_yield,
    r#"
from typing import Generator, Iterator
class A: ...
class B(A): ...
def gen() -> Generator[list[A], None, None]:
    yield [B()]
def iter() -> Iterator[list[A]]:
    yield [B()]
"#,
);

testcase!(
    test_context_lambda_return,
    r#"
from typing import Callable
class A: ...
class B(A): ...
f: Callable[[], list[A]] = lambda: [B()]
"#,
);

// We want to contextually type lambda params even when there is an arity mismatch.
testcase!(
    test_context_lambda_arity,
    r#"
from typing import Callable
f: Callable[[int], None] = lambda x, y: None # E: `(x: int, y: Unknown) -> None` is not assignable to `(int) -> None`
g: Callable[[int, int], None] = lambda x: None # E: `(x: int) -> None` is not assignable to `(int, int) -> None`
"#,
);

testcase!(
    test_context_lambda_generic,
    r#"
from typing import assert_type, Callable
def foo[T](x: T) -> T: ...
assert_type(foo(lambda: None), Callable[[], None])
assert_type(foo(lambda x: str(x))(1), str)
"#,
);

// This case is tricky. The call to `f` uses `g` to determine the paramspec `P`
// We then use `P` to contextually type the lambda. Importantly, the lambda's params
// need to match, including stuff like parameter name.
testcase!(
    test_context_lambda_paramspec,
    r#"
from typing import Callable, reveal_type

def f[**P, R](f: Callable[P, R], g: Callable[P, R]) -> Callable[P, R]:
    ...

def g(x: int, y: str):
    pass

x1 = f(g, lambda x, y: None)
reveal_type(x1) # E: revealed type: (x: int, y: str) -> None

x2 = f(g, lambda x, z: None) # E: Argument `(x: int, z: Unknown) -> None` is not assignable to parameter `g` with type `(x: int, y: str) -> None`
reveal_type(x2) # E: revealed type: (x: int, y: str) -> None
"#,
);

testcase!(
    bug = "Push expected return type context through generic function call",
    test_context_return,
    r#"
from typing import Callable

class A: ...
class B(A): ...

def f[T](x: T) -> T: ...

x: list[A] = f([B()]) # TODO # E: `list[B]` is not assignable to `list[A]`

y = f([B()])
z: list[A] = y # E: `list[B]` is not assignable to `list[A]`
"#,
);

testcase!(
    test_context_ctor_return,
    r#"
class A: ...
class B(A): ...

class C[T]:
    def __init__(self, x: T) -> None: ...

x: C[list[A]] = C([B()])
"#,
);

testcase!(
    test_context_in_multi_target_assign,
    r#"
class A: ...
class B(A): ...
x: list[A]
y: list[B]
x = y = [B()]  # E: Wrong type for assignment, expected `list[A]` and got `list[B]`
    "#,
);

testcase!(
    test_context_assign_expr,
    r#"
from typing import assert_type

class A: ...
class B(A): ...

xs: list[A] = (ys := [B()]) # E: `list[B]` is not assignable to `list[A]`
assert_type(ys, list[B])
    "#,
);

testcase!(
    bug = "We do not currently propagate context through unpacked assignment",
    test_context_assign_unpacked_tuple,
    r#"
class A: ...
class B(A): ...

xs: list[A] = []
(xs, _) = ([B()], None)  # E: list[B]` is not assignable to `list[A]
"#,
);

testcase!(
    bug = "Would be nice if this worked, but requires contextual typing on overloads",
    test_context_assign_subscript,
    r#"
class A: ...
class B(A): ...

xs: list[list[A]] = [[]]
xs[0] = [B()] # E: No matching overload found
"#,
);

testcase!(
    test_generic_get_literal,
    r#"
from typing import assert_type, TypeVar, Literal

class Foo[T]:
    def __init__(self, x: T) -> None: ...
    def get(self) -> T: ...

# Should propagate the context to the argument 42
x: Foo[Literal[42]] = Foo(42)
assert_type(x.get(), Literal[42])
"#,
);

testcase!(
    test_dict_infer_error,
    r#"
from typing import assert_type, Any
def test(x: int):
    assert_type({ **x }, dict[Any, Any])  # E: Expected a mapping, got int
    assert_type({ "x": 1, **x }, dict[str, int])  # E: Expected a mapping, got int
"#,
);

testcase!(
    test_override_classvar,
    r#"
from typing import ClassVar
class A:
    CONST: ClassVar[list[int | str]]
class B(A):
    CONST = [42]
    def f(self) -> list[int | str]:
        return self.CONST
class C(B):
    CONST = ["hello world"]
    "#,
);

testcase!(
    test_override_instance_var,
    r#"
class A:
    x: list[int | str]
class B(A):
    def __init__(self):
        self.x = [42]
    def f(self) -> list[int | str]:
        return self.x
class C(B):
    def __init__(self):
        self.x = ["hello world"]
    "#,
);
