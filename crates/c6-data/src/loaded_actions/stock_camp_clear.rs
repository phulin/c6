//! Stock non-Clans camp dispersal reached synchronously by ordinary placement.
//! Mutable gameplay stays in LoadedState, the existing sight owner, and the
//! Session's temporarily loaned research owner. Unknown receivers fail visibly.
use crate::loaded_actions::{
    choose_tech,
    stock_notifications::{self as stock, Runtime},
};
use crate::{
    melee_attack::resolve::completion::{boosts::Trigger, history},
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    research, Rules, SqlValue,
};
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    map::PlotFlag,
    unit::{ComponentId, PlotCoord},
};
use std::cell::RefCell;
use stock::movement::Error;
fn error(value: impl std::fmt::Debug) -> Error {
    Error::Requirement(format!("camp clear: {value:?}"))
}
fn missing(value: &'static str) -> Error {
    Error::Missing(value)
}

pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    improvements: LinuxDefinitionRegistry,
    camp_hash: u32,
    eras: LinuxDefinitionRegistry,
    gold: i32,
    on_remove: Option<u32>,
    goody: Vec<bool>,
    modifier_improvements: Vec<u32>,
    units: Vec<(u32, bool, bool)>,
    speed: Option<i32>,
    boosts: Vec<(i32, usize)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules, speed: u32) -> Result<Self, Error> {
        let collection = |table, column| {
            crate::unit_initialization_definitions::collection(rules, table, column).map_err(error)
        };
        let improvements = collection("Improvements", "ImprovementType")?;
        let table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(missing("camp Improvements"))?;
        let integer = |row: crate::Row<'_>, column| -> Result<i32, Error> {
            row.integer(column)
                .map_err(error)?
                .try_into()
                .map_err(error)
        };
        let mut camp = None;
        let mut goody = Vec::new();
        for entry in improvements.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(missing("camp improvement row"))?;
            goody.push(integer(row, "Goody")? != 0);
            if camp.is_none() && integer(row, "BarbarianCamp")? != 0 {
                let on_remove = match row.get("ImprovementOnRemove").map_err(error)? {
                    SqlValue::Null => None,
                    SqlValue::Text(name) => Some(linux_type_hash(&name)),
                    _ => return Err(missing("camp ImprovementOnRemove type")),
                };
                camp = Some((
                    linux_type_hash(&entry.type_name),
                    integer(row, "DispersalGold")?,
                    on_remove,
                ));
            }
        }
        let (camp_hash, gold, on_remove) = camp.ok_or(missing("camp improvement"))?;
        let modifier_improvements = rules
            .gameplay()
            .table("ImprovementModifiers")
            .ok_or(missing("camp ImprovementModifiers"))?
            .rows()
            .map(|r| {
                r.text("ImprovementType")
                    .map(|s| linux_type_hash(s.as_bytes()))
                    .map_err(error)
            })
            .collect::<Result<_, _>>()?;
        let unit_defs = collection("Units", "UnitType")?;
        let unit_rows = rules
            .gameplay()
            .table("Units")
            .ok_or(missing("camp Units"))?;
        let tags = rules
            .gameplay()
            .table("TypeTags")
            .ok_or(missing("camp TypeTags"))?;
        let mut units = Vec::new();
        for entry in unit_defs.entries() {
            let row = unit_rows
                .row_by_id(entry.database_rowid)
                .ok_or(missing("camp unit row"))?;
            let mut religious = false;
            for tag in tags.rows() {
                if tag.text("Type").map_err(error)?.as_bytes() == entry.type_name
                    && tag.text("Tag").map_err(error)? == "CLASS_RELIGIOUS_ALL"
                {
                    religious = true;
                }
            }
            units.push((
                linux_type_hash(&entry.type_name),
                integer(row, "MakeTradeRoute")? != 0,
                religious,
            ));
        }
        let speed = if speed == u32::MAX {
            None
        } else {
            let speeds = collection("GameSpeeds", "GameSpeedType")?;
            let entry = speeds.find(speed).ok_or(missing("camp game speed"))?;
            Some(integer(
                rules
                    .gameplay()
                    .table("GameSpeeds")
                    .and_then(|t| t.row_by_id(entry.database_rowid))
                    .ok_or(missing("camp game speed row"))?,
                "CostMultiplier",
            )?)
        };
        let mut boosts = Vec::new();
        for (index, row) in rules
            .gameplay()
            .table("Boosts")
            .ok_or(missing("camp Boosts"))?
            .rows()
            .enumerate()
        {
            if row.text("BoostClass").map_err(error)? == "BOOST_TRIGGER_CLEAR_CAMP" {
                boosts.push((integer(row, "BoostID")?, index));
            }
        }
        let eras = collection("Eras", "EraType")?;
        Ok(Self {
            binding: state.binding().clone(),
            improvements,
            eras,
            camp_hash,
            gold,
            on_remove,
            goody,
            modifier_improvements,
            units,
            speed,
            boosts,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Destroyed {
    DetachModifiers,
    Resources(i32),
    Barbarian,
    Unsupported(u64),
}
#[derive(Clone, Copy, Debug)]
pub enum Dispersed {
    Gossip,
    History,
    Quests,
    Boost(i32),
    PlayerStats(i32),
    Unsupported(u64),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Report {
    Activated {
        actor: ComponentId,
        plot: u32,
    },
    Dispersed {
        actor: ComponentId,
        plot: u32,
        gold: i32,
    },
    OwnerChanged {
        plot: u32,
        owner: i32,
    },
    Removed {
        plot: u32,
    },
}
/// All registrations and current services belong to this initialized instance.
/// Stock report/cache routes must have no added synchronous gameplay consumers.
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub destroyed: Vec<Destroyed>,
    pub dispersed: Vec<Dispersed>,
    pub tribe_destroyed: Vec<u64>,
    pub sight: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub history: &'a history::Definitions,
    pub history_inputs: history::Inputs<'a>,
    pub history_recorded: &'a [history::RecordedListener],
    pub gossip_history: &'a [c6_core::city_power_activation::gossip_delivery::TurnVisibility],
    pub quests: &'a RefCell<crate::game_update_state::round_quests::Runtime>,
    pub research: &'a research::Definitions,
    pub choices: &'a choose_tech::Definitions,
    pub culture: Option<&'a stock::research::CultureInputs<'a>>,
}
pub struct State<'a> {
    origin: ActionStamp,
    inputs: Inputs<'a>,
    pub reports: Vec<Report>,
    pub history_reports: Vec<crate::loaded_actions::government_actions::history::Report>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_camp_clear(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), Error> {
        if self.camp_clear.is_some() || state.binding() != &inputs.definitions.binding {
            return Err(missing("camp clear binding"));
        }
        self.camp_clear = Some(State {
            origin: state.action_stamp(),
            inputs,
            reports: vec![],
            history_reports: vec![],
        });
        Ok(())
    }
    pub fn camp_clear(&self) -> Option<&State<'a>> {
        self.camp_clear.as_ref()
    }
    pub(super) fn activate_stock_camp(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<bool, Error> {
        let mut camp = self
            .camp_clear
            .take()
            .ok_or(missing("camp clear services"))?;
        let result = camp.activate(state, sight, self, actor, plot);
        self.camp_clear = Some(camp);
        result
    }
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(missing("camp Player"))
}
fn location(state: &LoadedState, plot: u32) -> Result<PlotCoord, Error> {
    let map = state.world().map();
    if map.header.width <= 0 || plot as usize >= map.plots.len() {
        return Err(missing("camp plot"));
    }
    Ok(PlotCoord {
        x: plot as i32 % map.header.width,
        y: plot as i32 / map.header.width,
    })
}
impl State<'_> {
    fn activate<P>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        backend: &mut Runtime<'_, P>,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<bool, Error> {
        if !self.origin.same_instance(state.action_stamp())
            || state.binding() != &self.inputs.definitions.binding
        {
            return Err(missing("camp instance"));
        }
        let defs = self.inputs.definitions;
        let p = state
            .world()
            .map()
            .plots
            .get(plot as usize)
            .ok_or(missing("camp plot"))?;
        if p.improvement_hash != defs.camp_hash {
            return Err(missing("non-camp removable improvement"));
        }
        let unit = state
            .world()
            .player(actor.0)
            .map_err(error)?
            .unit(actor.1)
            .map_err(error)?
            .ok_or(missing("camp actor"))?
            .record;
        let (_, trade, religious) = defs
            .units
            .iter()
            .find(|u| u.0 == unit.e_type_index)
            .ok_or(missing("camp actor definition"))?;
        if *trade || *religious {
            return Ok(false);
        }
        let p_owner = player(state, actor.0)?;
        let replacement = p_owner
            .tail
            .as_ref()
            .ok_or(missing("camp Player tail"))?
            .improvement_replacements
            .iter()
            .rfind(|p| p.definition_hash == defs.camp_hash)
            .ok_or(missing("current camp replacement"))?
            .value;
        if replacement != u32::MAX
            && defs
                .improvements
                .find_index(replacement)
                .and_then(|i| defs.goody.get(i))
                .copied()
                .ok_or(missing("camp replacement definition"))?
        {
            return Err(missing("camp modified goody continuation"));
        }
        if defs.goody[defs
            .improvements
            .find_index(defs.camp_hash)
            .ok_or(missing("camp definition"))?]
        {
            return Err(missing("camp is goody hut"));
        }
        let identity = ComponentId {
            owner: actor.0 as i16,
            component_type: 1,
            id: actor.1,
        };
        self.reports.push(Report::Activated {
            actor: identity,
            plot,
        });
        let modifier = player(state, actor.0)?
            .treasury
            .gold_dispersal_change_q8
            .iter()
            .rfind(|p| p.definition_hash == defs.camp_hash)
            .ok_or(missing("camp Treasury dispersal change"))?
            .value
            >> 8;
        let raw = defs.gold.wrapping_add(modifier);
        let mut gold = 0;
        if raw != 0 {
            gold = c6_core::research_acquisition::units::adjust_for_game_speed(
                raw,
                defs.speed.ok_or(missing("camp speed multiplier"))?,
            );
            let old = player(state, actor.0)?.treasury.gold_balance_q8;
            let next = old.saturating_add(gold.wrapping_shl(8));
            if old != next {
                state.treasury_mut(actor.0).map_err(error)?.gold_balance_q8 = next;
                backend.deliver_gold_changed(state,actor.0,stock::gold::Scope::StockPlainHeadlessExternalPoliciesNoAddedReportConsumers).map_err(error)?;
            }
            let p = player(state, actor.0)?;
            if p.religion.faith_from_dispersal != 0 || p.header.healing_from_dispersal() > 0 {
                return Err(missing("positive camp faith/healing continuation"));
            }
        }
        self.reports.push(Report::Dispersed {
            actor: identity,
            plot,
            gold,
        });
        self.remove(state, sight, backend, plot)?;
        let value = &mut state
            .player_stats_mut(actor.0)
            .map_err(error)?
            .barbarian_camps_cleared;
        *value = value.wrapping_add(1);
        for callback in self.inputs.dispersed.clone() {
            match callback {
                Dispersed::Gossip if self.inputs.gossip_history.is_empty() => {}
                Dispersed::History => self.history(state, actor, plot)?,
                Dispersed::Quests => {
                    self.inputs.quests.borrow().validate(state).map_err(error)?;
                    if !self.inputs.quests.borrow().current.is_empty() {
                        return Err(missing("positive clear camp quest"));
                    }
                }
                Dispersed::Boost(row) => self.boost(state, backend, actor.0, row)?,
                Dispersed::PlayerStats(owner) => {
                    if owner != actor.0
                        && state
                            .game_relations()
                            .map_err(error)?
                            .has_met(owner, actor.0)
                            .map_err(error)?
                    {
                        let value = &mut state
                            .player_stats_mut(owner)
                            .map_err(error)?
                            .cleared_bandit_camp_counts[actor.0 as usize];
                        *value = value.wrapping_add(1);
                    }
                }
                _ => return Err(error(("unsupported camp dispersed receiver", callback))),
            }
        }
        if defs.on_remove.is_some() {
            return Err(missing("camp ImprovementOnRemove placement"));
        }
        Ok(true)
    }
    fn remove<P>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        backend: &mut Runtime<'_, P>,
        plot: u32,
    ) -> Result<(), Error> {
        let defs = self.inputs.definitions;
        let at = location(state, plot)?;
        let p = &state.world().map().plots[plot as usize];
        let owner = i32::from(p.improvement_owner);
        let area = p.area_id;
        let pillaged = p.flags.get(PlotFlag::ImprovementPillaged);
        if let Some(area) = state.world_mut().map_err(error)?.area_mut(area) {
            if let Some(v) = area
                .improvement_counts
                .iter_mut()
                .find(|p| p.hash == defs.camp_hash)
            {
                v.value = v.value.wrapping_sub(1);
            }
        }
        if owner != -1 {
            let source = ComponentId {
                owner: -1,
                component_type: 0,
                id: u32::MAX,
            };
            let mut plots = vec![plot as usize];
            for d in 0..6 {
                if let Some(p) =
                    crate::barbarian_camps::queries::adjacent(state, at, d).map_err(error)?
                {
                    plots.push(p);
                }
            }
            for p in plots {
                let mut context = crate::unit_sight_state::Context {
                    state,
                    runtime: sight,
                    effects: backend,
                    sight_definitions: self.inputs.sight,
                    actor: (-1, u32::MAX),
                };
                c6_core::visibility_change::change_visibility_count(
                    &mut context,
                    c6_core::unit_sight::VisibilityChange {
                        owner,
                        plot_index: p as u32,
                        delta: -1,
                        hidden_type: -1,
                        source_flag: true,
                        near_origin: false,
                        origin_index: -1,
                        sight_range: 1,
                        source,
                    },
                )
                .map_err(error)?;
            }
            state
                .remove_player_improvement(owner, plot as i32)
                .map_err(error)?;
            self.reports.push(Report::OwnerChanged { plot, owner });
        }
        let p = state
            .world_mut()
            .map_err(error)?
            .plot_mut(plot as usize)
            .ok_or(missing("camp plot"))?;
        p.improvement_owner = -1;
        p.improvement_hash = u32::MAX;
        p.flags.set(PlotFlag::ImprovementPillaged, false);
        self.reports.push(Report::OwnerChanged { plot, owner: -1 });
        for callback in self.inputs.destroyed.clone() {
            match callback {
                Destroyed::DetachModifiers
                    if !defs.modifier_improvements.contains(&defs.camp_hash) => {}
                Destroyed::Resources(_) => {
                    if state.world().map().plots[plot as usize].resource_hash != u32::MAX {
                        return Err(missing("resource-bearing camp removal"));
                    }
                }
                Destroyed::Barbarian => self.destroyed(state, plot)?,
                _ => {
                    return Err(error((
                        "unsupported camp removed receiver",
                        callback,
                        pillaged,
                    )))
                }
            }
        }
        self.reports.push(Report::Removed { plot });
        Ok(())
    }
    fn destroyed(&mut self, state: &mut LoadedState, plot: u32) -> Result<(), Error> {
        let at = location(state, plot)?;
        let manager = &state.retained().barbarian;
        if manager.flag_168_raw != 0
            || manager.improvement_hash != self.inputs.definitions.camp_hash
        {
            return Ok(());
        }
        let indices = manager
            .instances
            .iter()
            .enumerate()
            .filter(|(_, p)| p.camp_location.x == at.x && p.camp_location.y == at.y)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let owner = manager.player_ordinal;
        {
            let m = state.barbarian_manager_mut().map_err(error)?;
            m.value_e0 = m.value_e0.wrapping_sub(1);
        }
        for index in indices {
            state.barbarian_manager_mut().map_err(error)?.instances[index].living_raw = 0;
            // Native GetUnits(location, layer0) preserves membership order.
            let attacker = state
                .world()
                .plot_units(0, plot as usize)
                .map_err(error)?
                .iter()
                .find(|u| u.player() != owner)
                .map(|u| u.player());
            if let Some(attacker) = attacker {
                state.barbarian_manager_mut().map_err(error)?.instances[index]
                    .rampage_target_ordinal = attacker;
            }
            let tribe = state.retained().barbarian.instances[index].tribe_index;
            if state
                .world()
                .player(owner)
                .map_err(error)?
                .unit_records()
                .map_err(error)?
                .iter()
                .any(|p| p.value.e_ai_type == 4 && p.value.i_barbarian_tribe_index == tribe)
            {
                return Err(missing("living camp guard SetAIType continuation"));
            }
            if !self.inputs.tribe_destroyed.is_empty() {
                return Err(missing("positive BarbarianCampDestroyed receiver"));
            }
        }
        Ok(())
    }
    fn history(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Error> {
        let at = location(state, plot)?;
        let mut city = None;
        c6_core::unit_placement_tails::neighborhood(at, 6, |p| -> Result<(), Error> {
            if city.is_none() {
                city = state
                    .retained()
                    .players
                    .iter()
                    .filter(|v| v.header.owner == actor.0)
                    .flat_map(|v| &v.cities.records.entries)
                    .map(|v| &v.value)
                    .find(|c| c.header.location_x == p.x && c.header.location_y == p.y)
                    .map(|c| c.header.name.clone());
            }
            Ok(())
        })?;
        let mut effects = crate::loaded_actions::government_actions::history::StockEffects {
            recorded: self.inputs.history_recorded,
            reports: vec![],
        };
        history::handle_moment(
            state,
            self.inputs.history,
            self.inputs.history_inputs,
            c6_core::game_history::Moment {
                wire_version: c6_core::game_history::MOMENT_VERSION,
                type_hash: if city.is_some() {
                    0xcc33031a
                } else {
                    0xe1a9060b
                },
                turn: -1,
                action_plot: at,
                acting_player_ordinal: actor.0,
                id: -1,
                has_ever_been_commemorated_raw: 0,
                game_era_hash: u32::MAX,
                extra_data: vec![c6_core::game_history::MomentData {
                    kind_raw: 0x4de4292b,
                    value: self
                        .inputs
                        .definitions
                        .eras
                        .find_index(player(state, actor.0)?.eras.era_hash)
                        .map_or(-1, |v| v as i32),
                }],
                city_name_bytes: city.unwrap_or_default(),
                unit_citizen_name_bytes: vec![],
                other_city_name_bytes: vec![],
                era_score: 0,
            },
            &mut effects,
        )
        .map_err(error)?;
        self.history_reports.extend(effects.reports);
        Ok(())
    }
    fn boost<P>(
        &self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        row: i32,
    ) -> Result<(), Error> {
        let index = self
            .inputs
            .definitions
            .boosts
            .iter()
            .find(|p| p.0 == row)
            .ok_or(missing("camp captured Boost row"))?
            .1;
        let mut runtime = backend
            .action_research
            .take()
            .ok_or(missing("camp canonical research loan"))?;
        let result = {
            let mut effects = stock::research::Effects::new(
                backend,
                &mut runtime,
                self.inputs.choices,
                self.inputs.culture,
            );
            crate::melee_attack::resolve::completion::boosts::ResearchTrigger {
                definitions: self.inputs.research,
                effects: &mut effects,
            }
            .trigger(state, owner, index)
            .map_err(error)
        };
        backend.action_research = Some(runtime);
        result
    }
}
