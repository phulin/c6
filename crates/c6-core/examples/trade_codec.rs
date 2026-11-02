//! Read-only extracted-Graph comparison/benchmark. Inputs remain private.
use c6_core::trade::{Graph, Limits};
use std::{hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        return Err(
            "usage: trade_codec FIRST_GRAPH SECOND_GRAPH ITERATIONS [RANGE_OUTPUT_PREFIX]".into(),
        );
    }
    let a_bytes = std::fs::read(&args[1])?;
    let b_bytes = std::fs::read(&args[2])?;
    let iterations: usize = args[3].parse()?;
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let limits = Limits::default();
    let a = Graph::decode(&a_bytes, limits)?;
    let b = Graph::decode(&b_bytes, limits)?;
    assert_eq!(a.encode(limits)?, a_bytes);
    assert_eq!(b.encode(limits)?, b_bytes);
    assert!(a.logical_eq(&b));
    if let Some(prefix) = args.get(4) {
        // Reconstruct serialized Route boundaries from typed lengths, independently
        // checked against native serializer traces by trade_codec_check.py.
        for (index, graph) in [&a, &b].into_iter().enumerate() {
            let mut offset = 12;
            let mut ranges = Vec::new();
            for origin in graph.origins() {
                offset += 20;
                for destination in &origin.destinations {
                    offset += 24; // destination Vertex + Edge version and ID
                    let end = offset + destination.edge.most_recent_route.encode(limits)?.len();
                    ranges.push(format!("[{offset},{end}]"));
                    offset = end + 50 + 4 * destination.edge.most_recent_path.len();
                }
            }
            assert_eq!(offset, graph.encode(limits)?.len());
            std::fs::write(
                format!("{prefix}-{index}.json"),
                format!("[{}]", ranges.join(",")),
            )?;
        }
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(Graph::decode(black_box(&a_bytes), limits)?);
    }
    let decode_ns = start.elapsed().as_nanos() / iterations as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(a.encode(limits)?);
    }
    let encode_ns = start.elapsed().as_nanos() / iterations as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        for origin in a.origins() {
            for destination in &origin.destinations {
                assert!(black_box(
                    a.get_edge(black_box(origin.vertex), black_box(destination.vertex))
                )
                .is_some());
                assert!(black_box(a.get_edge_by_id(black_box(destination.edge.id))).is_some());
            }
        }
    }
    let lookups = iterations as u128 * a.edge_count() as u128 * 2;
    let lookup_ns = start.elapsed().as_nanos() / lookups.max(1);
    println!("{{\"origins\":{},\"edges\":{},\"total_edges_created\":{},\"graph_bytes\":{},\"exact_roundtrips\":2,\"logical_equal\":{},\"iteration_order_equal\":{},\"wire_state_equal\":{},\"iterations\":{},\"decode_ns\":{},\"encode_ns\":{},\"indexed_lookup_ns\":{}}}",
        a.origins().len(), a.edge_count(), a.total_edges_created(), a_bytes.len(),
        a.logical_eq(&b), a.iteration_order_eq(&b), a.wire_state_eq(&b), iterations, decode_ns, encode_ns, lookup_ns);
    Ok(())
}
