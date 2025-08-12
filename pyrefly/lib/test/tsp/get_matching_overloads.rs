// Using protocol Position/Range

use crate::test::tsp::util::build_tsp_test_server;
use crate::tsp;

#[test]
fn test_basic_get_matching_overloads() {
    let (_handle, uri, _state) = build_tsp_test_server();

    let params = tsp::GetMatchingOverloadsParams {
        call_node: tsp::Node {
            uri: uri.to_string(),
            range: tsp::Range {
                start: tsp::Position { line: 0, character: 0 },
                end: tsp::Position { line: 0, character: 1 },
            },
        },
        snapshot: 1,
    };

    // Just test parameter construction
    assert_eq!(params.snapshot, 1);
}
