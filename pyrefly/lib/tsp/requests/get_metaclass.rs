/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getMetaclass TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_metaclass(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::GetMetaclassParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        // TODO: Implement getMetaclass
        // This should return the metaclass of a type
        // For now, return None
        Ok(None)
    }
}
