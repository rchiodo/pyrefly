/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    test_typed_dict,
    r#"
from typing import TypedDict, Mapping
class Coord(TypedDict):
    x: int
    y: int
def foo(c: Coord) -> Mapping[str, object]:
    return c
def bar(c: Coord) -> Mapping[str, int]:
    return c  # E: is not assignable
def baz(c: Coord) -> Mapping[str, str]:
    return c  # E: is not assignable
"#,
);

testcase!(
    test_typed_dict_kwargs_type,
    r#"
from typing import assert_type, TypedDict, Unpack
class Coord(TypedDict):
    x: int
    y: int
def test1(**kwargs: Coord):
    assert_type(kwargs, dict[str, Coord])
def test2(**kwargs: Unpack[Coord]):
    assert_type(kwargs, Coord)
    "#,
);

testcase!(
    test_qualifiers,
    r#"
from typing import TypedDict, Required, NotRequired, ReadOnly, ClassVar, Final
class MyDict(TypedDict):
    v: ClassVar[int]  # E: `ClassVar` may not be used for TypedDict or NamedTuple members
    w: Final[int]  # E: `Final` may not be used for TypedDict or NamedTuple members
    x: NotRequired  # E: Expected a type argument for `NotRequired`
    y: Required  # E: Expected a type argument for `Required`
    z: ReadOnly  # E: Expected a type argument for `ReadOnly`
    "#,
);

testcase!(
    test_typed_dict_invalid_inheritance,
    r#"
from typing import TypedDict
class Coord(TypedDict, object):  # E: Typed dictionary definitions may only extend other typed dictionaries
    x: int
    y: int
    "#,
);

testcase!(
    test_typed_dict_literal,
    r#"
from typing import TypedDict, NotRequired
class Coord(TypedDict):
    x: int
    y: int
    z: NotRequired[int]

c1: Coord = {"x": 1, "y": 2}
c2: Coord = {"x": 1, "y": 2, "z": 3}
c3: Coord = {"x": 1, "y": 2, "a": 4}  # E: Key `a` is not defined in TypedDict `Coord`
c4: Coord = {"x": 1, "y": "foo"}  # E: `Literal['foo']` is not assignable to TypedDict key `y` with type `int`
c5: Coord = {"x": 1}  # E: Missing required key `y` for TypedDict `Coord`
c6: Coord = {"x": 1, **{"y": 2, **{"z": 3}}}
d: dict[str, int] = {}
c7: Coord = {"x": 1, **d}  # E: Unpacked `dict[str, int]` is not assignable to `TypedDict[Coord]`

def foo(c: Coord) -> None:
    pass
foo({"x": 1, "y": 2})
    "#,
);

testcase!(
    test_typed_dict_callable,
    r#"
from typing import TypedDict

class Movie(TypedDict):
    name: str
    year: int
m = Movie(name='Blade Runner', year=1982)
    "#,
);

testcase!(
    test_typed_dict_read_only,
    r#"
from typing import TypedDict, ReadOnly
class Coord(TypedDict):
    x: int
    y: ReadOnly[int]
def foo(c: Coord) -> None:
    c["x"] = 1
    c["x"] = "foo"  # E: `Literal['foo']` is not assignable to TypedDict key `x` with type `int`
    c["y"] = 3  # E: Key `y` in TypedDict `Coord` is read-only
    c["z"] = 4  # E: TypedDict `Coord` does not have key `z`
    "#,
);

testcase!(
    test_setitem_union,
    r#"
from typing import Any, TypedDict
class C(TypedDict):
    x: int
def f(c: C | Any):
    c["x"] = 0
    "#,
);

testcase!(
    test_typed_dict_readonly_partial_update,
    r#"
from typing import Never, NotRequired, TypedDict, ReadOnly
from typing_extensions import ReadOnly

class A(TypedDict):
    x: ReadOnly[int]
    y: int

a1: A = {"x": 1, "y": 2}
a2: A = {"x": 3, "y": 4}
a1.update(a2) # E: No matching overload 

class B(TypedDict):
    x: NotRequired[Never]
    y: ReadOnly[int]

def update_a(a: A, b: B) -> None:
    a.update(b)
    "#,
);

testcase!(
    test_update_with_readonly_key,
    r#"
from typing import ReadOnly, TypedDict
class A(TypedDict):
    x: ReadOnly[int]
a: A = {'x': 1}
a.update({'x': 2})  # E: Cannot update read-only field `x`
    "#,
);

testcase!(
    test_typed_dict_readonly_kwargs_tuple_update,
    r#"
from typing import TypedDict, ReadOnly

class A(TypedDict):
    x: ReadOnly[int]
    y: int

def test(a: A) -> None:
    a.update([("x", 123), ("y", 456)])  # E: No matching overload found for function `A.update`
    a.update([("y", 456)]) 
    a.update(x=789, y=999)  # E: No matching overload found for function `A.update`
    a.update(y=999) 
    "#,
);

testcase!(
    test_typed_dict_contextual,
    r#"
from typing import TypedDict
class MyDict(TypedDict, total=False):
    data: list[str | int]
def test():
    s: MyDict = {}
    s['data'] = []
    s['data'] = [42]
    s['data'] = [42, 'hello']
    s['data'] = ['hello']
    "#,
);

testcase!(
    test_typed_dict_metaclass,
    r#"
from enum import EnumMeta
from typing import TypedDict
class Coord(TypedDict, metaclass=EnumMeta):  # E: Typed dictionary definitions may not specify a metaclass
    x: int
    y: int
    "#,
);

testcase!(
    test_typed_dict_iterate,
    r#"
from typing import TypedDict, assert_type
class Coord(TypedDict):
    x: int
    y: int
def foo(c: Coord) -> None:
    for x in Coord:  # E: Type `type[Coord]` is not iterable
        pass
    for x in c:
        assert_type(x, str)
    "#,
);

testcase!(
    test_typed_dict_generic,
    r#"
from typing import TypedDict
class Coord[T](TypedDict):
    x: T
    y: T
def foo(c: Coord[int]):
    x: int = c["x"]
    y: str = c["y"]  # E: `int` is not assignable to `str`
    "#,
);

testcase!(
    test_typed_dict_initialized_field,
    r#"
from typing import TypedDict
class Coord(TypedDict):
    x: int
    y: int = 2  # E: TypedDict item `y` may not be initialized
    "#,
);

testcase!(
    test_typed_dict_access,
    r#"
from typing import TypedDict, Literal, assert_type
class Coord(TypedDict):
    x: int
    y: str
    z: bool
def foo(c: Coord, key: str, key2: Literal["x", "y"]):
    x: int = c["x"]
    x2: int = c.x  # E: Object of class `Coord` has no attribute `x`
    x3: int = c[key]  # E: Invalid key for TypedDict `Coord`, got `str`
    x4: int = c["aaaaaa"]  # E: TypedDict `Coord` does not have key `aaaaaa`
    assert_type(c[key2], int | str)
    "#,
);

testcase!(
    test_typed_dict_delete,
    r#"
from typing import TypedDict, ReadOnly, Required
class Coord(TypedDict, total=False):
    x: int
    y: ReadOnly[str]
    z: Required[bool]
def foo(c: Coord):
    del c["x"]  # OK
    del c["y"]  # E: Key `y` in TypedDict `Coord` may not be deleted
    del c["z"]  # E: Key `z` in TypedDict `Coord` may not be deleted
    "#,
);

testcase!(
    test_typed_dict_functional,
    r#"
from typing import TypedDict, Required, NotRequired
Coord = TypedDict("Coord", { "x": Required[int], " illegal ": int, "y": NotRequired[int] })
c: Coord = {"x": 1, " illegal ": 2}
def test(c: Coord):
    x: int = c[" illegal "]
Invalid = TypedDict()  # E: Expected a callable, got type[TypedDict]
    "#,
);

testcase!(
    test_typed_dict_pop,
    r#"
from typing import TypedDict, NotRequired

class TD(TypedDict):
    x: NotRequired[int]
    
def f(td: TD):
    td.pop("x") 
    "#,
);

testcase!(
    test_typed_dict_pop_2,
    r#"
from typing import TypedDict, NotRequired, assert_type, Any

class TDRequired(TypedDict):
    a: int
    b: str

class TDOptional(TypedDict):
    x: NotRequired[int]
    y: NotRequired[str]

class TDMixed(TypedDict):
    a: int
    x: NotRequired[int]

td_r: TDRequired = {"a": 10, "b": "hi"}
td_o: TDOptional = {"x": 42}
td_m: TDMixed = {"a": 1, "x": 99}

v1 = td_r.pop("a") # E: 
assert_type(v1, object)

v2 = td_r.pop("a", 3.14) # E: 
assert_type(v2, object)

v3 = td_o.pop("x")
assert_type(v3, int | None)

v4 = td_o.pop("x", -1)
assert_type(v4, int)

v5 = td_o.pop("x", "fallback")
assert_type(v5, int | str)

v6 = td_m.pop("a") # E: 
assert_type(v6, Any)

v7 = td_m.pop("x")
assert_type(v7, int | None)

v8 = td_m.pop("x", 0)
assert_type(v8, int)

v9 = td_m.pop("x", "fallback")
assert_type(v9, int | str)

v10 = td_r.pop("abc", 123) # E:
assert_type(v10, object)

v11 = td_r.pop("abc", "default") # E: 
assert_type(v11, object)
    "#,
);

testcase!(
    test_typed_dict_del,
    r#"
from typing import TypedDict, NotRequired

class TDRequired(TypedDict):
    a: int
    b: str

class TDOptional(TypedDict):
    x: NotRequired[int]
    y: NotRequired[str]

class TDMixed(TypedDict):
    a: int
    x: NotRequired[int]

td_r: TDRequired = {"a": 10, "b": "hi"}
td_o: TDOptional = {"x": 42}
td_m: TDMixed = {"a": 1, "x": 99}

del td_r["a"]  # E: 
del td_o["x"]  # OK
del td_o["y"]  # OK
del td_m["x"]  # OK
del td_m["a"]  # E: 
del td_r["nonexistent"]  # E: 

# Delete optional key from TDOptional again 
del td_o["x"]  # OK

del td_r["b"]  # E:

del td_o["unknown"]  # E:

key_var = "x"
del td_o[key_var]  

unknown_key = "a"
del td_m[unknown_key]  # E: 

td_r: TDRequired = {"a": 10, "b": "hi"}
td_o: TDOptional = {"x": 42}
td_m: TDMixed = {"a": 1, "x": 99}

td_r.__delitem__("a") # E: Argument `Literal['a']` is not assignable to parameter `k` with type `Never`
td_o.__delitem__("x")
td_o.__delitem__("y")
td_m.__delitem__("x")
td_m.__delitem__("a") # E: No matching overload found for function `TDMixed.__delitem__`

td_r.__delitem__("nonexistent") # E: Argument `Literal['nonexistent']` is not assignable to parameter `k` with type `Never`
td_r.__delitem__("unknown") # E:  Argument `Literal['unknown']` is not assignable to parameter `k` with type `Never
 
    "#,
);

testcase!(
    test_typed_dict_dunder_or,
    r#"
from typing import TypedDict, assert_type

class TD1(TypedDict):
    a: int 
    b: str 

class TD2(TypedDict):
    c: float 
    d: bool 

class TD3(TypedDict, total=False):
    a: str 
    f: int 

class TD4(TypedDict):
    a: int 

class TD5(TypedDict, total=False):
    a: int  

td1 = TD1(a=1, b="x")
td2 = TD2(c=3.14, d=True)
td3 = TD3(a="str", f=5)
td4 = TD4(a=42)
td5 = TD5(a=100)

td12 = td1 | td2 # E: `|` is not supported between `TypedDict[TD1]` and `TypedDict[TD2]`
td14 = td1 | td4
assert_type(td14.get("a"), int)
td13 = td1 | td3 # E:   No matching overload found for function `_typeshed._type_checker_internals.TypedDictFallback.__or__`
td15 = td1 | td5 # E: No matching overload found for function `_typeshed._type_checker_internals.TypedDictFallback.__or__`
    "#,
);

testcase!(
    test_typed_dict_dunder_ior,
    r#"
from typing import TypedDict, assert_type

class TD(TypedDict):
    a: int | str
    b: int | str

x: TD = {"a": 1, "b": 2}

y: TD = {"a": "3", "b": "4"}

x |= y  

assert_type((x["a"]), int | str)

x |= {} # E: Augmented assignment produces a value of type `dict[str, object]`, which is not assignable to `TypedDict[TD]`

x: TD = {"a": 1, "b": 2}
x.__ior__(y)
x.__ior__({}) # E:  Missing required key `a` for TypedDict `TD` # E: Missing required key `b` for TypedDict `TD` 

    "#,
);

testcase!(
    test_typed_dict_dunder_ror,
    r#"
from typing import TypedDict, assert_type

class TD(TypedDict):
    a: int | str
    b: int | str

x: TD = {"a": 1, "b": 2}

result = {"a": "1", "b": "2", "c":4} | x
assert_type(result.get("c"), object | None)

result = x.__ror__({"a": "1", "b": "2", "c":4})
assert_type(result.get("c"), object | None)
    "#,
);

testcase!(
    test_typed_dict_subtype,
    r#"
from typing import TypedDict
class Coord(TypedDict):
    x: int
    y: int
class Coord3D(TypedDict):
    x: int
    y: int
    z: int
class Pair(TypedDict):
    x: object
    y: object

def foo(a: Coord, b: Coord3D, c: Pair):
    coord: Coord = b
    coord2: Coord3D = a  # E: `TypedDict[Coord]` is not assignable to `TypedDict[Coord3D]`
    coord3: Coord = c  # E: `TypedDict[Pair]` is not assignable to `TypedDict[Coord]`
    coord4: Pair = a  # E: `TypedDict[Coord]` is not assignable to `TypedDict[Pair]`
    "#,
);

testcase!(
    test_typed_dict_read_only_subtype,
    r#"
from typing import ReadOnly, TypedDict

class TD(TypedDict):
    a: ReadOnly[int]

class TD2(TypedDict):
    a: ReadOnly[object]

def foo(td: TD, td2: TD2) -> None:
    td: TD = td2  # E: `TypedDict[TD2]` is not assignable to `TypedDict[TD]`
    td2: TD2 = td
    "#,
);

testcase!(
    test_typed_dict_not_required,
    r#"
from typing import TypedDict, NotRequired
class Coord(TypedDict):
    x: int
    y: int
class CoordNotRequired(TypedDict):
    x: NotRequired[int]
    y: NotRequired[int]

def foo(a: Coord, b: CoordNotRequired):
    coord: Coord = b  # E: `TypedDict[CoordNotRequired]` is not assignable to `TypedDict[Coord]`
    coord2: CoordNotRequired = a  # E: `TypedDict[Coord]` is not assignable to `TypedDict[CoordNotRequired]`
    "#,
);

testcase!(
    test_typed_dict_totality,
    r#"
from typing import TypedDict, NotRequired

class CoordXY(TypedDict, total=True):
    x: int
    y: int
class CoordZ(TypedDict, total=False):
    z: int

class Coord(CoordZ):
    x: int
    y: int
class Coord2(CoordXY, total=False):
    z: int
class Coord3(TypedDict):
    x: int
    y: int
    z: NotRequired[int]
class Coord4(TypedDict, CoordXY, CoordZ):
    pass

def foo(a: Coord, b: Coord2, c: Coord3, d: Coord4):
    coord: Coord = b
    coord = c
    coord = d
    coord2: Coord2 = a
    coord2 = c
    coord2 = d
    coord3: Coord3 = a
    coord3 = b
    coord3 = d
    coord4: Coord4 = a
    coord4 = b
    coord4 = c
    "#,
);

testcase!(
    test_typed_dict_kwargs_expansion,
    r#"
from typing import TypedDict, NotRequired
class Coord(TypedDict):
    x: int
    y: int
    z: NotRequired[int]

def f1(x: int, y: int, z: int = 1): ...
def f2(x: int, y: int, **kwargs: str): ...
def f3(x: int, y: int): ...
def f4(x: int, y: int = 2, z: int = 3): ...
def f5(x: int, y: int, z: int): ...
def f6(x: int, y: int, **kwargs: str): ...

x: Coord = {"x": 1, "y": 2}
f1(**x)
f2(**x)  # E: Argument `int` is not assignable to parameter `z` with type `str` in function `f2`
f3(**x)  # E: Unexpected keyword argument `z`
f4(**x)
f5(**x)  # E: Expected key `z` to be required
f6(**x)  # E: Argument `int` is not assignable to parameter `z` with type `str` in function `f6`
f1(1, **x)  # E: Multiple values for argument `x`
    "#,
);

testcase!(
    test_typed_dict_kwargs_unpack,
    r#"
from typing import TypedDict, NotRequired, Unpack, assert_type
class Coord(TypedDict):
    x: int
    y: int
    z: NotRequired[int]

class Coord2(TypedDict):
    x: int
    y: int

class Coord3(TypedDict):
    x: int
    y: int
    z: int

class Coord4(TypedDict):
    w: int
    x: int
    y: int
    z: NotRequired[int]

class Coord5(TypedDict):
    y: int
    z: NotRequired[int]

def f(**kwargs: Unpack[Coord]):
    assert_type(kwargs["x"], int)

def g(x: Coord, x2: Coord2, x3: Coord3, x4: Coord4, x5: Coord5):
    f(**x)
    f(**x2)
    f(**x3)
    f(**x4)
    f(**x5)  # E: Missing argument `x`
f(x=1, y=2)
f(x=1, y=2, z=3)
f(x=1, y=2, z=3, a=4)  # E: Unexpected keyword argument `a`
f(x="", y=2)  # E: Argument `Literal['']` is not assignable to parameter `x` with type `int` in function `f`
    "#,
);

testcase!(
    test_typed_dict_read_only_variance,
    r#"
from typing import ReadOnly, TypedDict

class TD(TypedDict):
    a: int

class TD2(TypedDict):
    a: ReadOnly[int]

class TD3(TypedDict):
    a: bool

def foo(td2: TD2, td3: TD3) -> None:
    t1: TD2 = td3  # OK
    t2: TD = td2  # E: `TypedDict[TD2]` is not assignable to `TypedDict[TD]`
    t3: TD = td3  # E: `TypedDict[TD3]` is not assignable to `TypedDict[TD]`
    "#,
);

testcase!(
    test_inheritance,
    r#"
from typing import TypedDict
class A(TypedDict):
    x: int
class B(A):
    y: str
B(x=0, y='1')  # OK
B(x=0, y=1)  # E: Argument `Literal[1]` is not assignable to parameter `y` with type `str` in function `B.__init__`
    "#,
);

testcase!(
    test_generic_instantiation,
    r#"
from typing import TypedDict, assert_type
class C[T](TypedDict):
     x: T
assert_type(C(x=0), C[int])
assert_type(C[str](x=""), C[str])
    "#,
);

testcase!(
    test_unpacked_typed_dict_assert_type,
    r#"
from typing import TypedDict, Unpack, assert_type
class Coord(TypedDict):
    x: int
    y: int
def foo(x: Coord, **kwargs: Unpack[Coord]):
    assert_type(x, Coord)
    assert_type(kwargs, Coord)
    "#,
);

testcase!(
    test_requireness_in_init,
    r#"
from typing import TypedDict, NotRequired
class D(TypedDict):
     x: int
     y: int = 5  # E: TypedDict item `y` may not be initialized
     z: NotRequired[int]
# Default values are completely ignored in constructor behavior, so requiredness in `__init__` should be
# determined entirely by whether the field is required in the resulting dict.
D(x=5)  # E: Missing argument `y`
    "#,
);

testcase!(
    test_cyclic_typed_dicts,
    r#"
from typing import TypedDict, assert_type
class TD0(TypedDict):
    x: int
    y: TD1
class TD1(TypedDict):
    x: int
    y: TD0
def foo(td0: TD0, td1: TD1) -> None:
    assert_type(td0, TD0)
    assert_type(td0['x'], int)
    assert_type(td0['y'], TD1)
    assert_type(td1, TD1)
    assert_type(td1['x'], int)
    assert_type(td1['y'], TD0)
    "#,
);

testcase!(
    test_typing_typeddict_functional,
    r#"
import typing
from typing import assert_type
X = typing.TypedDict('X', {'x': int})
assert_type(X, type[X])
    "#,
);

testcase!(
    test_assign_get_result,
    r#"
from typing import TypedDict
UserType1 = TypedDict("UserType1", {"name": str, "age": int}, total=False)
user1: UserType1 = {"name": "Bob", "age": 40}
name: str = user1.get("name", "n/a")
    "#,
);

testcase!(
    test_get,
    r#"
from typing import Any, Literal, TypedDict, assert_type
class C(TypedDict):
    x: int
    y: str
def f(c: C, k1: str, k2: int):
    assert_type(c.get("x"), int)
    assert_type(c.get(k1), object)
    assert_type(c.get(k1, 0), object)
    c.get(k2)  # E: No matching overload
    "#,
);

testcase!(
    test_get_not_required,
    r#"
from typing import assert_type, NotRequired, TypedDict
class C(TypedDict):
    x: NotRequired[int]
def f(c: C):
    assert_type(c.get("x"), int | None)
    "#,
);

// Clearing a TypedDict is not allowed, since doing so would remove keys it's expected to have.
testcase!(
    test_clear,
    r#"
from typing import TypedDict
class C(TypedDict):
    x: int
def f(c: C):
    c.clear()  # E: no attribute `clear`
    "#,
);

testcase!(
    test_update,
    r#"
from typing import TypedDict
class C(TypedDict):
    x: int
    y: int
class D(TypedDict):
    x: int
    y: int
class E(TypedDict):
    x: int
    y: str
class F(TypedDict):
    x: int
def f(c1: C, c2: C, c3: dict[str, int], d: D, e: E, f: F):
    c1.update(c2) 
    c1.update(c3)  # E: No matching overload found for function `C.update`
    c1.update(d) 
    c1.update(e)  # E: No matching overload found for function `C.update`
    # This is not ok because `F` could contain `y` with an incompatible type
    c1.update(f) # E: No matching overload found for function `C.update`
    c1.update({"x": 1, "y": 1})
    c1.update({"x": 1})  
    c1.update({"z": 1})  # E: Key `z` is not defined in TypedDict `C`
    c1.update([("x", 1), ("y", 2)])
    c1.update([("z", 3)]) # E: No matching overload found for function `C.update`
    c1.update(x=1, y=2)
    c1.update(z=1) # E: No matching overload found for function `C.update`
    "#,
);

testcase!(
    test_update_with_type_var,
    r#"
from typing import TypedDict

class X[T](TypedDict):
    a: T
    
def f(x: X[int], y: dict[str, int]):
    x.update(y)  # E: No matching overload found for function `X.update`
    x.update(x) 
    x.update({"a": 1})
    x.update({"b": 1}) # E: Key `b` is not defined in TypedDict `X`
    "#,
);

// Test an attribute that should be available on all TypedDict subclasses
testcase!(
    test_common_class_attr,
    r#"
from typing import TypedDict, assert_type
class C(TypedDict): ...
assert_type(C.__total__, bool)
    "#,
);

testcase!(
    bug = "Instances of TypedDicts are plain dicts at runtime and should not have a `__total__` attribute",
    test_class_attr_on_instance,
    r#"
from typing import TypedDict
class C(TypedDict): ...
def f(c: C):
    c.__total__  # This should be an error
    "#,
);

testcase!(
    test_setdefault,
    r#"
from typing import TypedDict, assert_type
class C(TypedDict):
    x: int
def f(c: C, s: str):
    assert_type(c.setdefault("x", 0), int)
    c.setdefault("x", 0.0)  # E: No matching overload
    c.setdefault("x")  # E: No matching overload
    c.setdefault(s, 0)  # E: No matching overload
    "#,
);

testcase!(
    test_setdefault_read_only,
    r#"
from typing import ReadOnly, TypedDict
class C(TypedDict):
    x: ReadOnly[int]
class D(TypedDict):
    x: int
    y: ReadOnly[str]
def f(c: C, d: D):
    c.setdefault("x", 0)  # E: `Literal['x']` is not assignable to parameter `k` with type `Never`
    d.setdefault("x", 0)
    d.setdefault("y", "oops")  # E: No matching overload
    "#,
);

testcase!(
    test_posonly_params,
    r#"
from typing import TypedDict
class C(TypedDict):
    x: int
def f(c: C):
    # Both of these methods take only positional arguments.
    c.get(key="x")  # E: No matching overload
    c.setdefault("x", default=0)  # E: No matching overload
    "#,
);

testcase!(
    test_kwonly_params_init,
    r#"
from typing import TypedDict
class C(TypedDict):
    x: int
C(0)  # E: Expected argument `x` to be passed by name in function `C.__init__`
    "#,
);

testcase!(
    test_attribute_named_self,
    r#"
from typing import TypedDict
class C(TypedDict):
    self: int
C(self=0)
    "#,
);

testcase!(
    test_required_and_notrequired_conflict,
    r#"
from typing import TypedDict, Required, NotRequired

class TD(TypedDict):
    x: Required[NotRequired[int]]  # E: Cannot combine `Required` and `NotRequired` for a TypedDict field
    y: NotRequired[Required[int]]  # E: Cannot combine `Required` and `NotRequired` for a TypedDict field
    "#,
);

testcase!(
    test_typed_dict_isinstance_issubclass_not_allowed,
    r#"
from typing import TypedDict

class D(TypedDict):
    x: int

x = int
isinstance(x, D)  # E: TypedDict `D` not allowed as second argument to isinstance()
issubclass(x, D)  # E: TypedDict `D` not allowed as second argument to issubclass()
"#,
);

testcase!(
    test_typeddict_valid_base_class,
    r#"
from typing import TypedDict

class Person(TypedDict):
    name: str
    age: int
    "#,
);

testcase!(
    test_typeddict_invalid_annotations,
    r#"
from typing import TypedDict, TypeVar

T = TypeVar("T", bound=TypedDict)  # E: `TypedDict` is not allowed in this context

def test(x: TypedDict):  # E: `TypedDict` is not allowed in this context
    pass

x: TypedDict  # E: `TypedDict` is not allowed in this context
    "#,
);

testcase!(
    test_invalid_typed_dict_keywords,
    r#"
from typing import TypedDict

class TD1(TypedDict, total=True):  # OK
    x: int

class TD2(TypedDict, foo=1):  # E: TypedDict does not support keyword argument `foo`
    x: int

class TD3(TypedDict, bar="test", baz=False):  # E: TypedDict does not support keyword argument `bar`  # E: TypedDict does not support keyword argument `baz`
    x: int
"#,
);

testcase!(
    test_nonrequired_readonly,
    r#"
from typing import NotRequired, ReadOnly, Required, TypedDict
class A(TypedDict):
    x: NotRequired[ReadOnly[str]]
class B(TypedDict):
    x: Required[str]
b: B = {'x': ''}
a: A = b
    "#,
);
