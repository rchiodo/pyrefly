/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getDiagnostics TSP request

use lsp_server::ResponseError;
use lsp_types::Diagnostic;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_diagnostics(
        &self,
        _transaction: &Transaction<'_>,
        _params: tsp::GetDiagnosticsParams,
    ) -> Result<Option<Vec<Diagnostic>>, ResponseError> {
        // TODO: Implement getDiagnostics
        // This should return diagnostics for the specified file
        // For now, return empty diagnostics
        Ok(Some(Vec::new()))
    }
}
