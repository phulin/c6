//! Linux UnitNearReligiousCity Test and its two independent UnitMoved filters.
use crate::{simple_requirement, unit::ComponentId};

pub trait TestContext {
    type Error;
    type Unit: Copy;
    type Plot: Copy;
    type City: Copy;
    fn subject_has_plot(&mut self) -> Result<bool, Self::Error>;
    fn subject_unit(&mut self) -> Result<Option<Self::Unit>, Self::Error>;
    fn unit_plot(&mut self, unit: Self::Unit) -> Result<Option<Self::Plot>, Self::Error>;
    fn unit_owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn religion_created(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn gains_founder_of_majority(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn religion_in_majority(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn owning_city(&mut self, plot: Self::Plot) -> Result<Option<Self::City>, Self::Error>;
    fn same_owner(&mut self) -> Result<bool, Self::Error>;
    fn city_majority(&mut self, city: Self::City) -> Result<i32, Self::Error>;
    fn city_owner(&mut self, city: Self::City) -> Result<i32, Self::Error>;
}
/// 0x2397d60. OwningCity is queried even when the player's religion is absent.
pub fn test<C: TestContext>(c: &mut C) -> Result<i32, C::Error> {
    if !c.subject_has_plot()? {
        return Ok(3);
    }
    let Some(unit) = c.subject_unit()? else {
        return Ok(1);
    };
    let Some(plot) = c.unit_plot(unit)? else {
        return Ok(1);
    };
    let owner = c.unit_owner(unit)?;
    let mut religion = c.religion_created(owner)?;
    if religion == -1 && c.gains_founder_of_majority(owner)? {
        religion = c.religion_in_majority(owner)?;
    }
    let city = c.owning_city(plot)?;
    if religion == -1 {
        return Ok(1);
    }
    let Some(city) = city else {
        return Ok(1);
    };
    let same_owner = c.same_owner()?;
    let majority = c.city_majority(city)?;
    let city_owner = c.city_owner(city)?;
    let owner = c.unit_owner(unit)?;
    Ok(i32::from(if same_owner {
        city_owner != owner || majority != religion
    } else {
        city_owner == owner || majority != religion
    }))
}

pub trait Context: simple_requirement::TestContext {
    fn context_unit_identity(&mut self) -> Result<Option<ComponentId>, Self::Error>;
    fn subject_unit_identity(&mut self) -> Result<Option<ComponentId>, Self::Error>;
    fn moved_unit_identity(&mut self) -> Result<ComponentId, Self::Error>;
}
pub fn invalidate<C: Context>(c: &mut C) -> Result<(), C::Error> {
    simple_requirement::invalidate(c)
}
/// 0x2397ee0. Subject identity is reread after the first possible invalidation;
/// an equal context and subject cause TWO invalidations, without deduplication.
pub fn on_unit_moved<C: Context>(c: &mut C) -> Result<(), C::Error> {
    if let Some(identity) = c.context_unit_identity()? {
        if identity == c.moved_unit_identity()? {
            invalidate(c)?;
        }
    }
    if let Some(identity) = c.subject_unit_identity()? {
        if identity == c.moved_unit_identity()? {
            invalidate(c)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MajorityError {
    ResourceLimit,
    InvalidReligionIndex(i32),
}
/// 0x2b5ee50: count all cities, skip native types -1/0, scan real religion
/// indexes in ascending order, and require strictly more than half of ALL cities.
pub fn majority_of_cities(
    definition_count: i32,
    cities: impl IntoIterator<Item = i32>,
) -> Result<i32, MajorityError> {
    if definition_count <= 0 {
        return Ok(-1);
    }
    let mut counts = Vec::new();
    counts
        .try_reserve_exact(definition_count as usize)
        .map_err(|_| MajorityError::ResourceLimit)?;
    counts.resize(definition_count as usize, 0i32);
    let mut total = 0u32;
    for religion in cities {
        total = total.wrapping_add(1);
        if religion == -1 || religion == 0 {
            continue;
        }
        let slot = usize::try_from(religion)
            .ok()
            .and_then(|i| counts.get_mut(i))
            .ok_or(MajorityError::InvalidReligionIndex(religion))?;
        *slot = slot.wrapping_add(1);
    }
    let mut best = -1;
    let mut most = 0;
    for (religion, count) in counts.into_iter().enumerate().skip(1) {
        if count > most {
            most = count;
            best = religion as i32;
        }
    }
    Ok(if most > (total >> 1) as i32 { best } else { -1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn majority_counts_unreligious_cities_and_requires_strict_half() {
        assert_eq!(majority_of_cities(4, [1, 1, 0, -1]), Ok(-1));
        assert_eq!(majority_of_cities(4, [2, 2, 0]), Ok(2));
        assert_eq!(majority_of_cities(4, [2, 1]), Ok(-1));
        assert_eq!(majority_of_cities(1, []), Ok(-1));
        assert_eq!(
            majority_of_cities(3, [3]),
            Err(MajorityError::InvalidReligionIndex(3))
        );
        assert_eq!(
            majority_of_cities(0, std::iter::from_fn(|| panic!())),
            Ok(-1)
        );
    }
    struct C {
        r: simple_requirement::Requirement,
        events: Vec<&'static str>,
        religion: i32,
        city: bool,
        flag: bool,
        context: bool,
        subject: bool,
        change_subject: bool,
        tests: usize,
    }
    impl C {
        fn new() -> Self {
            Self {
                r: simple_requirement::Requirement {
                    active: true,
                    state: 1,
                    inverse: false,
                    persistent: false,
                    triggered: false,
                },
                events: vec![],
                religion: -1,
                city: false,
                flag: false,
                context: true,
                subject: true,
                change_subject: false,
                tests: 0,
            }
        }
    }
    impl TestContext for C {
        type Error = ();
        type Unit = ();
        type Plot = ();
        type City = ();
        fn subject_has_plot(&mut self) -> Result<bool, ()> {
            self.events.push("plot-interface");
            self.tests += 1;
            Ok(true)
        }
        fn subject_unit(&mut self) -> Result<Option<()>, ()> {
            self.events.push("unit-interface");
            Ok(Some(()))
        }
        fn unit_plot(&mut self, _: ()) -> Result<Option<()>, ()> {
            self.events.push("unit-plot");
            Ok(Some(()))
        }
        fn unit_owner(&mut self, _: ()) -> Result<i32, ()> {
            self.events.push("unit-owner");
            Ok(0)
        }
        fn religion_created(&mut self, _: i32) -> Result<i32, ()> {
            self.events.push("created");
            Ok(self.religion)
        }
        fn gains_founder_of_majority(&mut self, _: i32) -> Result<bool, ()> {
            self.events.push("gains-founder");
            Ok(false)
        }
        fn religion_in_majority(&mut self, _: i32) -> Result<i32, ()> {
            panic!()
        }
        fn owning_city(&mut self, _: ()) -> Result<Option<()>, ()> {
            self.events.push("city");
            Ok(self.city.then_some(()))
        }
        fn same_owner(&mut self) -> Result<bool, ()> {
            self.events.push("flag");
            Ok(self.flag)
        }
        fn city_majority(&mut self, _: ()) -> Result<i32, ()> {
            self.events.push("city-majority");
            Ok(self.religion)
        }
        fn city_owner(&mut self, _: ()) -> Result<i32, ()> {
            self.events.push("city-owner");
            Ok(0)
        }
    }
    impl simple_requirement::Context for C {
        type Error = ();
        fn active(&mut self) -> Result<bool, ()> {
            Ok(self.r.active)
        }
        fn requirement(&mut self) -> Result<&mut simple_requirement::Requirement, ()> {
            Ok(&mut self.r)
        }
        fn changed(&mut self, _: i32, _: i32) -> Result<(), ()> {
            if self.change_subject {
                self.subject = false;
            }
            Ok(())
        }
    }
    impl simple_requirement::TestContext for C {
        fn test(&mut self) -> Result<i32, ()> {
            test(self)
        }
    }
    impl Context for C {
        fn context_unit_identity(&mut self) -> Result<Option<ComponentId>, ()> {
            Ok(self.context.then_some(ComponentId {
                owner: 0,
                component_type: 1,
                id: 9,
            }))
        }
        fn subject_unit_identity(&mut self) -> Result<Option<ComponentId>, ()> {
            Ok(self.subject.then_some(ComponentId {
                owner: 0,
                component_type: 1,
                id: 9,
            }))
        }
        fn moved_unit_identity(&mut self) -> Result<ComponentId, ()> {
            Ok(ComponentId {
                owner: 0,
                component_type: 1,
                id: 9,
            })
        }
    }
    #[test]
    fn missing_religion_still_queries_city_before_return() {
        let mut c = C::new();
        assert_eq!(test(&mut c), Ok(1));
        assert_eq!(
            c.events,
            [
                "plot-interface",
                "unit-interface",
                "unit-plot",
                "unit-owner",
                "created",
                "gains-founder",
                "city"
            ]
        );
    }
    #[test]
    fn owner_flag_and_two_independent_invalidations() {
        let mut c = C::new();
        c.religion = 2;
        c.city = true;
        c.flag = true;
        assert_eq!(test(&mut c), Ok(0));
        c.flag = false;
        assert_eq!(test(&mut c), Ok(1));
        c.tests = 0;
        c.flag = true;
        on_unit_moved(&mut c).unwrap();
        assert_eq!(c.tests, 2);
        c.tests = 0;
        c.r.state = 1;
        c.change_subject = true;
        on_unit_moved(&mut c).unwrap();
        assert_eq!(c.tests, 1);
    }
}
