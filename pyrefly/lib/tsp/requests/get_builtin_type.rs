/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getBuiltinType TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_builtin_type(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::GetBuiltinTypeParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        // TODO: Implement getBuiltinType
        // This should return the type of builtin types like int, str, list, etc.
        // For now, return None
        Ok(None)
    }
}
