//! Source-ordered stock UnitCreated consumers. Callers retain their qualified
//! live registration order; mutable Script, effects, requirements and collection
//! owners are borrowed from the ongoing action/turn, never initialized here.
use super::{collections, signals, Definitions};
use crate::{
    formation_rules, loaded_actions::stock_script,
    movement_placement_state::CurrentCivilizationLevels,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};

/// Immutable registrations for the canonical History.HandleMoment continuation.
/// History records and era scores remain in LoadedState; these are not copies.
pub struct History<'a> {
    pub definitions: &'a crate::melee_attack::resolve::completion::history::Definitions,
    pub inputs: crate::melee_attack::resolve::completion::history::Inputs<'a>,
    pub recorded: &'a [crate::melee_attack::resolve::completion::history::RecordedListener],
    /// Source-qualified moment/era-score/PrideMoment presentation routes, with
    /// no additional gameplay consumers. Unqualified routes remain unavailable.
    pub presentation_qualified: bool,
}
struct HistoryEffects<'a>(Option<&'a History<'a>>);
impl signals::history::Effects for HistoryEffects<'_> {
    fn handle_moment(
        &mut self,
        state: &mut LoadedState,
        moment: c6_core::game_history::Moment,
    ) -> Result<(), signals::Error> {
        let services = self
            .0
            .ok_or(signals::Error::Missing("current Unit history services"))?;
        if !services.presentation_qualified {
            return Err(signals::Error::Missing(
                "current Unit moment report consumers",
            ));
        }
        // Reuse the source-qualified general callbacks. The only mutable
        // gameplay records are the canonical History/Eras fields in state.
        let mut effects = crate::loaded_actions::government_actions::history::StockEffects {
            recorded: services.recorded,
            reports: Vec::new(),
        };
        let result = crate::melee_attack::resolve::completion::history::handle_moment(
            state,
            services.definitions,
            services.inputs,
            moment,
            &mut effects,
        );
        // Native cache/Host/PrideMoment output is omitted at this qualified
        // headless boundary; synchronous gameplay callbacks above are retained.
        result
            .map(|_| ())
            .map_err(|e| signals::Error::Moment(format!("{e:?}")))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Consumer {
    /// GeneralPlayerDataProvider2644890 only updates summary-local historical
    /// datapoints; it has no gameplay subscribers or RNG.
    HistoricalSummary,
    Script,
    SettlerGossip,
    TrainedGossip,
    History,
    TrainQuests,
    Boost {
        kind: signals::BoostCallback,
        row: i64,
    },
    Collection(collections::CreatedCollection),
    Achievement,
}
#[derive(Clone, Copy, Debug)]
pub enum PlayerConsumer {
    Control,
    Economic,
}
#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    Signal(signals::Error),
    Collection(collections::Error),
}
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub signals: &'a signals::Definitions,
    pub combat: &'a formation_rules::Definitions,
    pub build_maps: &'a signals::BuildMaps,
    pub levels: CurrentCivilizationLevels<'a>,
    pub scratch: &'a [(i32, bool)],
    pub quests: &'a [crate::city_turn::religion_callbacks::CurrentQuest],
    pub achievements: signals::AchievementInputs<'a>,
    pub history: Option<&'a History<'a>>,
    /// Live activation scope from the same Session visibility-history owner.
    pub gossip_history_empty: Option<bool>,
}
/// Includes both the factory's pre-placement dispatch and the camp's additional
/// post-placement dispatch. Null originating cities remain actual None values.
#[allow(clippy::too_many_arguments)]
pub fn game_created<P, E: collections::Effects>(
    owners: &mut collections::Owners<'_>,
    collection_runtime: &mut collections::Runtime,
    effects: &mut E,
    script: &mut stock_script::Runtime<P>,
    inputs: &Inputs<'_>,
    consumers: &[Consumer],
    unit: Option<ComponentId>,
    city: Option<ComponentId>,
) -> Result<(), Error> {
    for consumer in consumers {
        if matches!(consumer, Consumer::HistoricalSummary) {
            continue;
        }
        let reference = UnitReference::from_component_id(
            unit.ok_or(Error::Missing("nonnull stock UnitCreated Unit"))?,
        );
        match *consumer {
            Consumer::HistoricalSummary => unreachable!(),
            Consumer::Script => signals::script_created(owners.state, script, reference, city)
                .map_err(Error::Signal)?,
            Consumer::SettlerGossip => inputs
                .signals
                .settler_created_with_gossip_history(
                    owners.state,
                    &inputs.levels,
                    reference,
                    inputs.gossip_history_empty,
                )
                .map_err(Error::Signal)?,
            Consumer::TrainedGossip => inputs
                .signals
                .trained_created(owners.state, &inputs.levels, reference)
                .map_err(Error::Signal)?,
            Consumer::History => inputs
                .signals
                .history_created(
                    owners.state,
                    inputs.build_maps,
                    inputs.scratch,
                    reference,
                    &mut HistoryEffects(inputs.history),
                )
                .map_err(Error::Signal)?,
            Consumer::TrainQuests => {
                signals::train_quests_created(owners.state, inputs.levels.binding, inputs.quests)
                    .map_err(Error::Signal)?
            }
            Consumer::Boost { kind, row } => inputs
                .signals
                .boost_created(
                    owners.state,
                    inputs.combat,
                    inputs.build_maps,
                    kind,
                    row,
                    reference,
                )
                .map_err(Error::Signal)?,
            Consumer::Collection(route) => collections::on_unit_created(
                owners,
                collection_runtime,
                effects,
                inputs.definitions,
                route,
                reference,
            )
            .map_err(Error::Collection)?,
            Consumer::Achievement => inputs
                .signals
                .achievement_created(
                    owners.state,
                    inputs.combat,
                    reference,
                    signals::AchievementInputs {
                        context_present: inputs.achievements.context_present,
                        integrity: inputs.achievements.integrity,
                        editor: inputs.achievements.editor,
                        ruleset: inputs.achievements.ruleset,
                        civilization: inputs.achievements.civilization,
                    },
                )
                .map_err(Error::Signal)?,
        }
    }
    Ok(())
}
pub fn player_created(
    state: &mut LoadedState,
    definitions: &signals::Definitions,
    consumers: &[PlayerConsumer],
    owner: i32,
    unit: Option<ComponentId>,
) -> Result<(), Error> {
    for consumer in consumers {
        match consumer {
            PlayerConsumer::Control => definitions.player_control_created(state, owner),
            PlayerConsumer::Economic => definitions.economic_created(
                state,
                UnitReference::from_component_id(
                    unit.ok_or(Error::Missing("nonnull stock PlayerUnitCreated Unit"))?,
                ),
            ),
        }
        .map_err(Error::Signal)?;
    }
    Ok(())
}
