/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    bug = "Generic functions don't work with ParamSpec",
    test_param_spec_generic_function,
    r#"
from typing import Callable, reveal_type
def identity[**P, R](x: Callable[P, R]) -> Callable[P, R]:
    return x
def foo[T](x: T, y: T) -> T:
    return x
foo2 = identity(foo)
reveal_type(foo2)  # E: revealed type: (x: Unknown, y: Unknown) -> Unknown
"#,
);

testcase!(
    bug = "Generic class constructors don't work with ParamSpec",
    test_param_spec_generic_constructor,
    r#"
from typing import Callable, reveal_type
def identity[**P, R](x: Callable[P, R]) -> Callable[P, R]:
  return x
class C[T]:
  x: T
  def __init__(self, x: T) -> None:
    self.x = x
c2 = identity(C)
reveal_type(c2)  # E: revealed type: (x: Unknown) -> C[Unknown]
x: C[int] = c2(1)
"#,
);
