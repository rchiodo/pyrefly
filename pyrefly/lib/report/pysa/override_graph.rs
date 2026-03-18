/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::collections::HashMap;

use pyrefly_build::handle::Handle;
use pyrefly_types::class::Class;
use pyrefly_util::thread_pool::ThreadPool;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use ruff_python_ast::name::Name;

use crate::report::pysa::class::ClassId;
use crate::report::pysa::context::ModuleContext;
use crate::report::pysa::function::FunctionRef;
use crate::report::pysa::function::get_all_functions;
use crate::report::pysa::module::ModuleIds;
use crate::report::pysa::module_index::WholeProgramPysaModuleIndex;
use crate::report::pysa::slow_fun_monitor::slow_fun_monitor_scope;
use crate::report::pysa::step_logger::StepLogger;
use crate::state::state::Transaction;

pub struct ModuleReversedOverrideGraph(HashMap<FunctionRef, FunctionRef>);

pub struct WholeProgramReversedOverrideGraph(dashmap::ReadOnlyView<FunctionRef, FunctionRef>);

impl WholeProgramReversedOverrideGraph {
    #[cfg(test)]
    pub fn new() -> WholeProgramReversedOverrideGraph {
        WholeProgramReversedOverrideGraph(dashmap::DashMap::new().into_read_only())
    }

    pub fn get<'a>(&'a self, method: &FunctionRef) -> Option<&'a FunctionRef> {
        self.0.get(method)
    }
}

/// Find the overridden base method for a class field by looking up the super
/// class member via ad_hoc_solve, then resolving the FunctionRef through the
/// PysaModuleIndex instead of creating a cross-module ModuleContext.
fn find_overridden_base_method(
    field_name: &Name,
    class: &Class,
    pysa_module_index: &WholeProgramPysaModuleIndex,
    context: &ModuleContext,
) -> Option<FunctionRef> {
    assert_eq!(class.module(), &context.module_info);

    let super_class_member = context
        .transaction
        .ad_hoc_solve(&context.handle, "override_super_class_member", |solver| {
            solver.get_super_class_member(class, None, field_name)
        })
        .flatten()?;

    // Look up the FunctionRef from the defining class's module index
    // instead of creating a cross-module ModuleContext.
    let defining_class = &super_class_member.defining_class;
    let module_id = context.module_ids.get_from_module(defining_class.module());
    let class_id = ClassId::from_class(defining_class);
    pysa_module_index
        .get_function_ref_for_class_field(module_id, class_id, field_name)
        .cloned()
}

pub fn create_reversed_override_graph_for_module(
    context: &ModuleContext,
    pysa_module_index: &WholeProgramPysaModuleIndex,
) -> ModuleReversedOverrideGraph {
    let mut graph = ModuleReversedOverrideGraph(HashMap::new());
    for function in get_all_functions(context) {
        if !function.should_export(context) {
            continue;
        }
        let name = function.name();
        let overridden_base_method = function.defining_cls().and_then(|class| {
            find_overridden_base_method(&name, class, pysa_module_index, context)
        });
        match overridden_base_method {
            Some(overridden_base_method) => {
                let current_function = function.as_function_ref(context);
                assert!(
                    graph
                        .0
                        .insert(current_function, overridden_base_method)
                        .is_none(),
                    "Found function definitions with the same location"
                );
            }
            _ => (),
        }
    }

    graph
}

pub fn build_reversed_override_graph(
    handles: &Vec<Handle>,
    transaction: &Transaction,
    module_ids: &ModuleIds,
    pysa_module_index: &WholeProgramPysaModuleIndex,
) -> WholeProgramReversedOverrideGraph {
    let step = StepLogger::start(
        "Building reverse override graph",
        "Built reverse override graph",
    );

    let reversed_override_graph = dashmap::DashMap::new();

    ThreadPool::new().install(|| {
        slow_fun_monitor_scope(|slow_function_monitor| {
            handles.par_iter().for_each(|handle| {
                let context = ModuleContext::create(handle.clone(), transaction, module_ids);
                slow_function_monitor.monitor_function(
                    || {
                        for (key, value) in
                            create_reversed_override_graph_for_module(&context, pysa_module_index).0
                        {
                            reversed_override_graph.insert(key, value);
                        }
                    },
                    format!(
                        "Building reverse override graph for `{}`",
                        handle.module().as_str(),
                    ),
                    /* max_time_in_seconds */ 4,
                );
            });
        })
    });

    step.finish();
    WholeProgramReversedOverrideGraph(reversed_override_graph.into_read_only())
}
