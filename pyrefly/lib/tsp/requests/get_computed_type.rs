/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getComputedType TSP request

use lsp_server::ResponseError;
use tsp_types::GetTypeParams;
use tsp_types::Type;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;
use crate::tsp::type_conversion::convert_type;

impl<T: TspInterface> TspServer<T> {
    /// Return the computed (inferred) type at the given position.
    ///
    /// The computed type reflects the type checker's analysis of the code
    /// flow — e.g. after narrowing inside an `isinstance` guard the computed
    /// type of a variable may be more specific than its declared annotation.
    pub fn handle_get_computed_type(&self, params: GetTypeParams) -> Result<Type, ResponseError> {
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
