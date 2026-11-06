//! Bounded stock Achievements.OnUnitMoved early returns (Linux XP2 0x2901a00).
//!
//! The stock integrity/configuration/map lookups before the owner comparison
//! are read-only and do not draw RNG. Null sentinel plots and equal current Plot owners return before
//! diplomacy, distance, or platform achievement unlocking. This does not omit
//! other cross-owner paths or authorize arbitrary achievement subscribers.
//! A NO_PLAYER destination reaches IsAtWarWith(-1), whose invalid-player
//! guard returns false before distance/UnlockAchievement; it also has no RNG.
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
/// The historical entrypoint also supports a current NO_PLAYER destination:
/// IsAtWarWith(-1) -> GetValidPlayer -> IsValidRange returns false, before any
/// diplomacy matrix, distance, achievement unlock or RNG is reached.
/// The exact native -9999 sentinel resolves to null before wrapping; either
/// null plot returns before owner reads. Other missing/wrapped coordinates stay
/// outside this bounded canonical-coordinate profile.
pub fn omit_stock_same_owner(
    state: &LoadedState,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error> {
    let map = state.world().map();
    //2901a36/45 performs both lookups before2901a4b/54 null guards.
    let old_plot = map.plot_at(old.x, old.y);
    let new_plot = map.plot_at(new.x, new.y);
    if null_sentinel(old) || null_sentinel(new) {
        return Ok(());
    }
    let old_plot = old_plot.ok_or(Error::MissingCanonicalPlot(old))?;
    let new_plot = new_plot.ok_or(Error::MissingCanonicalPlot(new))?;
    let old_owner = movement_placement_state::plot_owner(old_plot).map_err(Error::PlotOwner)?;
    let new_owner = movement_placement_state::plot_owner(new_plot).map_err(Error::PlotOwner)?;
    equal_owner(old_owner, new_owner)
}
fn null_sentinel(point: PlotCoord) -> bool {
    point.x == -9999 || point.y == -9999
}
fn equal_owner(old: i32, new: i32) -> Result<(), Error> {
    if old == new || new == -1 {
        Ok(())
    } else {
        Err(Error::UnsupportedCrossOwner { old, new })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_null_sentinel_is_exact_and_checks_both_axes() {
        assert!(null_sentinel(PlotCoord { x: -9999, y: 0 }));
        assert!(null_sentinel(PlotCoord { x: 0, y: -9999 }));
        for value in [-10000, -9998, -1, 0] {
            assert!(!null_sentinel(PlotCoord { x: value, y: value }));
        }
    }
    #[test]
    fn equal_current_owners_include_both_unowned() {
        for owner in [-1, 0, 7, 63] {
            assert!(equal_owner(owner, owner).is_ok());
        }
    }
    #[test]
    fn no_player_destination_returns_before_distance_or_unlock() {
        for old in [-1, 0, 7, 63] {
            assert!(equal_owner(old, -1).is_ok());
        }
    }
    #[test]
    fn other_cross_owner_paths_stay_unsupported() {
        for (old, new) in [(0, 7), (7, 0), (-1, 0)] {
            assert!(
                matches!(equal_owner(old,new),Err(Error::UnsupportedCrossOwner {old:a,new:b})if(a,b)==(old,new))
            );
        }
    }
}
