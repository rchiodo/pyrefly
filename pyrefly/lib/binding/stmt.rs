/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use pyrefly_python::ast::Ast;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::short_identifier::ShortIdentifier;
use pyrefly_python::symbol_kind::SymbolKind;
use ruff_python_ast::AtomicNodeIndex;
use ruff_python_ast::Expr;
use ruff_python_ast::ExprCall;
use ruff_python_ast::ExprName;
use ruff_python_ast::Identifier;
use ruff_python_ast::Stmt;
use ruff_python_ast::StmtAssign;
use ruff_python_ast::StmtImportFrom;
use ruff_python_ast::StmtReturn;
use ruff_text_size::Ranged;
use ruff_text_size::TextRange;
use starlark_map::Hashed;

use crate::binding::binding::AnnotationTarget;
use crate::binding::binding::Binding;
use crate::binding::binding::BindingAnnotation;
use crate::binding::binding::BindingExpect;
use crate::binding::binding::ExprOrBinding;
use crate::binding::binding::Initialized;
use crate::binding::binding::IsAsync;
use crate::binding::binding::Key;
use crate::binding::binding::KeyAnnotation;
use crate::binding::binding::KeyExpect;
use crate::binding::binding::LinkedKey;
use crate::binding::binding::RaisedException;
use crate::binding::bindings::BindingsBuilder;
use crate::binding::bindings::MutableCaptureLookupKind;
use crate::binding::expr::Usage;
use crate::binding::narrow::NarrowOps;
use crate::binding::scope::FlowStyle;
use crate::binding::scope::LoopExit;
use crate::config::error_kind::ErrorKind;
use crate::error::context::ErrorInfo;
use crate::export::special::SpecialExport;
use crate::graph::index::Idx;
use crate::state::loader::FindError;
use crate::types::alias::resolve_typeshed_alias;
use crate::types::special_form::SpecialForm;
use crate::types::types::Type;

impl<'a> BindingsBuilder<'a> {
    fn bind_unimportable_names(&mut self, x: &StmtImportFrom, as_error: bool) {
        let any = if as_error {
            Type::any_error()
        } else {
            Type::any_explicit()
        };
        for x in &x.names {
            if &x.name != "*" {
                let asname = x.asname.as_ref().unwrap_or(&x.name);
                // We pass None as imported_from, since we are really faking up a local error definition
                self.bind_definition(asname, Binding::Type(any.clone()), FlowStyle::Other);
            }
        }
    }

    /// Bind a special assignment where we do not want the usage tracking or placeholder var pinning
    /// used for normal assignments.
    ///
    /// Used for legacy type variables and for `_Alias()` assignments in `typing` that
    /// we redirect to hard-coded alternative bindings.
    fn bind_legacy_type_var_or_typing_alias(
        &mut self,
        name: &ExprName,
        make_binding: impl FnOnce(Option<Idx<KeyAnnotation>>) -> Binding,
    ) {
        let assigned = self.declare_current_idx(Key::Definition(ShortIdentifier::expr_name(name)));
        // TODO(stroxler): It probably should be an error if the annotation is ever non-None
        let (ann, default) = self.bind_current(&name.id, &assigned, FlowStyle::Other);
        let mut binding = make_binding(ann);
        // TODO(stroxler): It probably should be an error if the default is ever non-None
        if let Some(default) = default {
            binding = Binding::Default(default, Box::new(binding));
        }
        self.insert_binding_current(assigned, binding);
    }

    fn assign_type_var(&mut self, name: &ExprName, call: &mut ExprCall) {
        // Type var declarations are static types only; skip them for first-usage type inference.
        let static_type_usage = &mut Usage::StaticTypeInformation;
        self.ensure_expr(&mut call.func, static_type_usage);
        let mut iargs = call.arguments.args.iter_mut();
        if let Some(expr) = iargs.next() {
            self.ensure_expr(expr, static_type_usage);
        }
        // The constraints (i.e., any positional arguments after the first)
        // and some keyword arguments are types.
        for arg in iargs {
            self.ensure_type(arg, &mut None);
        }
        for kw in call.arguments.keywords.iter_mut() {
            if let Some(id) = &kw.arg
                && (id.id == "bound" || id.id == "default")
            {
                self.ensure_type(&mut kw.value, &mut None);
            } else {
                self.ensure_expr(&mut kw.value, static_type_usage);
            }
        }
        self.bind_legacy_type_var_or_typing_alias(name, |ann| {
            Binding::TypeVar(
                ann,
                Ast::expr_name_identifier(name.clone()),
                Box::new(call.clone()),
            )
        })
    }

    fn ensure_type_var_tuple_and_param_spec_args(&mut self, call: &mut ExprCall) {
        // Type var declarations are static types only; skip them for first-usage type inference.
        let static_type_usage = &mut Usage::StaticTypeInformation;
        self.ensure_expr(&mut call.func, static_type_usage);
        for arg in call.arguments.args.iter_mut() {
            self.ensure_expr(arg, static_type_usage);
        }
        for kw in call.arguments.keywords.iter_mut() {
            if let Some(id) = &kw.arg
                && id.id == "default"
            {
                self.ensure_type(&mut kw.value, &mut None);
            } else {
                self.ensure_expr(&mut kw.value, static_type_usage);
            }
        }
    }

    fn assign_param_spec(&mut self, name: &ExprName, call: &mut ExprCall) {
        self.ensure_type_var_tuple_and_param_spec_args(call);
        self.bind_legacy_type_var_or_typing_alias(name, |ann| {
            Binding::ParamSpec(
                ann,
                Ast::expr_name_identifier(name.clone()),
                Box::new(call.clone()),
            )
        })
    }

    fn assign_type_var_tuple(&mut self, name: &ExprName, call: &mut ExprCall) {
        self.ensure_type_var_tuple_and_param_spec_args(call);
        self.bind_legacy_type_var_or_typing_alias(name, |ann| {
            Binding::TypeVarTuple(
                ann,
                Ast::expr_name_identifier(name.clone()),
                Box::new(call.clone()),
            )
        })
    }

    fn define_nonlocal_name(&mut self, name: &Identifier) {
        let key = Key::Definition(ShortIdentifier::new(name));
        let binding =
            match self.lookup_mutable_captured_name(&name.id, MutableCaptureLookupKind::Nonlocal) {
                Ok(found) => Binding::Forward(found),
                Err(error) => {
                    self.error(
                        name.range,
                        ErrorInfo::Kind(ErrorKind::UnknownName),
                        error.message(name),
                    );
                    Binding::Type(Type::any_error())
                }
            };
        let binding_key = self.insert_binding(key, binding);

        self.scopes
            .add_to_current_static(name.id.clone(), name.range, SymbolKind::Variable, None);

        self.bind_name(&name.id, binding_key, FlowStyle::Other);
    }

    fn define_global_name(&mut self, name: &Identifier) {
        let key = Key::Definition(ShortIdentifier::new(name));
        let binding =
            match self.lookup_mutable_captured_name(&name.id, MutableCaptureLookupKind::Global) {
                Ok(found) => Binding::Forward(found),
                Err(error) => {
                    self.error(
                        name.range,
                        ErrorInfo::Kind(ErrorKind::UnknownName),
                        error.message(name),
                    );
                    Binding::Type(Type::any_error())
                }
            };
        let binding_key = self.insert_binding(key, binding);

        self.scopes
            .add_to_current_static(name.id.clone(), name.range, SymbolKind::Variable, None);

        self.bind_name(&name.id, binding_key, FlowStyle::Other);
    }

    /// Bind the annotation in an `AnnAssign`
    pub fn bind_annotation(
        &mut self,
        name: &Identifier,
        annotation: &mut Expr,
        is_initialized: Initialized,
    ) -> Idx<KeyAnnotation> {
        let ann_key = KeyAnnotation::Annotation(ShortIdentifier::new(name));
        self.ensure_type(annotation, &mut None);
        let ann_val = if let Some(special) = SpecialForm::new(&name.id, annotation) {
            // Special case `_: SpecialForm` declarations (this mainly affects some names declared in `typing.pyi`)
            BindingAnnotation::Type(
                AnnotationTarget::Assign(name.id.clone(), Initialized::Yes),
                special.to_type(),
            )
        } else {
            BindingAnnotation::AnnotateExpr(
                if self.scopes.in_class_body() {
                    AnnotationTarget::ClassMember(name.id.clone())
                } else {
                    AnnotationTarget::Assign(name.id.clone(), is_initialized)
                },
                annotation.clone(),
                None,
            )
        };
        self.insert_binding(ann_key, ann_val)
    }

    /// Record a return statement for later analysis if we are in a function body, and mark
    /// that the flow has terminated.
    ///
    /// If this is the top level, report a type error about the invalid return
    /// and also create a binding to ensure we type check the expression.
    fn record_return(&mut self, mut x: StmtReturn) {
        let mut ret = self.declare_current_idx(Key::ReturnExplicit(x.range()));
        self.ensure_expr_opt(x.value.as_deref_mut(), ret.usage());
        if let Err((ret, oops_top_level)) = self.scopes.record_or_reject_return(ret, x) {
            match oops_top_level.value {
                Some(v) => self.insert_binding_current(ret, Binding::Expr(None, *v)),
                None => self.insert_binding_current(ret, Binding::Type(Type::None)),
            };
            self.error(
                oops_top_level.range,
                ErrorInfo::Kind(ErrorKind::BadReturn),
                "Invalid `return` outside of a function".to_owned(),
            );
        }
        self.scopes.mark_flow_termination();
    }

    /// Evaluate the statements and update the bindings.
    /// Every statement should end up in the bindings, perhaps with a location that is never used.
    pub fn stmt(&mut self, x: Stmt) {
        match x {
            Stmt::FunctionDef(x) => {
                self.function_def(x);
            }
            Stmt::ClassDef(x) => self.class_def(x),
            Stmt::Return(x) => {
                self.record_return(x);
            }
            Stmt::Delete(mut x) => {
                for target in &mut x.targets {
                    let mut delete_link = self.declare_current_idx(Key::UsageLink(target.range()));
                    if let Expr::Name(name) = target {
                        let idx = self.ensure_mutable_name(name, delete_link.usage());
                        self.scopes.upsert_flow_info(
                            Hashed::new(&name.id),
                            idx,
                            Some(FlowStyle::Uninitialized),
                        );
                    } else {
                        self.ensure_expr(target, delete_link.usage());
                    }
                    let delete_idx = self.insert_binding(
                        KeyExpect(target.range()),
                        BindingExpect::Delete(target.clone()),
                    );
                    self.insert_binding_current(
                        delete_link,
                        Binding::UsageLink(LinkedKey::Expect(delete_idx)),
                    );
                }
            }
            Stmt::Assign(ref x)
                if let [Expr::Name(name)] = x.targets.as_slice()
                    && let Some((module, forward)) =
                        resolve_typeshed_alias(self.module_info.name(), &name.id, &x.value) =>
            {
                // This hook is used to treat certain names defined in `typing.pyi` as `_Alias()`
                // assignments "as if" they were imports of the aliased name.
                //
                // For example, we treat `typing.List` as if it were an import of `builtins.list`.
                self.bind_legacy_type_var_or_typing_alias(name, |_| {
                    Binding::Import(module, forward, None)
                })
            }
            Stmt::Assign(mut x) => {
                if let [Expr::Name(name)] = x.targets.as_slice() {
                    if let Expr::Call(call) = &mut *x.value
                        && let Some(special) = self.as_special_export(&call.func)
                    {
                        match special {
                            SpecialExport::TypeVar => {
                                self.assign_type_var(name, call);
                                return;
                            }
                            SpecialExport::ParamSpec => {
                                self.assign_param_spec(name, call);
                                return;
                            }
                            SpecialExport::TypeVarTuple => {
                                self.assign_type_var_tuple(name, call);
                                return;
                            }
                            SpecialExport::Enum
                            | SpecialExport::IntEnum
                            | SpecialExport::StrEnum => {
                                if let Some((arg_name, members)) =
                                    call.arguments.args.split_first_mut()
                                {
                                    self.synthesize_enum_def(
                                        name,
                                        &mut call.func,
                                        arg_name,
                                        members,
                                    );
                                    return;
                                }
                            }
                            SpecialExport::TypedDict => {
                                if let Some((arg_name, members)) =
                                    call.arguments.args.split_first_mut()
                                {
                                    self.synthesize_typed_dict_def(
                                        name,
                                        &mut call.func,
                                        arg_name,
                                        members,
                                        &mut call.arguments.keywords,
                                    );
                                    return;
                                }
                            }
                            SpecialExport::TypingNamedTuple => {
                                if let Some((arg_name, members)) =
                                    call.arguments.args.split_first_mut()
                                {
                                    self.synthesize_typing_named_tuple_def(
                                        name,
                                        &mut call.func,
                                        arg_name,
                                        members,
                                    );
                                    return;
                                }
                            }
                            SpecialExport::CollectionsNamedTuple => {
                                if let Some((arg_name, members)) =
                                    call.arguments.args.split_first_mut()
                                {
                                    self.synthesize_collections_named_tuple_def(
                                        name,
                                        &mut call.func,
                                        arg_name,
                                        members,
                                        &mut call.arguments.keywords,
                                    );
                                    return;
                                }
                            }
                            SpecialExport::NewType => {
                                if let [new_type_name, base] = &mut *call.arguments.args {
                                    self.synthesize_typing_new_type(name, new_type_name, base);
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.bind_single_name_assign(
                        &Ast::expr_name_identifier(name.clone()),
                        x.value,
                        None,
                    );
                } else {
                    self.bind_targets_with_value(&mut x.targets, &mut x.value);
                }
            }
            Stmt::AnnAssign(mut x) => match *x.target {
                Expr::Name(name) => {
                    let name = Ast::expr_name_identifier(name);
                    // We have to handle the value carefully because the annotation, class field, and
                    // binding do not all treat `...` exactly the same:
                    // - an annotation key and a class field treat `...` as initializing, but only in stub files
                    // - we skip the `NameAssign` if we are in a stub and the value is `...`
                    let (value, maybe_ellipses) = if let Some(value) = x.value {
                        // Treat a name as initialized, but skip actually checking the value, if we are assigning `...` in a stub.
                        if self.module_info.path().is_interface()
                            && matches!(&*value, Expr::EllipsisLiteral(_))
                        {
                            (None, Some(*value))
                        } else {
                            (Some(value), None)
                        }
                    } else {
                        (None, None)
                    };
                    let ann_idx = self.bind_annotation(
                        &name,
                        &mut x.annotation,
                        match (&value, &maybe_ellipses) {
                            (None, None) => Initialized::No,
                            _ => Initialized::Yes,
                        },
                    );
                    let cannonical_ann_idx = match value {
                        Some(value) => self.bind_single_name_assign(
                            &name,
                            value,
                            Some((&x.annotation, ann_idx)),
                        ),
                        None => self.bind_definition(
                            &name,
                            Binding::AnnotatedType(
                                ann_idx,
                                Box::new(Binding::Type(Type::any_implicit())),
                            ),
                            if self.scopes.in_class_body() {
                                FlowStyle::ClassField {
                                    initial_value: maybe_ellipses,
                                }
                            } else {
                                FlowStyle::Uninitialized
                            },
                        ),
                    };
                    // This assignment gets checked with the provided annotation. But if there exists a prior
                    // annotation, we might be invalidating it unless the annotations are the same. Insert a
                    // check that in that case the annotations match.
                    if let Some(ann) = cannonical_ann_idx {
                        self.insert_binding(
                            KeyExpect(name.range),
                            BindingExpect::Redefinition {
                                new: ann_idx,
                                existing: ann,
                                name: name.id.clone(),
                            },
                        );
                    }
                }
                Expr::Attribute(attr) => {
                    let attr_name = attr.attr.id.clone();
                    self.ensure_type(&mut x.annotation, &mut None);
                    let ann_key = self.insert_binding(
                        KeyAnnotation::AttrAnnotation(x.annotation.range()),
                        BindingAnnotation::AnnotateExpr(
                            AnnotationTarget::ClassMember(attr_name.clone()),
                            *x.annotation,
                            None,
                        ),
                    );
                    let value = match x.value {
                        Some(mut assigned) => {
                            self.bind_attr_assign(attr.clone(), &mut assigned, |v, _| {
                                ExprOrBinding::Expr(v.clone())
                            })
                        }
                        _ => ExprOrBinding::Binding(Binding::Type(Type::any_implicit())),
                    };
                    if !self
                        .scopes
                        .record_self_attr_assign(&attr, value.clone(), Some(ann_key))
                    {
                        self.error(
                             x.range,
                             ErrorInfo::Kind(ErrorKind::BadAssignment),
                             format!(
                                "Type cannot be declared in assignment to non-self attribute `{}.{}`",
                                self.module_info.display(&attr.value),
                                attr_name,
                            ),

                         );
                    }
                }
                mut target => {
                    if matches!(&target, Expr::Subscript(..)) {
                        // Note that for Expr::Subscript Python won't fail at runtime,
                        // but Mypy and Pyright both error here, so let's do the same.
                        self.error(
                            x.annotation.range(),
                            ErrorInfo::Kind(ErrorKind::InvalidSyntax),
                            "Subscripts should not be annotated".to_owned(),
                        );
                    }
                    // Try and continue as much as we can, by throwing away the type or just binding to error
                    match x.value {
                        Some(value) => self.stmt(Stmt::Assign(StmtAssign {
                            node_index: AtomicNodeIndex::dummy(),
                            range: x.range,
                            targets: vec![target],
                            value,
                        })),
                        None => {
                            self.bind_target_no_expr(&mut target, &|_| {
                                Binding::Type(Type::any_error())
                            });
                        }
                    }
                }
            },
            Stmt::AugAssign(mut x) => {
                match x.target.as_ref() {
                    Expr::Name(name) => {
                        // TODO(stroxler): Is this really a good key for an augmented assignment?
                        // It works okay for type checking, but might have weird effects on the IDE.
                        let mut assigned = self
                            .declare_current_idx(Key::Definition(ShortIdentifier::expr_name(name)));
                        // Ensure the target name, which must already be in scope (it is part of the implicit dunder method call
                        // used in augmented assignment).
                        self.ensure_mutable_name(name, assigned.usage());
                        self.ensure_expr(&mut x.value, assigned.usage());
                        // TODO(stroxler): Should we really be using `bind_key` here? This will update the
                        // flow info to define the name, even if it was not previously defined.
                        let (ann, default) =
                            self.bind_current(&name.id, &assigned, FlowStyle::Other);
                        let mut binding = Binding::AugAssign(ann, x.clone());
                        if let Some(default) = default {
                            binding = Binding::Default(default, Box::new(binding));
                        }
                        self.insert_binding_current(assigned, binding);
                    }
                    Expr::Attribute(attr) => {
                        let mut x_cloned = x.clone();
                        self.bind_attr_assign(attr.clone(), &mut x.value, move |expr, ann| {
                            x_cloned.value = Box::new(expr.clone());
                            ExprOrBinding::Binding(Binding::AugAssign(ann, x_cloned))
                        });
                    }
                    Expr::Subscript(subscr) => {
                        let mut x_cloned = x.clone();
                        self.bind_subscript_assign(
                            subscr.clone(),
                            &mut x.value,
                            move |expr, ann| {
                                x_cloned.value = Box::new(expr.clone());
                                ExprOrBinding::Binding(Binding::AugAssign(ann, x_cloned))
                            },
                        );
                    }
                    illegal_target => {
                        // Most structurally invalid targets become errors in the parser, which we propagate so there
                        // is no need for duplicate errors. But we do want to catch unbound names (which the parser
                        // will not catch)
                        //
                        // We don't track first-usage in this context, since we won't analyze the usage anyway.
                        let mut e = illegal_target.clone();
                        self.ensure_expr(&mut e, &mut Usage::StaticTypeInformation);
                    }
                }
            }
            Stmt::TypeAlias(mut x) => {
                if let Expr::Name(name) = *x.name {
                    if let Some(params) = &mut x.type_params {
                        self.type_params(params);
                    }
                    self.ensure_type(&mut x.value, &mut None);
                    let binding = Binding::ScopedTypeAlias(
                        name.id.clone(),
                        x.type_params.map(|x| *x),
                        x.value,
                    );
                    self.bind_definition(
                        &Ast::expr_name_identifier(name),
                        binding,
                        FlowStyle::Other,
                    );
                } else {
                    self.error(
                        x.range,
                        ErrorInfo::Kind(ErrorKind::InvalidSyntax),
                        "Invalid assignment target".to_owned(),
                    );
                }
            }
            Stmt::For(mut x) => {
                self.bind_target_with_expr(&mut x.target, &mut x.iter, &|expr, ann| {
                    Binding::IterableValue(ann, expr.clone(), IsAsync::new(x.is_async))
                });
                // Note that we set up the loop *after* the header is fully bound, because the
                // loop iterator is only evaluated once before the loop begins.
                self.setup_loop(x.range, &NarrowOps::new());
                self.stmts(x.body);
                self.teardown_loop(x.range, &NarrowOps::new(), x.orelse);
            }
            Stmt::While(mut x) => {
                self.ensure_expr(&mut x.test, &mut Usage::Narrowing);
                let narrow_ops = NarrowOps::from_expr(self, Some(&x.test));
                self.setup_loop(x.range, &narrow_ops);
                // Note that is is important we ensure *after* we set up the loop, so that both the
                // narrowing and type checking are aware that the test might be impacted by changes
                // made in the loop (e.g. if we reassign the test variable).
                // Typecheck the test condition during solving.
                self.insert_binding(KeyExpect(x.test.range()), BindingExpect::Bool(*x.test));
                self.stmts(x.body);
                self.teardown_loop(x.range, &narrow_ops, x.orelse);
            }
            Stmt::If(x) => {
                let range = x.range;
                let mut exhaustive = false;
                let mut branches = Vec::new();
                // Type narrowing operations that are carried over from one branch to the next. For example, in:
                //   if x is None:
                //     pass
                //   else:
                //     pass
                // x is bound to Narrow(x, Is(None)) in the if branch, and the negation, Narrow(x, IsNot(None)),
                // is carried over to the else branch.
                let mut negated_prev_ops = NarrowOps::new();
                let mut implicit_else = true;
                for (range, mut test, body) in Ast::if_branches_owned(x) {
                    // If there is no test, it's an `else` clause and `this_branch_chosen` will be true.
                    let this_branch_chosen = match &test {
                        None => Some(true),
                        Some(x) => self.sys_info.evaluate_bool(x),
                    };
                    if this_branch_chosen == Some(false) {
                        continue; // We definitely won't pick this branch
                    }
                    self.bind_narrow_ops(&negated_prev_ops, range);
                    let mut base = self.scopes.clone_current_flow();
                    self.ensure_expr_opt(test.as_mut(), &mut Usage::Narrowing);
                    let new_narrow_ops = NarrowOps::from_expr(self, test.as_ref());
                    if let Some(test_expr) = test {
                        // Typecheck the test condition during solving.
                        self.insert_binding(
                            KeyExpect(test_expr.range()),
                            BindingExpect::Bool(test_expr),
                        );
                    } else {
                        implicit_else = false;
                    }
                    self.bind_narrow_ops(&new_narrow_ops, range);
                    negated_prev_ops.and_all(new_narrow_ops.negate());
                    self.stmts(body);
                    self.scopes.swap_current_flow_with(&mut base);
                    branches.push(base);
                    if this_branch_chosen == Some(true) {
                        exhaustive = true;
                        break; // We definitely picked this branch if we got here, nothing below is reachable.
                    }
                }
                // If the conditions are exhaustive, then we only need to merge the branches.
                //
                // Otherwise, we need to merge branches with `base` (which was
                // the flow above the `If`) because the if might be skipped
                // entirely.
                if exhaustive {
                    self.set_current_flow_to_merged_branches(branches, range);
                } else {
                    if implicit_else {
                        // If there is no explicit else branch, we still want to merge the negated ops
                        // from the previous branches into the flow env.
                        // Note, using a default use_range is OK. The range is only needed to make the
                        // key distinct from other keys.
                        self.bind_narrow_ops(&negated_prev_ops, TextRange::default());
                    }
                    self.merge_branches_into_current(branches, range);
                }
            }
            Stmt::With(x) => {
                let kind = IsAsync::new(x.is_async);
                for mut item in x.items {
                    let item_range = item.range();
                    let expr_range = item.context_expr.range();
                    let mut context = self.declare_current_idx(Key::ContextExpr(expr_range));
                    self.ensure_expr(&mut item.context_expr, context.usage());
                    let context_idx = self
                        .insert_binding_current(context, Binding::Expr(None, item.context_expr));
                    if let Some(mut opts) = item.optional_vars {
                        let make_binding =
                            |ann| Binding::ContextValue(ann, context_idx, expr_range, kind);
                        self.bind_target_no_expr(&mut opts, &make_binding);
                    } else {
                        self.insert_binding(
                            Key::Anon(item_range),
                            Binding::ContextValue(None, context_idx, expr_range, kind),
                        );
                    }
                }
                self.stmts(x.body);
            }
            Stmt::Match(x) => {
                self.stmt_match(x);
            }
            Stmt::Raise(x) => {
                if let Some(mut exc) = x.exc {
                    let mut current = self.declare_current_idx(Key::UsageLink(x.range));
                    self.ensure_expr(&mut exc, current.usage());
                    let raised = if let Some(mut cause) = x.cause {
                        self.ensure_expr(&mut cause, current.usage());
                        RaisedException::WithCause(Box::new((*exc, *cause)))
                    } else {
                        RaisedException::WithoutCause(*exc)
                    };
                    let idx = self.insert_binding(
                        KeyExpect(x.range),
                        BindingExpect::CheckRaisedException(raised),
                    );
                    self.insert_binding_current(
                        current,
                        Binding::UsageLink(LinkedKey::Expect(idx)),
                    );
                } else {
                    // If there's no exception raised, don't bother checking the cause.
                }
                self.scopes.mark_flow_termination();
            }
            Stmt::Try(x) => {
                let range = x.range;
                let mut branches = Vec::new();
                let mut base = self.scopes.clone_current_flow();

                // We branch before the body, conservatively assuming that any statement can fail
                // entry -> try -> else -> finally
                //   |                     ^
                //   ----> handler --------|

                self.stmts(x.body);
                self.stmts(x.orelse);
                self.scopes.swap_current_flow_with(&mut base);
                branches.push(base);

                for h in x.handlers {
                    base = self.scopes.clone_current_flow();
                    let range = h.range();
                    let h = h.except_handler().unwrap(); // Only one variant for now
                    if let Some(name) = h.name
                        && let Some(mut type_) = h.type_
                    {
                        let mut handler =
                            self.declare_current_idx(Key::Definition(ShortIdentifier::new(&name)));
                        self.ensure_expr(&mut type_, handler.usage());
                        self.bind_definition_current(
                            &name,
                            handler,
                            Binding::ExceptionHandler(type_, x.is_star),
                            FlowStyle::Other,
                        );
                    } else if let Some(mut type_) = h.type_ {
                        let mut handler = self.declare_current_idx(Key::Anon(range));
                        self.ensure_expr(&mut type_, handler.usage());
                        self.insert_binding_current(
                            handler,
                            Binding::ExceptionHandler(type_, x.is_star),
                        );
                    }
                    self.stmts(h.body);
                    self.scopes.swap_current_flow_with(&mut base);
                    branches.push(base);
                }

                self.set_current_flow_to_merged_branches(branches, range);
                self.stmts(x.finalbody);
            }
            Stmt::Assert(mut x) => {
                let assert_range = x.range();
                let test_range = x.test.range();
                self.ensure_expr(&mut x.test, &mut Usage::Narrowing);
                let narrow_ops = NarrowOps::from_expr(self, Some(&x.test));
                let static_test = self.sys_info.evaluate_bool(&x.test);
                self.insert_binding(Key::Anon(test_range), Binding::Expr(None, *x.test));
                if let Some(mut msg_expr) = x.msg {
                    let mut base = self.scopes.clone_current_flow();
                    // Negate the narrowing of the test expression when typechecking
                    // the error message, since we know the assertion was false
                    let negated_narrow_ops = narrow_ops.negate();
                    self.bind_narrow_ops(&negated_narrow_ops, msg_expr.range());
                    let mut msg = self.declare_current_idx(Key::UsageLink(msg_expr.range()));
                    self.ensure_expr(&mut msg_expr, msg.usage());
                    let idx = self.insert_binding(
                        KeyExpect(msg_expr.range()),
                        BindingExpect::TypeCheckExpr(*msg_expr),
                    );
                    self.insert_binding_current(msg, Binding::UsageLink(LinkedKey::Expect(idx)));
                    self.scopes.swap_current_flow_with(&mut base);
                };
                self.bind_narrow_ops(&narrow_ops, assert_range);
                if let Some(false) = static_test {
                    self.scopes.mark_flow_termination();
                }
            }
            Stmt::Import(x) => {
                for x in x.names {
                    let m = ModuleName::from_name(&x.name.id);
                    if let Err(err @ FindError::NotFound(..)) = self.lookup.get(m) {
                        let (ctx, msg) = err.display();
                        self.error_multiline(
                            x.range,
                            ErrorInfo::new(ErrorKind::ImportError, ctx.as_deref()),
                            msg,
                        );
                    }
                    match x.asname {
                        Some(asname) => {
                            self.bind_definition(
                                &asname,
                                Binding::Module(m, m.components(), None),
                                FlowStyle::ImportAs(m),
                            );
                        }
                        None => {
                            let first = m.first_component();
                            let flow_info = self.scopes.current().flow.info.get(&first);
                            let module_key = match flow_info {
                                Some(flow_info)
                                    if matches!(flow_info.style, FlowStyle::MergeableImport(_)) =>
                                {
                                    Some(flow_info.key)
                                }
                                _ => None,
                            };
                            let key = self.insert_binding(
                                Key::Import(first.clone(), x.name.range),
                                Binding::Module(m, vec![first.clone()], module_key),
                            );
                            self.bind_name(&first, key, FlowStyle::MergeableImport(m));
                        }
                    }
                }
            }
            Stmt::ImportFrom(x) => {
                if let Some(m) = self.module_info.name().new_maybe_relative(
                    self.module_info.path().is_init(),
                    x.level,
                    x.module.as_ref().map(|x| &x.id),
                ) {
                    match self.lookup.get(m) {
                        Ok(module_exports) => {
                            let exported = module_exports.exports(self.lookup);
                            for x in x.names {
                                if &x.name == "*" {
                                    for name in module_exports.wildcard(self.lookup).iter_hashed() {
                                        let key = Key::Import(name.into_key().clone(), x.range);
                                        let val = if exported.contains_key_hashed(name) {
                                            Binding::Import(m, name.into_key().clone(), None)
                                        } else {
                                            self.error(
                                                x.range,
                                                ErrorInfo::Kind(ErrorKind::MissingModuleAttribute),
                                                format!("Could not import `{name}` from `{m}`"),
                                            );
                                            Binding::Type(Type::any_error())
                                        };
                                        let key = self.insert_binding(key, val);
                                        self.bind_name(
                                            name.key(),
                                            key,
                                            FlowStyle::Import(m, name.into_key().clone()),
                                        );
                                    }
                                } else {
                                    let original_name_range = if x.asname.is_some() {
                                        Some(x.name.range)
                                    } else {
                                        None
                                    };
                                    let asname = x.asname.unwrap_or_else(|| x.name.clone());
                                    // A `from x import y` statement is ambiguous; if `x` is a package with
                                    // an `__init__.py` file, then it might import the name `y` from the
                                    // module `x` defined by the `__init__.py` file, or it might import a
                                    // submodule `x.y` of the package `x`.
                                    //
                                    // If both are present, generally we prefer the name defined in `x`,
                                    // but there is an exception: if we are already looking at the
                                    // `__init__` module of `x`, we always prefer the submodule.
                                    let val = if (self.module_info.name() != m)
                                        && exported.contains_key(&x.name.id)
                                    {
                                        Binding::Import(m, x.name.id.clone(), original_name_range)
                                    } else {
                                        let x_as_module_name = m.append(&x.name.id);
                                        match self.lookup.get(x_as_module_name) {
                                            Ok(_) => Binding::Module(
                                                x_as_module_name,
                                                x_as_module_name.components(),
                                                None,
                                            ),
                                            Err(FindError::Ignored) => {
                                                Binding::Type(Type::any_explicit())
                                            }
                                            _ => {
                                                self.error(
                                                    x.range,
                                                    ErrorInfo::Kind(
                                                        ErrorKind::MissingModuleAttribute,
                                                    ),
                                                    format!(
                                                        "Could not import `{}` from `{m}`",
                                                        x.name.id
                                                    ),
                                                );
                                                Binding::Type(Type::any_error())
                                            }
                                        }
                                    };
                                    self.bind_definition(
                                        &asname,
                                        val,
                                        FlowStyle::Import(m, x.name.id),
                                    );
                                }
                            }
                        }
                        Err(FindError::Ignored) => self.bind_unimportable_names(&x, false),
                        Err(
                            err @ (FindError::NoPyTyped
                            | FindError::NoSource(_)
                            | FindError::NotFound(..)),
                        ) => {
                            let (ctx, msg) = err.display();
                            self.error_multiline(
                                x.range,
                                ErrorInfo::new(ErrorKind::ImportError, ctx.as_deref()),
                                msg,
                            );
                            self.bind_unimportable_names(&x, true);
                        }
                    }
                } else {
                    self.error(
                        x.range,
                        ErrorInfo::Kind(ErrorKind::ImportError),
                        format!(
                            "Could not resolve relative import `{}`",
                            ".".repeat(x.level as usize)
                        ),
                    );
                    self.bind_unimportable_names(&x, true);
                }
            }
            Stmt::Global(x) => {
                for name in x.names {
                    self.define_global_name(&name);
                }
            }
            Stmt::Nonlocal(x) => {
                for name in x.names {
                    self.define_nonlocal_name(&name);
                }
            }
            Stmt::Expr(mut x) => {
                let mut current = self.declare_current_idx(Key::StmtExpr(x.value.range()));
                self.ensure_expr(&mut x.value, current.usage());
                self.insert_binding_current(current, Binding::Expr(None, *x.value));
            }
            Stmt::Pass(_) => { /* no-op */ }
            Stmt::Break(x) => {
                self.add_loop_exitpoint(LoopExit::Break, x.range);
            }
            Stmt::Continue(x) => {
                self.add_loop_exitpoint(LoopExit::Continue, x.range);
            }
            Stmt::IpyEscapeCommand(x) => self.error(
                x.range,
                ErrorInfo::Kind(ErrorKind::Unsupported),
                "IPython escapes are not supported".to_owned(),
            ),
        }
    }
}
