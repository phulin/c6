//! Linux District.IsBesieged / SetSiegeStatus shared by movement and combat.
use crate::{
    skip_turn::placement::coordinate_valid,
    unit::{ComponentId, PlotCoord},
};

pub const SIEGE_STATUS_CHANGED: u32 = 0xfbada2c8;
pub const AUTO_SIEGE_ABILITY: &str = "ABILITY_AUTO_SIEGE";

pub trait Context {
    type Error;
    type Map: Copy;
    type Player: Copy;
    type City: Copy;
    type District: Copy;
    type Plot: Copy;
    type Unit: Copy;
    type UnitContainer: Copy;
    type UnitList: Copy;
    fn map(&mut self) -> Result<Self::Map, Self::Error>;
    fn owner(&mut self, district: Self::District) -> Result<i32, Self::Error>;
    fn player(&mut self, owner: i32) -> Result<Self::Player, Self::Error>;
    fn city(&mut self, district: Self::District) -> Result<Option<Self::City>, Self::Error>;
    fn siege_protection(&mut self, city: Self::City) -> Result<bool, Self::Error>;
    fn location(&mut self, district: Self::District) -> Result<PlotCoord, Self::Error>;
    /// Computed Combat.Manager.GetBestDefender(coord,-1,owner,null,1).
    fn best_defender(
        &mut self,
        location: PlotCoord,
        first_player: i32,
        second_player: i32,
        flags: u32,
    ) -> Result<Option<Self::Unit>, Self::Error>;
    fn unit_container(&mut self, map: Self::Map) -> Result<Self::UnitContainer, Self::Error>;
    /// Native FindIndex(name), including its real -1 not-found value.
    fn ability_index(&mut self, name: &str) -> Result<i32, Self::Error>;
    fn adjacent(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
        direction: u32,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn plot_location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn impassable(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn units(
        &mut self,
        container: Self::UnitContainer,
        location: PlotCoord,
        layer: u32,
    ) -> Result<Self::UnitList, Self::Error>;
    fn unit_count(&mut self, list: Self::UnitList) -> Result<usize, Self::Error>;
    fn unit_id(&mut self, list: Self::UnitList, index: usize) -> Result<ComponentId, Self::Error>;
    fn find_unit(
        &mut self,
        player: Self::Player,
        id: u32,
    ) -> Result<Option<Self::Unit>, Self::Error>;
    fn has_ability(&mut self, unit: Self::Unit, ability: i32) -> Result<bool, Self::Error>;
    fn player_id(&mut self, player: Self::Player) -> Result<i32, Self::Error>;
    fn at_war(&mut self, player: Self::Player, other: i32) -> Result<bool, Self::Error>;
    fn enemy_or_zoc(&mut self, owner: i32, location: PlotCoord) -> Result<bool, Self::Error>;
}

/// 0x2920240. An open neighboring plot clears the accumulated siege result;
/// later AUTO_SIEGE units must still be scanned and can return true.
pub fn is_besieged<C: Context>(c: &mut C, district: C::District) -> Result<bool, C::Error> {
    let map = c.map()?;
    let owner = c.owner(district)?;
    let player = c.player(owner)?;
    if let Some(city) = c.city(district)? {
        if c.siege_protection(city)? {
            return Ok(false);
        }
    }
    let location = c.location(district)?;
    let owner = c.owner(district)?;
    if c.best_defender(location, -1, owner, 1)?.is_some() {
        return Ok(true);
    }
    let container = c.unit_container(map)?;
    let ability = c.ability_index(AUTO_SIEGE_ABILITY)?;
    let mut besieged = true;
    for direction in 0..6 {
        let location = c.location(district)?;
        let Some(plot) = c.adjacent(map, location, direction)? else {
            continue;
        };
        if !coordinate_valid(c.plot_location(plot)?) || c.impassable(plot)? {
            continue;
        }
        let location = c.plot_location(plot)?;
        let units = c.units(container, location, 0)?;
        let mut index = 0;
        while index < c.unit_count(units)? {
            let id = c.unit_id(units, index)?;
            index += 1;
            // ComponentID.IsValid does not test its component_type tag.
            if id.owner == -1 || id.id == u32::MAX {
                continue;
            }
            let unit_player = c.player(i32::from(id.owner))?;
            let Some(unit) = c.find_unit(unit_player, id.id)? else {
                continue;
            };
            if c.has_ability(unit, ability)? {
                let unit_owner = c.player_id(unit_player)?;
                if c.at_war(player, unit_owner)? {
                    return Ok(true);
                }
            }
        }
        let owner = c.owner(district)?;
        let location = c.plot_location(plot)?;
        if !c.enemy_or_zoc(owner, location)? {
            besieged = false;
        }
    }
    Ok(besieged)
}

pub trait StatusContext {
    type Error;
    type District: Copy;
    type City: Copy;
    fn siege_status(&self, district: Self::District) -> Result<bool, Self::Error>;
    fn store_siege_status(
        &mut self,
        district: Self::District,
        value: bool,
    ) -> Result<(), Self::Error>;
    fn city(&mut self, district: Self::District) -> Result<Self::City, Self::Error>;
    fn city_identity(&mut self, city: Self::City) -> Result<ComponentId, Self::Error>;
    fn report(&mut self, event: u32, city: ComponentId, value: i32) -> Result<(), Self::Error>;
}

/// 0x2920480. The real stored flag is assigned before the parent-city lookup;
/// the report receives its freshly reread value, not the caller's argument.
pub fn set_siege_status<C: StatusContext>(
    c: &mut C,
    district: C::District,
    value: bool,
) -> Result<(), C::Error> {
    if c.siege_status(district)? == value {
        return Ok(());
    }
    c.store_siege_status(district, value)?;
    let city = c.city(district)?;
    let identity = c.city_identity(city)?;
    let current = c.siege_status(district)?;
    c.report(SIEGE_STATUS_CHANGED, identity, i32::from(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        protection: bool,
        defender: bool,
        auto_siege: Option<u32>,
        threatened: bool,
        directions: Vec<u32>,
        ability_queries: usize,
        state: bool,
        reads: Vec<&'static str>,
        reports: Vec<(u32, ComponentId, i32)>,
        change_during_identity: bool,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                protection: false,
                defender: false,
                auto_siege: None,
                threatened: false,
                directions: vec![],
                ability_queries: 0,
                state: false,
                reads: vec![],
                reports: vec![],
                change_during_identity: false,
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type Map = ();
        type Player = i32;
        type City = ();
        type District = ();
        type Plot = u32;
        type Unit = u32;
        type UnitContainer = ();
        type UnitList = u32;
        fn map(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn owner(&mut self, _: ()) -> Result<i32, Self::Error> {
            Ok(3)
        }
        fn player(&mut self, owner: i32) -> Result<i32, Self::Error> {
            Ok(owner)
        }
        fn city(&mut self, _: ()) -> Result<Option<()>, Self::Error> {
            Ok(Some(()))
        }
        fn siege_protection(&mut self, _: ()) -> Result<bool, Self::Error> {
            Ok(self.protection)
        }
        fn location(&mut self, _: ()) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: 10, y: 10 })
        }
        fn best_defender(
            &mut self,
            p: PlotCoord,
            a: i32,
            b: i32,
            flags: u32,
        ) -> Result<Option<u32>, Self::Error> {
            assert_eq!((p, a, b, flags), (PlotCoord { x: 10, y: 10 }, -1, 3, 1));
            self.reads.push("defender");
            Ok(self.defender.then_some(77))
        }
        fn unit_container(&mut self, _: ()) -> Result<(), Self::Error> {
            Ok(())
        }
        fn ability_index(&mut self, name: &str) -> Result<i32, Self::Error> {
            assert_eq!(name, AUTO_SIEGE_ABILITY);
            self.ability_queries += 1;
            Ok(7)
        }
        fn adjacent(&mut self, _: (), _: PlotCoord, d: u32) -> Result<Option<u32>, Self::Error> {
            self.directions.push(d);
            Ok(Some(d))
        }
        fn plot_location(&mut self, p: u32) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: p as i32, y: 0 })
        }
        fn impassable(&mut self, _: u32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn units(&mut self, _: (), p: PlotCoord, layer: u32) -> Result<u32, Self::Error> {
            assert_eq!(layer, 0);
            Ok(p.x as u32)
        }
        fn unit_count(&mut self, p: u32) -> Result<usize, Self::Error> {
            Ok(usize::from(self.auto_siege == Some(p)))
        }
        fn unit_id(&mut self, _: u32, index: usize) -> Result<ComponentId, Self::Error> {
            assert_eq!(index, 0);
            Ok(ComponentId {
                owner: 8,
                component_type: 99,
                id: 55,
            })
        }
        fn find_unit(&mut self, player: i32, id: u32) -> Result<Option<u32>, Self::Error> {
            assert_eq!((player, id), (8, 55));
            Ok(Some(id))
        }
        fn has_ability(&mut self, unit: u32, ability: i32) -> Result<bool, Self::Error> {
            assert_eq!((unit, ability), (55, 7));
            Ok(true)
        }
        fn player_id(&mut self, player: i32) -> Result<i32, Self::Error> {
            Ok(player)
        }
        fn at_war(&mut self, player: i32, other: i32) -> Result<bool, Self::Error> {
            assert_eq!((player, other), (3, 8));
            Ok(true)
        }
        fn enemy_or_zoc(&mut self, owner: i32, _: PlotCoord) -> Result<bool, Self::Error> {
            assert_eq!(owner, 3);
            Ok(self.threatened)
        }
    }
    impl StatusContext for Fixture {
        type Error = &'static str;
        type District = ();
        type City = ();
        fn siege_status(&self, _: ()) -> Result<bool, Self::Error> {
            Ok(self.state)
        }
        fn store_siege_status(&mut self, _: (), v: bool) -> Result<(), Self::Error> {
            self.reads.push("store");
            self.state = v;
            Ok(())
        }
        fn city(&mut self, _: ()) -> Result<(), Self::Error> {
            self.reads.push("city");
            Ok(())
        }
        fn city_identity(&mut self, _: ()) -> Result<ComponentId, Self::Error> {
            self.reads.push("identity");
            if self.change_during_identity {
                self.state = false;
            }
            Ok(ComponentId {
                owner: 3,
                component_type: 0,
                id: 44,
            })
        }
        fn report(&mut self, e: u32, c: ComponentId, v: i32) -> Result<(), Self::Error> {
            self.reports.push((e, c, v));
            Ok(())
        }
    }
    #[test]
    fn protection_and_defender_are_ordered_early_returns() {
        let mut f = Fixture::new();
        f.protection = true;
        f.defender = true;
        assert!(!is_besieged(&mut f, ()).unwrap());
        assert!(f.reads.is_empty());
        assert_eq!(f.ability_queries, 0);
        f.protection = false;
        assert!(is_besieged(&mut f, ()).unwrap());
        assert_eq!(f.reads, vec!["defender"]);
        assert_eq!(f.ability_queries, 0);
    }
    #[test]
    fn later_auto_siege_overrides_earlier_open_neighbor() {
        let mut f = Fixture::new();
        f.auto_siege = Some(5);
        assert!(is_besieged(&mut f, ()).unwrap());
        assert_eq!(f.directions, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(f.ability_queries, 1);
        let mut f = Fixture::new();
        assert!(!is_besieged(&mut f, ()).unwrap());
        assert_eq!(f.directions.len(), 6);
        f.threatened = true;
        assert!(is_besieged(&mut f, ()).unwrap());
    }
    #[test]
    fn status_noop_skips_city_and_changed_report_reads_current_flag() {
        let mut f = Fixture::new();
        set_siege_status(&mut f, (), false).unwrap();
        assert!(f.reads.is_empty());
        assert!(f.reports.is_empty());
        f.change_during_identity = true;
        set_siege_status(&mut f, (), true).unwrap();
        assert_eq!(f.reads, vec!["store", "city", "identity"]);
        assert_eq!(
            f.reports,
            vec![(
                SIEGE_STATUS_CHANGED,
                ComponentId {
                    owner: 3,
                    component_type: 0,
                    id: 44
                },
                0
            )]
        );
    }
}
