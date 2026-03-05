/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Converts pyrefly's internal `Type` representation to TSP protocol types.
//!
//! The conversion maps pyrefly types to their closest TSP protocol equivalent:
//!  - `ClassType` → TSP `ClassType` with a `RegularDeclaration` pointing to
//!    the class definition in source (or bundled typeshed).
//!  - `ClassDef` → TSP `ClassType` with `Instantiable` flag.
//!  - `Function` → TSP `FunctionType` with declaration info.
//!  - `BoundMethod` → TSP `FunctionType` with `bound_to_type`.
//!  - `Literal` → TSP `ClassType` with `literal_value`.
//!  - `Union` → TSP `UnionType` (recursively converting members).
//!  - `Module` → TSP `ModuleType`.
//!  - `Any`, `Never`, `None`, `Ellipsis` → TSP `BuiltInType`.
//!  - Everything else → TSP `SynthesizedType` as a fallback.

use std::collections::HashSet;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;

use lsp_types::Url;
use pyrefly_types::callable::Callable;
use pyrefly_types::callable::FunctionKind;
use pyrefly_types::callable::Params;
use pyrefly_types::class::Class;
use pyrefly_types::class::ClassType as PyreflyClassType;
use pyrefly_types::literal::Lit;
use pyrefly_types::types::BoundMethodType;
use pyrefly_types::types::Type as PyreflyType;
use tsp_types::BuiltInType;
use tsp_types::ClassType as TspClassType;
use tsp_types::Declaration;
use tsp_types::DeclarationCategory;
use tsp_types::DeclarationKind;
use tsp_types::LiteralValue;
use tsp_types::ModuleType as TspModuleType;
use tsp_types::Node;
use tsp_types::OverloadedType as TspOverloadedType;
use tsp_types::Position as TspPosition;
use tsp_types::Range as TspRange;
use tsp_types::RegularDeclaration;
use tsp_types::SynthesizedType;
use tsp_types::SynthesizedTypeMetadata;
use tsp_types::Type as TspType;
use tsp_types::TypeFlags;
use tsp_types::TypeKind;
use tsp_types::UnionType;

use crate::lsp::module_helpers::to_real_path;

/// Monotonic counter used to assign unique ids to converted types.
static NEXT_TYPE_ID: AtomicI32 = AtomicI32::new(1);

/// Generate a fresh, unique type id.
fn next_id() -> i32 {
    NEXT_TYPE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Convert a pyrefly `Type` to a TSP protocol `Type`.
pub fn convert_type(ty: &PyreflyType) -> TspType {
    let display = Some(ty.to_string());
    match ty {
        // --- Built-in special types ---
        PyreflyType::Any(_) => builtin("any"),
        PyreflyType::Never(_) => builtin("never"),
        PyreflyType::None => builtin("none"),
        PyreflyType::Ellipsis => builtin("ellipsis"),

        // --- Class instances (int, str, list[int], user-defined classes, etc.) ---
        PyreflyType::ClassType(ct) => convert_class_type(ct, TypeFlags::INSTANCE, display),

        // --- Class definitions (the class object itself, e.g. `type[int]`) ---
        PyreflyType::ClassDef(cls) => convert_class_def(cls, display),

        // --- Literals (Literal[42], Literal["hi"], etc.) ---
        PyreflyType::Literal(lit) => convert_literal(lit, display),

        // --- Functions ---
        PyreflyType::Function(func) => {
            convert_function(&func.signature, &func.metadata.kind, display)
        }

        // --- Bound methods ---
        PyreflyType::BoundMethod(bm) => {
            match &bm.func {
                BoundMethodType::Function(f) => {
                    convert_function(&f.signature, &f.metadata.kind, display)
                }
                BoundMethodType::Forall(f) => {
                    convert_function(&f.body.signature, &f.body.metadata.kind, display)
                }
                BoundMethodType::Overload(_) => synthesized(ty),
            }
        }

        // --- Unions ---
        PyreflyType::Union(u) => {
            let sub_types: Vec<TspType> = u.members.iter().map(convert_type).collect();
            TspType::Union(UnionType {
                display,
                flags: TypeFlags::NONE,
                id: next_id(),
                kind: TypeKind::Union,
                sub_types,
                type_alias_info: None,
            })
        }

        // --- Modules ---
        PyreflyType::Module(m) => TspType::Module(TspModuleType {
            display,
            flags: TypeFlags::NONE,
            id: next_id(),
            kind: TypeKind::Module,
            module_name: m.to_string(),
            type_alias_info: None,
            uri: String::new(),
        }),

        // --- TypedDicts are instances of their class, not the class itself ---
        PyreflyType::TypedDict(td) | PyreflyType::PartialTypedDict(td) => {
            if let pyrefly_types::typed_dict::TypedDict::TypedDict(inner) = td {
                let cls = inner.class_object();
                let declaration = make_class_declaration(cls);
                TspType::Class(TspClassType {
                    declaration: Declaration::Regular(declaration),
                    display,
                    flags: TypeFlags::INSTANCE,
                    id: next_id(),
                    kind: TypeKind::Class,
                    literal_value: None,
                    type_alias_info: None,
                    type_args: None,
                })
            } else {
                synthesized(ty)
            }
        }

        // --- Overloaded functions ---
        // Each overload is converted separately as a SynthesizedType. Pylance's
        // type provider resolves each SynthesizedType independently. All stubs
        // share the same `fileInfo.moduleName` (__stub__), so printType won't
        // module-qualify identically-named classes from different stubs.
        PyreflyType::Overload(overload) => {
            let overloads: Vec<TspType> = overload
                .signatures
                .iter()
                .map(|sig| {
                    match sig {
                        pyrefly_types::types::OverloadType::Function(f) => {
                            convert_function(&f.signature, &f.metadata.kind, None)
                        }
                        pyrefly_types::types::OverloadType::Forall(f) => {
                            convert_function(&f.body.signature, &f.body.metadata.kind, None)
                        }
                    }
                })
                .collect();
            TspType::Overloaded(TspOverloadedType {
                display,
                flags: TypeFlags::CALLABLE,
                id: next_id(),
                implementation: None,
                kind: TypeKind::Overloaded,
                overloads,
                type_alias_info: None,
            })
        }

        // --- Tuples become a class type for builtins.tuple ---
        PyreflyType::Tuple(_) => {
            // Tuples are structurally typed; emit as SynthesizedType with
            // the display string so the client sees "tuple[int, str]" etc.
            synthesized(ty)
        }

        // --- type[X] wrapper ---
        PyreflyType::Type(inner) => {
            // Wrap the inner type with Instantiable flag
            let inner_tsp = convert_type(inner);
            // Return the inner type but mark it as instantiable
            match inner_tsp {
                TspType::Class(mut c) => {
                    c.flags = TypeFlags::INSTANTIABLE;
                    // Override display to show type[X]
                    c.display = display;
                    TspType::Class(c)
                }
                other => other,
            }
        }

        // --- SelfType is a class type ---
        PyreflyType::SelfType(ct) => convert_class_type(ct, TypeFlags::INSTANCE, display),

        // --- Fallback: emit a SynthesizedType with the Display string ---
        _other => synthesized(ty),
    }
}

/// Convert a pyrefly `ClassType` (an instantiated class) to a TSP `ClassType`.
fn convert_class_type(ct: &PyreflyClassType, flags: u32, display: Option<String>) -> TspType {
    let cls = ct.class_object();
    let declaration = make_class_declaration(cls);
    let type_args: Option<Vec<TspType>> = {
        let args = ct.targs();
        let slice = args.as_slice();
        if slice.is_empty() {
            None
        } else {
            Some(slice.iter().map(convert_type).collect())
        }
    };

    TspType::Class(TspClassType {
        declaration: Declaration::Regular(declaration),
        display,
        flags,
        id: next_id(),
        kind: TypeKind::Class,
        literal_value: None,
        type_alias_info: None,
        type_args,
    })
}

/// Convert a pyrefly `Class` (class definition object) to a TSP `ClassType`
/// with the `Instantiable` flag.
fn convert_class_def(cls: &Class, display: Option<String>) -> TspType {
    let declaration = make_class_declaration(cls);

    TspType::Class(TspClassType {
        declaration: Declaration::Regular(declaration),
        display,
        flags: TypeFlags::INSTANTIABLE,
        id: next_id(),
        kind: TypeKind::Class,
        literal_value: None,
        type_alias_info: None,
        type_args: None,
    })
}

/// Convert a pyrefly `Literal` to a TSP `ClassType` with `literal_value`.
fn convert_literal(lit: &pyrefly_types::literal::Literal, display: Option<String>) -> TspType {
    match &lit.value {
        Lit::Enum(e) => {
            // For enum literals, use the enum class as the declaration source
            let cls = e.class.class_object();
            let declaration = make_class_declaration(cls);
            TspType::Class(TspClassType {
                declaration: Declaration::Regular(declaration),
                display,
                flags: TypeFlags::LITERAL,
                id: next_id(),
                kind: TypeKind::Class,
                literal_value: None,
                type_alias_info: None,
                type_args: None,
            })
        }
        other => {
            let literal_value = match other {
                Lit::Int(i) => Some(LiteralValue::Int(i.as_i64().unwrap_or(0) as i32)),
                Lit::Bool(b) => Some(LiteralValue::Bool(*b)),
                Lit::Str(s) => Some(LiteralValue::String(s.to_string())),
                Lit::Bytes(_) | Lit::Enum(_) => None,
            };
            if let Some(lv) = literal_value {
                // Use a RegularDeclaration with no name so that Pylance's
                // `fromProtocolDecl` returns undefined, triggering the
                // display-based fallback (`buildTypeFromDisplay("Literal[1]")`).
                // Use INSTANCE flags (not LITERAL) to avoid Pylance's
                // `applyTypeFlags` replacing the real literal value with a
                // SentinelLiteral placeholder.
                let dummy_node = Node {
                    range: TspRange {
                        start: TspPosition {
                            line: 0,
                            character: 0,
                        },
                        end: TspPosition {
                            line: 0,
                            character: 0,
                        },
                    },
                    uri: String::new(),
                };
                TspType::Class(TspClassType {
                    declaration: Declaration::Regular(RegularDeclaration {
                        kind: DeclarationKind::Regular,
                        category: DeclarationCategory::Variable,
                        name: None,
                        node: dummy_node,
                    }),
                    display,
                    flags: TypeFlags::INSTANCE,
                    id: next_id(),
                    kind: TypeKind::Class,
                    literal_value: Some(lv),
                    type_alias_info: None,
                    type_args: None,
                })
            } else {
                // Bytes or other literals without a simple TSP representation
                synthesized(&PyreflyType::Literal(Box::new(lit.clone())))
            }
        }
    }
}

/// Convert a pyrefly function to a TSP `SynthesizedType` with stub content.
///
/// Functions don't carry a TextRange in their FuncId, so we can't point
/// Pylance at the original source declaration. Instead, we generate a
/// self-contained Python stub that Pylance's SynthesizedType handler can
/// parse, bind, and evaluate to reconstruct the function type.
fn convert_function(
    callable: &Callable,
    kind: &FunctionKind,
    display: Option<String>,
) -> TspType {
    if let FunctionKind::Def(func_id) = kind {
        let func_name = func_id.name.as_str();
        let (stub_content, offset) = generate_function_stub(callable, func_name);

        let module = &func_id.module;
        let module_path = module.path();
        let module_uri = path_to_uri(module_path);
        let module_name = module.name().to_string();

        TspType::Synthesized(SynthesizedType {
            display,
            flags: TypeFlags::CALLABLE,
            id: next_id(),
            kind: TypeKind::Synthesized,
            metadata: SynthesizedTypeMetadata {
                module: TspModuleType {
                    display: None,
                    flags: TypeFlags::NONE,
                    id: next_id(),
                    kind: TypeKind::Module,
                    module_name,
                    type_alias_info: None,
                    uri: module_uri,
                },
                primary_definition_offset: offset,
            },
            stub_content,
            type_alias_info: None,
        })
    } else {
        // Non-Def function kinds (IsInstance, Cast, etc.) — use synthesized
        TspType::Synthesized(SynthesizedType {
            display: None,
            flags: TypeFlags::CALLABLE,
            id: next_id(),
            kind: TypeKind::Synthesized,
            metadata: empty_synthesized_metadata(),
            stub_content: format!("{kind:?}"),
            type_alias_info: None,
        })
    }
}
/// Collect type names from a pyrefly `Type` that need local `class X: pass`
/// declarations in stub files. Stub `.pyi` files created by `addStubCode`
/// are isolated and don't have access to builtins or external modules, so
/// ALL referenced type names must be declared locally.
fn collect_type_names(ty: &PyreflyType, names: &mut HashSet<String>) {
    match ty {
        PyreflyType::ClassType(ct) => {
            let name = ct.class_object().name().to_string();
            names.insert(name);
            // Also recurse into type args
            for arg in ct.targs().as_slice() {
                collect_type_names(arg, names);
            }
        }
        PyreflyType::ClassDef(cls) => {
            let name = cls.name().to_string();
            names.insert(name);
        }
        PyreflyType::Quantified(q) | PyreflyType::QuantifiedValue(q) => {
            let name = q.name.to_string();
            names.insert(name);
        }
        PyreflyType::Union(u) => {
            for member in &u.members {
                collect_type_names(member, names);
            }
        }
        PyreflyType::Type(inner) => {
            collect_type_names(inner, names);
        }
        PyreflyType::Tuple(t) => match t {
            pyrefly_types::tuple::Tuple::Concrete(elts) => {
                for elem in elts {
                    collect_type_names(&elem, names);
                }
            }
            pyrefly_types::tuple::Tuple::Unbounded(elem) => {
                collect_type_names(elem.as_ref(), names);
            }
            pyrefly_types::tuple::Tuple::Unpacked(parts) => {
                for elem in &parts.0 {
                    collect_type_names(elem, names);
                }
                collect_type_names(&parts.1, names);
                for elem in &parts.2 {
                    collect_type_names(elem, names);
                }
            }
        },
        _ => {}
    }
}

/// Collect type names from a `Callable` (params + return type) for local
/// stub declarations. Delegates to `collect_type_names` for each sub-type.
fn collect_callable_type_names(callable: &Callable, names: &mut HashSet<String>) {
    match &callable.params {
        Params::List(params) => {
            for param in params.items() {
                collect_type_names(param.as_type(), names);
            }
        }
        Params::ParamSpec(prefix, _) => {
            for (ty, _) in prefix.iter() {
                collect_type_names(ty, names);
            }
        }
        Params::Ellipsis | Params::Materialization => {}
    }
    collect_type_names(&callable.ret, names);
}

/// Generate stub content for a function from its `Callable` signature.
///
/// The stub contains:
/// 1. `class X: pass` declarations for each type referenced in the
///    function signature (stub files are isolated so all names must be local).
/// 2. A `def func_name(params) -> ret: ...` definition.
///
/// Returns `(stub_content, primary_definition_offset)`.
fn generate_function_stub(callable: &Callable, func_name: &str) -> (String, i32) {
    let mut names = HashSet::new();
    collect_callable_type_names(callable, &mut names);

    // Build preamble with class declarations for ALL referenced type names.
    // Stub .pyi files are isolated and can't resolve builtins or imports, so
    // every name must be locally defined. Using `class X: pass` is a
    // simplification — the evaluator only needs the name for round-trips.
    let mut stub = String::new();
    let mut sorted_names: Vec<&String> = names.iter().collect();
    sorted_names.sort(); // deterministic order
    for name in sorted_names {
        stub.push_str(&format!("class {name}:\n    pass\n"));
    }

    // Record offset before function definition
    let offset = stub.len() as i32;

    // Generate function definition using the Callable's Display.
    // Callable::Display produces "(x: A) -> A" etc., so we prefix with "def name".
    stub.push_str(&format!("def {func_name}{callable}: ...\n"));

    (stub, offset)
}

/// Build a `RegularDeclaration` from a pyrefly `Class`.
fn make_class_declaration(cls: &Class) -> RegularDeclaration {
    let qname = cls.qname();
    let module = qname.module();
    let module_path = qname.module_path();
    let range = qname.range();

    let lsp_range = module.to_lsp_range(range);
    let uri = path_to_uri(module_path);

    RegularDeclaration {
        category: DeclarationCategory::Class,
        kind: DeclarationKind::Regular,
        name: Some(cls.name().to_string()),
        node: Node {
            range: lsp_range_to_tsp(lsp_range),
            uri,
        },
    }
}

/// Convert a `ModulePath` to a URI string, handling bundled typeshed paths.
fn path_to_uri(module_path: &pyrefly_python::module_path::ModulePath) -> String {
    if let Some(real_path) = to_real_path(module_path) {
        Url::from_file_path(&real_path)
            .map_or_else(|()| real_path.to_string_lossy().to_string(), |u| u.to_string())
    } else {
        // Fallback for paths that can't be materialized
        module_path.as_path().to_string_lossy().to_string()
    }
}

/// Convert an `lsp_types::Range` to a TSP `Range`.
fn lsp_range_to_tsp(r: lsp_types::Range) -> TspRange {
    TspRange {
        start: tsp_types::Position {
            line: r.start.line,
            character: r.start.character,
        },
        end: tsp_types::Position {
            line: r.end.line,
            character: r.end.character,
        },
    }
}

/// Build a TSP `BuiltInType` with the given name.
fn builtin(name: &str) -> TspType {
    TspType::BuiltInType(BuiltInType {
        declaration: None,
        display: Some(name.to_owned()),
        flags: TypeFlags::NONE,
        id: next_id(),
        kind: TypeKind::Builtin,
        name: name.to_owned(),
        possible_type: None,
        type_alias_info: None,
    })
}

/// Build a TSP `SynthesizedType` whose stub content is the type's display
/// string.
fn synthesized(ty: &PyreflyType) -> TspType {
    let display = ty.to_string();
    TspType::Synthesized(SynthesizedType {
        display: Some(display.clone()),
        flags: TypeFlags::INSTANCE,
        id: next_id(),
        kind: TypeKind::Synthesized,
        metadata: empty_synthesized_metadata(),
        stub_content: display,
        type_alias_info: None,
    })
}

/// Placeholder metadata for synthesized types.
fn empty_synthesized_metadata() -> SynthesizedTypeMetadata {
    SynthesizedTypeMetadata {
        module: TspModuleType {
            display: None,
            flags: TypeFlags::NONE,
            id: 0,
            kind: TypeKind::Module,
            module_name: String::new(),
            type_alias_info: None,
            uri: String::new(),
        },
        primary_definition_offset: 0,
    }
}
