use lsp_types::Position;
use lsp_types::Url;

use crate::test::tsp::util::build_tsp_test_server;
use crate::test::tsp::util::extract_cursor_location;
use crate::tsp;

#[test]
fn test_get_symbol_params_construction() {
    let (handle, uri, state) = build_tsp_test_server();
    let transaction = state.transaction();

    let content = r#"
def my_function():
#   ^
    pass

my_function()
"#;

    let position = extract_cursor_location(content, &uri);

    let params = tsp::GetSymbolParams {
        node: tsp::Node {
            uri: uri.clone(),
            range: lsp_types::Range {
                start: position,
                end: position,
            },
        },
        name: None,
        skip_unreachable_code: false,
        snapshot: 1,
    };

    // Just test that we can construct the parameters correctly
    assert_eq!(params.snapshot, 1);
    assert_eq!(params.skip_unreachable_code, false);
    assert!(params.name.is_none());
}
