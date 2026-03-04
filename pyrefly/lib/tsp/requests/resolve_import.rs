/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the resolveImport TSP request

use lsp_server::ResponseError;
use tsp_types::ResolveImportParams;
use tsp_types::ResolveImportResponse;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;

impl<T: TspInterface> TspServer<T> {
    /// Resolve a Python import to the file URI of the module it refers to.
    ///
    /// Handles both absolute imports (`import os.path`) and relative imports
    /// (`from . import utils`).  The `source_uri` identifies the file that
    /// contains the import statement — it is needed to locate the correct
    /// config and to resolve relative imports.
    pub fn handle_resolve_import(
        &self,
        params: ResolveImportParams,
    ) -> Result<Option<ResolveImportResponse>, ResponseError> {
        self.validate_snapshot(params.snapshot)?;
        Ok(self.inner.resolve_import(
            &params.source_uri,
            params.module_descriptor.leading_dots,
            &params.module_descriptor.name_parts,
        ))
    }
}
