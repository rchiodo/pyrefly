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
//!
//! Output structure:
//! ```text
//! <output_dir>/
//!   index.json          — lists all modules with paths
//!   types/<module>.json  — per-module type table + located types
//! ```

pub mod collect;
pub mod convert;
pub mod types;

use std::path::Path;

use pyrefly_build::handle::Handle;
use pyrefly_util::fs_anyhow;
use serde::Serialize;

use crate::report::cinderx::collect::collect_module_types;
use crate::report::cinderx::types::LocatedType;
use crate::report::cinderx::types::TypeTableEntry;
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

/// Per-module type report written to `types/<module>.json`.
#[derive(Debug, Serialize)]
struct ModuleReport {
    /// Deduplicated type table; indices in `locations` refer into this.
    type_table: Vec<TypeTableEntry>,
    /// Per-expression type annotations keyed by source location.
    locations: Vec<LocatedType>,
}

/// Write a CinderX type report to `output_dir`.
///
/// Writes an `index.json` listing every module, plus a `types/<module>.json`
/// for each module containing the deduplicated type table and per-expression
/// located type references.
pub fn write_results(
    output_dir: &Path,
    transaction: &Transaction,
    handles: &[Handle],
) -> anyhow::Result<()> {
    fs_anyhow::create_dir_all(output_dir)?;
    let types_dir = output_dir.join("types");
    fs_anyhow::create_dir_all(&types_dir)?;

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

    for handle in handles {
        if let Some(data) = collect_module_types(transaction, handle) {
            let report = ModuleReport {
                type_table: data.entries,
                locations: data.locations,
            };
            let module_json = serde_json::to_string_pretty(&report)?;
            let filename = format!("{}.json", handle.module());
            fs_anyhow::write(&types_dir.join(filename), module_json)?;
        }
    }

    Ok(())
}
