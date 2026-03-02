/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getExpectedType TSP request

use lsp_server::ResponseError;
use tsp_types::GetTypeParams;
use tsp_types::Type;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;
use crate::tsp::type_conversion::convert_type;

impl<T: TspInterface> TspServer<T> {
    /// Return the expected type at the given position.
    ///
    /// The expected type is the type that a surrounding context demands.
    /// For example, in `foo(4)` where `def foo(a: int | str)`, the expected
    /// type of the argument `4` is `int | str`.
    ///
    /// Currently, this piggy-backs on `get_type_at_position` which returns
    /// the computed type. A future improvement can query the expected type
    /// from the function parameter annotations or assignment target types.
    pub fn handle_get_expected_type(&self, params: GetTypeParams) -> Result<Type, ResponseError> {
        self.validate_snapshot(params.snapshot)?;
        let ty = self
            .inner
            .get_type_at_position(&params.uri, params.position.line, params.position.character)
            .ok_or_else(|| lsp_server::ResponseError {
                code: lsp_server::ErrorCode::InvalidParams as i32,
                message: "No type found at position".to_owned(),
                data: None,
            })?;
        Ok(convert_type(&ty))
    }
}
