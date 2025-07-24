/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::test::util::TestEnv;
use crate::test::util::testcase_for_macro;
use crate::testcase;

testcase!(
    test_canonicalized_call,
    r#"
from typing import Literal
def foo(x: Literal[1]) -> int:
    return x.__add__(1)
"#,
);

testcase!(
    test_union_call,
    r#"
from typing import assert_type
class A:
    def foo(self) -> int: ...
class B:
    def foo(self) -> str: ...
def foo(x: A | B) -> None:
    assert_type(x.foo(), int | str)
"#,
);

testcase!(
    test_simple_call,
    r#"
from typing import assert_type
def f(x: str) -> int:
    return 1
y = f("test")
assert_type(y, int)
"#,
);

testcase!(
    test_mypy_demo,
    r#"
from typing import Any
def input(prompt: str) -> str:
    return ""

def print(msg: str, val: Any) -> None:
    pass

def plus(x: int, y: int) -> int:
    return x

number = input("What is your favourite number?")
print("It is", plus(number, 1))  # E: Argument `str` is not assignable to parameter `x` with type `int`
"#,
);

testcase!(
    test_error_in_function,
    r#"
def f(x: str) -> int:
    return x  # E: Returned type `str` is not assignable to declared return type `int`
"#,
);

testcase!(
    test_create_class,
    r#"
from typing import assert_type
class C:
    pass
x = C()
assert_type(x, C)
"#,
);

testcase!(
    test_extend_final,
    r#"
from typing import final
@final
class A: ...
class B(A): ...  # E: Cannot extend final class `A`

class C: ...
@final
class D(C): ...  # OK
"#,
);

testcase!(
    test_delitem,
    r#"
x = {"name": "John"}
del x["name"]
"#,
);

testcase!(
    test_class_method,
    r#"
from typing import assert_type
class C:
    def method(self, arg: str) -> int:
        return 1
x = C()
y = x.method("test")
assert_type(y, int)
"#,
);

testcase!(
    test_return_notimplemented,
    r#"
class C:
    def __eq__(self, other: object) -> bool:
        return NotImplemented

    def __lt__(self, other: object) -> bool:
        return NotImplemented
"#,
);

testcase!(
    test_list_class_basic,
    r#"
from typing import assert_type
x = [3]
x.append(4)
assert_type(x, list[int])
"#,
);

testcase!(
    test_list_class_inner_generic,
    r#"
x = [3]
x.append("test")  # E: Argument `Literal['test']` is not assignable to parameter `object` with type `int`
"#,
);

testcase!(
    test_shadow_var,
    r#"
from typing import assert_type, Literal
x = 1
def f(x: str) -> str:
    return x
y = x
assert_type(y, Literal[1])
"#,
);

testcase!(
    test_unordered_defs,
    r#"
def f() -> int:
    return g()  # E: Returned type `str` is not assignable to declared return type `int`
def g() -> str:
    return "test"
"#,
);

testcase!(
    test_function_uses_class,
    r#"
from typing import assert_type
class C: pass

def foo() -> C:
    x : C = C()
    return x

assert_type(foo(), C)
"#,
);

testcase!(
    test_type_as_string,
    r#"
class Foo: ...
x: "list[Foo]" = []
def f(y: "None") -> list["Foo"]:
    return x
"#,
);

testcase!(
    test_type_as_concat_string,
    r#"
x: "list" "[int]" = [] # E: Expected a type form
"#,
);

testcase!(
    test_globals,
    r#"
from typing import assert_type, Any
assert_type(__file__, str)
assert_type(__name__, str)
assert_type(__debug__, bool)
assert_type(__package__, str | None)
assert_type(__annotations__, dict[str, Any])
assert_type(__spec__, Any)
"#,
);

testcase!(
    test_doc_yes,
    r#"
"""docstring"""
from typing import assert_type
assert_type(__doc__, str)
"#,
);

testcase!(
    test_doc_no,
    r#"
from typing import assert_type
assert_type(__doc__, None)
"#,
);

testcase!(
    test_import_globals,
    r#"
import typing
typing.assert_type(typing.__name__, str)
"#,
);

testcase!(
    test_argument_shadows_type,
    r#"
class C: ...
def f(C: C, D: C) -> None:
    return None
"#,
);

testcase!(
    test_union_or_in_argument,
    r#"
def f(x: int | str) -> None:
    return None
f(1)
f("test")
f(None)  # E: Argument `None` is not assignable to parameter `x` with type `int | str`
"#,
);

testcase!(
    test_types_with_flow,
    r#"
from typing import assert_type
T = str
def f() -> T: ...
T = int

assert_type(f(), str)
"#,
);

testcase!(
    test_class_rebind_attribute,
    r#"
from typing import assert_type
def f(x: str) -> int:
    return 42

attribute = "test"

class C:
    attribute = f(attribute)

assert_type(C().attribute, int)
"#,
);

testcase!(
    test_class_attribute_lookup,
    r#"
from typing import assert_type
class C:
    x = 1

assert_type(C.x, int)
"#,
);

testcase!(
    test_str_type_triple_quote,
    r#"
value: """
    str |
    int |
    list[int]
"""

value = 1
value = "test"
value = None  # E: `None` is not assignable to variable `value` with type `int | list[int] | str`
"#,
);

testcase!(
    test_nested_func,
    r#"
from typing import assert_type
def f(x: int) -> int:
    def g(y: str) -> int:
        return x
    return g("test")
assert_type(f(1), int)
"#,
);

testcase!(
    test_final_annotated,
    r#"
from typing import Final, assert_type, Literal, cast
x: Final[int] = 1
y: Final = "test"
z: Final[str] = cast(str, "")
w: Final[int] = "bad"  # E: `Literal['bad']` is not assignable to `int`

assert_type(x, int)
assert_type(y, Literal['test'])
assert_type(z, str)
"#,
);

testcase!(
    test_final_annotated_local,
    r#"
from typing import Final

x: Final[int] = 0
x = 1  # E: `x` is marked final

y: Final = "foo"
y = "bar"  # E: `y` is marked final
"#,
);

testcase!(
    test_reveal_type,
    r#"
from typing import reveal_type
reveal_type()  # E: reveal_type needs 1 positional argument, got 0
reveal_type(1)  # E: revealed type: Literal[1]
    "#,
);

testcase!(
    test_reveal_type_expand_var,
    r#"
from typing import reveal_type
def f[T](x: T) -> T:
    return x
reveal_type(f(0))  # E: revealed type: int
    "#,
);

testcase!(
    test_forward_refs_in_bases,
    r#"
from typing import assert_type, Any

class Base[T]:
    x: T

class A(Base["int"]):
    pass

class B("Base[str]"):  # E: Cannot use string annotation `Base[str]` as a base class
    pass

assert_type(A().x, int)
assert_type(B().x, str)
    "#,
);

testcase!(
    test_class_var,
    r#"
from typing import assert_type
class B:
    x: int = 1

assert_type(B.x, int)
"#,
);

testcase!(
    test_fstring_error,
    r#"
def f(x: str) -> str:
    return x

x = f"abc{f(1)}def"  # E: Argument `Literal[1]` is not assignable to parameter `x` with type `str`
"#,
);

testcase!(
    test_ternary_expression,
    r#"
from typing import assert_type, Literal

def derp() -> bool:
    ...

assert_type(0 if True else 1, Literal[0])
assert_type(0 if False else 1, Literal[1])
assert_type(0 if derp() else 1, Literal[0] | Literal[1])
"#,
);

testcase!(
    test_raise,
    r#"
def test_raise() -> None:
    raise
    raise None  # E: does not derive from BaseException
    raise BaseException
    raise BaseException()
    raise 42  # E: does not derive from BaseException
    raise BaseException from None
    raise BaseException() from None
    raise BaseException() from BaseException
    raise BaseException() from BaseException()
    raise BaseException() from 42   # E: does not derive from BaseException
"#,
);

testcase!(
    test_raise_filter_type,
    r#"
from typing import assert_type, Literal
def f(x):
    if x:
        y = "error"
        raise BaseException()
    else:
        y = "ok"
    assert_type(y, Literal["ok"])
"#,
);

testcase!(
    test_special_form_argument_counts,
    r#"
from typing import Callable, Optional, Type, TypeGuard, TypeIs

def test0() -> Type[int, int]: ...  # E: requires exactly one argument
def test1() -> TypeGuard[int, int]: ...  # E: requires exactly one argument
def test2() -> TypeIs[int, int]: ...  # E: requires exactly one argument
def test3() -> Optional[int, int]: ...  # E: requires exactly one argument
def test4() -> Callable[[], int, int]: ...  # E: requires exactly two arguments
"#,
);

testcase!(
    test_infinite_solver_1,
    r#"
from typing import Any, assert_type
x = [[], [], [[]]]
# Not too important it is precisely this type, but detect changes
assert_type(x, list[list[list[Any]]])
"#,
);

testcase!(
    test_infinite_solver_2,
    r#"
from typing import Any, assert_type
x = []
y = []
for z in []:
    x.append(z)
for z in x:
    y.append(z)
x = y
assert_type(x, list[Any])
"#,
);

testcase!(
    test_self_param_name,
    r#"
from typing import reveal_type
class C:
    def f(this):
        reveal_type(this)  # E: Self@C
    "#,
);

testcase!(
    test_getitem,
    r#"
from typing import assert_type
class C:
    pass
class D:
    __getitem__: int = 1

def f(x: list[int], y: dict[str, bool]) -> None:
    assert_type(x[0], int)
    assert_type(y["test"], bool)
    x["foo"]  # E: Cannot index into `list[int]`
    c = C()
    c[0]  # E: Cannot index into `C`\n  Object of class `C` has no attribute `__getitem__`
    d = D()
    d[0]  # E: Expected `__getitem__` to be a callable, got int
"#,
);

testcase!(
    test_dict_unpack,
    r#"
from typing import assert_type
x1: dict[str, int] = {"foo": 1, **{"bar": 2}}
x2: dict[str, int] = {"foo": 1, **{"bar": "bar"}}  # E: `dict[str, int | str]` is not assignable to `dict[str, int]`
assert_type({"foo": 1, **{"bar": "bar"}}, dict[str, int | str])
{"foo": 1, **1}  # E: Expected a mapping, got Literal[1]
"#,
);

testcase!(
    test_dict_unpack_mapping,
    r#"
from typing import Mapping, assert_type
def test(m: Mapping[str, int]) -> None:
    x1: dict[str, int] = {**m}
    x2: dict[int, int] = {**m} # E: `dict[str, int]` is not assignable to `dict[int, int]`
    assert_type({"foo": 1, **m}, dict[str, int])
"#,
);

testcase!(
    test_dict_unpack_subclass,
    r#"
from typing import assert_type
class Counter[T](dict[T, int]): ...
def test(c: Counter[str]) -> None:
    x1: dict[str, int] = {**c}
    x2: dict[int, int] = {**c}  # E: `dict[str, int]` is not assignable to `dict[int, int]`
    assert_type({"foo": 1, **c}, dict[str, int])
"#,
);

testcase!(
    test_dict_unpack_subtype,
    r#"
from typing import Literal

d1: dict[str, Literal[1]] = {}
d2: dict[str, int] = {**d1}
"#,
);

testcase!(
    test_iterable,
    r#"
from typing import Iterable, assert_type
def f(x: Iterable[int]):
    for i in x:
        assert_type(i, int)
"#,
);

testcase!(
    test_complex,
    r#"
z: complex =  3 + 4j
    "#,
);

testcase!(
    test_iterable_error,
    r#"
class A:
    pass
def f(x: A):
    for _ in x:  # E: Type `A` is not iterable
        pass
    "#,
);

testcase!(
    test_iterable_class_error,
    r#"
class A:
    pass
for _ in A:  # E: Type `type[A]` is not iterable
    pass
    "#,
);

testcase!(
    test_iterable_generic_class,
    r#"
from typing import Iterator, assert_type
class M(type):
    def __iter__(self) -> Iterator[int]: ...
class A[T](metaclass=M):
    pass
class B[T]:
    pass
for x in A[str]:
    assert_type(x, int)
for _ in B[str]:  # E: Type `type[B[str]]` is not iterable
    pass
    "#,
);

testcase!(
    test_iterable_bad_callable,
    r#"
from typing import Self
class A:
    __iter__: bool
class B:
    def __iter__(self) -> Self:
        return self
    __next__: str
def f(x: A, y: B):
    for _ in x:  # E: Type `A` is not iterable
        pass
    for _ in y:  # E: Type `B` is not iterable
        pass
    "#,
);

testcase!(
    test_iterable_bad_iterator,
    r#"
class A:
    def __iter__(self) -> None:
        return None
def f(x: A):
    for _ in x:  # E: Type `A` is not iterable
        pass
    "#,
);

testcase!(
    test_getitem_iteration,
    r#"
from typing import assert_type
class A:
    def __getitem__(self, i: int) -> str:
        return ""
def f(x: A):
    for s in x:
        assert_type(s, str)
    "#,
);

testcase!(
    test_getitem_iteration_bad,
    r#"
class A:
    def __getitem__(self, s: str) -> str:
        return s
def f(x: A):
    for _ in x:  # E: Type `A` is not iterable\n  Argument `int` is not assignable to parameter `s` with type `str` in function `A.__getitem__`
        pass
    "#,
);

testcase!(
    test_getitem_cannot_iterate_class,
    r#"
class A:
    def __getitem__(self, i: int) -> int: ...
for a in A:  # E: Type `type[A]` is not iterable
    pass
    "#,
);

testcase!(
    test_getitem_cannot_iterate_generic_class,
    r#"
class A[T]:
    def __getitem__(self, i: int) -> int: ...
def f(x: type[A[int]]):
    for a in x:  # E: Type `type[A[int]]` is not iterable
        pass
    "#,
);

testcase!(
    test_not_iterable,
    r#"
for _ in None:  # E: `None` is not iterable
    pass
    "#,
);

testcase!(
    test_assert,
    r#"
def foo() -> str: ...
assert foo(42)  # E: Expected 0 positional arguments
assert False, foo(42)  # E: Expected 0 positional arguments
    "#,
);

testcase!(
    test_subscript_error,
    r#"
class A:
    def __getitem__(self, i: int) -> int:
        return i
def f(a: A):
    return a["oops"]  # E: Argument `Literal['oops']` is not assignable to parameter `i` with type `int`
    "#,
);

testcase!(
    test_invalid_annotation,
    r#"
val = 42
def foo(arg): ...
def test(
    a: foo(arg=val),  # E: function call cannot be used in annotations
    b: lambda: None,  # E: lambda definition cannot be used in annotations
    c: [foo(arg=val)], # E: list literal cannot be used in annotations
    d: (1, 2), # E: tuple literal cannot be used in annotations
    e: 1 + 2,  # E: expression cannot be used in annotations
): ...
"#,
);

testcase!(
    test_invalid_type_arguments,
    r#"
from typing import assert_type
x: list[int, int] = []  # E: Expected 1 type argument for `list`, got 2
assert_type(x, list[int])
    "#,
);

testcase!(
    bug = "TODO",
    test_type_of_type,
    r#"
class C:
    pass
c1: type[C] = C
# TODO(stroxler): Handle `type[Any]` correctly here.
c2: type[C, C] = C  # E: Expected 1 type argument for `type`, got 2
    "#,
);

testcase!(
    test_annotated,
    r#"
from typing import Annotated, assert_type
def f(x: Annotated[int, "test"], y: Annotated[int, "test", "test"]):
    assert_type(x, int)
    assert_type(y, int)
    "#,
);

testcase!(
    test_no_backtracking,
    r#"
from typing import assert_type
def foo(x: tuple[list[int], list[int]] | tuple[list[str], list[str]]) -> None: ...
def test(x: list[str]) -> None:
    y = ([], x)
    # Because we pin down the `[]` first, we end up with a type error.
    # If we had backtracking we wouldn't.
    foo(y)  # E: Argument `tuple[list[int], list[str]]` is not assignable to parameter `x` with type `tuple[list[int], list[int]] | tuple[list[str], list[str]]`
"#,
);

testcase!(
    test_reassign_parameter,
    r#"
def foo(x: int):
    x = "test"  # E: `Literal['test']` is not assignable to variable `x` with type `int`
"#,
);

testcase!(
    test_builtins_type_constructor,
    r#"
from typing import assert_type
class Foo:
    @classmethod
    def method(cls, x: str) -> int:
        return 1
    def g(self):
        assert_type(type(self).method("tst"), int)
x = Foo()
assert_type(type(x), type[Foo])
"#,
);

testcase!(
    test_anywhere_binding,
    r#"
from typing import assert_type, Literal
x = 1
def foo():
    assert_type(x, Literal['test', 1])
foo()
x = "test"
"#,
);

testcase!(
    test_anywhere_class,
    r#"
from typing import assert_type
class C:
    def p(self) -> int: ...
    def p(self) -> str: ...

assert_type(C().p(), str)
"#,
);

testcase!(
    test_force_default_in_constructor,
    r#"
from typing import assert_type, Generic, TypeVar
T = TypeVar("T", default=str)
class C(Generic[T]): ...
assert_type(C(), C[str])
"#,
);

testcase!(
    test_identity_applied_to_list,
    r#"
from typing import assert_type
def id[T](x: T) -> T:
    return x

assert_type(id([0]), list[int])
"#,
);

testcase!(
    test_unpack_in_list_literal,
    r#"
from typing import assert_type
def test(x: list[int], y: list[str]):
    assert_type([*x, 1], list[int])
    assert_type([*x, "test"], list[int | str])
    assert_type([*x, *y], list[int | str])
    [*1]  # E: Expected an iterable
"#,
);

testcase!(
    test_union_never,
    r#"
from typing import Never, assert_type
def fail() -> Never: ...
def f(x: int):
    y = x or fail()
    assert_type(y, int)
    "#,
);

testcase!(
    test_type_of_class,
    r#"
from typing import assert_type
class A: pass
assert_type(A, type[A])
    "#,
);

testcase!(
    test_compare_int_str_error,
    r#"
0 < "oops"  # E: Argument `Literal['oops']` is not assignable to parameter `value` with type `int`
    "#,
);

testcase!(
    test_contains_error,
    r#"
class C:
    def __contains__(self, x: int) -> bool:
        return True
def f(c: C, x: int, y: str):
    x in c  # OK
    y in c  # E: Argument `str` is not assignable to parameter `x` with type `int`
    "#,
);

testcase!(
    test_mangled_syntax,
    r#"
# This parse error results in two identical Identifiers,
# which previously caused a panic.
# It should probably not produce identical identifiers - https://github.com/astral-sh/ruff/issues/16140
f"{None for y}" # E: Parse # E: Parse # E: Parse # E: Parse # E: Parse
"#,
);

testcase!(
    test_empty_if,
    r#"
# This parse error results in two identical Identifiers, which previously caused a panic.
a = True if # E: Parse 
"#,
);

testcase!(
    test_mangled_for,
    r#"
# This has identical Identifiers in the AST, which seems like the right AST.
for # E: Parse
"#,
);

testcase!(
    test_invalid_return,
    r#"
def f(x: str): ...
return f(0) # E: Invalid `return` outside of a function # E: Argument `Literal[0]` is not assignable to parameter `x` with type `str`
"#,
);

testcase!(
    test_class_field_init_error,
    r#"
class C:
    x: int = oops  # E: Could not find name `oops`
    "#,
);

testcase!(
    test_pyrereadonly,
    TestEnv::one("pyre_extensions", "class PyreReadOnly[T]: pass"),
    r#"
from pyre_extensions import PyreReadOnly
def f(x: PyreReadOnly[str]):
    pass
f("test")
    "#,
);

testcase!(
    test_resolving_any_correctly,
    r#"
import typing
x: typing.Any = 1
class Any: ...
a: Any = Any()
"#,
);

testcase!(
    test_assert_type_forward_ref,
    r#"
from typing import assert_type
def test(x: "ForwardRef") -> None:
    assert_type(x, "ForwardRef")
class ForwardRef:
    pass
    "#,
);

testcase!(
    test_assert_type_variations,
    r#"
import typing
# Calling by fully qualified name should work.
typing.assert_type(0, str)  # E: assert_type(Literal[0], str) failed
# Make sure that calling by bare name without importing performs the assertion, as this is very convenient for debugging.
# It's fine if a name error is also generated.
assert_type(0, str)  # E: assert_type(Literal[0], str) failed  # E: Could not find name `assert_type`
    "#,
);

testcase!(
    test_reveal_type_variations,
    r#"
import typing
# Calling by fully qualified name should work.
typing.reveal_type(0)  # E: revealed type: Literal[0]
# Make sure that calling by bare name without importing reveals the type, as this is very convenient for debugging.
# It's fine if a name error is also generated.
reveal_type(0)  # E: revealed type: Literal[0]  # E: Could not find name `reveal_type`
    "#,
);

testcase!(
    test_cast,
    r#"
from typing import assert_type, cast

x = cast(str, 1)
assert_type(x, str)

y = cast("str", 1)
assert_type(y, str)

z = cast(val=1, typ=str)
assert_type(z, str)

w = cast(val=1, typ="str")
assert_type(w, str)

cast()  # E: `typing.cast` missing required argument `typ`  # E: `typing.cast` missing required argument `val`
cast(1, 1)  # E: First argument to `typing.cast` must be a type
    "#,
);

testcase!(
    test_special_calls_unexpected_keyword,
    r#"
from typing import assert_type, reveal_type, Literal
assert_type(0, Literal[0], oops=1)  # E: `assert_type` got an unexpected keyword argument `oops`
reveal_type(0, oops=1)  # E: revealed type: Literal[0]  # E: `reveal_type` got an unexpected keyword argument `oops`
    "#,
);

testcase!(
    test_special_calls_alias,
    r#"
from typing import assert_type, reveal_type
at = assert_type
rt = reveal_type
at(0, str)  # E: assert_type(Literal[0], str) failed
rt(0)  # E: revealed type: Literal[0]
    "#,
);

testcase!(
    test_special_calls_name_clash,
    r#"
def assert_type():
    pass
def reveal_type(x, y, z):
    pass
assert_type()
reveal_type(1, 2, 3)
    "#,
);

testcase!(
    test_special_calls_return,
    r#"
from typing import Literal, assert_type, reveal_type

a = assert_type(0, str) # E: assert_type(Literal[0], str) failed
assert_type(a, Literal[0]) 

b = reveal_type(0) # E: revealed type: Literal[0]
assert_type(b, Literal[0])
    "#,
);

// Regression test for a bug where special exports were no longer recognized
// after a loop, due to flow styles being dropped.
testcase!(
    test_special_export_after_loop,
    r#"
from enum import Enum
from typing import assert_type

for _ in []:
    pass

# Enum's functional form is detected via a special export.
X = Enum('X', ['X'])
assert_type(X, type[X])
    "#,
);

testcase!(
    test_bad_type_in_cast,
    r#"
from typing import cast
cast(lambda x: x, 1)  # E: First argument to `typing.cast` must be a type
# Passing a listcomp as a type is nonsense; it's okay if we don't handle it optimally as long as we don't crash.
cast([x for x in []], 1)  # E: First argument to `typing.cast` must be a type  # E: Could not find name `x`
    "#,
);

testcase!(
    test_typing_alias,
    r#"
from typing import Dict, assert_type
y: Dict[str, int] = {"test": "test"} # E: `dict[str, str]` is not assignable to `dict[str, int]`
assert_type(y, dict[str, int])
"#,
);

testcase!(
    test_simple_attr,
    r#"
class A:
    pass
class B:
    def __radd__(self, other):
        return 42
A() + B()
    "#,
);

testcase!(
    test_function_stub,
    r#"
def not_a_stub() -> int:  # E: Function declared to return `int` but is missing an explicit `return`
    pass

def is_a_stub0(self) -> int:  # No error expected here.
    ...

def is_a_stub1(self) -> int:  # No error expected here.
    """
    Some docstring
    """
    ...
    "#,
);

testcase!(
    test_simple_operations,
    r#"
from typing import Literal, assert_type
def A(x: int | Literal[0], y: int | Literal[255]):
    assert_type(x - y, int)
    "#,
);

testcase!(
    test_index_any,
    r#"
from typing import Any, assert_type

def foo(x):
    assert_type(x[0], Any)
"#,
);

testcase!(
    test_module_type,
    r#"
import types
def f(x: types.ModuleType):
    pass
f(types)
    "#,
);

testcase!(
    test_function_and_method_types,
    r#"
import types
def is_func(x: types.FunctionType):
    pass
def is_method(x: types.MethodType):
    pass
class A:
    def f(self):
        pass
is_func(A.f)  # OK
is_method(A.f)  # E:
is_func(A().f)  # E:
is_method(A().f)  # OK
    "#,
);

testcase!(
    test_class_with_metaclass_is_type,
    r#"
class M(type):
    pass
class A(metaclass=M):
    pass
def f(x: type):
    pass
f(A)
    "#,
);

testcase!(
    test_list_star_with_hint,
    r#"
x: list[str] | int = ["a", *["b"]]
"#,
);

testcase!(
    test_negative_subscript_bytes_no_panic,
    r#"
# panic reported in https://github.com/facebook/pyrefly/issues/502
b''[-1]  # E: Index `-1` out of range for bytes with 0 elements
"#,
);

testcase!(
    test_parse_error,
    r#"
if  # E: Parse error: Expected an expression
"#,
);

testcase!(
    test_unpack_union,
    r#"
from typing import Sequence, assert_type

def foo(a: list[str] | Sequence[str]) -> None:
    assert_type({'a', 'b', *a}, set[str])
    "#,
);

testcase!(
    test_nonint_slice,
    r#"
from typing import assert_type
class A:
    def __getitem__[T](self, x: T) -> T:
        return x
assert_type(A()["":"":""], slice[str, str, str])
    "#,
);

testcase!(
    test_bool_and_literal_true_false,
    r#"
from typing import Literal
def f0(x: bool) -> Literal[True, False]:
    return x
def f1(x: bool) -> Literal[False, True]:
    return x
def f2(x: bool) -> Literal[False, True, 42]:
    return x
    "#,
);

testcase!(
    test_bool_nested,
    r#"
from typing import Literal, reveal_type

def f(b: bool, x: int | Literal[True], y: int | Literal[False]):
    reveal_type(x if b else y) # E: revealed type: bool | int
"#,
);

testcase!(
    test_literal_union,
    r#"
from typing import Literal, LiteralString, reveal_type

reveal_type(Literal[True, False])  # E: revealed type: type[bool]
reveal_type(Literal[4] | int)  # E: revealed type: type[int]
reveal_type(LiteralString | Literal["test"]) # E: revealed type: type[LiteralString]
reveal_type(LiteralString | str) # E: revealed type: type[str]
reveal_type(Literal[True] | bool) # E: revealed type: type[bool]

def f(cond: bool, x: LiteralString, y: str):
    reveal_type(x if cond else y)  # E: revealed type: str
"#,
);

testcase!(
    test_typing_type_as_type_any,
    r#"
from typing import Type
def f(x: Type) -> None: ...
def g(x: type) -> None: ...

f(int)
f(Type)
f(type)
f(42)  # E: not assignable to parameter `x` with type `type[Unknown]`

g(int)
g(Type)
g(type)
g(42)  # E: not assignable to parameter `x` with type `type`
"#,
);

testcase!(
    test_round,
    r#"
from typing import assert_type
assert_type(round(0.123456789, 3), float)
assert_type(round(0.123456789), int)
"#,
);

testcase!(
    test_aug_assign_star_no_panic,
    r#"
# This used to panic: see https://github.com/facebook/pyrefly/issues/454
*a += 0  # E: Parse error: Invalid augmented assignment target  # E: Could not find name `a`
    "#,
);

testcase!(
    test_debug,
    r#"
from typing import assert_type
assert_type(__debug__, bool)
    "#,
);

testcase!(
    test_invalid_assignment_no_panic,
    r#"
-a=a=  # E: Parse error: Invalid assignment target  # E: Parse error: Expected an expression
    "#,
);

testcase!(
    test_invalid_dunder_bool,
    r#"
class NotBoolable:
    __bool__: int = 3

# bool()
y = bool(NotBoolable())  # E: `NotBoolable.__bool__` has type `int`, which is not callable

# if expressions
x = 0 if NotBoolable() else 1  # E: `NotBoolable.__bool__` has type `int`, which is not callable

# if statements
if NotBoolable(): ...  # E: `NotBoolable.__bool__` has type `int`, which is not callable

# while statements
while NotBoolable(): ...  # E: `NotBoolable.__bool__` has type `int`, which is not callable

# expression evaluating to NotBoolable
def f() -> NotBoolable:
  return NotBoolable()

if (f() if True else None): ...  # E: `NotBoolable.__bool__` has type `int`, which is not callable
"#,
);

// Check that we don't raise false positives (or true positives that we want to avoid failing on)
// for some corner cases
testcase!(
    test_valid_dunder_bool,
    r#"

from typing import Any, Literal, Never

# Any is always assumed to be valid
def f(x):
  if x: ...

def g(x: Any):
  if x: ...

# Never does not raise a type error
def g() -> Never:
    raise Exception()

if g(): ...

# Union types are not checked due to risk of false positives
class B:
  def __bool__(self) -> bool:
    return True

class C:
  def __bool__(self) -> Literal[False]:
    return False

class D:
    __bool__ = 42

def j(x: B | C):
    if x: ...

# Invalid, but we don't check
def k(x: B | D):
    if x: ...

def l(x: int | None):
    if x: ...
"#,
);

testcase!(
    test_self_subscript,
    r#"
from collections.abc import Mapping

class Repro(Mapping):
    def __init__(self, settings) -> None:
        self._settings = settings

    def __getitem__(self, key):
        return self._settings[key]

    def __len__(self) -> int:
        return len(self._settings)

    def __iter__(self):
        raise NotImplementedError

    def f(self):
        print(self)
        print(self["test"])
"#,
);

testcase!(
    test_sum_map,
    r#"
from typing import Any, Callable
def g(f: Callable[[Any], int], inputs: Any) -> None:
    sum(map(f, inputs))
    "#,
);

testcase!(
    test_legacy_typevar_revealed_type,
    r#"
from typing import reveal_type, TypeVar

T = TypeVar("T")
TypeForm = type[T]

reveal_type(T)  # E: type[TypeVar[T]]
reveal_type(TypeForm)  # E: revealed type: type[type[T]]
    "#,
);

testcase!(
    test_union_function_exponential,
    r#"
# This used to take an exponential amount of time to type check
from typing import Any, Callable

def check(f: Callable[[int], bool] | Callable[[str], bool]) -> Any:
    f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(True)))))))))))))))))))))))) # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E:
"#,
);

testcase!(
    test_union_function_exponential2,
    r#"
# This used to take an exponential amount of time to type check
from typing import Callable, TypeVar

T = TypeVar('T', bound=Callable[[int], bool] | Callable[[str], bool])

def check(f: T) -> T:
    f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(f(True)))))))))))))))))))))))) # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E: # E:
    return f
"#,
);

#[test]
fn test_panic_on_unicode() {
    // This used to panic, see https://github.com/facebook/pyrefly/issues/501
    let _ = testcase_for_macro(TestEnv::new(), "e\u{81}\n", file!(), line!());
    // We manually ignore the missing expectations, because adding `# E:` would change the test.
}

testcase!(
    test_crash_on_invalid_walrus,
    r#"
# Used to crash, https://github.com/facebook/pyrefly/issues/518
if"":=  # E: Assignment expression target must be an identifier # E: Expected an expression
"#,
);

testcase!(
    test_check_invalid_rhs,
    r#"
def f(x): pass

1 := f() # E: Expected a statement # E: Missing argument `x`
1 = f() # E: Invalid assignment target # E: Missing argument `x`
"#,
);

testcase!(
    test_invalid_type_as_string,
    r#"
# Used to crash https://github.com/facebook/pyrefly/issues/517
t: "õ" # E: Could not find name `õ`
"#,
);

testcase!(
    test_loop_forever,
    r#"
# Used to loop forever, https://github.com/facebook/pyrefly/issues/519

# Note: Removing any element makes this test pass.
while True:
    z = "" if True else ""
    break
else:
    exit(1)


def func() -> int:
    return 1
"#,
);

testcase!(
    test_nested_self_init,
    r#"
# From https://github.com/facebook/pyrefly/issues/444, this used to error
class MyException:
    def __init__(self) -> None:
        self.x = ""
        self


def f(x: MyException):
    x.__init__()
"#,
);

testcase!(
    test_mapping_get,
    r#"
from typing import assert_type
import os
compiler = os.environ.get("CXX", "cl")
assert_type(compiler, str)
"#,
);

testcase!(
    test_call_union_any,
    r#"
from typing import Any
def f(x: dict[str, str] | Any):
    x["test"] = "test"
"#,
);

testcase!(
    test_call_union_two,
    r#"
from typing import Any, assert_type

def f() -> str: return "test"
def g() -> int: return 42

def op(b: bool):
    assert_type((f if b else g)(), str | int)
"#,
);

testcase!(
    test_invalid_base,
    r#"
class C(Invalid): # E: Could not find name `Invalid`
    pass

def f(x: C):
    x.unknown
    C("test")
"#,
);

testcase!(
    test_just_def,
    r#"
# Used to crash https://github.com/facebook/pyrefly/issues/620
def # E: # E:
"#,
);

testcase!(
    test_surprising_builtins,
    r#"
# These are defined in builtins, but exposed through special rules
print(__import__)
print(__build_class__)
"#,
);

testcase!(
    test_parameter_default_bad,
    r#"
def f(x: int = "test"): # E: Default `Literal['test']` is not assignable to parameter `x` with type `int`
    pass
"#,
);

testcase!(
    test_parameter_default_infer,
    r#"
from typing import reveal_type

def f(x = 1):
    reveal_type(x) # E: revealed type: int | Unknown
    return x

reveal_type(f) # E: revealed type: (x: int | Unknown = 1) -> int | Unknown
"#,
);

testcase!(
    test_self_field_gets_lost,
    r#"
# From https://github.com/facebook/pyrefly/issues/621
from typing import reveal_type

class NameTable:
    def __init__(self):
        self.names: list[str] = []

    def getName(self):
        last = ""
        for name in self.names:
            reveal_type(name) # E: revealed type: str
            last = name
        return last
"#,
);

testcase!(
    test_collapse_any,
    r#"
from typing import Any, reveal_type

def f(a, b: Any, x1: bool, x2: bool):
    c = error # E: Could not find name `error`
    x = a if x1 else b if x2 else c
    reveal_type([x]) # E: revealed type: list[Unknown]
"#,
);

testcase!(
    test_call_type_any,
    r#"
from typing import Any, assert_type

def f(x: type[Any]):
    assert_type(x(), Any)
    assert_type(x(7, arg=8), Any)
"#,
);

testcase!(
    test_bool_complex,
    r#"
def f(x:complex, y:bool):
    return x - y
    "#,
);

testcase!(
    test_bin_op_bool_num,
    r#"
def f(x:float, y:bool) -> float:
    return x * y 
"#,
);

testcase!(
    test_assert_type_class_union,
    r#"
from typing import assert_type

class A: pass

def f(condition: bool):
    if condition:
        x = A()
    else:
        x = A
    assert_type(x, A | type[A])
    "#,
);

testcase!(test_panic_docstring, "\"\"\" F\n\u{85}\"\"\"",);

testcase!(
    test_nested_list_comp,
    r#"from typing import assert_type
assert_type([a for a in [b for b in ["a", "b"]]], list[str])
"#,
);

testcase!(
    test_incomplete_nested_list_comp,
    r#"# Issue https://github.com/facebook/pyrefly/issues/455
[a for [a for # E: Could # E: Parse # E: Could # E: Parse # E: Parse
"#,
);
