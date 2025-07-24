/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

#![warn(clippy::all)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::module_inception)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::new_without_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::single_match)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::wrong_self_convention)]
#![deny(clippy::cloned_instead_of_copied)]
#![deny(clippy::derive_partial_eq_without_eq)]
#![deny(clippy::inefficient_to_string)]
#![deny(clippy::mem_replace_option_with_some)]
#![deny(clippy::str_to_string)]
#![deny(clippy::string_to_string)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![feature(box_patterns)]
#![feature(closure_lifetime_binder)]
#![feature(if_let_guard)]
#![feature(let_chains)]

mod alt;
mod binding;
mod buck;
#[cfg(not(target_arch = "wasm32"))]
mod commands;
mod compat;
mod error;
mod export;
mod graph;
#[cfg(not(target_arch = "wasm32"))]
mod lsp;
mod module;
pub mod playground;
pub mod query;
mod report;
mod solver;
mod state;
mod test;

pub(crate) use compat::*;

/// This interface is NOT stable and should not be relied upon.
/// It will change during minor version increments.
///
/// We name it `library` many times to make it longer than our real imports, and thus
/// to discourage Rust Analyzer from suggesting it for imports.
/// See https://github.com/rust-lang/rust-analyzer/issues/19689.
#[cfg(not(target_arch = "wasm32"))]
pub mod library {
    pub mod library {
        pub mod library {
            pub mod library {
                pub use crate::commands::all::Command;
                pub use crate::commands::check::CheckArgs;
                pub use crate::commands::check::FullCheckArgs;
                pub use crate::commands::config_finder::standard_config_finder;
                pub use crate::commands::util;
            }
        }
    }
}
