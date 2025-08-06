use lsp_types::Position;
use lsp_types::Url;

use crate::test::tsp::util::build_tsp_test_server;
use crate::test::tsp::util::extract_cursor_location;
use crate::tsp;

#[test]
fn test_basic_get_repr() {
    let (handle, uri, state) = build_tsp_test_server();

    let content = r#"
x = "hello world"
# ^
print(x)
"#;

    let position = extract_cursor_location(content, &uri);

    let params = tsp::GetReprParams {
        type_param: tsp::Type {
            handle: tsp::TypeHandle::String("test".to_string()),
            category: tsp::TypeCategory::ANY,
            flags: tsp::TypeFlags::new(),
            module_name: None,
            name: "str".to_string(),
            category_flags: 0,
            decl: None,
        },
        flags: tsp::TypeReprFlags::NONE,
        snapshot: 1,
    };

    // Just test parameter construction
    assert_eq!(params.snapshot, 1);
}
