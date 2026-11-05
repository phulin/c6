//! Emergency.DoTurn on the existing Congress/Emergency manager owner.
use super::activation::cityless_managers::Runtime;
use c6_core::{game_capabilities::Membership, game_update::Error, loaded_state::LoadedState};
pub const CAPABILITY: u32 = 0x370521e0;

/// The native temporary list contains pointers in the original list order.
/// No live instances are copied or removed. Its signal list is cleared before
/// visiting that saved order, including when the first positive branch fails.
fn ordered_turn<T>(
    manager: &mut T,
    count: usize,
    clear: impl FnOnce(&mut T) -> Result<(), Error>,
    mut instance: impl FnMut(&mut T, usize) -> Result<(), Error>,
) -> Result<(), Error> {
    let order: Vec<_> = (0..count).collect();
    clear(manager)?;
    for index in order {
        instance(manager, index)?;
    }
    Ok(())
}
/// Current empty or all-expired instances are supported. A nonexpired instance
/// requires the remaining rule-dependent emergency bookkeeping and callbacks;
/// it is never silently treated as a successful no-op.
pub fn run(
    state: &LoadedState,
    runtime: &mut Runtime,
    capabilities: &Membership,
) -> Result<(), Error> {
    if !capabilities.has(CAPABILITY) {
        return Ok(());
    }
    let count = runtime.congress(state)?.emergencies.len();
    // No positive callback is implemented yet, so no list mutation can
    // invalidate these logical instance positions.
    ordered_turn(
        runtime,
        count,
        |r| r.clear_emergency_signals(state),
        |r, index| {
            let instance = &r.congress(state)?.emergencies[index];
            if instance.expired_raw != 0 {
                Ok(())
            } else {
                Err(Error::Unsupported("current Emergency instance DoTurn"))
            }
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_precedes_source_order_and_failure_keeps_the_executed_prefix() {
        let mut calls = vec![];
        let result = ordered_turn(
            &mut calls,
            3,
            |calls| {
                calls.push(-1);
                Ok(())
            },
            |calls, index| {
                calls.push(index as i32);
                if index == 1 {
                    Err(Error::Unsupported("positive emergency"))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(calls, [-1, 0, 1]);
    }
    #[test]
    fn empty_instance_list_still_clears_signals() {
        let mut signals = vec![1];
        ordered_turn(
            &mut signals,
            0,
            |signals| {
                signals.clear();
                Ok(())
            },
            |_, _| panic!("empty copied list"),
        )
        .unwrap();
        assert!(signals.is_empty());
    }
}
