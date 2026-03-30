/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::testcase;

testcase!(
    test_typeform_recognized,
    r#"
from typing_extensions import TypeForm

# TypeForm should be recognized as a valid special form.
# TypeForm[T] in annotation context should not produce "unknown" errors.
x: TypeForm[int]
y: TypeForm[str | None]
    "#,
);

testcase!(
    test_typeform_bad_specialization,
    r#"
from typing_extensions import TypeForm

x: TypeForm[int, str]  # E: `TypeForm` requires exactly one argument but got 2
    "#,
);
