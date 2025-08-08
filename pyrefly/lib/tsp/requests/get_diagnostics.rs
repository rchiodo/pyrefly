/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getDiagnostics TSP request

use std::iter::once;

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use lsp_types::Diagnostic;
use pyrefly_python::module_name::ModuleName;

use crate::lsp::module_helpers::make_open_handle;
use crate::lsp::module_helpers::to_real_path;
use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn get_diagnostics(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetDiagnosticsParams,
    ) -> Result<Option<Vec<Diagnostic>>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        lsp_debug!("Getting diagnostics for URI: {}", params.uri);

        // Convert URI to file path
        let file_path = match params.uri.to_file_path() {
            Ok(path) => path,
            Err(_) => {
                return Err(ResponseError {
                    code: ErrorCode::InvalidParams as i32,
                    message: "Invalid URI - cannot convert to file path".to_owned(),
                    data: None,
                });
            }
        };

        // Check if workspace has language services enabled
        let Some(_handle) = self.make_handle_if_enabled(&params.uri) else {
            lsp_debug!("Language services disabled for workspace");
            return Ok(Some(Vec::new()));
        };

        // Create handle for the file
        let handle = make_open_handle(&self.state, &file_path);

        // Collect errors for this file
        let mut diagnostics = Vec::new();
        let open_files = self.open_files.read();

        for error in transaction.get_errors(once(&handle)).collect_errors().shown {
            // Apply the same filtering logic as get_diag_if_shown
            if let Some(path) = to_real_path(error.path()) {
                // When no file covers this, we'll get the default configured config which includes "everything"
                // and excludes `.<file>`s.
                let config = self
                    .state
                    .config_finder()
                    .python_file(ModuleName::unknown(), error.path());
                if open_files.contains_key(&path)
                    && !config.project_excludes.covers(&path)
                    && !self
                        .workspaces
                        .get_with(path.to_path_buf(), |w| w.disable_type_errors)
                {
                    diagnostics.push(error.to_diagnostic());
                }
            }
        }

        lsp_debug!(
            "Found {} diagnostics for URI: {}",
            diagnostics.len(),
            params.uri
        );
        Ok(Some(diagnostics))
    }
}
