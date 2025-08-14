/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashSet;
use std::sync::Arc;

use dupe::Dupe;
use lsp_server::Connection;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_server::ResponseError;
use lsp_types::InitializeParams;
use lsp_types::ServerCapabilities;
use tsp_types::*;

use crate::commands::lsp::IndexingMode;
use crate::lsp::lsp::as_request;
use crate::lsp::lsp::new_response;
use crate::lsp::queue::LspEvent;
use crate::lsp::queue::LspQueue;
use crate::lsp::server::ProcessEvent;
use crate::lsp::server::Server;
use crate::lsp::server::capabilities;
use crate::lsp::server::dispatch_lsp_events;
use crate::lsp::transaction_manager::TransactionManager;

/// TSP server that delegates to LSP server infrastructure while handling only TSP requests
pub struct TspServer {
    pub inner: Server,
}

impl TspServer {
    pub fn new(
        connection: Arc<Connection>,
        lsp_queue: LspQueue,
        initialization_params: InitializeParams,
        indexing_mode: IndexingMode,
    ) -> Self {
        let inner = Server::new(connection, lsp_queue, initialization_params, indexing_mode);
        Self { inner }
    }

    pub fn process_event<'a>(
        &'a self,
        ide_transaction_manager: &mut TransactionManager<'a>,
        canceled_requests: &mut HashSet<RequestId>,
        subsequent_mutation: bool,
        event: LspEvent,
    ) -> anyhow::Result<ProcessEvent> {
        // For TSP requests, handle them specially
        if let LspEvent::LspRequest(ref request) = event {
            if self.handle_tsp_request(ide_transaction_manager, request)? {
                return Ok(ProcessEvent::Continue);
            }
            // If it's not a TSP request, let the LSP server reject it since TSP server shouldn't handle LSP requests
            self.inner.send_response(lsp_server::Response::new_err(
                request.id.clone(),
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("TSP server does not support LSP method: {}", request.method),
            ));
            return Ok(ProcessEvent::Continue);
        }

        // For all other events (notifications, responses, etc.), delegate to inner server
        self.inner.process_event(
            ide_transaction_manager,
            canceled_requests,
            subsequent_mutation,
            event,
        )
    }

    fn handle_tsp_request<'a>(
        &'a self,
        ide_transaction_manager: &mut TransactionManager<'a>,
        request: &Request,
    ) -> anyhow::Result<bool> {
        // Change the request into a TSPRequests enum
        let wrapper = serde_json::json!({
            "method": request.method,
            "id": request.id,           // RequestId implements Serialize
            "params": request.params
        });
        if let Ok(msg) = serde_json::from_value::<TSPRequests>(wrapper) {
            match msg {
                TSPRequests::GetPythonSearchPathsRequest(params) => {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response(
                    request.id.clone(),
                    Ok(self.get_python_search_paths(&transaction, params)),
                ));
                ide_transaction_manager.save(transaction);
                return Ok(true);
                }
            }
        }
        // Handle all TSP-specific requests
        if let Some(params) = as_request::<GetPythonSearchPathsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetPythonSearchPathsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response(
                    request.id.clone(),
                    Ok(self.get_python_search_paths(&transaction, params)),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetSnapshotRequest>(request) {
            match params {
                Ok(_) => {
                    self.inner.send_response(new_response(
                        request.id.clone(),
                        Ok(self.current_snapshot()),
                    ));
                }
                Err(_) => {
                    self.inner.send_response(new_response(
                        request.id.clone(),
                        Ok(self.current_snapshot()),
                    ));
                }
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetSupportedProtocolVersionRequest>(request) {
            match params {
                Ok(_) => {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                    self.inner.send_response(new_response(
                        request.id.clone(),
                        Ok(self.get_supported_protocol_version(&transaction)),
                    ));
                    ide_transaction_manager.save(transaction);
                }
                Err(_) => {
                    let transaction =
                        ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                    self.inner.send_response(new_response(
                        request.id.clone(),
                        Ok(self.get_supported_protocol_version(&transaction)),
                    ));
                    ide_transaction_manager.save(transaction);
                }
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetTypeRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetTypeRequest>(params, &request.id)
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_type(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetSymbolRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetSymbolRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_symbol(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<ResolveImportDeclarationRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<ResolveImportDeclarationRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.resolve_import_declaration(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetTypeOfDeclarationRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetTypeOfDeclarationRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_type_of_declaration(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetReprRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetReprRequest>(params, &request.id)
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_repr(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetDocstringRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetDocstringRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_docstring(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<SearchForTypeAttributeRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<SearchForTypeAttributeRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.search_for_type_attribute(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetFunctionPartsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetFunctionPartsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_function_parts(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetDiagnosticsVersionRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetDiagnosticsVersionRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_diagnostics_version(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<ResolveImportRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<ResolveImportRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.resolve_import(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetTypeArgsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetTypeArgsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_type_args(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetOverloadsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetOverloadsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_overloads(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetMatchingOverloadsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetMatchingOverloadsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_matching_overloads(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetDiagnosticsRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetDiagnosticsRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_diagnostics(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetBuiltinTypeRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetBuiltinTypeRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_builtin_type(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetTypeAttributesRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetTypeAttributesRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_type_attributes(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetSymbolsForFileRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetSymbolsForFileRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_symbols_for_file(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetMetaclassRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetMetaclassRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_metaclass(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<GetTypeAliasInfoRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<GetTypeAliasInfoRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.get_type_alias_info(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<CombineTypesRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<CombineTypesRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.combine_types(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        } else if let Some(params) = as_request::<CreateInstanceTypeRequest>(request) {
            if let Some(params) = self
                .inner
                .extract_request_params_or_send_err_response::<CreateInstanceTypeRequest>(
                    params,
                    &request.id,
                )
            {
                let transaction =
                    ide_transaction_manager.non_commitable_transaction(&self.inner.state);
                self.inner.send_response(new_response_with_error_code(
                    request.id.clone(),
                    self.create_instance_type(&transaction, params),
                ));
                ide_transaction_manager.save(transaction);
            }
            return Ok(true);
        }

        // Not a TSP request
        Ok(false)
    }
}

pub fn tsp_loop(
    connection: Arc<Connection>,
    initialization_params: InitializeParams,
    indexing_mode: IndexingMode,
) -> anyhow::Result<()> {
    eprintln!("Reading TSP messages");
    let connection_for_dispatcher = connection.dupe();
    let lsp_queue = LspQueue::new();

    let server = TspServer::new(
        connection,
        lsp_queue.dupe(),
        initialization_params,
        indexing_mode,
    );

    let lsp_queue2 = lsp_queue.dupe();
    std::thread::spawn(move || {
        dispatch_lsp_events(&connection_for_dispatcher, lsp_queue2);
    });

    let mut ide_transaction_manager = TransactionManager::default();
    let mut canceled_requests = HashSet::new();

    while let Ok((subsequent_mutation, event)) = lsp_queue.recv() {
        match server.process_event(
            &mut ide_transaction_manager,
            &mut canceled_requests,
            subsequent_mutation,
            event,
        )? {
            ProcessEvent::Continue => {}
            ProcessEvent::Exit => break,
        }
    }

    Ok(())
}

/// Generate TSP-specific server capabilities using the same capabilities as LSP
pub fn tsp_capabilities(
    indexing_mode: IndexingMode,
    initialization_params: &InitializeParams,
) -> ServerCapabilities {
    // Use the same capabilities as LSP - TSP server supports the same features
    // but will only respond to TSP protocol requests
    capabilities(indexing_mode, initialization_params)
}

pub fn new_response_with_error_code<T>(id: RequestId, params: Result<T, ResponseError>) -> Response
where
    T: serde::Serialize,
{
    match params {
        Ok(params) => Response {
            id,
            result: Some(serde_json::to_value(params).unwrap()),
            error: None,
        },
        Err(error) => Response {
            id,
            result: None,
            error: Some(error),
        },
    }
}
