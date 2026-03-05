# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

# pyre-ignore-all-errors

"""Runtime implementation of shape typing constructs.

The .pyi stub provides full type information to pyrefly. This .py file
provides minimal runtime classes so that annotations using these types
don't crash when evaluated by Python.
"""


class Dim[T]:
    """Symbolic integer type for dimension values.

    At runtime this is a no-op generic class. The type checker uses the
    .pyi stub for shape inference.
    """

    pass
