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
use crate::module::module_info::ModuleInfo;
use crate::state::handle::Handle;
use crate::state::state::Transaction;
use crate::tsp;

/// Create an unresolved import declaration
///
/// This helper function creates a declaration marked as unresolved import
/// when import resolution fails.
pub fn create_unresolved_import_declaration(original_decl: &tsp::Declaration) -> tsp::Declaration {
    tsp::Declaration {
        handle: original_decl.handle.clone(),
        category: original_decl.category,
        flags: original_decl.flags.with_unresolved_import(),
        node: original_decl.node.clone(),
        module_name: original_decl.module_name.clone(),
        name: original_decl.name.clone(),
        uri: original_decl.uri.clone(),
    }
}

/// Resolve import target handle from source module
///
/// This function uses the transaction to resolve an import and get the target module handle.
pub fn resolve_import_target_handle(
    transaction: &Transaction<'_>,
    source_handle: &Handle,
    module_name: &tsp::ModuleName,
) -> Result<Handle, ()> {
    // Convert TSP ModuleName to pyrefly ModuleName
    let pyrefly_module_name = tsp::convert_tsp_module_name_to_pyrefly(module_name);

    // Use the transaction to resolve the import
    transaction
        .import_handle(source_handle, pyrefly_module_name, None)
        .map_err(|_| ())
}

/// Search for symbol definition in module content
///
/// This function searches for common Python definition patterns in module content
/// and returns the position if found.
pub fn find_symbol_definition_position(module_content: &str, import_name: &str) -> Option<usize> {
    // Look for function, class, or variable definitions of the imported name
    let patterns = [
        format!("def {import_name}("),   // Function definition
        format!("class {import_name}("), // Class definition
        format!("class {import_name}:"), // Class definition without inheritance
        format!("{import_name} ="),      // Variable assignment
    ];

    for pattern in &patterns {
        if let Some(pos) = module_content.find(pattern) {
            return Some(pos);
        }
    }
    None
}

/// Convert definition metadata to TSP declaration category and flags
///
/// This helper function maps pyrefly definition metadata to appropriate TSP
/// declaration categories and flags.
pub fn metadata_to_tsp_category_and_flags(
    metadata: &crate::state::lsp::DefinitionMetadata,
) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
    match metadata {
        crate::state::lsp::DefinitionMetadata::Variable(Some(symbol_kind)) => match symbol_kind {
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
        },
        _ => (
            tsp::DeclarationCategory::VARIABLE,
            tsp::DeclarationFlags::new(),
        ),
    }
}

/// Create a resolved declaration from definition information
///
/// This function creates a TSP Declaration from resolved definition information.
pub fn create_resolved_declaration_from_definition(
    definition: &crate::state::lsp::FindDefinitionItemWithDocstring,
    target_module_info: &ModuleInfo,
    import_name: &str,
    uri_converter: impl Fn(&ModuleInfo) -> Option<lsp_types::Url>,
    fallback_uri: lsp_types::Url,
) -> tsp::Declaration {
    let def_metadata = &definition.metadata;
    let def_range = definition.definition_range;
    let def_module = &definition.module;

    // Create a resolved declaration with proper category and flags
    let (category, flags) = metadata_to_tsp_category_and_flags(def_metadata);

    tsp::Declaration {
        handle: tsp::TypeHandle::String(format!(
            "resolved_{}_{}",
            def_module.name().as_str(),
            import_name
        )),
        category,
        flags,
        node: Some(tsp::Node {
            uri: uri_converter(def_module).unwrap_or_else(|| fallback_uri.clone()),
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
        name: import_name.to_owned(),
        uri: uri_converter(def_module).unwrap_or(fallback_uri),
    }
}

/// Create a fallback resolved declaration when specific symbol is not found
///
/// This function creates a generic resolved declaration pointing to the target module
/// when we can't find the specific symbol definition.
pub fn create_fallback_resolved_declaration(
    target_module_info: &ModuleInfo,
    import_name: &str,
    uri_converter: impl Fn(&ModuleInfo) -> Option<lsp_types::Url>,
    fallback_uri: lsp_types::Url,
) -> tsp::Declaration {
    tsp::Declaration {
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
        name: import_name.to_owned(),
        uri: uri_converter(target_module_info).unwrap_or(fallback_uri),
    }
}

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
            return Ok(Some(create_unresolved_import_declaration(&params.decl)));
        }

        // Check if workspace has language services enabled and get the source handle
        let Some(source_handle) = self.make_handle_if_enabled(importing_uri) else {
            return Ok(Some(create_unresolved_import_declaration(&params.decl)));
        };

        // Use standalone function to resolve import target handle
        let target_handle =
            match resolve_import_target_handle(transaction, &source_handle, module_name) {
                Ok(handle) => handle,
                Err(_) => {
                    // Import resolution failed, return unresolved import
                    return Ok(Some(create_unresolved_import_declaration(&params.decl)));
                }
            };

        // Try to get module info for the target module, loading it if necessary
        let (target_module_info, fresh_transaction) =
            match self.get_module_info_with_loading(transaction, &target_handle) {
                Ok((Some(info), fresh_tx)) => (info, fresh_tx),
                Ok((None, _)) => {
                    // Module not found, possibly an unresolved import
                    return Ok(Some(create_unresolved_import_declaration(&params.decl)));
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

        // Use standalone function to search for symbol definition
        let module_content = target_module_info.contents();
        if let Some(pos) = find_symbol_definition_position(module_content, import_name) {
            let text_pos = TextSize::new(pos as u32);
            if let Some(first_definition) = active_transaction
                .find_definition(&target_handle, text_pos, true)
                .into_iter()
                .next()
            {
                // Use standalone function to create resolved declaration
                return Ok(Some(create_resolved_declaration_from_definition(
                    &first_definition,
                    &target_module_info,
                    import_name,
                    |module_info| module_info_to_uri(module_info),
                    params.decl.uri.clone(),
                )));
            }
        }

        // Use standalone function to create fallback resolved declaration
        Ok(Some(create_fallback_resolved_declaration(
            &target_module_info,
            import_name,
            |module_info| module_info_to_uri(module_info),
            params.decl.uri.clone(),
        )))
    }
}
