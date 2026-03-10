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
//!   mro.json            — global MRO table for all classes
//! ```

pub mod collect;
pub mod convert;
pub mod types;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use pyrefly_build::handle::Handle;
use pyrefly_types::class::Class;
use pyrefly_util::display::Fmt;
use pyrefly_util::fs_anyhow;
use serde::Serialize;

use crate::alt::types::class_metadata::ClassMro;
use crate::binding::binding::KeyClassMro;
use crate::report::cinderx::collect::collect_module_types;
use crate::report::cinderx::convert::canonicalize_class_qname;
use crate::report::cinderx::convert::qname_to_full_string;
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

/// One entry in the global MRO table.
#[derive(Debug, Serialize)]
struct MroEntry {
    /// Canonicalized qualified name of the class.
    qname: String,
    /// Ancestor classes in method resolution order (excludes `self` and `object`).
    ancestors: Vec<String>,
}

/// Global MRO table written to `mro.json`.
#[derive(Debug, Serialize)]
struct MroTable {
    /// One entry per unique class encountered in the type report.
    entries: Vec<MroEntry>,
}

/// Collect MRO entries for all classes, deduplicated by qname.
///
/// For each unique class, looks up its MRO via `KeyClassMro` from the
/// defining module's solutions. Classes whose defining module is not
/// available (e.g. third-party stubs not loaded) are silently skipped.
fn collect_mro_entries(classes: Vec<Class>, transaction: &Transaction) -> Vec<MroEntry> {
    // Build a handle lookup by module name for cross-module MRO resolution.
    let handle_by_module: HashMap<_, _> = transaction
        .handles()
        .into_iter()
        .map(|h| (h.module(), h))
        .collect();

    // Deduplicate classes by canonicalized qname, keeping the first seen.
    let mut seen_qnames: HashMap<String, Class> = HashMap::new();
    for cls in classes {
        let raw = format!("{}", Fmt(|f| cls.qname().fmt_with_module(f)));
        let qname = canonicalize_class_qname(&raw);
        seen_qnames.entry(qname).or_insert(cls);
    }

    let mut entries: Vec<MroEntry> = Vec::with_capacity(seen_qnames.len());
    for (qname, cls) in &seen_qnames {
        let Some(defining_handle) = handle_by_module.get(&cls.module_name()) else {
            continue;
        };
        let Some(solutions) = transaction.get_solutions(defining_handle) else {
            continue;
        };
        let mro: &Arc<ClassMro> = solutions.get(&KeyClassMro(cls.index()));
        let ancestors: Vec<String> = mro
            .ancestors_no_object()
            .iter()
            .map(|ancestor| {
                let raw = qname_to_full_string(ancestor.qname());
                canonicalize_class_qname(&raw)
            })
            .collect();
        entries.push(MroEntry {
            qname: qname.clone(),
            ancestors,
        });
    }

    // Sort by qname for deterministic output.
    entries.sort_by(|a, b| a.qname.cmp(&b.qname));
    entries
}

/// Write a CinderX type report to `output_dir`.
///
/// Writes:
/// - `index.json` listing every module
/// - `types/<module>.json` for each module with type table + located types
/// - `mro.json` with the global MRO table for all classes encountered
pub fn write_results(
    output_dir: &Path,
    transaction: &Transaction,
    handles: &[Handle],
) -> anyhow::Result<()> {
    fs_anyhow::create_dir_all(output_dir)?;
    let types_dir = output_dir.join("types");
    fs_anyhow::create_dir_all(&types_dir)?;

    let mut modules = Vec::with_capacity(handles.len());
    let mut all_classes: Vec<Class> = Vec::new();

    for handle in handles {
        let data = collect_module_types(transaction, handle).ok_or_else(|| {
            anyhow!(
                "missing module type data for {} ({})",
                handle.module(),
                handle.path().as_path().display()
            )
        })?;

        modules.push(ModuleEntry {
            module_name: handle.module().to_string(),
            path: handle.path().as_path().display().to_string(),
        });

        all_classes.extend(data.classes);

        let report = ModuleReport {
            type_table: data.entries,
            locations: data.locations,
        };
        let module_json = serde_json::to_string_pretty(&report)?;
        let filename = format!("{}.json", handle.module());
        fs_anyhow::write(&types_dir.join(filename), module_json)?;
    }

    let index = CinderxIndex {
        version: "0.1".to_owned(),
        modules,
    };
    let json = serde_json::to_string_pretty(&index)?;
    fs_anyhow::write(&output_dir.join("index.json"), json)?;

    // Write global MRO table.
    let mro_table = MroTable {
        entries: collect_mro_entries(all_classes, transaction),
    };
    let mro_json = serde_json::to_string_pretty(&mro_table)?;
    fs_anyhow::write(&output_dir.join("mro.json"), mro_json)?;

    Ok(())
}
