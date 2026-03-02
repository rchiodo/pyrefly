/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getDeclaredType TSP request

use lsp_server::ResponseError;
use tsp_types::GetTypeParams;
use tsp_types::Type;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;
use crate::tsp::type_conversion::convert_type;

impl<T: TspInterface> TspServer<T> {
    /// Return the declared type at the given position.
    ///
    /// The declared type is the annotation explicitly written by the user.
    /// For example, `a: int | str` has declared type `int | str` even if
    /// type narrowing later restricts the computed type to `int`.
    ///
    /// Currently, this piggy-backs on `get_type_at_position` which returns
    /// the computed type. A future improvement can separate the annotation
    /// type from the inferred type in the binding infrastructure.
    pub fn handle_get_declared_type(&self, params: GetTypeParams) -> Result<Type, ResponseError> {
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
