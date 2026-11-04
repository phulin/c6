//! Live UnitNearReligiousCity predicate and source-ordered callback adapter.
//! Religion/city payloads come from canonical state; no captured Test answer.
pub use crate::plot_adjacent_owner_requirement::{DefinitionFlags, SimpleState};
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    geometry,
    loaded_state::LoadedState,
    map::UnitReference,
    movement_world, simple_requirement,
    unit::{ComponentId, Unit},
    unit_near_religious_city_requirement as kernel,
};
use std::collections::BTreeMap;

pub struct Definitions {
    religions: LinuxDefinitionRegistry,
}
impl Definitions {
    /// Native ReligionCollectionBuilder220a240 orders all Religions by rowid;
    /// BuildCollection225afc0 assigns each current size before push_back.
    pub fn from_rules(
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, unit_initialization_definitions::Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)?;
        Ok(Self {
            religions: collection(rules, "Religions", "ReligionType")?,
        })
    }
    fn religion(&self, hash: u32) -> i32 {
        self.religions.find_index(hash).map_or(-1, |i| i as i32)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitInterface {
    Absent,
    Unit(ComponentId),
    Unavailable { object: u64 },
}
/// Plot and unit interfaces are independent facts. Unknown is never null.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Object {
    pub plot_interface: Option<bool>,
    pub unit: UnitInterface,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receiver {
    pub simple: SimpleState,
    pub context: UnitInterface,
    pub subject: Object,
    pub same_owner: Option<bool>,
}
impl Receiver {
    pub fn from_initial(
        active: bool,
        state: i32,
        definition: Option<DefinitionFlags>,
        context: UnitInterface,
        subject: Object,
        same_owner: Option<bool>,
    ) -> Self {
        let simple = match definition {
            Some(f) => SimpleState::Defined(simple_requirement::Requirement {
                active,
                state,
                inverse: f.inverse,
                persistent: f.persistent,
                triggered: f.triggered,
            }),
            None => SimpleState::DefinitionUnavailable { active, state },
        };
        Self {
            simple,
            context,
            subject,
            same_owner,
        }
    }
    pub fn active(&self) -> bool {
        match self.simple {
            SimpleState::Defined(r) => r.active,
            SimpleState::DefinitionUnavailable { active, .. } => active,
        }
    }
    pub fn state(&self) -> i32 {
        match self.simple {
            SimpleState::Defined(r) => r.state,
            SimpleState::DefinitionUnavailable { state, .. } => state,
        }
    }
}
#[derive(Debug)]
pub enum QueryError {
    Binding,
    DuplicateReceiver(u64),
    MissingReceiver(u64),
    MissingInterface(u64),
    MissingPlotInterface,
    MissingCityAttributes,
    MissingDefinition(u64),
    MissingFlag(u64),
    MissingOwner(i32),
    MissingUnit(UnitReference),
    MissingMovedUnit,
    MissingCity(i32, i32),
    World(c6_core::world::MissingInput),
    MapGeometry,
    Majority(kernel::MajorityError),
}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnitNearReligiousCity: {self:?}")
    }
}
impl std::error::Error for QueryError {}
pub struct Runtime {
    binding: WireBinding,
    receivers: BTreeMap<u64, Receiver>,
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        receivers: Vec<(u64, Receiver)>,
    ) -> Result<Self, QueryError> {
        if state.binding() != binding {
            return Err(QueryError::Binding);
        }
        let mut entries = BTreeMap::new();
        for (id, value) in receivers {
            if entries.insert(id, value).is_some() {
                return Err(QueryError::DuplicateReceiver(id));
            }
        }
        Ok(Self {
            binding: binding.clone(),
            receivers: entries,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), QueryError> {
        if &self.binding == state.binding() {
            Ok(())
        } else {
            Err(QueryError::Binding)
        }
    }
    pub fn receiver(&self, id: u64) -> Result<&Receiver, QueryError> {
        self.receivers
            .get(&id)
            .ok_or(QueryError::MissingReceiver(id))
    }
    pub fn receiver_mut(&mut self, id: u64) -> Result<&mut Receiver, QueryError> {
        self.receivers
            .get_mut(&id)
            .ok_or(QueryError::MissingReceiver(id))
    }
    pub fn state(&self, id: u64) -> Result<i32, QueryError> {
        Ok(self.receiver(id)?.state())
    }
}
pub trait Context {
    type Error;
    fn state(&self) -> &LoadedState;
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
    fn definitions(&self) -> &Definitions;
    fn changed(&mut self, receiver: u64, old: i32, new: i32) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Query(QueryError),
    Consumer {
        receiver: u64,
        old: i32,
        new: i32,
        error: E,
    },
}
fn unit_interface(value: UnitInterface) -> Result<Option<UnitReference>, QueryError> {
    match value {
        UnitInterface::Absent => Ok(None),
        UnitInterface::Unit(id) => Ok(Some(UnitReference::from_component_id(id))),
        UnitInterface::Unavailable { object } => Err(QueryError::MissingInterface(object)),
    }
}
fn unit(state: &LoadedState, id: UnitReference) -> Result<&Unit, QueryError> {
    state
        .world()
        .unit(id)
        .map_err(QueryError::World)?
        .map(|u| u.record)
        .ok_or(QueryError::MissingUnit(id))
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, QueryError> {
    state.world().player(owner).map_err(QueryError::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(QueryError::MissingOwner(owner))
}
fn city(state: &LoadedState, token: (i32, i32)) -> Result<&c6_core::city::City, QueryError> {
    player(state, token.0)?
        .cities
        .records
        .entries
        .iter()
        .find(|c| c.index == token.1)
        .map(|c| &c.value)
        .ok_or(QueryError::MissingCity(token.0, token.1))
}
/// First matching founder in current native record order. The exact wrapper's
/// edit2a17950 only marks native archive transport metadata; no payload/callback
/// or RNG effect is omitted (existing exact-helper source closure).
pub fn religion_created(
    state: &LoadedState,
    definitions: &Definitions,
    owner: i32,
) -> Result<i32, QueryError> {
    let founder = player(state, owner)?.header.owner;
    Ok(state
        .retained()
        .religion
        .records
        .iter()
        .find(|r| r.founder == founder)
        .map_or(-1, |r| definitions.religion(r.religion_hash)))
}
pub fn religion_in_majority(
    state: &LoadedState,
    definitions: &Definitions,
    owner: i32,
) -> Result<i32, QueryError> {
    let count = definitions.religions.entries().len() as i32;
    if count <= 0 {
        return Ok(-1);
    }
    let cities = &player(state, owner)?.cities.records.entries;
    kernel::majority_of_cities(
        count,
        cities
            .iter()
            .map(|c| definitions.religion(c.value.religion.majority_religion_hash)),
    )
    .map_err(QueryError::Majority)
}
struct Read<'a> {
    state: &'a LoadedState,
    definitions: &'a Definitions,
    receiver: &'a Receiver,
    id: u64,
}
impl kernel::TestContext for Read<'_> {
    type Error = QueryError;
    type Unit = UnitReference;
    type Plot = u32;
    type City = (i32, i32);
    fn subject_has_plot(&mut self) -> Result<bool, QueryError> {
        self.receiver
            .subject
            .plot_interface
            .ok_or(QueryError::MissingPlotInterface)
    }
    fn subject_unit(&mut self) -> Result<Option<UnitReference>, QueryError> {
        unit_interface(self.receiver.subject.unit)
    }
    fn unit_plot(&mut self, id: UnitReference) -> Result<Option<u32>, QueryError> {
        let at = unit(self.state, id)?.k_location;
        if !geometry::plot_valid(at.x, at.y) {
            return Ok(None);
        }
        let map = self.state.world().map();
        if map.header.width <= 0 || map.header.height <= 0 {
            return Err(QueryError::MapGeometry);
        }
        let x = movement_world::coord_range(at.x, map.header.width, map.header.wrap_x);
        let y = movement_world::coord_range(at.y, map.header.height, map.header.wrap_y);
        Ok(map
            .plot_at(x, y)
            .map(|_| y.wrapping_mul(map.header.width).wrapping_add(x) as u32))
    }
    fn unit_owner(&mut self, id: UnitReference) -> Result<i32, QueryError> {
        Ok(unit(self.state, id)?.e_owner)
    }
    fn religion_created(&mut self, owner: i32) -> Result<i32, QueryError> {
        religion_created(self.state, self.definitions, owner)
    }
    fn gains_founder_of_majority(&mut self, owner: i32) -> Result<bool, QueryError> {
        Ok(player(self.state, owner)?
            .religion
            .founder_belief_of_majority_religion)
    }
    fn religion_in_majority(&mut self, owner: i32) -> Result<i32, QueryError> {
        religion_in_majority(self.state, self.definitions, owner)
    }
    fn owning_city(&mut self, index: u32) -> Result<Option<(i32, i32)>, QueryError> {
        let plot = self
            .state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(QueryError::MapGeometry)?;
        if plot.flags.get(c6_core::map::PlotFlag::HasCityAttributes)
            != plot.city_attributes.is_some()
        {
            return Err(QueryError::MissingCityAttributes);
        }
        let Some(attrs) = plot.city_attributes.as_ref() else {
            return Ok(None);
        };
        let owner = i32::from(attrs.player);
        if owner == -1 || attrs.owning_city == u32::MAX {
            return Ok(None);
        }
        if self
            .state
            .world()
            .valid_player(owner)
            .map_err(QueryError::World)?
            .is_none()
        {
            return Ok(None);
        }
        let slot = (attrs.owning_city & 0xffff) as i32;
        Ok(player(self.state, owner)?
            .cities
            .records
            .entries
            .iter()
            .any(|c| c.index == slot)
            .then_some((owner, slot)))
    }
    fn same_owner(&mut self) -> Result<bool, QueryError> {
        self.receiver
            .same_owner
            .ok_or(QueryError::MissingFlag(self.id))
    }
    fn city_majority(&mut self, token: (i32, i32)) -> Result<i32, QueryError> {
        Ok(self
            .definitions
            .religion(city(self.state, token)?.religion.majority_religion_hash))
    }
    fn city_owner(&mut self, token: (i32, i32)) -> Result<i32, QueryError> {
        Ok(city(self.state, token)?.header.owner)
    }
}
pub fn test(
    state: &LoadedState,
    runtime: &Runtime,
    definitions: &Definitions,
    receiver: u64,
) -> Result<i32, QueryError> {
    runtime.check(state)?;
    kernel::test(&mut Read {
        state,
        definitions,
        receiver: runtime.receiver(receiver)?,
        id: receiver,
    })
}
struct Live<'a, C> {
    owner: &'a mut C,
    receiver: u64,
    moved: Option<UnitReference>,
}
impl<C: Context> simple_requirement::Context for Live<'_, C> {
    type Error = Error<C::Error>;
    fn active(&mut self) -> Result<bool, Self::Error> {
        Ok(self
            .owner
            .runtime()
            .receiver(self.receiver)
            .map_err(Error::Query)?
            .active())
    }
    fn requirement(&mut self) -> Result<&mut simple_requirement::Requirement, Self::Error> {
        match &mut self
            .owner
            .runtime_mut()
            .receiver_mut(self.receiver)
            .map_err(Error::Query)?
            .simple
        {
            SimpleState::Defined(r) => Ok(r),
            SimpleState::DefinitionUnavailable { .. } => {
                Err(Error::Query(QueryError::MissingDefinition(self.receiver)))
            }
        }
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error> {
        self.owner
            .changed(self.receiver, old, new)
            .map_err(|error| Error::Consumer {
                receiver: self.receiver,
                old,
                new,
                error,
            })
    }
}
impl<C: Context> simple_requirement::TestContext for Live<'_, C> {
    fn test(&mut self) -> Result<i32, Self::Error> {
        test(
            self.owner.state(),
            self.owner.runtime(),
            self.owner.definitions(),
            self.receiver,
        )
        .map_err(Error::Query)
    }
}
impl<C: Context> kernel::Context for Live<'_, C> {
    fn context_unit_identity(&mut self) -> Result<Option<ComponentId>, Self::Error> {
        unit_interface(
            self.owner
                .runtime()
                .receiver(self.receiver)
                .map_err(Error::Query)?
                .context,
        )
        .map(|v| v.map(UnitReference::component_id))
        .map_err(Error::Query)
    }
    fn subject_unit_identity(&mut self) -> Result<Option<ComponentId>, Self::Error> {
        unit_interface(
            self.owner
                .runtime()
                .receiver(self.receiver)
                .map_err(Error::Query)?
                .subject
                .unit,
        )
        .map(|v| v.map(UnitReference::component_id))
        .map_err(Error::Query)
    }
    fn moved_unit_identity(&mut self) -> Result<ComponentId, Self::Error> {
        let moved = self
            .moved
            .ok_or(Error::Query(QueryError::MissingMovedUnit))?;
        let current = unit(self.owner.state(), moved).map_err(Error::Query)?;
        Ok(ComponentId {
            owner: current.e_owner as i16,
            component_type: 1,
            id: current.id,
        })
    }
}
pub fn on_unit_moved<C: Context>(
    owner: &mut C,
    receiver: u64,
    moved: UnitReference,
) -> Result<(), Error<C::Error>> {
    owner.runtime().check(owner.state()).map_err(Error::Query)?;
    kernel::on_unit_moved(&mut Live {
        owner,
        receiver,
        moved: Some(moved),
    })
}
pub fn invalidate<C: Context>(owner: &mut C, receiver: u64) -> Result<(), Error<C::Error>> {
    owner.runtime().check(owner.state()).map_err(Error::Query)?;
    kernel::invalidate(&mut Live {
        owner,
        receiver,
        moved: None,
    })
}
