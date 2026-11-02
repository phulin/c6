use c6_core::rng::RandomState;
use std::{hint::black_box, time::Instant};
fn main() {
    let draws: u32 = std::env::var("C6_BENCH_DRAWS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000_000);
    assert!(draws > 0);
    let mut rng = RandomState::new(0x12345678);
    let start = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..draws {
        checksum = checksum.wrapping_add(u64::from(black_box(rng.draw(black_box(65535)))));
    }
    let elapsed = start.elapsed();
    println!("primitive=GameCore::Random::Get draws={draws} ns_per_draw={:.3} checksum={checksum} final_seed={} calls={}",elapsed.as_nanos() as f64/f64::from(draws),rng.seed,rng.calls);
}
