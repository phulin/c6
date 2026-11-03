//! Original Linux XP2 operation predicates, not action admission or execution.
//! Live player/placement/rules inputs remain explicit; no default provider exists.
use crate::unit::Unit;

/// Unit::IsBusy (0x2c27360): signed operation timer strictly above zero.
pub fn is_busy(unit: &Unit) -> bool {
    unit.i_operation_timer > 0
}

/// Unit::CanMove (0x2c27930): signed FixedPointT<8> raw value above zero.
/// A positive fraction below one movement point still passes.
pub fn can_move(unit: &Unit) -> bool {
    unit.x_moves_remaining > 0
}

/// Required original gameplay queries, in native call order. These are not
/// acceptance booleans. Errors mean missing/unsupported inputs, not native false.
/// Implementations must preserve query effects and resolve actual layered map
/// membership, Formation and live rules. See docs/linux-unit-operation-legality.md.
/// Mutable Unit permits synchronous query effects; subsequent predicates reread it.
pub trait SkipTurnQueries {
    type Error;
    fn is_human(&mut self, unit: &mut Unit) -> Result<bool, Self::Error>;
    /// Original Formation overload, false bool argument: unsigned maximum over
    /// existing, non-delayed-death members' original friendly-unit type counts.
    fn friendly_units_of_type_count(&mut self, unit: &mut Unit) -> Result<u32, Self::Error>;
    fn plot_unit_limit(&mut self, unit: &mut Unit) -> Result<i32, Self::Error>;
}

/// Complete SkipTurn::CanStart handler (0x2dac950), separate from the full
/// manager predicate, scripting veto and request ordering. Does not check moves.
/// `testVisible` does not bypass the human branch. On error, any effects already
/// performed by the provider remain; this query is not transactional.
pub fn skip_turn_can_start<Q: SkipTurnQueries>(
    unit: &mut Unit,
    queries: &mut Q,
) -> Result<bool, Q::Error> {
    if queries.is_human(unit)? {
        if unit.i_fortify_turns > 0 || [super::HOLD, 0x024a74eb].contains(&unit.e_activity) {
            return Ok(false);
        }
        let count = queries.friendly_units_of_type_count(unit)?;
        let limit = queries.plot_unit_limit(unit)?;
        // Native CMP/JA compares the same 32 bits unsigned, even for negative limits.
        if count > limit as u32 {
            return Ok(false);
        }
    }
    if unit.e_activity == 0x5ff1bbdf {
        return Ok(false);
    }
    Ok(unit.k_espionage.e_operation == u32::MAX)
}
