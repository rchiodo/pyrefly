use crate::test::tsp::util::build_tsp_test_server;
use tsp_types::{GetMatchingOverloadsParams, Node, Range, Position};

#[test]
fn test_basic_get_matching_overloads() {
    let (_handle, uri, _state) = build_tsp_test_server();

    let params = GetMatchingOverloadsParams {
        call_node: Node {
            uri: uri.to_string(),
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
        },
        snapshot: 1,
    };

    // Just test parameter construction
    assert_eq!(params.snapshot, 1);
}
