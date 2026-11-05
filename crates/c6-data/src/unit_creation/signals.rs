//! Current UnitCreated rule and Script leaves shared by production callers.
use super::*;
use crate::{
    loaded_actions::stock_script, movement_placement_state::CurrentCivilizationLevels, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, map::UnitReference};
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Script(stock_script::Error),
    Rules(crate::Error),
    Missing(&'static str),
    Control(c6_core::loaded_state::unit_creation_control::Error),
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    eras: LinuxDefinitionRegistry,
    found_city: Vec<bool>,
    makes_trade_route: Vec<bool>,
    prerequisite_era: Vec<i32>,
    obsolete: i32,
    history_special: Vec<bool>,
    boosts: Vec<Boost>,
    replacements: Vec<(i32, i32)>,
}
struct Boost {
    row: i64,
    unit: Option<i32>,
    items: i32,
    improvement: Option<u32>,
    resource: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum BoostCallback {
    Unit,
    Land,
    Corps,
    Armies,
    UnitAndImprovement,
}
/// Owned current unsaved Player.Units build-map projection. Missing owners or
/// cells stay unsupported; modifiers must maintain this same owner when added.
pub struct BuildMaps {
    pub binding: WireBinding,
    pub players: Vec<(i32, Vec<i32>)>,
}
pub struct AchievementInputs<'a> {
    pub context_present: bool,
    pub integrity: bool,
    pub editor: bool,
    pub ruleset: &'a str,
    pub civilization: &'a str,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Missing("creation signal rules binding"));
        }
        let registry = |table, column| {
            collection(rules, table, column)
                .map_err(|_| Error::Missing("creation signal definition collection"))
        };
        let units = registry("Units", "UnitType")?;
        let eras = registry("Eras", "EraType")?;
        let technologies = registry("Technologies", "TechnologyType")?;
        let civics = registry("Civics", "CivicType")?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(Error::Missing("creation signal rules table"))
        };
        let reference_era = |unit: crate::Row<'_>,
                             column: &'static str,
                             refs: &LinuxDefinitionRegistry,
                             table_name: &'static str|
         -> Result<Option<(i32, i32)>, Error> {
            let name = match unit.get(column).map_err(Error::Rules)? {
                SqlValue::Null => return Ok(None),
                SqlValue::Text(v) => v,
                _ => return Err(Error::Missing("Unit prerequisite reference")),
            };
            let Some(index) = refs.find_index(crate::registry::linux_type_hash(name)) else {
                return Ok(None);
            };
            let row = table(table_name)?
                .row_by_id(refs.entries()[index].database_rowid)
                .ok_or(Error::Missing("prerequisite row"))?;
            let name = match row.get("EraType").map_err(Error::Rules)? {
                SqlValue::Null => return Ok(None),
                SqlValue::Text(v) => v,
                _ => return Err(Error::Missing("prerequisite Era reference")),
            };
            let Some(index) = eras.find_index(crate::registry::linux_type_hash(name)) else {
                return Ok(None);
            };
            let row = table("Eras")?
                .row_by_id(eras.entries()[index].database_rowid)
                .ok_or(Error::Missing("Era row"))?;
            Ok(Some((
                i32::try_from(row.integer("ChronologyIndex").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("Era chronology"))?,
                index as i32,
            )))
        };
        let mut found_city = Vec::new();
        let mut makes_trade_route = Vec::new();
        let mut history_special = Vec::new();
        let mut prerequisite_era = Vec::new();
        for entry in units.entries() {
            let row = table("Units")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Unit row"))?;
            found_city.push(row.integer("FoundCity").map_err(Error::Rules)? != 0);
            makes_trade_route.push(row.integer("MakeTradeRoute").map_err(Error::Rules)? != 0);
            history_special.push(
                !matches!(row.get("TraitType").map_err(Error::Rules)?, SqlValue::Null)
                    || !matches!(
                        row.get("StrategicResource").map_err(Error::Rules)?,
                        SqlValue::Null
                    ),
            );
            //26adee0: technology first, civic only on strictly greater chronology.
            let mut best = (i32::MIN, -1);
            for candidate in [
                reference_era(row, "PrereqTech", &technologies, "Technologies")?,
                reference_era(row, "PrereqCivic", &civics, "Civics")?,
            ]
            .into_iter()
            .flatten()
            {
                if candidate.0 > best.0 {
                    best = candidate;
                }
            }
            prerequisite_era.push(best.1);
        }
        let gossip = table("Gossips")?
            .rows()
            .find(|r| matches!(r.get("GossipType"),Ok(SqlValue::Text(v))if v==b"GOSSIP_TRAIN_UNIT"))
            .ok_or(Error::Missing("GOSSIP_TRAIN_UNIT definition"))?;
        let obsolete = i32::try_from(gossip.integer("ErasUntilObsolete").map_err(Error::Rules)?)
            .map_err(|_| Error::Missing("gossip obsolete eras"))?;
        let optional_hash =
            |row: crate::Row<'_>, column: &'static str| -> Result<Option<u32>, Error> {
                match row.get(column).map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    SqlValue::Text(v) => Ok(Some(crate::registry::linux_type_hash(v))),
                    _ => Err(Error::Missing("Boost reference type")),
                }
            };
        let mut boosts = Vec::new();
        for row in table("Boosts")?.rows() {
            boosts.push(Boost {
                row: row.row_id().ok_or(Error::Missing("Boost row identity"))?.0,
                unit: optional_hash(row, "Unit1Type")?
                    .and_then(|hash| units.find_index(hash))
                    .map(|i| i as i32),
                items: i32::try_from(row.integer("NumItems").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("Boost item count"))?,
                improvement: optional_hash(row, "ImprovementType")?,
                resource: optional_hash(row, "ResourceType")?,
            });
        }
        let mut replacements = Vec::new();
        for row in table("UnitReplaces")?.rows() {
            if let (Some(a), Some(b)) = (
                optional_hash(row, "ReplacesUnitType")?.and_then(|h| units.find_index(h)),
                optional_hash(row, "CivUniqueUnitType")?.and_then(|h| units.find_index(h)),
            ) {
                replacements.push((a as i32, b as i32));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            units,
            eras,
            found_city,
            makes_trade_route,
            prerequisite_era,
            obsolete,
            history_special,
            boosts,
            replacements,
        })
    }
    ///201dc60: active-player control may only schedule a new unit for AI
    /// players. Human/inactive branches return before controller iteration.
    pub fn player_control_created(&self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        if state.binding() != &self.binding {
            return Err(Error::Missing("creation control binding"));
        }
        if !state.turn_active(owner).map_err(Error::State)? {
            return Ok(());
        }
        if state
            .partial_player_controls(owner)
            .map_err(Error::State)?
            .slot_status
            == 3
        {
            return Ok(());
        }
        state
            .schedule_created_unit_control(owner)
            .map_err(Error::Control)
    }
    ///20a8520→21b5120: archive edit has no gameplay side effect; non-Trader
    /// returns before changing the new unit's controller or querying movement.
    pub fn economic_created(&self, state: &LoadedState, unit: UnitReference) -> Result<(), Error> {
        self.actor(state, unit)?;
        let index = state
            .unit_runtime(unit.player(), unit.id)
            .map_err(Error::State)?
            .definition_index;
        if !*self
            .makes_trade_route
            .get(usize::try_from(index).map_err(|_| Error::Missing("trade Unit index"))?)
            .ok_or(Error::Missing("trade Unit definition"))?
        {
            return Ok(());
        }
        Err(Error::Missing("new Trade unit controller assignment"))
    }
    ///28fa530: qualify the ordinary creation branch without emulating native
    /// achievement-local counters. Those counters feed only platform awards;
    /// a reached award path is still explicit here rather than invoking a host.
    pub fn achievement_created(
        &self,
        state: &LoadedState,
        combat: &crate::formation_rules::Definitions,
        unit: UnitReference,
        inputs: AchievementInputs<'_>,
    ) -> Result<(), Error> {
        let integrity = inputs.context_present && inputs.integrity;
        // CanUnlock evaluates editor and tutorial even when integrity is false.
        let tutorial = inputs.ruleset == "RULESET_TUTORIAL";
        if !(integrity && !inputs.editor && !tutorial) {
            return Ok(());
        }
        let actor = self.actor(state, unit)?;
        if actor.e_owner == -1 {
            return Ok(());
        }
        let current = state
            .unit_runtime(unit.player(), unit.id)
            .map_err(Error::State)?;
        let entry = self
            .units
            .entries()
            .get(
                usize::try_from(current.definition_index)
                    .map_err(|_| Error::Missing("achievement Unit index"))?,
            )
            .ok_or(Error::Missing("achievement Unit definition"))?;
        // Native +e8 increments for non-Great units. This is presentation-only
        // achievement progress, not Player.Stats or Game.Stats bookkeeping.
        if inputs.ruleset == "RULESET_SCENARIO_BLACKDEATH"
            && entry.type_name == b"UNIT_PLAGUE_DOCTOR"
        {
            return Err(Error::Missing(
                "plague-doctor platform achievement progress",
            ));
        }
        if inputs.civilization == "CIVILIZATION_SCYTHIA"
            && entry.type_name == b"UNIT_SCYTHIAN_HORSE_ARCHER"
        {
            return Err(Error::Missing("Scythian horse-archer achievement count"));
        }
        let mut army = 0_i32;
        let mut armada = 0_i32;
        for row in state
            .world()
            .player(actor.e_owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?
        {
            let value = &row.value;
            let runtime = state
                .unit_runtime(value.e_owner, value.id)
                .map_err(Error::State)?;
            if runtime.domain != 2 && runtime.domain != 0 {
                continue;
            }
            let combat_unit = combat
                .combat(state, value)
                .map_err(|_| Error::Missing("achievement combat"))?
                > 0
                || combat
                    .ranged(state, value)
                    .map_err(|_| Error::Missing("achievement ranged"))?
                    > 0
                || combat
                    .bombard(state, value)
                    .map_err(|_| Error::Missing("achievement bombard"))?
                    > 0;
            if !combat_unit || value.b_delayed_death || value.e_military_formation != 2 {
                continue;
            }
            if runtime.domain == 2 {
                army = army.wrapping_add(1)
            } else {
                armada = armada.wrapping_add(1)
            }
        }
        if army > 0 && armada >= 2 {
            return Err(Error::Missing("army and armada platform achievement"));
        }
        Ok(())
    }
    ///29e7ad0: current major/scratch/levied/hero guards, then unique/resource
    /// moments and finally naval/air moments. Ordinary land Builder has neither.
    pub fn history_created(
        &self,
        state: &LoadedState,
        levels: &CurrentCivilizationLevels<'_>,
        scratch: &[(i32, bool)],
        unit: UnitReference,
    ) -> Result<(), Error> {
        let actor = self.actor(state, unit)?;
        if !self.major(state, levels, actor.e_owner)? {
            return Ok(());
        }
        if scratch
            .iter()
            .find(|p| p.0 == actor.e_owner)
            .ok_or(Error::Missing("current scratch flag"))?
            .1
            || actor.b_is_levied
            || actor.k_hero.e_class != u32::MAX
        {
            return Ok(());
        }
        let current = state
            .unit_runtime(unit.player(), unit.id)
            .map_err(Error::State)?;
        let index = usize::try_from(current.definition_index)
            .map_err(|_| Error::Missing("history Unit definition"))?;
        if *self
            .history_special
            .get(index)
            .ok_or(Error::Missing("history Unit references"))?
        {
            return Err(Error::Missing("unique or strategic-resource Unit moment"));
        }
        if matches!(current.domain, 0 | 1) {
            return Err(Error::Missing("first naval/air Unit moment"));
        }
        Ok(())
    }
    /// Source Boost UnitCreated/count callback batch. Queries read the current
    /// roster on every callback; a reached TriggerBoost remains required.
    pub fn boost_created(
        &self,
        state: &LoadedState,
        combat: &crate::formation_rules::Definitions,
        maps: &BuildMaps,
        kind: BoostCallback,
        row: i64,
        unit: UnitReference,
    ) -> Result<(), Error> {
        let actor = self.actor(state, unit)?;
        let boost = self
            .boosts
            .iter()
            .find(|b| b.row == row)
            .ok_or(Error::Missing("current Boost definition row"))?;
        let owner = actor.e_owner;
        let records = state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?;
        let domain = |u: &Unit| {
            state
                .unit_runtime(owner, u.id)
                .map(|r| r.domain)
                .map_err(Error::State)
        };
        let is_combat = |u: &Unit| -> Result<bool, Error> {
            Ok(combat
                .combat(state, u)
                .map_err(|_| Error::Missing("current unit combat"))?
                > 0
                || combat
                    .ranged(state, u)
                    .map_err(|_| Error::Missing("current unit ranged combat"))?
                    > 0
                || combat
                    .bombard(state, u)
                    .map_err(|_| Error::Missing("current unit bombard combat"))?
                    > 0)
        };
        let count_type = |target: i32| -> Result<i32, Error> {
            let mut count = 0_i32;
            for r in records {
                if state
                    .unit_runtime(owner, r.value.id)
                    .map_err(Error::State)?
                    .definition_index
                    == target
                {
                    count = count.wrapping_add(1);
                }
            }
            Ok(count)
        };
        let build_type = |target: i32| -> Result<i32, Error> {
            if target < 0 {
                return Ok(-1);
            }
            if maps.binding != *state.binding() {
                return Err(Error::Missing("current build map binding"));
            }
            maps.players
                .iter()
                .find(|r| r.0 == owner)
                .and_then(|r| r.1.get(target as usize))
                .copied()
                .ok_or(Error::Missing("current Player unit build map cell"))
        };
        match kind {
            BoostCallback::Unit | BoostCallback::UnitAndImprovement => {
                let Some(target) = boost.unit else {
                    return Ok(());
                };
                let built = build_type(target)?; // source query precedes type test
                if matches!(kind, BoostCallback::Unit) {
                    let actual = state
                        .unit_runtime(unit.player(), unit.id)
                        .map_err(Error::State)?
                        .definition_index;
                    if target != actual
                        && !self.replacements.iter().any(|&(a, b)| {
                            (a == target && b == actual) || (b == target && a == actual)
                        })
                    {
                        return Ok(());
                    }
                    if count_type(built)? < boost.items {
                        return Ok(());
                    }
                } else {
                    if count_type(built)? <= 0 {
                        return Ok(());
                    }
                    let improvement = boost
                        .improvement
                        .ok_or(Error::Missing("Boost improvement reference"))?;
                    for plot in &state.world().map().plots {
                        if plot
                            .city_attributes
                            .as_ref()
                            .map_or(-1, |a| i32::from(a.player))
                            != owner
                            || plot.improvement_hash != improvement
                        {
                            continue;
                        }
                        if boost.resource.is_none_or(|r| r == plot.resource_hash) {
                            return Err(Error::Missing("reached unit/improvement TriggerBoost"));
                        }
                    }
                    return Ok(());
                }
            }
            BoostCallback::Land | BoostCallback::Corps | BoostCallback::Armies => {
                let mut count = 0_i32;
                for domain_needed in if matches!(kind, BoostCallback::Armies) {
                    &[2, 0][..]
                } else {
                    &[2][..]
                } {
                    for record in records {
                        let u = &record.value;
                        if domain(u)? != *domain_needed || !is_combat(u)? || u.b_delayed_death {
                            continue;
                        }
                        let wanted = match kind {
                            BoostCallback::Corps => Some(1),
                            BoostCallback::Armies => Some(2),
                            _ => None,
                        };
                        if wanted.is_none_or(|v| u.e_military_formation == v) {
                            count = count.wrapping_add(1);
                        }
                    }
                }
                if count < boost.items {
                    return Ok(());
                }
            }
        }
        Err(Error::Missing("reached UnitCreated TriggerBoost"))
    }
    fn actor<'s>(&self, state: &'s LoadedState, unit: UnitReference) -> Result<&'s Unit, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Missing("UnitCreated rules binding"));
        }
        state
            .world()
            .unit(unit)
            .map_err(Error::World)?
            .map(|r| r.record)
            .ok_or(Error::Missing("UnitCreated retained Unit"))
    }
    fn major(
        &self,
        state: &LoadedState,
        levels: &CurrentCivilizationLevels<'_>,
        owner: i32,
    ) -> Result<bool, Error> {
        if levels.binding != state.binding() {
            return Err(Error::Missing("UnitCreated current civilization binding"));
        }
        state.world().player(owner).map_err(Error::World)?;
        Ok(levels
            .values
            .iter()
            .find(|r| r.0 == owner)
            .ok_or(Error::Missing("current civilization level"))?
            .1
            == 0x253718b0)
    }
    ///29c2140. Positive settlement gossip remains a real required effect.
    pub fn settler_created(
        &self,
        state: &LoadedState,
        levels: &CurrentCivilizationLevels<'_>,
        unit: UnitReference,
    ) -> Result<(), Error> {
        let actor = self.actor(state, unit)?;
        if !self.major(state, levels, actor.e_owner)? {
            return Ok(());
        }
        let index = usize::try_from(
            state
                .unit_runtime(unit.player(), unit.id)
                .map_err(Error::State)?
                .definition_index,
        )
        .map_err(|_| Error::Missing("Unit definition index"))?;
        if *self
            .found_city
            .get(index)
            .ok_or(Error::Missing("Unit FoundCity definition"))?
        {
            return Err(Error::Missing("settler gossip occurrence and delivery"));
        }
        Ok(())
    }
    ///29c2420. The ordinary Builder has no prerequisites (era−1), so the
    /// computed obsolete-era branch returns before any global statistics read.
    pub fn trained_created(
        &self,
        state: &LoadedState,
        levels: &CurrentCivilizationLevels<'_>,
        unit: UnitReference,
    ) -> Result<(), Error> {
        let actor = self.actor(state, unit)?;
        if !self.major(state, levels, actor.e_owner)? {
            return Ok(());
        }
        let index = usize::try_from(
            state
                .unit_runtime(unit.player(), unit.id)
                .map_err(Error::State)?
                .definition_index,
        )
        .map_err(|_| Error::Missing("Unit definition index"))?;
        if self.units.type_hash(index) != Some(actor.e_type_index) {
            return Err(Error::Missing("UnitCreated current definition"));
        }
        let era_hash = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == actor.e_owner)
            .ok_or(Error::Missing("current Player eras"))?
            .eras
            .era_hash;
        let player_era = self.eras.find_index(era_hash).map_or(-1, |i| i as i32);
        let unit_era = *self
            .prerequisite_era
            .get(index)
            .ok_or(Error::Missing("Unit prerequisite era"))?;
        if unit_era <= player_era.wrapping_sub(self.obsolete) {
            return Ok(());
        }
        Err(Error::Missing(
            "trained-unit gossip global count and occurrence",
        ))
    }
}
///2c9d3a0: re-read both object identities, append city arguments only when
/// present, and invoke the SAME persistent Script owner. Nonempty hooks refuse.
pub fn script_created<P>(
    state: &LoadedState,
    script: &mut stock_script::Runtime<P>,
    unit: UnitReference,
    city: Option<ComponentId>,
) -> Result<(), Error> {
    script_unit_event(state, script, unit, city, "UnitCreated")
}
///2c9d510 uses the same four identity arguments for UnitInitialized.
pub fn script_initialized<P>(
    state: &LoadedState,
    script: &mut stock_script::Runtime<P>,
    unit: UnitReference,
    city: Option<ComponentId>,
) -> Result<(), Error> {
    script_unit_event(state, script, unit, city, "UnitInitialized")
}
fn script_unit_event<P>(
    state: &LoadedState,
    script: &mut stock_script::Runtime<P>,
    unit: UnitReference,
    city: Option<ComponentId>,
    event: &str,
) -> Result<(), Error> {
    let actor = state
        .world()
        .unit(unit)
        .map_err(Error::World)?
        .ok_or(Error::Missing("UnitCreated Script Unit"))?
        .record;
    let mut args = vec![actor.e_owner, actor.id as i32];
    if let Some(city) = city {
        if city.component_type != 2 {
            return Err(Error::Missing("UnitCreated Script City type"));
        }
        let city = state
            .city(i32::from(city.owner), city.id)
            .map_err(Error::State)?;
        args.extend([city.header.owner, city.header.id as i32]);
    }
    script
        .call_empty(state, event, &args)
        .map_err(Error::Script)
}

///2c9e7e0 emits owner, ID and both coordinates through the same Script owner.
pub fn script_moved(
    state: &LoadedState,
    script: &mut stock_script::Runtime,
    unit: UnitReference,
    old: c6_core::unit::PlotCoord,
    new: c6_core::unit::PlotCoord,
) -> Result<(), Error> {
    let actor = state
        .world()
        .unit(unit)
        .map_err(Error::World)?
        .ok_or(Error::Missing("current moved Script Unit"))?
        .record;
    script
        .call_empty(
            state,
            "OnUnitMoved",
            &[actor.e_owner, actor.id as i32, old.x, old.y, new.x, new.y],
        )
        .map_err(Error::Script)
}

///2c980e0 obtains the current quest list before reading either argument. An
/// actually empty list has no quest-completion, callback or RNG effects.
pub fn train_quests_created(
    state: &LoadedState,
    binding: &WireBinding,
    quests: &[crate::city_turn::religion_callbacks::CurrentQuest],
) -> Result<(), Error> {
    if binding != state.binding() {
        return Err(Error::Missing("current quest binding"));
    }
    if !quests.is_empty() {
        return Err(Error::Missing(
            "current TrainUnitType quest filtering/completion",
        ));
    }
    Ok(())
}
///2901a00 resolves both plots, then returns if either is null. Initial creation
/// therefore exits before owner/diplomacy/distance or platform award queries.
pub fn initial_moved_achievement(
    state: &LoadedState,
    old: c6_core::unit::PlotCoord,
    new: c6_core::unit::PlotCoord,
) -> Result<(), Error> {
    let old = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(state, old)
        .map_err(|_| Error::Missing("achievement old plot lookup"))?;
    let new = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(state, new)
        .map_err(|_| Error::Missing("achievement new plot lookup"))?;
    let (Some(old), Some(new)) = (old, new) else {
        return Ok(());
    };
    let map = state.world().map();
    let a = crate::movement_placement_state::plot_owner(
        map.plots
            .get(old as usize)
            .ok_or(Error::Missing("achievement old plot"))?,
    )
    .map_err(|_| Error::Missing("achievement old owner"))?;
    let b = crate::movement_placement_state::plot_owner(
        map.plots
            .get(new as usize)
            .ok_or(Error::Missing("achievement new plot"))?,
    )
    .map_err(|_| Error::Missing("achievement new owner"))?;
    if a == b {
        Ok(())
    } else {
        Err(Error::Missing("cross-owner movement achievement"))
    }
}
