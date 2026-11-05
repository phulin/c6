//! Current global constructed-count gate of the Linux UnitCreated gossip leaf.
//! The occurrence/delivery branch stays explicit until its effects are supplied.
use super::Error;
use c6_core::loaded_state::LoadedState;

/// Called only after the major-player and obsolete-era guards. Native code
/// compares the global Game.Stats count with exactly one, without changing it.
/// Counts other than one return before occurrence construction, callbacks or RNG.
pub fn require_no_occurrence(state: &LoadedState, unit_hash: u32) -> Result<(), Error> {
    let count = state
        .game_unit_constructed_count(unit_hash)
        .map_err(Error::Statistics)?;
    if count != 1 {
        return Ok(());
    }
    Err(Error::Missing(
        "trained-unit gossip occurrence and delivery",
    ))
}
