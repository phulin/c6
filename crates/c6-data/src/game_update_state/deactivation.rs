//! Current TurnManager deactivation bookkeeping. A changed visibility record
//! requires the actual gossip effects; it is not a player-policy boundary.
use crate::registry::LinuxDefinitionHashRegistry;
pub mod resources;
pub mod visibility_sources;
use c6_core::{
    city_power_activation::gossip_delivery::TurnVisibility,
    loaded_state::{ActionStamp, LoadedState},
};
use visibility_sources::Definitions;

/// The visibility portion of the live gossip owner, independent of its pending
/// gossip queues. Initial history and fallback must come from this world.
pub struct VisibilityHistory {
    instance: ActionStamp,
    pub history: Vec<TurnVisibility>,
    pub fallback: TurnVisibility,
    pub alive_majors: Vec<i32>,
    sources: LinuxDefinitionHashRegistry,
    stock: Option<StockVisibility>,
    definitions: Option<Definitions>,
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
    Random(c6_core::loaded_state::game_sync::AccessError),
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
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
            definitions: None,
        }
    }

    /// Source definitions are bound to this world's current rules identity.
    pub fn configure_definitions(
        &mut self,
        state: &LoadedState,
        definitions: Definitions,
    ) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp())
            || state.binding() != &definitions.binding
        {
            return Err(Error::Missing("visibility source definitions world/rules"));
        }
        if self
            .sources
            .hashes()
            .iter()
            .any(|hash| !definitions.sources.iter().any(|v| v.hash == *hash))
        {
            return Err(Error::Missing("visibility source definition hash"));
        }
        self.definitions = Some(definitions);
        Ok(())
    }

    /// Bind current civilization name extents and actual initialized listener
    /// order. Empty civilization collections return -1 before usage/RNG reads.
    /// Name usage and the shared GameSync remain owned by the loaded world.
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
    pub fn record(&mut self, state: &mut LoadedState, owner: i32) -> Result<usize, Error> {
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
                // Spy sources bypass ordinary name allocation. Prerequisite/trait
                // sources inspect their retained diplomatic source selector.
                if value.source != -1 {
                    let definitions = self
                        .definitions
                        .as_ref()
                        .ok_or(Error::Missing("visibility source definitions"))?;
                    let definition = definitions
                        .sources
                        .iter()
                        .find(|v| v.hash == hash)
                        .ok_or(Error::Missing("visibility source definition"))?;
                    if definition.spy {
                        return Err(Error::Missing("visibility source spy name"));
                    }
                    if definition.dynamic {
                        return Err(Error::Missing("visibility dynamic citizen-name selector"));
                    }
                }
                let name = choose_citizen_name(state, owner, name_count)?;
                append_changed(
                    &mut self.history,
                    value,
                    || Ok(name),
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

/// Ordinary diplomatic source (selector zero): scan the first nonnegative
/// usage tier in native index order, give each candidate weight one, draw from
/// GameSync even for one candidate, and increment the selected signed short.
fn choose_citizen_name(
    state: &mut LoadedState,
    owner: i32,
    name_count: usize,
) -> Result<i32, Error> {
    if name_count == 0 {
        return Ok(-1);
    }
    let counts = &state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("visibility citizen-name player"))?
        .name_manager
        .citizen_name_usage_counts;
    let candidates = citizen_candidates(counts, name_count)?;
    let index = draw_citizen_index(&candidates, |bound| {
        state.game_sync_rand(bound).map_err(Error::Random)
    })?;
    let counts = state
        .player_citizen_name_usage_mut(owner)
        .map_err(Error::State)?;
    counts[index] = counts[index].wrapping_add(1);
    Ok(index as i32)
}
fn draw_citizen_index(
    candidates: &[usize],
    draw: impl FnOnce(u16) -> Result<u16, Error>,
) -> Result<usize, Error> {
    let choice = usize::from(draw(candidates.len() as u16)?);
    candidates
        .get(choice)
        .copied()
        .ok_or(Error::Missing("visibility name random index"))
}
fn citizen_candidates(counts: &[i16], name_count: usize) -> Result<Vec<usize>, Error> {
    let counts = &counts[..counts.len().min(name_count)];
    let min = counts
        .iter()
        .copied()
        .filter(|v| *v >= 0)
        .min()
        .ok_or(Error::Missing(
            "visibility nonnegative citizen-name usage tier",
        ))?;
    Ok(counts
        .iter()
        .enumerate()
        .filter(|(_, v)| **v == min)
        .map(|(i, _)| i)
        .collect())
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
    fn citizen_selection_draws_even_for_single_option_and_uses_native_order() {
        let calls = Cell::new(0);
        assert_eq!(
            draw_citizen_index(&[7], |bound| {
                assert_eq!(bound, 1);
                calls.set(calls.get() + 1);
                Ok(0)
            })
            .unwrap(),
            7
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            draw_citizen_index(&[2, 5, 6], |bound| {
                assert_eq!(bound, 3);
                Ok(1)
            })
            .unwrap(),
            5
        );
        assert!(draw_citizen_index(&[1], |_| Err(Error::Missing("GameSync"))).is_err());
    }
    #[test]
    fn citizen_tiers_preserve_native_order_and_skip_negative_usage() {
        assert_eq!(citizen_candidates(&[-1, 2, 0, 0, 7], 4).unwrap(), [2, 3]);
        assert_eq!(citizen_candidates(&[4, 2, 2, 0], 3).unwrap(), [1, 2]);
        assert!(citizen_candidates(&[-1], 1).is_err());
        assert!(citizen_candidates(&[], 40).is_err());
    }
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
