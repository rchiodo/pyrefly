/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Display a type. The complexity comes from if we have two classes with the same name,
//! we want to display disambiguating information (e.g. module name or location).
use std::fmt;
use std::fmt::Display;

use pyrefly_python::module_name::ModuleName;
use pyrefly_util::display::Fmt;
use pyrefly_util::display::append;
use pyrefly_util::display::commas_iter;
use ruff_python_ast::name::Name;
use ruff_text_size::TextRange;
use starlark_map::small_map::Entry;
use starlark_map::small_map::SmallMap;
use starlark_map::smallmap;

use crate::callable::Function;
use crate::class::Class;
use crate::qname::QName;
use crate::tuple::Tuple;
use crate::types::AnyStyle;
use crate::types::BoundMethod;
use crate::types::Forall;
use crate::types::Forallable;
use crate::types::NeverStyle;
use crate::types::SuperObj;
use crate::types::TArgs;
use crate::types::TParam;
use crate::types::Type;

/// Information about the qnames we have seen.
/// Set to None to indicate we have seen different values, or Some if they are all the same.
#[derive(Clone, Debug)]
struct QNameInfo {
    /// For each module, record either the one unique range, or None if there are multiple.
    info: SmallMap<ModuleName, Option<TextRange>>,
}

impl QNameInfo {
    fn new(qname: &QName) -> Self {
        Self {
            info: smallmap! {qname.module_name() => Some(qname.range())},
        }
    }

    fn qualified() -> Self {
        Self {
            info: SmallMap::new(),
        }
    }

    fn update(&mut self, qname: &QName) {
        match self.info.entry(qname.module_name()) {
            Entry::Vacant(e) => {
                e.insert(Some(qname.range()));
            }
            Entry::Occupied(mut e) => {
                if e.get() != &Some(qname.range()) {
                    *e.get_mut() = None;
                }
            }
        }
    }

    fn fmt(&self, qname: &QName, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module_name = qname.module_name();
        match self.info.get(&module_name) {
            Some(None) | None => qname.fmt_with_location(f),
            _ if self.info.len() > 1 => qname.fmt_with_module(f),
            _ => qname.fmt_name(f),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeDisplayContext<'a> {
    qnames: SmallMap<&'a Name, QNameInfo>,
}

impl<'a> TypeDisplayContext<'a> {
    pub fn new(xs: &[&'a Type]) -> Self {
        let mut res = Self::default();
        for x in xs {
            res.add(x);
        }
        res
    }

    fn add_qname(&mut self, qname: &'a QName) {
        match self.qnames.entry(qname.id()) {
            Entry::Vacant(e) => {
                e.insert(QNameInfo::new(qname));
            }
            Entry::Occupied(mut e) => e.get_mut().update(qname),
        }
    }

    pub fn add(&mut self, t: &'a Type) {
        t.universe(&mut |t| {
            if let Some(qname) = t.qname() {
                self.add_qname(qname);
            }
        })
    }

    /// Force that we always display at least the module name for qualified names.
    pub fn always_display_module_name(&mut self) {
        // We pretend that every qname is also in a fake module, and thus requires disambiguating.
        let fake_module = ModuleName::from_str("__pyrefly__type__display__context__");
        for c in self.qnames.values_mut() {
            c.info.insert(fake_module, None);
        }
    }

    pub fn display(&'a self, t: &'a Type) -> impl Display + 'a {
        Fmt(|f| self.fmt(t, f))
    }

    fn fmt_targ(&self, param: &TParam, arg: &Type, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if param.quantified.is_type_var_tuple()
            && let Type::Tuple(tuple) = arg
        {
            match tuple {
                Tuple::Concrete(elts) if !elts.is_empty() => write!(
                    f,
                    "{}",
                    commas_iter(|| elts.iter().map(|elt| self.display(elt)))
                ),
                Tuple::Unpacked(box (prefix, middle, suffix)) => {
                    let unpacked_middle = Type::Unpack(Box::new(middle.clone()));
                    write!(
                        f,
                        "{}",
                        commas_iter(|| {
                            prefix
                                .iter()
                                .chain(std::iter::once(&unpacked_middle))
                                .chain(suffix.iter())
                                .map(|elt| self.display(elt))
                        })
                    )
                }
                _ => {
                    write!(f, "*{}", self.display(arg))
                }
            }
        } else {
            write!(f, "{}", self.display(arg))
        }
    }

    fn fmt_targs(&self, targs: &TArgs, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !targs.is_empty() {
            write!(
                f,
                "[{}]",
                commas_iter(|| targs
                    .iter_paired()
                    .map(|(param, arg)| Fmt(|f| self.fmt_targ(param, arg, f))))
            )
        } else {
            Ok(())
        }
    }

    fn fmt_qname(&self, qname: &QName, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.qnames.get(&qname.id()) {
            Some(info) => info.fmt(qname, f),
            None => QNameInfo::qualified().fmt(qname, f), // we should not get here, if we do, be safe
        }
    }

    fn fmt<'b>(&self, t: &'b Type, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match t {
            // Things that have QName's and need qualifying
            Type::ClassDef(cls) => {
                write!(f, "type[")?;
                self.fmt_qname(cls.qname(), f)?;
                write!(f, "]")
            }
            Type::ClassType(class_type)
                if class_type.qname().module_name().as_str() == "builtins"
                    && class_type.qname().id().as_str() == "tuple"
                    && class_type.targs().as_slice().len() == 1 =>
            {
                self.fmt_qname(class_type.qname(), f)?;
                write!(
                    f,
                    "[{}, ...]",
                    self.display(&class_type.targs().as_slice()[0])
                )
            }
            Type::ClassType(class_type) => {
                self.fmt_qname(class_type.qname(), f)?;
                self.fmt_targs(class_type.targs(), f)
            }
            Type::TypedDict(typed_dict) => {
                write!(f, "TypedDict[")?;
                self.fmt_qname(typed_dict.qname(), f)?;
                self.fmt_targs(typed_dict.targs(), f)?;
                write!(f, "]")
            }
            Type::PartialTypedDict(typed_dict) => {
                write!(f, "Partial[")?;
                self.fmt_qname(typed_dict.qname(), f)?;
                self.fmt_targs(typed_dict.targs(), f)?;
                write!(f, "]")
            }
            Type::TypeVar(t) => {
                write!(f, "TypeVar[")?;
                self.fmt_qname(t.qname(), f)?;
                write!(f, "]")
            }
            Type::TypeVarTuple(t) => {
                write!(f, "TypeVarTuple[")?;
                self.fmt_qname(t.qname(), f)?;
                write!(f, "]")
            }
            Type::ParamSpec(t) => {
                write!(f, "ParamSpec[")?;
                self.fmt_qname(t.qname(), f)?;
                write!(f, "]")
            }
            Type::SelfType(cls) => {
                write!(f, "Self@")?;
                self.fmt_qname(cls.qname(), f)
            }

            // Other things
            Type::Literal(lit) => write!(f, "Literal[{lit}]"),
            Type::LiteralString => write!(f, "LiteralString"),
            Type::Callable(box c)
            | Type::Function(box Function {
                signature: c,
                metadata: _,
            }) => c.fmt_with_type(f, &|t| self.display(t)),
            Type::Overload(overload) => {
                write!(
                    f,
                    "Overload[{}",
                    self.display(&overload.signatures.first().as_type())
                )?;
                for sig in overload.signatures.iter().skip(1) {
                    write!(f, ", {}", self.display(&sig.as_type()))?;
                }
                write!(f, "]")
            }
            Type::ParamSpecValue(x) => {
                write!(f, "[")?;
                x.fmt_with_type(f, &|t| self.display(t))?;
                write!(f, "]")
            }
            Type::BoundMethod(box BoundMethod { obj, func }) => {
                write!(
                    f,
                    "BoundMethod[{}, {}]",
                    self.display(obj),
                    self.display(&func.as_type())
                )
            }
            Type::Never(NeverStyle::NoReturn) => write!(f, "NoReturn"),
            Type::Never(NeverStyle::Never) => write!(f, "Never"),
            Type::Union(types) if types.is_empty() => write!(f, "Never"),
            Type::Union(types) => {
                // All Literals will be collected into a single Literal at the index of the first Literal.
                let mut literal_idx = None;
                let mut literals = Vec::new();
                let mut display_types = Vec::new();
                for (i, t) in types.iter().enumerate() {
                    match t {
                        Type::Literal(lit) => {
                            if literal_idx.is_none() {
                                literal_idx = Some(i);
                            }
                            literals.push(lit)
                        }
                        Type::Callable(_) | Type::Function(_) => {
                            display_types.push(format!("({})", self.display(t)))
                        }
                        _ => display_types.push(format!("{}", self.display(t))),
                    }
                }
                if let Some(i) = literal_idx {
                    display_types.insert(i, format!("Literal[{}]", commas_iter(|| &literals)));
                }
                write!(f, "{}", display_types.join(" | "))
            }
            Type::Intersect(types) => {
                write!(
                    f,
                    "Intersect[{}]",
                    commas_iter(|| types.iter().map(|t| self.display(t)))
                )
            }
            Type::Tuple(t) => t.fmt_with_type(f, |t| self.display(t)),
            Type::Forall(box Forall {
                tparams,
                body: body @ Forallable::Function(_),
            }) => {
                write!(
                    f,
                    "[{}]{}",
                    commas_iter(|| tparams.iter()),
                    self.display(&body.clone().as_type()),
                )
            }
            Type::Forall(box Forall {
                tparams,
                body: Forallable::TypeAlias(ta),
            }) => ta.fmt_with_type(f, &|t| self.display(t), Some(tparams)),
            Type::Type(ty) => write!(f, "type[{}]", self.display(ty)),
            Type::TypeGuard(ty) => write!(f, "TypeGuard[{}]", self.display(ty)),
            Type::TypeIs(ty) => write!(f, "TypeIs[{}]", self.display(ty)),
            Type::Unpack(box ty @ Type::TypedDict(_)) => write!(f, "Unpack[{}]", self.display(ty)),
            Type::Unpack(ty) => write!(f, "*{}", self.display(ty)),
            Type::Concatenate(args, pspec) => write!(
                f,
                "Concatenate[{}]",
                commas_iter(|| append(args.iter(), [pspec]))
            ),
            Type::Module(m) => write!(f, "Module[{m}]"),
            Type::Var(var) => write!(f, "{var}"),
            Type::Quantified(var) => write!(f, "{var}"),
            Type::Args(q) => {
                write!(f, "Args[{q}]")
            }
            Type::Kwargs(q) => {
                write!(f, "Kwargs[{q}]")
            }
            Type::SpecialForm(x) => write!(f, "{x}"),
            Type::Ellipsis => write!(f, "Ellipsis"),
            Type::Any(style) => match style {
                AnyStyle::Explicit => write!(f, "Any"),
                AnyStyle::Implicit | AnyStyle::Error => write!(f, "Unknown"),
            },
            Type::TypeAlias(ta) => ta.fmt_with_type(f, &|t| self.display(t), None),
            Type::SuperInstance(box (cls, obj)) => {
                write!(f, "super[")?;
                self.fmt_qname(cls.qname(), f)?;
                write!(f, ", ")?;
                match obj {
                    SuperObj::Instance(obj) => {
                        self.fmt_qname(obj.qname(), f)?;
                        self.fmt_targs(obj.targs(), f)?;
                    }
                    SuperObj::Class(obj) => {
                        self.fmt_qname(obj.qname(), f)?;
                    }
                }
                write!(f, "]")
            }
            Type::KwCall(call) => self.fmt(&call.return_ty, f),
            Type::None => write!(f, "None"),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        TypeDisplayContext::new(&[self]).fmt(self, f)
    }
}

pub struct ClassDisplayContext<'a>(TypeDisplayContext<'a>);

impl<'a> ClassDisplayContext<'a> {
    pub fn new(classes: &[&'a Class]) -> Self {
        let mut ctx = TypeDisplayContext::new(&[]);
        for cls in classes {
            ctx.add_qname(cls.qname());
        }
        Self(ctx)
    }

    pub fn display(&'a self, cls: &'a Class) -> impl Display + 'a {
        Fmt(|f| self.0.fmt_qname(cls.qname(), f))
    }
}

#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use dupe::Dupe;
    use pyrefly_python::module::Module;
    use pyrefly_python::module_path::ModulePath;
    use pyrefly_util::uniques::UniqueFactory;
    use ruff_python_ast::Identifier;
    use ruff_text_size::TextSize;

    use super::*;
    use crate::callable::Callable;
    use crate::callable::Param;
    use crate::callable::ParamList;
    use crate::callable::Required;
    use crate::class::Class;
    use crate::class::ClassDefIndex;
    use crate::class::ClassType;
    use crate::literal::Lit;
    use crate::quantified::Quantified;
    use crate::quantified::QuantifiedInfo;
    use crate::quantified::QuantifiedKind;
    use crate::tuple::Tuple;
    use crate::type_var::PreInferenceVariance;
    use crate::type_var::Restriction;
    use crate::type_var::TypeVar;
    use crate::typed_dict::TypedDict;
    use crate::types::TParam;
    use crate::types::TParams;

    pub fn fake_class(name: &str, module: &str, range: u32) -> Class {
        let mi = Module::new(
            ModuleName::from_str(module),
            ModulePath::filesystem(PathBuf::from(module)),
            Arc::new("1234567890".to_owned()),
        );

        Class::new(
            ClassDefIndex(0),
            Identifier::new(Name::new(name), TextRange::empty(TextSize::new(range))),
            mi,
            None,
            SmallMap::new(),
        )
    }

    pub fn fake_tparams(tparams: Vec<TParam>) -> Arc<TParams> {
        Arc::new(TParams::new(tparams))
    }

    fn fake_tparam(uniques: &UniqueFactory, name: &str, kind: QuantifiedKind) -> TParam {
        TParam {
            quantified: Quantified::new(
                uniques.fresh(),
                QuantifiedInfo {
                    name: Name::new(name),
                    kind,
                    restriction: Restriction::Unrestricted,
                    default: None,
                },
            ),
            variance: PreInferenceVariance::PInvariant,
        }
    }

    fn fake_tyvar(name: &str, module: &str, range: u32) -> TypeVar {
        let mi = Module::new(
            ModuleName::from_str(module),
            ModulePath::filesystem(PathBuf::from(module)),
            Arc::new("1234567890".to_owned()),
        );
        TypeVar::new(
            Identifier::new(Name::new(name), TextRange::empty(TextSize::new(range))),
            mi,
            Restriction::Unrestricted,
            None,
            PreInferenceVariance::PInvariant,
        )
    }

    #[test]
    fn test_display() {
        let uniques = UniqueFactory::new();
        let foo1 = fake_class("foo", "mod.ule", 5);
        let foo2 = fake_class("foo", "mod.ule", 8);
        let foo3 = fake_class("foo", "ule", 3);
        let bar = fake_class("bar", "mod.ule", 0);
        let bar_tparams = fake_tparams(vec![fake_tparam(&uniques, "T", QuantifiedKind::TypeVar)]);
        let tuple_param = fake_class("TupleParam", "mod.ule", 0);
        let tuple_param_tparams = fake_tparams(vec![fake_tparam(
            &uniques,
            "T",
            QuantifiedKind::TypeVarTuple,
        )]);
        fn class_type(class: &Class, targs: TArgs) -> Type {
            Type::ClassType(ClassType::new(class.dupe(), targs))
        }

        assert_eq!(
            class_type(
                &tuple_param,
                TArgs::new(
                    tuple_param_tparams.dupe(),
                    vec![Type::tuple(vec![
                        class_type(&foo1, TArgs::default()),
                        class_type(&foo1, TArgs::default())
                    ])]
                )
            )
            .to_string(),
            "TupleParam[foo, foo]"
        );
        assert_eq!(
            class_type(
                &tuple_param,
                TArgs::new(tuple_param_tparams.dupe(), vec![Type::tuple(Vec::new())])
            )
            .to_string(),
            "TupleParam[*tuple[()]]"
        );
        assert_eq!(
            class_type(
                &tuple_param,
                TArgs::new(
                    tuple_param_tparams.dupe(),
                    vec![Type::Tuple(Tuple::Unbounded(Box::new(class_type(
                        &foo1,
                        TArgs::default()
                    ))))]
                )
            )
            .to_string(),
            "TupleParam[*tuple[foo, ...]]"
        );
        assert_eq!(
            class_type(
                &tuple_param,
                TArgs::new(
                    tuple_param_tparams.dupe(),
                    vec![Type::Tuple(Tuple::Unpacked(Box::new((
                        vec![class_type(&foo1, TArgs::default())],
                        Type::Tuple(Tuple::Unbounded(Box::new(class_type(
                            &foo1,
                            TArgs::default(),
                        )))),
                        vec![class_type(&foo1, TArgs::default())],
                    ))))]
                )
            )
            .to_string(),
            "TupleParam[foo, *tuple[foo, ...], foo]"
        );

        assert_eq!(
            Type::Tuple(Tuple::unbounded(class_type(&foo1, TArgs::default()))).to_string(),
            "tuple[foo, ...]"
        );
        assert_eq!(
            Type::Tuple(Tuple::concrete(vec![
                class_type(&foo1, TArgs::default()),
                class_type(
                    &bar,
                    TArgs::new(
                        bar_tparams.dupe(),
                        vec![class_type(&foo1, TArgs::default())]
                    )
                )
            ]))
            .to_string(),
            "tuple[foo, bar[foo]]"
        );
        assert_eq!(
            Type::Tuple(Tuple::concrete(vec![
                class_type(&foo1, TArgs::default()),
                class_type(
                    &bar,
                    TArgs::new(
                        bar_tparams.dupe(),
                        vec![class_type(&foo2, TArgs::default())]
                    )
                )
            ]))
            .to_string(),
            "tuple[mod.ule.foo@1:6, bar[mod.ule.foo@1:9]]"
        );
        assert_eq!(
            Type::Tuple(Tuple::concrete(vec![
                class_type(&foo1, TArgs::default()),
                class_type(&foo3, TArgs::default())
            ]))
            .to_string(),
            "tuple[mod.ule.foo, ule.foo]"
        );
        assert_eq!(
            Type::Tuple(Tuple::concrete(vec![])).to_string(),
            "tuple[()]"
        );

        let t1 = class_type(&foo1, TArgs::default());
        let t2 = class_type(&foo2, TArgs::default());
        let ctx = TypeDisplayContext::new(&[&t1, &t2]);
        assert_eq!(
            format!("{} <: {}", ctx.display(&t1), ctx.display(&t2)),
            "mod.ule.foo@1:6 <: mod.ule.foo@1:9"
        );
    }

    #[test]
    fn test_display_qualified() {
        let c = fake_class("foo", "mod.ule", 5);
        let t = Type::ClassType(ClassType::new(c, TArgs::default()));
        let mut ctx = TypeDisplayContext::new(&[&t]);
        assert_eq!(ctx.display(&t).to_string(), "foo");

        ctx.always_display_module_name();
        assert_eq!(ctx.display(&t).to_string(), "mod.ule.foo");
    }

    #[test]
    fn test_display_typevar() {
        let t1 = fake_tyvar("foo", "bar", 1);
        let t2 = fake_tyvar("foo", "bar", 2);
        let t3 = fake_tyvar("qux", "bar", 2);

        assert_eq!(
            Type::Union(vec![t1.to_type(), t2.to_type()]).to_string(),
            "TypeVar[bar.foo@1:2] | TypeVar[bar.foo@1:3]"
        );
        assert_eq!(
            Type::Union(vec![t1.to_type(), t3.to_type()]).to_string(),
            "TypeVar[foo] | TypeVar[qux]"
        );
    }

    #[test]
    fn test_display_literal() {
        assert_eq!(Type::Literal(Lit::Bool(true)).to_string(), "Literal[True]");
        assert_eq!(
            Type::Literal(Lit::Bool(false)).to_string(),
            "Literal[False]"
        );
    }

    #[test]
    fn test_display_union() {
        let lit1 = Type::Literal(Lit::Bool(true));
        let lit2 = Type::Literal(Lit::Str("test".into()));
        let nonlit1 = Type::None;
        let nonlit2 = Type::LiteralString;

        assert_eq!(
            Type::Union(vec![nonlit1.clone(), nonlit2.clone()]).to_string(),
            "None | LiteralString"
        );
        assert_eq!(
            Type::Union(vec![nonlit1, lit1, nonlit2, lit2]).to_string(),
            "None | Literal[True, 'test'] | LiteralString"
        );
    }

    #[test]
    fn test_display_callable() {
        let param1 = Param::Pos(Name::new_static("hello"), Type::None, Required::Required);
        let param2 = Param::KwOnly(Name::new_static("world"), Type::None, Required::Required);
        let callable = Callable::list(ParamList::new(vec![param1, param2]), Type::None);
        assert_eq!(
            Type::Callable(Box::new(callable)).to_string(),
            "(hello: None, *, world: None) -> None"
        );
    }

    #[test]
    fn test_display_args_kwargs_callable() {
        let args = Param::VarArg(Some(Name::new_static("my_args")), Type::any_implicit());
        let kwargs = Param::Kwargs(Some(Name::new_static("my_kwargs")), Type::any_implicit());
        let callable = Callable::list(ParamList::new(vec![args, kwargs]), Type::None);
        assert_eq!(
            Type::Callable(Box::new(callable)).to_string(),
            "(*my_args: Unknown, **my_kwargs: Unknown) -> None"
        );
    }

    #[test]
    fn test_display_optional_parameter() {
        let param1 = Param::PosOnly(
            Some(Name::new_static("x")),
            Type::any_explicit(),
            Required::Optional(None),
        );
        let param2 = Param::Pos(
            Name::new_static("y"),
            Type::any_explicit(),
            Required::Optional(Some(Type::Literal(Lit::Bool(true)))),
        );
        let param3 = Param::Pos(
            Name::new_static("z"),
            Type::any_explicit(),
            Required::Optional(Some(Type::None)),
        );
        let callable = Callable::list(ParamList::new(vec![param1, param2, param3]), Type::None);
        assert_eq!(
            Type::Callable(Box::new(callable)).to_string(),
            "(x: Any = ..., /, y: Any = True, z: Any = None) -> None"
        );
    }

    #[test]
    fn test_posonly_parameter_only() {
        let param = Param::PosOnly(
            Some(Name::new_static("x")),
            Type::any_explicit(),
            Required::Required,
        );
        let callable = Callable::list(ParamList::new(vec![param]), Type::None);
        assert_eq!(
            Type::Callable(Box::new(callable)).to_string(),
            "(x: Any, /) -> None"
        );
    }

    #[test]
    fn test_anon_posonly_parameters() {
        let param1 = Param::PosOnly(None, Type::any_explicit(), Required::Required);
        let param2 = Param::PosOnly(None, Type::any_explicit(), Required::Optional(None));
        let callable = Callable::list(ParamList::new(vec![param1, param2]), Type::None);
        assert_eq!(
            Type::Callable(Box::new(callable)).to_string(),
            "(Any, _: Any = ...) -> None"
        );
    }

    #[test]
    fn test_display_generic_typeddict() {
        let uniques = UniqueFactory::new();
        let cls = fake_class("C", "test", 0);
        let tparams = fake_tparams(vec![fake_tparam(&uniques, "T", QuantifiedKind::TypeVar)]);
        let t = Type::None;
        let targs = TArgs::new(tparams.dupe(), vec![t]);
        let td = TypedDict::new(cls, targs);
        assert_eq!(Type::TypedDict(td).to_string(), "TypedDict[C[None]]");
    }
}
