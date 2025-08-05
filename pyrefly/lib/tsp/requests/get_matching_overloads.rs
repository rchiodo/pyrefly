/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get matching overloads request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;
use lsp_server::ResponseError;
use ruff_text_size::TextSize;

impl Server {
    /// Get matching overloads for a function call site
    /// 
    /// This function analyzes a call site and returns the overloads of the function
    /// being called that could potentially match the call. Currently, it returns all
    /// available overloads since full argument analysis is not yet implemented.
    /// 
    /// In a complete implementation, this would:
    /// 1. Parse the call arguments at the call site
    /// 2. Match argument types against each overload's parameter types
    /// 3. Return only the overloads that could accept the provided arguments
    pub(crate) fn get_matching_overloads(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetMatchingOverloadsParams,
    ) -> Result<Option<Vec<tsp::Type>>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        lsp_debug!("Getting matching overloads for call node: {:?}", params.call_node);

        // Get the URI and check if workspace has language services enabled
        let uri = &params.call_node.uri;
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            lsp_debug!("Language services disabled for workspace");
            return Ok(None);
        };

        // Get module info to ensure the module is loaded
        let Some(module_info) = transaction.get_module_info(&handle) else {
            lsp_debug!("Module not loaded for handle: {:?}", handle);
            return Ok(None);
        };

        // Convert the Range position to TextSize using the module's line buffer
        let position = module_info.lined_buffer().from_lsp_position(params.call_node.range.start);

        // Try to find the type at the call site position
        let Some(call_site_type) = self.get_type_at_position(transaction, &handle, position) else {
            lsp_debug!("Could not determine type at call site position");
            return Ok(None);
        };

        // Check if the call site type is an overloaded function
        match &call_site_type {
            crate::types::types::Type::Overload(overload_type) => {
                // TODO: Implement sophisticated argument matching
                // For now, we return all overloads since we don't have argument analysis yet.
                // In a complete implementation, we would:
                // 1. Parse the call arguments from the AST at the call site
                // 2. Infer types of each argument
                // 3. Match argument types against each overload's parameter signature
                // 4. Return only overloads where arguments could match parameters
                
                let mut result_types = Vec::new();
                
                for signature in overload_type.signatures.iter() {
                    match signature {
                        crate::types::types::OverloadType::Callable(function) => {
                            let function_type = crate::types::types::Type::Function(Box::new(function.clone()));
                            result_types.push(self.convert_and_register_type(function_type));
                        },
                        crate::types::types::OverloadType::Forall(forall) => {
                            let function_type = crate::types::types::Type::Function(Box::new(forall.body.clone()));
                            result_types.push(self.convert_and_register_type(function_type));
                        },
                    }
                }
                
                lsp_debug!("Found {} matching overloads", result_types.len());
                Ok(Some(result_types))
            },
            
            // If it's a regular function, return it as a single-item array
            crate::types::types::Type::Function(_) => {
                lsp_debug!("Found single function at call site");
                let tsp_type = self.convert_and_register_type(call_site_type);
                Ok(Some(vec![tsp_type]))
            },
            
            // Other types don't have overloads
            _ => {
                lsp_debug!("Type at call site is not a function or overload: {:?}", call_site_type);
                Ok(None)
            }
        }
    }

    /// Helper method to get the type at a specific position in a module
    fn get_type_at_position(
        &self,
        transaction: &Transaction<'_>,
        handle: &crate::state::handle::Handle,
        position: TextSize,
    ) -> Option<crate::types::types::Type> {
        transaction.get_type_at(handle, position)
    }
}
