//! Read-only codec validation and benchmark of an extracted native owner section.
use c6_core::requirements::{Limits, RequirementSystem};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err("usage: requirements_codec INPUT OUTPUT BOUNDARIES ITERATIONS".into());
    }
    let bytes = std::fs::read(&args[1])?;
    let iterations: usize = args[4].parse()?;
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let limits = Limits::default();
    let system = RequirementSystem::decode(&bytes, limits)?;
    for end in [0, 1, 4, 5, bytes.len() - 1] {
        assert!(RequirementSystem::decode(&bytes[..end], limits).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[0] = 4;
    assert!(RequirementSystem::decode(&invalid, limits).is_err());
    invalid[0] = 3;
    invalid[1..5].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(RequirementSystem::decode(&invalid, limits).is_err());
    assert!(RequirementSystem::decode(
        &bytes,
        Limits {
            max_records: system.records.len() - 1,
            ..limits
        }
    )
    .is_err());
    let encoded = system.encode(limits)?;
    assert_eq!(encoded, bytes);
    std::fs::write(&args[2], encoded)?;
    let mut offset = 5;
    let mut boundaries = String::new();
    for record in &system.records {
        let fields = offset + 4 + record.requirement_id.len();
        boundaries.push_str(&format!(
            "{offset},{fields},{},{},{}\n",
            record.context_id, record.subject_id, record.state_raw
        ));
        offset = fields + 9;
    }
    assert_eq!(offset, bytes.len());
    std::fs::write(&args[3], boundaries)?;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(RequirementSystem::decode(black_box(&bytes), limits)?);
    }
    let decode_ns = start.elapsed().as_nanos() / iterations as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(system.encode(limits)?);
    }
    let encode_ns = start.elapsed().as_nanos() / iterations as u128;
    println!("{{\"records\":{},\"bytes\":{},\"iterations\":{iterations},\"decode_ns\":{decode_ns},\"encode_ns\":{encode_ns}}}", system.records.len(), bytes.len());
    Ok(())
}
