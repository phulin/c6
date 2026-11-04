//! Bounded stock Achievements.OnUnitMoved early return (Linux XP2 0x2901a00).
//!
//! The stock integrity/configuration/map lookups before the owner comparison
//! are read-only and do not draw RNG. Equal current Plot owners return before
//! diplomacy, distance, or platform achievement unlocking. This does not omit
//! the cross-owner path or authorize arbitrary achievement subscribers.
use crate::movement_placement_state;
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};

#[derive(Debug)]
pub enum Error {
    MissingCanonicalPlot(PlotCoord),
    PlotOwner(movement_placement_state::Error),
    UnsupportedCrossOwner { old: i32, new: i32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock UnitMoved achievement early return: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Use only for the source-qualified stock `StartupGame::$_60` listener, with
/// stock read-only preamble getters. Read owners now, after preceding listeners;
/// initial owners or the moved Unit's owner cannot replace these current reads.
/// Wrapped/missing coordinates are outside this narrow canonical-coordinate
/// profile, even though the native callback has additional inert branches.
pub fn omit_stock_same_owner(
    state: &LoadedState,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error> {
    let map = state.world().map();
    let old_plot = map
        .plot_at(old.x, old.y)
        .ok_or(Error::MissingCanonicalPlot(old))?;
    let new_plot = map
        .plot_at(new.x, new.y)
        .ok_or(Error::MissingCanonicalPlot(new))?;
    let old_owner = movement_placement_state::plot_owner(old_plot).map_err(Error::PlotOwner)?;
    let new_owner = movement_placement_state::plot_owner(new_plot).map_err(Error::PlotOwner)?;
    equal_owner(old_owner, new_owner)
}
fn equal_owner(old: i32, new: i32) -> Result<(), Error> {
    if old == new {
        Ok(())
    } else {
        Err(Error::UnsupportedCrossOwner { old, new })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_current_owners_include_both_unowned() {
        for owner in [-1, 0, 7, 63] {
            assert!(equal_owner(owner, owner).is_ok());
        }
    }
    #[test]
    fn cross_owner_path_stays_unsupported_in_either_direction() {
        for (old, new) in [(0, 7), (7, 0), (-1, 0), (0, -1)] {
            assert!(
                matches!(equal_owner(old,new),Err(Error::UnsupportedCrossOwner {old:a,new:b})if(a,b)==(old,new))
            );
        }
    }
}
