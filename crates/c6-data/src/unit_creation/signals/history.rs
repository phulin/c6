//! Linux UnitCreatedMomentDispatcher29e7ad0, using the one current History owner.
//! Moment admission, recording, subscribers and era score are synchronous caller
//! effects; a request must never be mistaken for a completed recorded moment.
use super::{BuildMaps, Error};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    map::UnitReference,
    movement_world::DefinitionLookup,
    unit::PlotCoord,
};

const UNIT: u32 = 0xa136e882;
const RESOURCE: u32 = 0x3bda8bb9;
const PLAYER_ERA: u32 = 0x4de4292b;
const UNIQUE: u32 = 0x69770679;
const RESOURCE_WORLD: u32 = 0x60c5d5f2;
const RESOURCE_PLAYER: u32 = 0xdcff48fb;

struct UnitDefinition {
    trait_hash: Option<u32>,
    resource_index: Option<i32>,
}
pub struct Definitions {
    binding: WireBinding,
    units: Vec<UnitDefinition>,
    eras: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Missing("Unit history rules binding"));
        }
        let registry = |table, column| {
            collection(rules, table, column)
                .map_err(|_| Error::Missing("Unit history definition collection"))
        };
        let units = registry("Units", "UnitType")?;
        let traits = registry("Traits", "TraitType")?;
        let resources = registry("Resources", "ResourceType")?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Unit history Units"))?;
        let mut definitions = Vec::new();
        for entry in units.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Unit history Unit row"))?;
            let reference = |column, registry: &LinuxDefinitionRegistry| match row
                .get(column)
                .map_err(Error::Rules)?
            {
                SqlValue::Null => Ok(None),
                SqlValue::Text(name) => Ok(registry.find_index(linux_type_hash(name))),
                _ => Err(Error::Missing("Unit history definition reference")),
            };
            definitions.push(UnitDefinition {
                trait_hash: reference("TraitType", &traits)?
                    .and_then(|index| traits.type_hash(index)),
                resource_index: reference("StrategicResource", &resources)?
                    .map(|index| index as i32),
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            units: definitions,
            eras: registry("Eras", "EraType")?,
        })
    }
}

pub trait Effects {
    /// Must call the canonical general HandleMoment and finish its synchronous
    /// MomentRecorded, report, era-score and notice continuations before return.
    fn handle_moment(&mut self, state: &mut LoadedState, moment: Moment) -> Result<(), Error>;
    /// Source requires both configured Civilization and Leader definitions, then
    /// checks the Unit Trait in Civilization traits before Leader traits.
    fn unique_trait_matches(&mut self, _: &LoadedState, _: i32, _: u32) -> Result<bool, Error> {
        Err(Error::Missing(
            "Unit history current Civilization/Leader traits",
        ))
    }
}

fn has(
    state: &LoadedState,
    owner: i32,
    kind: u32,
    data: Option<(u32, i32)>,
) -> Result<bool, Error> {
    let history = state
        .game_history()
        .map_err(|_| Error::Missing("Unit history canonical History"))?;
    Ok(history.moments.iter().any(|moment| {
        moment.type_hash == kind
            && (owner == -1 || moment.acting_player_ordinal == owner)
            && data.is_none_or(|(key, value)| {
                moment
                    .extra_data
                    .iter()
                    .find(|d| d.kind_raw == key)
                    .map_or(-1, |d| d.value)
                    == value
            })
    }))
}

fn request(kind: u32, owner: i32, plot: PlotCoord, data: &[(u32, i32)]) -> Moment {
    Moment {
        wire_version: MOMENT_VERSION,
        type_hash: kind,
        turn: -1,
        action_plot: plot,
        acting_player_ordinal: owner,
        id: -1,
        has_ever_been_commemorated_raw: 0,
        game_era_hash: u32::MAX,
        extra_data: data
            .iter()
            .map(|&(kind_raw, value)| MomentData { kind_raw, value })
            .collect(),
        city_name_bytes: Vec::new(),
        unit_citizen_name_bytes: Vec::new(),
        other_city_name_bytes: Vec::new(),
        era_score: 0,
    }
}

/// Dispatch unique, strategic-resource, then naval/air moments in source order.
/// Reads History anew after each synchronous continuation, including any nested
/// effects. There is no dispatcher major-player guard; HandleMoment owns it.
pub fn on_created(
    state: &mut LoadedState,
    defs: &Definitions,
    maps: &BuildMaps,
    scratch: &[(i32, bool)],
    unit: UnitReference,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    if state.binding() != &defs.binding {
        return Err(Error::Missing("Unit history binding"));
    }
    let actor = state
        .world()
        .unit(unit)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Unit history retained Unit"))?
        .record;
    let owner = actor.e_owner;
    if scratch
        .iter()
        .find(|r| r.0 == owner)
        .ok_or(Error::Missing("Unit history scratch flag"))?
        .1
        || actor.b_is_levied
        || actor.k_hero.e_class != u32::MAX
    {
        return Ok(());
    }
    let index = state
        .unit_runtime(unit.player(), unit.id)
        .map_err(Error::State)?
        .definition_index;
    let Some(definition) = usize::try_from(index)
        .ok()
        .and_then(|index| defs.units.get(index))
    else {
        return Ok(()); // Native missing Unit definition returns.
    };
    if let Some(trait_hash) = definition.trait_hash {
        if effects.unique_trait_matches(state, owner, trait_hash)?
            && !has(state, owner, UNIQUE, Some((UNIT, index)))?
        {
            let plot = location(state, unit)?;
            effects.handle_moment(state, request(UNIQUE, owner, plot, &[(UNIT, index)]))?;
        }
    }
    if let Some(resource) = definition.resource_index {
        if maps.binding != *state.binding() {
            return Err(Error::Missing("Unit history build map binding"));
        }
        let mapped = maps
            .players
            .iter()
            .find(|r| r.0 == owner)
            .and_then(|r| r.1.get(index as usize))
            .ok_or(Error::Missing("Unit history current replacement cell"))?;
        // CanEverTrain21c10a0 compares the current replacement cell to type.
        if *mapped == index {
            if let Some(kind) = first_kind(
                state,
                owner,
                RESOURCE_WORLD,
                RESOURCE_PLAYER,
                Some((RESOURCE, resource)),
            )? {
                let plot = location(state, unit)?;
                effects.handle_moment(
                    state,
                    request(kind, owner, plot, &[(UNIT, index), (RESOURCE, resource)]),
                )?;
            }
        }
    }
    let domain = state
        .unit_runtime(unit.player(), unit.id)
        .map_err(Error::State)?
        .domain;
    let (world, player) = match domain {
        0 => (0x1269d6e0, 0x08ac7f62),
        1 => (0x34c90daf, 0x3f0506ae),
        _ => return Ok(()),
    };
    if let Some(kind) = first_kind(state, owner, world, player, None)? {
        let era_hash = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("Unit history current Player era"))?
            .eras
            .era_hash;
        let era = if era_hash == u32::MAX {
            -1
        } else {
            defs.eras
                .find_index(era_hash)
                .ok_or(Error::Missing("Unit history current era definition"))? as i32
        };
        let plot = location(state, unit)?;
        effects.handle_moment(
            state,
            request(kind, owner, plot, &[(UNIT, index), (PLAYER_ERA, era)]),
        )?;
    }
    Ok(())
}

fn first_kind(
    state: &LoadedState,
    owner: i32,
    world: u32,
    player: u32,
    data: Option<(u32, i32)>,
) -> Result<Option<u32>, Error> {
    if !has(state, -1, world, data)? {
        return Ok(Some(world));
    }
    if has(state, owner, world, data)? || has(state, owner, player, data)? {
        return Ok(None);
    }
    Ok(Some(player))
}

fn location(state: &LoadedState, unit: UnitReference) -> Result<PlotCoord, Error> {
    Ok(state
        .world()
        .unit(unit)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Unit history current location"))?
        .record
        .k_location)
}
