//! Current TurnManager deactivation bookkeeping. A changed visibility record
//! requires the actual gossip effects; it is not a player-policy boundary.
use crate::registry::LinuxDefinitionHashRegistry;
pub mod resources;
use c6_core::{
    city_power_activation::gossip_delivery::TurnVisibility,
    loaded_state::{ActionStamp, LoadedState},
};

/// The visibility portion of the live gossip owner, independent of its pending
/// gossip queues. Initial history and fallback must come from this world.
pub struct VisibilityHistory {
    instance: ActionStamp,
    pub history: Vec<TurnVisibility>,
    pub fallback: TurnVisibility,
    pub alive_majors: Vec<i32>,
    sources: LinuxDefinitionHashRegistry,
}
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Missing(&'static str),
    /// AddTurnVisibility must choose the source, archive the record and dispatch
    /// VisibilityChanged before deactivation may continue.
    ChangedVisibility(TurnVisibility),
}
impl VisibilityHistory {
    pub fn new(
        state: &LoadedState,
        history: Vec<TurnVisibility>,
        fallback: TurnVisibility,
        alive_majors: Vec<i32>,
        native_source_hashes: Vec<u32>,
    ) -> Self {
        Self {
            instance: state.action_stamp(),
            history,
            fallback,
            alive_majors,
            sources: LinuxDefinitionHashRegistry::from_native_order(native_source_hashes),
        }
    }

    /// RecordVisibilityLevels2aad880 followed by AddTurnVisibility29b1bb0's
    /// unchanged-source-and-level return. Current diplomacy is read per major;
    /// historical levels are never substituted for current gameplay fields.
    pub fn record(&mut self, state: &LoadedState, owner: i32) -> Result<usize, Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Missing("visibility history world instance"));
        }
        let mut unchanged = 0;
        for &other in &self.alive_majors {
            if other == owner {
                continue;
            }
            if !state
                .game_relations()
                .map_err(Error::State)?
                .has_met(owner, other)
                .map_err(Error::Relations)?
            {
                continue;
            }
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.requested_owner == owner)
                .ok_or(Error::Missing("viewing player"))?;
            let index =
                usize::try_from(other).map_err(|_| Error::Missing("viewed player index"))?;
            let level = *player
                .diplomacy
                .visibility
                .get(index)
                .ok_or(Error::Missing("current visibility level"))?;
            let hash = *player
                .diplomacy
                .visibility_source_hashes
                .get(index)
                .ok_or(Error::Missing("current visibility source"))?;
            let source = if hash == u32::MAX {
                -1
            } else {
                self.sources
                    .find_index(hash)
                    .ok_or(Error::Missing("native visibility source index"))? as i32
            };
            let value = TurnVisibility {
                turn: state.retained().core.current_turn as u16,
                viewing_player: owner as i8,
                viewed_player: other as i8,
                visibility: level as i8,
                source: source as i8,
                ..TurnVisibility::constructed()
            };
            let previous = self
                .history
                .iter()
                .rev()
                .find(|v| {
                    v.viewing_player == value.viewing_player
                        && v.viewed_player == value.viewed_player
                })
                .unwrap_or(&self.fallback);
            if previous.source != value.source || previous.visibility != value.visibility {
                return Err(Error::ChangedVisibility(value));
            }
            unchanged += 1;
        }
        Ok(unchanged)
    }
}
