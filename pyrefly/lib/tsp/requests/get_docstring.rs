/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get docstring request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use pyrefly_python::docstring::Docstring;
use lsp_server::ResponseError;
use ruff_text_size::TextSize;

impl Server {
    pub(crate) fn get_docstring(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetDocstringParams,
    ) -> Result<Option<String>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Extract the location information from the declaration
        let Some(node) = &params.decl.node else {
            // If there's no node information, we can't find the docstring
            return Ok(None);
        };

        // Convert Node URI to a handle
        let uri = &node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Ok(None);
        };

        // Get module info for position conversion
        let Some(_module_info) = transaction.get_module_info(&handle) else {
            // If module not loaded in transaction, try to load it
            let Some(fresh_transaction) = self.load_module_if_needed(transaction, &handle, crate::state::require::Require::Everything) else {
                return Ok(None);
            };
            
            return self.extract_docstring_from_transaction(&fresh_transaction, &handle, node);
        };

        // Use the current transaction to find the docstring
        self.extract_docstring_from_transaction(transaction, &handle, node)
    }

    /// Helper method to extract docstring from a transaction, reusing hover logic
    fn extract_docstring_from_transaction(
        &self,
        transaction: &Transaction<'_>,
        handle: &crate::state::handle::Handle,
        node: &tsp::Node,
    ) -> Result<Option<String>, ResponseError> {
        // Convert position to TextSize
        let position = TextSize::new(node.start as u32);

        // Try to find definition at the position - this is the same logic as hover
        if let Some(first_definition) = transaction.find_definition(handle, position, true).into_iter().next() {
            let _definition_metadata = &first_definition.metadata;
            let _definition_range = first_definition.definition_range;
            let docstring_range = first_definition.docstring_range;
            
            if let Some(docstring_range) = docstring_range {
                // Get the module info for this handle
                let Some(module_info) = transaction.get_load(handle) else {
                    return Ok(None);
                };

                // Use the Docstring class to properly format the docstring, same as hover
                let docstring = Docstring(docstring_range, module_info.module_info.clone());
                return Ok(Some(docstring.resolve()));
            }
        }

        // No docstring found
        Ok(None)
    }
}
