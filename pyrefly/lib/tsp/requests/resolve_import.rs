/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP resolve import request implementation

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use lsp_types::Url;
use pyrefly_util::absolutize::Absolutize;

use crate::lsp::module_helpers::to_real_path;
use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn resolve_import(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::ResolveImportParams,
    ) -> Result<Option<lsp_types::Url>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Convert source URI to file path (validation only)
        if params.source_uri.to_file_path().is_err() {
            return Err(ResponseError {
                code: ErrorCode::InvalidParams as i32,
                message: "Invalid source URI - cannot convert to file path".to_owned(),
                data: None,
            });
        }

        // Check if workspace has language services enabled and get the source handle
        let Some(source_handle) = self.make_handle_if_enabled(&params.source_uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled for this workspace".to_owned(),
                data: None,
            });
        };

        // Use the transaction to resolve the import
        let pyrefly_module_name =
            tsp::convert_tsp_module_name_to_pyrefly(&params.module_descriptor);
        match transaction.import_handle(&source_handle, pyrefly_module_name, None) {
            Ok(resolved_handle) => {
                // For import resolution, we don't need to load the module at all.
                // We can get the path directly from the resolved handle and convert it to a URI.
                // This avoids the expensive module loading operation.
                let path = match to_real_path(resolved_handle.path()) {
                    Some(path) => path,
                    None => {
                        lsp_debug!("Could not get real path for: {:?}", resolved_handle.path());
                        return Ok(None);
                    }
                };

                let final_path = path.absolutize();

                match Url::from_file_path(final_path) {
                    Ok(url) => Ok(Some(url)),
                    Err(_) => {
                        lsp_debug!(
                            "Could not convert path to URI for: {:?}",
                            resolved_handle.path()
                        );
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                // For debugging, use {:?} instead of {}
                lsp_debug!("Import resolution failed: {:?}", e);
                // Return None instead of an error if the import cannot be resolved
                Ok(None)
            }
        }
    }
}
