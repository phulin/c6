//! Linux CommandUnits auto-notification queries over authoritative typed state.
//!
//! Source-initialized Unit/Formation runtime is required where the original
//! queries dereference it. That diagnostic origin is not an observed final
//! native runtime capture, and these queries do not establish action parity.
use super::{notifications, reporting, Error};
use c6_core::{loaded_state::LoadedState, notifications::Notification, skip_turn::legality};

pub const TYPE_HASH: u32 = 0xfe4d_413d;
pub const VTABLE_ADDRESSPOINT: u64 = 0x3af_53a0;
pub const DEFAULT_BLOCKING: i32 = 0x0169_297f;

/// Four native signed wrapping counters, in Processor::UnitCounts field order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UnitCounts {
    pub total: i32,
    pub movable_noncycling: i32,
    pub unavailable: i32,
    pub needing_attention: i32,
}
impl UnitCounts {
    pub fn command_count(self) -> i32 {
        self.movable_noncycling.wrapping_sub(self.unavailable)
    }
}

/// Subclass portion of Initialize after the shared base Initialize has run.
/// The caller keeps the allocated object across errors: +0x199, +0x19a and
/// +0x18c are written before GetUnitCounts, while count/blocking follow it.
/// Manager initialization, creator registration and allocation authority belong
/// to the enclosing Create orchestration, never to a fabricated success flag.
pub fn initialize_auto(
    state: &LoadedState,
    queries: &reporting::Provider<'_>,
    created: &mut notifications::Created,
) -> Result<(), Error> {
    if created.record.type_hash != TYPE_HASH
        || created.record.factory_type != TYPE_HASH
        || created.vtable_addresspoint != VTABLE_ADDRESSPOINT
        || !created.record.parameters.entries.is_empty()
        || !created.record.native_19b
    {
        return Err(Error::Unsupported(
            "CommandUnits auto Initialize receiver binding",
        ));
    }
    created.record.native_199 = false;
    created.record.native_19a = true;
    created.record.native_18c = 0;
    let count =
        get_unit_counts(state, i32::from(created.record.id.owner), queries)?.command_count();
    created.record.native_190 = count;
    created.default_blocking = DEFAULT_BLOCKING;
    Ok(())
}

/// CommandUnits::GetUnitCounts first constructs four actual zero counters, then
/// skips Processor::GetUnitCounts for NO_PLAYER and nonhuman configurations.
/// Human unit iteration uses the retained complete native collection order.
/// A reached unknown HoldCycling callback is an error, never an assumed result.
pub fn get_unit_counts(
    state: &LoadedState,
    owner: i32,
    queries: &reporting::Provider<'_>,
) -> Result<UnitCounts, Error> {
    let mut counts = UnitCounts::default();
    if owner == -1 {
        return Ok(counts);
    }
    if state
        .player_controls(owner)
        .map_err(Error::State)?
        .slot_status
        != 3
    {
        return Ok(counts);
    }
    let player = state.world().player(owner).map_err(Error::World)?;
    for slot in player.unit_records().map_err(Error::World)? {
        let unit = &slot.value;
        counts.total = counts.total.wrapping_add(1);
        if unit.b_delayed_death {
            continue;
        }
        if unit.b_needs_attention {
            counts.needing_attention = counts.needing_attention.wrapping_add(1);
        }
        if !legality::can_move(unit) || queries.can_cycle_past(state, unit)? {
            continue;
        }
        counts.movable_noncycling = counts.movable_noncycling.wrapping_add(1);
        if !queries.is_ready_to_move(state, unit)?
            || (unit.e_automation != -1 && unit.e_ai_type != -1)
        {
            counts.unavailable = counts.unavailable.wrapping_add(1);
            continue;
        }
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let reference = runtime.formation.ok_or(Error::Unsupported(
            "CommandUnits ready unit Formation.HasMoved receiver is null",
        ))?;
        state
            .formation_runtime(reference.owner, reference.requested_id)
            .map_err(Error::State)?;
        let formation = state
            .world()
            .player(reference.owner)
            .map_err(Error::World)?
            .formation(reference.requested_id)
            .map_err(Error::World)?
            .ok_or(Error::Unsupported(
                "CommandUnits Formation.HasMoved receiver unavailable",
            ))?;
        if formation.resolved_id != reference.resolved_id {
            return Err(Error::Unsupported(
                "CommandUnits formation runtime binding changed",
            ));
        }
        if formation.record.has_moved && !unit.b_needs_attention {
            counts.unavailable = counts.unavailable.wrapping_add(1);
        }
    }
    Ok(counts)
}

/// 0x2d2aab0: positive current count equal to the stored count survives.
pub fn is_expired(
    state: &LoadedState,
    record: &Notification,
    queries: &reporting::Provider<'_>,
) -> Result<bool, Error> {
    let count = get_unit_counts(state, i32::from(record.id.owner), queries)?.command_count();
    Ok(count <= 0 || count != record.native_190)
}

/// Inherited Instance::IsNeeded invokes virtual IsExpired only for +0x19a.
pub fn is_needed(
    state: &LoadedState,
    record: &Notification,
    queries: &reporting::Provider<'_>,
) -> Result<bool, Error> {
    if !record.native_19a {
        return Ok(true);
    }
    Ok(!is_expired(state, record, queries)?)
}

/// CommandUnits first executes the base duplicate comparison, then GetCount.
/// This bounded entry accepts only the auto=true/null-parameters candidate.
pub fn is_duplicate(candidate: &Notification, existing: &Notification) -> Result<bool, Error> {
    Ok(notifications::base_auto_duplicate(candidate, existing)
        .map_err(|_| Error::Unsupported("CommandUnits base auto duplicate parameters"))?
        && candidate.native_190 == existing.native_190)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_requires_count_after_base_target_comparison() {
        let candidate = Notification {
            type_hash: TYPE_HASH,
            native_190: 3,
            ..Default::default()
        };
        let mut existing = candidate.clone();
        existing.id.id = 427;
        assert!(is_duplicate(&candidate, &existing).unwrap());
        existing.native_190 = 2;
        assert!(!is_duplicate(&candidate, &existing).unwrap());
        existing.native_190 = 3;
        existing.component.id = 427;
        assert!(!is_duplicate(&candidate, &existing).unwrap());
    }

    #[test]
    fn command_count_preserves_native_signed_wrap() {
        assert_eq!(
            UnitCounts {
                movable_noncycling: i32::MIN,
                unavailable: 1,
                ..Default::default()
            }
            .command_count(),
            i32::MAX
        );
    }
}
