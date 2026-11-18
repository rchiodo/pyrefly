/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::test::util::TestEnv;
use crate::testcase;

fn deprecated_library() -> TestEnv {
    TestEnv::one(
        "deprecated_library",
        r#"
from typing import Self, overload
from typing_extensions import deprecated

@deprecated("Use Spam instead")
class Ham: ...

@deprecated("It is pining for the fjords")
def norwegian_blue(x: int) -> int: ...

@overload
@deprecated("Only str will be allowed")
def foo(x: int) -> str: ...
@overload
def foo(x: str) -> str: ...
def foo(x: int | str) -> str: ...

class Spam:
    @deprecated("There is enough spam in the world")
    def __add__(self, other: object) -> Self: ...

    @property
    @deprecated("All spam will be equally greasy")
    def greasy(self) -> float: ...

    @property
    def shape(self) -> str: ...

    @shape.setter
    @deprecated("Shapes are becoming immutable")
    def shape(self, value: str) -> None: ...
"#,
    )
}

testcase!(
    test_deprecated_normative_cross_module_uses,
    deprecated_library(),
    r#"
from deprecated_library import Ham  # E: `Ham` is deprecated
import deprecated_library as library

library.norwegian_blue(1)  # E: `deprecated_library.norwegian_blue` is deprecated
map(library.norwegian_blue, [1, 2, 3])  # E: `deprecated_library.norwegian_blue` is deprecated

library.foo(1)  # E: Call to deprecated overload `deprecated_library.foo`
library.foo("x")

ham = Ham()
spam = library.Spam()

spam + 1  # E: `deprecated_library.Spam.__add__` is deprecated
spam += 1  # E: `deprecated_library.Spam.__add__` is deprecated

spam.greasy  # E: `deprecated_library.Spam.greasy` is deprecated
spam.shape
spam.shape = "cube"  # E: `deprecated_library.Spam.shape` is deprecated
spam.shape += "cube"  # E: `deprecated_library.Spam.shape` is deprecated
"#,
);

testcase!(
    test_deprecated_normative_local_and_protocol_uses,
    r#"
from typing import Protocol
from typing_extensions import deprecated

class Invocable:
    @deprecated("Deprecated")
    def __call__(self) -> None: ...

invocable = Invocable()
invocable()  # E: `Invocable.__call__` is deprecated

@deprecated("Deprecated")
def lorem() -> None: ...

lorem()  # E: `lorem` is deprecated

class SupportsFoo(Protocol):
    @deprecated("Deprecated")
    def foo(self) -> None: ...

def foo_it(f: SupportsFoo) -> None:
    f.foo()  # E: `SupportsFoo.foo` is deprecated
"#,
);
