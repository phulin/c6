//! Source-bounded UnitMoved filters for PlotAdjacentFriendlyUnitTagMatches.
//!
//! This executes the source early-return path using current actor type and
//! admitted native-order definitions. Reached Test/positive-mask continuations
//! are explicit unsupported operations; this does not implement the whole Test.
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
    unit::ComponentId,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interface<T> {
    Absent,
    Present(T),
    Unavailable { object: u64 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectInterfaces {
    pub unit: Interface<ComponentId>,
    pub player: Interface<i32>,
    /// Presence only: the filtered path never invokes GetLocation.
    pub plot: Interface<()>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMask {
    pub bit_count: usize,
    pub words: Vec<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    pub active: bool,
    pub context: ObjectInterfaces,
    pub subject: ObjectInterfaces,
    pub type_mask: Option<TypeMask>,
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Resources(unit_initialization_definitions::Error),
    DuplicateReceiver(u64),
    MissingReceiver(u64),
    MissingInterface { object: u64 },
    MissingMask(u64),
    MaskBounds { receiver: u64, index: usize },
    MissingActor(ComponentId),
    MissingUnitDefinition(u32),
    World(c6_core::world::MissingInput),
    TestUnavailable(u64),
    PositiveMaskContinuationUnavailable(u64),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tag UnitMoved filter: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Initial cached masks and interface identities. No native Test results are
/// accepted. Definitions are initialized once and reused for every callback.
pub struct Runtime {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    receivers: BTreeMap<u64, Receiver>,
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
        receivers: Vec<(u64, Receiver)>,
    ) -> Result<Self, Error> {
        if authority.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Resources)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Resources)?;
        let mut values = BTreeMap::new();
        for (id, receiver) in receivers {
            if values.insert(id, receiver).is_some() {
                return Err(Error::DuplicateReceiver(id));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            units,
            receivers: values,
        })
    }
    pub fn receiver(&self, receiver: u64) -> Result<&Receiver, Error> {
        self.receivers
            .get(&receiver)
            .ok_or(Error::MissingReceiver(receiver))
    }
}
fn interface<T: Copy>(value: Interface<T>) -> Result<Option<T>, Error> {
    match value {
        Interface::Absent => Ok(None),
        Interface::Present(value) => Ok(Some(value)),
        Interface::Unavailable { object } => Err(Error::MissingInterface { object }),
    }
}
fn cascade<T: Copy>(subject: Interface<T>, context: Interface<T>) -> Result<Option<T>, Error> {
    match interface(subject)? {
        Some(value) => Ok(Some(value)),
        None => interface(context),
    }
}
fn filter(
    receiver: u64,
    value: &Receiver,
    current: ComponentId,
    type_index: impl FnOnce() -> Result<usize, Error>,
) -> Result<(), Error> {
    // Native CascadeQueryInterface searches subject before context.
    if cascade(value.subject.unit, value.context.unit)? == Some(current) {
        return if value.active {
            Err(Error::TestUnavailable(receiver))
        } else {
            Ok(())
        };
    }
    let player = cascade(value.subject.player, value.context.player)?;
    // The IPlot query occurs even when the player interface is null.
    let plot = interface(value.subject.plot)?;
    if player.is_none() || plot.is_none() {
        return Ok(());
    }
    let index = type_index()?;
    let mask = value
        .type_mask
        .as_ref()
        .ok_or(Error::MissingMask(receiver))?;
    if index >= mask.bit_count {
        return Err(Error::MaskBounds { receiver, index });
    }
    let word = mask
        .words
        .get(index / 64)
        .ok_or(Error::MaskBounds { receiver, index })?;
    if word & (1u64 << (index % 64)) == 0 {
        return Ok(());
    }
    // Positive masks reach friendliness, adjacency and/or live Test. Never
    // substitute a predicate value or silently discard their possible effects.
    Err(Error::PositiveMaskContinuationUnavailable(receiver))
}
/// Exact early-return filter over the canonical moved actor. Only source-proven
/// no-effect branches return Ok; positive/Test paths remain explicit errors.
pub fn filter_unit_moved(
    state: &LoadedState,
    runtime: &Runtime,
    receiver: u64,
    current: ComponentId,
) -> Result<(), Error> {
    if state.binding() != &runtime.binding {
        return Err(Error::Binding);
    }
    let value = runtime.receiver(receiver)?;
    filter(receiver, value, current, || {
        let unit = state
            .world()
            .unit(UnitReference::from_component_id(current))
            .map_err(Error::World)?
            .ok_or(Error::MissingActor(current))?
            .record;
        runtime
            .units
            .find_index(unit.e_type_index)
            .ok_or(Error::MissingUnitDefinition(unit.e_type_index))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn id(id: u32) -> ComponentId {
        ComponentId {
            owner: 0,
            component_type: 1,
            id,
        }
    }
    fn object(unit: Interface<ComponentId>) -> ObjectInterfaces {
        ObjectInterfaces {
            unit,
            player: Interface::Present(0),
            plot: Interface::Present(()),
        }
    }
    fn receiver() -> Receiver {
        Receiver {
            active: true,
            context: object(Interface::Present(id(9))),
            subject: object(Interface::Present(id(8))),
            type_mask: Some(TypeMask {
                bit_count: 129,
                words: vec![0, 1 << 5, 0],
            }),
        }
    }
    #[test]
    fn native_mask_false_skips_unimplemented_positive_path() {
        let r = receiver();
        assert!(filter(7, &r, id(1), || Ok(1)).is_ok());
        assert!(matches!(
            filter(7, &r, id(1), || Ok(69)),
            Err(Error::PositiveMaskContinuationUnavailable(7))
        ));
    }
    #[test]
    fn subject_interface_wins_and_unused_context_is_not_required() {
        let mut r = receiver();
        r.context.unit = Interface::Unavailable { object: 55 };
        assert!(filter(7, &r, id(1), || Ok(1)).is_ok());
        r.subject.unit = Interface::Absent;
        assert!(matches!(
            filter(7, &r, id(1), || panic!("type query too early")),
            Err(Error::MissingInterface { object: 55 })
        ));
    }
    #[test]
    fn matched_active_receiver_reaches_test_before_mask_queries() {
        let mut r = receiver();
        r.subject.unit = Interface::Present(id(1));
        r.type_mask = None;
        assert!(matches!(
            filter(7, &r, id(1), || panic!("type queried after self match")),
            Err(Error::TestUnavailable(7))
        ));
        r.active = false;
        assert!(filter(7, &r, id(1), || panic!("inactive Test")).is_ok());
    }
    #[test]
    fn null_player_still_requires_the_source_subject_plot_query() {
        let mut r = receiver();
        r.subject.player = Interface::Absent;
        r.context.player = Interface::Absent;
        r.subject.plot = Interface::Unavailable { object: 77 };
        assert!(matches!(
            filter(7, &r, id(1), || panic!("type queried before interfaces")),
            Err(Error::MissingInterface { object: 77 })
        ));
    }
}
