/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashSet;
use std::sync::Arc;

use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::InitializeParams;

use crate::commands::lsp::IndexingMode;
use crate::lsp::queue::LspEvent;
use crate::lsp::queue::LspQueue;
use crate::lsp::server::ProcessEvent;
use crate::lsp::transaction_manager::TransactionManager;

/// Interface that defines the minimal set of operations needed by TSP server
/// from an LSP server implementation
pub trait LspServerInterface {
    /// Send a response back to the LSP client
    fn send_response(&self, response: Response);

    /// Process an LSP event and return the next step
    fn process_event<'a>(
        &'a self,
        ide_transaction_manager: &mut TransactionManager<'a>,
        canceled_requests: &mut HashSet<RequestId>,
        subsequent_mutation: bool,
        event: LspEvent,
    ) -> anyhow::Result<ProcessEvent>;
}

/// Factory for creating LSP server instances that implement LspServerInterface
pub struct LspServerFactory;

impl LspServerFactory {
    /// Create a new LSP server instance
    pub fn create(
        connection: Arc<lsp_server::Connection>,
        lsp_queue: LspQueue,
        initialization_params: InitializeParams,
        indexing_mode: IndexingMode,
        workspace_indexing_limit: usize,
    ) -> Box<dyn LspServerInterface> {
        Box::new(super::server::Server::new(
            connection,
            lsp_queue,
            initialization_params,
            indexing_mode,
            workspace_indexing_limit,
        ))
    }
}
