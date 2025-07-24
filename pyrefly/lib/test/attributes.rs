/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::test::util::TestEnv;
use crate::testcase;

testcase!(
    test_set_attribute,
    r#"
class A:
    x: int
def f(a: A):
    a.x = 1  # OK
    a.x = "oops"  # E: `Literal['oops']` is not assignable to attribute `x` with type `int`
    "#,
);

testcase!(
    test_set_attribute_in_unpacked_assign,
    r#"
class A:
    x: int
    y: str
def f(a: A):
    a.x, a.y = "x", "y"  # E: `Literal['x']` is not assignable to attribute `x` with type `int`
    "#,
);

testcase!(
    test_self_attribute_unannotated,
    r#"
from typing import assert_type
class A:
    def __init__(self, x: int):
        self.x = x
def f(a: A):
    assert_type(a.x, int)
    "#,
);

testcase!(
    test_unannotated_attribute_bad_assignment,
    r#"
class A:
    def __init__(self):
        self.x = 0
    def f(self):
        self.x = "oops"  # E: `Literal['oops']` is not assignable to attribute `x` with type `int`
    "#,
);

testcase!(
    test_super_object_bad_assignment,
    r#"
class A:
    a: int = 3

class B(A): pass

super(B, B()).a = 3  # E: Cannot set field `a`
    "#,
);

testcase!(
    test_super_object_delete_error,
    r#"
class A:
    a: int = 3

class B(A): pass
del super(B, B()).a # E: Cannot delete field `a`
    "#,
);

testcase!(
    test_self_attribute_assign_twice,
    r#"
from typing import assert_type
class A:
    def f(self, x: str):
        self.x = x  # E: `str` is not assignable to attribute `x` with type `int`
    def __init__(self, x: int):
        self.x = x
    "#,
);

testcase!(
    test_self_attribute_in_unrecognized_method,
    r#"
from typing import assert_type
class A:
    def f(self, x: int):
        self.x = x  # E: Attribute `x` is implicitly defined by assignment in method `f`, which is not a constructor
def f(a: A):
    assert_type(a.x, int)
    "#,
);

testcase!(
    test_inherited_attribute_in_unrecognized_method,
    r#"
from typing import assert_type
class A:
    x: int
class B(A):
    def f(self, x: int):
        self.x = x
    "#,
);

// Ref https://github.com/facebook/pyrefly/issues/370
// Ref https://github.com/facebook/pyrefly/issues/522
testcase!(
    bug =
        "Attributes initialized in `__new__` and `__init_subclass__` should not be instance-only.",
    test_cls_attribute_in_constructor,
    r#"
from typing import ClassVar
class A:
    def __new__(cls, x: int):
        cls.x = x # E: Instance-only attribute `x` of class `A` is not visible on the class
class B:
    def __init_subclass__(cls, x: int):
        cls.x = x # E: Instance-only attribute `x` of class `B` is not visible on the class
class C:
    x: ClassVar[int]
    def __new__(cls, x: int):
        cls.x = x
class D:
    x: ClassVar[int]
    def __init_subclass__(cls, x: int):
        cls.x = x
    "#,
);

testcase!(
    test_self_attribute_in_test_setup,
    r#"
class MyTestCase:
    def setUp(self):
        self.x = 5
    def run(self):
        assert self.x == 5
    "#,
);

testcase!(
    bug = "Example of how making methods read-write but not invariant is unsound",
    test_method_assign,
    r#"
from typing import Protocol
class X(Protocol):
    def foo(self) -> object:
        return 1
class Y:
    def foo(self) -> int:
        return 1
def func(x: X):
    x.foo = lambda: "hi"
y: Y = Y()
func(y)
y.foo()  # result is "hi"
    "#,
);

testcase!(
    test_attribute_union,
    r#"
class A:
    x: int
class B:
    x: str
def test(x: A | B):
    del x.x
    x.x = 1  # E: `Literal[1]` is not assignable to attribute `x` with type `str`
    "#,
);

testcase!(
    test_callable_boundmethod_subset,
    r#"
from typing import Callable

class C:
    def f(self, x: int, /) -> str:
        return ""
class C2:
    @classmethod
    def f(cls, x: int, /) -> str:
        return ""
class C3:
    @staticmethod
    def f(x: int, /) -> str:
        return ""
def foo(x: Callable[[int], str], c: C, c2: C2, c3: C3):
    C.f = x  # E: `(int) -> str` is not assignable to attribute `f` with type `(self: Self@C, x: int, /) -> str`
    c.f = x
    C2.f = x
    c2.f = x
    C3.f = x
    c3.f = x
    "#,
);

testcase!(
    test_use_of_class_body_scope_in_class_body_statement,
    r#"
class A:
    x: int = 5
    y: int = x
    "#,
);

testcase!(
    test_annotating_non_self_attributes,
    r#"
class A:
    x: int

class B:
    def __init__(self, a: A):
        a.x: int = 1  # E: Type cannot be declared in assignment to non-self attribute `a.x`

a: A = A()
a.x: int = 5  # E: Type cannot be declared in assignment to non-self attribute `a.x`
    "#,
);

testcase!(
    test_self_attribute_annotated_in_class_body,
    r#"
from typing import assert_type
class A:
    x: str
    def __init__(self, x: int):
        self.x = x  # E: `int` is not assignable to attribute `x` with type `str`
    "#,
);

testcase!(
    test_self_attribute_annotated_assignment,
    r#"
from typing import assert_type

class A:
    def __init__(self, x: str):
        self.x: int = x  # E: `str` is not assignable to attribute `x` with type `int`
def f(a: A):
    assert_type(a.x, int)
    "#,
);

testcase!(
    test_generic_classvar,
    r#"
from typing import ClassVar
class A[T]:
    x: ClassVar[T]  # E: `ClassVar` arguments may not contain any type variables
    y: ClassVar[list[T]]  # E: `ClassVar` arguments may not contain any type variables
    "#,
);

testcase!(
    test_self_attribute_annotated_twice,
    r#"
from typing import assert_type, Literal, Final
class A:
    x: int
    y: str
    def __init__(self):
        self.x: Literal[1] = 1
        self.y: Final = "y"
def f(a: A):
    assert_type(a.x, int)
    "#,
);

testcase!(
    bug = "TODO(stroxler): We are always promoting literals. It is sound to preserve literals for read-only attributes",
    test_final_attribute_assigned_in_init,
    r#"
from typing import assert_type, Final, Literal
class A:
    def __init__(self):
        self.x: Final = 0
def f(a: A):
    assert_type(a.x, Literal[0])  # E: assert_type(int, Literal[0])
    "#,
);

testcase!(
    test_literal_attr_with_annotation,
    r#"
from typing import ClassVar, assert_type
class C:
    x0 = 0
    x1: ClassVar = 0
assert_type(C.x0, int)
assert_type(C.x1, int)
"#,
);

testcase!(
    test_final_annotated_override,
    r#"
from typing import Final
def f() -> int: ...
class Base:
    p: Final = f()
class Derived(Base):
    p = f()  # E: `p` is declared as final in parent class `Base`
"#,
);

testcase!(
    test_self_attribute_bare_annotation,
    r#"
from typing import assert_type
class A:
    def __init__(self, x: str):
        self.x: int
        self.x = x  # E: `str` is not assignable to attribute `x` with type `int`
def f(a: A):
    assert_type(a.x, int)
    "#,
);

testcase!(
    test_attribute_inference,
    r#"
class C:
    x: list[int | str]
def f(c: C):
    c.x = [5]
    "#,
);

testcase!(
    test_set_attribute_in_init_nested,
    r#"
from typing import assert_type
class C:
    def __init__(self):
        def f():
            self.x = 0
        f()
def f(c: C):
    assert_type(c.x, int)
    "#,
);

// TODO: Should we implement simple control-flow heuristics so `C.x` is recognized here?
testcase!(
    test_set_attribute_in_init_indirect,
    r#"
class C:
    def __init__(self):
        self.f()
    def f(self):
        self.x = 0  # E: Attribute `x` is implicitly defined by assignment in method `f`, which is not a constructor
def f(c: C) -> int:
    return c.x
    "#,
);

testcase!(
    test_missing_self_parameter,
    r#"
class C:
    def f():
        pass
C().f()  # E: Expected 0 positional arguments, got 1 (including implicit `self`)
    "#,
);

testcase!(
    test_generic_instance_method,
    r#"
class C:
    def f[T](self: T, x: T):
        pass
C().f(C())  # OK
C().f(0)    # E: Argument `Literal[0]` is not assignable to parameter `x` with type `C`
    "#,
);

// Make sure we treat `callable_attr` as a bare instance attribute, not a bound method.
testcase!(
    test_callable_instance_only_attribute,
    r#"
from typing import Callable, assert_type, Literal
class C:
    callable_attr: Callable[[int], int]
    def __init__(self):
       self.callable_attr = lambda x: x
c = C()
x = c.callable_attr(42)
assert_type(x, int)
    "#,
);

// We currently treat `Callable` as not having method binding behavior. This is
// not compatible with Pyright and mypy, both of which assume in the face of
// ambiguity that the callable is probably a function or lambda.
//
// See https://discuss.python.org/t/when-should-we-assume-callable-types-are-method-descriptors/92938
testcase!(
    bug = "We probably need to treat `f` as a method here.",
    test_callable_as_class_var,
    r#"
from typing import assert_type, Callable, ClassVar
def get_callback() -> Callable[[object, int], int]: ...
class C:
    f: ClassVar[Callable[[object, int], int]] = get_callback()
assert_type(C.f(None, 1), int)
# We probably should to be treating f as a bound method here.
assert_type(C().f(None, 1), int)
"#,
);

// Mypy and Pyright treat `f` as not a method here; its actual behavior
// is ambiguous even if we assume the values are always functions or lambdas
// because the default value can be overridden by instance assignment.
//
// Our behavior is compatible, but the underlying implementation is not, we are
// behaving this way based on how we treate the Callable type rather than based
// on the absence of `ClassVar`.
//
// See https://discuss.python.org/t/when-should-we-assume-callable-types-are-method-descriptors/92938
testcase!(
    test_callable_with_ambiguous_binding,
    r#"
from typing import assert_type, Callable
def get_callback() -> Callable[[object, int], int]: ...
class C:
    f = get_callback()
assert_type(C.f(None, 1), int)
assert_type(C().f(None, 1), int)
# This is why the behavior is ambiguous - at runtime, the default `C.f` is a
# method but the instance-level shadow is not.
C().f = lambda _, x: x
"#,
);

testcase!(
    test_class_access_of_instance_only_attribute,
    r#"
from typing import assert_type, Any
class C:
    x: int
    def __init__(self, y: str):
        self.x = 0
        self.y = y
assert_type(C.x, Any)  # E: Instance-only attribute `x` of class `C` is not visible on the class
assert_type(C.y, Any)  # E: Instance-only attribute `y` of class `C` is not visible on the class
c = C("y")
assert_type(c.x, int)
assert_type(c.y, str)
"#,
);

testcase!(
    test_match_method_against_callable,
    r#"
from typing import Callable
class C:
    def f(self, x: int) -> None:
        pass
def f1(c: Callable[[int], None]):
    pass
def f2(c: Callable[[C, int], None]):
    pass
f1(C.f)  # E: Argument `(self: Self@C, x: int) -> None` is not assignable to parameter `c` with type `(int) -> None`
f1(C().f)
f2(C.f)
f2(C().f)  # E: Argument `BoundMethod[C, (self: Self@C, x: int) -> None]` is not assignable to parameter `c` with type `(C, int) -> None`
    "#,
);

testcase!(
    test_simple_inheritance,
    r#"
from typing import assert_type
class B:
    x: int

class HasBase(B):
    y: str

assert_type(HasBase().x, int)
"#,
);

testcase!(
    test_generic_multiple_inheritance,
    r#"
from typing import assert_type
class A[T]:
    x: T

class B[T]:
    y: T

class C[T](A[int], B[T]):
    z: bool

c: C[str] = C()
assert_type(c.x, int)
assert_type(c.y, str)
assert_type(c.z, bool)
"#,
);

testcase!(
    test_generic_chained_inheritance,
    r#"
from typing import assert_type
class A[T]:
    x: T

class B[T](A[list[T]]):
    y: T

class C[T](B[T]):
    z: bool

c: C[str] = C()
assert_type(c.x, list[str])
assert_type(c.y, str)
assert_type(c.z, bool)
"#,
);

testcase!(
    test_nested_class_attribute_with_inheritance,
    r#"
from typing import assert_type

class B:
    class Nested:
        x: int

class C(B):
    pass

N0: B.Nested = C.Nested()
N1: C.Nested = B.Nested()
assert_type(N1.x, int)
"#,
);

testcase!(
    test_class_generic_attribute_lookup,
    r#"
class C[T]:
    x = T

C.x  # E: Generic attribute `x` of class `C` is not visible on the class
"#,
);

testcase!(
    test_var_attribute,
    r#"
from typing import assert_type
def f[T](x: T) -> T:
    return x
class C:
    def __init__(self):
        self.x = 42
assert_type(f(C()).x, int)
    "#,
);

testcase!(
    test_never_attr,
    r#"
from typing import Never, NoReturn, assert_type
def f() -> NoReturn: ...
def g():
    x = f().x
    assert_type(x, Never)
    "#,
);

testcase!(
    test_callable_attr,
    r#"
from typing import assert_type
from types import CodeType
def f():
    pass
def g():
    assert_type(f.__code__, CodeType)
    "#,
);

testcase!(
    test_boundmethod_attr,
    r#"
from typing import assert_type
class A:
    def f(self):
        pass
def g(a: A):
    assert_type(a.f.__self__, object)
    "#,
);

testcase!(
    test_ellipsis_attr,
    r#"
x = ...
x.x  # E: Object of class `EllipsisType` has no attribute `x`
    "#,
);

testcase!(
    test_forall_attr,
    r#"
from typing import assert_type
from types import CodeType
def f[T](x: T) -> T:
    return x
assert_type(f.__code__, CodeType)
    "#,
);

testcase!(
    test_metaclass_attr,
    r#"
from typing import assert_type

class A: ...
class B[T]: ...
assert_type(A.mro(), list[type])
assert_type(B[int].mro(), list[type])

class Meta(type):
    x: int
class C(metaclass=Meta):
    pass
assert_type(C.x, int)
    "#,
);

fn env_with_stub() -> TestEnv {
    let mut t = TestEnv::new();
    t.add_with_path(
        "foo",
        "foo.pyi",
        r#"
class A:
    x: int = ...
    y: int
    "#,
    );
    t
}

testcase!(
    test_stub_initializes_attr,
    env_with_stub(),
    r#"
from typing import assert_type
from foo import A

assert_type(A.x, int)
assert_type(A.y, int)
    "#,
);

testcase!(
    test_object_getattribute,
    r#"
from typing import *
class A:
    def __getattribute__(self, name: str, /) -> int: ...
    def __setattr__(self, name: str, value: Any, /) -> None: ...
    def __delattr__(self, name: str, /) -> None: ...
class B:
    def __getattribute__(self, name: str, /) -> str: ...
a = A()
b = B()
assert_type(a.x, int)
assert_type(b.x, str)
a.x = 1
del a.x
b.x = 1  # E: Object of class `B` has no attribute `x`
del b.x  # E: Object of class `B` has no attribute `x`
    "#,
);

testcase!(
    test_object_getattr,
    r#"
from typing import assert_type

class Foo:
    def __getattr__(self, name: str) -> int: ...

def test(foo: Foo) -> None:
    assert_type(foo.x, int)
    assert_type(foo.y, int)
    foo.x = 1  # E: Object of class `Foo` has no attribute `x`
    del foo.y  # E: Object of class `Foo` has no attribute `y`
    "#,
);

testcase!(
    test_object_getattr_wrong_signature,
    r#"
from typing import assert_type

class Foo:
    def __getattr__(self, name: int) -> int: ...

def test(foo: Foo) -> None:
    assert_type(foo.x, int)  # E: Argument `Literal['x']` is not assignable to parameter `name`
    assert_type(foo.y, int)  # E: Argument `Literal['y']` is not assignable to parameter `name`
    foo.x = 1  # E: Object of class `Foo` has no attribute `x`
    del foo.y  # E: Object of class `Foo` has no attribute `y`
    "#,
);

testcase!(
    test_object_setattr,
    r#"
from typing import assert_type

class Foo:
    def __getattr__(self, name: str) -> int: ...
    def __setattr__(self, name: str, value: int) -> None: ...

def test(foo: Foo) -> None:
    foo.x = 1
    foo.x = ""  # E: Argument `Literal['']` is not assignable to parameter `value` with type `int`
    "#,
);

testcase!(
    test_object_delattr,
    r#"
from typing import assert_type

class Foo:
    def __getattr__(self, name: str) -> int: ...
    def __delattr__(self, name: str) -> None: ...

def test(foo: Foo) -> None:
    del foo.x
    "#,
);

testcase!(
    test_object_setattr_wrong_signature,
    r#"
from typing import assert_type

class Foo:
    def __getattr__(self, name: int) -> int: ...
    def __setattr__(self, name: int, value: int) -> None: ...

def test(foo: Foo) -> None:
    foo.x = 1  # E: Argument `Literal['x']` is not assignable to parameter `name` with type `int`
    "#,
);

testcase!(
    test_argparse_namespace_setattr,
    r#"
from argparse import ArgumentParser, Namespace

ap: ArgumentParser = ArgumentParser()
ap.add_argument("-b", "--bool-flag", default=False, action='store_true')
ap.add_argument("-i", "--integer", default=1, type=int)
ap.add_argument("-s", "--string-arg", type=str, default="")
args: Namespace = ap.parse_args()
if not args.string_arg:
    args.string_arg = "string-goes-here"
    "#,
);

testcase!(
    test_module_getattr,
    TestEnv::one("foo", "def __getattr__(name: str) -> int: ..."),
    r#"
from typing import assert_type
import foo
assert_type(foo.x, int)
assert_type(foo.y, int)
foo.x = 1  # E: No attribute `x` in module `foo`
del foo.y  # E: No attribute `y` in module `foo`
    "#,
);

testcase!(
    test_any_subclass,
    r#"
from typing import Any, assert_type

class A(Any):
    x: int

class B(A):
    y: str

def test0(a: A, b: B, ta: type[A]) -> None:
    assert_type(a.x, int)
    assert_type(b.x, int)
    assert_type(b.y, str)
    assert_type(ta.mro(), list[type])

    assert_type(a.z, Any)
    assert_type(b.z, Any)
    assert_type(ta.z, Any)

class Test(B):
    def m(self) -> None:
        assert_type(super().z, Any)
    @classmethod
    def m2(cls) -> None:
        assert_type(super().z, Any)
    "#,
);

testcase!(
    test_field_using_method_scope_type_variable,
    r#"
from typing import assert_type, Any

class C:
    def __init__[R](self, field: R):
        self.field = field  # E: Cannot initialize attribute `field` to a value that depends on method-scoped type variable `R`

c = C("test")
assert_type(c.field, Any)
"#,
);

testcase!(
    test_attr_unknown,
    r#"
class Op:
    default: str
class Namespace:
    def __getattr__(self, op_name):
        if op_name == "__file__":
            return "test"
        return Op()
x = Namespace().some_op.default # E:  Object of class `str` has no attribute `default

"#,
);

testcase!(
    test_with,
    r#"
class C:
    def __init__(self) -> None:
        self.prev = False
    def __enter__(self) -> None:
        self.prev = False
    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        self.prev = False
    def __new__(cls, orig_func=None):
        if orig_func is None:
            return super().__new__(cls)
def f():
    with C():  # E: `NoneType` has no attribute `__enter__`  # E: `NoneType` has no attribute `__exit__`
        pass
    "#,
);

testcase!(
    bug = "TODO(stroxler): We need to define the semantics of generic class nesting and avoid leaked type variables",
    test_class_nested_inside_generic_class,
    r#"
from typing import Any, assert_type, reveal_type
class Outer[T]:
    class Inner:
        x: T | None = None
assert_type(Outer[int].Inner, type[Outer.Inner])
assert_type(Outer.Inner, type[Outer.Inner])
reveal_type(Outer[int].Inner.x)  # E: revealed type: T | None
reveal_type(Outer.Inner.x)  # E: revealed type: T | None
reveal_type(Outer[int].Inner().x)  # E: revealed type: T | None
reveal_type(Outer.Inner().x)  # E: revealed type: T | None
   "#,
);

testcase!(
    test_attr_base,
    r#"
def f(x, key, dict):
    for param in x:
        if key in dict:
            pass
    "#,
);

testcase!(
    test_classvar_no_value,
    r#"
from typing import ClassVar, assert_type
class C:
    x: ClassVar[int]
assert_type(C.x, int)
    "#,
);

testcase!(
    test_set_attribute_on_typevar_annotated_self,
    r#"
from typing import Self, TypeVar
Self2 = TypeVar('Self2', bound='A')
class A:
    x: int
    def f1(self: Self, x: int, y: float) -> Self:
        self.x = x
        self.x = y  # E: `float` is not assignable to attribute `x` with type `int`
        return self
    def f2(self: Self2, x: int, y: float) -> Self2:
        self.x = x
        self.x = y  # E: `float` is not assignable to attribute `x` with type `int`
        return self
    def f3[Self3: A](self: Self3, x: int, y: float) -> Self3:
        self.x = x
        self.x = y  # E: `float` is not assignable to attribute `x` with type `int`
        return self
    "#,
);

testcase!(
    test_typevar_attr_default_error,
    r#"
from typing import Any, assert_type
class A[T1 = int, T2]:  # E:
    x: T2
def f(a: A):
    assert_type(a.x, Any)
    "#,
);

testcase!(
    test_union_base_class,
    r#"
from typing import Any, assert_type
class A:
    x: int
class B:
    x: str
try:
    C = A
except:
    C = B
class D(C): # E: Invalid base class: `A | B`
    pass
def f(d: D):
    assert_type(d.x, Any)
    "#,
);

testcase!(
    test_getattr_dispatch_for_metaclass,
    r#"
from typing import assert_type
class EMeta(type):
    def __getattr__(self, attr: str) -> int: ...
class E(int, metaclass=EMeta):
    pass
assert_type(E.EXAMPLE_VALUE, int)
    "#,
);

testcase!(
    test_getattr_selection_for_class_object_w_metaclass,
    r#"
from typing import assert_type
class EMeta(type):
    def __getattr__(self, attr: str) -> int: ...
class E(metaclass=EMeta):
    def __getattr__(self, attr: str) -> str: ...
assert_type(E.EXAMPLE_VALUE, int)
    "#,
);

testcase!(
    test_getattr_selection_for_class_object_no_metaclass,
    r#"
from typing import assert_type
class E:
    def __getattr__(self, attr: str) -> str: ...
E.EXAMPLE_VALUE # E: Class `E` has no class attribute `EXAMPLE_VALUE`
    "#,
);

testcase!(
    test_attribute_access_on_type_var,
    r#"
from typing import assert_type, Any
class Foo:
    x: int = 1
def f[T: Foo](y: T) -> T:
    assert_type(y.x, int)
    assert_type(T.x, int)
    return y
    "#,
);

testcase!(
    test_attribute_access_on_quantified_bound_by_union,
    r#"
from typing import assert_type
class Foo:
    x: int
class Bar:
    x: str
def f[T: Foo | Bar](y: T, z: Foo | Bar) -> T:
    assert_type(z.x, int | str)
    assert_type(y.x, int | str)
    return y
    "#,
);

testcase!(
    test_type_magic_dunder_compare,
    r#"
def test(x: type[int], y: type[int]) -> None:
    # These are OK because `type` inherits `__eq__` and `__ne__` from `object`.
    x == y
    x != y

    # These are always OK
    x is y
    x is not y

    # These are not OK because the corresponding dunder methods are not defined on `type`
    x < y       # E: `<` is not supported between `type[int]` and `type[int]`
    x <= y      # E: `<=` is not supported between `type[int]` and `type[int]`
    x > y       # E: `>` is not supported between `type[int]` and `type[int]`
    x >= y      # E: `>=` is not supported between `type[int]` and `type[int]`
    x in y      # E: `in` is not supported between `type[int]` and `type[int]`
    x not in y  # E: `not in` is not supported between `type[int]` and `type[int]`
    "#,
);

testcase!(
    test_access_method_using_class_param_on_class,
    r#"
from typing import assert_type, Any
class A[T]:
    def f(self) -> T: ...
assert_type(A.f(A[int]()), int)
    "#,
);

testcase!(
    test_access_generic_method_using_class_param_on_class,
    r#"
from typing import assert_type, Any
class A[T]:
    def f[S](self, x: S) -> tuple[S, T]: ...
assert_type(A.f(A[int](), ""), tuple[str, int]) # E: assert_type(tuple[str, Any], tuple[str, int])
    "#,
);

testcase!(
    test_access_overloaded_method_using_class_param_on_class,
    r#"
from typing import assert_type, overload
class A[T]:
    @overload
    def f(self) -> T: ...
    @overload
    def f(self, x: T | None) -> T: ...
    def f(self, x=None): ...
assert_type(A.f(A[int]()), int)
    "#,
);

testcase!(
    test_invalid_augmented_assign_in_init,
    r#"
class C:
    def __init__(self):
        self.x += 5  # E: Object of class `C` has no attribute `x`
    "#,
);

testcase!(
    test_attributes_when_raw_class_field_type_contains_var,
    r#"
from typing import assert_type, Any
# This test is making sure we don't leak a `Var` into a ClassField, which can lead to nondeterminism.
class A:
    x = []
    y = []
assert_type(A().x, list[Any])
A().x = [42]
A().y = [42]
assert_type(A().y, list[Any])
    "#,
);

testcase!(
    test_read_only_frozen_dataclass,
    r#"
import dataclasses

@dataclasses.dataclass(frozen=True)
class FrozenData:
    x: int
    y: str

def f(d: FrozenData):
    d.x = 42  # E: Cannot set field `x`
    d.y = "new"  # E: Cannot set field `y`
    "#,
);

testcase!(
    test_read_only_namedtuple,
    r#"
from typing import NamedTuple

class Point(NamedTuple):
    x: int
    y: int

def f(p: Point):
    p.x = 10  # E: Cannot set field `x`
    p.y = 20  # E: Cannot set field `y`
    "#,
);

testcase!(
    test_read_only_annotation_typeddict,
    r#"
from typing_extensions import TypedDict, ReadOnly

class Config(TypedDict):
    name: ReadOnly[str]
    value: int

def f(c: Config):
    c["value"] = 42  # OK
    c["name"] = "new"  # E: Key `name` in TypedDict `Config` is read-only
    "#,
);

testcase!(
    bug = "We probably should disallow reassignment of class objects",
    test_nested_class_mutability,
    r#"
class Backend:
    class Options:
        pass
class Options2(Backend.Options):
    pass
Backend.Options = Options2  # E: A class object initialized in the class body is considered read-only
    "#,
);

testcase!(
    test_nested_class_inheritance,
    r#"
class Backend:
    class Options:
        pass
class ProcessGroupGloo(Backend):
    class Options(Backend.Options):
        pass
    "#,
);

testcase!(
    test_nested_class_inheritance_via_assignment,
    r#"
class Backend:
    class Options:
        pass
class Options2(Backend.Options):
    pass
class ProcessGroupGloo(Backend):
    Options = Options2
    "#,
);

testcase!(
    test_read_only_class_var,
    r#"
from typing import ClassVar, Final
class C:
    x: ClassVar[Final[int]] = 42
C.x = 43  # E: This field is marked as Final
    "#,
);

testcase!(
    bug = "other.output type is too general. Also, there should be no errors.",
    test_attr_cast,
    r#"
from typing import Self, cast, Any, assert_type

class C:
    outputs: list[Any]
    def f(self, other):
        other = cast(Self, other)
        assert_type(other, Self)
        assert_type(other.outputs, Any) # E: TODO: Expr::attr_infer_for_type
        len(self.outputs) == len(other.outputs) # E: TODO: Expr::attr_infer_for_type attribute base undefined for type: Self
    "#,
);

testcase!(
    bug = "There should be no errors here.",
    test_attr_tuple,
    r#"
from typing import Any, Tuple

def g(ann) -> None:
    if ann is Tuple: ...
    ann.__module__ # E: TODO: Expr::attr_infer_for_type attribute base undefined for type: type[Tuple] | Unknown (trying to access __module__)
    "#,
);

testcase!(
    bug = "PyTorch TODO: First error message can be improved and there should be  no error on obj.__name__",
    test_attr,
    r#"
def f(obj, g, field_type, my_type,):
    assert issubclass(obj, tuple) and hasattr(obj, "_fields")
    for f in obj._fields: # E: TODO: Expr::attr_infer_for_type attribute base undefined for type: type[tuple[Unknown, ...]]
        if isinstance(field_type, my_type) and g is not None:
            if g is None:
                raise ValueError(
                    f"{obj.__name__}." # E: TODO: Expr::attr_infer_for_type attribute base undefined for type: @_ (trying to access __name__)
                )
    "#,
);
