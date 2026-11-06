//! Current Rules.Units.ViewUnit continuation for ordinary major observers.
//! Encounter flags are gameplay state. A newly encountered type synchronously
//! dispatches the bound, ordered GameSignals+0x5e8 consumers.
use crate::loaded_actions::{
    choose_tech,
    stock_notifications::{self as stock, Runtime},
};
use crate::melee_attack::resolve::completion::boosts::{ResearchTrigger, Trigger};
use crate::{registry::linux_type_hash, research, Rules, SqlValue};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
use stock::movement::Error;
fn missing(message: &'static str) -> Error {
    Error::Missing(message)
}
fn error(value: impl std::fmt::Debug) -> Error {
    Error::Requirement(format!("unit view: {value:?}"))
}
struct Boost {
    row: i32,
    index: usize,
    unit: Option<i32>,
}
pub struct Definitions {
    binding: WireBinding,
    boosts: Vec<Boost>,
    replacements: Vec<(i32, i32)>,
    unit_hashes: Vec<u32>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let units = crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(error)?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(missing("unit view rules table"))
        };
        let names = table("BoostNames")?;
        let mut boosts = Vec::new();
        for (index, row) in table("Boosts")?.rows().enumerate() {
            let class = row.text("BoostClass").map_err(error)?;
            let value = names
                .rows()
                .find(|n| n.text("BoostType").ok() == Some(class))
                .map(|n| n.integer("BoostValue"))
                .transpose()
                .map_err(error)?;
            // Linux Initialize's value-one jump target is the lambda15
            // subscription to EncounterUnit; unresolved BoostReference skips.
            if value != Some(1) {
                continue;
            }
            let unit = match row.get("Unit1Type").map_err(error)? {
                SqlValue::Null => None,
                SqlValue::Text(name) => units.find_index(linux_type_hash(name)).map(|i| i as i32),
                _ => return Err(missing("unit view Unit1Type")),
            };
            boosts.push(Boost {
                row: row
                    .integer("BoostID")
                    .map_err(error)?
                    .try_into()
                    .map_err(error)?,
                index,
                unit,
            });
        }
        let mut replacements = Vec::new();
        for row in table("UnitReplaces")?.rows() {
            let index = |column| -> Result<Option<i32>, Error> {
                Ok(units
                    .find_index(linux_type_hash(row.text(column).map_err(error)?.as_bytes()))
                    .map(|i| i as i32))
            };
            if let (Some(base), Some(replacement)) =
                (index("ReplacesUnitType")?, index("CivUniqueUnitType")?)
            {
                replacements.push((base, replacement));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            boosts,
            replacements,
            unit_hashes: units
                .entries()
                .iter()
                .map(|entry| linux_type_hash(&entry.type_name))
                .collect(),
        })
    }
    fn matching_boost(&self, row: i32, seen: i32) -> Result<Option<usize>, Error> {
        let boost = self
            .boosts
            .iter()
            .find(|b| b.row == row)
            .ok_or(missing("unit view encountered Boost row"))?;
        Ok(boost
            .unit
            .filter(|&base| {
                base == seen
                    || self
                        .replacements
                        .iter()
                        .any(|&(a, b)| (a == base && b == seen) || (a == seen && b == base))
            })
            .map(|_| boost.index))
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Consumer {
    /// Captured Boost definition row ID, preserving actual signal order.
    Boost(i32),
    Unsupported(u64),
}
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    /// Actual current complete subscriber list. Empty must be observed empty.
    pub consumers: Vec<Consumer>,
    pub research: &'a research::Definitions,
    pub choices: &'a choose_tech::Definitions,
    pub culture: Option<&'a stock::research::CultureInputs<'a>>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_first_contact_unit_view(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), Error> {
        self.check(state).map_err(error)?;
        if self.first_contact_unit_view.is_some() || inputs.definitions.binding != *state.binding()
        {
            return Err(missing("unit view binding"));
        }
        for consumer in &inputs.consumers {
            if let Consumer::Boost(row) = consumer {
                inputs.definitions.matching_boost(*row, -1)?;
            }
        }
        self.first_contact_unit_view = Some(inputs);
        Ok(())
    }
    pub(super) fn preflight_stock_encountered_unit(
        &self,
        state: &LoadedState,
        player: i32,
        index: i32,
    ) -> Result<(), Error> {
        let inputs = self
            .first_contact_unit_view
            .as_ref()
            .ok_or(missing("unit view current encounter subscribers"))?;
        if inputs.definitions.binding != *state.binding() {
            return Err(missing("unit view definitions binding"));
        }
        if inputs
            .consumers
            .iter()
            .any(|consumer| matches!(consumer, Consumer::Unsupported(_)))
        {
            return Err(missing("unit view unsupported encounter subscriber"));
        }
        let ordinal = usize::try_from(index).map_err(|_| missing("unit view type ordinal"))?;
        let hash = *inputs
            .definitions
            .unit_hashes
            .get(ordinal)
            .ok_or(missing("unit view type hash"))?;
        let retained = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == player)
            .and_then(|p| p.post_heroes_arrays.as_ref())
            .and_then(|a| {
                a.unit_definitions
                    .pairs
                    .iter()
                    .rev()
                    .find(|p| p.definition_hash == hash)
            })
            .ok_or(missing("unit view retained encountered flag"))?
            .value;
        let current = *self
            .player_services
            .as_ref()
            .and_then(|services| services.players.iter().find(|p| p.owner == player))
            .and_then(|p| p.encountered_units.as_ref())
            .and_then(|flags| flags.get(ordinal))
            .ok_or(missing("unit view current encountered flag"))?;
        if retained != current {
            return Err(missing("unit view encountered flag views disagree"));
        }
        Ok(())
    }
    pub(super) fn stock_signal_encountered_unit(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        index: i32,
    ) -> Result<(), Error> {
        let inputs = self
            .first_contact_unit_view
            .as_ref()
            .ok_or(missing("unit view current encounter subscribers"))?;
        let hash = *usize::try_from(index)
            .ok()
            .and_then(|ordinal| inputs.definitions.unit_hashes.get(ordinal))
            .ok_or(missing("unit view type hash"))?;
        // Player.SetHasEncountered writes Player+0x320. The retained hashed
        // unit table serializes this SAME array; update it before dispatch.
        state
            .player_discovery_mut(player)
            .map_err(Error::State)?
            .unit_definitions
            .pairs
            .iter_mut()
            .rev()
            .find(|p| p.definition_hash == hash)
            .ok_or(missing("unit view retained encountered flag"))?
            .value = true;
        let inputs = self
            .first_contact_unit_view
            .take()
            .ok_or(missing("unit view subscribers"))?;
        let result = (|| {
            for consumer in &inputs.consumers {
                match consumer {
                    Consumer::Boost(row) => {
                        if let Some(boost) = inputs.definitions.matching_boost(*row, index)? {
                            let mut runtime = self
                                .action_research
                                .take()
                                .ok_or(missing("unit view canonical research loan"))?;
                            let result = {
                                let mut effects = stock::research::Effects::new(
                                    self,
                                    &mut runtime,
                                    inputs.choices,
                                    inputs.culture,
                                );
                                ResearchTrigger {
                                    definitions: inputs.research,
                                    effects: &mut effects,
                                }
                                .trigger(state, player, boost)
                                .map_err(error)
                            };
                            self.action_research = Some(runtime);
                            result?;
                        }
                    }
                    Consumer::Unsupported(_) => {
                        return Err(missing("unit view unsupported encounter subscriber"))
                    }
                }
            }
            Ok(())
        })();
        self.first_contact_unit_view = Some(inputs);
        result
    }
    pub(super) fn stock_view_unit(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        unit: (i32, u32),
        _origin: i32,
        _first_reveal: bool,
    ) -> Result<(), Error> {
        self.check(state).map_err(error)?;
        // Basic ViewUnit's IsValid guard precedes GetType and array access.
        if state
            .world()
            .player_is_valid(player)
            .map_err(Error::World)?
        {
            let index = state
                .unit_runtime(unit.0, unit.1)
                .map_err(Error::State)?
                .definition_index;
            let ordinal =
                usize::try_from(index).map_err(|_| missing("unit view native type index"))?;
            let services = self
                .player_services
                .as_ref()
                .ok_or(missing("unit view current player services"))?;
            if services.binding != *state.binding() {
                return Err(missing("unit view player services binding"));
            }
            let known = *services
                .player::<Error>(player)
                .map_err(error)?
                .encountered_units
                .as_ref()
                .and_then(|v| v.get(ordinal))
                .ok_or(missing("unit view current encountered flag"))?;
            if !known {
                // Support admission precedes the flag write. Once admitted,
                // each callback observes true even if a later consumer fails.
                self.preflight_stock_encountered_unit(state, player, index)?;
                let flag = self
                    .player_services
                    .as_mut()
                    .ok_or(missing("unit view player services"))?
                    .players
                    .iter_mut()
                    .find(|p| p.owner == player)
                    .and_then(|p| p.encountered_units.as_mut())
                    .and_then(|v| v.get_mut(ordinal))
                    .ok_or(missing("unit view encountered flag"))?;
                *flag = true;
                self.stock_signal_encountered_unit(state, player, index)?;
            }
        }
        // Four-argument ViewUnit tests observer barbarian status after Basic.
        // No first_reveal read, policy decision, or RNG precedes this guard.
        let owner = state
            .world()
            .player(unit.0)
            .map_err(Error::World)?
            .unit(unit.1)
            .map_err(Error::World)?
            .ok_or(missing("unit view current unit"))?
            .record
            .e_owner;
        if owner != player {
            let services = self
                .player_services
                .as_ref()
                .ok_or(missing("unit view current player services"))?;
            if services.binding != *state.binding() {
                return Err(missing("unit view player services binding"));
            }
            if services
                .player::<Error>(player)
                .map_err(error)?
                .civilization_level
                .ok_or(missing("unit view civilization level"))?
                == 0xd959_ec24
            {
                return Err(missing("unit view barbarian clan continuation"));
            }
        }
        Ok(())
    }
}
