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
use crate::tsp::common::snapshot_outdated_error;
use crate::tsp::common::tsp_debug;

/// Extract function parts from a function type
///
/// This is the core logic for getting function parts that can be used independently
/// of the Server implementation for unit testing.
pub fn extract_function_parts_from_function(
    func_type: &crate::types::callable::Function,
    flags: tsp::TypeReprFlags,
    transaction: &Transaction<'_>,
) -> Option<tsp::FunctionParts> {
    // Extract parameter information from the function's signature
    let signature = &func_type.signature;
    extract_function_parts_from_callable(signature, flags, transaction)
}

/// Extract function parts from a callable type
///
/// This is the core logic for getting function parts from callables that can be used
/// independently of the Server implementation for unit testing.
pub fn extract_function_parts_from_callable(
    callable_type: &crate::types::callable::Callable,
    flags: tsp::TypeReprFlags,
    transaction: &Transaction<'_>,
) -> Option<tsp::FunctionParts> {
    // Extract parameter information from callable
    let mut params = Vec::new();

    // Handle different types of params
    match &callable_type.params {
        crate::types::callable::Params::List(param_list) => {
            for param in param_list.items() {
                let param_str = format_param_for_display(param, flags, transaction);
                params.push(param_str);
            }
        }
        crate::types::callable::Params::Ellipsis => {
            params.push("...".to_owned());
        }
        crate::types::callable::Params::ParamSpec(types, param_spec) => {
            // Handle concatenated parameters with a ParamSpec
            for (i, param_type) in types.iter().enumerate() {
                let type_str = format_type_for_display(param_type, flags, transaction);
                params.push(format!("param{i}: {type_str}"));
            }
            let param_spec_str = format_type_for_display(param_spec, flags, transaction);
            params.push(format!("*{param_spec_str}"));
        }
    }

    // Get return type
    let return_type_str = format_type_for_display(&callable_type.ret, flags, transaction);

    Some(tsp::FunctionParts {
        params,
        return_type: return_type_str,
    })
}

/// Format a parameter for display
///
/// This is a helper function that can be used independently for formatting parameters.
pub fn format_param_for_display(
    param: &crate::types::callable::Param,
    flags: tsp::TypeReprFlags,
    transaction: &Transaction<'_>,
) -> String {
    use crate::types::callable::Param;

    match param {
        Param::PosOnly(name, param_type, _required) => {
            let type_str = format_type_for_display(param_type, flags, transaction);
            if let Some(name) = name {
                format!("{name}: {type_str}")
            } else {
                type_str
            }
        }
        Param::Pos(name, param_type, _required) => {
            let type_str = format_type_for_display(param_type, flags, transaction);
            format!("{name}: {type_str}")
        }
        Param::VarArg(name, param_type) => {
            let type_str = format_type_for_display(param_type, flags, transaction);
            if let Some(name) = name {
                format!("*{name}: {type_str}")
            } else {
                format!("*{type_str}")
            }
        }
        Param::KwOnly(name, param_type, _required) => {
            let type_str = format_type_for_display(param_type, flags, transaction);
            format!("{name}: {type_str}")
        }
        Param::Kwargs(name, param_type) => {
            let type_str = format_type_for_display(param_type, flags, transaction);
            if let Some(name) = name {
                format!("**{name}: {type_str}")
            } else {
                format!("**{type_str}")
            }
        }
    }
}

/// Format a type for display
///
/// This is a helper function that can be used independently for formatting types.
pub fn format_type_for_display(
    type_obj: &crate::types::types::Type,
    flags: tsp::TypeReprFlags,
    transaction: &Transaction<'_>,
) -> String {
    let mut working_type = type_obj.clone();

    // Apply type alias expansion if requested
    if flags.has_expand_type_aliases() {
        working_type = expand_type_aliases(working_type, transaction);
    }

    // Apply instance type conversion if requested
    if flags.has_convert_to_instance_type() {
        working_type = convert_to_instance_type(working_type, transaction);
    }

    // Format the final type with variance information if requested
    if flags.has_print_type_var_variance() {
        format_type_with_variance(&working_type)
    } else {
        working_type.to_string()
    }
}

/// Expand type aliases recursively
fn expand_type_aliases(
    type_obj: crate::types::types::Type,
    transaction: &Transaction<'_>,
) -> crate::types::types::Type {
    use pyrefly_types::simplify::unions;

    use crate::types::types::Type;

    match type_obj {
        Type::TypeAlias(alias) => {
            // Get the underlying type from the alias
            let underlying = alias.as_type();
            // Recursively expand any nested aliases
            expand_type_aliases(underlying, transaction)
        }
        Type::Union(union_types) => {
            // Expand aliases in union members
            let expanded_types: Vec<_> = union_types
                .iter()
                .map(|t| expand_type_aliases(t.clone(), transaction))
                .collect();
            unions(expanded_types)
        }
        Type::ClassType(class_type) => {
            // Expand aliases in type arguments
            let expanded_targs = expand_type_args_aliases(class_type.targs(), transaction);
            if &expanded_targs != class_type.targs() {
                Type::ClassType(crate::types::class::ClassType::new(
                    class_type.class_object().clone(),
                    expanded_targs,
                ))
            } else {
                Type::ClassType(class_type)
            }
        }
        Type::Function(func) => {
            // Expand aliases in function signature
            let expanded_callable = expand_callable_aliases(&func.signature, transaction);
            if &expanded_callable != &func.signature {
                // Create a new function with expanded signature
                // Note: We can't modify the existing function, so we return the original for now
                // In a real implementation, you'd need to create a new Function type
                Type::Function(func)
            } else {
                Type::Function(func)
            }
        }
        Type::Callable(callable) => {
            // Expand aliases in callable signature
            let expanded = expand_callable_aliases(&callable, transaction);
            Type::Callable(Box::new(expanded))
        }
        // For other types, return as-is
        _ => type_obj,
    }
}

/// Convert class types to instance types
fn convert_to_instance_type(
    type_obj: crate::types::types::Type,
    transaction: &Transaction<'_>,
) -> crate::types::types::Type {
    use pyrefly_types::simplify::unions;

    use crate::types::types::Type;

    match type_obj {
        Type::ClassDef(class_def) => {
            // Convert ClassDef to ClassType (instance)
            let empty_tparams = std::sync::Arc::new(crate::types::types::TParams::new(Vec::new()));
            let empty_targs = crate::types::types::TArgs::new(empty_tparams, Vec::new());
            Type::ClassType(crate::types::class::ClassType::new(class_def, empty_targs))
        }
        Type::Union(union_types) => {
            // Convert class types in union members
            let converted_types: Vec<_> = union_types
                .iter()
                .map(|t| convert_to_instance_type(t.clone(), transaction))
                .collect();
            unions(converted_types)
        }
        // For other types, return as-is
        _ => type_obj,
    }
}

/// Format a type with variance information for type variables
fn format_type_with_variance(type_obj: &crate::types::types::Type) -> String {
    use pyrefly_types::type_var::PreInferenceVariance;

    use crate::types::types::Type;

    match type_obj {
        Type::TypeVar(tvar) => {
            let base_name = tvar.qname().id().to_string();

            // Add variance information if available
            match tvar.variance() {
                PreInferenceVariance::PCovariant => format!("{base_name} (covariant)"),
                PreInferenceVariance::PContravariant => format!("{base_name} (contravariant)"),
                PreInferenceVariance::PInvariant => format!("{base_name} (invariant)"),
                PreInferenceVariance::PUndefined => base_name,
            }
        }
        _ => type_obj.to_string(),
    }
}

/// Helper function to expand type aliases in type arguments
fn expand_type_args_aliases(
    targs: &crate::types::types::TArgs,
    _transaction: &Transaction<'_>,
) -> crate::types::types::TArgs {
    // For now, return the original targs since we need access to the internal structure
    // In a full implementation, you'd iterate through targs and expand each type
    targs.clone()
}

/// Helper function to expand type aliases in callable signatures
fn expand_callable_aliases(
    callable: &crate::types::callable::Callable,
    transaction: &Transaction<'_>,
) -> crate::types::callable::Callable {
    use crate::types::callable::Callable;
    use crate::types::callable::Params;

    let expanded_ret = expand_type_aliases(callable.ret.clone(), transaction);

    let expanded_params = match &callable.params {
        Params::List(param_list) => {
            let expanded_items: Vec<_> = param_list
                .items()
                .iter()
                .map(|param| expand_param_aliases(param, transaction))
                .collect();
            Params::List(crate::types::callable::ParamList::new(expanded_items))
        }
        Params::Ellipsis => Params::Ellipsis,
        Params::ParamSpec(types, param_spec) => {
            let expanded_types: Vec<_> = types
                .iter()
                .map(|t| expand_type_aliases(t.clone(), transaction))
                .collect();
            let expanded_param_spec = expand_type_aliases(param_spec.clone(), transaction);
            Params::ParamSpec(expanded_types.into_boxed_slice(), expanded_param_spec)
        }
    };

    // Create a new callable based on the original method
    match (&callable.params, &expanded_params) {
        (Params::List(_), Params::List(new_list)) => Callable::list(new_list.clone(), expanded_ret),
        (Params::Ellipsis, Params::Ellipsis) => Callable::ellipsis(expanded_ret),
        (Params::ParamSpec(_, _), Params::ParamSpec(new_types, new_param_spec)) => {
            Callable::concatenate(new_types.clone(), new_param_spec.clone(), expanded_ret)
        }
        // Fallback to original callable if structure doesn't match
        _ => callable.clone(),
    }
}

/// Helper function to expand type aliases in function parameters
fn expand_param_aliases(
    param: &crate::types::callable::Param,
    transaction: &Transaction<'_>,
) -> crate::types::callable::Param {
    use crate::types::callable::Param;

    match param {
        Param::PosOnly(name, param_type, required) => {
            let expanded_type = expand_type_aliases(param_type.clone(), transaction);
            Param::PosOnly(name.clone(), expanded_type, required.clone())
        }
        Param::Pos(name, param_type, required) => {
            let expanded_type = expand_type_aliases(param_type.clone(), transaction);
            Param::Pos(name.clone(), expanded_type, required.clone())
        }
        Param::VarArg(name, param_type) => {
            let expanded_type = expand_type_aliases(param_type.clone(), transaction);
            Param::VarArg(name.clone(), expanded_type)
        }
        Param::KwOnly(name, param_type, required) => {
            let expanded_type = expand_type_aliases(param_type.clone(), transaction);
            Param::KwOnly(name.clone(), expanded_type, required.clone())
        }
        Param::Kwargs(name, param_type) => {
            let expanded_type = expand_type_aliases(param_type.clone(), transaction);
            Param::Kwargs(name.clone(), expanded_type)
        }
    }
}

impl Server {
    pub(crate) fn get_function_parts(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetFunctionPartsParams,
    ) -> Result<Option<tsp::FunctionParts>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(snapshot_outdated_error());
        }

        tsp_debug!(
            "Getting function parts for type: {:?}",
            params.type_param.handle
        );

        // Get the internal type from the type handle
        let internal_type = match self.lookup_type_from_tsp_type(&params.type_param) {
            Some(t) => t,
            None => {
                tsp_debug!(
                    "Could not resolve type handle: {:?}",
                    params.type_param.handle
                );
                return Ok(None);
            }
        };

        // Extract function parts based on the type
        match &internal_type {
            crate::types::types::Type::Function(func_type) => Ok(
                extract_function_parts_from_function(func_type, params.flags, transaction),
            ),
            crate::types::types::Type::Callable(callable_type) => Ok(
                extract_function_parts_from_callable(callable_type, params.flags, transaction),
            ),
            crate::types::types::Type::Overload(_overload_type) => {
                // For overloaded functions, we could return the signature of the first overload
                // or a combined representation. For now, let's return None as it's complex.
                tsp_debug!("Function parts for overloaded functions not yet implemented");
                Ok(None)
            }
            _ => {
                tsp_debug!(
                    "get_function_parts only works on function types, got: {:?}",
                    internal_type
                );
                Ok(None)
            }
        }
    }
}
