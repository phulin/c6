//! Current CityCreated gameplay listeners, in the caller's live subscription order.
//! Native Summary repository output is excluded; policies remain external.
pub mod history;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{loaded_components::RetainedPlayer, loaded_state::LoadedState, unit::ComponentId};

#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Rules(crate::Error),
    Eras(c6_core::loaded_state::game_eras::AccessError),
    Definitions(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
    Effect(String),
}
impl From<c6_core::loaded_state::Error> for Error {
    fn from(e: c6_core::loaded_state::Error) -> Self {
        Self::State(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CityCreated: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug)]
pub enum Requirement {
    CityOccupied {
        label: u64,
        context: Option<i32>,
        subject: Option<i32>,
        only_context: bool,
    },
    PopulationLead {
        label: u64,
        context: i32,
        subject: i32,
        minimum: i32,
        percent_bits: u32,
    },
    TeamDomination {
        label: u64,
        team: i32,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum Listener {
    Summary,
    GeneralPlayerSummary,
    Lua,
    Gossip,
    History,
    CoastalBoost { definition: crate::RulesRowId },
    Diplomacy { owner: i32 },
    PlayerEvent { owner: i32 },
    PlayerCities { receiver: u64, owner: i32 },
    AllCities { receiver: u64 },
    Requirement(Requirement),
    Achievements,
}
pub struct Current<'a> {
    pub alive: &'a [i32],
    pub ever_alive_majors: &'a [i32],
    pub levels: &'a [(i32, u32)],
}
pub struct Definitions {
    terrains: LinuxDefinitionRegistry,
    water: Vec<bool>,
    ocean_min_size: i32,
    governors: LinuxDefinitionRegistry,
    maximum_appointments: i32,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let terrains = collection(rules, "Terrains", "TerrainType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Terrains")
            .ok_or(Error::Missing("Terrains"))?;
        let water = terrains
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Missing("Terrain row"))?;
                crate::game_parameters::sql_integer(row.get("Water").map_err(Error::Rules)?)
                    .map(|v| v != 0)
                    .ok_or(Error::Missing("Terrain.Water"))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let parameter = |name| {
            rules
                .gameplay()
                .table("GlobalParameters")
                .ok_or(Error::Missing("GlobalParameters"))?
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or(Error::Missing(name))
        };
        Ok(Self {
            terrains,
            water,
            ocean_min_size: parameter("OCEAN_MIN_WATER_SIZE")?,
            governors: collection(rules, "Governors", "GovernorType")
                .map_err(Error::Definitions)?,
            maximum_appointments: parameter("MAX_GOVERNOR_APPOINTMENTS")?,
        })
    }
    fn water(&self, hash: u32) -> Result<bool, Error> {
        let index = self
            .terrains
            .find_index(hash)
            .ok_or(Error::Missing("current Terrain"))?;
        Ok(index == 15 || index == 16 || self.water[index])
    }
    /// Plot.IsCoastalLand(-1), using current Map and the configured ocean size.
    pub fn is_coastal(&self, state: &LoadedState, city: ComponentId) -> Result<bool, Error> {
        let city = state.city(i32::from(city.owner), city.id)?;
        let map = state.world().map();
        let coord = c6_core::unit::PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        };
        let (_, plot) = c6_core::movement_world::plot_at(map, coord)
            .map_err(|_| Error::Missing("current City plot"))?;
        if self.water(plot.terrain_hash)? {
            return Ok(false);
        }
        for direction in 0..6 {
            let next = c6_core::found_city::placement::plot_from_delta(
                map,
                coord,
                [0, 1, 1, 0, -1, -1][direction],
                [1, 0, -1, -1, 0, 1][direction],
                1,
            )
            .map_err(|_| Error::Missing("current coastal adjacency"))?;
            let Some(next) = next else { continue };
            let plot = &map.plots[next];
            if !self.water(plot.terrain_hash)? || plot.flags.get(c6_core::map::PlotFlag::Impassable)
            {
                continue;
            }
            if self.ocean_min_size <= 0 {
                return Ok(true);
            }
            if map
                .landmasses
                .entries
                .iter()
                .find(|r| r.index == plot.landmass_id)
                .is_some_and(|r| r.value.plot_count >= self.ocean_min_size)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current listener Player"))
}
fn population(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    Ok(player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .fold(0i32, |n, c| n.wrapping_add(c.value.header.population)))
}
/// Native CityOccupied::Test reads the live diplomatic occupation array.
pub fn occupied_test(
    state: &LoadedState,
    current: &Current<'_>,
    context: Option<i32>,
    subject: Option<i32>,
    only_context: bool,
) -> Result<i32, Error> {
    let (Some(context), Some(subject)) = (context, subject) else {
        return Ok(1);
    };
    player(state, context)?;
    let diplomacy = &player(state, subject)?.diplomacy;
    for &owner in current.alive {
        if owner == subject || (only_context && owner != context) {
            continue;
        }
        let index =
            usize::try_from(owner).map_err(|_| Error::Missing("occupation Player index"))?;
        if *diplomacy
            .occupies_original_city
            .get(index)
            .ok_or(Error::Missing("occupation Player extent"))?
        {
            return Ok(0);
        }
    }
    Ok(1)
}
fn team(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .team
        .ok_or(Error::Missing("current Player team"))
}
fn major(current: &Current<'_>, owner: i32) -> Result<bool, Error> {
    Ok(current
        .levels
        .iter()
        .find(|r| r.0 == owner)
        .ok_or(Error::Missing("current civilization level"))?
        .1
        == 0x253718b0)
}
/// Exact four-state TeamDomination Test, including the never-founded major gate.
pub fn domination_test(
    state: &LoadedState,
    current: &Current<'_>,
    required_team: i32,
) -> Result<i32, Error> {
    if required_team == -1 || current.alive.is_empty() {
        return Ok(3);
    }
    let mut all_same_team = true;
    for &owner in current.alive {
        if state.player_alive(owner).map_err(Error::State)?
            && team(state, owner)? != required_team
        {
            all_same_team = false;
            break;
        }
    }
    if all_same_team {
        return Ok(2);
    }
    for &owner in current.alive {
        let p = player(state, owner)?;
        if major(current, owner)? && p.cities.peak_cities == 0 {
            return Ok(1);
        }
        for c in &p.cities.records.entries {
            let c = &c.value.header;
            if c.delayed_death && !c.transferring {
                continue;
            }
            if c.original_capital
                && current.ever_alive_majors.contains(&c.original_owner)
                && team(state, owner)? != required_team
            {
                return Ok(1);
            }
        }
    }
    Ok(2)
}

/// Concrete shared services. Requirement invalidation must use the SAME live
/// SimpleRequirement/Set/modifier runtime, preserving changed delivery.
pub trait Effects {
    fn lua_city_built(&mut self, state: &mut LoadedState, city: ComponentId) -> Result<(), Error>;
    fn gossip_visibility_count(&mut self, state: &LoadedState) -> Result<usize, Error>;
    fn add_city_founded_gossip(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), Error>;
    fn history_city_built(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), Error>;
    fn trigger_boost(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        definition: crate::RulesRowId,
    ) -> Result<(), Error>;
    fn diplomacy_forward_settle(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: ComponentId,
    ) -> Result<(), Error>;
    fn governor_title_message(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        can_appoint: bool,
        can_promote: bool,
    ) -> Result<(), Error>;
    fn player_cities(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        city: ComponentId,
    ) -> Result<(), Error>;
    fn all_cities(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        city: ComponentId,
    ) -> Result<(), Error>;
    fn requirement_active(&mut self, label: u64) -> Result<bool, Error>;
    fn invalidate_requirement(
        &mut self,
        state: &mut LoadedState,
        label: u64,
        tested: i32,
    ) -> Result<(), Error>;
    fn era_score_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
    ) -> Result<(), Error>;
    fn achievements(&mut self, state: &mut LoadedState, city: ComponentId) -> Result<(), Error>;
}
/// Execute one actual listener. A caller may use a mutation-safe live ring so
/// newly appended collection listeners participate in later CityCreated events.
pub fn dispatch_one(
    state: &mut LoadedState,
    city: ComponentId,
    initializing: bool,
    listener: Listener,
    defs: &Definitions,
    current: &Current<'_>,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let owner = i32::from(city.owner);
    state.city(owner, city.id)?;
    match listener {
        // Both callbacks only update GameSummary repository datasets; no signal,
        // gameplay query mutation or RNG occurs in their reached source paths.
        Listener::Summary | Listener::GeneralPlayerSummary => Ok(()),
        Listener::Lua => effects.lua_city_built(state, city),
        Listener::Gossip => {
            if initializing || effects.gossip_visibility_count(state)? == 0 {
                Ok(())
            } else {
                effects.add_city_founded_gossip(state, city)
            }
        }
        Listener::History => {
            if initializing {
                Ok(())
            } else {
                effects.history_city_built(state, city)
            }
        }
        Listener::CoastalBoost { definition } => {
            if !initializing && defs.is_coastal(state, city)? {
                effects.trigger_boost(state, owner, definition)
            } else {
                Ok(())
            }
        }
        Listener::Diplomacy { owner: receiver } => {
            if receiver == owner || state.city(owner, city.id)?.header.original_capital {
                Ok(())
            } else {
                effects.diplomacy_forward_settle(state, receiver, city)
            }
        }
        Listener::PlayerEvent { owner: receiver } => {
            if receiver != owner {
                return Ok(());
            }
            state.set_governor_title_considered(owner, false)?;
            let governors = &player(state, owner)?.governors;
            let spent = governors
                .entries
                .iter()
                .flat_map(|e| &e.governor.promotions)
                .fold(0i32, |sum, p| sum.wrapping_add(i32::from(p.earned)));
            if spent >= governors.governor_points {
                return Ok(());
            }
            let owned = defs
                .governors
                .entries()
                .iter()
                .filter(|d| {
                    governors
                        .entries
                        .iter()
                        .any(|g| g.governor.definition_hash == linux_type_hash(&d.type_name))
                })
                .count();
            let can_appoint =
                owned < defs.governors.entries().len() && owned as i32 <= defs.maximum_appointments;
            if !governors.entries.is_empty() {
                return Err(Error::Missing("positive Governor promotion availability"));
            }
            effects.governor_title_message(state, owner, can_appoint, false)
        }
        Listener::PlayerCities {
            receiver,
            owner: receiver_owner,
        } => {
            let current = state.city(owner, city.id)?;
            if current.header.owner != receiver_owner || current.header.delayed_death {
                Ok(())
            } else {
                effects.player_cities(state, receiver, city)
            }
        }
        Listener::AllCities { receiver } => effects.all_cities(state, receiver, city),
        Listener::Requirement(requirement) => {
            if matches!(requirement, Requirement::CityOccupied { context, .. } if context != Some(owner))
            {
                return Ok(());
            }
            let label = match requirement {
                Requirement::CityOccupied { label, .. }
                | Requirement::PopulationLead { label, .. }
                | Requirement::TeamDomination { label, .. } => label,
            };
            if !effects.requirement_active(label)? {
                return Ok(());
            }
            let (label, tested) = match requirement {
                Requirement::CityOccupied {
                    label,
                    context,
                    subject,
                    only_context,
                } => {
                    if context != Some(owner) {
                        return Ok(());
                    }
                    (
                        label,
                        occupied_test(state, current, context, subject, only_context)?,
                    )
                }
                Requirement::PopulationLead {
                    label,
                    context,
                    subject,
                    minimum,
                    percent_bits,
                } => (
                    label,
                    crate::city_turn::religion_callbacks::population_lead_test(
                        population(state, context)?,
                        population(state, subject)?,
                        minimum,
                        f32::from_bits(percent_bits),
                    ),
                ),
                Requirement::TeamDomination { label, team } => {
                    (label, domination_test(state, current, team)?)
                }
            };
            effects.invalidate_requirement(state, label, tested)
        }
        Listener::Achievements => effects.achievements(state, city),
    }
}

/// HandleMoment's positive score continuation. Native reason zero does not
/// enter the quest-recorded notification switch, even for a human Player.
/// Source logging and human queries have no retained gameplay/RNG effect here.
pub fn credit_historic_era_score(
    state: &mut LoadedState,
    owner: i32,
    amount: i32,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    if state
        .change_game_era_score(owner, amount, 0)
        .map_err(Error::Eras)?
    {
        effects.era_score_report(state, owner, amount)?;
    }
    Ok(())
}
