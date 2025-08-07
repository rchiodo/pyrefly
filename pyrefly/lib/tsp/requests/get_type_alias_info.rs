/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getTypeAliasInfo TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_type_alias_info(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::GetTypeAliasInfoParams,
    ) -> Result<Option<tsp::TypeAliasInfo>, ResponseError> {
        // TODO: Implement getTypeAliasInfo
        // This should return information about type aliases (name and type arguments)
        // For now, return None
        Ok(None)
    }
}
