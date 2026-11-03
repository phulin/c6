//! Private-fixture fresh-container reconstruction check; no native execution.
use c6_core::trade::{self, Graph, Limits};
#[path = "../src/trade_read_order.rs"]
mod trade_read_order;
use trade_read_order::fresh_read_wire_order;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: trade_read_order INPUT_GRAPH EXPECTED_GRAPH OUTPUT_GRAPH".into());
    }
    let limits = Limits::default();
    let input = Graph::decode(&std::fs::read(&args[1])?, limits)?;
    let expected_bytes = std::fs::read(&args[2])?;
    let expected = Graph::decode(&expected_bytes, limits)?;
    let actual = fresh_read_wire_order(&input)?;
    assert!(actual.logical_eq(&input));
    let encoded = actual.encode(limits)?;
    std::fs::write(&args[3], &encoded)?;
    println!("{{\"origins\":{},\"edges\":{},\"logical_equal_input\":{},\"wire_equal_expected\":{},\"encoded_equal_expected\":{}}}", actual.origins().len(), actual.edge_count(), actual.logical_eq(&input), actual.wire_state_eq(&expected), encoded == expected_bytes);
    Ok(())
}
