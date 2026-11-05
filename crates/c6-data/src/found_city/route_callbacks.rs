//! Reached GameSignals.RouteChanged History listener (Linux29eb070).
//! Current first-city founding cannot connect two distinct owned cities. The
//! History guards still execute in source order against the live manager.
use c6_core::{
    core_player::initialization::WireBinding, game_history::Moment, loaded_state::LoadedState,
    unit::PlotCoord,
};

const FIRST_RAILROAD: u32 = 0x7dbc_e35d;
const RAILROAD: u32 = 0x435e_2210;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Listener {
    History { route_type: i32 },
    Unsupported { label: u64 },
}
pub struct Inputs {
    binding: WireBinding,
    listeners: Vec<Listener>,
}
impl Inputs {
    pub fn new(state: &LoadedState, listeners: Vec<Listener>) -> Self {
        Self {
            binding: state.binding().clone(),
            listeners,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Missing(&'static str),
    UnsupportedListener {
        index: usize,
        label: u64,
    },
    Connectivity {
        owner: i32,
        route_type: i32,
        moment: u32,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "route callback: {self:?}")
    }
}
impl std::error::Error for Error {}
fn has(moments: &[Moment], kind: u32, owner: i32) -> bool {
    moments
        .iter()
        .any(|m| m.type_hash == kind && (owner == -1 || m.acting_player_ordinal == owner))
}
fn next_moment(moments: &[Moment], owner: i32) -> Option<u32> {
    if !has(moments, FIRST_RAILROAD, -1) {
        return Some(FIRST_RAILROAD);
    }
    if has(moments, FIRST_RAILROAD, owner) || has(moments, RAILROAD, owner) {
        return None;
    }
    Some(RAILROAD)
}
fn valid(location: PlotCoord, route: i32, owner: i32) -> bool {
    location.x > -9999 && location.y > -9999 && route != -1 && owner != -1
}
/// Called synchronously after the source route-field/cache/report prefix.
/// No receiver is silently discarded. Inputs are the caller's current ordered
/// subscription projection; unknown recipients retain the caller's mutation.
pub fn route_changed(
    state: &mut LoadedState,
    inputs: &Inputs,
    location: PlotCoord,
    route: i32,
    owner: i32,
) -> Result<(), Error> {
    if state.binding() != &inputs.binding {
        return Err(Error::Binding);
    }
    for (index, listener) in inputs.listeners.iter().enumerate() {
        match *listener {
            Listener::Unsupported { label } => {
                return Err(Error::UnsupportedListener { index, label })
            }
            Listener::History { route_type } => {
                if !valid(location, route, owner) {
                    continue;
                }
                let history = state
                    .game_history()
                    .map_err(|_| Error::Missing("current History"))?;
                let Some(moment) = next_moment(&history.moments, owner) else {
                    continue;
                };
                let player = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("current route owner Cities"))?;
                // HasTwoCities2bce500 initializes two INVALID coordinates.
                // FindTwoCities2bce5f0 visits each plot index once; only an
                // owned City assigns either result. Thus fewer than two live
                // canonical City records proves false without any route flood.
                // This is a pure local search: no gameplay writes/callbacks/RNG.
                if player.cities.records.entries.len() < 2 {
                    continue;
                }
                return Err(Error::Connectivity {
                    owner,
                    route_type,
                    moment,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn moment(kind: u32, owner: i32) -> Moment {
        Moment {
            wire_version: 8,
            type_hash: kind,
            turn: 0,
            action_plot: PlotCoord { x: 0, y: 0 },
            acting_player_ordinal: owner,
            id: 0,
            has_ever_been_commemorated_raw: 0,
            game_era_hash: u32::MAX,
            extra_data: vec![],
            city_name_bytes: vec![],
            unit_citizen_name_bytes: vec![],
            other_city_name_bytes: vec![],
            era_score: 0,
        }
    }
    #[test]
    fn history_global_first_then_own_moments_selects_the_source_kind() {
        assert_eq!(next_moment(&[], 0), Some(FIRST_RAILROAD));
        assert_eq!(next_moment(&[moment(FIRST_RAILROAD, 1)], 0), Some(RAILROAD));
        assert_eq!(next_moment(&[moment(FIRST_RAILROAD, 0)], 0), None);
        assert_eq!(
            next_moment(&[moment(FIRST_RAILROAD, 1), moment(RAILROAD, 0)], 0),
            None
        );
        // A non-first entry cannot suppress the still-unrecorded global first.
        assert_eq!(next_moment(&[moment(RAILROAD, 0)], 0), Some(FIRST_RAILROAD));
    }
    #[test]
    fn source_validity_does_not_filter_on_the_cached_history_route() {
        assert!(valid(PlotCoord { x: 18, y: 11 }, 0, 0));
        assert!(!valid(PlotCoord { x: -9999, y: 11 }, 0, 0));
        assert!(!valid(PlotCoord { x: -10000, y: 11 }, 0, 0));
        assert!(!valid(PlotCoord { x: 18, y: 11 }, -1, 0));
        assert!(!valid(PlotCoord { x: 18, y: 11 }, 0, -1));
    }
}
