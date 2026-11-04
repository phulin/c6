//! Linux city founding/territory state transitions. All current getters and
//! synchronous effects are ordered; no RNG draw or callback is assumed absent.
use crate::unit::PlotCoord;
pub mod plot_yields;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CityId {
    pub owner: i32,
    pub id: u32,
}
impl CityId {
    pub const NONE: Self = Self {
        owner: -1,
        id: u32::MAX,
    };
    pub fn is_valid(self) -> bool {
        self.owner != -1 && self.id != u32::MAX
    }
}
// Fixed host HEXVec_i::TRANSLATIONS4678910; EnumerateRing2a44b30 uses
// this order, which is distinct from clockwise Map adjacency order.
pub fn ring_one(center: PlotCoord) -> [PlotCoord; 6] {
    fn half(y: i32) -> i32 {
        if y >= 0 {
            y >> 1
        } else {
            let n = y.wrapping_sub(1);
            y.wrapping_add(((n as u32) >> 31) as i32).wrapping_sub(1) >> 1
        }
    }
    let q = center.x.wrapping_sub(half(center.y));
    [(0_i32, 1_i32), (-1, 0), (1, -1), (0, -1), (1, 0), (-1, 1)].map(|(x, y)| {
        let row = center.y.wrapping_add(y);
        PlotCoord {
            x: q.wrapping_add(x).wrapping_add(half(row)),
            y: row,
        }
    })
}
pub trait StartingContext {
    type Error;
    fn city_present(&mut self) -> Result<bool, Self::Error>;
    fn owner(&mut self) -> Result<i32, Self::Error>;
    fn level_starting_tiles(&mut self, owner: i32) -> Result<Option<i32>, Self::Error>;
    fn modified_city_tiles(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn is_major(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn era_starting_tiles(&mut self) -> Result<i32, Self::Error>;
}
pub fn starting_plots<C: StartingContext>(c: &mut C) -> Result<i32, C::Error> {
    if !c.city_present()? {
        return Ok(0);
    }
    let owner = c.owner()?;
    if owner == -1 {
        return Ok(0);
    }
    let base = c.level_starting_tiles(owner)?.unwrap_or(0);
    let mut count = base.wrapping_add(c.modified_city_tiles(owner)?);
    if c.is_major(owner)? {
        count = count.wrapping_add(c.era_starting_tiles()?);
    }
    Ok(count)
}
pub trait AcquireContext {
    type Error;
    type Plot: Copy;
    fn city_owner(&mut self) -> Result<i32, Self::Error>;
    fn city_plot_location(&mut self) -> Result<PlotCoord, Self::Error>;
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<Self::Plot>, Self::Error>;
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn district_type(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn plot_location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn annex_plot(&mut self, location: PlotCoord) -> Result<(), Self::Error>;
    fn next_buyable_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error>;
}
pub fn acquire_plots<C: AcquireContext>(
    c: &mut C,
    mut count: i32,
    initial_ring: bool,
) -> Result<(), C::Error> {
    let owner = c.city_owner()?;
    if initial_ring {
        let center = c.city_plot_location()?;
        for location in ring_one(center) {
            let Some(plot) = c.plot_at(location)? else {
                continue;
            };
            // Native callback re-reads GetOwner for its second comparison.
            if c.plot_owner(plot)? == -1
                || (c.plot_owner(plot)? == owner && c.district_type(plot)? == -1)
            {
                let location = c.plot_location(plot)?;
                c.annex_plot(location)?;
                count = count.wrapping_sub(1);
            }
        }
    }
    let mut n = 0_i32;
    while n < count {
        if let Some(plot) = c.next_buyable_plot()? {
            let location = c.plot_location(plot)?;
            c.annex_plot(location)?;
        }
        n = n.wrapping_add(1);
    }
    Ok(())
}

pub trait OwnerContext {
    type Error;
    type Plot: Copy;
    fn valid_player(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn owning_city(&mut self, plot: Self::Plot) -> Result<CityId, Self::Error>;
    fn set_owning_city(&mut self, plot: Self::Plot, city: CityId) -> Result<(), Self::Error>;
    fn remove_city_attributes(&mut self, plot: Self::Plot) -> Result<(), Self::Error>;
    fn adjacent_plot(
        &mut self,
        plot: Self::Plot,
        direction: u8,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn change_owner_visibility(
        &mut self,
        owner: i32,
        plot: Self::Plot,
        delta: i32,
    ) -> Result<(), Self::Error>;
    fn report_owner_changed(&mut self, plot: Self::Plot, old_owner: i32)
        -> Result<(), Self::Error>;
    fn signal_owner_changed(
        &mut self,
        plot: Self::Plot,
        new_city: CityId,
        old_city: CityId,
    ) -> Result<(), Self::Error>;
}
fn visibility<C: OwnerContext>(
    c: &mut C,
    owner: i32,
    plot: C::Plot,
    delta: i32,
) -> Result<(), C::Error> {
    c.change_owner_visibility(owner, plot, delta)?;
    for direction in 0..6 {
        if let Some(adjacent) = c.adjacent_plot(plot, direction)? {
            c.change_owner_visibility(owner, adjacent, delta)?;
        }
    }
    Ok(())
}
pub fn set_owner<C: OwnerContext>(
    c: &mut C,
    plot: C::Plot,
    requested_owner: i32,
    city: u32,
) -> Result<(), C::Error> {
    let owner = if requested_owner != -1 && c.valid_player(requested_owner)? {
        requested_owner
    } else {
        -1
    };
    if c.plot_owner(plot)? == owner {
        if owner == -1 || c.owning_city(plot)?.id == city {
            return Ok(());
        }
        let old = c.owning_city(plot)?;
        let new = CityId { owner, id: city };
        c.set_owning_city(plot, new)?;
        c.report_owner_changed(plot, owner)?;
        return c.signal_owner_changed(plot, new, old);
    }
    let old_owner = c.plot_owner(plot)?;
    let old = if old_owner != -1 {
        let id = c.owning_city(plot)?;
        visibility(c, old_owner, plot, -1)?;
        id
    } else {
        CityId::NONE
    };
    let new = if owner != -1 {
        CityId { owner, id: city }
    } else {
        CityId::NONE
    };
    if owner != -1 {
        c.set_owning_city(plot, new)?;
    } else {
        c.remove_city_attributes(plot)?;
    }
    c.report_owner_changed(plot, old_owner)?;
    c.signal_owner_changed(plot, new, old)?;
    if owner != -1 {
        visibility(c, owner, plot, 1)?;
    }
    Ok(())
}

pub trait PurchasedContext: OwnerContext {
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<Self::Plot>, Self::Error>;
    fn city_exists(&mut self, city: CityId) -> Result<bool, Self::Error>;
    fn add_plot_yields(
        &mut self,
        plot: Self::Plot,
        city: CityId,
        improvement: i32,
    ) -> Result<u32, Self::Error>;
    fn plot_yield_effect(&mut self, plot: Self::Plot) -> Result<u32, Self::Error>;
    fn remove_plot_yields(&mut self, effect: u32) -> Result<(), Self::Error>;
    fn set_plot_yield_effect(&mut self, plot: Self::Plot, effect: u32) -> Result<(), Self::Error>;
    fn set_valid_domains(&mut self, plot: Self::Plot) -> Result<(), Self::Error>;
}
pub fn set_purchased_by_city<C: PurchasedContext>(
    c: &mut C,
    location: PlotCoord,
    owner: i32,
    id: u32,
) -> Result<(), C::Error> {
    let Some(plot) = c.plot_at(location)? else {
        return Ok(());
    };
    set_owner(c, plot, owner, id)?;
    if !c.valid_player(owner)? {
        return Ok(());
    }
    let city = CityId { owner, id };
    if !c.city_exists(city)? {
        return Ok(());
    }
    let added = c.add_plot_yields(plot, city, -1)?;
    // Read the old effect AFTER Add/ObjectCreated callbacks.
    let old = c.plot_yield_effect(plot)?;
    if old != 0 {
        c.remove_plot_yields(old)?;
    }
    c.set_plot_yield_effect(plot, added)?;
    c.set_valid_domains(plot)
}

pub trait AnnexContext: PurchasedContext {
    fn current_city(&mut self) -> Result<CityId, Self::Error>;
    fn set_working_city(&mut self, location: PlotCoord, city: CityId) -> Result<(), Self::Error>;
    fn improvement_type(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn improvement_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn set_improvement(
        &mut self,
        plot: Self::Plot,
        kind: i32,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn capturable_improvement(&mut self, kind: i32) -> Result<Option<bool>, Self::Error>;
    fn unit_count(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn expel_from_plot(&mut self, location: PlotCoord) -> Result<(), Self::Error>;
    fn activate_improvement_without_unit(&mut self, plot: Self::Plot) -> Result<(), Self::Error>;
    fn acquired(&mut self, city: CityId, delta: i32, reset_next: bool) -> Result<(), Self::Error>;
    fn reallocate_citizens(&mut self, city: CityId) -> Result<(), Self::Error>;
    fn report_annexed(&mut self, city: CityId, location: PlotCoord) -> Result<(), Self::Error>;
    fn signal_annexed(
        &mut self,
        owner: i32,
        city: CityId,
        plot: Self::Plot,
        old_city: CityId,
    ) -> Result<(), Self::Error>;
}
pub fn annex_plot<C: AnnexContext>(c: &mut C, location: PlotCoord) -> Result<(), C::Error> {
    let Some(plot) = c.plot_at(location)? else {
        return Ok(());
    };
    let old = c.owning_city(plot)?;
    let city = c.current_city()?;
    set_purchased_by_city(c, location, city.owner, city.id)?;
    let city = c.current_city()?;
    c.set_working_city(location, city)?;
    let improvement = c.improvement_type(plot)?;
    let owner = c.current_city()?.owner;
    if improvement != -1 && c.improvement_owner(plot)? != owner {
        c.set_improvement(plot, -1, -1)?;
        if c.capturable_improvement(improvement)? == Some(true) {
            c.set_improvement(plot, improvement, owner)?;
        }
    }
    if c.unit_count(plot)? > 0 {
        c.expel_from_plot(location)?;
    }
    c.activate_improvement_without_unit(plot)?;
    let city = c.current_city()?;
    c.acquired(city, 1, true)?;
    c.reallocate_citizens(city)?;
    let city = c.current_city()?;
    c.report_annexed(city, location)?;
    let city = c.current_city()?;
    c.signal_annexed(owner, city, plot, old)?;
    if old.is_valid() && c.city_exists(old)? {
        c.acquired(old, -1, false)?;
        c.reallocate_citizens(old)?;
        c.report_annexed(old, location)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ring_preserves_native_translation_order() {
        assert_eq!(
            ring_one(PlotCoord { x: 18, y: 11 }),
            [(19, 12), (17, 11), (19, 10), (18, 10), (19, 11), (18, 12)]
                .map(|(x, y)| PlotCoord { x, y })
        );
        assert_eq!(
            ring_one(PlotCoord { x: 0, y: -1 }),
            [(1, 0), (-1, -1), (1, -2), (0, -2), (1, -1), (0, 0)].map(|(x, y)| PlotCoord { x, y })
        );
    }
    struct Acquire {
        owners: [i32; 6],
        districts: [i32; 6],
        seen: Vec<u32>,
        next: usize,
        extra: usize,
    }
    impl AcquireContext for Acquire {
        type Error = ();
        type Plot = u32;
        fn city_owner(&mut self) -> Result<i32, ()> {
            Ok(4)
        }
        fn city_plot_location(&mut self) -> Result<PlotCoord, ()> {
            Ok(PlotCoord { x: 18, y: 11 })
        }
        fn plot_at(&mut self, p: PlotCoord) -> Result<Option<u32>, ()> {
            Ok(ring_one(PlotCoord { x: 18, y: 11 })
                .iter()
                .position(|v| *v == p)
                .map(|v| v as u32))
        }
        fn plot_owner(&mut self, p: u32) -> Result<i32, ()> {
            Ok(self.owners[p as usize])
        }
        fn district_type(&mut self, p: u32) -> Result<i32, ()> {
            Ok(self.districts[p as usize])
        }
        fn plot_location(&mut self, p: u32) -> Result<PlotCoord, ()> {
            Ok(PlotCoord { x: p as i32, y: 0 })
        }
        fn annex_plot(&mut self, p: PlotCoord) -> Result<(), ()> {
            self.seen.push(p.x as u32);
            Ok(())
        }
        fn next_buyable_plot(&mut self) -> Result<Option<u32>, ()> {
            self.next += 1;
            Ok(if self.next <= self.extra {
                Some(10)
            } else {
                None
            })
        }
    }
    #[test]
    fn ring_eligibility_is_independent_of_requested_count_and_extra_nulls_consume_iterations() {
        let mut c = Acquire {
            owners: [-1, 4, 4, 5, -1, 4],
            districts: [8, -1, 0, -1, -1, -1],
            seen: vec![],
            next: 0,
            extra: 0,
        };
        acquire_plots(&mut c, 0, true).unwrap();
        assert_eq!(c.seen, [0, 1, 4, 5]);
        assert_eq!(c.next, 0);
        c.seen.clear();
        acquire_plots(&mut c, 3, false).unwrap();
        assert_eq!(c.next, 3);
        assert!(c.seen.is_empty());
    }
    struct Owner {
        owner: i32,
        city: u32,
        events: Vec<(i32, i32)>,
    }
    impl OwnerContext for Owner {
        type Error = ();
        type Plot = u32;
        fn valid_player(&mut self, p: i32) -> Result<bool, ()> {
            Ok((0..8).contains(&p))
        }
        fn plot_owner(&mut self, _: u32) -> Result<i32, ()> {
            Ok(self.owner)
        }
        fn owning_city(&mut self, _: u32) -> Result<CityId, ()> {
            Ok(CityId {
                owner: self.owner,
                id: self.city,
            })
        }
        fn set_owning_city(&mut self, _: u32, c: CityId) -> Result<(), ()> {
            self.owner = c.owner;
            self.city = c.id;
            self.events.push((0, c.owner));
            Ok(())
        }
        fn remove_city_attributes(&mut self, _: u32) -> Result<(), ()> {
            self.owner = -1;
            self.city = u32::MAX;
            self.events.push((0, -1));
            Ok(())
        }
        fn adjacent_plot(&mut self, _: u32, d: u8) -> Result<Option<u32>, ()> {
            Ok(if d == 2 { Some(8) } else { None })
        }
        fn change_owner_visibility(&mut self, p: i32, _: u32, delta: i32) -> Result<(), ()> {
            self.events.push((delta, p));
            Ok(())
        }
        fn report_owner_changed(&mut self, _: u32, old: i32) -> Result<(), ()> {
            self.events.push((2, old));
            Ok(())
        }
        fn signal_owner_changed(&mut self, _: u32, new: CityId, old: CityId) -> Result<(), ()> {
            self.events.push((3, new.owner));
            self.events.push((4, old.owner));
            Ok(())
        }
    }
    #[test]
    fn ownership_removes_old_visibility_before_write_and_adds_new_after_signal() {
        let mut c = Owner {
            owner: 1,
            city: 8,
            events: vec![],
        };
        set_owner(&mut c, 0, 2, 9).unwrap();
        assert_eq!(
            c.events,
            [
                (-1, 1),
                (-1, 1),
                (0, 2),
                (2, 1),
                (3, 2),
                (4, 1),
                (1, 2),
                (1, 2)
            ]
        );
        c.events.clear();
        set_owner(&mut c, 0, 2, 9).unwrap();
        assert!(c.events.is_empty());
        set_owner(&mut c, 0, 2, 10).unwrap();
        assert_eq!(c.events, [(0, 2), (2, 2), (3, 2), (4, 2)]);
        c.events.clear();
        set_owner(&mut c, 0, 99, 10).unwrap();
        assert_eq!(
            c.events,
            [(-1, 2), (-1, 2), (0, -1), (2, 2), (3, -1), (4, 2)]
        );
    }
}
