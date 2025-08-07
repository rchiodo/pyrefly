/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Common utilities shared between TSP request implementations

use lsp_server::ErrorCode;
use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::tsp;

impl Server {
    /// Common validation for snapshot
    pub(crate) fn validate_snapshot(&self, snapshot: i32) -> Result<(), ResponseError> {
        if snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }
        Ok(())
    }

    /// Common validation for language services enablement
    pub(crate) fn validate_language_services(
        &self,
        uri: &lsp_types::Url,
    ) -> Result<(), ResponseError> {
        if self.make_handle_if_enabled(uri).is_none() {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_owned(),
                data: None,
            });
        }
        Ok(())
    }

    /// Convert from pyrefly symbol kind to TSP declaration category and flags
    pub(crate) fn symbol_kind_to_tsp_category(
        symbol_kind: &pyrefly_python::symbol_kind::SymbolKind,
    ) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
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
            pyrefly_python::symbol_kind::SymbolKind::Attribute => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new().with_class_member(),
            ),
            _ => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new(),
            ),
        }
    }

    /// Convert from pyrefly symbol kind to TSP declaration category and flags with class member flag
    pub(crate) fn symbol_kind_to_tsp_category_with_class_member(
        symbol_kind: &pyrefly_python::symbol_kind::SymbolKind,
    ) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
        let (category, mut flags) = Self::symbol_kind_to_tsp_category(symbol_kind);

        // Add class member flag for appropriate categories
        match category {
            tsp::DeclarationCategory::FUNCTION | tsp::DeclarationCategory::VARIABLE => {
                flags = flags.with_class_member();
            }
            _ => {}
        }

        (category, flags)
    }

    /// Convert LSP SymbolKind to TSP DeclarationCategory and flags
    pub(crate) fn lsp_symbol_kind_to_tsp_category(
        kind: lsp_types::SymbolKind,
    ) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
        match kind {
            lsp_types::SymbolKind::FUNCTION | lsp_types::SymbolKind::METHOD => (
                tsp::DeclarationCategory::FUNCTION,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::CLASS => (
                tsp::DeclarationCategory::CLASS,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::VARIABLE | lsp_types::SymbolKind::FIELD => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::CONSTANT => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new().with_constant(),
            ),
            lsp_types::SymbolKind::MODULE | lsp_types::SymbolKind::NAMESPACE => (
                tsp::DeclarationCategory::IMPORT,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::CONSTRUCTOR => (
                tsp::DeclarationCategory::FUNCTION,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::PROPERTY => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new().with_class_member(),
            ),
            lsp_types::SymbolKind::ENUM | lsp_types::SymbolKind::ENUM_MEMBER => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new().with_constant(),
            ),
            lsp_types::SymbolKind::INTERFACE => (
                tsp::DeclarationCategory::CLASS,
                tsp::DeclarationFlags::new(),
            ),
            lsp_types::SymbolKind::TYPE_PARAMETER => (
                tsp::DeclarationCategory::TYPE_PARAM,
                tsp::DeclarationFlags::new(),
            ),
            // Default to variable for other kinds
            _ => (
                tsp::DeclarationCategory::VARIABLE,
                tsp::DeclarationFlags::new(),
            ),
        }
    }

    /// Create a TSP ModuleName from a module name string
    pub(crate) fn create_tsp_module_name(module_name_str: &str) -> tsp::ModuleName {
        let module_parts: Vec<String> = module_name_str.split('.').map(|s| s.to_owned()).collect();
        tsp::ModuleName {
            leading_dots: 0,
            name_parts: module_parts,
        }
    }

    /// Check if a module is from builtins and update category accordingly
    pub(crate) fn apply_builtins_category(
        category: tsp::DeclarationCategory,
        flags: tsp::DeclarationFlags,
        module_name: &tsp::ModuleName,
    ) -> (tsp::DeclarationCategory, tsp::DeclarationFlags) {
        if module_name
            .name_parts
            .first()
            .is_some_and(|first| first == "builtins")
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
        }
    }
}
