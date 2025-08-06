/*
 * Copyright /// Standalone get_type function that can be used independently of the Server
/// This follows the same pattern as the hover feature
pub fn get_type(
    transaction: &Transaction<'_>, 
    handle: &Handle,
    module_info: &ModuleInfo,
    params: &tsp::GetTypeParams,
) -> Option<tsp::Type> { Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get type request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::state::handle::Handle;
use crate::module::module_info::ModuleInfo;
use crate::tsp;
use crate::tsp::common::lsp_debug;
use lsp_server::{ErrorCode, ResponseError};

/// Standalone get_type function that can be used independently of the Server
/// This follows the same pattern as the hover feature
pub fn get_type(
    transaction: &Transaction<'_>,
    handle: &Handle,
    module_info: &ModuleInfo,
    params: &tsp::GetTypeParams,
) -> Option<tsp::Type> {
    // Convert range start to TextSize position using module_info
    let position = module_info.lined_buffer().from_lsp_position(params.node.range.start);

    // Try to get the type at the specified position
    let type_info = transaction.get_type_at(handle, position)?;

    // Convert pyrefly Type to TSP Type format using protocol helper
    Some(crate::tsp::protocol::convert_to_tsp_type(type_info))
}

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

        // Call the standalone get_type function
        let tsp_type = get_type(active_transaction, &handle, &module_info, &params);

        // Register the type in the lookup table (Server-specific functionality)
        if let Some(ref tsp_type) = tsp_type {
            if let tsp::TypeHandle::String(handle_str) = &tsp_type.handle {
                // Extract the pyrefly type from the TSP type for registration
                if let Some(pyrefly_type) = active_transaction.get_type_at(&handle, module_info.lined_buffer().from_lsp_position(params.node.range.start)) {
                    self.state.register_type_handle(handle_str.clone(), pyrefly_type);
                }
            }
        }

        Ok(tsp_type)
    }
}
