/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP resolve import declaration request implementation

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use ruff_text_size::TextSize;

use crate::lsp::module_helpers::module_info_to_uri;
use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn resolve_import_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::ResolveImportDeclarationParams,
    ) -> Result<Option<tsp::Declaration>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Only resolve import declarations
        if params.decl.category != tsp::DeclarationCategory::IMPORT {
            // Return the same declaration if it's not an import
            return Ok(Some(params.decl));
        }

        // Parse the module name from the declaration
        let module_name = &params.decl.module_name;
        let import_name = &params.decl.name;

        // Convert source URI to file path (validation only)
        let importing_uri = &params.decl.uri;
        if importing_uri.to_file_path().is_err() {
            return Ok(Some(tsp::Declaration {
                handle: params.decl.handle,
                category: params.decl.category,
                flags: params.decl.flags.with_unresolved_import(),
                node: params.decl.node,
                module_name: params.decl.module_name,
                name: params.decl.name,
                uri: params.decl.uri.clone(),
            }));
        }

        // Check if workspace has language services enabled and get the source handle
        let Some(source_handle) = self.make_handle_if_enabled(importing_uri) else {
            return Ok(Some(tsp::Declaration {
                handle: params.decl.handle,
                category: params.decl.category,
                flags: params.decl.flags.with_unresolved_import(),
                node: params.decl.node,
                module_name: params.decl.module_name,
                name: params.decl.name,
                uri: params.decl.uri.clone(),
            }));
        };

        // Convert TSP ModuleName to pyrefly ModuleName
        let pyrefly_module_name = tsp::convert_tsp_module_name_to_pyrefly(module_name);

        // Use the transaction to resolve the import - same logic as resolve_import
        let target_handle =
            match transaction.import_handle(&source_handle, pyrefly_module_name, None) {
                Ok(resolved_handle) => resolved_handle,
                Err(_) => {
                    // Import resolution failed, return unresolved import
                    return Ok(Some(tsp::Declaration {
                        handle: params.decl.handle,
                        category: params.decl.category,
                        flags: params.decl.flags.with_unresolved_import(),
                        node: params.decl.node,
                        module_name: params.decl.module_name,
                        name: params.decl.name,
                        uri: params.decl.uri,
                    }));
                }
            };

        // Try to get module info for the target module, loading it if necessary
        let (target_module_info, fresh_transaction) =
            match self.get_module_info_with_loading(transaction, &target_handle) {
                Ok((Some(info), fresh_tx)) => (info, fresh_tx),
                Ok((None, _)) => {
                    // Module not found, possibly an unresolved import
                    return Ok(Some(tsp::Declaration {
                        handle: params.decl.handle,
                        category: params.decl.category,
                        flags: params.decl.flags.with_unresolved_import(),
                        node: params.decl.node,
                        module_name: params.decl.module_name,
                        name: params.decl.name,
                        uri: params.decl.uri,
                    }));
                }
                Err(_) => {
                    return Err(ResponseError {
                        code: ErrorCode::InternalError as i32,
                        message: "Failed to load target module".to_owned(),
                        data: None,
                    });
                }
            };

        // Use the appropriate transaction (fresh if module was loaded, original if already loaded)
        let active_transaction = fresh_transaction.as_ref().unwrap_or(transaction);

        // Look for the specific symbol in the target module
        // We'll use find_definition with a synthetic position to locate the symbol
        // This is a simplified approach - a full implementation would need to:
        // 1. Parse the target module's AST to find all exports
        // 2. Check __all__ if it exists
        // 3. Handle star imports properly
        // 4. Respect visibility rules (private vs public symbols)

        // Try to find all identifiers in the target module that match our import name
        // For simplicity, we'll search through the module content for the symbol definition
        let module_content = target_module_info.contents();

        // Look for function, class, or variable definitions of the imported name
        let patterns = [
            format!("def {}(", import_name),   // Function definition
            format!("class {}(", import_name), // Class definition
            format!("class {}:", import_name), // Class definition without inheritance
            format!("{} =", import_name),      // Variable assignment
        ];

        let mut found_position = None;
        for pattern in &patterns {
            if let Some(pos) = module_content.find(pattern) {
                found_position = Some(pos);
                break;
            }
        }

        // If we found the symbol, try to get its definition info
        if let Some(pos) = found_position {
            let text_pos = TextSize::new(pos as u32);
            if let Some(first_definition) = active_transaction
                .find_definition(&target_handle, text_pos, true)
                .into_iter()
                .next()
            {
                let def_metadata = &first_definition.metadata;
                let def_range = first_definition.definition_range;
                let def_module = &first_definition.module;

                // Create a resolved declaration with proper category and flags
                let (category, flags) = match &def_metadata {
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
                            _ => (
                                tsp::DeclarationCategory::VARIABLE,
                                tsp::DeclarationFlags::new(),
                            ),
                        }
                    }
                    _ => (
                        tsp::DeclarationCategory::VARIABLE,
                        tsp::DeclarationFlags::new(),
                    ),
                };

                return Ok(Some(tsp::Declaration {
                    handle: tsp::TypeHandle::String(format!(
                        "resolved_{}_{}",
                        def_module.name().as_str(),
                        import_name
                    )),
                    category,
                    flags,
                    node: Some(tsp::Node {
                        uri: module_info_to_uri(def_module)
                            .unwrap_or_else(|| params.decl.uri.clone()),
                        range: target_module_info.lined_buffer().to_lsp_range(def_range),
                    }),
                    module_name: tsp::ModuleName {
                        leading_dots: 0,
                        name_parts: def_module
                            .name()
                            .as_str()
                            .split('.')
                            .map(|s| s.to_owned())
                            .collect(),
                    },
                    name: import_name.clone(),
                    uri: module_info_to_uri(def_module).unwrap_or_else(|| params.decl.uri.clone()),
                }));
            }
        }

        // Fallback: create a generic resolved declaration pointing to the target module
        let resolved_declaration = tsp::Declaration {
            handle: tsp::TypeHandle::String(format!(
                "resolved_{}_{}",
                target_module_info.name().as_str(),
                import_name
            )),
            category: tsp::DeclarationCategory::VARIABLE, // Default to variable since we couldn't determine the type
            flags: tsp::DeclarationFlags::new(),
            node: None, // We don't have the exact location in the target module
            module_name: tsp::ModuleName {
                leading_dots: 0,
                name_parts: target_module_info
                    .name()
                    .as_str()
                    .split('.')
                    .map(|s| s.to_owned())
                    .collect(),
            },
            name: import_name.clone(),
            uri: module_info_to_uri(&target_module_info).unwrap_or_else(|| params.decl.uri.clone()), // Convert module info to URI
        };

        Ok(Some(resolved_declaration))
    }
}
