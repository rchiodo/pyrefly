/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Expression walker for the CinderX type report.
//!
//! Walks every expression in a module's AST, looks up the inferred type,
//! and builds a deduplicated `TypeTable` plus a list of `LocatedType` entries.

use std::sync::Arc;

use pyrefly_build::handle::Handle;
use pyrefly_python::short_identifier::ShortIdentifier;
use pyrefly_types::class::Class;
use pyrefly_types::types::Type;
use pyrefly_util::visit::Visit;
use ruff_python_ast::Expr;
use ruff_python_ast::ExprName;
use ruff_python_ast::ModModule;
use ruff_text_size::Ranged;

use crate::alt::answers::Answers;
use crate::binding::binding::Key;
use crate::binding::bindings::Bindings;
use crate::module::module_info::ModuleInfo;
use crate::report::cinderx::convert::type_to_structured;
use crate::report::cinderx::types::LocatedType;
use crate::report::cinderx::types::TypeTable;
use crate::report::cinderx::types::TypeTableEntry;
use crate::state::state::Transaction;

/// Collected per-module type data for the CinderX report.
pub(crate) struct ModuleTypeData {
    /// Deduplicated type table entries.
    pub entries: Vec<TypeTableEntry>,
    /// Per-expression located type references into the table.
    pub locations: Vec<LocatedType>,
}

/// Collect per-expression types for a single module.
///
/// Walks every expression in the AST, looks up the inferred type via
/// `Answers`, converts it to a `StructuredType` entry in a shared
/// `TypeTable`, and records a `LocatedType` mapping source location
/// to table index.
///
/// Returns `None` if any of the required data (AST, answers, bindings,
/// module info) is unavailable for the given handle.
pub(crate) fn collect_module_types(
    transaction: &Transaction,
    handle: &Handle,
) -> Option<ModuleTypeData> {
    let ast = transaction.get_ast(handle)?;
    let module_info = transaction.get_module_info(handle)?;
    let answers = transaction.get_answers(handle)?;
    let bindings = transaction.get_bindings(handle)?;

    let mut table = TypeTable::new();
    let mut locations = Vec::new();
    let mut pending_class_traits: Vec<(usize, Class)> = Vec::new();

    walk_expressions(
        &ast,
        &module_info,
        &answers,
        &bindings,
        &mut table,
        &mut locations,
        &mut pending_class_traits,
    );

    // TODO(stroxler): Post-process pending_class_traits to fill in trait
    // information on class entries. This requires solver access; for now
    // traits are left empty on ClassType entries.

    Some(ModuleTypeData {
        entries: table.into_entries(),
        locations,
    })
}

/// Walk all expressions in the AST and collect types.
fn walk_expressions(
    ast: &Arc<ModModule>,
    module_info: &ModuleInfo,
    answers: &Answers,
    bindings: &Bindings,
    table: &mut TypeTable,
    locations: &mut Vec<LocatedType>,
    pending_class_traits: &mut Vec<(usize, Class)>,
) {
    /// Try to find a binding key for a name expression.
    ///
    /// Checks `BoundName` (use/load site) first, then `Definition`
    /// (store site), returning `None` if neither exists in bindings.
    fn try_find_key_for_name(name: &ExprName, bindings: &Bindings) -> Option<Key> {
        let key = Key::BoundName(ShortIdentifier::expr_name(name));
        if bindings.is_valid_key(&key) {
            return Some(key);
        }
        let key = Key::Definition(ShortIdentifier::expr_name(name));
        if bindings.is_valid_key(&key) {
            return Some(key);
        }
        None
    }

    /// Look up the inferred type for an expression.
    ///
    /// For `Expr::Name` nodes, looks up via the binding key for precise
    /// results. For all other expressions, falls back to the trace map
    /// which is populated during solving.
    fn lookup_type(x: &Expr, answers: &Answers, bindings: &Bindings) -> Option<Type> {
        if let Expr::Name(name) = x
            && let Some(key) = try_find_key_for_name(name, bindings)
            && let Some(ty) = answers.get_type_at(bindings.key_to_idx(&key))
        {
            return Some(ty);
        }
        answers.get_type_trace(x.range())
    }

    /// Recursive expression visitor: looks up type, converts to structured
    /// form, records location, then recurses into child expressions.
    fn visit_expr(
        x: &Expr,
        parent: Option<&Expr>,
        module_info: &ModuleInfo,
        answers: &Answers,
        bindings: &Bindings,
        table: &mut TypeTable,
        locations: &mut Vec<LocatedType>,
        pending_class_traits: &mut Vec<(usize, Class)>,
    ) {
        if let Some(ty) = lookup_type(x, answers, bindings) {
            let range = x.range();
            let location = module_info
                .lined_buffer()
                .python_ast_range_for_expr(range, x, parent);
            let type_index = type_to_structured(&ty, table, pending_class_traits);
            locations.push(LocatedType {
                location,
                type_index,
            });
        }

        x.recurse(&mut |child| {
            visit_expr(
                child,
                Some(x),
                module_info,
                answers,
                bindings,
                table,
                locations,
                pending_class_traits,
            )
        });
    }

    ast.visit(&mut |x: &Expr| {
        visit_expr(
            x,
            None,
            module_info,
            answers,
            bindings,
            table,
            locations,
            pending_class_traits,
        )
    });
}
