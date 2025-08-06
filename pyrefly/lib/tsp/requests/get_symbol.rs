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
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::lsp_debug;

impl Server {
    pub(crate) fn get_symbol(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetSymbolParams,
    ) -> Result<Option<tsp::Symbol>, ResponseError> {
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
                        message: "Failed to load module".to_string(),
                        data: None,
                    });
                };

                let Some(info) = fresh_transaction.get_module_info(&handle) else {
                    return Err(ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: "Failed to get module info after loading".to_string(),
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

        // First, check if we can get type information at this position
        let type_info = active_transaction.get_type_at(&handle, position);

        // Try to find definition at the position
        let (symbol_name, declarations, synthesized_types) = if let Some(first_definition) =
            active_transaction
                .find_definition(&handle, position, true)
                .into_iter()
                .next()
        {
            let definition_metadata = &first_definition.metadata;
            let definition_range = first_definition.definition_range;
            let definition_module = &first_definition.module;

            // Use provided name or extract from definition
            let name = params.name.unwrap_or_else(|| {
                // Try to extract symbol name from the source code at the position
                let start = position;
                let end = module_info
                    .lined_buffer()
                    .from_lsp_position(params.node.range.end);
                module_info.code_at(TextRange::new(start, end)).to_string()
            });

            // Create declarations from the definition
            let mut decls = Vec::new();

            // Generate a unique handle for this declaration
            let declaration_handle = tsp::TypeHandle::String(format!(
                "decl_{:p}_{}",
                &definition_metadata as *const _,
                u32::from(position)
            ));

            // Determine the category and flags based on definition metadata
            let (category, flags) = match &definition_metadata {
                crate::state::lsp::DefinitionMetadata::Variable(Some(symbol_kind)) => {
                    match symbol_kind {
                        pyrefly_python::symbol_kind::SymbolKind::Function => (
                            tsp::DeclarationCategory::FUNCTION,
                            tsp::DeclarationFlags::new(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::Class => (
                            tsp::DeclarationCategory::CLASS,
                            tsp::DeclarationFlags::new(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::Variable => (
                            tsp::DeclarationCategory::VARIABLE,
                            tsp::DeclarationFlags::new(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::Constant => (
                            tsp::DeclarationCategory::VARIABLE,
                            tsp::DeclarationFlags::new().with_constant(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::Parameter => (
                            tsp::DeclarationCategory::PARAM,
                            tsp::DeclarationFlags::new(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::TypeParameter => (
                            tsp::DeclarationCategory::TYPE_PARAM,
                            tsp::DeclarationFlags::new(),
                        ),
                        pyrefly_python::symbol_kind::SymbolKind::TypeAlias => (
                            tsp::DeclarationCategory::TYPE_ALIAS,
                            tsp::DeclarationFlags::new(),
                        ),
                        _ => (
                            tsp::DeclarationCategory::VARIABLE,
                            tsp::DeclarationFlags::new(),
                        ),
                    }
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
                crate::state::lsp::DefinitionMetadata::VariableOrAttribute(
                    _,
                    Some(symbol_kind),
                ) => match symbol_kind {
                    pyrefly_python::symbol_kind::SymbolKind::Function => (
                        tsp::DeclarationCategory::FUNCTION,
                        tsp::DeclarationFlags::new().with_class_member(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::Class => (
                        tsp::DeclarationCategory::CLASS,
                        tsp::DeclarationFlags::new(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::Variable => (
                        tsp::DeclarationCategory::VARIABLE,
                        tsp::DeclarationFlags::new().with_class_member(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::Constant => (
                        tsp::DeclarationCategory::VARIABLE,
                        tsp::DeclarationFlags::new()
                            .with_class_member()
                            .with_constant(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::Attribute => (
                        tsp::DeclarationCategory::VARIABLE,
                        tsp::DeclarationFlags::new().with_class_member(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::Parameter => (
                        tsp::DeclarationCategory::PARAM,
                        tsp::DeclarationFlags::new(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::TypeParameter => (
                        tsp::DeclarationCategory::TYPE_PARAM,
                        tsp::DeclarationFlags::new(),
                    ),
                    pyrefly_python::symbol_kind::SymbolKind::TypeAlias => (
                        tsp::DeclarationCategory::TYPE_ALIAS,
                        tsp::DeclarationFlags::new(),
                    ),
                    _ => (
                        tsp::DeclarationCategory::VARIABLE,
                        tsp::DeclarationFlags::new().with_class_member(),
                    ),
                },
                _ => (
                    tsp::DeclarationCategory::VARIABLE,
                    tsp::DeclarationFlags::new(),
                ),
            };

            // Extract module name from the definition's module (where the symbol is actually defined)
            let definition_module_name = definition_module.name();
            let module_parts: Vec<String> = definition_module_name
                .as_str()
                .split('.')
                .map(|s| s.to_string())
                .collect();
            let module_name = tsp::ModuleName {
                leading_dots: 0,
                name_parts: module_parts.clone(),
            };

            // Check if this is from builtins and update category/flags accordingly
            let (category, flags) = if module_parts
                .first()
                .map_or(false, |first| first == "builtins")
            {
                match category {
                    tsp::DeclarationCategory::FUNCTION
                    | tsp::DeclarationCategory::CLASS
                    | tsp::DeclarationCategory::VARIABLE => {
                        (tsp::DeclarationCategory::INTRINSIC, flags)
                    }
                    _ => (category, flags),
                }
            } else {
                (category, flags)
            };

            // Create node pointing to the actual definition location using the same logic as goto_definition
            let definition_uri = module_info_to_uri(definition_module);
            let definition_uri_final = definition_uri.unwrap_or_else(|| params.node.uri.clone());

            // For declarations in different modules, use the original input range instead of the definition range
            // because the definition range points to the target module, not the symbol in the current module
            // This applies to imports, import aliases, and any other cross-module references
            let (node_range, node_uri) = if definition_module.name() != module_info.name() {
                // Different module: use the original range in the current module
                (params.node.range, params.node.uri.clone())
            } else {
                // Same module: use the definition range and URI
                (
                    module_info.lined_buffer().to_lsp_range(definition_range),
                    definition_uri_final.clone(),
                )
            };

            // Create the declaration node
            let declaration_node = tsp::Node {
                uri: node_uri.clone(),
                range: node_range,
            };

            // Verify that the declaration node's text matches the expected name
            // For declarations in different modules, we validate using the current module since we're using the original range
            // For other declarations, we need to use the appropriate module
            let validation_module_info = if definition_module.name() != module_info.name() {
                // Different module: use the current module since we're using the original range
                module_info
            } else {
                // Same module: use the current module (which is the same as definition module)
                module_info
            };

            let declaration_text_range = validation_module_info
                .lined_buffer()
                .from_lsp_range(declaration_node.range);
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

            // Add the primary declaration
            decls.push(tsp::Declaration {
                handle: declaration_handle,
                category,
                flags,
                node: Some(declaration_node),
                module_name,
                name: name.clone(),
                uri: node_uri,
            });

            // Get synthesized types if available
            let mut synth_types = Vec::new();
            if let Some(type_info) = type_info {
                synth_types.push(self.convert_and_register_type(type_info));
            }

            (name, decls, synth_types)
        } else {
            // If no definition found, try to get type information at least
            let name = params.name.unwrap_or_else(|| {
                let start = position;
                let end = module_info
                    .lined_buffer()
                    .from_lsp_position(params.node.range.end);
                module_info.code_at(TextRange::new(start, end)).to_string()
            });

            let mut synth_types = Vec::new();
            if let Some(type_info) = type_info {
                synth_types.push(self.convert_and_register_type(type_info));
            } else {
                // No definition found and no type information available
                lsp_debug!(
                    "Warning: No symbol definition or type information found at position {} in {}",
                    position.to_usize(),
                    uri
                );
                return Ok(None);
            }

            (name, Vec::new(), synth_types)
        };

        Ok(Some(tsp::Symbol {
            node: params.node,
            name: symbol_name,
            decls: declarations,
            synthesized_types,
        }))
    }
}
