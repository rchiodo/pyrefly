/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get function parts request implementation

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn get_function_parts(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetFunctionPartsParams,
    ) -> Result<Option<tsp::FunctionParts>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        lsp_debug!(
            "Getting function parts for type: {:?}",
            params.type_param.handle
        );

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

        // Extract function parts based on the type
        match &internal_type {
            crate::types::types::Type::Function(func_type) => {
                self.extract_function_parts_from_function(func_type, &params.flags, transaction)
            }
            crate::types::types::Type::Callable(callable_type) => {
                self.extract_function_parts_from_callable(callable_type, &params.flags, transaction)
            }
            crate::types::types::Type::Overload(_overload_type) => {
                // For overloaded functions, we could return the signature of the first overload
                // or a combined representation. For now, let's return None as it's complex.
                lsp_debug!("Function parts for overloaded functions not yet implemented");
                Ok(None)
            }
            _ => {
                lsp_debug!(
                    "get_function_parts only works on function types, got: {:?}",
                    internal_type
                );
                Ok(None)
            }
        }
    }

    fn extract_function_parts_from_function(
        &self,
        func_type: &crate::types::callable::Function,
        flags: &tsp::TypeReprFlags,
        transaction: &Transaction<'_>,
    ) -> Result<Option<tsp::FunctionParts>, ResponseError> {
        // Extract parameter information from the function's signature
        let signature = &func_type.signature;
        self.extract_function_parts_from_callable(signature, flags, transaction)
    }

    fn extract_function_parts_from_callable(
        &self,
        callable_type: &crate::types::callable::Callable,
        flags: &tsp::TypeReprFlags,
        transaction: &Transaction<'_>,
    ) -> Result<Option<tsp::FunctionParts>, ResponseError> {
        // Extract parameter information from callable
        let mut params = Vec::new();

        // Handle different types of params
        match &callable_type.params {
            crate::types::callable::Params::List(param_list) => {
                for param in param_list.items() {
                    let param_str = self.format_param_for_display(param, flags, transaction);
                    params.push(param_str);
                }
            }
            crate::types::callable::Params::Ellipsis => {
                params.push("...".to_string());
            }
            crate::types::callable::Params::ParamSpec(types, param_spec) => {
                // Handle concatenated parameters with a ParamSpec
                for (i, param_type) in types.iter().enumerate() {
                    let type_str = self.format_type_for_display(param_type, flags, transaction);
                    params.push(format!("param{}: {}", i, type_str));
                }
                let param_spec_str = self.format_type_for_display(param_spec, flags, transaction);
                params.push(format!("*{}", param_spec_str));
            }
        }

        // Get return type
        let return_type_str = self.format_type_for_display(&callable_type.ret, flags, transaction);

        Ok(Some(tsp::FunctionParts {
            params,
            return_type: return_type_str,
        }))
    }

    fn format_param_for_display(
        &self,
        param: &crate::types::callable::Param,
        flags: &tsp::TypeReprFlags,
        transaction: &Transaction<'_>,
    ) -> String {
        use crate::types::callable::Param;

        match param {
            Param::PosOnly(name, param_type, _required) => {
                let type_str = self.format_type_for_display(param_type, flags, transaction);
                if let Some(name) = name {
                    format!("{}: {}", name, type_str)
                } else {
                    type_str
                }
            }
            Param::Pos(name, param_type, _required) => {
                let type_str = self.format_type_for_display(param_type, flags, transaction);
                format!("{}: {}", name, type_str)
            }
            Param::VarArg(name, param_type) => {
                let type_str = self.format_type_for_display(param_type, flags, transaction);
                if let Some(name) = name {
                    format!("*{}: {}", name, type_str)
                } else {
                    format!("*{}", type_str)
                }
            }
            Param::KwOnly(name, param_type, _required) => {
                let type_str = self.format_type_for_display(param_type, flags, transaction);
                format!("{}: {}", name, type_str)
            }
            Param::Kwargs(name, param_type) => {
                let type_str = self.format_type_for_display(param_type, flags, transaction);
                if let Some(name) = name {
                    format!("**{}: {}", name, type_str)
                } else {
                    format!("**{}", type_str)
                }
            }
        }
    }

    fn format_type_for_display(
        &self,
        type_obj: &crate::types::types::Type,
        flags: &tsp::TypeReprFlags,
        _transaction: &Transaction<'_>,
    ) -> String {
        // This is a simplified implementation. You might want to use a more sophisticated
        // type formatting system that respects the TypeReprFlags
        if flags.has_expand_type_aliases() {
            // Expand type aliases if requested
            // This would require more complex logic to expand aliases
        }

        if flags.has_convert_to_instance_type() {
            // Convert class types to instance types if requested
            // This would require type conversion logic
        }

        // For now, just use the default string representation
        type_obj.to_string()
    }
}
