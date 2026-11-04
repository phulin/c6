//! Linux GameSignals.UnitMoved and its two GameEffects movement fanouts.
//! Current registration order is a required runtime input. This does not infer
//! listeners from an observed Lua call, Rules rows, or saved Requirement states.
use crate::unit::{ComponentId, PlotCoord, Unit};

pub mod modifiers;
pub mod requirement_sets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subscriber {
    LuaOnUnitMoved,
    GameEffectsUnitFanout,
    GameEffectsPlayerFanout,
    /// Requires the actual requirement receiver/Test/state-change consumers.
    Requirement {
        receiver: u64,
    },
    Achievement,
    Unknown {
        identity: u64,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subscription(usize);
#[derive(Clone, Copy, Debug)]
struct Link {
    next: usize,
    previous: usize,
    subscriber: Option<Subscriber>,
    handle_alive: bool,
    retained: usize,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    Allocation,
    MissingGlobalOrder,
    MissingUnitScopes,
    MissingUnitLookup(ComponentId),
    MissingPlayerScopes,
    DuplicateScope,
    InvalidSubscription,
    CursorInvalidated,
    RetainOverflow,
}

/// One specific UnitMoved listener ring. Logical labels preserve callback
/// insertion/removal behavior; native allocations, addresses and pool sizes are
/// not simulated. Removed labels are never reused while this owner is alive.
#[derive(Debug)]
pub struct ListenerRing {
    links: Vec<Link>,
}
impl ListenerRing {
    pub fn from_ordered(subscribers: &[Subscriber]) -> Result<Self, RegistryError> {
        let mut ring = Self { links: Vec::new() };
        ring.links
            .try_reserve(subscribers.len().saturating_add(1))
            .map_err(|_| RegistryError::Allocation)?;
        ring.links.push(Link {
            next: 0,
            previous: 0,
            subscriber: None,
            handle_alive: true,
            retained: 0,
        });
        for &subscriber in subscribers {
            ring.subscribe(subscriber)?;
        }
        Ok(ring)
    }
    /// Native Subscribe appends immediately before the sentinel. Appending in
    /// a callback can therefore be reached by this same dispatch.
    pub fn subscribe(&mut self, subscriber: Subscriber) -> Result<Subscription, RegistryError> {
        self.links
            .try_reserve(1)
            .map_err(|_| RegistryError::Allocation)?;
        let previous = self.links[0].previous;
        let id = self.links.len();
        self.links.push(Link {
            next: 0,
            previous,
            subscriber: Some(subscriber),
            handle_alive: true,
            retained: 0,
        });
        self.links[previous].next = id;
        self.links[0].previous = id;
        Ok(Subscription(id))
    }
    /// A held dispatch cursor keeps a removed current link until its next
    /// pointer has been read; its callable is cleared immediately. An unheld
    /// removed successor is skipped.
    pub fn unsubscribe(&mut self, subscription: Subscription) -> Result<(), RegistryError> {
        let link = self
            .links
            .get_mut(subscription.0)
            .filter(|l| l.handle_alive && subscription.0 != 0)
            .ok_or(RegistryError::InvalidSubscription)?;
        link.handle_alive = false;
        // Unsubscribe clears std::function immediately, before Dec_Ref. A
        // held current link keeps its next/previous pointers, not its callback.
        link.subscriber = None;
        if link.retained == 0 {
            self.unlink(subscription.0)?;
        }
        Ok(())
    }
    fn unlink(&mut self, id: usize) -> Result<(), RegistryError> {
        let link = *self.links.get(id).ok_or(RegistryError::CursorInvalidated)?;
        self.links
            .get_mut(link.previous)
            .ok_or(RegistryError::CursorInvalidated)?
            .next = link.next;
        self.links
            .get_mut(link.next)
            .ok_or(RegistryError::CursorInvalidated)?
            .previous = link.previous;
        let removed = &mut self.links[id];
        removed.subscriber = None;
        removed.next = 0;
        removed.previous = 0;
        Ok(())
    }
    fn retain(&mut self, id: usize) -> Result<(), RegistryError> {
        let link = self
            .links
            .get_mut(id)
            .ok_or(RegistryError::CursorInvalidated)?;
        link.retained = link
            .retained
            .checked_add(1)
            .ok_or(RegistryError::RetainOverflow)?;
        Ok(())
    }
    fn release(&mut self, id: usize) -> Result<(), RegistryError> {
        let link = self
            .links
            .get_mut(id)
            .ok_or(RegistryError::CursorInvalidated)?;
        link.retained = link
            .retained
            .checked_sub(1)
            .ok_or(RegistryError::CursorInvalidated)?;
        if !link.handle_alive && link.retained == 0 {
            self.unlink(id)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    Global,
    Unit(ComponentId),
    Player(i32),
}
/// None is unavailable input, while Some(empty) is an actual empty map/ring.
/// This owner must remain live across callbacks; replacing a whole scoped
/// receiver during an active dispatch is unsupported, just as actor replacement.
pub struct Registry {
    global: Option<ListenerRing>,
    units: Option<Vec<(ComponentId, ListenerRing)>>,
    players: Option<Vec<(i32, ListenerRing)>>,
    unit_lookups: Option<Vec<(ComponentId, Option<ListenerRing>)>>,
}
impl Registry {
    pub fn new(
        global: Option<ListenerRing>,
        units: Option<Vec<(ComponentId, ListenerRing)>>,
        players: Option<Vec<(i32, ListenerRing)>>,
    ) -> Result<Self, RegistryError> {
        if units.as_ref().is_some_and(|v| {
            v.iter()
                .enumerate()
                .any(|(i, a)| v[..i].iter().any(|b| a.0 == b.0))
        }) || players.as_ref().is_some_and(|v| {
            v.iter()
                .enumerate()
                .any(|(i, a)| v[..i].iter().any(|b| a.0 == b.0))
        }) {
            return Err(RegistryError::DuplicateScope);
        }
        Ok(Self {
            global,
            units,
            players,
            unit_lookups: None,
        })
    }
    /// Partial initial/current inputs: each supplied key is either an actual
    /// present scoped ring or an actual absent map entry. Every unsupplied key
    /// remains unknown. No complete-map reconstruction is required for one Find.
    pub fn with_unit_lookups(
        mut self,
        lookups: Vec<(ComponentId, Option<ListenerRing>)>,
    ) -> Result<Self, RegistryError> {
        if self.units.is_some()
            || lookups
                .iter()
                .enumerate()
                .any(|(i, a)| lookups[..i].iter().any(|b| a.0 == b.0))
        {
            return Err(RegistryError::DuplicateScope);
        }
        self.unit_lookups = Some(lookups);
        Ok(self)
    }
    pub fn global_mut(&mut self) -> Result<&mut ListenerRing, RegistryError> {
        self.global
            .as_mut()
            .ok_or(RegistryError::MissingGlobalOrder)
    }
    pub fn unit_mut(
        &mut self,
        id: ComponentId,
    ) -> Result<Option<&mut ListenerRing>, RegistryError> {
        if let Some(units) = self.units.as_mut() {
            return Ok(units.iter_mut().find(|r| r.0 == id).map(|r| &mut r.1));
        }
        if let Some(lookups) = self.unit_lookups.as_mut() {
            return lookups
                .iter_mut()
                .find(|r| r.0 == id)
                .map(|r| r.1.as_mut())
                .ok_or(RegistryError::MissingUnitLookup(id));
        }
        Err(RegistryError::MissingUnitScopes)
    }
    pub fn player_mut(&mut self, owner: i32) -> Result<Option<&mut ListenerRing>, RegistryError> {
        Ok(self
            .players
            .as_mut()
            .ok_or(RegistryError::MissingPlayerScopes)?
            .iter_mut()
            .find(|r| r.0 == owner)
            .map(|r| &mut r.1))
    }
    fn find(&self, signal: Signal) -> Result<Option<&ListenerRing>, RegistryError> {
        match signal {
            Signal::Global => self
                .global
                .as_ref()
                .map(Some)
                .ok_or(RegistryError::MissingGlobalOrder),
            Signal::Unit(id) => {
                if let Some(units) = self.units.as_ref() {
                    return Ok(units.iter().find(|r| r.0 == id).map(|r| &r.1));
                }
                if let Some(lookups) = self.unit_lookups.as_ref() {
                    return lookups
                        .iter()
                        .find(|r| r.0 == id)
                        .map(|r| r.1.as_ref())
                        .ok_or(RegistryError::MissingUnitLookup(id));
                }
                Err(RegistryError::MissingUnitScopes)
            }
            Signal::Player(owner) => Ok(self
                .players
                .as_ref()
                .ok_or(RegistryError::MissingPlayerScopes)?
                .iter()
                .find(|r| r.0 == owner)
                .map(|r| &r.1)),
        }
    }
    fn edit(&mut self, signal: Signal) -> Result<&mut ListenerRing, RegistryError> {
        match signal {
            Signal::Global => self.global_mut(),
            Signal::Unit(id) => self.unit_mut(id)?.ok_or(RegistryError::CursorInvalidated),
            Signal::Player(owner) => self
                .player_mut(owner)?
                .ok_or(RegistryError::CursorInvalidated),
        }
    }
}

pub trait Context {
    type Error;
    /// Same retained live Unit; callbacks do not replace it with a copied record.
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn registrations(&self) -> Result<&Registry, Self::Error>;
    fn registrations_mut(&mut self) -> Result<&mut Registry, Self::Error>;
    /// Dispatch only a source-qualified concrete requirement implementation.
    /// Other receiver kinds remain unsupported; no predicate result is supplied.
    fn on_requirement(
        &mut self,
        receiver: u64,
        _old: PlotCoord,
        _new: PlotCoord,
    ) -> Result<(), Error<Self::Error>> {
        Err(Error::UnsupportedRequirement(receiver))
    }
    /// Execute the stock achievement listener or a source-proven inert branch.
    fn on_achievement(
        &mut self,
        _old: PlotCoord,
        _new: PlotCoord,
    ) -> Result<(), Error<Self::Error>> {
        Err(Error::UnsupportedAchievement)
    }
    /// The existing stock Utility.CallHook/ScriptState Call6 leaf, with full
    /// synchronization and reached Lua/GC effects. This is not whole dispatch.
    fn call_on_unit_moved(
        &mut self,
        current: ComponentId,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Context(E),
    Registry(RegistryError),
    UnsupportedRequirement(u64),
    UnsupportedAchievement,
    UnknownSubscriber(u64),
}
fn identity<C: Context>(c: &C) -> Result<ComponentId, Error<C::Error>> {
    let u = c.unit().map_err(Error::Context)?;
    Ok(ComponentId {
        owner: u.e_owner as i16,
        component_type: 1,
        id: u.id,
    })
}
/// Constructor lambda $_571: resolve CURRENT Unit ComponentID in the per-unit
/// map; missing key is inert. This does not insert a map entry.
pub fn game_effects_unit_moved<C: Context>(
    c: &mut C,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    let id = identity(c)?;
    dispatch_signal(c, Signal::Unit(id), old, new)
}
/// Constructor lambda $_572: reread CURRENT owner, even after the preceding
/// per-unit callback changed it. Missing player key is inert, not an empty-input
/// assumption; the current map itself must be supplied.
pub fn game_effects_player_unit_moved<C: Context>(
    c: &mut C,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    let owner = c.unit().map_err(Error::Context)?.e_owner;
    dispatch_signal(c, Signal::Player(owner), old, new)
}
fn invoke<C: Context>(
    c: &mut C,
    subscriber: Subscriber,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    match subscriber {
        Subscriber::LuaOnUnitMoved => {
            let id = identity(c)?;
            c.call_on_unit_moved(id, old, new).map_err(Error::Context)
        }
        Subscriber::GameEffectsUnitFanout => game_effects_unit_moved(c, old, new),
        Subscriber::GameEffectsPlayerFanout => game_effects_player_unit_moved(c, old, new),
        Subscriber::Requirement { receiver } => c.on_requirement(receiver, old, new),
        Subscriber::Achievement => c.on_achievement(old, new),
        Subscriber::Unknown { identity } => Err(Error::UnknownSubscriber(identity)),
    }
}
fn dispatch_signal<C: Context>(
    c: &mut C,
    signal: Signal,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    if c.registrations()
        .map_err(Error::Context)?
        .find(signal)
        .map_err(Error::Registry)?
        .is_none()
    {
        return Ok(());
    }
    c.registrations_mut()
        .map_err(Error::Context)?
        .edit(signal)
        .map_err(Error::Registry)?
        .retain(0)
        .map_err(Error::Registry)?;
    let mut current = 0;
    loop {
        let route = c
            .registrations()
            .map_err(Error::Context)?
            .find(signal)
            .map_err(Error::Registry)?
            .ok_or(Error::Registry(RegistryError::CursorInvalidated))?
            .links
            .get(current)
            .ok_or(Error::Registry(RegistryError::CursorInvalidated))?
            .subscriber;
        if let Some(route) = route {
            if let Err(error) = invoke(c, route, old, new) {
                // Dispose only this implementation's cursor hold on failure.
                c.registrations_mut()
                    .map_err(Error::Context)?
                    .edit(signal)
                    .map_err(Error::Registry)?
                    .release(current)
                    .map_err(Error::Registry)?;
                return Err(error);
            }
        }
        let ring = c
            .registrations_mut()
            .map_err(Error::Context)?
            .edit(signal)
            .map_err(Error::Registry)?;
        let next = ring
            .links
            .get(current)
            .ok_or(Error::Registry(RegistryError::CursorInvalidated))?
            .next;
        ring.retain(next).map_err(Error::Registry)?;
        ring.release(current).map_err(Error::Registry)?;
        current = next;
        if current == 0 {
            ring.release(current).map_err(Error::Registry)?;
            return Ok(());
        }
    }
}
/// Whole UnitMoved: dispatch supplied live registration order, never a fixed
/// Lua-only substitute. Unknown reached routes stop with a typed failure and
/// leave the completed gameplay prefix intact. No full action is implied.
pub fn dispatch<C: Context>(
    c: &mut C,
    old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    dispatch_signal(c, Signal::Global, old, new)
}

#[cfg(test)]
mod tests {
    use super::*;
    const OLD: PlotCoord = PlotCoord { x: 81, y: 58 };
    const NEW: PlotCoord = PlotCoord { x: 80, y: 57 };
    struct Fixture {
        unit: Unit,
        registry: Registry,
        calls: Vec<ComponentId>,
        mutate_owner: bool,
        mutate_links: bool,
        recurse: bool,
    }
    impl Fixture {
        fn new(order: &[Subscriber]) -> Self {
            let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
            unit.e_owner = 1;
            unit.id = 10;
            Self {
                unit,
                registry: Registry::new(
                    Some(ListenerRing::from_ordered(order).unwrap()),
                    Some(vec![]),
                    Some(vec![]),
                )
                .unwrap(),
                calls: vec![],
                mutate_owner: false,
                mutate_links: false,
                recurse: false,
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn registrations(&self) -> Result<&Registry, Self::Error> {
            Ok(&self.registry)
        }
        fn registrations_mut(&mut self) -> Result<&mut Registry, Self::Error> {
            Ok(&mut self.registry)
        }
        fn call_on_unit_moved(
            &mut self,
            id: ComponentId,
            old: PlotCoord,
            new: PlotCoord,
        ) -> Result<(), Self::Error> {
            assert_eq!((old, new), (OLD, NEW));
            self.calls.push(id);
            if self.mutate_owner {
                self.mutate_owner = false;
                self.unit.e_owner = 7;
                self.unit.id = 11;
            }
            if self.mutate_links {
                self.mutate_links = false;
                let ring = self.registry.global_mut().unwrap();
                ring.unsubscribe(Subscription(1)).unwrap();
                ring.unsubscribe(Subscription(2)).unwrap();
                ring.subscribe(Subscriber::LuaOnUnitMoved).unwrap();
            }
            if self.recurse {
                self.recurse = false;
                self.registry
                    .global_mut()
                    .unwrap()
                    .unsubscribe(Subscription(1))
                    .unwrap();
                dispatch(self, old, new).unwrap();
            }
            Ok(())
        }
    }
    #[test]
    fn fanouts_use_actual_current_identity_then_reread_owner_after_callbacks() {
        let mut f = Fixture::new(&[
            Subscriber::GameEffectsUnitFanout,
            Subscriber::GameEffectsPlayerFanout,
            Subscriber::LuaOnUnitMoved,
        ]);
        let initial = ComponentId {
            owner: 1,
            component_type: 1,
            id: 10,
        };
        f.registry.units = Some(vec![(
            initial,
            ListenerRing::from_ordered(&[Subscriber::LuaOnUnitMoved]).unwrap(),
        )]);
        f.registry.players = Some(vec![
            (
                1,
                ListenerRing::from_ordered(&[Subscriber::Unknown { identity: 1 }]).unwrap(),
            ),
            (
                7,
                ListenerRing::from_ordered(&[Subscriber::LuaOnUnitMoved]).unwrap(),
            ),
        ]);
        f.mutate_owner = true;
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(
            f.calls,
            vec![
                initial,
                ComponentId {
                    owner: 7,
                    component_type: 1,
                    id: 11
                },
                ComponentId {
                    owner: 7,
                    component_type: 1,
                    id: 11
                }
            ]
        );
    }
    #[test]
    fn callback_removal_skips_successor_and_append_is_reached_this_dispatch() {
        let mut f = Fixture::new(&[Subscriber::LuaOnUnitMoved; 3]);
        f.mutate_links = true;
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 3);
        f.calls.clear();
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 2);
    }
    #[test]
    fn removed_current_callable_is_skipped_by_nested_dispatch_while_link_is_retained() {
        let mut f = Fixture::new(&[Subscriber::LuaOnUnitMoved; 2]);
        f.recurse = true;
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 3);
        f.calls.clear();
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 1);
    }
    #[test]
    fn unknown_current_routes_and_missing_maps_are_not_empty_lua_only_lists() {
        let mut f = Fixture::new(&[
            Subscriber::Requirement { receiver: 42 },
            Subscriber::LuaOnUnitMoved,
        ]);
        assert_eq!(
            dispatch(&mut f, OLD, NEW),
            Err(Error::UnsupportedRequirement(42))
        );
        assert!(f.calls.is_empty());
        f.registry.global = Some(
            ListenerRing::from_ordered(&[
                Subscriber::GameEffectsUnitFanout,
                Subscriber::LuaOnUnitMoved,
            ])
            .unwrap(),
        );
        f.registry.units = None;
        assert_eq!(
            dispatch(&mut f, OLD, NEW),
            Err(Error::Registry(RegistryError::MissingUnitScopes))
        );
        assert!(f.calls.is_empty());
        f.registry.units = Some(vec![]);
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 1);
        f.registry.global = None;
        assert_eq!(
            dispatch(&mut f, OLD, NEW),
            Err(Error::Registry(RegistryError::MissingGlobalOrder))
        );
    }
    #[test]
    fn partial_unit_lookup_does_not_turn_unqueried_callback_changed_keys_into_absence() {
        let mut f = Fixture::new(&[
            Subscriber::GameEffectsUnitFanout,
            Subscriber::GameEffectsUnitFanout,
        ]);
        let id = ComponentId {
            owner: 1,
            component_type: 1,
            id: 10,
        };
        f.registry.units = None;
        f.registry = f
            .registry
            .with_unit_lookups(vec![(
                id,
                Some(ListenerRing::from_ordered(&[Subscriber::LuaOnUnitMoved]).unwrap()),
            )])
            .unwrap();
        f.mutate_owner = true;
        assert_eq!(
            dispatch(&mut f, OLD, NEW),
            Err(Error::Registry(RegistryError::MissingUnitLookup(
                ComponentId {
                    owner: 7,
                    component_type: 1,
                    id: 11
                }
            )))
        );
        assert_eq!(f.calls.len(), 1);
        let mut f = Fixture::new(&[
            Subscriber::GameEffectsUnitFanout,
            Subscriber::LuaOnUnitMoved,
        ]);
        f.registry.units = None;
        f.registry = f.registry.with_unit_lookups(vec![(id, None)]).unwrap();
        assert_eq!(dispatch(&mut f, OLD, NEW), Ok(()));
        assert_eq!(f.calls.len(), 1);
    }
}
