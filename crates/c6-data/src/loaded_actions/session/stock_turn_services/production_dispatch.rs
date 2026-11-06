//! Select a production adapter over the same current gameplay owners.
use crate::{
    city_production_turn as production, district_actions::stock as district,
    loaded_actions::stock_notifications::city_production as building,
};
use c6_core::{
    city_production::{Order, QueueEntry},
    loaded_state::LoadedState,
};

pub(super) enum Effects<'s, 'runtime, 'a, 'd, 'r, P> {
    Building(building::Effects<'s, 'runtime, 'a, P>),
    Unit(building::unit::Effects<'s, 'runtime, 'd, 'r, P>),
    District(district::Effects<'s, 'runtime, 'a, P>),
}
macro_rules! dispatch {
    ($self:ident, $method:ident($($arg:expr),*)) => {
        match $self {
            Effects::Building(e) => production::Effects::$method(e, $($arg),*),
            Effects::Unit(e) => production::Effects::$method(e, $($arg),*),
            Effects::District(e) => production::Effects::$method(e, $($arg),*),
        }
    };
}
impl<P> production::Effects for Effects<'_, '_, '_, '_, '_, P> {
    fn population_services(&mut self) -> Option<&mut dyn crate::city_turn::PopulationEffects> {
        dispatch!(self, population_services())
    }
    fn calculate_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        y: i32,
        k: u32,
    ) -> Result<i32, production::Error> {
        dispatch!(self, calculate_yield_q8(s, o, c, y, k))
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        dispatch!(self, current_build(s, o, c, e))
    }
    fn start_progress(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        dispatch!(self, start_progress(s, o, c, e))
    }
    fn progress_report(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        dispatch!(self, progress_report(s, o, c, e))
    }
    fn finish(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<bool, production::Error> {
        dispatch!(self, finish(s, o, c, e))
    }
    fn finish_with_assignment(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
        assignment: &mut production::PopulationAssignment<'_>,
    ) -> Result<bool, production::Error> {
        dispatch!(self, finish_with_assignment(s, o, c, e, assignment))
    }
    fn new_front(&mut self, s: &mut LoadedState, o: i32, c: u32) -> Result<(), production::Error> {
        dispatch!(self, new_front(s, o, c))
    }
}

/// Empty queues still use ordinary city yield/overflow handling. Every queued
/// production kind must select its own mechanics before city upkeep starts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Building,
    District,
    Unit,
}
impl Kind {
    pub(super) fn for_order(order: Option<Order>) -> Result<Self, production::Error> {
        match order {
            None | Some(Order::Construct { .. }) => Ok(Self::Building),
            Some(Order::Zone { .. }) => Ok(Self::District),
            Some(Order::Train { .. }) => Ok(Self::Unit),
            Some(Order::Advance { .. } | Order::Other(_)) => Err(production::Error::Unsupported(
                "stock turn production order",
            )),
        }
    }
}

impl Kind {
    pub(super) fn borrow_sight<T>(
        self,
        sight: &std::cell::RefCell<T>,
    ) -> Result<Option<std::cell::RefMut<'_, T>>, production::Error> {
        if self == Self::Unit {
            sight.try_borrow_mut().map(Some).map_err(|_| {
                production::Error::Unsupported("current Unit production sight already borrowed")
            })
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn queued_unit_uses_factory_dispatch_instead_of_building() {
        let train = QueueEntry::train_request(0x1234);
        let building = QueueEntry::construct_request(0x1234);
        assert_eq!(Kind::for_order(Some(train.order)).unwrap(), Kind::Unit);
        assert_eq!(
            Kind::for_order(Some(building.order)).unwrap(),
            Kind::Building
        );
        assert_eq!(
            Kind::for_order(Some(Order::Zone {
                district_hash: 0x1234
            }))
            .unwrap(),
            Kind::District
        );
        assert_eq!(Kind::for_order(None).unwrap(), Kind::Building);
        for order in [
            Order::Advance {
                project_hash: 0x1234,
            },
            Order::Other(-1),
        ] {
            assert!(matches!(
                Kind::for_order(Some(order)),
                Err(production::Error::Unsupported(_))
            ));
        }
    }

    #[test]
    fn unit_factory_requires_exclusive_canonical_sight_and_preserves_mutations() {
        let sight = RefCell::new(vec![1]);
        let loan = sight.borrow();
        assert!(matches!(
            Kind::Unit.borrow_sight(&sight),
            Err(production::Error::Unsupported(_))
        ));
        // Building and district upkeep need no Unit factory loan.
        assert!(Kind::Building.borrow_sight(&sight).unwrap().is_none());
        assert!(Kind::District.borrow_sight(&sight).unwrap().is_none());
        drop(loan);
        let result = (|| -> Result<(), production::Error> {
            let mut factory_sight = Kind::Unit.borrow_sight(&sight)?.unwrap();
            factory_sight.push(2);
            Err(production::Error::Unsupported("later completion callback"))
        })();
        assert!(result.is_err());
        assert_eq!(*sight.borrow(), vec![1, 2]);
        assert!(Kind::Unit.borrow_sight(&sight).is_ok());
    }
}
