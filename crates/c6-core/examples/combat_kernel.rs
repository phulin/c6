//! Line adapter for the bounded Linux XP2 average-damage comparison.
use c6_core::combat::{calculate_average_damage, DamageParameters};
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

fn exponential_binding() -> Result<usize, &'static str> {
    // This reference shares the core's explicitly versioned expf relocation.
    // Observe the actual selected implementation, including
    // symbol-version and IFUNC selection, without calling it separately.
    let address = expf as *const std::ffi::c_void;
    let mut info = DlInfo {
        filename: std::ptr::null(),
        base: std::ptr::null_mut(),
        symbol: std::ptr::null(),
        address: std::ptr::null_mut(),
    };
    if unsafe { dladdr(address, &mut info) } == 0 || info.base.is_null() {
        return Err("candidate exponential binding unavailable");
    }
    Ok(address as usize - info.base as usize)
}

#[cfg(target_arch = "x86_64")]
fn floating_environment() -> (u32, u16) {
    let mut mxcsr = 0_u32;
    let mut x87_control = 0_u16;
    // Read control/status registers without changing the inherited FP state.
    unsafe {
        std::arch::asm!("stmxcsr [{ptr}]", ptr = in(reg) &mut mxcsr, options(nostack, preserves_flags));
        std::arch::asm!("fnstcw [{ptr}]", ptr = in(reg) &mut x87_control, options(nostack, preserves_flags));
    }
    (mxcsr, x87_control)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("C6_COMBAT_CANDIDATE_EXPF_OFFSET {}", exponential_binding()?);
    #[cfg(target_arch = "x86_64")]
    {
        let (mxcsr, x87_control) = floating_environment();
        eprintln!("C6_COMBAT_CANDIDATE_FP {mxcsr} {x87_control}");
    }
    let extra: i32 = std::env::args()
        .nth(1)
        .ok_or("missing global max extra damage")?
        .parse()?;
    let repetitions: Option<usize> = std::env::args().nth(2).map(|s| s.parse()).transpose()?;
    let mut cases = Vec::new();
    for line in io::stdin().lock().lines() {
        let values: Vec<i64> = line?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        if values.len() != 12
            || !(0..=u32::MAX as i64).contains(&values[5])
            || !(0..=1).contains(&values[11])
            || values
                .iter()
                .enumerate()
                .any(|(i, &n)| i != 5 && i != 11 && i32::try_from(n).is_err())
        {
            return Err("unsupported parameter record".into());
        }
        let p = DamageParameters {
            attacker_strength_raw: values[0] as i32,
            defender_strength_raw: values[1] as i32,
            defender_max_hit_points: values[2] as i32,
            outer_defense_damage: values[3] as i32,
            outer_defense_max_hit_points: values[4] as i32,
            defense_type: values[5] as u32,
            power_scaling_raw: values[6] as i32,
            minimum_damage: values[7] as i32,
            base_damage: values[8] as i32,
            power_dampener: values[9] as i32,
            max_extra_damage: values[10] as i32,
            bypass_defenses: values[11] != 0,
        };
        match calculate_average_damage(&p, extra) {
            Ok(damage) if repetitions.is_none() => println!("{damage}"),
            Ok(_) => cases.push(p),
            Err(error) => return Err(format!("unsupported arithmetic: {error:?}").into()),
        }
    }
    if let Some(repetitions) = repetitions {
        let started = std::time::Instant::now();
        let mut checksum = 0_i64;
        for _ in 0..repetitions {
            for p in &cases {
                let damage =
                    calculate_average_damage(std::hint::black_box(p), std::hint::black_box(extra))
                        .map_err(|error| format!("unsupported arithmetic: {error:?}"))?;
                checksum = checksum.wrapping_add(i64::from(std::hint::black_box(damage)));
            }
        }
        println!(
            "{{\"calls\":{},\"seconds\":{},\"checksum\":{}}}",
            cases.len() * repetitions,
            started.elapsed().as_secs_f64(),
            checksum
        );
    }
    Ok(())
}
