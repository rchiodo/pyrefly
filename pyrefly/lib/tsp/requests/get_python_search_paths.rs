/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Implementation of the `typeServer/getPythonSearchPaths` TSP request.
//!
//! Returns the list of directories that pyrefly uses to resolve Python
//! imports for a given source file. The result includes user-configured
//! search paths, inferred import roots, and site-packages directories.

use lsp_server::RequestId;
use lsp_server::ResponseError;
use lsp_types::Url;
use tsp_types::protocol::GetPythonSearchPathsParams;

use crate::lsp::non_wasm::server::TspInterface;
use crate::tsp::server::TspServer;
use crate::tsp::validation::invalid_params_error;

impl<T: TspInterface> TspServer<T> {
    /// Handle a `typeServer/getPythonSearchPaths` request.
    ///
    /// Validates the snapshot, parses the `from_uri`, and delegates to
    /// [`TspInterface::get_python_search_paths`] to collect the ordered
    /// list of directories used for import resolution.
    pub fn handle_get_python_search_paths(
        &self,
        id: RequestId,
        params: GetPythonSearchPathsParams,
    ) {
        // --- 1. Validate snapshot ---
        if let Err(err) = self.validate_snapshot(params.snapshot) {
            self.send_err(id, err);
            return;
        }

        // --- 2. Parse from_uri and delegate ---
        match parse_from_uri(&params.from_uri) {
            Ok(url) => {
                let paths = self.inner.get_python_search_paths(&url);
                self.send_ok(id, paths);
            }
            Err(err) => {
                self.send_err(id, err);
            }
        }
    }
}

/// Parse and validate the `fromUri` parameter.
///
/// Accepts a URI string and returns a validated [`Url`] that must be a
/// `file://` scheme URI. Returns an `InvalidParams` error if the URI is
/// malformed or not a `file://` URI.
fn parse_from_uri(from_uri: &str) -> Result<Url, ResponseError> {
    let url =
        Url::parse(from_uri).map_err(|_| invalid_params_error("fromUri is not a valid URI"))?;
    // Ensure it's a file:// URI so we can resolve a filesystem path.
    if url.scheme() != "file" {
        return Err(invalid_params_error("fromUri must be a file:// URI"));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_from_uri unit tests ---

    #[test]
    fn test_valid_file_uri() {
        let url = parse_from_uri("file:///home/user/project/main.py").unwrap();
        assert_eq!(url.scheme(), "file");
    }

    #[test]
    fn test_valid_file_uri_windows_style() {
        let url = parse_from_uri("file:///C:/Users/test/project/main.py").unwrap();
        assert_eq!(url.scheme(), "file");
        // Should be convertible to a file path
        assert!(url.to_file_path().is_ok());
    }

    #[test]
    fn test_empty_uri_is_error() {
        let result = parse_from_uri("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("not a valid URI"));
    }

    #[test]
    fn test_relative_path_is_error() {
        // A bare path without a scheme is not a valid URI
        let result = parse_from_uri("some/path/main.py");
        assert!(result.is_err());
    }

    #[test]
    fn test_http_scheme_is_error() {
        let result = parse_from_uri("http://example.com/main.py");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("file://"));
    }

    #[test]
    fn test_https_scheme_is_error() {
        let result = parse_from_uri("https://example.com/main.py");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("file://"));
    }

    #[test]
    fn test_untitled_scheme_is_error() {
        let result = parse_from_uri("untitled:Untitled-1");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("file://"));
    }

    #[test]
    fn test_uri_with_spaces_encoded() {
        let url = parse_from_uri("file:///home/user/my%20project/main.py").unwrap();
        assert_eq!(url.scheme(), "file");
    }
}
