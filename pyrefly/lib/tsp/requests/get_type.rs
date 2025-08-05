/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get type request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;
use lsp_server::{ErrorCode, ResponseError};

impl Server {
    pub(crate) fn get_type(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Convert Node to URI and position
        let uri = &params.node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_string(),
                data: None,
            });
        };

        // Try to get module info, loading it if necessary
        let (module_info, fresh_transaction) = match self.get_module_info_with_loading(transaction, &handle) {
            Ok((Some(info), fresh_tx)) => (info, fresh_tx),
            Ok((None, _)) => {
                lsp_debug!("Warning: Could not load module for get_type request: {}", uri);
                return Ok(None);
            },
            Err(_) => {
                return Err(ResponseError {
                    code: ErrorCode::InternalError as i32,
                    message: "Failed to load module".to_string(),
                    data: None,
                });
            }
        };

        // Use the appropriate transaction (fresh if module was loaded, original if already loaded)
        let active_transaction = fresh_transaction.as_ref().unwrap_or(transaction);

        // Convert range start to TextSize position using module_info
        let position = module_info.lined_buffer().from_lsp_position(params.node.range.start);

        // Try to get the type at the specified position
        let Some(type_info) = active_transaction.get_type_at(&handle, position) else {
            lsp_debug!("Warning: Could not get type at position {:?} in {}", params.node.range.start, uri);
            return Ok(None);
        };

        // Convert pyrefly Type to TSP Type format
        Ok(Some(self.convert_and_register_type(type_info)))
    }
}
