//! Real canonical District creation and completion for Linux city placement.
//! Signals are mandatory synchronous effects. Nonempty unit awards currently
//! stop at their precise unsupported leaf after retaining the native prefix.
use crate::loaded_actions::city_range_attack;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions, Rules,
};
use c6_core::{
    city_initialization,
    core_player::initialization::WireBinding,
    district_initialization as kernel,
    loaded_state::{self, LoadedState},
    unit::PlotCoord,
};
use std::convert::Infallible;

#[derive(Debug)]
pub enum DefinitionError {
    Binding,
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
}
impl std::fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "District definitions: {self:?}")
    }
}
impl std::error::Error for DefinitionError {}
#[derive(Debug)]
pub enum Error<E = Infallible> {
    State(loaded_state::Error),
    District(loaded_state::district_initialization::DistrictError),
    Allocation(city_initialization::Error),
    Construction(kernel::Error),
    Combat(crate::loaded_actions::Error),
    Missing(&'static str),
    Effects(E),
    FoundingUnitAward {
        owner: i32,
        district_hash: u32,
        unit_hash: u32,
    },
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "District initialization: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

pub struct Definitions {
    binding: WireBinding,
    districts: LinuxDefinitionRegistry,
    yields: Vec<u32>,
    great_people: Vec<u32>,
    replacements: Vec<(u32, u32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        if state.binding() != bound.destination_binding() {
            return Err(DefinitionError::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(DefinitionError::Definitions)?;
        let collection = |table, column| {
            unit_initialization_definitions::collection(rules, table, column)
                .map_err(DefinitionError::Definitions)
        };
        let hashes = |r: LinuxDefinitionRegistry| {
            r.entries()
                .iter()
                .map(|d| linux_type_hash(&d.type_name))
                .collect()
        };
        let mut replacements = Vec::new();
        let table =
            rules
                .gameplay()
                .table("DistrictReplaces")
                .ok_or(DefinitionError::Definitions(
                    unit_initialization_definitions::Error::MissingTable("DistrictReplaces"),
                ))?;
        for row in table.rows() {
            replacements.push((
                linux_type_hash(
                    row.text("CivUniqueDistrictType")
                        .map_err(DefinitionError::Rules)?
                        .as_bytes(),
                ),
                linux_type_hash(
                    row.text("ReplacesDistrictType")
                        .map_err(DefinitionError::Rules)?
                        .as_bytes(),
                ),
            ));
        }
        Ok(Self {
            binding: state.binding().clone(),
            districts: collection("Districts", "DistrictType")?,
            yields: hashes(collection("Yields", "YieldType")?),
            great_people: hashes(collection("GreatPersonClasses", "GreatPersonClassType")?),
            replacements,
        })
    }
    fn validate<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if &self.binding != state.binding() {
            Err(Error::Missing("District definitions binding"))
        } else {
            Ok(())
        }
    }
    pub fn type_index(&self, hash: u32) -> Option<i32> {
        self.districts
            .find_index(hash)
            .and_then(|i| i32::try_from(i).ok())
    }
    fn type_hash<E>(&self, index: i32) -> Result<u32, Error<E>> {
        self.districts
            .at(index as u32)
            .map(|d| linux_type_hash(&d.type_name))
            .ok_or(Error::Missing("current District definition"))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictReference {
    pub owner: i32,
    pub id: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Creation {
    pub owner: i32,
    pub city_id: u32,
    pub location: PlotCoord,
    pub district_type: i32,
    pub complete: bool,
    pub bought: bool,
}
pub trait Effects {
    type Error;
    fn player_district_completed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        district_type: i32,
        city_id: u32,
    ) -> Result<(), Self::Error>;
    fn game_district_completed(
        &mut self,
        state: &mut LoadedState,
        district: DistrictReference,
        bought: bool,
    ) -> Result<(), Self::Error>;
}
fn plot_index<E>(state: &LoadedState, location: (i32, i32)) -> Result<usize, Error<E>> {
    let map = state.world().map();
    let w = map.header.width;
    let h = map.header.height;
    if w <= 0 || h <= 0 || location.0 == -9999 || location.1 == -9999 {
        return Err(Error::Missing("District.GetPlot map/location"));
    }
    let x = c6_core::movement_world::coord_range(location.0, w, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(location.1, h, map.header.wrap_y);
    if x < 0 || x >= w || y < 0 || y >= h {
        return Err(Error::Missing("District.GetPlot null"));
    }
    let index = (y as usize)
        .checked_mul(w as usize)
        .and_then(|i| i.checked_add(x as usize))
        .ok_or(Error::Missing("District plot index"))?;
    if index >= map.plots.len() {
        return Err(Error::Missing("District plot storage"));
    }
    Ok(index)
}
fn plot_owner<E>(state: &LoadedState, district: DistrictReference) -> Result<i32, Error<E>> {
    let record = state
        .district(district.owner, district.id)
        .map_err(Error::District)?;
    let plot = &state.world().map().plots[plot_index(state, record.location)?];
    if plot.flags.get(c6_core::map::PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Missing("District plot attributes pointer/flag"));
    }
    Ok(plot
        .city_attributes
        .as_ref()
        .map_or(-1, |a| i32::from(a.player)))
}

/// Player.Districts.Create then City.Districts.Create, including source combat
/// Restore BEFORE city membership/plot assignment and initial completion.
pub fn create_in_city<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions,
    combat: &city_range_attack::Queries<'_>,
    effects: &mut E,
    creation: Creation,
) -> Result<Option<DistrictReference>, Error<E::Error>> {
    definitions.validate(state)?;
    state
        .city(creation.owner, creation.city_id)
        .map_err(Error::State)?;
    let hash = definitions.type_hash(creation.district_type)?;
    let allocation = {
        let records = &mut state
            .player_districts_mut(creation.owner)
            .map_err(Error::State)?
            .records;
        let Some(allocation) = city_initialization::allocate(records).map_err(Error::Allocation)?
        else {
            return Ok(None);
        };
        let value = kernel::construct(kernel::Definitions {
            yields: &definitions.yields,
            great_person_classes: &definitions.great_people,
        })
        .map_err(Error::Construction)?;
        allocation
            .insert(records, value)
            .map_err(Error::Allocation)?;
        allocation
    };
    let reference = DistrictReference {
        owner: creation.owner,
        id: allocation.id,
    };
    {
        let value = state
            .district_mut(reference.owner, reference.id)
            .map_err(Error::District)?;
        value.id = allocation.id;
        value.location = (creation.location.x, creation.location.y);
        value.city_id = creation.city_id;
        value.district_hash = hash;
    }
    let strength = {
        let city = state
            .city(creation.owner, creation.city_id)
            .map_err(Error::State)?;
        let district = state
            .district(reference.owner, reference.id)
            .map_err(Error::District)?;
        city_range_attack::attack_strength_q8(state, city, district, combat)
            .map_err(Error::Combat)?
    };
    if strength > 0 {
        // Restore re-reads GetCity after CanAttack. Its null branch retains the
        // constructor's count; this exact new-city identity still must resolve.
        let owner = plot_owner(state, reference)?;
        match state.city(owner, creation.city_id) {
            Ok(city) => {
                let attacks = city.header.attacks_per_turn;
                state
                    .district_mut(reference.owner, reference.id)
                    .map_err(Error::District)?
                    .remaining_attacks = attacks;
            }
            Err(loaded_state::Error::Missing(loaded_state::MissingInput::City { .. })) => {}
            Err(error) => return Err(Error::State(error)),
        }
    }
    let city = state
        .city_mut(creation.owner, creation.city_id)
        .map_err(Error::State)?;
    city.districts
        .district_ids
        .try_reserve(1)
        .map_err(|_| Error::Missing("City district membership allocation"))?;
    city.districts.district_ids.push(reference.id);
    let index = plot_index(state, (creation.location.x, creation.location.y))?;
    let plot = state
        .world_mut()
        .map_err(Error::State)?
        .plot_mut(index)
        .ok_or(Error::Missing("District plot"))?;
    let attributes = plot
        .city_attributes
        .as_mut()
        .ok_or(Error::Missing("City.Districts.Create plot attributes"))?;
    attributes.district = reference.id;
    set_complete(
        state,
        definitions,
        effects,
        reference,
        creation.complete,
        creation.bought,
    )?;
    Ok(Some(reference))
}

pub fn set_complete<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions,
    effects: &mut E,
    district: DistrictReference,
    complete: bool,
    bought: bool,
) -> Result<(), Error<E::Error>> {
    definitions.validate(state)?;
    kernel::set_complete(
        &mut Completion {
            state,
            definitions,
            effects,
            district,
        },
        complete,
        bought,
    )
}
struct Completion<'a, E> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    effects: &'a mut E,
    district: DistrictReference,
}
impl<E: Effects> kernel::CompleteContext for Completion<'_, E> {
    type Error = Error<E::Error>;
    type City = (i32, u32);
    fn assign_complete(&mut self, v: bool) -> Result<(), Self::Error> {
        self.state
            .district_mut(self.district.owner, self.district.id)
            .map_err(Error::District)?
            .is_complete = v;
        Ok(())
    }
    fn game_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn city(&mut self) -> Result<Option<Self::City>, Self::Error> {
        let owner = plot_owner(self.state, self.district)?;
        let id = self
            .state
            .district(self.district.owner, self.district.id)
            .map_err(Error::District)?
            .city_id;
        match self.state.city(owner, id) {
            Ok(_) => Ok(Some((owner, id))),
            Err(loaded_state::Error::Missing(loaded_state::MissingInput::City { .. })) => Ok(None),
            Err(error) => Err(Error::State(error)),
        }
    }
    fn plot_owner(&mut self) -> Result<i32, Self::Error> {
        plot_owner(self.state, self.district)
    }
    fn edit_city(&mut self) -> Result<Self::City, Self::Error> {
        self.city()?
            .ok_or(Error::Missing("District.EditCity null after valid GetCity"))
    }
    fn current_type_index(&mut self) -> Result<i32, Self::Error> {
        let hash = self.current_type_hash()?;
        self.definitions
            .type_index(hash)
            .ok_or(Error::Missing("District.GetType mapping"))
    }
    fn city_id(&mut self, c: Self::City) -> Result<u32, Self::Error> {
        Ok(c.1)
    }
    fn signal_player_completed(&mut self, o: i32, t: i32, id: u32) -> Result<(), Self::Error> {
        self.effects
            .player_district_completed(self.state, o, t, id)
            .map_err(Error::Effects)
    }
    fn signal_game_completed(&mut self, b: bool) -> Result<(), Self::Error> {
        self.effects
            .game_district_completed(self.state, self.district, b)
            .map_err(Error::Effects)
    }
    fn current_type_hash(&mut self) -> Result<u32, Self::Error> {
        Ok(self
            .state
            .district(self.district.owner, self.district.id)
            .map_err(Error::District)?
            .district_hash)
    }
    fn founding_rewards(
        &mut self,
        owner: i32,
        hash: u32,
        _city: Self::City,
    ) -> Result<(), Self::Error> {
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("District founding Player.Units"))?;
        let maps = &player.units.add_on_district_founding;
        let direct = maps.iter().find(|m| m.key == hash);
        let replacement = self
            .definitions
            .replacements
            .iter()
            .rev()
            .find(|r| r.0 == hash)
            .map(|r| r.1);
        let found = direct.or_else(|| replacement.and_then(|h| maps.iter().find(|m| m.key == h)));
        if let Some(map) = found {
            for pair in &map.values {
                if pair.value != 0 {
                    return Err(Error::FoundingUnitAward {
                        owner,
                        district_hash: map.key,
                        unit_hash: pair.key,
                    });
                }
            }
        }
        Ok(())
    }
}
