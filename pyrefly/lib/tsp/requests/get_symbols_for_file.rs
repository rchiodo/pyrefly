/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getSymbolsForFile TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_symbols_for_file(
        &self,
        _transaction: &Transaction<'_>,
        params: tsp::GetSymbolsForFileParams,
    ) -> Result<Option<tsp::FileSymbolInfo>, ResponseError> {
        // TODO: Implement getSymbolsForFile
        // This should return all symbols in a specific file
        // For now, return empty file symbol info
        Ok(Some(tsp::FileSymbolInfo {
            uri: params.uri,
            symbols: Vec::new(),
        }))
    }
}
