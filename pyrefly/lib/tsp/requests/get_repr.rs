/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get repr request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use lsp_server::ResponseError;

// Import the lsp_debug macro from common
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn get_repr(
        &self,
        _transaction: &Transaction<'_>,
        params: tsp::GetReprParams,
    ) -> Result<String, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Use the handle mapping to get the actual pyrefly type
        let Some(internal_type) = self.lookup_type_from_tsp_type(&params.type_param) else {
            // If we can't find the internal type, fall back to the basic formatter
            lsp_debug!("Warning: Could not resolve type handle for repr: {:?}", params.type_param.handle);
            let type_repr = format_type_representation(&params.type_param, params.flags);
            return Ok(type_repr);
        };

        // Use pyrefly's native type formatting
        let type_repr = if params.flags.has_convert_to_instance_type() {
            // Convert class types to instance types
            match &internal_type {
                crate::types::types::Type::ClassDef(class) => {
                    // Convert ClassDef to ClassType (instance)
                    let empty_tparams = std::sync::Arc::new(crate::types::types::TParams::new(Vec::new()));
                    let empty_targs = crate::types::types::TArgs::new(empty_tparams, Vec::new());
                    let class_type = crate::types::class::ClassType::new(
                        class.clone(),
                        empty_targs,
                    );
                    format!("{}", crate::types::types::Type::ClassType(class_type))
                }
                _ => format!("{}", internal_type)
            }
        } else {
            // Standard type representation
            format!("{}", internal_type)
        };

        // Apply additional formatting based on flags
        let final_repr = if params.flags.has_expand_type_aliases() {
            // For now, we don't have specific alias expansion logic in the Display impl,
            // but this is where we would implement it if needed
            type_repr
        } else {
            type_repr
        };

        lsp_debug!("Generated repr for type {:?}: {}", params.type_param.handle, final_repr);
        Ok(final_repr)
    }
}

/// Format a type representation when internal type lookup fails
fn format_type_representation(type_param: &tsp::Type, flags: tsp::TypeReprFlags) -> String {
    use tsp::TypeCategory;
    
    let mut result = String::new();
    
    // Handle different type categories
    match type_param.category {
        TypeCategory::ANY => result.push_str("Any"),
        TypeCategory::FUNCTION => {
            // For functions, show signature if available
            if type_param.name.is_empty() {
                result.push_str("Callable[..., Any]");
            } else {
                result.push_str(&type_param.name);
            }
        },
        TypeCategory::OVERLOADED => {
            result.push_str("Overload[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::CLASS => {
            // For classes, show the class name
            if flags.has_convert_to_instance_type() {
                // Convert to instance type representation
                result.push_str(&type_param.name);
            } else {
                // Show as type
                result.push_str("type[");
                result.push_str(&type_param.name);
                result.push(']');
            }
        },
        TypeCategory::MODULE => {
            result.push_str("Module[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::UNION => {
            // For unions, we'd need to format multiple types
            result.push_str("Union[");
            result.push_str(&type_param.name);
            result.push(']');
        },
        TypeCategory::TYPE_VAR => {
            result.push_str(&type_param.name);
            // Add variance information if requested
            if flags.has_print_type_var_variance() {
                // This would require additional metadata about variance
                // For now, just show the basic type var name
            }
        },
        _ => {
            // Default case for unknown categories
            if type_param.name.is_empty() {
                result.push_str("Unknown");
            } else {
                result.push_str(&type_param.name);
            }
        }
    }
    
    // Add module information if available and it's not a builtin
    if let Some(module_name) = &type_param.module_name {
        if !module_name.name_parts.is_empty() && module_name.name_parts[0] != "builtins" {
            let module_path = module_name.name_parts.join(".");
            result = format!("{}.{}", module_path, result);
        }
    }

    result
}
