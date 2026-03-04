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

use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;

use lsp_types::Url;
use pyrefly_types::callable::FunctionKind;
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
use tsp_types::FunctionType as TspFunctionType;
use tsp_types::LiteralValue;
use tsp_types::ModuleType as TspModuleType;
use tsp_types::Node;
use tsp_types::Range as TspRange;
use tsp_types::RegularDeclaration;
use tsp_types::SynthesizedDeclaration;
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
    match ty {
        // --- Built-in special types ---
        PyreflyType::Any(_) => builtin("any"),
        PyreflyType::Never(_) => builtin("never"),
        PyreflyType::None => builtin("none"),
        PyreflyType::Ellipsis => builtin("ellipsis"),

        // --- Class instances (int, str, list[int], user-defined classes, etc.) ---
        PyreflyType::ClassType(ct) => convert_class_type(ct, TypeFlags::INSTANCE),

        // --- Class definitions (the class object itself, e.g. `type[int]`) ---
        PyreflyType::ClassDef(cls) => convert_class_def(cls),

        // --- Literals (Literal[42], Literal["hi"], etc.) ---
        PyreflyType::Literal(lit) => convert_literal(lit),

        // --- Functions ---
        PyreflyType::Function(func) => convert_function(&func.metadata.kind, None),

        // --- Bound methods ---
        PyreflyType::BoundMethod(bm) => {
            let bound_to = convert_type(&bm.obj);
            let func_kind = match &bm.func {
                BoundMethodType::Function(f) => Some(&f.metadata.kind),
                BoundMethodType::Forall(f) => Some(&f.body.metadata.kind),
                BoundMethodType::Overload(_) => None,
            };
            if let Some(kind) = func_kind {
                convert_function(kind, Some(bound_to))
            } else {
                synthesized(ty)
            }
        }

        // --- Unions ---
        PyreflyType::Union(u) => {
            let sub_types: Vec<TspType> = u.members.iter().map(convert_type).collect();
            TspType::Union(UnionType {
                flags: TypeFlags::NONE,
                id: next_id(),
                kind: TypeKind::Union,
                sub_types,
                type_alias_info: None,
            })
        }

        // --- Modules ---
        PyreflyType::Module(m) => TspType::Module(TspModuleType {
            flags: TypeFlags::NONE,
            id: next_id(),
            kind: TypeKind::Module,
            module_name: m.to_string(),
            type_alias_info: None,
            uri: String::new(),
        }),

        // --- TypedDicts are a special case of class types ---
        PyreflyType::TypedDict(td) | PyreflyType::PartialTypedDict(td) => {
            if let pyrefly_types::typed_dict::TypedDict::TypedDict(inner) = td {
                convert_class_def(inner.class_object())
            } else {
                synthesized(ty)
            }
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
                    TspType::Class(c)
                }
                other => other,
            }
        }

        // --- SelfType is a class type ---
        PyreflyType::SelfType(ct) => convert_class_type(ct, TypeFlags::INSTANCE),

        // --- Fallback: emit a SynthesizedType with the Display string ---
        _other => synthesized(ty),
    }
}

/// Convert a pyrefly `ClassType` (an instantiated class) to a TSP `ClassType`.
fn convert_class_type(ct: &PyreflyClassType, flags: u32) -> TspType {
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
fn convert_class_def(cls: &Class) -> TspType {
    let declaration = make_class_declaration(cls);

    TspType::Class(TspClassType {
        declaration: Declaration::Regular(declaration),
        flags: TypeFlags::INSTANTIABLE,
        id: next_id(),
        kind: TypeKind::Class,
        literal_value: None,
        type_alias_info: None,
        type_args: None,
    })
}

/// Convert a pyrefly `Literal` to a TSP `ClassType` with `literal_value`.
fn convert_literal(lit: &pyrefly_types::literal::Literal) -> TspType {
    match &lit.value {
        Lit::Enum(e) => {
            // For enum literals, use the enum class as the declaration source
            let cls = e.class.class_object();
            let declaration = make_class_declaration(cls);
            TspType::Class(TspClassType {
                declaration: Declaration::Regular(declaration),
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
                TspType::Class(TspClassType {
                    declaration: Declaration::Synthesized(SynthesizedDeclaration {
                        kind: DeclarationKind::Synthesized,
                        uri: "builtins".to_owned(),
                    }),
                    flags: TypeFlags::LITERAL,
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

/// Convert a pyrefly function to a TSP `FunctionType`.
fn convert_function(kind: &FunctionKind, bound_to_type: Option<TspType>) -> TspType {
    if let FunctionKind::Def(func_id) = kind {
        let module = &func_id.module;
        let module_path = module.path();
        let uri = path_to_uri(module_path);

        // Functions don't carry a TextRange in their FuncId, so we use a
        // SynthesizedDeclaration with the module URI. A future improvement
        // could look up the actual range from the module's AST.
        let declaration = Declaration::Synthesized(SynthesizedDeclaration {
            kind: DeclarationKind::Synthesized,
            uri: uri.clone(),
        });

        TspType::Function(TspFunctionType {
            bound_to_type: bound_to_type.map(Box::new),
            declaration,
            flags: TypeFlags::CALLABLE,
            id: next_id(),
            kind: TypeKind::Function,
            return_type: None, // Would require inspecting the Callable signature
            specialized_types: None,
            type_alias_info: None,
        })
    } else {
        // Non-Def function kinds (IsInstance, Cast, etc.) — use synthesized
        TspType::Synthesized(SynthesizedType {
            flags: TypeFlags::CALLABLE,
            id: next_id(),
            kind: TypeKind::Synthesized,
            metadata: empty_synthesized_metadata(),
            stub_content: format!("{kind:?}"),
            type_alias_info: None,
        })
    }
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
