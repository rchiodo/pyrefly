/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP search for type attribute request implementation

use lsp_server::ResponseError;
use tsp_types::snapshot_outdated_error;
use tsp_types::tsp_debug;
use tsp_types::{self as tsp};

use crate::module::module_info::ModuleInfo;
use crate::state::handle::Handle;
use crate::state::state::Transaction;
use crate::tsp::server::TspServer;

/// Search for an attribute in a class type using the solver
///
/// This is the core logic for searching attributes that can be used independently
/// of the Server implementation for unit testing.
pub fn search_attribute_in_class_type(
    transaction: &Transaction<'_>,
    class_type: &crate::types::class::ClassType,
    attribute_name: &str,
    handle_factory: impl Fn(&ModuleInfo) -> Handle,
) -> Option<crate::types::types::Type> {
    // Convert string attribute name to ruff_python_ast::name::Name
    use ruff_python_ast::name::Name;
    let attr_name = Name::new(attribute_name);

    // Get the module info from the class type
    let class_qname = class_type.class_object().qname();
    let module_info = class_qname.module();

    // Create a handle for the module containing the class
    let handle = handle_factory(module_info);

    // Use ad_hoc_solve to access the solver and get the attribute type
    let result = transaction.ad_hoc_solve(&handle, |solver| {
        // Use type_of_attr_get to get the resolved type
        let class_instance_type = crate::types::types::Type::ClassType(class_type.clone());
        let attribute_type = solver.type_of_attr_get(
            &class_instance_type,
            &attr_name,
            ruff_text_size::TextRange::default(), // Use a default range
            &crate::error::collector::ErrorCollector::new(
                module_info.clone(),
                crate::error::style::ErrorStyle::Never,
            ), // Create a temporary error collector
            None,                                 // No context
            "search_for_type_attribute",          // Context description
        );

        attribute_type
    });

    result
}

/// Convert a pyrefly Type to TSP Attribute format
///
/// This helper function creates a TSP Attribute from a pyrefly Type and attribute name.
pub fn create_tsp_attribute_from_type(
    attribute_type: crate::types::types::Type,
    attribute_name: &str,
    type_converter: impl Fn(crate::types::types::Type) -> tsp::Type,
) -> tsp::Attribute {
    let tsp_type = type_converter(attribute_type);
    let flags = tsp::AttributeFlags::NONE.0;
    tsp::Attribute {
        name: attribute_name.to_owned(),
        type_: tsp_type,
        owner: None,
        bound_type: None,
        flags,
        decls: Vec::new(),
    }
}

impl TspServer {
    pub fn search_for_type_attribute(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::SearchForTypeAttributeParams,
    ) -> Result<Option<tsp::Attribute>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(snapshot_outdated_error());
        }

        tsp_debug!(
            "Searching for attribute '{}' with access flags: {:?}",
            params.attribute_name,
            params.access_flags
        );

        // Get the internal type from the start_type handle
        let internal_type = match self.lookup_type_from_tsp_type(&params.start_type) {
            Some(t) => t,
            None => {
                tsp_debug!(
                    "Could not resolve type handle: {:?}",
                    params.start_type.handle
                );
                return Ok(None);
            }
        };

        // Only work on class types - this method is specifically for class attribute lookup
        match &internal_type {
            crate::types::types::Type::ClassType(class_type) => {
                // Use standalone function to search for attribute
                let attribute_type = search_attribute_in_class_type(
                    transaction,
                    class_type,
                    &params.attribute_name,
                    |module_info| self.create_handle_for_module(module_info),
                );

                match attribute_type {
                    Some(attr_type) => {
                        tsp_debug!(
                            "Found attribute '{}' in class type with type: {:?}",
                            params.attribute_name,
                            attr_type
                        );

                        // Convert to TSP attribute using standalone function
                        let tsp_attribute = create_tsp_attribute_from_type(
                            attr_type,
                            &params.attribute_name,
                            |attr_type| self.convert_and_register_type(attr_type),
                        );
                        Ok(Some(tsp_attribute))
                    }
                    None => {
                        tsp_debug!(
                            "Could not resolve attribute '{}' in class type due to ad_hoc_solve failure",
                            params.attribute_name
                        );
                        
                        // Still return an attribute with Unknown type instead of None
                        let unknown_type = crate::types::types::Type::Any(crate::types::types::AnyStyle::Implicit);
                        let tsp_attribute = create_tsp_attribute_from_type(
                            unknown_type,
                            &params.attribute_name,
                            |attr_type| self.convert_and_register_type(attr_type),
                        );
                        Ok(Some(tsp_attribute))
                    }
                }
            }
            _ => {
                tsp_debug!(
                    "search_for_type_attribute only works on class types, got: {:?}",
                    internal_type
                );
                
                // Return an attribute with Unknown type instead of None for non-class types
                let unknown_type = crate::types::types::Type::Any(crate::types::types::AnyStyle::Implicit);
                let tsp_attribute = create_tsp_attribute_from_type(
                    unknown_type,
                    &params.attribute_name,
                    |attr_type| self.convert_and_register_type(attr_type),
                );
                Ok(Some(tsp_attribute))
            }
        }
    }

    /// Create a handle for a module (Server-specific functionality)
    fn create_handle_for_module(&self, module_info: &ModuleInfo) -> Handle {
        let module_path = module_info.path().clone();
        let module_name = module_info.name();
        let config = self
            .inner
            .state
            .config_finder()
            .python_file(module_name, &module_path);
        Handle::new(module_name, module_path, config.get_sys_info())
    }
}
