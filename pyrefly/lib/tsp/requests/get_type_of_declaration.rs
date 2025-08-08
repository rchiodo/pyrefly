/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get type of declaration request implementation

use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::module::module_info::ModuleInfo;
use crate::state::handle::Handle;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::create_default_type_for_declaration;

/// Extract type from declaration using transaction and module info
///
/// This is the core logic for getting the type of a declaration that can be
/// used independently of the Server implementation for unit testing.
pub fn extract_type_from_declaration(
    transaction: &Transaction<'_>,
    handle: &Handle,
    module_info: &ModuleInfo,
    params: &tsp::GetTypeOfDeclarationParams,
) -> Option<crate::types::types::Type> {
    // Extract the location information from the declaration
    let Some(node) = &params.decl.node else {
        return None;
    };

    // Convert declaration position to TextSize using module_info
    let position = module_info
        .lined_buffer()
        .from_lsp_position(node.range.start);

    // Try to get the type at the declaration's position
    transaction.get_type_at(handle, position)
}

/// Try to resolve import declaration and get its type
///
/// This helper function attempts to resolve an import declaration and
/// extract type information from the resolved symbol.
pub fn extract_type_from_resolved_import(
    transaction: &Transaction<'_>,
    resolved_decl: &tsp::Declaration,
    handle_factory: impl Fn(&lsp_types::Url) -> Option<Handle>,
) -> Option<crate::types::types::Type> {
    let Some(resolved_node) = &resolved_decl.node else {
        return None;
    };

    let resolved_uri = &resolved_node.uri;
    let Some(resolved_handle) = handle_factory(resolved_uri) else {
        return None;
    };

    // Get module info for the resolved declaration to convert position
    let Some(resolved_module_info) = transaction.get_module_info(&resolved_handle) else {
        return None;
    };

    let resolved_position = resolved_module_info
        .lined_buffer()
        .from_lsp_position(resolved_node.range.start);

    transaction.get_type_at(&resolved_handle, resolved_position)
}

impl Server {
    pub(crate) fn get_type_of_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeOfDeclarationParams,
    ) -> Result<tsp::Type, ResponseError> {
        // Extract the location information from the declaration
        let Some(node) = &params.decl.node else {
            return Err(ResponseError {
                code: lsp_server::ErrorCode::InvalidParams as i32,
                message: "Declaration has no node information".to_owned(),
                data: None,
            });
        };

        // Use common helper to validate, get handle, module info and maybe a fresh transaction
        let (handle, module_info, transaction_to_use) = self.with_active_transaction(
            transaction,
            &node.uri,
            params.snapshot,
            crate::state::require::Require::Everything,
        )?;

        // Try to extract type using standalone function against the appropriate transaction
        let active_transaction = transaction_to_use.as_ref().unwrap_or(transaction);
        if let Some(type_info) =
            extract_type_from_declaration(active_transaction, &handle, &module_info, &params)
        {
            return Ok(self.convert_and_register_type(type_info));
        }

        // For imports, we might need to resolve the imported symbol first
        if params.decl.category == tsp::DeclarationCategory::IMPORT {
            // First attempt with the active transaction
            if let Ok(Some(import_type)) =
                self.get_type_for_import_declaration(active_transaction, &params)
            {
                return Ok(import_type);
            }

            // If we don't yet have a fresh transaction, try resolving with one
            if transaction_to_use.is_none() {
                if let Some(mut fresh_transaction) = self.load_module_if_needed(
                    transaction,
                    &handle,
                    crate::state::require::Require::Everything,
                ) {
                    if let Ok(Some(import_type)) = self
                        .get_type_for_import_declaration_with_fresh_transaction(
                            &mut fresh_transaction,
                            &params,
                        )
                    {
                        return Ok(import_type);
                    }
                }
            }
        }

        // If still no type found, create a generic type based on the declaration category
        Ok(create_default_type_for_declaration(&params.decl))
    }

    /// Try to get type information for an import declaration by resolving the import
    pub(crate) fn get_type_for_import_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: &tsp::GetTypeOfDeclarationParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        let resolve_params = tsp::ResolveImportDeclarationParams {
            decl: params.decl.clone(),
            options: tsp::ResolveImportOptions::default(),
            snapshot: params.snapshot,
        };

        if let Ok(Some(resolved_decl)) =
            self.resolve_import_declaration(transaction, resolve_params)
        {
            // Use standalone function to extract type from resolved import
            if let Some(resolved_type) =
                extract_type_from_resolved_import(transaction, &resolved_decl, |uri| {
                    self.make_handle_if_enabled(uri)
                })
            {
                return Ok(Some(self.convert_and_register_type(resolved_type)));
            }
        }

        Ok(None)
    }

    /// Try to get type information for an import declaration using a fresh transaction
    pub(crate) fn get_type_for_import_declaration_with_fresh_transaction(
        &self,
        fresh_transaction: &mut Transaction,
        params: &tsp::GetTypeOfDeclarationParams,
    ) -> Result<Option<tsp::Type>, ResponseError> {
        let resolve_params = tsp::ResolveImportDeclarationParams {
            decl: params.decl.clone(),
            options: tsp::ResolveImportOptions::default(),
            snapshot: params.snapshot,
        };

        if let Ok(Some(resolved_decl)) =
            self.resolve_import_declaration(fresh_transaction, resolve_params)
        {
            // Make sure the resolved module is also loaded
            if let Some(resolved_node) = &resolved_decl.node {
                if let Some(resolved_handle) = self.make_handle_if_enabled(&resolved_node.uri) {
                    fresh_transaction.run(&[(
                        resolved_handle.clone(),
                        crate::state::require::Require::Everything,
                    )]);
                }
            }

            // Use standalone function to extract type from resolved import
            if let Some(resolved_type) =
                extract_type_from_resolved_import(fresh_transaction, &resolved_decl, |uri| {
                    self.make_handle_if_enabled(uri)
                })
            {
                return Ok(Some(self.convert_and_register_type(resolved_type)));
            }
        }

        Ok(None)
    }
}
