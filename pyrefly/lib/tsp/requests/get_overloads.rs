/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get overloads request implementation

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn get_overloads(
        &self,
        _transaction: &Transaction<'_>,
        params: tsp::GetOverloadsParams,
    ) -> Result<Option<Vec<tsp::Type>>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Get the internal type from the type handle
        let internal_type = match self.lookup_type_from_tsp_type(&params.type_param) {
            Some(t) => t,
            None => {
                lsp_debug!(
                    "Could not resolve type handle: {:?}",
                    params.type_param.handle
                );
                return Ok(None);
            }
        };

        // Only process overloaded function types
        match &internal_type {
            crate::types::types::Type::Overload(overload_type) => {
                let mut result_types = Vec::new();

                // Convert each overload signature to a TSP Type
                for signature in overload_type.signatures.iter() {
                    match signature {
                        crate::types::types::OverloadType::Callable(function) => {
                            // OverloadType::Callable already contains a Function
                            let function_type =
                                crate::types::types::Type::Function(Box::new(function.clone()));
                            result_types.push(self.convert_and_register_type(function_type));
                        }
                        crate::types::types::OverloadType::Forall(forall) => {
                            // Convert Forall<Function> to Function type
                            let function_type =
                                crate::types::types::Type::Function(Box::new(forall.body.clone()));
                            result_types.push(self.convert_and_register_type(function_type));
                        }
                    }
                }

                Ok(Some(result_types))
            }

            // Non-overloaded types return None
            _ => {
                lsp_debug!(
                    "get_overloads called on non-overloaded type: {:?}",
                    internal_type
                );
                Ok(None)
            }
        }
    }
}
