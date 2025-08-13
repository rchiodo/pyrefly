use crate::test::tsp::util::build_tsp_test_server;
use crate::test::tsp::util::extract_cursor_location;
use tsp_types::{GetReprParams, Type, TypeCategory, TypeFlags, TypeHandle, TypeReprFlags};

#[test]
fn test_basic_get_repr() {
    let (_handle, uri, _state) = build_tsp_test_server();

    let content = r#"
x = "hello world"
# ^
print(x)
"#;

    let _position = extract_cursor_location(content, &uri);

    let params = GetReprParams {
        type_: Type {
            handle: TypeHandle::String("test".to_owned()),
            category: TypeCategory::Any,
            flags: TypeFlags::new(),
            module_name: None,
            name: "str".to_owned(),
            category_flags: 0,
            decl: None,
            alias_name: None,
        },
        flags: TypeReprFlags::NONE,
        snapshot: 1,
    };

    // Just test parameter construction
    assert_eq!(params.snapshot, 1);
}
