//! Concrete stock OnCityPowerChanged subscribers. These kernels do not infer the
//! current dynamic requirement subscription list from a saved RequirementSystem.
//! Requirement-change and gossip-report continuations remain synchronous effects.
use crate::unit::ComponentId;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectCity {
    NotCity,
    NullCity,
    City(ComponentId),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    /// Actual SimpleRequirement.m_IsActive flag, not transport record presence.
    pub active: bool,
    pub state: i32,
    pub inverse: bool,
    pub persistent: bool,
}
pub trait RequirementContext {
    type Error;
    type Handle: Copy;
    fn requirement(&mut self, handle: Self::Handle) -> Result<&mut Requirement, Self::Error>;
    fn subject_city(&mut self, handle: Self::Handle) -> Result<SubjectCity, Self::Error>;
    /// Execute current Power.IsFullyPowered including the lazy active-project
    /// override. The data Power definitions implement that computation.
    fn city_is_fully_powered(&mut self, city: ComponentId) -> Result<bool, Self::Error>;
    /// Actual SimpleRequirement change signal, after the new state is stored.
    /// Reentrant modifier updates/RNG may not be replaced with a no-op.
    fn requirement_changed(
        &mut self,
        handle: Self::Handle,
        old: i32,
        new: i32,
    ) -> Result<(), Self::Error>;
}
pub fn inverse_state(state: i32) -> i32 {
    match state {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        _ => 1,
    }
}
/// CityIsPowered::OnCityPowerChanged -> Invalidate -> Test -> SetState.
/// The incoming first-generation bool is unused by this subscriber.
pub fn city_is_powered_changed<C: RequirementContext>(
    c: &mut C,
    handle: C::Handle,
    emitted_city: ComponentId,
) -> Result<(), C::Error> {
    if c.subject_city(handle)? != SubjectCity::City(emitted_city) {
        return Ok(());
    }
    if !c.requirement(handle)?.active {
        return Ok(());
    }
    // Test queries the subject again, rather than caching the earlier interface.
    let tested = match c.subject_city(handle)? {
        SubjectCity::City(city) => {
            if c.city_is_fully_powered(city)? {
                0
            } else {
                1
            }
        }
        SubjectCity::NotCity | SubjectCity::NullCity => 1,
    };
    // Native SetState tests active status again after the virtual Test call.
    let (old, new) = {
        let state = c.requirement(handle)?;
        if !state.active {
            return Ok(());
        }
        let old = state.state;
        state.state = if state.inverse {
            inverse_state(tested)
        } else {
            tested
        };
        if state.persistent && state.state == 0 {
            state.state = 4;
        }
        (old, state.state)
    };
    if old != new || new == 4 {
        c.requirement_changed(handle, old, new)?;
    }
    Ok(())
}

pub const POWERED_CITY_GOSSIP: u32 = 0x8061f96e;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoweredCityOccurrence {
    pub turn: i32,
    pub player: i32,
    pub city_id: u32,
}
/// Shared logical object identity preserves the native retained occurrence while
/// ReportGossip and its receiver callbacks run; no native address is required.
pub type Occurrence = Rc<RefCell<PoweredCityOccurrence>>;
pub trait GossipContext {
    type Error;
    fn city_owner(&mut self) -> Result<i32, Self::Error>;
    fn city_id(&mut self) -> Result<u32, Self::Error>;
    fn turn_visibility_count(&mut self) -> Result<usize, Self::Error>;
    fn current_turn(&mut self) -> Result<i32, Self::Error>;
    /// Append this exact logical occurrence to the current general Gossip pending
    /// list (preserving its existing entries), before ReportGossip.
    fn append_powered_city(&mut self, occurrence: Occurrence) -> Result<(), Self::Error>;
    /// Actual visibility/filter/alive-major distribution and SendGossip effects.
    /// The append remains committed if this continuation fails.
    fn report_powered_city(&mut self, occurrence: Occurrence) -> Result<(), Self::Error>;
}
/// Stock Gossip.PoweredCity handler. False performs no access/allocation. True
/// constructs an occurrence, and AddGossipItem drops it if visibility is empty.
pub fn powered_city_changed<C: GossipContext>(
    c: &mut C,
    first_generation: bool,
) -> Result<(), C::Error> {
    if !first_generation {
        return Ok(());
    }
    let player = c.city_owner()?;
    let occurrence = Rc::new(RefCell::new(PoweredCityOccurrence {
        turn: -1,
        player,
        city_id: c.city_id()?,
    }));
    if c.turn_visibility_count()? == 0 {
        return Ok(());
    }
    occurrence.borrow_mut().turn = c.current_turn()?;
    c.append_powered_city(Rc::clone(&occurrence))?;
    c.report_powered_city(occurrence)
}

#[derive(Debug)]
pub enum CounterError {
    /// Wire vectors with duplicate/out-of-order keys are not the current native
    /// ordered maps. The owner must apply actual Read construction first.
    NotLogicalMap,
    Allocation(std::collections::TryReserveError),
}
/// Native IncrementGossipReports field computation for an already-met viewed
/// player. Missing outer map is deliberately NOT created by the target body.
/// SendGossip performs HasMet before reaching this call; that gate and its later
/// localization/status callbacks belong to the caller, not this field kernel.
pub fn increment_powered_gossip_reports(
    reports: &mut crate::player_diplomacy::GossipReports,
    viewed_player: i32,
) -> Result<(), CounterError> {
    if !reports.windows(2).all(|pair| pair[0].0 < pair[1].0) {
        return Err(CounterError::NotLogicalMap);
    }
    let Some((_, types)) = reports
        .iter_mut()
        .find(|(player, _)| *player == viewed_player)
    else {
        return Ok(());
    };
    if !types
        .entries
        .windows(2)
        .all(|pair| (pair[0].0 as u32) < (pair[1].0 as u32))
    {
        return Err(CounterError::NotLogicalMap);
    }
    let at = types
        .entries
        .partition_point(|(kind, _)| (*kind as u32) < POWERED_CITY_GOSSIP);
    if let Some((kind, count)) = types.entries.get_mut(at) {
        if *kind as u32 == POWERED_CITY_GOSSIP {
            *count = count.wrapping_add(1);
            return Ok(());
        }
    }
    types
        .entries
        .try_reserve(1)
        .map_err(CounterError::Allocation)?;
    types.entries.insert(at, (POWERED_CITY_GOSSIP as i32, 1));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u32) -> ComponentId {
        ComponentId {
            owner: 0,
            component_type: 2,
            id: n,
        }
    }
    struct R {
        r: Requirement,
        subject: SubjectCity,
        reads: usize,
        events: Vec<(i32, i32)>,
        powered: bool,
        fail: bool,
    }
    impl RequirementContext for R {
        type Error = &'static str;
        type Handle = ();
        fn requirement(&mut self, _: ()) -> Result<&mut Requirement, Self::Error> {
            Ok(&mut self.r)
        }
        fn subject_city(&mut self, _: ()) -> Result<SubjectCity, Self::Error> {
            self.reads += 1;
            Ok(self.subject)
        }
        fn city_is_fully_powered(&mut self, _: ComponentId) -> Result<bool, Self::Error> {
            Ok(self.powered)
        }
        fn requirement_changed(&mut self, _: (), old: i32, new: i32) -> Result<(), Self::Error> {
            assert_eq!(self.r.state, new);
            self.events.push((old, new));
            if self.fail {
                Err("changed")
            } else {
                Ok(())
            }
        }
    }
    fn r() -> R {
        R {
            r: Requirement {
                active: true,
                state: 1,
                inverse: false,
                persistent: false,
            },
            subject: SubjectCity::City(id(3)),
            reads: 0,
            events: vec![],
            powered: true,
            fail: false,
        }
    }
    #[test]
    fn matching_subject_retests_and_stores_before_signal_failure() {
        let mut c = r();
        c.fail = true;
        assert_eq!(city_is_powered_changed(&mut c, (), id(3)), Err("changed"));
        assert_eq!(c.r.state, 0);
        assert_eq!(c.events, [(1, 0)]);
        assert_eq!(c.reads, 2);
    }
    #[test]
    fn other_city_and_inactive_do_not_test() {
        let mut c = r();
        city_is_powered_changed(&mut c, (), id(4)).unwrap();
        assert_eq!(c.reads, 1);
        assert!(c.events.is_empty());
        c.r.active = false;
        c.reads = 0;
        city_is_powered_changed(&mut c, (), id(3)).unwrap();
        assert_eq!(c.reads, 1);
    }
    #[test]
    fn persistent_success_dispatches_even_when_already_four() {
        let mut c = r();
        c.r.persistent = true;
        c.r.state = 4;
        city_is_powered_changed(&mut c, (), id(3)).unwrap();
        assert_eq!(c.events, [(4, 4)]);
    }
    #[test]
    fn inverse_and_unchanged_state_follow_native_table() {
        let mut c = r();
        c.r.inverse = true;
        city_is_powered_changed(&mut c, (), id(3)).unwrap();
        assert_eq!(c.r.state, 1);
        assert!(c.events.is_empty());
        assert_eq!(
            (0..6).map(inverse_state).collect::<Vec<_>>(),
            [1, 0, 3, 2, 1, 1]
        );
    }
    struct G {
        pending: Vec<Occurrence>,
        count: usize,
        accesses: usize,
        fail: bool,
    }
    impl GossipContext for G {
        type Error = &'static str;
        fn city_owner(&mut self) -> Result<i32, Self::Error> {
            self.accesses += 1;
            Ok(3)
        }
        fn city_id(&mut self) -> Result<u32, Self::Error> {
            self.accesses += 1;
            Ok(17)
        }
        fn turn_visibility_count(&mut self) -> Result<usize, Self::Error> {
            self.accesses += 1;
            Ok(self.count)
        }
        fn current_turn(&mut self) -> Result<i32, Self::Error> {
            self.accesses += 1;
            Ok(592)
        }
        fn append_powered_city(&mut self, o: Occurrence) -> Result<(), Self::Error> {
            self.pending.push(o);
            Ok(())
        }
        fn report_powered_city(&mut self, o: Occurrence) -> Result<(), Self::Error> {
            assert!(Rc::ptr_eq(self.pending.last().unwrap(), &o));
            assert_eq!(o.borrow().turn, 592);
            if self.fail {
                Err("report")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn gossip_false_is_early_and_empty_visibility_does_not_append() {
        let mut c = G {
            pending: vec![],
            count: 0,
            accesses: 0,
            fail: false,
        };
        powered_city_changed(&mut c, false).unwrap();
        assert_eq!(c.accesses, 0);
        powered_city_changed(&mut c, true).unwrap();
        assert_eq!(c.accesses, 3);
        assert!(c.pending.is_empty());
    }
    #[test]
    fn gossip_appends_same_object_before_report_and_retains_on_failure() {
        let mut c = G {
            pending: vec![],
            count: 1,
            accesses: 0,
            fail: true,
        };
        assert_eq!(powered_city_changed(&mut c, true), Err("report"));
        assert_eq!(
            *c.pending[0].borrow(),
            PoweredCityOccurrence {
                turn: 592,
                player: 3,
                city_id: 17
            }
        );
    }
    #[test]
    fn gossip_counters_wrap_and_insert_by_unsigned_key_without_creating_outer_map() {
        use crate::player_diplomacy::TypeMap;
        let mut reports = vec![(3, TypeMap::from(vec![(1, 9), (u32::MAX as i32, 7)]))];
        increment_powered_gossip_reports(&mut reports, 5).unwrap();
        assert_eq!(reports.len(), 1);
        increment_powered_gossip_reports(&mut reports, 3).unwrap();
        assert_eq!(
            reports[0].1.entries,
            vec![(1, 9), (POWERED_CITY_GOSSIP as i32, 1), (-1, 7)]
        );
        reports[0].1.entries[1].1 = u32::MAX;
        increment_powered_gossip_reports(&mut reports, 3).unwrap();
        assert_eq!(reports[0].1.entries[1].1, 0);
    }
    #[test]
    fn raw_duplicate_gossip_maps_refuse_before_mutation() {
        let mut reports = vec![(3, vec![(1, 2), (1, 9)].into())];
        let before = reports.clone();
        assert!(matches!(
            increment_powered_gossip_reports(&mut reports, 3),
            Err(CounterError::NotLogicalMap)
        ));
        assert_eq!(reports, before);
    }
}
