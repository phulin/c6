//! Canonical logical CombatProposals owner. Current counter, ordered objects
//! and listener maps are explicit inputs, never reconstructed as empty defaults.
use crate::{
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
    unit_moved_signal::{ListenerRing, RegistryError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Participant {
    Unit(UnitReference),
    District(ComponentId),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostCombatYield {
    pub yield_type: i32,
    pub amount: i32,
    /// Exact third/fourth constructor arguments; their downstream yield
    /// interpretation is not implemented by proposal registration.
    pub option_1: bool,
    pub option_2: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    /// Current ModifierObject owned modifier IDs used by ClearModifiers.
    pub owned_modifiers: Vec<i32>,
    /// Native object virtual IsTemporary, distinct from registration lifetime.
    pub temporary: bool,
    /// Current ModifierObject.MarkAsAffected entries, not a strength cache.
    pub modifiers_affecting: crate::unit_moved_signal::modifiers::AffectedModifiers,
    pub attacker: Participant,
    pub defender: Option<Participant>,
    pub plot: Option<PlotCoord>,
    pub combat_type: u32,
    pub attacker_strength_modifier: i32,
    pub defender_strength_modifier: i32,
    pub capture_combat_units: bool,
    pub capture_unit_type: i32,
    pub attacker_yields: Vec<PostCombatYield>,
    pub defender_yields: Vec<PostCombatYield>,
}
impl Proposal {
    /// Linux Unit/District constructor23caa00, before registry callbacks.
    pub fn melee_unit_district(
        attacker: UnitReference,
        defender: ComponentId,
        plot: Option<PlotCoord>,
    ) -> Self {
        let mut proposal = Self::melee_units(attacker, attacker, plot);
        proposal.defender = Some(Participant::District(defender));
        proposal
    }
    /// Native Unit/Unit ProposedCombat23ca920 defaults, before ObjectCreated.
    pub fn melee_units(
        attacker: UnitReference,
        defender: UnitReference,
        plot: Option<PlotCoord>,
    ) -> Self {
        Self {
            owned_modifiers: vec![],
            temporary: true,
            modifiers_affecting:
                crate::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(vec![])
                    .expect("new object has empty sorted modifier list"),
            attacker: Participant::Unit(attacker),
            defender: Some(Participant::Unit(defender)),
            plot,
            combat_type: super::super::results::MELEE,
            attacker_strength_modifier: 0,
            defender_strength_modifier: 0,
            capture_combat_units: false,
            capture_unit_type: -1,
            attacker_yields: vec![],
            defender_yields: vec![],
        }
    }
    /// Linux District/Unit ProposedCombat constructor. The district is a real
    /// gameplay participant; it must never be represented by a synthetic Unit.
    pub fn ranged_district_unit(
        attacker: ComponentId,
        defender: UnitReference,
        plot: Option<PlotCoord>,
    ) -> Self {
        Self {
            owned_modifiers: vec![],
            temporary: true,
            modifiers_affecting:
                crate::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(vec![])
                    .expect("new object has empty sorted modifier list"),
            attacker: Participant::District(attacker),
            defender: Some(Participant::Unit(defender)),
            plot,
            combat_type: 0x2ec4ce4d,
            attacker_strength_modifier: 0,
            defender_strength_modifier: 0,
            capture_combat_units: false,
            capture_unit_type: -1,
            attacker_yields: vec![],
            defender_yields: vec![],
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Handle(usize);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Subscriber {
    Owners,
    UnitComponents,
    Callback(u64),
    Unknown(u64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signal {
    Global,
    Owner(i32),
    Unit(ComponentId),
}
#[derive(Debug)]
pub enum Error {
    Allocation,
    MissingRuntime,
    MissingSignal(Signal),
    DuplicateScope,
    InvalidHandle,
    MissingDestructionSignal,
    MissingCollectionModifiers,
    Affected(crate::unit_moved_signal::modifiers::AffectedError),
    MissingCollections,
    AlreadyInitialized,
    Ring(RegistryError),
}
#[derive(Debug)]
pub struct Runtime {
    collection_modifiers: Option<super::collection_modifiers::Runtime>,
    destruction: Vec<Option<ListenerRing<u64>>>,
    counter: u32,
    // Stable logical labels retain a proposal across Forget/reentrant appends.
    // This is object identity bookkeeping, not a second gameplay payload copy.
    objects: Vec<Proposal>,
    object_ids: Vec<u32>,
    active: Vec<(u32, Handle)>,
    global: Option<ListenerRing<Subscriber>>,
    owners: Option<Vec<(i32, ListenerRing<Subscriber>)>>,
    units: Option<Vec<(ComponentId, ListenerRing<Subscriber>)>>,
    collections: Option<super::collections::Runtime>,
}
impl Runtime {
    pub fn from_initial(
        counter: u32,
        active: Vec<(u32, Proposal)>,
        global: Option<ListenerRing<Subscriber>>,
        owners: Option<Vec<(i32, ListenerRing<Subscriber>)>>,
        units: Option<Vec<(ComponentId, ListenerRing<Subscriber>)>>,
    ) -> Result<Self, Error> {
        if owners.as_ref().is_some_and(|v| {
            v.iter()
                .enumerate()
                .any(|(i, a)| v[..i].iter().any(|b| a.0 == b.0))
        }) || units.as_ref().is_some_and(|v| {
            v.iter()
                .enumerate()
                .any(|(i, a)| v[..i].iter().any(|b| a.0 == b.0))
        }) {
            return Err(Error::DuplicateScope);
        }
        let (ids, objects): (Vec<_>, Vec<_>) = active
            .into_iter()
            .enumerate()
            .map(|(i, (id, p))| ((id, Handle(i)), p))
            .unzip();
        let object_ids = ids.iter().map(|r: &(u32, Handle)| r.0).collect();
        let destruction = (0..objects.len()).map(|_| None).collect();
        Ok(Self {
            collection_modifiers: None,
            destruction,
            counter,
            objects,
            object_ids,
            active: ids,
            global,
            owners,
            units,
            collections: None,
        })
    }
    pub fn with_collections(
        mut self,
        collections: super::collections::Runtime,
    ) -> Result<Self, Error> {
        if self.collections.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        self.collections = Some(collections);
        Ok(self)
    }
    pub fn collections(&self) -> Result<&super::collections::Runtime, Error> {
        self.collections.as_ref().ok_or(Error::MissingCollections)
    }
    pub fn collections_mut(&mut self) -> Result<&mut super::collections::Runtime, Error> {
        self.collections.as_mut().ok_or(Error::MissingCollections)
    }
    pub fn with_collection_modifiers(
        mut self,
        modifiers: super::collection_modifiers::Runtime,
    ) -> Self {
        self.collection_modifiers = Some(modifiers);
        self
    }
    pub fn collection_modifiers(&self) -> Result<&super::collection_modifiers::Runtime, Error> {
        self.collection_modifiers
            .as_ref()
            .ok_or(Error::MissingCollectionModifiers)
    }
    pub fn collection_modifiers_mut(
        &mut self,
    ) -> Result<&mut super::collection_modifiers::Runtime, Error> {
        self.collection_modifiers
            .as_mut()
            .ok_or(Error::MissingCollectionModifiers)
    }
    pub fn destruction_mut(&mut self, handle: Handle) -> Result<&mut ListenerRing<u64>, Error> {
        self.destruction
            .get_mut(handle.0)
            .ok_or(Error::InvalidHandle)?
            .as_mut()
            .ok_or(Error::MissingDestructionSignal)
    }
    pub fn mark_subject_affected(&mut self, modifier: u64, handle: Handle) -> Result<(), Error> {
        let m = self
            .collection_modifiers
            .as_ref()
            .ok_or(Error::MissingCollectionModifiers)?
            .modifier(modifier)
            .ok_or(Error::MissingCollectionModifiers)?;
        self.objects
            .get_mut(handle.0)
            .ok_or(Error::InvalidHandle)?
            .modifiers_affecting
            .mark(&m.definition, m.instance)
            .map_err(Error::Affected)
    }
    /// Object identity remains readable after Forget removes its active entry.
    pub fn object_id(&self, handle: Handle) -> Result<u32, Error> {
        self.object_ids
            .get(handle.0)
            .copied()
            .ok_or(Error::InvalidHandle)
    }
    /// Remove registration before destruction callbacks, retaining the one live payload.
    pub(crate) fn forget_registration(&mut self, id: u32) -> Option<Handle> {
        let position = self.active.iter().position(|r| r.0 == id)?;
        Some(self.active.remove(position).1)
    }
    pub fn counter(&self) -> u32 {
        self.counter
    }
    pub fn active(&self) -> &[(u32, Handle)] {
        &self.active
    }
    pub fn proposal(&self, handle: Handle) -> Result<&Proposal, Error> {
        self.objects.get(handle.0).ok_or(Error::InvalidHandle)
    }
    pub fn proposal_mut(&mut self, handle: Handle) -> Result<&mut Proposal, Error> {
        self.objects.get_mut(handle.0).ok_or(Error::InvalidHandle)
    }
    /// Counter increment precedes allocation/registration; callback failures
    /// must not undo the newly registered object or any earlier callback writes.
    pub fn register(&mut self, proposal: Proposal) -> Result<(u32, Handle), Error> {
        self.counter = self.counter.wrapping_add(1);
        self.objects.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.destruction
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        let destruction = ListenerRing::from_ordered(&[]).map_err(Error::Ring)?;
        self.active.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.object_ids
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        let handle = Handle(self.objects.len());
        self.objects.push(proposal);
        self.object_ids.push(self.counter);
        self.destruction.push(Some(destruction));
        self.active.push((self.counter, handle));
        Ok((self.counter, handle))
    }
    /// UnitCombat.OnInitialize23434f0 calls WhenUnitCombatProposed, whose map
    /// insertion creates a signal only inside the admitted current Unit map.
    pub fn subscribe_unit_combat(&mut self, unit: ComponentId, receiver: u64) -> Result<(), Error> {
        let units = self
            .units
            .as_mut()
            .ok_or(Error::MissingSignal(Signal::Unit(unit)))?;
        let index = if let Some(index) = units.iter().position(|row| row.0 == unit) {
            index
        } else {
            units.push((unit, ListenerRing::from_ordered(&[]).map_err(Error::Ring)?));
            units.len() - 1
        };
        units[index]
            .1
            .subscribe(Subscriber::Callback(receiver))
            .map_err(Error::Ring)?;
        Ok(())
    }
    pub fn ring(&self, signal: Signal) -> Result<Option<&ListenerRing<Subscriber>>, Error> {
        Ok(match signal {
            Signal::Global => Some(self.global.as_ref().ok_or(Error::MissingSignal(signal))?),
            Signal::Owner(id) => self
                .owners
                .as_ref()
                .ok_or(Error::MissingSignal(signal))?
                .iter()
                .find(|v| v.0 == id)
                .map(|v| &v.1),
            Signal::Unit(id) => self
                .units
                .as_ref()
                .ok_or(Error::MissingSignal(signal))?
                .iter()
                .find(|v| v.0 == id)
                .map(|v| &v.1),
        })
    }
    pub fn ring_mut(
        &mut self,
        signal: Signal,
    ) -> Result<Option<&mut ListenerRing<Subscriber>>, Error> {
        Ok(match signal {
            Signal::Global => Some(self.global.as_mut().ok_or(Error::MissingSignal(signal))?),
            Signal::Owner(id) => self
                .owners
                .as_mut()
                .ok_or(Error::MissingSignal(signal))?
                .iter_mut()
                .find(|v| v.0 == id)
                .map(|v| &mut v.1),
            Signal::Unit(id) => self
                .units
                .as_mut()
                .ok_or(Error::MissingSignal(signal))?
                .iter_mut()
                .find(|v| v.0 == id)
                .map(|v| &mut v.1),
        })
    }
}

pub trait Context {
    type Error;
    fn runtime(&self) -> Result<&Runtime, Self::Error>;
    fn runtime_mut(&mut self) -> Result<&mut Runtime, Self::Error>;
    fn owner(&mut self, participant: Participant) -> Result<i32, Self::Error>;
    fn unit_id(&mut self, unit: UnitReference) -> Result<ComponentId, Self::Error>;
    /// One source-qualified synchronous registered receiver, on this same state.
    fn callback(&mut self, receiver: u64, proposal: Handle) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum DispatchError<E> {
    Context(E),
    Runtime(Error),
    UnknownSubscriber(u64),
}
use DispatchError::{Context as C, Runtime as R};

pub fn propose<Cx: Context>(
    c: &mut Cx,
    proposal: Proposal,
) -> Result<(u32, Handle), DispatchError<Cx::Error>> {
    let result = c.runtime_mut().map_err(C)?.register(proposal).map_err(R)?;
    dispatch(c, Signal::Global, result.1)?;
    Ok(result)
}
fn dispatch<Cx: Context>(
    c: &mut Cx,
    signal: Signal,
    handle: Handle,
) -> Result<(), DispatchError<Cx::Error>> {
    if c.runtime().map_err(C)?.ring(signal).map_err(R)?.is_none() {
        return Ok(());
    }
    let mut cursor = c
        .runtime_mut()
        .map_err(C)?
        .ring_mut(signal)
        .map_err(R)?
        .ok_or(R(Error::MissingSignal(signal)))?
        .cursor_start()
        .map_err(|e| R(Error::Ring(e)))?;
    loop {
        let route = c
            .runtime()
            .map_err(C)?
            .ring(signal)
            .map_err(R)?
            .ok_or(R(Error::MissingSignal(signal)))?
            .cursor_route(cursor)
            .map_err(|e| R(Error::Ring(e)))?;
        if let Some(route) = route {
            if let Err(error) = invoke(c, route, handle) {
                c.runtime_mut()
                    .map_err(C)?
                    .ring_mut(signal)
                    .map_err(R)?
                    .ok_or(R(Error::MissingSignal(signal)))?
                    .cursor_release(cursor)
                    .map_err(|e| R(Error::Ring(e)))?;
                return Err(error);
            }
        }
        match c
            .runtime_mut()
            .map_err(C)?
            .ring_mut(signal)
            .map_err(R)?
            .ok_or(R(Error::MissingSignal(signal)))?
            .cursor_next(cursor)
            .map_err(|e| R(Error::Ring(e)))?
        {
            Some(next) => cursor = next,
            None => return Ok(()),
        }
    }
}
fn invoke<Cx: Context>(
    c: &mut Cx,
    route: Subscriber,
    handle: Handle,
) -> Result<(), DispatchError<Cx::Error>> {
    match route {
        Subscriber::Callback(receiver) => c.callback(receiver, handle).map_err(C),
        Subscriber::Unknown(id) => Err(DispatchError::UnknownSubscriber(id)),
        Subscriber::Owners => {
            let p = c.runtime().map_err(C)?.proposal(handle).map_err(R)?;
            let (a, b) = (p.attacker, p.defender);
            let a = c.owner(a).map_err(C)?;
            let b = if let Some(b) = b {
                c.owner(b).map_err(C)?
            } else {
                -1
            };
            if a != -1 {
                dispatch(c, Signal::Owner(a), handle)?;
            }
            if b != -1 && b != a {
                dispatch(c, Signal::Owner(b), handle)?;
            }
            Ok(())
        }
        Subscriber::UnitComponents => {
            let p = c.runtime().map_err(C)?.proposal(handle).map_err(R)?;
            let (a, b) = (Some(p.attacker), p.defender);
            let invalid = ComponentId {
                owner: -1,
                component_type: 0,
                id: u32::MAX,
            };
            let a = if let Some(Participant::Unit(u)) = a {
                c.unit_id(u).map_err(C)?
            } else {
                invalid
            };
            let b = if let Some(Participant::Unit(u)) = b {
                c.unit_id(u).map_err(C)?
            } else {
                invalid
            };
            if super::component_valid(a) {
                dispatch(c, Signal::Unit(a), handle)?;
            }
            if super::component_valid(b) && b != a {
                dispatch(c, Signal::Unit(b), handle)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            component_type: 1,
            id,
        })
    }
    fn proposal() -> Proposal {
        Proposal::melee_units(unit(12, 7), unit(62, 8), Some(PlotCoord { x: 60, y: 45 }))
    }
    struct Test {
        runtime: Runtime,
        events: Vec<u64>,
        fail: bool,
        append: bool,
    }
    impl Context for Test {
        type Error = &'static str;
        fn runtime(&self) -> Result<&Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn runtime_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn owner(&mut self, p: Participant) -> Result<i32, Self::Error> {
            match p {
                Participant::Unit(u) => Ok(i32::from(u.component_id().owner)),
                Participant::District(id) => Ok(i32::from(id.owner)),
            }
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, id: u64, h: Handle) -> Result<(), Self::Error> {
            self.events.push(id);
            self.runtime
                .proposal_mut(h)
                .unwrap()
                .attacker_strength_modifier += 7;
            if self.append && id == 1 {
                self.runtime
                    .ring_mut(Signal::Global)
                    .unwrap()
                    .unwrap()
                    .subscribe(Subscriber::Callback(2))
                    .unwrap();
            }
            if self.fail {
                Err("callback")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn district_proposal_dispatches_both_owners_and_only_real_defender_unit() {
        let mut c = context(&[Subscriber::Owners, Subscriber::UnitComponents], 0);
        let district = ComponentId {
            owner: 12,
            component_type: 3,
            id: 7,
        };
        let defender = unit(62, 8);
        c.runtime.owners = Some(vec![
            (
                12,
                ListenerRing::from_ordered(&[Subscriber::Callback(1)]).unwrap(),
            ),
            (
                62,
                ListenerRing::from_ordered(&[Subscriber::Callback(2)]).unwrap(),
            ),
        ]);
        c.runtime.units = Some(vec![(
            defender.component_id(),
            ListenerRing::from_ordered(&[Subscriber::Callback(3)]).unwrap(),
        )]);
        let (id, handle) = propose(
            &mut c,
            Proposal::ranged_district_unit(district, defender, Some(PlotCoord { x: 1, y: 2 })),
        )
        .unwrap();
        assert_eq!(id, 1);
        assert_eq!(c.events, [1, 2, 3]);
        let p = c.runtime.proposal(handle).unwrap();
        assert_eq!(p.attacker, Participant::District(district));
        assert_eq!(p.defender, Some(Participant::Unit(defender)));
        assert_eq!(p.combat_type, 0x2ec4ce4d);
        assert_eq!(p.attacker_strength_modifier, 21);
    }
    fn context(order: &[Subscriber], counter: u32) -> Test {
        Test {
            runtime: Runtime::from_initial(
                counter,
                vec![],
                Some(ListenerRing::from_ordered(order).unwrap()),
                Some(vec![]),
                Some(vec![]),
            )
            .unwrap(),
            events: vec![],
            fail: false,
            append: false,
        }
    }
    #[test]
    fn unit_combat_attachment_appends_to_admitted_current_signal() {
        let unit = UnitReference { owner: 0, id: 3 }.component_id();
        let mut runtime = Runtime::from_initial(5, vec![], None, None, Some(vec![])).unwrap();
        runtime.subscribe_unit_combat(unit, 10).unwrap();
        runtime.subscribe_unit_combat(unit, 11).unwrap();
        let ring = runtime.ring_mut(Signal::Unit(unit)).unwrap().unwrap();
        let head = ring.cursor_start().unwrap();
        let first = ring.cursor_next(head).unwrap().unwrap();
        assert_eq!(
            ring.cursor_route(first).unwrap(),
            Some(Subscriber::Callback(10))
        );
        let next = ring.cursor_next(first).unwrap().unwrap();
        assert_eq!(
            ring.cursor_route(next).unwrap(),
            Some(Subscriber::Callback(11))
        );
        ring.cursor_release(next).unwrap();
        assert_eq!(runtime.counter(), 5);
        assert!(runtime.active().is_empty());
        let mut unknown = Runtime::from_initial(5, vec![], None, None, None).unwrap();
        assert!(matches!(
            unknown.subscribe_unit_combat(unit, 10),
            Err(Error::MissingSignal(Signal::Unit(_)))
        ));
    }

    #[test]
    fn counter_wrap_and_registered_payload_survive_callback_error() {
        let mut c = context(&[Subscriber::Callback(1)], u32::MAX);
        c.fail = true;
        assert!(matches!(
            propose(&mut c, proposal()),
            Err(DispatchError::Context("callback"))
        ));
        assert_eq!(c.runtime.counter(), 0);
        let (id, h) = c.runtime.active()[0];
        assert_eq!(id, 0);
        assert_eq!(c.runtime.proposal(h).unwrap().attacker_strength_modifier, 7);
        c.fail = false;
        assert_eq!(propose(&mut c, proposal()).unwrap().0, 1);
        assert_eq!(c.events, [1, 1]);
    }
    #[test]
    fn callback_added_during_global_dispatch_is_reached() {
        let mut c = context(&[Subscriber::Callback(1)], 7832);
        c.append = true;
        let (id, h) = propose(&mut c, proposal()).unwrap();
        assert_eq!(id, 7833);
        assert_eq!(c.events, [1, 2]);
        assert_eq!(
            c.runtime.proposal(h).unwrap().attacker_strength_modifier,
            14
        );
    }
    #[test]
    fn stock_fanouts_use_actual_maps_and_missing_input_is_not_empty() {
        let mut c = context(&[Subscriber::Owners, Subscriber::UnitComponents], 7832);
        c.runtime.owners = Some(vec![(
            12,
            ListenerRing::from_ordered(&[Subscriber::Callback(10)]).unwrap(),
        )]);
        c.runtime.units = Some(vec![
            (
                unit(12, 7).component_id(),
                ListenerRing::from_ordered(&[Subscriber::Callback(11)]).unwrap(),
            ),
            (
                unit(62, 8).component_id(),
                ListenerRing::from_ordered(&[Subscriber::Callback(12)]).unwrap(),
            ),
        ]);
        propose(&mut c, proposal()).unwrap();
        assert_eq!(c.events, [10, 11, 12]);
        c.runtime.units = None;
        assert!(matches!(
            propose(&mut c, proposal()),
            Err(DispatchError::Runtime(Error::MissingSignal(Signal::Unit(
                _
            ))))
        ));
        assert_eq!(c.runtime.active().len(), 2);
    }
}
