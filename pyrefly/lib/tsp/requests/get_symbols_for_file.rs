/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getSymbolsForFile TSP request

use lsp_server::{ErrorCode, ResponseError};
use pyrefly_python::module::Module;

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;

impl Server {
    pub(crate) fn get_symbols_for_file(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetSymbolsForFileParams,
    ) -> Result<Option<tsp::FileSymbolInfo>, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        let uri = &params.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_owned(),
                data: None,
            });
        };

        // Get module info - try to load it if not already loaded
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

        // Get all symbols from the file using the existing symbols() method
        let document_symbols = active_transaction.symbols(&handle).unwrap_or_default();
        
        // Convert from LSP DocumentSymbol to TSP Symbol
        let mut tsp_symbols = Vec::new();
        Self::convert_document_symbols_to_tsp(
            &document_symbols,
            &mut tsp_symbols,
            uri,
            &module_info,
            module_info.name().as_str(),
        );

        Ok(Some(tsp::FileSymbolInfo {
            uri: params.uri,
            symbols: tsp_symbols,
        }))
    }

    /// Convert LSP DocumentSymbols to TSP Symbols recursively
    fn convert_document_symbols_to_tsp(
        document_symbols: &[lsp_types::DocumentSymbol],
        tsp_symbols: &mut Vec<tsp::Symbol>,
        uri: &lsp_types::Url,
        module_info: &Module,
        module_name_str: &str,
    ) {
        for doc_symbol in document_symbols {
            // Create the TSP Symbol from DocumentSymbol
            let tsp_symbol = Self::document_symbol_to_tsp_symbol(doc_symbol, uri, module_name_str);
            tsp_symbols.push(tsp_symbol);

            // Recursively convert children if they exist
            if let Some(ref children) = doc_symbol.children {
                Self::convert_document_symbols_to_tsp(children, tsp_symbols, uri, module_info, module_name_str);
            }
        }
    }

    /// Convert a single DocumentSymbol to a TSP Symbol
    fn document_symbol_to_tsp_symbol(
        doc_symbol: &lsp_types::DocumentSymbol,
        uri: &lsp_types::Url,
        module_name_str: &str,
    ) -> tsp::Symbol {
        // Create a Node from the symbol's selection range
        let node = tsp::Node {
            uri: uri.clone(),
            range: doc_symbol.selection_range,
        };

        // Convert LSP SymbolKind to TSP DeclarationCategory
        let (category, flags) = Self::lsp_symbol_kind_to_tsp_category(doc_symbol.kind);

        // Create a unique handle for this declaration
        let declaration_handle = tsp::TypeHandle::String(format!(
            "symbol_{}_{}_{}",
            doc_symbol.name,
            doc_symbol.selection_range.start.line,
            doc_symbol.selection_range.start.character
        ));

        // Parse module name into ModuleName struct
        let module_parts: Vec<String> = module_name_str
            .split('.')
            .map(|s| s.to_owned())
            .collect();
        let module_name = tsp::ModuleName {
            leading_dots: 0,
            name_parts: module_parts,
        };

        // Create a declaration for this symbol
        let declaration = tsp::Declaration {
            handle: declaration_handle,
            category,
            flags,
            node: Some(node.clone()),
            module_name,
            name: doc_symbol.name.clone(),
            uri: uri.clone(),
        };

        tsp::Symbol {
            node,
            name: doc_symbol.name.clone(),
            decls: vec![declaration],
            synthesized_types: Vec::new(), // TODO: Could be enhanced to include type information
        }
    }

    /// Convert LSP SymbolKind to TSP DeclarationCategory and flags
    fn lsp_symbol_kind_to_tsp_category(
        kind: lsp_types::SymbolKind,
    ) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
        match kind {
            lsp_types::SymbolKind::FUNCTION | lsp_types::SymbolKind::METHOD => {
                (tsp::DeclarationCategory::FUNCTION, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::CLASS => {
                (tsp::DeclarationCategory::CLASS, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::VARIABLE | lsp_types::SymbolKind::FIELD => {
                (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::CONSTANT => {
                (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_constant())
            }
            lsp_types::SymbolKind::MODULE | lsp_types::SymbolKind::NAMESPACE => {
                (tsp::DeclarationCategory::IMPORT, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::CONSTRUCTOR => {
                (tsp::DeclarationCategory::FUNCTION, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::PROPERTY => {
                (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_class_member())
            }
            lsp_types::SymbolKind::ENUM | lsp_types::SymbolKind::ENUM_MEMBER => {
                (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new().with_constant())
            }
            lsp_types::SymbolKind::INTERFACE => {
                (tsp::DeclarationCategory::CLASS, tsp::DeclarationFlags::new())
            }
            lsp_types::SymbolKind::TYPE_PARAMETER => {
                (tsp::DeclarationCategory::TYPE_PARAM, tsp::DeclarationFlags::new())
            }
            // Default to variable for other kinds
            _ => (tsp::DeclarationCategory::VARIABLE, tsp::DeclarationFlags::new()),
        }
    }
}
