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
    stock: Option<StockVisibility>,
}
/// Initialized synchronous consumers of the visibility-changed signal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisibilityListener {
    /// Reads current visibility and may emit an external achievement. It has no
    /// gameplay state mutation or RNG and is omitted at the headless boundary.
    Achievement,
}
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}
struct StockVisibility {
    citizen_name_counts: Vec<(i32, usize)>,
    listeners: Vec<VisibilityListener>,
}
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Missing(&'static str),
    /// AddTurnVisibility must choose the source, archive the record and dispatch
    /// VisibilityChanged before deactivation may continue.
    ChangedVisibility(TurnVisibility),
    Allocation(std::collections::TryReserveError),
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
            stock: None,
        }
    }

    /// Bind current civilization name extents and actual initialized listener
    /// order. Empty civilization collections return -1 before usage/RNG reads.
    /// Nonempty collections remain explicit until their selection is supplied.
    pub fn configure_stock(
        &mut self,
        state: &LoadedState,
        citizen_name_counts: Vec<(i32, usize)>,
        listeners: Vec<VisibilityListener>,
        _scope: Scope,
    ) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Missing("visibility history world instance"));
        }
        for (i, &(owner, _)) in citizen_name_counts.iter().enumerate() {
            if citizen_name_counts[..i].iter().any(|v| v.0 == owner) {
                return Err(Error::Missing("duplicate visibility civilization owner"));
            }
            state.player_slot_status(owner).map_err(Error::State)?;
        }
        self.stock = Some(StockVisibility {
            citizen_name_counts,
            listeners,
        });
        Ok(())
    }

    /// Record current visibility in alive-major order. An unchanged source and
    /// level returns before name selection; changes choose the source name,
    /// archive, then notify. Historical levels never replace current diplomacy.
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
                let stock = self.stock.as_ref().ok_or(Error::ChangedVisibility(value))?;
                let name_count = stock
                    .citizen_name_counts
                    .iter()
                    .find(|v| v.0 == owner)
                    .ok_or(Error::Missing(
                        "current civilization citizen-name collection",
                    ))?
                    .1;
                // With no source definition, native ChooseNewSource still runs
                // for visibility 0. A zero civilization collection returns -1
                // before touching usage counts or the shared GameSync stream.
                if value.source != -1 {
                    return Err(Error::Missing(
                        "visibility source spy/dynamic-name selection",
                    ));
                }
                if name_count != 0 {
                    return Err(Error::Missing("visibility citizen-name selection"));
                }
                append_changed(
                    &mut self.history,
                    value,
                    || Ok(-1),
                    |_| {
                        for listener in &stock.listeners {
                            match listener {
                                VisibilityListener::Achievement => {}
                            }
                        }
                        Ok(())
                    },
                )?;
            }
            unchanged += 1;
        }
        Ok(unchanged)
    }
}

/// Source ordering: choose a source name even at visibility 0, assign it only
/// for positive visibility, archive the record, then notify current consumers.
fn append_changed(
    history: &mut Vec<TurnVisibility>,
    mut value: TurnVisibility,
    choose_name: impl FnOnce() -> Result<i32, Error>,
    changed: impl FnOnce(TurnVisibility) -> Result<(), Error>,
) -> Result<(), Error> {
    let name = choose_name()?;
    if value.visibility > 0 {
        value.source_name_index = name;
    }
    history.try_reserve(1).map_err(Error::Allocation)?;
    history.push(value);
    changed(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn zero_visibility_still_chooses_before_append_and_dispatch() {
        let mut history = vec![];
        let phase = Cell::new(0);
        let value = TurnVisibility {
            visibility: 0,
            ..TurnVisibility::constructed()
        };
        append_changed(
            &mut history,
            value,
            || {
                assert_eq!(phase.replace(1), 0);
                Ok(7)
            },
            |v| {
                assert_eq!(phase.replace(2), 1);
                assert_eq!(v.source_name_index, -1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(phase.get(), 2);
        assert_eq!(history, [value]);
    }
    #[test]
    fn callback_failure_preserves_archived_record_and_positive_name() {
        let mut history = vec![];
        let value = TurnVisibility {
            visibility: 2,
            ..TurnVisibility::constructed()
        };
        assert!(matches!(
            append_changed(
                &mut history,
                value,
                || Ok(13),
                |_| { Err(Error::Missing("consumer")) }
            ),
            Err(Error::Missing("consumer"))
        ));
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].source_name_index, 13);
    }
    #[test]
    fn selection_failure_precedes_archive_mutation() {
        let mut history = vec![];
        assert!(append_changed(
            &mut history,
            TurnVisibility::constructed(),
            || { Err(Error::Missing("name RNG")) },
            |_| panic!("no dispatch")
        )
        .is_err());
        assert!(history.is_empty());
    }
}
