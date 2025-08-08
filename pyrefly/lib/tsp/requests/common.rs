/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Common utilities shared between TSP request implementations

use lsp_server::ErrorCode;
use lsp_server::ResponseError;
use ruff_text_size::TextSize;

use crate::lsp::server::Server;
use crate::module::module_info::ModuleInfo;
use crate::state::handle::Handle;
use crate::state::require::Require;
use crate::state::state::Transaction;
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
        symbol_kind: pyrefly_python::symbol_kind::SymbolKind,
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
        symbol_kind: pyrefly_python::symbol_kind::SymbolKind,
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

    /// Get module info from the current transaction or load it at the required level.
    /// Returns the ModuleInfo and, if a fresh transaction was needed, that Transaction.
    pub(crate) fn get_or_load_module_info(
        &self,
        transaction: &Transaction<'_>,
        handle: &Handle,
        required_level: Require,
    ) -> Result<(ModuleInfo, Option<Transaction>), ResponseError> {
        if let Some(info) = transaction.get_module_info(handle) {
            return Ok((info, None));
        }

        let Some(fresh_transaction) =
            self.load_module_if_needed(transaction, handle, required_level)
        else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Failed to load module".to_owned(),
                data: None,
            });
        };

        let Some(info) = fresh_transaction.get_module_info(handle) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Failed to get module info after loading".to_owned(),
                data: None,
            });
        };

        Ok((info, Some(fresh_transaction)))
    }

    /// Validate snapshot and language services, obtain Handle and ModuleInfo, and an optional
    /// fresh Transaction if the module needed loading. Encapsulates common handler boilerplate.
    pub(crate) fn with_active_transaction(
        &self,
        transaction: &Transaction<'_>,
        uri: &lsp_types::Url,
        snapshot: i32,
        required_level: Require,
    ) -> Result<(Handle, ModuleInfo, Option<Transaction>), ResponseError> {
        // Snapshot validation
        self.validate_snapshot(snapshot)?;

        // Language services validation
        self.validate_language_services(uri)?;

        // Create handle (after validation we expect it to be enabled)
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_owned(),
                data: None,
            });
        };

        // Get or load module info
        let (module_info, maybe_fresh_tx) =
            self.get_or_load_module_info(transaction, &handle, required_level)?;

        Ok((handle, module_info, maybe_fresh_tx))
    }
}

/// A builder to create TSP declarations consistently across handlers
pub struct DeclarationBuilder {
    handle: Option<tsp::TypeHandle>,
    category: tsp::DeclarationCategory,
    flags: tsp::DeclarationFlags,
    node: Option<tsp::Node>,
    module_name: tsp::ModuleName,
    name: String,
    uri: lsp_types::Url,
}

impl DeclarationBuilder {
    /// Create a builder for a symbol in the given module and URI
    pub fn new(name: impl Into<String>, module_name: tsp::ModuleName, uri: lsp_types::Url) -> Self {
        Self {
            handle: None,
            category: tsp::DeclarationCategory::VARIABLE,
            flags: tsp::DeclarationFlags::new(),
            node: None,
            module_name,
            name: name.into(),
            uri,
        }
    }

    /// Set a string handle
    pub fn handle_str(mut self, handle: impl Into<String>) -> Self {
        self.handle = Some(tsp::TypeHandle::String(handle.into()));
        self
    }

    /// Set the declaration category
    pub fn category(mut self, category: tsp::DeclarationCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the flags
    pub fn flags(mut self, flags: tsp::DeclarationFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set the declaration node directly
    pub fn node(mut self, node: tsp::Node) -> Self {
        self.node = Some(node);
        self
    }

    /// Build the declaration
    pub fn build(self) -> tsp::Declaration {
        tsp::Declaration {
            handle: self.handle.unwrap_or_else(|| {
                // default unique-ish handle from name and uri
                tsp::TypeHandle::String(format!(
                    "decl_{}_{}_{}",
                    self.name,
                    self.uri,
                    self.node
                        .as_ref()
                        .map(|n| format!(
                            "{}:{}",
                            n.range.start.line, n.range.start.character
                        ))
                        .unwrap_or_else(|| "no_node".to_owned())
                ))
            }),
            category: self.category,
            flags: self.flags,
            node: self.node,
            module_name: self.module_name,
            name: self.name,
            uri: self.uri,
        }
    }
}

/// Helper to compute the start position of a TSP node in the given module.
pub(crate) fn node_start_position(module_info: &ModuleInfo, node: &tsp::Node) -> TextSize {
    module_info
        .lined_buffer()
        .from_lsp_position(node.range.start)
}
