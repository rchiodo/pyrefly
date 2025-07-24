/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Common utilities and helper functions for TSP request handling

use lsp_server::ErrorCode;
use lsp_server::ResponseError;

use crate::lsp::server::Server;
use crate::tsp;
use crate::state::state::Transaction;

/// LSP debug logging that can be disabled in release builds
#[cfg(debug_assertions)]
macro_rules! lsp_debug {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}

#[cfg(not(debug_assertions))]
macro_rules! lsp_debug {
    ($($arg:tt)*) => {};
}

// Re-export the macro for use in TSP request modules
pub(crate) use lsp_debug;

/// Creates a snapshot outdated error
pub(crate) fn snapshot_outdated_error() -> ResponseError {
    ResponseError {
        code: ErrorCode::RequestFailed as i32,
        message: "Snapshot outdated".to_string(),
        data: None,
    }
}

/// Creates a common error response for internal errors
pub(crate) fn create_internal_error(message: &str) -> ResponseError {
    ResponseError {
        code: ErrorCode::InternalError as i32,
        message: message.to_string(),
        data: None,
    }
}

/// Creates a common error response for language services being disabled
pub(crate) fn language_services_disabled_error() -> ResponseError {
    ResponseError {
        code: ErrorCode::RequestFailed as i32,
        message: "Language services disabled".to_string(),
        data: None,
    }
}

/// Create a default type for a declaration when we can't determine the exact type
pub fn create_default_type_for_declaration(decl: &tsp::Declaration) -> tsp::Type {
    let (category, flags) = match decl.category {
        tsp::DeclarationCategory::FUNCTION => (tsp::TypeCategory::FUNCTION, tsp::TypeFlags::new().with_callable()),
        tsp::DeclarationCategory::CLASS => (tsp::TypeCategory::CLASS, tsp::TypeFlags::new().with_instantiable()),
        tsp::DeclarationCategory::IMPORT => (tsp::TypeCategory::MODULE, tsp::TypeFlags::new()),
        tsp::DeclarationCategory::TYPE_ALIAS => (tsp::TypeCategory::ANY, tsp::TypeFlags::new().with_from_alias()),
        tsp::DeclarationCategory::TYPE_PARAM => (tsp::TypeCategory::TYPE_VAR, tsp::TypeFlags::new()),
        _ => (tsp::TypeCategory::ANY, tsp::TypeFlags::new()),
    };

    tsp::Type {
        handle: decl.handle.clone(),
        category,
        flags,
        module_name: Some(decl.module_name.clone()),
        name: decl.name.clone(),
        category_flags: 0,
        decl: None,
    }
}
