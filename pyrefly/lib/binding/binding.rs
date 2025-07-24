/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hash;

use dupe::Dupe;
use itertools::Either;
use pyrefly_derive::TypeEq;
use pyrefly_derive::VisitMut;
use pyrefly_python::dunder;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::short_identifier::ShortIdentifier;
use pyrefly_python::symbol_kind::SymbolKind;
use pyrefly_util::assert_bytes;
use pyrefly_util::assert_words;
use pyrefly_util::display::DisplayWith;
use pyrefly_util::display::DisplayWithCtx;
use pyrefly_util::display::commas_iter;
use pyrefly_util::display::intersperse_iter;
use pyrefly_util::uniques::Unique;
use pyrefly_util::visit::VisitMut;
use ruff_python_ast::Expr;
use ruff_python_ast::ExprAttribute;
use ruff_python_ast::ExprCall;
use ruff_python_ast::ExprSubscript;
use ruff_python_ast::ExprYield;
use ruff_python_ast::ExprYieldFrom;
use ruff_python_ast::Identifier;
use ruff_python_ast::StmtAugAssign;
use ruff_python_ast::StmtClassDef;
use ruff_python_ast::StmtFunctionDef;
use ruff_python_ast::TypeParams;
use ruff_python_ast::name::Name;
use ruff_text_size::Ranged;
use ruff_text_size::TextRange;
use starlark_map::small_map::SmallMap;
use starlark_map::small_set::SmallSet;

use crate::alt::class::base_class::BaseClass;
use crate::alt::class::class_field::ClassField;
use crate::alt::class::variance_inference::VarianceMap;
use crate::alt::solve::TypeFormContext;
use crate::alt::types::class_metadata::ClassMetadata;
use crate::alt::types::class_metadata::ClassMro;
use crate::alt::types::class_metadata::ClassSynthesizedFields;
use crate::alt::types::decorated_function::DecoratedFunction;
use crate::alt::types::legacy_lookup::LegacyTypeParameterLookup;
use crate::alt::types::yields::YieldFromResult;
use crate::alt::types::yields::YieldResult;
use crate::binding::bindings::Bindings;
use crate::binding::narrow::NarrowOp;
use crate::graph::index::Idx;
use crate::module::module_info::ModuleInfo;
use crate::types::annotation::Annotation;
use crate::types::class::Class;
use crate::types::class::ClassDefIndex;
use crate::types::class::ClassFieldProperties;
use crate::types::equality::TypeEq;
use crate::types::globals::Global;
use crate::types::quantified::QuantifiedKind;
use crate::types::stdlib::Stdlib;
use crate::types::tuple::Tuple;
use crate::types::type_info::TypeInfo;
use crate::types::types::TParams;
use crate::types::types::Type;
use crate::types::types::Var;

assert_words!(Key, 5);
assert_words!(KeyExpect, 1);
assert_words!(KeyExport, 3);
assert_words!(KeyClass, 1);
assert_bytes!(KeyTParams, 4);
assert_words!(KeyClassField, 4);
assert_bytes!(KeyClassSynthesizedFields, 4);
assert_bytes!(KeyAnnotation, 12);
assert_bytes!(KeyClassMetadata, 4);
assert_bytes!(KeyClassMro, 4);
assert_words!(KeyLegacyTypeParam, 1);
assert_words!(KeyYield, 1);
assert_words!(KeyYieldFrom, 1);
assert_words!(KeyFunction, 1);

assert_words!(Binding, 11);
assert_words!(BindingExpect, 11);
assert_words!(BindingAnnotation, 15);
assert_words!(BindingClass, 22);
assert_words!(BindingTParams, 10);
assert_words!(BindingClassMetadata, 8);
assert_bytes!(BindingClassMro, 4);
assert_words!(BindingClassField, 21);
assert_bytes!(BindingClassSynthesizedFields, 4);
assert_bytes!(BindingLegacyTypeParam, 4);
assert_words!(BindingYield, 4);
assert_words!(BindingYieldFrom, 4);
assert_words!(BindingFunction, 23);

#[derive(Clone, Dupe, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnyIdx {
    Key(Idx<Key>),
    KeyExpect(Idx<KeyExpect>),
    KeyClass(Idx<KeyClass>),
    KeyTParams(Idx<KeyTParams>),
    KeyClassField(Idx<KeyClassField>),
    KeyVariance(Idx<KeyVariance>),
    KeyClassSynthesizedFields(Idx<KeyClassSynthesizedFields>),
    KeyExport(Idx<KeyExport>),
    KeyFunction(Idx<KeyFunction>),
    KeyAnnotation(Idx<KeyAnnotation>),
    KeyClassMetadata(Idx<KeyClassMetadata>),
    KeyClassMro(Idx<KeyClassMro>),
    KeyLegacyTypeParam(Idx<KeyLegacyTypeParam>),
    KeyYield(Idx<KeyYield>),
    KeyYieldFrom(Idx<KeyYieldFrom>),
}

impl DisplayWith<Bindings> for AnyIdx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        match self {
            Self::Key(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyExpect(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyClass(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyTParams(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyClassField(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyVariance(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyClassSynthesizedFields(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyExport(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyFunction(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyAnnotation(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyClassMetadata(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyClassMro(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyLegacyTypeParam(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyYield(idx) => write!(f, "{}", ctx.display(*idx)),
            Self::KeyYieldFrom(idx) => write!(f, "{}", ctx.display(*idx)),
        }
    }
}

pub trait Keyed: Hash + Eq + Clone + DisplayWith<ModuleInfo> + Debug + Ranged + 'static {
    const EXPORTED: bool = false;
    type Value: Debug + DisplayWith<Bindings>;
    type Answer: Clone + Debug + Display + TypeEq + VisitMut<Type>;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx;
}

/// Should be equivalent to Keyed<EXPORTED=true>.
/// Once `associated_const_equality` is stabilised, can switch to that.
pub trait Exported: Keyed {}

impl Keyed for Key {
    type Value = Binding;
    type Answer = TypeInfo;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::Key(idx)
    }
}
impl Keyed for KeyExpect {
    type Value = BindingExpect;
    type Answer = EmptyAnswer;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyExpect(idx)
    }
}
impl Keyed for KeyClass {
    type Value = BindingClass;
    type Answer = NoneIfRecursive<Class>;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyClass(idx)
    }
}
impl Keyed for KeyTParams {
    const EXPORTED: bool = true;
    type Value = BindingTParams;
    type Answer = TParams;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyTParams(idx)
    }
}
impl Exported for KeyTParams {}
impl Keyed for KeyClassField {
    const EXPORTED: bool = true;
    type Value = BindingClassField;
    type Answer = ClassField;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyClassField(idx)
    }
}
impl Exported for KeyClassField {}
impl Keyed for KeyClassSynthesizedFields {
    const EXPORTED: bool = true;
    type Value = BindingClassSynthesizedFields;
    type Answer = ClassSynthesizedFields;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyClassSynthesizedFields(idx)
    }
}
impl Exported for KeyVariance {}
impl Keyed for KeyVariance {
    const EXPORTED: bool = true;
    type Value = BindingVariance;
    type Answer = VarianceMap;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyVariance(idx)
    }
}

impl Exported for KeyClassSynthesizedFields {}
impl Keyed for KeyExport {
    const EXPORTED: bool = true;
    type Value = BindingExport;
    type Answer = Type;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyExport(idx)
    }
}
impl Exported for KeyExport {}
impl Keyed for KeyFunction {
    type Value = BindingFunction;
    type Answer = DecoratedFunction;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyFunction(idx)
    }
}
impl Keyed for KeyAnnotation {
    type Value = BindingAnnotation;
    type Answer = AnnotationWithTarget;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyAnnotation(idx)
    }
}
impl Keyed for KeyClassMetadata {
    const EXPORTED: bool = true;
    type Value = BindingClassMetadata;
    type Answer = ClassMetadata;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyClassMetadata(idx)
    }
}
impl Exported for KeyClassMetadata {}
impl Keyed for KeyClassMro {
    const EXPORTED: bool = true;
    type Value = BindingClassMro;
    type Answer = ClassMro;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyClassMro(idx)
    }
}
impl Exported for KeyClassMro {}
impl Keyed for KeyLegacyTypeParam {
    type Value = BindingLegacyTypeParam;
    type Answer = LegacyTypeParameterLookup;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyLegacyTypeParam(idx)
    }
}
impl Keyed for KeyYield {
    type Value = BindingYield;
    type Answer = YieldResult;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyYield(idx)
    }
}
impl Keyed for KeyYieldFrom {
    type Value = BindingYieldFrom;
    type Answer = YieldFromResult;
    fn to_anyidx(idx: Idx<Self>) -> AnyIdx {
        AnyIdx::KeyYieldFrom(idx)
    }
}

/// Keys that refer to a `Type`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// I am an `import` at this location with this name.
    /// Used for `import foo.x` (the `foo` might not be literally present with `.` modules),
    /// and `from foo import *` (the names are injected from the exports)
    Import(Name, TextRange),
    /// I am a module-level global variable like `__file__` or `__doc__`.
    Global(Name),
    /// I am defined in this module at this location.
    Definition(ShortIdentifier),
    /// I am a name assignment that is also a first use of some other name assign.
    ///
    /// My raw definition contains unpinned placeholder types from both myself
    /// and upstream definitions, this binding will have all upstream
    /// placeholders (but not those originating from me) pinned.
    UpstreamPinnedDefinition(ShortIdentifier),
    /// I am the pinned version of a definition corresponding to a name assignment.
    ///
    /// Used in cases where the raw definition might introduce placeholder `Var` types
    /// that need to hidden from all lookups except the first usage to avoid nondeterminism.
    PinnedDefinition(ShortIdentifier),
    /// I am a name with possible attribute/subscript narrowing coming from an assignment at this location.
    FacetAssign(ShortIdentifier),
    /// The type at a specific return point.
    ReturnExplicit(TextRange),
    /// The implicit return type of a function, either Type::None or Type::Never.
    ReturnImplicit(ShortIdentifier),
    /// The actual type of the return for a function.
    ReturnType(ShortIdentifier),
    /// I am a name in this module at this location, bound to the associated binding.
    BoundName(ShortIdentifier),
    /// I am an expression that does not have a simple name but needs its type inferred.
    Anon(TextRange),
    /// I am a a narrowing operation created by a pattern in a match statement
    PatternNarrow(TextRange),
    /// I am an expression that appears in a statement. The range for this key is the range of the expr itself, which is different than the range of the stmt expr.
    StmtExpr(TextRange),
    /// I am an expression that appears in a `with` context.
    ContextExpr(TextRange),
    /// I am the result of joining several branches.
    Phi(Name, TextRange),
    /// I am the result of narrowing a type. The two ranges are the range at which the operation is
    /// defined and the one at which it is used. For example, in:
    ///   if x is None:
    ///       pass
    ///   else:
    ///       pass
    /// The `x is None` operation is defined once in the `if` test but generates two key/binding
    /// pairs, when it is used to narrow `x` in the `if` and the `else`, respectively.
    Narrow(Name, TextRange, TextRange),
    /// The binding definition site, anywhere it occurs
    Anywhere(Name, TextRange),
    /// Result of a super() call
    SuperInstance(TextRange),
    /// The intermediate used in an unpacking assignment.
    Unpack(TextRange),
    /// A usage link - a placeholder used for first-usage type inference.
    UsageLink(TextRange),
    /// A use of `typing.Self` in an expression. Used to redirect to the appropriate type (which is aware of the current class).
    SelfTypeLiteral(TextRange),
}

impl Ranged for Key {
    fn range(&self) -> TextRange {
        match self {
            Self::Import(_, r) => *r,
            Self::Global(_) => TextRange::default(),
            Self::Definition(x) => x.range(),
            Self::UpstreamPinnedDefinition(x) => x.range(),
            Self::PinnedDefinition(x) => x.range(),
            Self::FacetAssign(x) => x.range(),
            Self::ReturnExplicit(r) => *r,
            Self::ReturnImplicit(x) => x.range(),
            Self::ReturnType(x) => x.range(),
            Self::BoundName(x) => x.range(),
            Self::Anon(r) => *r,
            Self::StmtExpr(r) => *r,
            Self::ContextExpr(r) => *r,
            Self::Phi(_, r) => *r,
            Self::Narrow(_, r, _) => *r,
            Self::Anywhere(_, r) => *r,
            Self::SuperInstance(r) => *r,
            Self::Unpack(r) => *r,
            Self::UsageLink(r) => *r,
            Self::SelfTypeLiteral(r) => *r,
            Self::PatternNarrow(r) => *r,
        }
    }
}

impl DisplayWith<ModuleInfo> for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        let short = |x: &ShortIdentifier| format!("{} {}", ctx.display(x), ctx.display(&x.range()));

        match self {
            Self::Import(n, r) => write!(f, "Key::Import({n} {})", ctx.display(r)),
            Self::Global(n) => write!(f, "Key::Global({n})"),
            Self::Definition(x) => write!(f, "Key::Definition({})", short(x)),
            Self::UpstreamPinnedDefinition(x) => {
                write!(f, "Key::UpstreamPinnedDefinition({})", short(x))
            }
            Self::PinnedDefinition(x) => write!(f, "Key::PinnedDefinition({})", short(x)),
            Self::FacetAssign(x) => write!(f, "Key::FacetAssign({})", short(x)),
            Self::BoundName(x) => write!(f, "Key::BoundName({})", short(x)),
            Self::Anon(r) => write!(f, "Key::Anon({})", ctx.display(r)),
            Self::StmtExpr(r) => write!(f, "Key::StmtExpr({})", ctx.display(r)),
            Self::ContextExpr(r) => write!(f, "Key::ContextExpr({})", ctx.display(r)),
            Self::Phi(n, r) => write!(f, "Key::Phi({n} {})", ctx.display(r)),
            Self::Narrow(n, r1, r2) => {
                write!(
                    f,
                    "Key::Narrow({n} {} {})",
                    ctx.display(r1),
                    ctx.display(r2)
                )
            }
            Self::Anywhere(n, r) => write!(f, "Key::Anywhere({n} {})", ctx.display(r)),
            Self::ReturnType(x) => write!(f, "Key::Return({})", short(x)),
            Self::ReturnExplicit(r) => write!(f, "Key::ReturnExplicit({})", ctx.display(r)),
            Self::ReturnImplicit(x) => write!(f, "Key::ReturnImplicit({})", short(x)),
            Self::SuperInstance(r) => write!(f, "Key::SuperInstance({})", ctx.display(r)),
            Self::Unpack(r) => write!(f, "Key::Unpack({})", ctx.display(r)),
            Self::UsageLink(r) => write!(f, "Key::UsageLink({})", ctx.display(r)),
            Self::SelfTypeLiteral(r) => write!(f, "Key::SelfTypeLiteral({})", ctx.display(r)),
            Self::PatternNarrow(r) => write!(f, "Key::PatternNarrow({})", ctx.display(r)),
        }
    }
}

impl DisplayWith<Bindings> for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(f, "{}", ctx.module().display(self))
    }
}

/// An expectation to be checked. For example, that a sequence is of an expected length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyExpect(pub TextRange);

impl Ranged for KeyExpect {
    fn range(&self) -> TextRange {
        self.0
    }
}

impl DisplayWith<ModuleInfo> for KeyExpect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyExpect({})", ctx.display(&self.0))
    }
}

#[derive(Clone, Debug)]
pub enum ExprOrBinding {
    Expr(Expr),
    Binding(Binding),
}

impl DisplayWith<Bindings> for ExprOrBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        match self {
            Self::Expr(x) => write!(f, "{}", x.display_with(ctx.module())),
            Self::Binding(x) => write!(f, "{}", x.display_with(ctx)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum BindingExpect {
    /// An expression where we need to check for type errors, but don't need the result type.
    TypeCheckExpr(Expr),
    /// The expected number of values in an unpacked iterable expression.
    UnpackedLength(Idx<Key>, TextRange, SizeExpectation),
    /// An exception and its cause from a raise statement.
    CheckRaisedException(RaisedException),
    /// If a name already has an existing definition and we encounter a new definition,
    /// make sure the annotations are equal, with an associated name for error messages.
    Redefinition {
        new: Idx<KeyAnnotation>,
        existing: Idx<KeyAnnotation>,
        name: Name,
    },
    /// `del` statement
    Delete(Expr),
    /// Expression used in a boolean context (`bool()`, `if`, or `while`)
    Bool(Expr),
}

impl DisplayWith<Bindings> for BindingExpect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        let m = ctx.module();
        match self {
            Self::TypeCheckExpr(x) => {
                write!(f, "TypeCheckExpr({})", m.display(x))
            }
            Self::Bool(x) => {
                write!(f, "Bool({})", m.display(x))
            }
            Self::Delete(x) => {
                write!(f, "Delete({})", m.display(x))
            }
            Self::UnpackedLength(x, range, expect) => {
                let expectation = match expect {
                    SizeExpectation::Eq(n) => format!("=={n}"),
                    SizeExpectation::Ge(n) => format!(">={n}"),
                };
                write!(
                    f,
                    "UnpackLength({} {} {})",
                    ctx.display(*x),
                    ctx.module().display(range),
                    expectation,
                )
            }
            Self::CheckRaisedException(RaisedException::WithoutCause(exc)) => {
                write!(f, "RaisedException::WithoutCause({})", m.display(exc))
            }
            Self::CheckRaisedException(RaisedException::WithCause(box (exc, cause))) => {
                write!(
                    f,
                    "RaisedException::WithCause({}, {})",
                    m.display(exc),
                    m.display(cause)
                )
            }
            Self::Redefinition {
                new,
                existing,
                name,
            } => write!(
                f,
                "Redefinition({} == {} on {})",
                ctx.display(*new),
                ctx.display(*existing),
                name
            ),
        }
    }
}

#[derive(Debug, Clone, TypeEq, VisitMut, PartialEq, Eq)]
pub struct EmptyAnswer;

impl Display for EmptyAnswer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "()")
    }
}

#[derive(Debug, Clone, TypeEq, VisitMut, PartialEq, Eq)]
pub struct NoneIfRecursive<T>(pub Option<T>);

impl<T> Display for NoneIfRecursive<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Some(x) => x.fmt(f),
            None => write!(f, "recursive"),
        }
    }
}

/// The binding definition site, at the end of the module (used for export).
/// If it has an annotation, only the annotation will be returned.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyExport(pub Name);

impl Ranged for KeyExport {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyExport({})", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyFunction(pub ShortIdentifier);

impl Ranged for KeyFunction {
    fn range(&self) -> TextRange {
        self.0.range()
    }
}

impl DisplayWith<ModuleInfo> for KeyFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(
            f,
            "KeyFunction({} {})",
            ctx.display(&self.0),
            ctx.display(&self.0.range())
        )
    }
}

/// A reference to a class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyClass(pub ShortIdentifier);

impl Ranged for KeyClass {
    fn range(&self) -> TextRange {
        self.0.range()
    }
}

impl DisplayWith<ModuleInfo> for KeyClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(
            f,
            "KeyClass({} {})",
            ctx.display(&self.0),
            ctx.display(&self.0.range())
        )
    }
}

/// A reference to a class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyTParams(pub ClassDefIndex);

impl Ranged for KeyTParams {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyTParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyTParams({})", self.0)
    }
}

/// A reference to a field in a class.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyClassField(pub ClassDefIndex, pub Name);

impl Ranged for KeyClassField {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyClassField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyClassField(class{}, {})", self.0, self.1)
    }
}

/// Keys that refer to fields synthesized by a class, such as a dataclass's `__init__` method. This
/// has to be its own key/binding type because of the dependencies between the various pieces of
/// information about a class: ClassDef -> ClassMetadata -> ClassField -> ClassSynthesizedFields.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyClassSynthesizedFields(pub ClassDefIndex);

impl Ranged for KeyClassSynthesizedFields {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyClassSynthesizedFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyClassSynthesizedFields(class{})", self.0)
    }
}

// A key that denotes the variance of a type parameter
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyVariance(pub ClassDefIndex);

impl Ranged for KeyVariance {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyVariance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyVariance(class{})", self.0)
    }
}

/// Keys that refer to an `Annotation`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyAnnotation {
    /// I am the annotation for this instance of a name.
    Annotation(ShortIdentifier),
    /// The return type annotation for a function.
    ReturnAnnotation(ShortIdentifier),
    /// I am the annotation for the attribute at this range.
    AttrAnnotation(TextRange),
}

impl Ranged for KeyAnnotation {
    fn range(&self) -> TextRange {
        match self {
            Self::Annotation(x) => x.range(),
            Self::ReturnAnnotation(x) => x.range(),
            Self::AttrAnnotation(r) => *r,
        }
    }
}

impl DisplayWith<ModuleInfo> for KeyAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        let short = |x: &ShortIdentifier| format!("{} {}", ctx.display(x), ctx.display(&x.range()));
        match self {
            Self::Annotation(x) => write!(f, "KeyAnnotation::Annotation({})", short(x)),
            Self::ReturnAnnotation(x) => write!(f, "KeyAnnotation::ReturnAnnotation({})", short(x)),
            Self::AttrAnnotation(r) => {
                write!(f, "KeyAnnotation::AttAnnotation({})", ctx.display(r))
            }
        }
    }
}

/// Keys that refer to a class's `Mro` (which tracks its ancestors, in method
/// resolution order).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyClassMetadata(pub ClassDefIndex);

impl Ranged for KeyClassMetadata {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyClassMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyClassMetadata(class{})", self.0)
    }
}

/// Keys that refer to a class's `Mro` (which tracks its ancestors, in method
/// resolution order).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyClassMro(pub ClassDefIndex);

impl Ranged for KeyClassMro {
    fn range(&self) -> TextRange {
        TextRange::default()
    }
}

impl DisplayWith<ModuleInfo> for KeyClassMro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyClassMro(class{})", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyLegacyTypeParam(pub ShortIdentifier);

impl Ranged for KeyLegacyTypeParam {
    fn range(&self) -> TextRange {
        self.0.range()
    }
}

impl DisplayWith<ModuleInfo> for KeyLegacyTypeParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(
            f,
            "KeyLegacyTypeParam({} {})",
            ctx.display(&self.0),
            ctx.display(&self.0.range()),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyYield(pub TextRange);

impl Ranged for KeyYield {
    fn range(&self) -> TextRange {
        self.0
    }
}

impl DisplayWith<ModuleInfo> for KeyYield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyYield({})", ctx.display(&self.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyYieldFrom(pub TextRange);

impl Ranged for KeyYieldFrom {
    fn range(&self) -> TextRange {
        self.0
    }
}

impl DisplayWith<ModuleInfo> for KeyYieldFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &ModuleInfo) -> fmt::Result {
        write!(f, "KeyYieldFrom({})", ctx.display(&self.0),)
    }
}

#[derive(Clone, Copy, Dupe, Debug)]
pub enum UnpackedPosition {
    /// Zero-based index
    Index(usize),
    /// A negative index, counting from the back
    ReverseIndex(usize),
    /// Slice represented as an index from the front to an index from the back.
    /// Note that even though the second index is conceptually negative, we can
    /// represent it as a usize because it is always negative.
    Slice(usize, usize),
}

#[derive(Clone, Debug)]
pub enum SizeExpectation {
    Eq(usize),
    Ge(usize),
}

#[derive(Clone, Debug)]
pub enum RaisedException {
    WithoutCause(Expr),
    WithCause(Box<(Expr, Expr)>),
}

#[derive(Clone, Dupe, Copy, Debug, Eq, PartialEq)]
pub enum IsAsync {
    Sync,
    Async,
}

impl IsAsync {
    pub fn new(is_async: bool) -> Self {
        if is_async { Self::Async } else { Self::Sync }
    }

    pub fn is_async(self) -> bool {
        matches!(self, Self::Async)
    }

    pub fn context_exit_dunder(self) -> Name {
        match self {
            Self::Sync => dunder::EXIT,
            Self::Async => dunder::AEXIT,
        }
    }
}

/// Is the body of this function stubbed out (contains nothing but `...`)?
#[derive(Clone, Copy, Debug, PartialEq, Eq, TypeEq, VisitMut)]
pub enum FunctionStubOrImpl {
    /// The function body is `...`.
    Stub,
    /// The function body is not `...`.
    Impl,
}

#[derive(Clone, Debug)]
pub struct BindingFunction {
    /// A function definition, but with the return/body stripped out.
    pub def: StmtFunctionDef,
    pub stub_or_impl: FunctionStubOrImpl,
    pub class_key: Option<Idx<KeyClass>>,
    pub decorators: Box<[Idx<Key>]>,
    pub legacy_tparams: Box<[Idx<KeyLegacyTypeParam>]>,
    pub successor: Option<Idx<KeyFunction>>,
    pub docstring_range: Option<TextRange>,
}

impl DisplayWith<Bindings> for BindingFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &Bindings) -> fmt::Result {
        write!(f, "BindingFunction({})", self.def.name.id)
    }
}

#[derive(Clone, Debug)]
pub struct ClassBinding {
    /// A class definition, but with the body stripped out.
    pub def: StmtClassDef,
    pub def_index: ClassDefIndex,
    /// The fields are all the names declared on the class that we were able to detect
    /// from an AST traversal, which includes:
    /// - any name defined in the class body (e.g. by assignment or a def statement)
    /// - attributes annotated in the class body (but not necessarily defined)
    /// - anything assigned to something we think is a `self` or `cls` argument
    ///
    /// The last case may include names that are actually declared in a parent class,
    /// because at binding time we cannot know that so we have to treat assignment
    /// as potentially defining a field that would not otherwise exist.
    pub fields: SmallMap<Name, ClassFieldProperties>,
    /// Were we able to determine, using only syntactic analysis at bindings time,
    /// that there can be no legacy tparams? If no, we need a `BindingTParams`, if yes
    /// we can directly compute the `TParams` from the class def.
    pub tparams_require_binding: bool,
    pub docstring_range: Option<TextRange>,
}

#[derive(Clone, Debug)]
pub struct ReturnExplicit {
    pub annot: Option<Idx<KeyAnnotation>>,
    pub expr: Option<Box<Expr>>,
    pub is_generator: bool,
    pub is_async: bool,
}

#[derive(Clone, Debug)]
pub enum ReturnTypeKind {
    /// We have an explicit return annotation, and we should validate it against the implicit returns
    ShouldValidateAnnotation {
        range: TextRange,
        annotation: Idx<KeyAnnotation>,
        /// Used to skip the validation for stub functions (returning `...`). This is
        /// unsafe, but is convenient and matches Pyright's behavior.
        stub_or_impl: FunctionStubOrImpl,
        /// We keep this just so we can scan for `@abstractmethod` and use the info to decide
        /// whether to skip the validation.
        decorators: Box<[Idx<Key>]>,
        implicit_return: Idx<Key>,
        is_generator: bool,
        has_explicit_return: bool,
    },
    /// We have an explicit return annotation, and we should blindly trust it without any validation
    ShouldTrustAnnotation {
        annotation: Idx<KeyAnnotation>,
        is_generator: bool,
    },
    /// We don't have an explicit return annotation, and we should just act as if the return is annotated as `Any`
    ShouldReturnAny { is_generator: bool },
    /// We don't have an explicit return annotation, and we should do our best to infer the return type
    ShouldInferType {
        /// The returns from the function.
        returns: Box<[Idx<Key>]>,
        implicit_return: Idx<Key>,
        /// The `yield`s and `yield from`s. If either of these are nonempty, this is a generator function.
        /// We don't need to store `is_generator` flag in this case, as we can deduce that info by checking
        /// whether these two fields are empty or not.
        yields: Box<[Idx<KeyYield>]>,
        yield_froms: Box<[Idx<KeyYieldFrom>]>,
    },
}

impl ReturnTypeKind {
    pub fn has_return_annotation(&self) -> bool {
        match self {
            Self::ShouldValidateAnnotation { .. } => true,
            Self::ShouldTrustAnnotation { .. } => true,
            Self::ShouldReturnAny { .. } => false,
            Self::ShouldInferType { .. } => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReturnType {
    pub kind: ReturnTypeKind,
    pub is_async: bool,
}

#[derive(Clone, Dupe, Copy, Debug)]
pub enum LastStmt {
    /// The last statement is a expression
    Expr,
    /// The last statement is a `with`, with the following context,
    /// which might (if exit is true) catch an exception
    With(IsAsync),
}

#[derive(Clone, Debug)]
pub struct ReturnImplicit {
    /// Terminal statements in the function control flow, used to determine whether the
    /// function has an implicit `None` return.
    /// When `None`, the function always has an implicit `None` return. When `Some(xs)`,
    /// the function has an implicit `None` return if there exists a non-`Never` in this
    /// list.
    pub last_exprs: Option<Box<[(LastStmt, Idx<Key>)]>>,
}

#[derive(Clone, Debug)]
pub enum SuperStyle {
    /// A `super(cls, obj)` call. The keys are the arguments.
    ExplicitArgs(Idx<Key>, Idx<Key>),
    /// A no-argument `super()` call. The key is the `Self` type of the class we are in.
    /// The name is the method we are in.
    ImplicitArgs(Idx<KeyClass>, Identifier),
    /// `super(Any, Any)`. Useful when we encounter an error.
    Any,
}

#[derive(Clone, Debug, Copy, Dupe, PartialEq, Eq)]
pub enum AnnotationStyle {
    /// Annotated assignment, x: MyType = my_value
    Direct,
    /// Forwarded annotation, x: MyType; x = my_value
    Forwarded,
}

#[derive(Clone, Debug)]
pub struct TypeParameter {
    pub name: Name,
    pub unique: Unique,
    pub kind: QuantifiedKind,
    pub bound: Option<Expr>,
    pub default: Option<Expr>,
    pub constraints: Option<(Vec<Expr>, TextRange)>,
}

/// Represents an `Idx<K>` for some `K: Keyed` other than `Key`
/// that we want to track for first-usage type inference.
#[derive(Clone, Debug)]
pub enum LinkedKey {
    Yield(Idx<KeyYield>),
    YieldFrom(Idx<KeyYieldFrom>),
    Expect(Idx<KeyExpect>),
}

#[derive(Clone, Debug)]
pub enum FirstUse {
    /// We are still awaiting a first use
    Undetermined,
    /// We encountered the first use, and it does not pin the type (so we should force
    /// all placeholder variables to default values).
    DoesNotPin,
    /// This binding is the first use, we should calculate it to get first-use based
    /// inference.
    UsedBy(Idx<Key>),
}

#[derive(Clone, Debug)]
pub enum Binding {
    /// An expression, optionally with a Key saying what the type must be.
    /// The Key must be a type of types, e.g. `Type::Type`.
    Expr(Option<Idx<KeyAnnotation>>, Expr),
    /// Propagate a type to a new binding. Takes an optional annotation to
    /// check against (which will override the computed type if they disagree).
    MultiTargetAssign(Option<Idx<KeyAnnotation>>, Idx<Key>, TextRange),
    /// TypeVar, ParamSpec, or TypeVarTuple
    TypeVar(Option<Idx<KeyAnnotation>>, Identifier, Box<ExprCall>),
    ParamSpec(Option<Idx<KeyAnnotation>>, Identifier, Box<ExprCall>),
    TypeVarTuple(Option<Idx<KeyAnnotation>>, Identifier, Box<ExprCall>),
    /// An expression returned from a function.
    ReturnExplicit(ReturnExplicit),
    /// The implicit return from a function.
    ReturnImplicit(ReturnImplicit),
    /// The return type of a function.
    ReturnType(Box<ReturnType>),
    /// A value in an iterable expression, e.g. IterableValue(\[1\]) represents 1.
    /// The second argument is the expression being iterated.
    /// The third argument indicates whether iteration is async or not.
    IterableValue(Option<Idx<KeyAnnotation>>, Expr, IsAsync),
    /// A value produced by entering a context manager.
    /// The second argument is the expression of the context manager and its range.
    /// The fourth argument indicates whether the context manager is async or not.
    ContextValue(Option<Idx<KeyAnnotation>>, Idx<Key>, TextRange, IsAsync),
    /// A value at a specific position in an unpacked iterable expression.
    /// Example: UnpackedValue(('a', 'b')), 1) represents 'b'.
    UnpackedValue(
        Option<Idx<KeyAnnotation>>,
        Idx<Key>,
        TextRange,
        UnpackedPosition,
    ),
    /// A type where we have an annotation, but also a type we computed.
    /// If the annotation has a type inside it (e.g. `int` then use the annotation).
    /// If the annotation doesn't (e.g. it's `Final`), then use the binding.
    AnnotatedType(Idx<KeyAnnotation>, Box<Binding>),
    /// A record of an "augmented assignment" statement like `x -= _`
    /// or `a.b *= _`. These desugar to special method calls.
    AugAssign(Option<Idx<KeyAnnotation>>, StmtAugAssign),
    /// An explicit type.
    Type(Type),
    /// A global variable.
    Global(Global),
    /// A type parameter.
    TypeParameter(Box<TypeParameter>),
    /// The type of a function. The fields are:
    /// - A reference to the KeyFunction that point to the def
    /// - An optional reference to any previous function in the same flow by the same name;
    ///   this is needed to fold `@overload` decorated defs into a single type.
    /// - An optional reference to class metadata, which will be non-None when the function
    ///   is defined within a class scope.
    Function(
        Idx<KeyFunction>,
        Option<Idx<Key>>,
        Option<Idx<KeyClassMetadata>>,
    ),
    /// An import statement, typically with Self::Import.
    /// The option range tracks the original name's location for renamed import.
    /// e.g. in `from foo import bar as baz`, we should track the range of `bar`.
    Import(ModuleName, Name, Option<TextRange>),
    /// A class definition, points to a BindingClass and any decorators.
    ClassDef(Idx<KeyClass>, Box<[Idx<Key>]>),
    /// A forward reference to another binding.
    Forward(Idx<Key>),
    /// A phi node, representing the union of several alternative keys.
    Phi(SmallSet<Idx<Key>>),
    /// Used if the binding ends up being recursive, instead of defaulting to `Any`, should
    /// default to the given type.
    Default(Idx<Key>, Box<Binding>),
    /// A narrowed type.
    Narrow(Idx<Key>, Box<NarrowOp>, TextRange),
    /// An import of a module.
    /// Also contains the path along the module to bind, and optionally a key
    /// with the previous import to this binding (in which case merge the modules).
    Module(ModuleName, Vec<Name>, Option<Idx<Key>>),
    /// A name that might be a legacy type parameter. Solving this gives the Quantified type if so.
    /// The TextRange is optional and should be set at most once per identifier
    /// to avoid duplicate type errors (this is not type safe, because we might
    /// produce multiple `CheckLegacyTypeParam` bindings for the same
    /// identifier).
    /// It controls whether to produce an error saying there are scoped type parameters for this
    /// function / class, and therefore the use of legacy type parameters is invalid.
    CheckLegacyTypeParam(Idx<KeyLegacyTypeParam>, Option<TextRange>),
    /// An assignment to a name.
    NameAssign(
        Name,
        Option<(AnnotationStyle, Idx<KeyAnnotation>)>,
        Box<Expr>,
    ),
    /// A type alias declared with the `type` soft keyword
    ScopedTypeAlias(Name, Option<TypeParams>, Box<Expr>),
    /// An entry in a MatchMapping. The Key looks up the value being matched, the Expr is the key we're extracting.
    PatternMatchMapping(Expr, Idx<Key>),
    /// An entry in a MatchClass. The Key looks up the value being matched, the Expr is the class name.
    /// Positional patterns index into __match_args__, and keyword patterns match an attribute name.
    PatternMatchClassPositional(Box<Expr>, usize, Idx<Key>, TextRange),
    PatternMatchClassKeyword(Box<Expr>, Identifier, Idx<Key>),
    /// Binding for an `except` (if the boolean flag is false) or `except*` (if the boolean flag is true) clause
    ExceptionHandler(Box<Expr>, bool),
    /// Binding for an `@decorator` decoration on a function or class
    Decorator(Expr),
    /// Binding for a lambda parameter.
    LambdaParameter(Var),
    /// Binding for a function parameter. We either have an annotation, or we will determine the
    /// parameter type when solving the function type.
    FunctionParameter(Either<Idx<KeyAnnotation>, Var>),
    /// The result of a `super()` call.
    SuperInstance(SuperStyle, TextRange),
    /// The result of assigning to an attribute. This operation cannot change the *type* of the
    /// name to which we are assigning, but it *can* change the live attribute narrows.
    AssignToAttribute(ExprAttribute, Box<ExprOrBinding>),
    /// The result of assigning to a subscript, used for narrowing.
    AssignToSubscript(ExprSubscript, Box<ExprOrBinding>),
    /// A placeholder binding, used to force the solving of some other `K::Value` (for
    /// example, forcing a `BindingExpect` to be solved) in the context of first-usage-based
    /// type inference.
    UsageLink(LinkedKey),
    /// Inside of a class body, we check whether an expression resolves to the `SelfType` special
    /// export. If so, we create a `SelfTypeLiteral` key/binding pair so that the AnswersSolver can
    /// later synthesize the correct `Type::SelfType` (this binding is needed
    /// because we need access to the current class to do so).
    SelfTypeLiteral(Idx<KeyClass>, TextRange),
    /// Binding used to pin placeholder types from `NameAssign` bindings. The first
    /// entry should always correspond to a `Key::Definition` from a name assignment
    /// and the second entry tells us if and where this definition is first used.
    Pin(Idx<Key>, FirstUse),
    /// Binding used to pin any *upstream* placeholder types for a NameAssign that is also
    /// a first use. First uses depend on this binding, so that upstream `Var`s cannot
    /// leak into them but `Var`s originating from this assignment can.
    ///
    /// The Idx is the upstream raw `NameAssign`, and the slice has `Idx`s that point at
    /// all the `Pin`s for which that raw `NameAssign` was the first use.
    PinUpstream(Idx<Key>, Box<[Idx<Key>]>),
}

impl DisplayWith<Bindings> for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        let m = ctx.module();
        let ann = |k: &Option<Idx<KeyAnnotation>>| match k {
            None => "None".to_owned(),
            Some(k) => ctx.display(*k).to_string(),
        };
        match self {
            Self::Expr(a, x) => write!(f, "Expr({}, {})", ann(a), m.display(x)),
            Self::MultiTargetAssign(a, idx, range) => {
                write!(
                    f,
                    "MultiTargetAssign({}, {}, {})",
                    ann(a),
                    ctx.display(*idx),
                    m.display(range),
                )
            }
            Self::TypeVar(a, name, x) => {
                write!(f, "TypeVar({}, {name}, {})", ann(a), m.display(x))
            }
            Self::ParamSpec(a, name, x) => {
                write!(f, "ParamSpec({}, {name}, {})", ann(a), m.display(x))
            }
            Self::TypeVarTuple(a, name, x) => {
                write!(f, "TypeVarTuple({}, {name}, {})", ann(a), m.display(x))
            }
            Self::ReturnExplicit(x) => {
                write!(f, "ReturnExplicit({}, ", ann(&x.annot))?;
                match &x.expr {
                    None => write!(f, "None")?,
                    Some(x) => write!(f, "{}", m.display(x))?,
                }
                if x.is_generator {
                    write!(f, ", is_generator")?;
                }
                if x.is_async {
                    write!(f, ", is_async")?;
                }
                write!(f, ")")
            }
            Self::ReturnImplicit(_) => write!(f, "ReturnImplicit(_)"),
            Self::ReturnType(_) => write!(f, "ReturnType(_)"),
            Self::IterableValue(a, x, sync) => {
                write!(f, "IterableValue({}, {}, {sync:?})", ann(a), m.display(x))
            }
            Self::ExceptionHandler(x, b) => write!(f, "ExceptionHandler({}, {b:?})", m.display(x)),
            Self::ContextValue(a, x, _, kind) => {
                write!(f, "ContextValue({}, {}, {kind:?})", ann(a), ctx.display(*x))
            }
            Self::UnpackedValue(a, x, range, pos) => {
                let pos = match pos {
                    UnpackedPosition::Index(i) => i.to_string(),
                    UnpackedPosition::ReverseIndex(i) => format!("-{i}"),
                    UnpackedPosition::Slice(i, j) => {
                        let end = match j {
                            0 => "".to_owned(),
                            _ => format!("-{j}"),
                        };
                        format!("{i}:{end}")
                    }
                };
                write!(
                    f,
                    "UnpackedValue({}, {}, {}, {})",
                    ann(a),
                    ctx.display(*x),
                    m.display(range),
                    pos
                )
            }
            Self::Function(x, _pred, _class) => write!(f, "Function({})", ctx.display(*x)),
            Self::Import(m, n, original_name) => write!(f, "Import({m}, {n}, {original_name:?})"),
            Self::ClassDef(x, _) => write!(f, "ClassDef({})", ctx.display(*x)),
            Self::Forward(k) => write!(f, "Forward({})", ctx.display(*k)),
            Self::AugAssign(a, s) => write!(f, "AugAssign({}, {})", ann(a), m.display(s)),
            Self::Type(t) => write!(f, "Type({t})"),
            Self::Global(g) => write!(f, "Global({})", g.name()),
            Self::TypeParameter(box TypeParameter { unique, kind, .. }) => {
                write!(f, "TypeParameter({unique}, {kind}, ..)")
            }
            Self::CheckLegacyTypeParam(k, _) => {
                write!(f, "CheckLegacyTypeParam({})", ctx.display(*k))
            }
            Self::AnnotatedType(k1, k2) => {
                write!(
                    f,
                    "AnnotatedType({}, {})",
                    ctx.display(*k1),
                    k2.display_with(ctx)
                )
            }
            Self::Module(m, path, key) => {
                write!(
                    f,
                    "Module({m}, {}, {})",
                    path.join("."),
                    match key {
                        None => "None".to_owned(),
                        Some(k) => ctx.display(*k).to_string(),
                    }
                )
            }
            Self::Phi(xs) => {
                write!(
                    f,
                    "Phi({})",
                    intersperse_iter("; ", || xs.iter().map(|x| ctx.display(*x)))
                )
            }
            Self::Default(k, x) => {
                write!(f, "Default({}, {})", ctx.display(*k), x.display_with(ctx))
            }
            Self::Narrow(k, op, _) => {
                write!(
                    f,
                    "Narrow({}, {})",
                    ctx.display(*k),
                    op.display_with(ctx.module())
                )
            }
            Self::NameAssign(name, None, expr) => {
                write!(f, "NameAssign({name}, None, {})", m.display(expr))
            }
            Self::NameAssign(name, Some((style, annot)), expr) => {
                write!(
                    f,
                    "NameAssign({name}, {style:?}, {}, {})",
                    ctx.display(*annot),
                    m.display(expr)
                )
            }
            Self::ScopedTypeAlias(name, params, expr) => {
                write!(
                    f,
                    "ScopedTypeAlias({name}, {}, {})",
                    match params {
                        None => "None".to_owned(),
                        Some(params) => commas_iter(|| params.iter().map(|p| p.name())).to_string(),
                    },
                    m.display(expr)
                )
            }
            Self::PatternMatchMapping(mapping_key, binding_key) => {
                write!(
                    f,
                    "PatternMatchMapping({}, {})",
                    m.display(mapping_key),
                    ctx.display(*binding_key),
                )
            }
            Self::PatternMatchClassPositional(class, idx, key, range) => {
                write!(
                    f,
                    "PatternMatchClassPositional({}, {idx}, {}, {})",
                    m.display(class),
                    ctx.display(*key),
                    m.display(range),
                )
            }
            Self::PatternMatchClassKeyword(class, attr, key) => {
                write!(
                    f,
                    "PatternMatchClassKeyword({}, {attr}, {})",
                    m.display(class),
                    ctx.display(*key),
                )
            }
            Self::Decorator(e) => write!(f, "Decorator({})", m.display(e)),
            Self::LambdaParameter(x) => write!(f, "LambdaParameter({x})"),
            Self::FunctionParameter(x) => write!(
                f,
                "FunctionParameter({})",
                match x {
                    Either::Left(k) => ctx.display(*k).to_string(),
                    Either::Right(x) => x.to_string(),
                }
            ),
            Self::SuperInstance(SuperStyle::ExplicitArgs(cls, obj), _range) => {
                write!(
                    f,
                    "SuperInstance::Explicit({}, {})",
                    ctx.display(*cls),
                    ctx.display(*obj)
                )
            }
            Self::SuperInstance(SuperStyle::ImplicitArgs(k, v), _range) => {
                write!(f, "SuperInstance::Implicit({}, {v})", ctx.display(*k))
            }
            Self::SuperInstance(SuperStyle::Any, _range) => write!(f, "SuperInstance::Any"),
            Self::AssignToAttribute(attr, x) => {
                write!(
                    f,
                    "AssignToAttribute({}, {}, {})",
                    m.display(&attr.value),
                    attr.attr,
                    x.display_with(ctx)
                )
            }
            Self::AssignToSubscript(subscript, x) => {
                write!(
                    f,
                    "AssignToSubscript({}, {}, {})",
                    m.display(subscript.value.as_ref()),
                    m.display(subscript.slice.as_ref()),
                    x.display_with(ctx)
                )
            }
            Self::UsageLink(usage_key) => {
                write!(f, "UsageLink(")?;
                match usage_key {
                    LinkedKey::Yield(idx) => write!(f, "{}", m.display(ctx.idx_to_key(*idx)))?,
                    LinkedKey::YieldFrom(idx) => write!(f, "{}", m.display(ctx.idx_to_key(*idx)))?,
                    LinkedKey::Expect(idx) => write!(f, "{}", m.display(ctx.idx_to_key(*idx)))?,
                }
                write!(f, ")")
            }
            Self::SelfTypeLiteral(class_key, r) => {
                write!(
                    f,
                    "SelfTypeLiteral({}, {})",
                    m.display(ctx.idx_to_key(*class_key)),
                    m.display(r)
                )
            }
            Self::Pin(k, first_use) => {
                write!(f, "Pin({}, ", ctx.display(*k),)?;
                match first_use {
                    FirstUse::Undetermined => write!(f, "Undetermined")?,
                    FirstUse::DoesNotPin => write!(f, "DoesNotPin")?,
                    FirstUse::UsedBy(idx) => write!(f, "UsedBy {}", ctx.display(*idx))?,
                }
                write!(f, ")")
            }
            Self::PinUpstream(k, first_used_by) => {
                write!(
                    f,
                    "PinUpstream({}, [{}])",
                    ctx.display(*k),
                    commas_iter(|| first_used_by.iter().map(|x| ctx.display(*x)))
                )
            }
        }
    }
}

impl Binding {
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Binding::TypeVar(_, _, _)
            | Binding::ParamSpec(_, _, _)
            | Binding::TypeVarTuple(_, _, _)
            | Binding::TypeParameter(_)
            | Binding::CheckLegacyTypeParam(_, _) => Some(SymbolKind::TypeParameter),
            Binding::Global(_) => Some(SymbolKind::Variable),
            Binding::Function(_, _, _) => Some(SymbolKind::Function),
            Binding::Import(_, _, _) => {
                // TODO: maybe we can resolve it to see its symbol kind
                Some(SymbolKind::Variable)
            }
            Binding::ClassDef(_, _) => Some(SymbolKind::Class),
            Binding::Module(_, _, _) => Some(SymbolKind::Module),
            Binding::ScopedTypeAlias(_, _, _) => Some(SymbolKind::TypeAlias),
            Binding::NameAssign(name, _, _) if name.as_str() == name.to_uppercase() => {
                Some(SymbolKind::Constant)
            }
            Binding::NameAssign(name, _, _) => {
                if name.as_str().chars().all(|c| c.is_uppercase() || c == '_') {
                    Some(SymbolKind::Constant)
                } else {
                    Some(SymbolKind::Variable)
                }
            }
            Binding::LambdaParameter(_) | Binding::FunctionParameter(_) => {
                Some(SymbolKind::Parameter)
            }
            Binding::Expr(_, _)
            | Binding::MultiTargetAssign(_, _, _)
            | Binding::ReturnExplicit(_)
            | Binding::ReturnImplicit(_)
            | Binding::ReturnType(_)
            | Binding::IterableValue(_, _, _)
            | Binding::ContextValue(_, _, _, _)
            | Binding::UnpackedValue(_, _, _, _)
            | Binding::AnnotatedType(_, _)
            | Binding::AugAssign(_, _)
            | Binding::Type(_)
            | Binding::Forward(_)
            | Binding::Phi(_)
            | Binding::Default(_, _)
            | Binding::Narrow(_, _, _)
            | Binding::PatternMatchMapping(_, _)
            | Binding::PatternMatchClassPositional(_, _, _, _)
            | Binding::PatternMatchClassKeyword(_, _, _)
            | Binding::Decorator(_)
            | Binding::ExceptionHandler(_, _)
            | Binding::SuperInstance(_, _)
            | Binding::AssignToAttribute(_, _)
            | Binding::UsageLink(_)
            | Binding::SelfTypeLiteral(..)
            | Binding::AssignToSubscript(_, _)
            | Binding::Pin(..)
            | Binding::PinUpstream(..) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BindingExport(pub Binding);

impl DisplayWith<Bindings> for BindingExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        DisplayWith::fmt(&self.0, f, ctx)
    }
}

#[derive(Debug, Clone, Copy, VisitMut, TypeEq, PartialEq, Eq)]
pub enum Initialized {
    Yes,
    No,
}

#[derive(Debug, Clone, VisitMut, TypeEq, PartialEq, Eq)]
pub struct AnnotationWithTarget {
    pub target: AnnotationTarget,
    pub annotation: Annotation,
}

impl AnnotationWithTarget {
    pub fn ty(&self, stdlib: &Stdlib) -> Option<Type> {
        let annotation_ty = self.annotation.ty.as_ref()?;
        match self.target {
            AnnotationTarget::ArgsParam(_) => {
                if let Type::Unpack(unpacked) = annotation_ty {
                    Some(Type::Tuple(Tuple::unpacked(
                        Vec::new(),
                        (**unpacked).clone(),
                        Vec::new(),
                    )))
                } else if matches!(annotation_ty, Type::Args(_)) {
                    Some(annotation_ty.clone())
                } else {
                    Some(Type::Tuple(Tuple::Unbounded(Box::new(
                        annotation_ty.clone(),
                    ))))
                }
            }
            AnnotationTarget::KwargsParam(_) => {
                if let Type::Unpack(unpacked) = annotation_ty {
                    Some((**unpacked).clone())
                } else if matches!(annotation_ty, Type::Kwargs(_) | Type::Unpack(_)) {
                    Some(annotation_ty.clone())
                } else {
                    Some(
                        stdlib
                            .dict(stdlib.str().clone().to_type(), annotation_ty.clone())
                            .to_type(),
                    )
                }
            }
            _ => Some(annotation_ty.clone()),
        }
    }
}

impl Display for AnnotationWithTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.target, self.annotation)
    }
}

#[derive(Debug, Clone, VisitMut, TypeEq, PartialEq, Eq)]
pub enum AnnotationTarget {
    /// A function parameter with a type annotation
    Param(Name),
    ArgsParam(Name),
    KwargsParam(Name),
    /// A return type annotation on a function. The name is that of the function
    Return(Name),
    /// An annotated assignment. For attribute assignments, the name is the attribute name ("attr" in "x.attr")
    /// Does the annotated assignment have an initial value?
    Assign(Name, Initialized),
    /// A member of a class
    ClassMember(Name),
}

impl Display for AnnotationTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Param(name) => write!(f, "param {name}"),
            Self::ArgsParam(name) => write!(f, "args {name}"),
            Self::KwargsParam(name) => write!(f, "kwargs {name}"),
            Self::Return(name) => write!(f, "{name} return"),
            Self::Assign(name, _initialized) => write!(f, "var {name}"),
            Self::ClassMember(name) => write!(f, "attr {name}"),
        }
    }
}

impl AnnotationTarget {
    pub fn type_form_context(&self) -> TypeFormContext {
        match self {
            Self::Param(_) => TypeFormContext::ParameterAnnotation,
            Self::ArgsParam(_) => TypeFormContext::ParameterArgsAnnotation,
            Self::KwargsParam(_) => TypeFormContext::ParameterKwargsAnnotation,
            Self::Return(_) => TypeFormContext::ReturnAnnotation,
            Self::Assign(_, is_initialized) => TypeFormContext::VarAnnotation(*is_initialized),
            Self::ClassMember(_) => TypeFormContext::ClassVarAnnotation,
        }
    }
}

/// Values that return an annotation.
#[derive(Clone, Debug)]
pub enum BindingAnnotation {
    /// The type is annotated to be this key, will have the outer type removed.
    /// Optionally occurring within a class, in which case Self refers to this class.
    AnnotateExpr(AnnotationTarget, Expr, Option<Idx<KeyClass>>),
    /// A literal type we know statically.
    Type(AnnotationTarget, Type),
}

impl DisplayWith<Bindings> for BindingAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        match self {
            Self::AnnotateExpr(target, x, class_key) => write!(
                f,
                "AnnotateExpr({target}, {}, {})",
                ctx.module().display(x),
                match class_key {
                    None => "None".to_owned(),
                    Some(t) => ctx.display(*t).to_string(),
                }
            ),
            Self::Type(target, t) => write!(f, "Type({target}, {t})"),
        }
    }
}

/// Binding for a class.
#[derive(Clone, Debug)]
pub enum BindingClass {
    ClassDef(ClassBinding),
    FunctionalClassDef(
        ClassDefIndex,
        Identifier,
        SmallMap<Name, ClassFieldProperties>,
    ),
}

impl DisplayWith<Bindings> for BindingClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &Bindings) -> fmt::Result {
        match self {
            Self::ClassDef(c) => write!(f, "ClassDef({})", c.def.name),
            Self::FunctionalClassDef(_, id, _) => write!(f, "FunctionalClassDef({id})"),
        }
    }
}

/// Binding for a class.
#[derive(Clone, Debug)]
pub struct BindingTParams {
    pub name: Identifier,
    pub scoped_type_params: Option<Box<TypeParams>>,
    pub bases: Box<[Expr]>,
    pub legacy_tparams: Box<[Idx<KeyLegacyTypeParam>]>,
}

impl DisplayWith<Bindings> for BindingTParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _: &Bindings) -> fmt::Result {
        write!(f, "BindingTParams({})", self.name)
    }
}

/// Represents everything we know about a class field definition at binding time.
#[derive(Clone, Debug)]
pub enum ClassFieldDefinition {
    /// Declared by an annotation, with no assignment
    DeclaredByAnnotation { annotation: Idx<KeyAnnotation> },
    /// Declared with no annotation or assignment (this is impossible
    /// in a normal class, but can happen with some synthesized classes).
    DeclaredWithoutAnnotation,
    /// Defined via assignment, possibly with an annotation
    AssignedInBody {
        value: ExprOrBinding,
        annotation: Option<Idx<KeyAnnotation>>,
    },
    /// Defined by a `def` form. Because of decorators it may not
    /// actually *be* a method, hence the name `MethodLike`.
    MethodLike {
        definition: Idx<Key>,
        has_return_annotation: bool,
    },
    /// Defined in some way other than assignment or a `def` form,
    /// for example a name imported into a class body.
    DefinedWithoutAssign { definition: Idx<Key> },
    /// Implicitly defined in a method, without any explicit reference
    /// in the class body.
    DefinedInMethod {
        value: ExprOrBinding,
        annotation: Option<Idx<KeyAnnotation>>,
        method: MethodThatSetsAttr,
    },
}

impl DisplayWith<Bindings> for ClassFieldDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        match self {
            Self::DeclaredByAnnotation { annotation } => {
                write!(
                    f,
                    "ClassFieldDefinition::DeclaredByAnnotation({})",
                    ctx.display(*annotation),
                )
            }
            Self::DeclaredWithoutAnnotation => {
                write!(f, "ClassFieldDefinition::DeclaredWithoutAnnotation",)
            }
            Self::AssignedInBody { value, .. } => {
                write!(
                    f,
                    "ClassFieldDefinition::AssignedInBody({}, ..)",
                    value.display_with(ctx),
                )
            }
            Self::MethodLike { definition, .. } => {
                write!(
                    f,
                    "ClassFieldDefinition::MethodLike({}, ..)",
                    ctx.display(*definition)
                )
            }
            Self::DefinedWithoutAssign { definition, .. } => {
                write!(
                    f,
                    "ClassFieldDefinition::DefinedWithoutAssign({})",
                    ctx.display(*definition),
                )
            }
            Self::DefinedInMethod { value, .. } => {
                write!(
                    f,
                    "ClassFieldDefinition::DefinedInMethod({}, ..)",
                    value.display_with(ctx),
                )
            }
        }
    }
}

/// Binding for a class field, which is any attribute (including methods) of a class defined in
/// either the class body or in method (like `__init__`) that we recognize as
/// defining instance attributes.
#[derive(Clone, Debug)]
pub struct BindingClassField {
    pub class_idx: Idx<KeyClass>,
    pub name: Name,
    pub range: TextRange,
    pub definition: ClassFieldDefinition,
}

impl DisplayWith<Bindings> for BindingClassField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(
            f,
            "BindingClassField({}, {}, {})",
            ctx.display(self.class_idx),
            self.name,
            self.definition.display_with(ctx),
        )
    }
}

/// The method where an attribute was defined implicitly by assignment to `self.<attr_name>`
///
/// We track whether this method is recognized as a valid attribute-defining
/// method (e.g. a constructor); if an attribute is inferred only from assignments
/// in non-recognized methods, we will infer its type but also produce a type error.
#[derive(Clone, Debug)]
pub struct MethodThatSetsAttr {
    pub method_name: Name,
    pub recognized_attribute_defining_method: bool,
}

/// Bindings for fields synthesized by a class, such as a dataclass's `__init__` method. This
/// has to be its own key/binding type because of the dependencies between the various pieces of
/// information about a class: ClassDef -> ClassMetadata -> ClassField -> ClassSynthesizedFields.
#[derive(Clone, Debug)]
pub struct BindingClassSynthesizedFields(pub Idx<KeyClass>);

impl DisplayWith<Bindings> for BindingClassSynthesizedFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(f, "BindingClassSynthesizedFields({})", ctx.display(self.0))
    }
}

#[derive(Clone, Debug)]
pub struct BindingVariance {
    pub class_key: Idx<KeyClass>,
}

impl DisplayWith<Bindings> for BindingVariance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(f, "BindingVariance({})", ctx.display(self.class_key))
    }
}

/// Binding for the class's metadata (anything obtained directly from base classes,
/// except for the MRO which is kept separate to avoid cycles.
#[derive(Clone, Debug)]
pub struct BindingClassMetadata {
    pub class_idx: Idx<KeyClass>,
    /// The base class list, as expressions.
    pub bases: Box<[Expr]>,
    /// The class keywords (these are keyword args that appear in the base class list, the
    /// Python runtime will dispatch most of them to the metaclass, but the metaclass
    /// itself can also potentially be one of these).
    pub keywords: Box<[(Name, Expr)]>,
    /// The class decorators.
    pub decorators: Box<[(Idx<Key>, TextRange)]>,
    /// Is this a new type? True only for synthesized classes created from a `NewType` call.
    pub is_new_type: bool,
    /// May contain a base class to directly inject into the base class list. This is needed
    /// for some synthesized classes, which have no actual class body and therefore usually have no
    /// base class expressions, but may have a known base class for the synthesized class.
    pub special_base: Option<Box<BaseClass>>,
}

impl DisplayWith<Bindings> for BindingClassMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(
            f,
            "BindingClassMetadata({}, ..)",
            ctx.display(self.class_idx)
        )
    }
}

/// Binding for the class's MRO
/// This rerquires base classes; these should match what `BindingClassMetadata` has.
#[derive(Clone, Debug)]
pub struct BindingClassMro {
    pub class_idx: Idx<KeyClass>,
}

impl DisplayWith<Bindings> for BindingClassMro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(f, "BindingClassMro({}, ..)", ctx.display(self.class_idx))
    }
}

#[derive(Clone, Debug)]
pub struct BindingLegacyTypeParam(pub Idx<Key>);

impl DisplayWith<Bindings> for BindingLegacyTypeParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        write!(f, "BindingLegacyTypeParam({})", ctx.display(self.0))
    }
}

#[derive(Clone, Debug)]
pub enum BindingYield {
    Yield(Option<Idx<KeyAnnotation>>, ExprYield),
    Invalid(ExprYield),
}

impl BindingYield {
    fn expr(&self) -> &ExprYield {
        match self {
            Self::Yield(_, x) => x,
            Self::Invalid(x) => x,
        }
    }
}

impl DisplayWith<Bindings> for BindingYield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        let m = ctx.module();
        write!(f, "BindingYield({})", m.display(&self.expr()))
    }
}

#[derive(Clone, Debug)]
pub enum BindingYieldFrom {
    YieldFrom(Option<Idx<KeyAnnotation>>, ExprYieldFrom),
    Invalid(ExprYieldFrom),
}

impl BindingYieldFrom {
    fn expr(&self) -> &ExprYieldFrom {
        match self {
            Self::YieldFrom(_, x) => x,
            Self::Invalid(x) => x,
        }
    }
}

impl DisplayWith<Bindings> for BindingYieldFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &Bindings) -> fmt::Result {
        let m = ctx.module();
        write!(f, "BindingYieldFrom({})", m.display(&self.expr()))
    }
}
