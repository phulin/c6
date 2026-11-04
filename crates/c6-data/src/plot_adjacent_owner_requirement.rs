//! Live PlotAdjacentToOwner evaluation over canonical loaded Units/City records.
//! Initial receiver fields and object identities are imported, never native Test
//! answers. Missing captured payloads remain explicit dependencies.
use crate::loaded_actions::city_range_attack;
use c6_core::{
    plot_adjacent_owner_requirement::Requirement,
    core_player::initialization::WireBinding,
    geometry,
    loaded_state::LoadedState,
    map::UnitReference,
    movement_world, plot_adjacent_owner_requirement as kernel,
    unit::{ComponentId, PlotCoord},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefinitionFlags {
    pub inverse: bool,
    pub persistent: bool,
    pub triggered: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlotObject {
    /// A source-proven class whose QueryInterface(IPlotInterface,true) is null.
    NoPlotInterface,
    Unit(ComponentId),
    City(ComponentId),
    District(ComponentId),
    /// The initial object pointer is known but its class/identity is unavailable.
    Unavailable {
        object: u64,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpleState {
    Defined(Requirement),
    DefinitionUnavailable { active: bool, state: i32 },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    pub simple: SimpleState,
    pub minimum_distance: i32,
    pub maximum_distance: i32,
    pub context: PlotObject,
    pub subject: PlotObject,
}
impl Receiver {
    #[allow(clippy::too_many_arguments)]
    pub fn from_initial(
        active: bool,
        state: i32,
        minimum_distance: i32,
        maximum_distance: i32,
        definition: Option<DefinitionFlags>,
        context: PlotObject,
        subject: PlotObject,
    ) -> Self {
        let simple = match definition {
            Some(flags) => SimpleState::Defined(Requirement {
                active,
                state,
                inverse: flags.inverse,
                persistent: flags.persistent,
                triggered: flags.triggered,
            }),
            None => SimpleState::DefinitionUnavailable { active, state },
        };
        Self {
            simple,
            minimum_distance,
            maximum_distance,
            context,
            subject,
        }
    }
    pub fn active(&self) -> bool {
        match &self.simple {
            SimpleState::Defined(r) => r.active,
            SimpleState::DefinitionUnavailable { active, .. } => *active,
        }
    }
    pub fn state(&self) -> i32 {
        match &self.simple {
            SimpleState::Defined(r) => r.state,
            SimpleState::DefinitionUnavailable { state, .. } => *state,
        }
    }
}
#[derive(Debug)]
pub enum QueryError {
    Binding,
    DuplicateReceiver(u64),
    MissingReceiver(u64),
    MissingObject(u64),
    MissingDefinition(u64),
    MissingOwner(i32),
    MapGeometry,
    World(c6_core::world::MissingInput),
}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PlotAdjacentToOwner: {self:?}")
    }
}
impl std::error::Error for QueryError {}

/// Logical requirement receiver ownership, retained across callbacks/actions.
/// Numeric receiver labels preserve registration identity; no native pointer is
/// dereferenced by production code. Dynamic state lives here, not in a transcript.
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
        let mut retained = BTreeMap::new();
        for (id, receiver) in receivers {
            if retained.insert(id, receiver).is_some() {
                return Err(QueryError::DuplicateReceiver(id));
            }
        }
        Ok(Self {
            binding: binding.clone(),
            receivers: retained,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), QueryError> {
        if state.binding() == &self.binding {
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

/// The enclosing owner keeps Runtime inside the same live Effects object. The
/// callback can mutate state, requirement registrations or Script storage without
/// extracting Runtime, cloning World or overwriting callback changes on return.
pub trait Context {
    type Error;
    fn state(&self) -> &LoadedState;
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
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

fn interface(object: PlotObject) -> Result<Option<PlotObject>, QueryError> {
    match object {
        PlotObject::NoPlotInterface => Ok(None),
        PlotObject::Unavailable { object } => Err(QueryError::MissingObject(object)),
        known => Ok(Some(known)),
    }
}
fn invalid() -> PlotCoord {
    PlotCoord { x: -9999, y: -9999 }
}
fn map_location(state: &LoadedState, at: Option<PlotCoord>) -> Result<PlotCoord, QueryError> {
    let Some(at) = at else { return Ok(invalid()) };
    if !geometry::plot_valid(at.x, at.y) {
        return Ok(invalid());
    }
    let map = state.world().map();
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(QueryError::MapGeometry);
    }
    let x = movement_world::coord_range(at.x, map.header.width, map.header.wrap_x);
    let y = movement_world::coord_range(at.y, map.header.height, map.header.wrap_y);
    Ok(if map.plot_at(x, y).is_some() {
        PlotCoord { x, y }
    } else {
        invalid()
    })
}
fn location(state: &LoadedState, object: PlotObject) -> Result<PlotCoord, QueryError> {
    let at = match object {
        PlotObject::Unit(id) => state
            .world()
            .unit(UnitReference::from_component_id(id))
            .map_err(QueryError::World)?
            .map(|u| u.record.k_location),
        PlotObject::City(id) | PlotObject::District(id) => {
            let owner = i32::from(id.owner);
            if state
                .world()
                .valid_player(owner)
                .map_err(QueryError::World)?
                .is_none()
            {
                return Ok(invalid());
            }
            let player = state
                .retained()
                .players
                .iter()
                .find(|p| p.requested_owner == owner)
                .ok_or(QueryError::MissingOwner(owner))?;
            if id.id == u32::MAX {
                return Ok(invalid());
            }
            let slot = (id.id & 0xffff) as i32;
            match object {
                PlotObject::City(_) => player
                    .cities
                    .records
                    .entries
                    .iter()
                    .find(|e| e.index == slot)
                    .map(|e| PlotCoord {
                        x: e.value.header.location_x,
                        y: e.value.header.location_y,
                    }),
                PlotObject::District(_) => player
                    .districts
                    .records
                    .entries
                    .iter()
                    .find(|e| e.index == slot)
                    .map(|e| PlotCoord {
                        x: e.value.location.0,
                        y: e.value.location.1,
                    }),
                _ => unreachable!(),
            }
        }
        PlotObject::Unavailable { object } => return Err(QueryError::MissingObject(object)),
        PlotObject::NoPlotInterface => return Err(QueryError::MissingObject(0)),
    };
    map_location(state, at)
}
fn fixed(object: PlotObject) -> Result<bool, QueryError> {
    match object {
        PlotObject::Unit(_) => Ok(false),
        PlotObject::City(_) | PlotObject::District(_) => Ok(true),
        PlotObject::Unavailable { object } => Err(QueryError::MissingObject(object)),
        PlotObject::NoPlotInterface => Err(QueryError::MissingObject(0)),
    }
}
struct Read<'a> {
    state: &'a LoadedState,
    receiver: &'a Receiver,
}
impl kernel::PlotContext for Read<'_> {
    type Error = QueryError;
    type Plot = PlotObject;
    fn distance_bounds(&mut self) -> Result<(i32, i32), Self::Error> {
        Ok((
            self.receiver.minimum_distance,
            self.receiver.maximum_distance,
        ))
    }
    fn context_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error> {
        interface(self.receiver.context)
    }
    fn subject_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error> {
        interface(self.receiver.subject)
    }
    fn location(&mut self, object: Self::Plot) -> Result<PlotCoord, Self::Error> {
        location(self.state, object)
    }
    fn fixed_location(&mut self, object: Self::Plot) -> Result<bool, Self::Error> {
        fixed(object)
    }
    fn distance(&mut self, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error> {
        Ok(city_range_attack::plot_distance(
            self.state,
            (a.x, a.y),
            (b.x, b.y),
        ))
    }
}
/// Compute the native enum from current World objects and map topology.
pub fn test(state: &LoadedState, runtime: &Runtime, receiver: u64) -> Result<i32, QueryError> {
    runtime.check(state)?;
    kernel::test(&mut Read {
        state,
        receiver: runtime.receiver(receiver)?,
    })
}
struct Live<'a, C> {
    owner: &'a mut C,
    receiver: u64,
}
impl<C: Context> Live<'_, C> {
    fn read(&self) -> Result<Read<'_>, Error<C::Error>> {
        Ok(Read {
            state: self.owner.state(),
            receiver: self
                .owner
                .runtime()
                .receiver(self.receiver)
                .map_err(Error::Query)?,
        })
    }
}
impl<C: Context> kernel::PlotContext for Live<'_, C> {
    type Error = Error<C::Error>;
    type Plot = PlotObject;
    fn distance_bounds(&mut self) -> Result<(i32, i32), Self::Error> {
        kernel::PlotContext::distance_bounds(&mut self.read()?).map_err(Error::Query)
    }
    fn context_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error> {
        kernel::PlotContext::context_plot(&mut self.read()?).map_err(Error::Query)
    }
    fn subject_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error> {
        kernel::PlotContext::subject_plot(&mut self.read()?).map_err(Error::Query)
    }
    fn location(&mut self, p: Self::Plot) -> Result<PlotCoord, Self::Error> {
        kernel::PlotContext::location(&mut self.read()?, p).map_err(Error::Query)
    }
    fn fixed_location(&mut self, p: Self::Plot) -> Result<bool, Self::Error> {
        kernel::PlotContext::fixed_location(&mut self.read()?, p).map_err(Error::Query)
    }
    fn distance(&mut self, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error> {
        kernel::PlotContext::distance(&mut self.read()?, a, b).map_err(Error::Query)
    }
}
impl<C: Context> kernel::Context for Live<'_, C> {
    fn active(&mut self) -> Result<bool, Self::Error> {
        Ok(self
            .owner
            .runtime()
            .receiver(self.receiver)
            .map_err(Error::Query)?
            .active())
    }
    fn requirement(&mut self) -> Result<&mut Requirement, Self::Error> {
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
/// UnitMoved's lambda ignores event coordinates and invokes this exact current
/// receiver. Errors after a state-change consumer preserve its completed prefix.
pub fn invalidate<C: Context>(owner: &mut C, receiver: u64) -> Result<(), Error<C::Error>> {
    owner.runtime().check(owner.state()).map_err(Error::Query)?;
    kernel::invalidate(&mut Live { owner, receiver })
}
