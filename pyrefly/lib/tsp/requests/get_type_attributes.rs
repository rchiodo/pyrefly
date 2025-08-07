/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getTypeAttributes TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_type_attributes(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::GetTypeAttributesParams,
    ) -> Result<Option<Vec<tsp::Attribute>>, ResponseError> {
        // TODO: Implement getTypeAttributes
        // This should return all attributes of a specific type (class members, function parameters, etc.)
        // For now, return empty attributes
        Ok(Some(Vec::new()))
    }
}
