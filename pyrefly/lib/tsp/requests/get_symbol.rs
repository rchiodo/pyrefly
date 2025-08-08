/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getSymbol TSP request

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use ruff_text_size::TextRange;

use crate::lsp::module_helpers::module_info_to_uri;
use crate::lsp::server::Server;
use crate::module::module_info::ModuleInfo;
use crate::state::lsp::FindDefinitionItemWithDocstring;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

/// Extract symbol name from a node or use the provided name
///
/// This is a helper function that can be used independently for extracting symbol names.
pub fn extract_symbol_name(
    params_name: Option<String>,
    node: &tsp::Node,
    module_info: &ModuleInfo,
) -> String {
    params_name.unwrap_or_else(|| {
        let start = module_info
            .lined_buffer()
            .from_lsp_position(node.range.start);
        let end = module_info.lined_buffer().from_lsp_position(node.range.end);
        module_info.code_at(TextRange::new(start, end)).to_owned()
    })
}

/// Create a declaration from definition metadata
///
/// This is the core logic for creating TSP declarations that can be used independently
/// of the Server implementation for unit testing.
pub fn create_declaration_from_definition(
    definition: &FindDefinitionItemWithDocstring,
    position: ruff_text_size::TextSize,
    name: &str,
    node_range: lsp_types::Range,
    node_uri: lsp_types::Url,
    type_info: Option<&crate::types::types::Type>,
) -> tsp::Declaration {
    let definition_metadata = &definition.metadata;
    let definition_module = &definition.module;

    // Generate a unique handle for this declaration
    let declaration_handle = tsp::TypeHandle::String(format!(
        "decl_{:p}_{}",
        definition_metadata as *const _,
        u32::from(position)
    ));

    // Determine the category and flags based on definition metadata
    let (category, flags) = match definition_metadata {
        crate::state::lsp::DefinitionMetadata::Variable(Some(symbol_kind)) => {
            Server::symbol_kind_to_tsp_category(*symbol_kind)
        }
        crate::state::lsp::DefinitionMetadata::Module => {
            // For module imports, check if type info is available to determine if resolved
            let mut import_flags = tsp::DeclarationFlags::new();
            if type_info.is_none() {
                // If we can't get type info for an import, it might be unresolved
                import_flags = import_flags.with_unresolved_import();
            }
            (tsp::DeclarationCategory::IMPORT, import_flags)
        }
        crate::state::lsp::DefinitionMetadata::Attribute(_) => {
            // Attributes are typically class members
            (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new().with_class_member(),
            )
        }
        crate::state::lsp::DefinitionMetadata::VariableOrAttribute(_, Some(symbol_kind)) => {
            Server::symbol_kind_to_tsp_category_with_class_member(*symbol_kind)
        }
        _ => (
            tsp::DeclarationCategory::VARIABLE,
            tsp::DeclarationFlags::new(),
        ),
    };

    // Extract module name from the definition's module (where the symbol is actually defined)
    let definition_module_name = definition_module.name();
    let module_name = Server::create_tsp_module_name(definition_module_name.as_str());

    // Check if this is from builtins and update category/flags accordingly
    let (category, flags) = Server::apply_builtins_category(category, flags, &module_name);

    // Create the declaration node
    let declaration_node = tsp::Node {
        uri: node_uri.clone(),
        range: node_range,
    };

    tsp::Declaration {
        handle: declaration_handle,
        category,
        flags,
        node: Some(declaration_node),
        module_name,
        name: name.to_owned(),
        uri: node_uri,
    }
}

/// Determine the appropriate node range and URI for a declaration
///
/// This helper function determines whether to use the original node range/URI
/// or the definition range/URI based on whether the definition is in the same module.
pub fn determine_declaration_node_info(
    definition_module: &ModuleInfo,
    current_module: &ModuleInfo,
    definition_range: ruff_text_size::TextRange,
    original_node: &tsp::Node,
) -> (lsp_types::Range, lsp_types::Url) {
    // For declarations in different modules, use the original input range instead of the definition range
    // because the definition range points to the target module, not the symbol in the current module
    if definition_module.name() != current_module.name() {
        // Different module: use the original range in the current module
        (original_node.range, original_node.uri.clone())
    } else {
        // Same module: use the definition range and URI
        let definition_uri = module_info_to_uri(definition_module);
        let definition_uri_final = definition_uri.unwrap_or_else(|| original_node.uri.clone());
        (
            current_module.lined_buffer().to_lsp_range(definition_range),
            definition_uri_final,
        )
    }
}

/// Extract symbol information from a transaction at a specific position
///
/// This is the core logic for getting symbol information that can be used independently
/// of the Server implementation for unit testing.
pub fn extract_symbol_from_transaction(
    transaction: &Transaction<'_>,
    handle: &crate::state::handle::Handle,
    module_info: &ModuleInfo,
    position: ruff_text_size::TextSize,
    params: &tsp::GetSymbolParams,
) -> Option<(
    String,
    Vec<tsp::Declaration>,
    Vec<crate::types::types::Type>,
)> {
    // First, check if we can get type information at this position
    let type_info = transaction.get_type_at(handle, position);

    // Try to find definition at the position
    if let Some(first_definition) = transaction
        .find_definition(handle, position, true)
        .into_iter()
        .next()
    {
        let definition_range = first_definition.definition_range;
        let definition_module = &first_definition.module;

        // Extract symbol name
        let name = extract_symbol_name(params.name.clone(), &params.node, module_info);

        // Determine the appropriate node range and URI
        let (node_range, node_uri) = determine_declaration_node_info(
            definition_module,
            module_info,
            definition_range,
            &params.node,
        );

        // Create the declaration
        let declaration = create_declaration_from_definition(
            &first_definition,
            position,
            &name,
            node_range,
            node_uri,
            type_info.as_ref(),
        );

        // Validate that the declaration node's text matches the expected name
        let validation_module_info = if definition_module.name() != module_info.name() {
            // Different module: use the current module since we're using the original range
            module_info
        } else {
            // Same module: use the current module (which is the same as definition module)
            module_info
        };

        let declaration_text_range = validation_module_info
            .lined_buffer()
            .from_lsp_range(declaration.node.as_ref().unwrap().range);
        let declaration_text = validation_module_info.code_at(declaration_text_range);

        if declaration_text != name {
            panic!(
                "Declaration node text '{}' doesn't match expected name '{}' at range {:?} in module {} (definition in module {})",
                declaration_text,
                name,
                declaration_text_range,
                validation_module_info.name(),
                definition_module.name()
            );
        }

        // Get synthesized types if available
        let mut synth_types = Vec::new();
        if let Some(type_info) = type_info {
            synth_types.push(type_info);
        }

        Some((name, vec![declaration], synth_types))
    } else {
        // If no definition found, try to get type information at least
        let name = extract_symbol_name(params.name.clone(), &params.node, module_info);

        if let Some(type_info) = type_info {
            Some((name, Vec::new(), vec![type_info]))
        } else {
            // No definition found and no type information available
            lsp_debug!(
                "Warning: No symbol definition or type information found at position {} in {}",
                position.to_usize(),
                params.node.uri
            );
            None
        }
    }
}

impl Server {
    pub(crate) fn get_symbol(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetSymbolParams,
    ) -> Result<Option<tsp::Symbol>, ResponseError> {
        // Common validation logic
        self.validate_snapshot(params.snapshot)?;

        // Convert Node to URI and position
        let uri = &params.node.uri;

        // Common language services validation
        self.validate_language_services(uri)?;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_owned(),
                data: None,
            });
        };

        // Get module info for position conversion
        // If the module is not loaded in the transaction, try to load it
        let (module_info, transaction_to_use) = match transaction.get_module_info(&handle) {
            Some(info) => (info, None), // Use the existing transaction
            None => {
                // Module not loaded in transaction, try to load it
                let Some(fresh_transaction) = self.load_module_if_needed(
                    transaction,
                    &handle,
                    crate::state::require::Require::Everything,
                ) else {
                    return Err(ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: "Failed to load module".to_owned(),
                        data: None,
                    });
                };

                let Some(info) = fresh_transaction.get_module_info(&handle) else {
                    return Err(ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: "Failed to get module info after loading".to_owned(),
                        data: None,
                    });
                };

                (info, Some(fresh_transaction))
            }
        };

        // Use the appropriate transaction for the rest of the function
        let active_transaction = transaction_to_use.as_ref().unwrap_or(transaction);

        // Convert range start to TextSize using module_info
        let position = module_info
            .lined_buffer()
            .from_lsp_position(params.node.range.start);

        // Extract symbol information using standalone function
        let Some((symbol_name, declarations, synthesized_internal_types)) =
            extract_symbol_from_transaction(
                active_transaction,
                &handle,
                &module_info,
                position,
                &params,
            )
        else {
            return Ok(None);
        };

        // Convert internal types to TSP types
        let synthesized_types = synthesized_internal_types
            .into_iter()
            .map(|t| self.convert_and_register_type(t))
            .collect();

        Ok(Some(tsp::Symbol {
            node: params.node,
            name: symbol_name,
            decls: declarations,
            synthesized_types,
        }))
    }
}
