/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;

use lsp_server::RequestId;
use lsp_types::DocumentDiagnosticParams;
use lsp_types::InitializeParams;
use lsp_types::Registration;
use lsp_types::RegistrationParams;
use lsp_types::request::DocumentDiagnosticRequest;
use lsp_types::request::RegisterCapability;
use pyrefly_util::telemetry::QueueName;
use pyrefly_util::telemetry::Telemetry;
use pyrefly_util::telemetry::TelemetryEvent;
use pyrefly_util::telemetry::TelemetryEventKind;
use tracing::info;
use tsp_types::TSPNotificationMethods;
use tsp_types::TSPRequests;
use tsp_types::snapshot_outdated_error;

use crate::commands::lsp::IndexingMode;
use crate::lsp::non_wasm::lsp::new_response;
use crate::lsp::non_wasm::protocol::Message;
use crate::lsp::non_wasm::protocol::Notification;
use crate::lsp::non_wasm::protocol::Request;
use crate::lsp::non_wasm::protocol::Response;
use crate::lsp::non_wasm::queue::LspEvent;
use crate::lsp::non_wasm::server::InitializeInfo;
use crate::lsp::non_wasm::server::MessageReader;
use crate::lsp::non_wasm::server::ProcessEvent;
use crate::lsp::non_wasm::server::ServerCapabilitiesWithTypeHierarchy;
use crate::lsp::non_wasm::server::TspInterface;
use crate::lsp::non_wasm::server::capabilities;
use crate::lsp::non_wasm::transaction_manager::TransactionManager;

/// TSP server that delegates to LSP server infrastructure while handling only TSP requests
pub struct TspServer<T: TspInterface> {
    pub inner: T,
    /// Current snapshot version, updated on RecheckFinished events
    pub(crate) current_snapshot: Arc<Mutex<i32>>,
    /// Monotonic counter for outgoing request IDs (e.g. client/registerCapability)
    outgoing_request_id: AtomicI32,
}

impl<T: TspInterface> TspServer<T> {
    pub fn new(lsp_server: T) -> Self {
        Self {
            inner: lsp_server,
            current_snapshot: Arc::new(Mutex::new(0)), // Start at 0, increments on RecheckFinished
            outgoing_request_id: AtomicI32::new(1000), // Start high to avoid collisions with incoming IDs
        }
    }

    pub fn process_event<'a>(
        &'a self,
        ide_transaction_manager: &mut TransactionManager<'a>,
        canceled_requests: &mut HashSet<RequestId>,
        telemetry: &'a impl Telemetry,
        telemetry_event: &mut TelemetryEvent,
        subsequent_mutation: bool,
        event: LspEvent,
    ) -> anyhow::Result<ProcessEvent> {
        // Remember if this event should increment the snapshot after processing
        let should_increment_snapshot = match &event {
            LspEvent::RecheckFinished => {
                eprintln!("TSP event: RecheckFinished (will increment snapshot)");
                true
            }
            // Increment on DidChange since it affects type checker state via synchronous validation
            LspEvent::DidChangeTextDocument(_) => {
                eprintln!("TSP event: DidChangeTextDocument (will increment snapshot)");
                true
            }
            _ => {
                eprintln!("TSP event: {:?} (no snapshot increment)", event.describe());
                false
            }
        };

        // For TSP requests, handle them specially
        if let LspEvent::LspRequest(ref request) = event {
            if self.handle_tsp_request(ide_transaction_manager, request)? {
                return Ok(ProcessEvent::Continue);
            }
            // Handle textDocument/diagnostic (standard LSP pull diagnostics)
            if request.method == <DocumentDiagnosticRequest as lsp_types::request::Request>::METHOD {
                self.handle_document_diagnostic(request);
                return Ok(ProcessEvent::Continue);
            }
            // If it's not a TSP request or a supported LSP request, reject it
            self.inner.send_response(Response::new_err(
                request.id.clone(),
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("TSP server does not support LSP method: {}", request.method),
            ));
            return Ok(ProcessEvent::Continue);
        }

        // For all other events (notifications, responses, etc.), delegate to inner server
        let result = self.inner.process_event(
            ide_transaction_manager,
            canceled_requests,
            telemetry,
            telemetry_event,
            subsequent_mutation,
            event,
        )?;

        // Increment snapshot after the inner server has processed the event
        if should_increment_snapshot && let Ok(mut current) = self.current_snapshot.lock() {
            let old_snapshot = *current;
            *current += 1;
            let new_snapshot = *current;
            drop(current); // Release the lock before sending the notification
            self.send_snapshot_changed_notification(old_snapshot, new_snapshot);
        }

        Ok(result)
    }

    /// Validate that the caller's snapshot matches the current one.
    /// Returns `Err(snapshot_outdated_error())` when stale.
    pub fn validate_snapshot(&self, snapshot: i32) -> Result<(), lsp_server::ResponseError> {
        let current = self.get_snapshot();
        if snapshot != current {
            Err(snapshot_outdated_error())
        } else {
            Ok(())
        }
    }

    /// Send a snapshotChanged notification to the client.
    ///
    /// Called whenever the snapshot counter increments, so the client knows
    /// any previously-returned types are stale.
    fn send_snapshot_changed_notification(&self, old_snapshot: i32, new_snapshot: i32) {
        let method = serde_json::to_value(TSPNotificationMethods::TypeServerSnapshotChanged)
            .expect("TSPNotificationMethods serialization is infallible");
        let method_str = method
            .as_str()
            .expect("TSPNotificationMethods serializes to a string")
            .to_owned();

        let _ = self
            .inner
            .sender()
            .send(Message::Notification(Notification {
                method: method_str,
                params: serde_json::json!({ "old": old_snapshot, "new": new_snapshot }),
                activity_key: None,
            }));
    }

    fn handle_tsp_request<'a>(
        &'a self,
        _ide_transaction_manager: &mut TransactionManager<'a>,
        request: &Request,
    ) -> anyhow::Result<bool> {
        eprintln!("TSP handle_tsp_request: method={}, params={}", request.method, serde_json::to_string(&request.params).unwrap_or_default());
        // Convert the request into a TSPRequests enum
        let wrapper = serde_json::json!({
            "method": request.method,
            "id": request.id,
            "params": request.params
        });

        let Ok(msg) = serde_json::from_value::<TSPRequests>(wrapper) else {
            // Not a TSP request
            eprintln!("TSP: not a TSP request, skipping");
            return Ok(false);
        };

        match msg {
            TSPRequests::GetSupportedProtocolVersionRequest { .. } => {
                self.inner.send_response(new_response(
                    request.id.clone(),
                    Ok(self.get_supported_protocol_version()),
                ));
                Ok(true)
            }
            TSPRequests::GetSnapshotRequest { .. } => {
                // Get snapshot doesn't need a transaction since it just returns the cached value
                self.inner
                    .send_response(new_response(request.id.clone(), Ok(self.get_snapshot())));
                Ok(true)
            }
            TSPRequests::GetPythonSearchPathsRequest { params, .. } => {
                let response = match self.handle_get_python_search_paths(params) {
                    Ok(result) => Response::new_ok(request.id.clone(), result),
                    Err(e) => Response::new_err(request.id.clone(), e.code, e.message),
                };
                self.inner.send_response(response);
                Ok(true)
            }
            TSPRequests::ResolveImportRequest { params, .. } => {
                eprintln!("TSP: ResolveImport params={:?}", params);
                let response = match self.handle_resolve_import(params) {
                    Ok(result) => Response::new_ok(request.id.clone(), result),
                    Err(e) => Response::new_err(request.id.clone(), e.code, e.message),
                };
                self.inner.send_response(response);
                Ok(true)
            }
            TSPRequests::GetComputedTypeRequest { params, .. } => {
                eprintln!("TSP: GetComputedType params={}", serde_json::to_string(&params).unwrap_or_default());
                let response = match serde_json::from_value::<tsp_types::GetTypeParams>(params) {
                    Ok(typed_params) => {
                        eprintln!("TSP: GetComputedType uri={}, position=({}, {}), snapshot={}", typed_params.uri(), typed_params.position().line, typed_params.position().character, typed_params.snapshot);
                        match self.handle_get_computed_type(typed_params) {
                            Ok(result) => {
                                eprintln!("TSP: GetComputedType OK: {:?}", serde_json::to_string(&result).unwrap_or_default());
                                Response::new_ok(request.id.clone(), result)
                            }
                            Err(e) => {
                                eprintln!("TSP: GetComputedType handler error: {}", e.message);
                                Response::new_err(request.id.clone(), e.code, e.message)
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("TSP: GetComputedType params deserialization error: {}", e);
                        Response::new_err(
                            request.id.clone(),
                            lsp_server::ErrorCode::InvalidParams as i32,
                            format!("Invalid params for getComputedType: {e}"),
                        )
                    }
                };
                self.inner.send_response(response);
                Ok(true)
            }
            TSPRequests::GetDeclaredTypeRequest { params, .. } => {
                let response = match serde_json::from_value::<tsp_types::GetTypeParams>(params) {
                    Ok(typed_params) => match self.handle_get_declared_type(typed_params) {
                        Ok(result) => Response::new_ok(request.id.clone(), result),
                        Err(e) => Response::new_err(request.id.clone(), e.code, e.message),
                    },
                    Err(e) => Response::new_err(
                        request.id.clone(),
                        lsp_server::ErrorCode::InvalidParams as i32,
                        format!("Invalid params for getDeclaredType: {e}"),
                    ),
                };
                self.inner.send_response(response);
                Ok(true)
            }
            TSPRequests::GetExpectedTypeRequest { params, .. } => {
                let response = match serde_json::from_value::<tsp_types::GetTypeParams>(params) {
                    Ok(typed_params) => match self.handle_get_expected_type(typed_params) {
                        Ok(result) => Response::new_ok(request.id.clone(), result),
                        Err(e) => Response::new_err(request.id.clone(), e.code, e.message),
                    },
                    Err(e) => Response::new_err(
                        request.id.clone(),
                        lsp_server::ErrorCode::InvalidParams as i32,
                        format!("Invalid params for getExpectedType: {e}"),
                    ),
                };
                self.inner.send_response(response);
                Ok(true)
            }
        }
    }

    /// Handle a `textDocument/diagnostic` (pull diagnostics) request.
    fn handle_document_diagnostic(&self, request: &Request) {
        let response = match serde_json::from_value::<DocumentDiagnosticParams>(request.params.clone()) {
            Ok(params) => {
                let uri_str = params.text_document.uri.as_str();
                let report = self.inner.get_document_diagnostics(uri_str);
                match serde_json::to_value(report) {
                    Ok(value) => Response {
                        id: request.id.clone(),
                        result: Some(value),
                        error: None,
                    },
                    Err(e) => Response::new_err(
                        request.id.clone(),
                        lsp_server::ErrorCode::InternalError as i32,
                        format!("Failed to serialize diagnostics: {e}"),
                    ),
                }
            }
            Err(e) => Response::new_err(
                request.id.clone(),
                lsp_server::ErrorCode::InvalidParams as i32,
                format!("Invalid params for textDocument/diagnostic: {e}"),
            ),
        };
        self.inner.send_response(response);
    }

    /// Send `client/registerCapability` to register `textDocument/diagnostic`
    /// with the client so it uses the pull diagnostics model.
    fn register_document_diagnostics(&self) {
        let id = RequestId::from(self.outgoing_request_id.fetch_add(1, Ordering::SeqCst));
        let params = RegistrationParams {
            registrations: vec![Registration {
                id: "tsp-document-diagnostics".to_owned(),
                method: <DocumentDiagnosticRequest as lsp_types::request::Request>::METHOD.to_owned(),
                register_options: None,
            }],
        };
        let request = Request {
            id,
            method: <RegisterCapability as lsp_types::request::Request>::METHOD.to_owned(),
            params: serde_json::to_value(params).unwrap(),
            activity_key: None,
        };
        let _ = self.inner.sender().send(Message::Request(request));
    }
}

pub fn tsp_loop(
    lsp_server: impl TspInterface,
    mut reader: MessageReader,
    _initialization: InitializeInfo,
    telemetry: &impl Telemetry,
) -> anyhow::Result<()> {
    eprintln!("Reading TSP messages");
    let server = TspServer::new(lsp_server);

    // Register pull diagnostics capability with the client so it sends
    // textDocument/diagnostic requests instead of relying on push only.
    server.register_document_diagnostics();

    std::thread::scope(|scope| {
        // Start the recheck queue thread to process async tasks
        scope.spawn(|| server.inner.run_recheck_queue(telemetry));

        scope.spawn(|| {
            server.inner.dispatch_lsp_events(&mut reader);
        });

        let mut ide_transaction_manager = TransactionManager::default();
        let mut canceled_requests = HashSet::new();

        while let Ok((subsequent_mutation, event, enqueued_at)) = server.inner.lsp_queue().recv() {
            let (mut event_telemetry, queue_duration) = TelemetryEvent::new_dequeued(
                TelemetryEventKind::LspEvent(event.describe()),
                enqueued_at,
                server.inner.telemetry_state(),
                QueueName::LspQueue,
            );
            let event_description = event.describe();

            let result = server.process_event(
                &mut ide_transaction_manager,
                &mut canceled_requests,
                telemetry,
                &mut event_telemetry,
                subsequent_mutation,
                event,
            );
            let process_duration =
                event_telemetry.finish_and_record(telemetry, result.as_ref().err());
            match result? {
                ProcessEvent::Continue => {
                    info!(
                        "Type server processed event `{}` in {:.2}s ({:.2}s waiting)",
                        event_description,
                        process_duration.as_secs_f32(),
                        queue_duration.as_secs_f32()
                    );
                }
                ProcessEvent::Exit => break,
            }
        }

        server.inner.stop_recheck_queue();
        Ok(())
    })
}

/// Generate TSP-specific server capabilities using the same capabilities as LSP
pub fn tsp_capabilities(
    indexing_mode: IndexingMode,
    initialization_params: &InitializeParams,
) -> ServerCapabilitiesWithTypeHierarchy {
    // Use the same capabilities as LSP - TSP server supports the same features
    // but will only respond to TSP protocol requests
    capabilities(indexing_mode, initialization_params)
}
