/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! CinderX type report.
//!
//! Produces a flat JSON report with per-expression structured type data,
//! intended for consumption by CinderX's static Python compiler.

#[allow(dead_code)]
pub mod collect;
#[allow(dead_code)]
pub mod convert;
#[allow(dead_code)]
pub mod types;

use std::path::Path;

use pyrefly_build::handle::Handle;
use pyrefly_util::fs_anyhow;
use serde::Serialize;

use crate::state::state::Transaction;

/// A module entry in the cinderx index.
#[derive(Debug, Serialize)]
struct ModuleEntry {
    /// Fully-qualified module name.
    module_name: String,
    /// Filesystem path to the source file.
    path: String,
}

/// Top-level index written to `index.json`.
#[derive(Debug, Serialize)]
struct CinderxIndex {
    /// Report format version.
    version: String,
    /// One entry per type-checked module.
    modules: Vec<ModuleEntry>,
}

/// Write a stub CinderX report to `output_dir`.
///
/// Currently writes only an `index.json` listing every module name and path.
/// Future commits will add per-module type tables.
pub fn write_results(
    output_dir: &Path,
    _transaction: &Transaction,
    handles: &[Handle],
) -> anyhow::Result<()> {
    fs_anyhow::create_dir_all(output_dir)?;

    let modules: Vec<ModuleEntry> = handles
        .iter()
        .map(|handle| ModuleEntry {
            module_name: handle.module().to_string(),
            path: handle.path().as_path().display().to_string(),
        })
        .collect();

    let index = CinderxIndex {
        version: "0.1".to_owned(),
        modules,
    };

    let json = serde_json::to_string_pretty(&index)?;
    fs_anyhow::write(&output_dir.join("index.json"), json)?;

    Ok(())
}
