/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    test_new_type_simple,
    r#"
from typing import NewType, assert_type

UserId = NewType("UserId", int)
UserId("user")  # E: Argument `Literal['user']` is not assignable to parameter `_x` with type `int` in function `UserId.__new__`
u1: UserId = 42 # E: `Literal[42]` is not assignable to `UserId`
u2: UserId = UserId(42)

assert_type(UserId(5) + 1, int)

isinstance(u2, UserId) # E: NewType `UserId` not allowed in isinstance

class UserIdDerived(UserId): # E: Subclassing a NewType not allowed
    pass
     "#,
);

testcase!(
    test_new_type_naming,
    r#"
from typing import NewType 

GoodName = NewType("BadName", int) # E: Expected string literal "GoodName"  

GoodNewType1 = NewType("GoodNewType1", list)  

GoodNewType2 = NewType("GoodNewType2", GoodNewType1) 

nt1: GoodNewType1[int] # E: Expected 0 type arguments for `GoodNewType1`, got 1

     "#,
);

testcase!(
    test_new_type_generic,
    r#"
from typing import NewType, TypeVar, Hashable, Literal

BadNewType1 = NewType("BadNewType1", int | str) # E: Second argument to NewType is invalid

T = TypeVar("T")
BadNewType2 = NewType("BadNewType2", list[T])  # E: Second argument to NewType cannot be an unbound generic

BadNewType3 = NewType("BadNewType3", Hashable) # E: Second argument to NewType cannot be a protocol

BadNewType4 = NewType("BadNewType4", Literal[7]) # E: Second argument to NewType is invalid
     "#,
);

testcase!(
    test_new_type_wrong_arity,
    r#"
from typing import NewType 
UserId = NewType("UserId", int, int) # E: Expected 2 positional arguments, got 3
UserId = NewType("UserId") # E: Missing argument `tp`
userId = NewType() # E: Missing argument `name` # E: Missing argument `tp`
     "#,
);

testcase!(
    test_new_type_not_allowed,
    r#"
from typing import NewType , TypedDict, Any

class TD1(TypedDict):
    a: int

BadNewType1 = NewType("BadNewType1", TD1)  # E: Second argument to NewType is invalid

BadNewType2 = NewType("BadNewType2", Any)  # E: Second argument to NewType is invalid
     "#,
);

testcase!(
    test_new_type_as_alias,
    r#"
from typing import NewType, TypeAlias

class R:
    ValueType = NewType("ValueType", int)
    V: TypeAlias = ValueType

    def test(self, v: V) -> int:
        return v
     "#,
);

testcase!(
    bug =
        "concrete tuple base classes become unbounded when converted to their class representation",
    test_new_type_tuple,
    r#"
from typing import NewType
Foo = NewType("Foo", tuple[int, int])
Bar = NewType("Bar", tuple[int, ...])

Foo((1, 2))  # OK
Foo((1, 2, 3))  # this shouldn't be allowed
     "#,
);

testcase!(
    test_new_type_is_not_type,
    r#"
from typing import Any, NewType
Foo = NewType("Foo", int)
x: type = Foo  # E: `type[Foo]` is not assignable to `type`
y: type[Any] = Foo  # E: `type[Foo]` is not assignable to `type[Any]`
    "#,
);
