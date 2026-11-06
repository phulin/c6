//! Explicit Raze for an ordinary captured noncapital City. This is immediate
//! destruction on the Linux target. Every synchronous subscriber and the
//! non-transfer destruction path must finish before the choice is consumed.
use super::Definitions;
use crate::city_capture::{self, Disposition, PendingDisposition};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

pub mod requirements;

pub const DIRECTIVE: u32 = 0x8805_00fc;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Admission,
    Markers,
    CommandStarted,
    Counter,
    CityRazed,
    Destruction,
    Blocking,
    MessagesProcessed,
    Queue,
}
#[derive(Debug)]
pub struct Error {
    pub stage: Stage,
    pub detail: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Raze City {:?}: {}", self.stage, self.detail)
    }
}
impl std::error::Error for Error {}
pub(crate) fn error(stage: Stage, detail: impl std::fmt::Debug) -> Error {
    Error {
        stage,
        detail: format!("{detail:?}"),
    }
}
/// All consumers borrow the ongoing owners, including modifier/requirements,
/// diplomacy, history, placement and RNG. No callback has a successful default.
pub trait Effects {
    fn command_started(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        district: ComponentId,
    ) -> Result<(), String>;
    fn city_razed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        previous_owner: i32,
    ) -> Result<(), String>;
    /// City.Manager.Destroy(city,false), using city_destroy::transition and the
    /// same retained destruction/territory/services used by capture.
    fn destroy(&mut self, state: &mut LoadedState, city: ComponentId) -> Result<(), String>;
}
/// The exposed profile requires a completed ordinary capture of a noncapital
/// from a surviving major. Native Destroy.CanStart alone does not authenticate
/// a captured-city decision; current Rules.Players.CanRaze supplies legality.
pub fn admit(
    state: &LoadedState,
    choice: &PendingDisposition,
    definitions: &Definitions,
    levels: &[(i32, u32)],
) -> Result<(), Error> {
    let fail = |detail| error(Stage::Admission, detail);
    if state.binding() != &definitions.binding
        || choice.city.component_type != 2
        || choice.district.component_type != 3
        || choice.city.owner != choice.district.owner
        || !choice.legal.contains(&Disposition::Raze)
    {
        return Err(fail("Raze decision identity/definition binding"));
    }
    let owner = i32::from(choice.city.owner);
    let city = state
        .city(owner, choice.city.id)
        .map_err(|e| error(Stage::Admission, e))?;
    let district = state
        .district(owner, choice.district.id)
        .map_err(|e| error(Stage::Admission, e))?;
    if city.header.owner != owner
        || city.header.just_conquered_from != choice.previous_owner
        || city.header.owner_before_occupation != choice.previous_owner
        || choice.previous_owner == owner
        || !(0..64).contains(&choice.previous_owner)
        || city.header.last_transfer_type as u32 != c6_core::city_capture::TRANSFER_COMBAT
        || city.header.original_capital
        || district.city_id != choice.city.id
        || district.district_hash != c6_core::city_district::CITY_CENTER_HASH
        || !city.districts.district_ids.contains(&choice.district.id)
    {
        return Err(fail(
            "Raze supports current ordinary noncapital combat capture",
        ));
    }
    let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
    for player in [owner, choice.previous_owner] {
        if levels.iter().find(|row| row.0 == player).map(|row| row.1) != Some(major)
            || !state
                .player_alive(player)
                .map_err(|e| error(Stage::Admission, e))?
        {
            return Err(fail("Raze requires surviving major owners"));
        }
    }
    if !city_capture::can_raze(state, choice.city, &definitions.capture, levels)
        .map_err(|e| error(Stage::Admission, e))?
    {
        return Err(fail("Raze no longer legal"));
    }
    Ok(())
}
/// XP1.Destroy.Start2820030, Raze28202a6. Errors retain the applied prefix;
/// Session must freeze after a mutating failure and retain its pending choice.
pub fn execute(
    state: &mut LoadedState,
    choice: &PendingDisposition,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    effects: &mut impl Effects,
) -> Result<(), Error> {
    admit(state, choice, definitions, levels)?;
    let owner = i32::from(choice.city.owner);
    let city = state
        .city_mut(owner, choice.city.id)
        .map_err(|e| error(Stage::Markers, e))?;
    city.header.just_conquered_from = -1;
    city.header.just_rebelled_from = -1;
    effects
        .command_started(state, choice.city, choice.district)
        .map_err(|e| error(Stage::CommandStarted, e))?;
    // The previous owner is reread after BaseHandler.Start and can be changed by
    // a synchronous consumer; the queued observation is not reused as an answer.
    let previous = state
        .city(owner, choice.city.id)
        .map_err(|e| error(Stage::Counter, e))?
        .header
        .owner_before_occupation;
    let at = usize::try_from(previous)
        .ok()
        .filter(|i| *i < 64)
        .ok_or_else(|| error(Stage::Counter, "previous owner index"))?;
    let diplomacy = state
        .war_diplomacy_mut(owner)
        .map_err(|e| error(Stage::Counter, e))?;
    diplomacy.cities_razed[at] = diplomacy.cities_razed[at].wrapping_add(1);
    let previous = state
        .city(owner, choice.city.id)
        .map_err(|e| error(Stage::CityRazed, e))?
        .header
        .owner_before_occupation;
    effects
        .city_razed(state, choice.city, previous)
        .map_err(|e| error(Stage::CityRazed, e))?;
    effects
        .destroy(state, choice.city)
        .map_err(|e| error(Stage::Destruction, e))
}

/// DisplaceSpiesOnCityDestroy2be9fe0 tailcalls the same canonical capture
/// displacement helper. Matching spies stay explicitly unsupported.
pub fn no_spies_to_displace(
    state: &LoadedState,
    city: ComponentId,
    definitions: &Definitions,
    alive_majors: &[i32],
) -> Result<(), String> {
    city_capture::no_spies_to_displace(state, city, &definitions.capture, alive_majors)
}

/// Post-destruction Player.Cities.Destroy2a59590 traverses a copy of the current
/// source plot's Unit IDs, skipping missing, delayed-death and non-SEA Units.
/// This bounded branch validates current native domain state. A live SEA Unit
/// requires IterateRing/placement-or-destruction and cannot silently stay put.
pub fn no_naval_units_to_relocate(
    state: &LoadedState,
    location: c6_core::unit::PlotCoord,
) -> Result<(), String> {
    let map = state.world().map();
    if map.plot_at(location.x, location.y).is_none() {
        return Err("destroyed City source plot".into());
    }
    let index = (location.y as u32)
        .wrapping_mul(map.units.width)
        .wrapping_add(location.x as u32);
    let Some(layer) = map.units.layers.iter().find(|layer| layer.layer_id == 0) else {
        return Ok(());
    };
    if index >= layer.maximum {
        return Ok(());
    }
    let references = state
        .world()
        .plot_units(0, index as usize)
        .map_err(|e| format!("{e:?}"))?;
    for reference in references {
        let Some(unit) = state
            .world()
            .player(reference.player())
            .map_err(|e| format!("{e:?}"))?
            .unit_records()
            .map_err(|e| format!("{e:?}"))?
            .iter()
            .find(|row| row.value.id == reference.id)
            .map(|row| &row.value)
        else {
            continue;
        };
        if !unit.b_delayed_death
            && state
                .unit_runtime(reference.player(), reference.id)
                .map_err(|e| format!("{e:?}"))?
                .domain
                == 0
        {
            return Err("destroyed City SEA Unit ring relocation/destruction".into());
        }
    }
    Ok(())
}

/// Actual Diplomacy.OnCityRazed2aa27f0 for the surviving-major surprise-war
/// profile. The live CityRazed ring supplies receivers in registration order.
#[allow(clippy::too_many_arguments)]
pub fn on_city_razed<P, R: crate::loaded_actions::declare_war::diplomacy::Reports<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    definitions: &super::keep::Definitions,
    alive_majors: &[i32],
    levels: &[(i32, u32)],
    receiver: i32,
    previous_owner: i32,
    city: ComponentId,
    reports: &mut R,
) -> Result<(), Error>
where
    R::Error: std::fmt::Debug,
{
    use super::keep::{native_i32, percent, player};
    let fail = |e| error(Stage::CityRazed, e);
    if state.binding() != &definitions.binding {
        return Err(fail("Raze diplomacy binding"));
    }
    let owner = i32::from(city.owner);
    let turn = state.retained().core.current_turn;
    // Raze writes the shared last-city-capture turn only on the razer's own
    // receiver, unlike Keep's unconditional write on every Diplomacy receiver.
    if receiver == owner {
        state
            .war_diplomacy_mut(receiver)
            .map_err(|e| error(Stage::CityRazed, e))?
            .last_war_and_city_turns[7] = turn;
        return Ok(()); // Player.Diplomacy.HasMet(self) is false.
    }
    if !state
        .game_relations()
        .map_err(|e| error(Stage::CityRazed, e))?
        .has_met(receiver, owner)
        .map_err(|e| error(Stage::CityRazed, e))?
    {
        return Ok(());
    }
    let major = crate::registry::linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
    if levels.iter().find(|r| r.0 == owner).map(|r| r.1) != Some(major) {
        return Ok(());
    }
    if levels.iter().find(|r| r.0 == previous_owner).map(|r| r.1) != Some(major)
        || !state
            .player_alive(previous_owner)
            .map_err(|e| error(Stage::CityRazed, e))?
    {
        return Err(fail("Raze diplomacy requires surviving previous major"));
    }
    let score = state
        .game_relations()
        .map_err(|e| error(Stage::CityRazed, e))?
        .grievance_score(owner, previous_owner)
        .map_err(|e| error(Stage::CityRazed, e))?;
    let mut amount =
        super::keep::surprise_raze_points(state, definitions, alive_majors, city, previous_owner)
            .map_err(|e| error(Stage::CityRazed, e))?;
    if receiver != previous_owner {
        if score > 0 {
            amount = percent(amount, definitions.defensive_percent);
        }
        let relations = state
            .game_relations()
            .map_err(|e| error(Stage::CityRazed, e))?;
        amount = if relations
            .is_allied(receiver, previous_owner)
            .map_err(|e| error(Stage::CityRazed, e))?
        {
            percent(amount, definitions.ally_percent)
        } else if relations
            .has_declared_friendship(receiver, previous_owner)
            .map_err(|e| error(Stage::CityRazed, e))?
        {
            percent(amount, definitions.friendship_percent)
        } else {
            0
        };
    }
    if amount == 0 || levels.iter().find(|r| r.0 == receiver).map(|r| r.1) != Some(major) {
        return Ok(());
    }
    let name = state
        .city(owner, city.id)
        .map_err(|e| error(Stage::CityRazed, e))?
        .header
        .name
        .clone();
    let modifiers = player(state, receiver)
        .map_err(|e| error(Stage::CityRazed, e))?
        .diplomacy
        .grievance_generation_modifier
        .wrapping_add(
            player(state, owner)
                .map_err(|e| error(Stage::CityRazed, e))?
                .diplomacy
                .grievance_generation_modifier,
        )
        .wrapping_add(100);
    let adjusted = native_i32(f64::from(modifiers.wrapping_mul(amount)) * 0.01);
    state
        .war_relations_mut()
        .map_err(|e| error(Stage::CityRazed, e))?
        .change_grievance_and_tension(receiver, owner, adjusted, turn)
        .map_err(|e| error(Stage::CityRazed, e))?;
    reports
        .grievances(state, backend, receiver, owner)
        .map_err(|e| error(Stage::CityRazed, e))?;
    state
        .war_relations_mut()
        .map_err(|e| error(Stage::CityRazed, e))?
        .append_grievance_log(c6_core::game_diplomatic_relations::war::GrievanceLogItem {
            holder: receiver,
            source: owner,
            turn,
            amount: adjusted,
            reason: b"LOC_DIPLO_GRIEVANCE_LOG_CITY_RAZED".to_vec(),
            other_reason: name,
        })
        .map_err(|e| error(Stage::CityRazed, e))
}
