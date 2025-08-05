/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! TSP get type of declaration request implementation

use crate::lsp::server::Server;
use crate::state::state::Transaction;
use crate::tsp;
use crate::tsp::common::create_default_type_for_declaration;
use lsp_server::{ErrorCode, ResponseError};

impl Server {
    pub(crate) fn get_type_of_declaration(
        &self,
        transaction: &Transaction<'_>,
        params: tsp::GetTypeOfDeclarationParams,
    ) -> Result<tsp::Type, ResponseError> {
        // Check if the snapshot is still valid
        if params.snapshot != self.current_snapshot() {
            return Err(Self::snapshot_outdated_error());
        }

        // Extract the location information from the declaration
        let Some(node) = &params.decl.node else {
            // If there's no node information, we can't get the type
            return Err(ResponseError {
                code: ErrorCode::InvalidParams as i32,
                message: "Declaration has no node information".to_string(),
                data: None,
            });
        };

        // Convert Node URI to a handle
        let uri = &node.uri;

        // Check if workspace has language services enabled
        let Some(handle) = self.make_handle_if_enabled(uri) else {
            return Err(ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: "Language services disabled".to_string(),
                data: None,
            });
        };

        // Get module info for position conversion
        // Note: If the module is not loaded in the transaction, we'll load it ourselves
        // If we can't get module info, the file might not be loaded in the transaction
        // This can happen when the declaration points to a definition in a file that's not currently loaded
        let module_info = match transaction.get_module_info(&handle) {
            Some(info) => info,
            None => {
                // Module not loaded in transaction, try to load it
                let Some(mut fresh_transaction) = self.load_module_if_needed(transaction, &handle, crate::state::require::Require::Everything) else {
                    // If we still can't load the module, fall back to default type
                    return Ok(create_default_type_for_declaration(&params.decl));
                };
                
                // Get module info from the fresh transaction for position conversion
                let Some(fresh_module_info) = fresh_transaction.get_module_info(&handle) else {
                    return Ok(create_default_type_for_declaration(&params.decl));
                };
                
                // Convert declaration position to TextSize using fresh module_info
                let position = fresh_module_info.lined_buffer().from_lsp_position(node.range.start);
                
                // Try to get the type at the declaration's position using the fresh transaction
                let Some(type_info) = fresh_transaction.get_type_at(&handle, position) else {
                    // If we can't get type info from the position, try alternative approaches
                    
                    // For imports, we might need to resolve the imported symbol first
                    if params.decl.category == tsp::DeclarationCategory::IMPORT {
                        if let Ok(Some(import_type)) = self.get_type_for_import_declaration_with_fresh_transaction(&mut fresh_transaction, &params) {
                            return Ok(import_type);
                        }
                    }
                    
                    // If still no type found, create a generic type based on the declaration category
                    return Ok(create_default_type_for_declaration(&params.decl));
                };
                
                // Convert pyrefly Type to TSP Type format using the fresh transaction result
                return Ok(self.convert_and_register_type(type_info));
            }
        };

        // Convert declaration position to TextSize using module_info
        let position = module_info.lined_buffer().from_lsp_position(node.range.start);

        // Try to get the type at the declaration's position
        let Some(type_info) = transaction.get_type_at(&handle, position) else {
            // If we can't get type info from the position, try alternative approaches
            
            // For imports, we might need to resolve the imported symbol first
            if params.decl.category == tsp::DeclarationCategory::IMPORT {
                if let Ok(Some(import_type)) = self.get_type_for_import_declaration(transaction, &params) {
                    return Ok(import_type);
                }
            }
            
            // If still no type found, create a generic type based on the declaration category
            return Ok(create_default_type_for_declaration(&params.decl));
        };

        // Convert pyrefly Type to TSP Type format
        Ok(self.convert_and_register_type(type_info))
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
        
        if let Ok(Some(resolved_decl)) = self.resolve_import_declaration(transaction, resolve_params) {
            if let Some(resolved_node) = &resolved_decl.node {
                let resolved_uri = &resolved_node.uri;
                
                if let Some(resolved_handle) = self.make_handle_if_enabled(resolved_uri) {
                    // Get module info for the resolved declaration to convert position
                    if let Some(resolved_module_info) = transaction.get_module_info(&resolved_handle) {
                        let resolved_position = resolved_module_info.lined_buffer().from_lsp_position(resolved_node.range.start);
                        if let Some(resolved_type) = transaction.get_type_at(&resolved_handle, resolved_position) {
                            return Ok(Some(self.convert_and_register_type(resolved_type)));
                        }
                    }
                }
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
        
        if let Ok(Some(resolved_decl)) = self.resolve_import_declaration(fresh_transaction, resolve_params) {
            if let Some(resolved_node) = &resolved_decl.node {
                let resolved_uri = &resolved_node.uri;
                
                if let Some(resolved_handle) = self.make_handle_if_enabled(resolved_uri) {
                    // Make sure the resolved module is also loaded
                    fresh_transaction.run(&[(resolved_handle.clone(), crate::state::require::Require::Everything)]);
                    
                    // Get module info for the resolved declaration to convert position
                    if let Some(resolved_module_info) = fresh_transaction.get_module_info(&resolved_handle) {
                        let resolved_position = resolved_module_info.lined_buffer().from_lsp_position(resolved_node.range.start);
                        if let Some(resolved_type) = fresh_transaction.get_type_at(&resolved_handle, resolved_position) {
                            return Ok(Some(self.convert_and_register_type(resolved_type)));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
}
