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
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    ActorIdentity,
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
/// The cross-owner continuation queries the moving Unit owner's current
/// diplomacy against the current destination Plot owner (2901a75..2901aa4).
/// A false war predicate returns before distance or platform effects/RNG.
pub fn omit_stock_current(
    state: &LoadedState,
    current: c6_core::unit::ComponentId,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error> {
    match omit_stock_same_owner(state, old, new) {
        Ok(()) => Ok(()),
        Err(Error::UnsupportedCrossOwner {
            old: old_owner,
            new: new_owner,
        }) => {
            if current.component_type != 1 {
                return Err(Error::ActorIdentity);
            }
            let owner = i32::from(current.owner);
            let unit = state
                .world()
                .player(owner)
                .map_err(Error::World)?
                .unit(current.id)
                .map_err(Error::World)?
                .ok_or(Error::ActorIdentity)?
                .record;
            if unit.e_owner != owner {
                return Err(Error::ActorIdentity);
            }
            // IsAtWarWith2aa4bb0 validates the other Player and tests IsAlive
            // before the canonical DiplomaticRelations.IsAtWar query.
            if !state
                .world()
                .player_is_valid(new_owner)
                .map_err(Error::World)?
                || !state.player_alive(new_owner).map_err(Error::State)?
            {
                return Ok(());
            }
            if !state
                .game_relations()
                .map_err(Error::State)?
                .is_at_war(owner, new_owner)
                .map_err(Error::Relations)?
            {
                return Ok(());
            }
            Err(Error::UnsupportedCrossOwner {
                old: old_owner,
                new: new_owner,
            })
        }
        Err(error) => Err(error),
    }
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
