//! Validate an extracted original Linux PlayerNotifications owner section.
use c6_core::notifications::{Limits, PlayerNotifications};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 5 {
        return Err("usage: notifications_codec INPUT OUTPUT BOUNDARIES ITERATIONS".into());
    }
    let bytes = std::fs::read(&a[1])?;
    let iterations: u32 = a[4].parse()?;
    if iterations == 0 {
        return Err("zero iterations".into());
    }
    let l = Limits::default();
    let (value, ranges) = PlayerNotifications::decode_with_boundaries(&bytes, l)?;
    assert_eq!(value.encode(l)?, bytes);
    std::fs::write(&a[2], value.encode(l)?)?;
    let mut boundaries = String::new();
    for (r, span) in value.records.iter().zip(&ranges) {
        let summary_start = span.start + 33;
        let summary_end = summary_start + r.summary.base.len();
        let message_start = summary_end + 11;
        let message_end = message_start + r.message.base.len();
        let parameters_start = message_end + 35;
        let parameters_end = span.end - 2;
        boundaries.push_str(&format!(
            "{},{},{},{},{},{},{summary_start},{summary_end},{message_start},{message_end},{parameters_start},{parameters_end}\n",
            span.start, span.end, r.id.id, r.type_hash, r.is_dismissed, r.has_been_activated
        ));
    }
    std::fs::write(&a[3], boundaries)?;
    // Every truncated prefix fails, including all field boundaries.
    for end in 0..bytes.len() {
        assert!(
            PlayerNotifications::decode(&bytes[..end], l).is_err(),
            "accepted truncation {end}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(PlayerNotifications::decode(&extra, l).is_err());
    let mut bad = bytes.clone();
    bad[..4].copy_from_slice(&2u32.to_le_bytes());
    assert!(PlayerNotifications::decode(&bad, l).is_err());
    bad = bytes.clone();
    bad[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(PlayerNotifications::decode(&bad, l).is_err());
    assert!(PlayerNotifications::decode(
        &bytes,
        Limits {
            max_bytes: bytes.len() - 1,
            ..l
        }
    )
    .is_err());
    if !ranges.is_empty() {
        assert!(PlayerNotifications::decode(
            &bytes,
            Limits {
                max_records: ranges.len() - 1,
                ..l
            }
        )
        .is_err());
        for span in &ranges {
            let mut bad = bytes.clone();
            bad[span.end - 1] = 2;
            assert!(PlayerNotifications::decode(&bad, l).is_err());
        }
    }
    assert!(PlayerNotifications::decode(&bytes, Limits { max_items: 0, ..l }).is_err());
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(PlayerNotifications::decode(black_box(&bytes), l)?);
    }
    let decode_ns = start.elapsed().as_nanos() / iterations as u128;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(value.encode(l)?);
    }
    let encode_ns = start.elapsed().as_nanos() / iterations as u128;
    println!("{{\"records\":{},\"bytes\":{},\"iterations\":{iterations},\"decode_ns\":{decode_ns},\"encode_ns\":{encode_ns},\"all_truncated_prefixes_rejected\":true}}",ranges.len(),bytes.len());
    Ok(())
}
