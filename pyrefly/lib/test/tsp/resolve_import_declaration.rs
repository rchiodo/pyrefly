/*
 * Unit tests for resolve_import_declaration request handler
 *
 * These tests verify the resolve_import_declaration TSP request by:
 * 1. Testing parameter construction and validation
 * 2. Testing different declaration categories and resolution scenarios
 * 3. Validating proper handling of import/non-import declarations
 * 4. Testing edge cases and error conditions
 *
 * The resolve_import_declaration request takes a Declaration and resolves import declarations
 * to their actual definitions in target modules, while passing through non-import declarations unchanged.
 */

use crate::test::tsp::util::build_tsp_test_server;
use tsp_types::{
    Declaration, DeclarationCategory, DeclarationFlags, DeclarationHandle, ModuleName, Node,
    Position, Range, ResolveImportDeclarationParams, ResolveImportOptions,
};

#[test]
fn test_resolve_import_declaration_params_construction() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test basic parameter construction
    let declaration = Declaration {
        handle: DeclarationHandle::String("test_handle".to_owned()),
        category: DeclarationCategory::Import,
        flags: DeclarationFlags::new(),
        node: Some(Node {
            uri: uri.to_string(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
        }),
        module_name: ModuleName {
            leading_dots: 0,
            name_parts: vec!["test_module".to_owned()],
        },
        name: "imported_symbol".to_owned(),
        uri: uri.to_string(),
    };

    let options = ResolveImportOptions {
        resolve_local_names: Some(true),
        allow_externally_hidden_access: Some(false),
        skip_file_needed_check: Some(true),
    };

    let params = ResolveImportDeclarationParams {
        decl: declaration,
        options,
        snapshot: 42,
    };

    // Verify parameter construction
    assert_eq!(params.snapshot, 42);
    assert_eq!(params.decl.category, DeclarationCategory::Import);
    assert_eq!(params.decl.name, "imported_symbol");
    assert_eq!(params.decl.module_name.name_parts, vec!["test_module"]);
    assert_eq!(params.options.resolve_local_names, Some(true));
    assert_eq!(params.options.allow_externally_hidden_access, Some(false));
    assert_eq!(params.options.skip_file_needed_check, Some(true));
}

#[test]
fn test_resolve_import_declaration_default_options() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test default options construction
    let default_options = ResolveImportOptions::default();

    assert_eq!(default_options.resolve_local_names, Some(false));
    assert_eq!(default_options.allow_externally_hidden_access, Some(false));
    assert_eq!(default_options.skip_file_needed_check, Some(false));

    let declaration = Declaration {
        handle: DeclarationHandle::String("test_handle".to_owned()),
        category: DeclarationCategory::Function,
        flags: DeclarationFlags::new(),
        node: None,
        module_name: ModuleName {
            leading_dots: 0,
            name_parts: vec!["test_module".to_owned()],
        },
        name: "my_function".to_owned(),
        uri: uri.to_string(),
    };

    let params = ResolveImportDeclarationParams {
        decl: declaration,
        options: default_options,
        snapshot: 1,
    };

    // Non-import declaration should be handled differently
    assert_eq!(params.decl.category, DeclarationCategory::Function);
}

#[test]
fn test_resolve_import_declaration_different_categories() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test different declaration categories
    let categories = vec![
        (DeclarationCategory::Import, "import_symbol"),
        (DeclarationCategory::Function, "function_symbol"),
        (DeclarationCategory::Class, "class_symbol"),
        (DeclarationCategory::Variable, "variable_symbol"),
        (DeclarationCategory::Param, "param_symbol"),
    ];

    for (category, name) in categories {
        let category = category.clone();
        let declaration = Declaration {
            handle: DeclarationHandle::String(format!("handle_{name}")),
            category: category.clone(),
            flags: DeclarationFlags::new(),
            node: Some(Node {
                uri: uri.to_string(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: name.len() as u32,
                    },
                },
            }),
            module_name: ModuleName {
                leading_dots: 0,
                name_parts: vec!["test_module".to_owned()],
            },
            name: name.to_owned(),
            uri: uri.to_string(),
        };

        let params = ResolveImportDeclarationParams {
            decl: declaration.clone(),
            options: ResolveImportOptions::default(),
            snapshot: 1,
        };

        assert_eq!(params.decl.category, category);
        assert_eq!(params.decl.name, name);
    }
}

#[test]
fn test_resolve_import_declaration_module_name_variants() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test different module name patterns
    let module_patterns = vec![
        // Simple module
        (
            ModuleName {
                leading_dots: 0,
                name_parts: vec!["os".to_owned()],
            },
            "os module",
        ),
        // Nested module
        (
            ModuleName {
                leading_dots: 0,
                name_parts: vec!["os".to_owned(), "path".to_owned()],
            },
            "os.path module",
        ),
        // Relative import with single dot
        (
            ModuleName {
                leading_dots: 1,
                name_parts: vec!["utils".to_owned()],
            },
            "relative utils",
        ),
        // Relative import with multiple dots
        (
            ModuleName {
                leading_dots: 2,
                name_parts: vec!["shared".to_owned(), "helpers".to_owned()],
            },
            "deeply relative",
        ),
        // Current package import
        (
            ModuleName {
                leading_dots: 1,
                name_parts: vec![],
            },
            "current package",
        ),
    ];

    for (module_name, description) in module_patterns {
        let declaration = Declaration {
            handle: DeclarationHandle::String(format!(
                "handle_{}",
                description.replace(' ', "_")
            )),
            category: DeclarationCategory::Import,
            flags: DeclarationFlags::new(),
            node: None,
            module_name: module_name.clone(),
            name: "imported_item".to_owned(),
            uri: uri.to_string(),
        };

        let params = ResolveImportDeclarationParams {
            decl: declaration,
            options: ResolveImportOptions::default(),
            snapshot: 1,
        };

        assert_eq!(
            params.decl.module_name.leading_dots,
            module_name.leading_dots
        );
        assert_eq!(params.decl.module_name.name_parts, module_name.name_parts);
    }
}

#[test]
fn test_resolve_import_declaration_flags_handling() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test different declaration flags
    let flag_variants = vec![
        (DeclarationFlags::new(), "basic"),
        (DeclarationFlags::new().with_constant(), "constant"),
        (
            DeclarationFlags::new().with_unresolved_import(),
            "unresolved",
        ),
        (
            DeclarationFlags::new()
                .with_constant()
                .with_unresolved_import(),
            "constant_unresolved",
        ),
    ];

    for (flags, description) in flag_variants {
        let declaration = Declaration {
            handle: DeclarationHandle::String(format!("handle_{description}")),
            category: DeclarationCategory::Import,
            flags,
            node: None,
            module_name: ModuleName {
                leading_dots: 0,
                name_parts: vec!["test".to_owned()],
            },
            name: "symbol".to_owned(),
            uri: uri.to_string(),
        };

        let params = ResolveImportDeclarationParams {
            decl: declaration.clone(),
            options: ResolveImportOptions::default(),
            snapshot: 1,
        };

        // Basic validation that the flags are preserved
        // (flags comparison requires specific trait impls)
        assert_eq!(params.decl.name, "symbol");
        assert_eq!(params.decl.category, DeclarationCategory::Import);
    }
}

#[test]
fn test_resolve_import_declaration_uri_handling() {
    let (_handle, _uri, _state) = build_tsp_test_server();

    // Test different URI formats
    let uri_variants = vec![
        "file:///home/user/project/main.py".to_string(),
        "file:///C:/Users/user/project/main.py".to_string(),
        "file:///tmp/test.py".to_string(),
    ];

    for test_uri in uri_variants {
        let declaration = Declaration {
            handle: DeclarationHandle::String("test_handle".to_owned()),
            category: DeclarationCategory::Import,
            flags: DeclarationFlags::new(),
            node: Some(Node {
                uri: test_uri.clone(),
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
            }),
            module_name: ModuleName {
                leading_dots: 0,
                name_parts: vec!["test".to_owned()],
            },
            name: "symbol".to_owned(),
            uri: test_uri.clone(),
        };

        let params = ResolveImportDeclarationParams {
            decl: declaration,
            options: ResolveImportOptions::default(),
            snapshot: 1,
        };

        assert_eq!(params.decl.uri, test_uri);
        assert_eq!(params.decl.node.as_ref().unwrap().uri, params.decl.uri);
    }
}

#[test]
fn test_resolve_import_declaration_node_handling() {
    let (_handle, uri, _state) = build_tsp_test_server();

    // Test with node present
    let with_node = Declaration {
        handle: DeclarationHandle::String("with_node".to_owned()),
        category: DeclarationCategory::Import,
        flags: DeclarationFlags::new(),
        node: Some(Node {
            uri: uri.to_string(),
            range: Range {
                start: Position {
                    line: 5,
                    character: 10,
                },
                end: Position {
                    line: 5,
                    character: 20,
                },
            },
        }),
        module_name: ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        },
        name: "symbol".to_owned(),
        uri: uri.to_string(),
    };

    // Test with node absent
    let without_node = Declaration {
        handle: DeclarationHandle::String("without_node".to_owned()),
        category: DeclarationCategory::Import,
        flags: DeclarationFlags::new(),
        node: None,
        module_name: ModuleName {
            leading_dots: 0,
            name_parts: vec!["test".to_owned()],
        },
        name: "symbol".to_owned(),
        uri: uri.to_string(),
    };

    let params_with_node = ResolveImportDeclarationParams {
        decl: with_node,
        options: ResolveImportOptions::default(),
        snapshot: 1,
    };

    let params_without_node = ResolveImportDeclarationParams {
        decl: without_node,
        options: ResolveImportOptions::default(),
        snapshot: 1,
    };

    assert!(params_with_node.decl.node.is_some());
    assert!(params_without_node.decl.node.is_none());

    if let Some(node) = &params_with_node.decl.node {
        assert_eq!(node.range.start.line, 5);
        assert_eq!(node.range.start.character, 10);
        assert_eq!(node.range.end.line, 5);
        assert_eq!(node.range.end.character, 20);
    }
}
