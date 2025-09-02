/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::bounded;
use lsp_server::Connection;
use lsp_server::Message;
use lsp_server::Notification;
use lsp_server::Request;
use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::CodeActionKind;
use lsp_types::CodeActionOptions;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CompletionOptions;
use lsp_types::DidOpenTextDocumentParams;
use lsp_types::HoverProviderCapability;
use lsp_types::OneOf;
use lsp_types::PositionEncodingKind;
use lsp_types::RenameOptions;
use lsp_types::ServerCapabilities;
use lsp_types::SignatureHelpOptions;
use lsp_types::TextDocumentItem;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::TypeDefinitionProviderCapability;
use lsp_types::Url;
use lsp_types::WorkspaceFoldersServerCapabilities;
use lsp_types::WorkspaceServerCapabilities;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::notification::Exit;
use lsp_types::notification::Notification as _;
use pretty_assertions::assert_eq;

use crate::commands::lsp::IndexingMode;
use crate::commands::tsp::TspArgs;
use crate::commands::tsp::run_tsp;
// Re-export necessary types and functions for TSP tests
pub use crate::test::lsp::lsp_interaction::util::TestCase;
use crate::test::lsp::lsp_interaction::util::get_initialize_messages;
// Don't import get_initialize_responses from LSP - we'll define our own for TSP
use crate::test::util::init_test;

/// Simple TSP test runner that follows the same pattern as LSP tests but uses TSP server
pub fn run_test_tsp(test_case: TestCase) {
    init_test();
    let timeout = Duration::from_secs(25);
    let args = TspArgs {
        indexing_mode: test_case.indexing_mode,
        workspace_indexing_limit: test_case.workspace_indexing_limit,
    };

    let (language_client_sender, language_client_receiver) = bounded::<Message>(0);
    let (language_server_sender, language_server_receiver) = bounded::<Message>(0);
    let (server_response_received_sender, server_response_received_receiver) =
        bounded::<RequestId>(0);
    let (client_request_received_sender, client_request_received_receiver) =
        bounded::<RequestId>(0);

    let connection = Connection {
        sender: language_client_sender,
        receiver: language_server_receiver,
    };

    thread::scope(|scope| {
        // TSP server thread
        scope.spawn(move || {
            run_tsp(Arc::new(connection), args)
                .map(|_| ())
                .map_err(|e| std::io::Error::other(e.to_string()))
        });

        // Client sender thread
        scope.spawn(move || {
            let exit_message = Message::Notification(Notification {
                method: Exit::METHOD.to_owned(),
                params: serde_json::json!(null),
            });

            for msg in get_initialize_messages(&test_case.workspace_folders, test_case.configuration, test_case.file_watch)
                .into_iter()
                .chain(test_case.messages_from_language_client)
                .chain(std::iter::once(exit_message.clone()))
            {
                let stop_language_server = || {
                    language_server_sender.send_timeout(exit_message.clone(), timeout).unwrap();
                };

                let send = || {
                    eprintln!("client--->server {}", serde_json::to_string(&msg).unwrap());
                    if let Err(err) = language_server_sender.send_timeout(msg.clone(), timeout) {
                        panic!("Failed to send message to language server: {err:?}");
                    }
                };

                match &msg {
                    Message::Request(Request { id, .. }) => {
                        send();
                        if let Ok(response) = server_response_received_receiver.recv_timeout(timeout)
                            && response == id.clone()
                        {
                            // Continue to next message
                        } else {
                            stop_language_server();
                            panic!("Did not receive response for request {id:?}");
                        }
                    }
                    Message::Notification(_) => send(),
                    Message::Response(lsp_server::Response { id: response_id, .. }) => {
                        let request_id = client_request_received_receiver.recv_timeout(timeout).unwrap();
                        if request_id == response_id.clone() {
                            send();
                        } else {
                            stop_language_server();
                            panic!(
                                "language client received request {request_id}, expecting to send response for {response_id}"
                            );
                        }
                    }
                }
            }
        });

        // Server receiver thread
        scope.spawn(move || {
            let mut responses =
                get_tsp_initialize_responses(test_case.indexing_mode != IndexingMode::None)
                    .into_iter()
                    .chain(test_case.expected_messages_from_language_server)
                    .collect::<Vec<_>>();

            loop {
                if responses.is_empty() {
                    break;
                }
                match language_client_receiver.recv_timeout(timeout) {
                    Ok(msg) => {
                        eprintln!("client<---server {}", serde_json::to_string(&msg).unwrap());

                        match &msg {
                            Message::Response(Response { id, .. }) => {
                                let expected = responses.remove(0);
                                let assert = |expected_response: String, response: String| {
                                    if let Some(index) =
                                        expected_response.find("$$MATCH_EVERYTHING$$")
                                    {
                                        assert_eq!(
                                            response[..index].to_string(),
                                            expected_response[..index].to_string(),
                                            "Response mismatch"
                                        );
                                    } else {
                                        assert_eq!(
                                            response, expected_response,
                                            "Response mismatch"
                                        );
                                    }
                                };
                                assert(
                                    serde_json::to_string(&expected).unwrap(),
                                    serde_json::to_string(&msg).unwrap(),
                                );
                                server_response_received_sender.send(id.clone()).unwrap();
                            }
                            Message::Notification(notification) => {
                                eprintln!("Received notification: {notification:?}");
                            }
                            Message::Request(Request { id, .. }) => {
                                let expected = responses.remove(0);
                                let assert = |expected_response: String, response: String| {
                                    if let Some(index) =
                                        expected_response.find("$$MATCH_EVERYTHING$$")
                                    {
                                        assert_eq!(
                                            response[..index].to_string(),
                                            expected_response[..index].to_string(),
                                            "Response mismatch"
                                        );
                                    } else {
                                        assert_eq!(
                                            response, expected_response,
                                            "Response mismatch"
                                        );
                                    }
                                };
                                assert(
                                    serde_json::to_string(&expected).unwrap(),
                                    serde_json::to_string(&msg).unwrap(),
                                );
                                client_request_received_sender.send(id.clone()).unwrap();
                            }
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        panic!("Timeout waiting for response. Expected {responses:?}.");
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        panic!("Channel disconnected. Expected {responses:?}.");
                    }
                }
            }
        });
    });
}

pub fn build_did_open_notification(file_path: std::path::PathBuf) -> lsp_server::Notification {
    let content = std::fs::read_to_string(&file_path).unwrap();
    let uri = Url::from_file_path(file_path).unwrap();

    lsp_server::Notification {
        method: DidOpenTextDocument::METHOD.to_owned(),
        params: serde_json::to_value(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "python".to_owned(),
                version: 1,
                text: content,
            },
        })
        .unwrap(),
    }
}

/// TSP-specific initialize responses - same as LSP but without serverInfo
pub fn get_tsp_initialize_responses(find_refs: bool) -> Vec<Message> {
    vec![Message::Response(Response {
        id: RequestId::from(1),
        result: Some(serde_json::json!({"capabilities": &ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
            definition_provider: Some(OneOf::Left(true)),
            type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
            code_action_provider: Some(CodeActionProviderCapability::Options(
                CodeActionOptions {
                    code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                    ..Default::default()
                },
            )),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![".".to_owned()]),
                ..Default::default()
            }),
            document_highlight_provider: Some(OneOf::Left(true)),
            // Find references won't work properly if we don't know all the files.
            references_provider: if find_refs {
                Some(OneOf::Left(true))
            } else {
                None
            },
            rename_provider: if find_refs {
                Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                }))
            } else {
                None
            },
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_owned(), ",".to_owned()]),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                file_operations: None,
            }),
            semantic_tokens_provider: None,
            ..Default::default()
        }})),
        error: None,
    })]
}
