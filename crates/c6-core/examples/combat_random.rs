//! Line adapter for the conditional Game sync random-damage comparison.
use c6_core::combat::DamageParameters;
use c6_core::combat_random::{
    calculate_random_damage, ArchiveOrderKey, GameSyncArchiveTracking, GameSyncDamageState,
    GameSyncRandom, RandomDamageOutcome,
};
use c6_core::rng::native_serialization::SerializedRandomRecord;
use std::io::{self, BufRead};

#[repr(C)]
struct DlInfo {
    filename: *const std::ffi::c_char,
    base: *mut std::ffi::c_void,
    symbol: *const std::ffi::c_char,
    address: *mut std::ffi::c_void,
}
#[link(name = "dl")]
extern "C" {
    fn dladdr(address: *const std::ffi::c_void, info: *mut DlInfo) -> i32;
    #[link_name = "expf@GLIBC_2.2.5"]
    fn expf(value: f32) -> f32;
}
fn observe_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let address = expf as *const std::ffi::c_void;
    let mut info = DlInfo {
        filename: std::ptr::null(),
        base: std::ptr::null_mut(),
        symbol: std::ptr::null(),
        address: std::ptr::null_mut(),
    };
    if unsafe { dladdr(address, &mut info) } == 0 || info.base.is_null() {
        return Err("exponential binding unavailable".into());
    }
    let mut mxcsr = 0_u32;
    let mut x87_control = 0_u16;
    unsafe {
        std::arch::asm!("stmxcsr [{p}]",p=in(reg)&mut mxcsr,options(nostack,preserves_flags));
        std::arch::asm!("fnstcw [{p}]",p=in(reg)&mut x87_control,options(nostack,preserves_flags));
    }
    eprintln!(
        "C6_COMBAT_RANDOM_RUNTIME {} {mxcsr} {x87_control}",
        address as usize - info.base as usize
    );
    Ok(())
}
fn parameters(line: &str) -> Result<DamageParameters, Box<dyn std::error::Error>> {
    let v: Vec<i64> = line
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    if v.len() != 12
        || !(0..=u32::MAX as i64).contains(&v[5])
        || !(0..=1).contains(&v[11])
        || v.iter()
            .enumerate()
            .any(|(i, &n)| i != 5 && i != 11 && i32::try_from(n).is_err())
    {
        return Err("unsupported parameter record".into());
    }
    Ok(DamageParameters {
        attacker_strength_raw: v[0] as i32,
        defender_strength_raw: v[1] as i32,
        defender_max_hit_points: v[2] as i32,
        outer_defense_damage: v[3] as i32,
        outer_defense_max_hit_points: v[4] as i32,
        defense_type: v[5] as u32,
        power_scaling_raw: v[6] as i32,
        minimum_damage: v[7] as i32,
        base_damage: v[8] as i32,
        power_dampener: v[9] as i32,
        max_extra_damage: v[10] as i32,
        bypass_defenses: v[11] != 0,
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    observe_runtime()?;
    let extra: i32 = std::env::args()
        .nth(1)
        .ok_or("missing actual extra-damage rule")?
        .parse()?;
    let mut lines = io::stdin().lock().lines();
    let header = lines.next().ok_or("missing named-stream state")??;
    let words: Vec<u64> = header
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    if words.len() < 7
        || words[0..4].iter().any(|&n| u32::try_from(n).is_err())
        || words[4] > 255
        || words[6] > 4096
        || words.len() != 7 + words[6] as usize
    {
        return Err("invalid initial named-stream state".into());
    }
    let mut state = GameSyncDamageState {
        random: GameSyncRandom {
            record: SerializedRandomRecord {
                seed: words[0] as u32,
                calls: words[1] as u32,
                resets: words[2] as u32,
                synchronous_byte: words[4] as u8,
            },
            lock_count: words[3] as u32,
        },
        archive: GameSyncArchiveTracking {
            sync_variable_key: ArchiveOrderKey(words[5]),
            changed_keys: words[7..].iter().copied().map(ArchiveOrderKey).collect(),
        },
    };
    for (index, line) in lines.enumerate() {
        let outcome = calculate_random_damage(&parameters(&line?)?, extra, &mut state)
            .map_err(|e| format!("preflight rejection: {e:?}"))?;
        let (kind, amount, draw) = match outcome {
            RandomDamageOutcome::Damage { amount, draw } => ("damage", amount, draw),
            RandomDamageOutcome::NativeArithmeticFaultAfterDraw { draw, .. } => {
                ("fault_after_draw", 0, draw)
            }
        };
        let r = state.random.record;
        print!(
            "{index} {kind} {amount} {draw} {} {} {} {} {} {}",
            r.seed,
            r.calls,
            r.resets,
            state.random.lock_count,
            r.synchronous_byte,
            state.archive.changed_keys.len()
        );
        for key in &state.archive.changed_keys {
            print!(" {}", key.0);
        }
        println!();
    }
    Ok(())
}
