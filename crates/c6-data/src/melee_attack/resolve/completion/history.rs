//! Current History.HandleMoment and the ordinary land unit/unit combat moment.
//! Synchronous MomentRecorded/report/positive-score continuations are mandatory.
use super::super::GeneratedMembers;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_capabilities::Membership,
    game_history::{Moment, MomentData},
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
};
const BATTLE: u32 = 0xba979044;
struct Definition {
    hash: u32,
    score: Option<i32>,
    minimum: Option<i32>,
    maximum: Option<i32>,
    obsolete: Option<u32>,
    cooldown: Option<i32>,
}
pub struct Definitions {
    binding: WireBinding,
    eras: LinuxDefinitionRegistry,
    chronology: Vec<i32>,
    moments: Vec<Definition>,
    water: Vec<(u32, bool)>,
    combat_support_abilities: [Option<usize>; 3],
}
#[derive(Debug)]
pub enum DefinitionError {
    Binding,
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
}
impl std::fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "History definitions: {self:?}")
    }
}
impl std::error::Error for DefinitionError {}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(DefinitionError::Definitions)?;
        if state.binding() != bound.destination_binding() {
            return Err(DefinitionError::Binding);
        }
        let eras = crate::unit_initialization_definitions::collection(rules, "Eras", "EraType")
            .map_err(DefinitionError::Definitions)?;
        let era_table = rules
            .gameplay()
            .table("Eras")
            .ok_or(DefinitionError::Missing("Eras"))?;
        let chronology = eras
            .entries()
            .iter()
            .map(|e| {
                era_table
                    .row_by_id(e.database_rowid)
                    .ok_or(DefinitionError::Missing("Era row"))
                    .and_then(|r| integer(r, "ChronologyIndex"))
            })
            .collect::<Result<_, _>>()?;
        let mut moments = vec![];
        for row in rules
            .gameplay()
            .table("Moments")
            .ok_or(DefinitionError::Missing("Moments"))?
            .rows()
        {
            let reference = |column| -> Result<Option<u32>, DefinitionError> {
                Ok(match row.get(column).map_err(DefinitionError::Rules)? {
                    SqlValue::Null => None,
                    SqlValue::Text(name) => {
                        let hash = linux_type_hash(name);
                        eras.find_index(hash).map(|_| hash)
                    }
                    _ => return Err(DefinitionError::Missing(column)),
                })
            };
            moments.push(Definition {
                hash: linux_type_hash(
                    row.text("MomentType")
                        .map_err(DefinitionError::Rules)?
                        .as_bytes(),
                ),
                score: optional(row, "EraScore")?,
                minimum: reference("MinimumGameEra")?
                    .and_then(|h| eras.find_index(h))
                    .map(|i| i as i32),
                maximum: reference("MaximumGameEra")?
                    .and_then(|h| eras.find_index(h))
                    .map(|i| i as i32),
                obsolete: reference("ObsoleteEra")?,
                cooldown: optional(row, "RepeatTurnCooldown")?,
            });
        }
        let water = rules
            .gameplay()
            .table("Terrains")
            .ok_or(DefinitionError::Missing("Terrains"))?
            .rows()
            .map(|r| {
                Ok((
                    linux_type_hash(
                        r.text("TerrainType")
                            .map_err(DefinitionError::Rules)?
                            .as_bytes(),
                    ),
                    integer(r, "Water")? != 0,
                ))
            })
            .collect::<Result<_, _>>()?;
        let abilities = crate::unit_initialization_definitions::collection(
            rules,
            "UnitAbilities",
            "UnitAbilityType",
        )
        .map_err(DefinitionError::Definitions)?;
        let combat_support_abilities = [
            b"ABILITY_GREAT_ADMIRAL_STRENGTH".as_slice(),
            b"ABILITY_GREAT_GENERAL_STRENGTH".as_slice(),
            b"ABILITY_COMANDANTE_AOE_STRENGTH".as_slice(),
        ]
        .map(|name| abilities.find_index(linux_type_hash(name)));
        Ok(Self {
            combat_support_abilities,
            binding: state.binding().clone(),
            eras,
            chronology,
            moments,
            water,
        })
    }
    fn latest(&self, a: i32, b: i32) -> i32 {
        let (Some(ai), Some(bi)) = (
            self.eras.find_index(a as u32),
            self.eras.find_index(b as u32),
        ) else {
            return -1;
        };
        if self.chronology[ai] < self.chronology[bi] {
            b
        } else {
            a
        }
    }
    fn era_index<E>(&self, hash: u32) -> Result<i32, Error<E>> {
        if hash == u32::MAX {
            return Ok(-1);
        }
        self.eras
            .find_index(hash)
            .map(|i| i as i32)
            .ok_or(Error::Missing("current Player era definition"))
    }
}
fn integer(row: crate::Row<'_>, column: &'static str) -> Result<i32, DefinitionError> {
    crate::game_parameters::sql_integer(row.get(column).map_err(DefinitionError::Rules)?)
        .ok_or(DefinitionError::Missing(column))
}
fn optional(row: crate::Row<'_>, column: &'static str) -> Result<Option<i32>, DefinitionError> {
    if matches!(
        row.get(column).map_err(DefinitionError::Rules)?,
        SqlValue::Null
    ) {
        Ok(None)
    } else {
        integer(row, column).map(Some)
    }
}
#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub capabilities: &'a Membership,
    pub civilization_levels: &'a [(i32, u32)],
    pub start_era_hash: Option<u32>,
}
pub trait Effects {
    type Error;
    fn moment_recorded(
        &mut self,
        state: &mut LoadedState,
        id: i32,
        kind: u32,
        owner: i32,
    ) -> Result<(), Self::Error>;
    /// Source event0x4710f1a4, first=id, second=type, after MomentRecorded.
    fn report(&mut self, state: &mut LoadedState, id: i32, kind: u32) -> Result<(), Self::Error>;
    fn change_era_score(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), Self::Error>;
    fn era_score_notice(
        &mut self,
        state: &mut LoadedState,
        moment: &Moment,
    ) -> Result<(), Self::Error>;
    /// Native Economic.AdjustForGameSpeed, queried only for positive cooldowns.
    fn adjust_cooldown(&mut self, _: &LoadedState, _: i32) -> Result<Option<i32>, Self::Error> {
        Ok(None)
    }
}
#[derive(Debug)]
pub enum Error<E> {
    Binding,
    History(c6_core::loaded_state::game_history::AccessError),
    Eras(c6_core::loaded_state::game_eras::AccessError),
    Actor(crate::melee_attack::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Missing(&'static str),
    SpecializedCombatMoment,
    NavalCombatMoment,
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "current History: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
/// General manager entry, shared with City-created moments. No callback is
/// skipped because it is absent from the serialized History list.
pub fn handle_moment<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    inputs: Inputs<'_>,
    mut moment: Moment,
    effects: &mut E,
) -> Result<Option<i32>, Error<E::Error>> {
    if state.binding() != &defs.binding {
        return Err(Error::Binding);
    }
    if !inputs.capabilities.has(0xdf1fe6a1) {
        return Ok(None);
    }
    let owner = moment.acting_player_ordinal;
    if owner == -1 {
        return Ok(None);
    }
    let level = inputs
        .civilization_levels
        .iter()
        .find(|r| r.0 == owner)
        .ok_or(Error::Missing("current History Player level"))?
        .1;
    if level != 0x253718b0 {
        return Ok(None);
    }
    let Some(definition) = defs.moments.iter().rfind(|r| r.hash == moment.type_hash) else {
        return Ok(None);
    };
    let current_era = state
        .game_eras()
        .map_err(Error::Eras)?
        .current_era()
        .ok_or(Error::Missing("current Game era"))?;
    if let Some(min) = definition.minimum {
        if min != current_era && defs.latest(min, current_era) == min {
            return Ok(None);
        }
    }
    if let Some(max) = definition.maximum {
        if max != current_era && defs.latest(max, current_era) == current_era {
            return Ok(None);
        }
    }
    if let Some(obsolete) = definition.obsolete {
        let start = inputs
            .start_era_hash
            .ok_or(Error::Missing("current starting era"))? as i32;
        if obsolete as i32 == start || defs.latest(obsolete as i32, start) == start {
            return Ok(None);
        }
    }
    let turn = state.retained().core.current_turn;
    if let Some(cooldown) = definition.cooldown.filter(|v| *v > 0) {
        let adjusted = effects
            .adjust_cooldown(state, cooldown)
            .map_err(Error::Effect)?
            .ok_or(Error::Missing("current moment cooldown adjustment"))?;
        let threshold = turn.wrapping_sub(adjusted);
        for previous in state
            .game_history()
            .map_err(Error::History)?
            .moments
            .iter()
            .rev()
        {
            if previous.turn < threshold {
                break;
            }
            if previous.type_hash == moment.type_hash && previous.acting_player_ordinal == owner {
                return Ok(None);
            }
        }
    }
    if moment.city_name_bytes.is_empty()
        && c6_core::geometry::plot_valid(moment.action_plot.x, moment.action_plot.y)
    {
        if let Some(city) = state
            .retained()
            .players
            .iter()
            .flat_map(|p| p.cities.records.entries.iter())
            .map(|r| &r.value)
            .find(|c| {
                c.header.location_x == moment.action_plot.x
                    && c.header.location_y == moment.action_plot.y
            })
        {
            moment.city_name_bytes = city.header.name.clone();
        }
    }
    moment.era_score = match definition.score {
        Some(base) => state
            .game_era_pride_score(owner, base)
            .map_err(Error::Eras)?,
        None => 0,
    };
    moment.turn = turn;
    moment.game_era_hash = if current_era == -1 {
        u32::MAX
    } else {
        defs.eras
            .type_hash(current_era as u32 as usize)
            .ok_or(Error::Missing("current Game era definition"))?
    };
    let id = state
        .record_historic_moment(moment.clone())
        .map_err(Error::History)?;
    moment.id = id;
    effects
        .moment_recorded(state, id, moment.type_hash, owner)
        .map_err(Error::Effect)?;
    effects
        .report(state, id, moment.type_hash)
        .map_err(Error::Effect)?;
    if moment.era_score > 0 {
        effects
            .change_era_score(state, owner, moment.era_score, 0)
            .map_err(Error::Effect)?;
        effects
            .era_score_notice(state, &moment)
            .map_err(Error::Effect)?;
    }
    Ok(Some(id))
}
/// Typed ordinary land branch. Positive defeated-defender special moments and naval
/// kill moments remain explicit rather than being silently omitted.
pub fn on_combat<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    inputs: Inputs<'_>,
    members: &GeneratedMembers,
    effects: &mut E,
) -> Result<Option<i32>, Error<E::Error>> {
    // Combat.Member.GetPlayer returns its stored ComponentID owner. Earlier
    // subscribers may have changed the live Unit owner; that is a different query.
    let attacker = members.damage.attacker.player();
    let defender = members.damage.defender.player();
    if !state
        .world()
        .player_is_valid(defender)
        .map_err(Error::World)?
    {
        return Ok(None);
    }
    if members.damage.defender_final_damage >= 100 {
        // Native29e939c checks formation disadvantage and a two-level XP
        // disadvantage before current support abilities. Equal ordinary Units
        // reach no specialized moment and still record the general battle.
        let a = crate::melee_attack::actor(state, members.damage.attacker).map_err(Error::Actor)?;
        let d = crate::melee_attack::actor(state, members.damage.defender).map_err(Error::Actor)?;
        if a.e_military_formation != -1
            && d.e_military_formation != -1
            && ((a.e_military_formation == 0 && d.e_military_formation != 0)
                || (a.e_military_formation == 1 && d.e_military_formation == 2))
            || a.k_experience.i_level.wrapping_add(2) <= d.k_experience.i_level
        {
            return Err(Error::SpecializedCombatMoment);
        }
        if c6_core::geometry::plot_valid(a.k_location.x, a.k_location.y) {
            let runtime = state.unit_runtime(a.e_owner, a.id).map_err(Error::State)?;
            for index in defs.combat_support_abilities.into_iter().flatten() {
                if runtime.abilities.get(index).copied().unwrap_or(0) > 0 {
                    return Err(Error::SpecializedCombatMoment);
                }
            }
        }
    }
    let plot = c6_core::movement_world::plot_at(state.world().map(), members.location)
        .map_err(|_| Error::Missing("combat history location"))?
        .1;
    let water = defs
        .water
        .iter()
        .rfind(|r| r.0 == plot.terrain_hash)
        .ok_or(Error::Missing("combat history Terrain"))?
        .1;
    if water && members.damage.attacker_final_damage >= 100 {
        return Err(Error::NavalCombatMoment);
    }
    general_battle(
        state,
        defs,
        inputs,
        attacker,
        defender,
        members.location,
        effects,
    )
}
/// District attacks skip the Unit/Unit special-moment block. The native common
/// tail still records a battle, including attacks against barbarians.
pub fn on_district_combat<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    inputs: Inputs<'_>,
    members: &crate::city_strike::Generated,
    effects: &mut E,
) -> Result<Option<i32>, Error<E::Error>> {
    let attacker = i32::from(members.attacker.owner);
    let defender = members.defender.player();
    if !state
        .world()
        .player_is_valid(defender)
        .map_err(Error::World)?
    {
        return Ok(None);
    }
    let plot = c6_core::movement_world::plot_at(state.world().map(), members.location)
        .map_err(|_| Error::Missing("combat history location"))?
        .1;
    let water = defs
        .water
        .iter()
        .rfind(|r| r.0 == plot.terrain_hash)
        .ok_or(Error::Missing("combat history Terrain"))?
        .1;
    // District generation explicitly writes attacker final damage to zero.
    if water && members.defender_final_damage >= 100 {
        return Err(Error::NavalCombatMoment);
    }
    general_battle(
        state,
        defs,
        inputs,
        attacker,
        defender,
        members.location,
        effects,
    )
}
pub(crate) fn general_battle<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    inputs: Inputs<'_>,
    attacker: i32,
    defender: i32,
    location: c6_core::unit::PlotCoord,
    effects: &mut E,
) -> Result<Option<i32>, Error<E::Error>> {
    let era = |owner| -> Result<i32, Error<E::Error>> {
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("current History Player"))?;
        defs.era_index(p.eras.era_hash)
    };
    let moment = Moment {
        wire_version: c6_core::game_history::MOMENT_VERSION,
        type_hash: BATTLE,
        turn: -1,
        action_plot: location,
        acting_player_ordinal: attacker,
        id: -1,
        has_ever_been_commemorated_raw: 0,
        game_era_hash: u32::MAX,
        extra_data: vec![
            MomentData {
                kind_raw: 0x4de4292b,
                value: era(attacker)?,
            },
            MomentData {
                kind_raw: 0xa6fe45c4,
                value: defender,
            },
            MomentData {
                kind_raw: 0x569de29b,
                value: era(defender)?,
            },
        ],
        city_name_bytes: Vec::new(),
        unit_citizen_name_bytes: Vec::new(),
        other_city_name_bytes: Vec::new(),
        era_score: 0,
    };
    handle_moment(state, defs, inputs, moment, effects)
}
/// Source-qualified stock output-only subscribers. Current subscription order
/// remains caller-owned; custom listeners require a concrete implementation.
#[derive(Clone, Copy, Debug)]
pub enum RecordedListener {
    EraLog,
    Achievement,
    Unsupported(u64),
}
#[derive(Debug)]
pub enum RecordedError {
    AchievementMoment,
    Unknown(u64),
}
pub fn stock_recorded(listeners: &[RecordedListener], kind: u32) -> Result<(), RecordedError> {
    for listener in listeners {
        match *listener {
            RecordedListener::EraLog => {} // Pure getter/string/log output; no retained writes/RNG.
            RecordedListener::Achievement => {
                if kind == 0x17cd04dd {
                    return Err(RecordedError::AchievementMoment);
                }
            }
            RecordedListener::Unsupported(id) => return Err(RecordedError::Unknown(id)),
        }
    }
    Ok(())
}
