/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the createInstanceType TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn create_instance_type(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::CreateInstanceTypeParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        // TODO: Implement createInstanceType
        // This should generate an instance type representation for the provided type
        // For now, return None
        Ok(None)
    }
}
