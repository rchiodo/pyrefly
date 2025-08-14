/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use lsp_types::InitializeParams;
use lsp_types::ServerCapabilities;

use crate::commands::lsp::IndexingMode;

/// Generate TSP-specific server capabilities
/// 
/// This function creates server capabilities that are specific to the TSP protocol.
/// Unlike the LSP server, TSP focuses only on type-related operations and doesn't
/// need text editing capabilities like completion, hover, etc.
pub fn tsp_capabilities(
    _indexing_mode: IndexingMode,
    _initialization_params: &InitializeParams,
) -> ServerCapabilities {
    // TSP server capabilities - we don't support text document sync, completion, etc.
    // We only support custom TSP requests, so the capabilities are minimal
    ServerCapabilities {
        position_encoding: Some(lsp_types::PositionEncodingKind::UTF16),
        ..Default::default()
    }
}
