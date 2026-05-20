/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::test::util::TestEnv;
use crate::testcase;

fn env() -> TestEnv {
    let mut e = TestEnv::new();
    e.add(
        "torch",
        r#"
class Tensor:
    def item(self) -> int | float: ...
    def sum(self) -> "Tensor": ...
"#,
    );
    e
}

fn env_with_lint() -> TestEnv {
    env().enable_pytorch_efficiency_lint_error()
}

testcase!(
    test_tensor_item_call,
    env_with_lint(),
    r#"
import torch

def f(x: torch.Tensor) -> None:
    v = x.item()  # E: `Tensor.item()` causes implicit GPU-to-CPU synchronization
"#,
);

testcase!(
    test_tensor_item_call_disabled_by_default,
    env(),
    r#"
import torch

def f(x: torch.Tensor) -> None:
    v = x.item()
"#,
);

testcase!(
    test_non_tensor_item_call_ok,
    env_with_lint(),
    r#"
class Foo:
    def item(self) -> int:
        return 0

def f(x: Foo) -> None:
    v = x.item()
"#,
);

testcase!(
    test_tensor_item_with_args_ok,
    env_with_lint(),
    r#"
import torch

def f(x: torch.Tensor) -> None:
    v = x.item(0)  # E: Expected 0 positional argument
"#,
);

testcase!(
    test_tensor_other_method_ok,
    env_with_lint(),
    r#"
import torch

def f(x: torch.Tensor) -> None:
    v = x.sum()
"#,
);
