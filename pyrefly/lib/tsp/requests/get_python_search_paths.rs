/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the getPythonSearchPaths TSP request

use lsp_server::ResponseError;
use tsp_types::GetPythonSearchPathsParams;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;

impl<T: TspInterface> TspServer<T> {
    /// Return the Python search paths configured for the project that owns the
    /// given source URI.  This includes user-configured search paths, inferred
    /// import roots, and site-packages paths.
    pub fn handle_get_python_search_paths(
        &self,
        params: GetPythonSearchPathsParams,
    ) -> Result<Vec<String>, ResponseError> {
        self.validate_snapshot(params.snapshot)?;
        Ok(self.inner.get_python_search_paths(&params.from_uri))
    }
}
