//! Decimal line adapter for original Linux component differential fixtures.
use c6_core::random_variants::{RandomPcg32, ScopedRandom};
use std::io::{self, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scoped = [ScopedRandom::new(0); 4];
    let mut pcg = [RandomPcg32::default(); 4];
    for line in io::stdin().lock().lines() {
        let values = line?
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != 6 || values[1] >= 4 || values[5] > 1 {
            return Err("invalid fixture record".into());
        }
        let [op, stream, arg, state, increment, retain]: [u64; 6] = values.try_into().unwrap();
        let index = stream as usize;
        if arg > u32::MAX as u64 || (op <= 2 && state > u32::MAX as u64) {
            return Err("fixture integer exceeds native argument width".into());
        }
        if retain == 0 {
            scoped[index] = ScopedRandom::new(state as u32);
            pcg[index] = RandomPcg32 { state, increment };
        }
        let mut result = 0;
        match op {
            0 => scoped[index] = ScopedRandom::new(arg as u32),
            1 => scoped[index].reset(arg as u32),
            2 => result = scoped[index].get(arg as u16),
            3 => pcg[index] = RandomPcg32::default(),
            4 => pcg[index] = RandomPcg32::new(arg as u32),
            5 => pcg[index].seed(arg as u32),
            6 => result = pcg[index].rand(),
            _ => return Err("unknown operation".into()),
        }
        if op <= 2 {
            println!("{result} {} 0", scoped[index].seed());
        } else {
            println!("{result} {} {}", pcg[index].state, pcg[index].increment);
        }
    }
    Ok(())
}
