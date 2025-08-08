/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getTypeAttributes TSP request

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::module::module_info::ModuleInfo;
use crate::state::handle::Handle;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;
use crate::types::types::Type;

/// Extract all attributes from a pyrefly type
/// This function examines a type and returns all its attributes (class members, function parameters, etc.)
pub fn extract_type_attributes(
    transaction: &Transaction<'_>,
    pyrefly_type: &Type,
    handle_factory: impl Fn(&ModuleInfo) -> Handle,
) -> Result<Vec<tsp::Attribute>, ResponseError> {
    let mut attributes = Vec::new();

    match pyrefly_type {
        Type::ClassType(class_type) => {
            // Get the module info from the class type
            let class_qname = class_type.class_object().qname();
            let module_info = class_qname.module();

            // Create a handle for the module containing the class
            let handle = handle_factory(module_info);

            // Use ad_hoc_solve to access the solver and get all class attributes
            let result = transaction.ad_hoc_solve(&handle, |solver| {
                let class_instance_type = Type::ClassType(class_type.clone());

                // Get all field names from the class
                let field_names: Vec<_> = class_type
                    .class_object()
                    .fields()
                    .collect();

                let mut class_attributes = Vec::new();

                // For each field, get its type using the solver
                for field_name in field_names {
                    let attribute_type = solver.type_of_attr_get(
                        &class_instance_type,
                        field_name, // This is already &Name from the iterator
                        ruff_text_size::TextRange::default(), // Use a default range
                        &crate::error::collector::ErrorCollector::new(
                            module_info.clone(),
                            crate::error::style::ErrorStyle::Never,
                        ), // Create a temporary error collector
                        None,                                 // No context
                        "get_type_attributes",                // Context description
                    );

                    // Convert the pyrefly type to TSP type
                    let tsp_type = crate::tsp::protocol::convert_to_tsp_type(attribute_type);

                    let attribute = tsp::Attribute {
                        name: field_name.as_str().to_owned(),
                        type_info: tsp_type,
                        owner: None, // Could be enhanced to include owner type
                        bound_type: None,
                        flags: tsp::AttributeFlags::NONE,
                        decls: Vec::new(), // Could be enhanced with declaration info
                    };
                    class_attributes.push(attribute);
                }

                class_attributes
            });

            if let Some(class_attrs) = result {
                attributes.extend(class_attrs);
            }
        }

        Type::Function(function_type) => {
            // For functions, we could return the parameters as attributes
            // For now, we'll return empty as function parameter introspection is complex
            // TODO: Implement function parameter extraction when public API is available
            lsp_debug!(
                "Function parameter extraction not yet implemented for: {:?}",
                function_type.signature
            );
        }

        Type::Module(module_type) => {
            // For modules, we could potentially list exported symbols
            // For now, we'll return empty as module introspection is complex
            lsp_debug!(
                "Module attribute extraction not yet implemented for: {:?}",
                module_type
            );
        }

        Type::Overload(overload_type) => {
            // For overloaded functions, we could show all overload signatures
            // For now, we'll return a summary
            for (i, signature) in overload_type.signatures.iter().enumerate() {
                let signature_name = format!("overload_{}", i);
                match signature {
                    crate::types::types::OverloadType::Callable(function) => {
                        let function_type = Type::Function(Box::new(function.clone()));
                        let tsp_type = crate::tsp::protocol::convert_to_tsp_type(function_type);

                        let attribute = tsp::Attribute {
                            name: signature_name,
                            type_info: tsp_type,
                            owner: None,
                            bound_type: None,
                            flags: tsp::AttributeFlags::NONE,
                            decls: Vec::new(),
                        };
                        attributes.push(attribute);
                    }
                    crate::types::types::OverloadType::Forall(forall) => {
                        let function_type = Type::Function(Box::new(forall.body.clone()));
                        let tsp_type = crate::tsp::protocol::convert_to_tsp_type(function_type);

                        let attribute = tsp::Attribute {
                            name: signature_name,
                            type_info: tsp_type,
                            owner: None,
                            bound_type: None,
                            flags: tsp::AttributeFlags::NONE,
                            decls: Vec::new(),
                        };
                        attributes.push(attribute);
                    }
                }
            }
        }

        _ => {
            // For other types (primitives, unions, etc.), there are no attributes
            lsp_debug!("No attributes available for type: {:?}", pyrefly_type);
        }
    }

    Ok(attributes)
}

impl Server {
    pub(crate) fn get_type_attributes(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeAttributesParams,
    ) -> Result<Option<Vec<tsp::Attribute>>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        lsp_debug!("Getting attributes for type: {:?}", params.type_param);

        // Convert TSP type to pyrefly type
        let pyrefly_type = match self.lookup_type_from_tsp_type(&params.type_param) {
            Some(pyrefly_type) => pyrefly_type,
            None => {
                lsp_debug!("Could not convert TSP type to pyrefly type");
                return Ok(Some(Vec::new()));
            }
        };

        // Extract attributes from the pyrefly type
        let attributes = extract_type_attributes(transaction, &pyrefly_type, |module_info| {
            let module_path = module_info.path().clone();
            let module_name = module_info.name();
            let config = self
                .state
                .config_finder()
                .python_file(module_name, &module_path);
            Handle::new(module_name, module_path, config.get_sys_info())
        })?;

        lsp_debug!("Found {} attributes", attributes.len());
        Ok(Some(attributes))
    }
}
