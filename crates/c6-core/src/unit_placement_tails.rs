//! Linux movement rules following Placement's sight/improvement callbacks.
//! These leaves compute source-order guards, field changes and iteration. Live
//! map/rules providers and synchronous effects remain required; this is not a
//! complete Placement or MoveTo implementation.
use crate::{
    diplomacy_meeting::{self, Context as MeetingContext},
    skip_turn::placement::coordinate_valid,
    unit::{ComponentId, PlotCoord, Unit},
    visibility_contents::{self, UnitViewContext},
};

pub const SPREAD_CHARGES_CHANGED: u32 = 0xb2176dc4;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpreadChargesReport {
    pub event: u32,
    pub identity: ComponentId,
    pub current: i32,
    pub previous: i32,
}
pub trait BonusContext {
    type Error;
    type Map: Copy;
    type Plot: Copy;
    fn map(&mut self) -> Result<Self::Map, Self::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    fn adjacent(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
        direction: i32,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn natural_wonder(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn preflight_spread_report(&self) -> Result<(), Self::Error>;
    fn spread_report(&mut self, report: SpreadChargesReport) -> Result<(), Self::Error>;
}
/// AwardPostMoveBonuses 0x2bef410 and Religion::ChangeSpreadCharges 0x2c44090.
/// The Unit-owned Religion has its real nonnull parent. A zero delta still
/// assigns the wrapping sum but sends no report. After a subscriber, deferred
/// charges are changed on the current Religion and then reread for the next
/// wonder. The six-direction scan continues even when that reread becomes zero.
pub fn award_post_move_bonuses<C: BonusContext>(
    c: &mut C,
    destination: PlotCoord,
) -> Result<(), C::Error> {
    let map = c.map()?;
    let mut charges = c.unit()?.k_religion.i_nw_deferred_charges;
    if charges <= 0 {
        return Ok(());
    }
    for direction in 0..6 {
        let Some(plot) = c.adjacent(map, destination, direction)? else {
            continue;
        };
        if !c.natural_wonder(plot)? {
            continue;
        }
        if charges != 0 {
            c.preflight_spread_report()?;
        }
        let previous = c.unit()?.k_religion.i_spread_charges;
        c.unit_mut()?.k_religion.i_spread_charges = previous.wrapping_add(charges);
        if charges != 0 {
            let unit = c.unit()?;
            let report = SpreadChargesReport {
                event: SPREAD_CHARGES_CHANGED,
                identity: ComponentId {
                    owner: unit.e_owner as i16,
                    component_type: 1,
                    id: unit.id,
                },
                current: unit.k_religion.i_spread_charges,
                previous,
            };
            c.spread_report(report)?;
        }
        let current = c.unit()?.k_religion.i_nw_deferred_charges;
        c.unit_mut()?.k_religion.i_nw_deferred_charges = current.wrapping_sub(charges);
        charges = c.unit()?.k_religion.i_nw_deferred_charges;
    }
    Ok(())
}

/// Native Hex::EnumerateNeighborhood 0x2a44850, including center and each
/// ring's corners/edges. The callback maps coordinates through the retained
/// Map, preserving wrapped duplicates rather than deduplicating plots.
pub fn neighborhood<E>(
    center: PlotCoord,
    radius: u32,
    mut visit: impl FnMut(PlotCoord) -> Result<(), E>,
) -> Result<(), E> {
    fn half(y: i32) -> i32 {
        if y >= 0 {
            y >> 1
        } else {
            let a = y.wrapping_sub(1);
            a.wrapping_add(((a as u32) >> 31) as i32) >> 1
        }
    }
    let q = center.x.wrapping_sub(half(center.y));
    let translations: [(i32, i32); 6] = [(0, 1), (-1, 0), (1, -1), (0, -1), (1, 0), (-1, 1)];
    let edges = [1, 2, 0, 4, 5, 3];
    let location = |x: i32, y: i32| {
        let y = center.y.wrapping_add(y);
        PlotCoord {
            x: q.wrapping_add(x).wrapping_add(half(y)),
            y,
        }
    };
    visit(center)?;
    for ring in 1..=radius {
        let ring = ring as i32;
        for (direction, &(x, y)) in translations.iter().enumerate() {
            let (x, y) = (x.wrapping_mul(ring), y.wrapping_mul(ring));
            visit(location(x, y))?;
            let (dx, dy) = translations[edges[direction]];
            for offset in 1..ring {
                visit(location(
                    x.wrapping_add(dx.wrapping_mul(offset)),
                    y.wrapping_add(dy.wrapping_mul(offset)),
                ))?;
            }
        }
    }
    Ok(())
}

pub trait SiegeContext {
    type Error;
    type Map: Copy;
    type Plot: Copy;
    type District: Copy;
    fn map(&mut self) -> Result<Self::Map, Self::Error>;
    fn plot_at(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn distance(&mut self, map: Self::Map, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error>;
    fn owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn is_city(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    /// Current CityAttributes::GetDistrict and ComponentID::IsValid. None is
    /// actual invalid identity; a missing runtime record is an error.
    fn district(&mut self, plot: Self::Plot) -> Result<Option<Self::District>, Self::Error>;
    fn is_besieged(&mut self, district: Self::District) -> Result<bool, Self::Error>;
    fn set_siege_status(
        &mut self,
        district: Self::District,
        value: bool,
    ) -> Result<(), Self::Error>;
}
/// Rules.Combat.CheckForSiegeNear 0x2be1ab0 / lambda 0x2be1ff0. IsBesieged is
/// a computed district query, not the stored siege flag being written back.
pub fn check_for_siege_near<C: SiegeContext>(
    c: &mut C,
    center: PlotCoord,
    owner: i32,
) -> Result<(), C::Error> {
    let map = c.map()?;
    neighborhood(center, 3, |candidate| {
        let Some(plot) = c.plot_at(map, candidate)? else {
            return Ok(());
        };
        if c.location(plot)? == center {
            return Ok(());
        }
        let location = c.location(plot)?;
        if c.distance(map, center, location)? > 3
            || c.owner(plot)? == -1
            || !c.is_city(plot)?
            || c.owner(plot)? == owner
        {
            return Ok(());
        }
        if let Some(district) = c.district(plot)? {
            let value = c.is_besieged(district)?;
            c.set_siege_status(district, value)?;
        }
        Ok(())
    })
}

/// Actual retained GetAlive vector receiver and cached end. Entries must be
/// reread through this token after callbacks; copying IDs would change native
/// semantics. Replacing/reallocating retained storage must fail explicitly.
pub trait ContactContext: MeetingContext {
    type AlivePlayers: Copy;
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error>;
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error>;
    /// Normal shared group-one visibility at the copied coordinate.
    fn plot_visible(&mut self, viewer: i32, location: PlotCoord) -> Result<bool, Self::Error>;
    fn current_unit_plot(&mut self, unit: Self::Unit) -> Result<Option<Self::Plot>, Self::Error>;
}
/// Rules.Diplomacy.UnitMoved 0x2be2980. The old-coordinate argument is unused.
/// Current location, owner and moving-player human status are cached before
/// iteration; each reached Meet receives the unit's newly reread current plot.
pub fn diplomacy_unit_moved<C: ContactContext>(
    c: &mut C,
    unit: C::Unit,
) -> Result<(), diplomacy_meeting::Error<C::Error>> {
    use diplomacy_meeting::Error::Context;
    let location = c.unit(unit).map_err(Context)?.k_location;
    if !coordinate_valid(location) {
        return Ok(());
    }
    let owner = c.unit(unit).map_err(Context)?.e_owner;
    let player = c.player(owner).map_err(Context)?;
    let human = c.human(player).map_err(Context)?;
    let (players, end) = c.alive_players().map_err(Context)?;
    for index in 0..end {
        let player = c.alive_player(players, index).map_err(Context)?;
        if c.id(player).map_err(Context)? == owner {
            continue;
        }
        let other_human = c.human(player).map_err(Context)?;
        if human && !other_human {
            continue;
        }
        let viewer = c.id(player).map_err(Context)?;
        if !c.plot_visible(viewer, location).map_err(Context)? {
            continue;
        }
        let other = c.id(player).map_err(Context)?;
        if c.relations()
            .map_err(Context)?
            .has_met(other, owner)
            .map_err(diplomacy_meeting::Error::Relations)?
        {
            continue;
        }
        let other = c.id(player).map_err(Context)?;
        let plot = c.current_unit_plot(unit).map_err(Context)?;
        diplomacy_meeting::meet(
            c,
            other,
            owner,
            plot,
            ComponentId {
                owner: -1,
                component_type: 0,
                id: u32::MAX,
            },
        )?;
    }
    Ok(())
}

pub trait UnitsMovedContext: UnitViewContext {
    type AlivePlayers: Copy;
    type Player: Copy;
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error>;
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error>;
    fn current_player_id(&mut self, player: Self::Player) -> Result<i32, Self::Error>;
    fn current_player_barbarian(&mut self, player: Self::Player) -> Result<bool, Self::Error>;
    fn unit_location(&mut self, unit: Self::Unit) -> Result<PlotCoord, Self::Error>;
    fn plot_visible(&mut self, viewer: i32, location: PlotCoord) -> Result<bool, Self::Error>;
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<Self::Plot>, Self::Error>;
    fn barbarian_camp(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    /// Real ClansManager.OnForeignUnitMovedNearCamp, including gameplay/RNG.
    fn foreign_unit_moved_near_camp(
        &mut self,
        unit: Self::Unit,
        plot: Self::Plot,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum UnitsMovedError<E> {
    Context(E),
    View(visibility_contents::UnitViewError<E>),
}
/// Rules.Units.UnitMoved 0x2bf4a70 / camp lambda 0x2bf53f0. Clan checks happen
/// independently of visibility, after the basic two-argument ViewUnit effects.
pub fn units_unit_moved<C: UnitsMovedContext>(
    c: &mut C,
    unit: C::Unit,
) -> Result<(), UnitsMovedError<C::Error>> {
    use UnitsMovedError::Context;
    let location = c.unit_location(unit).map_err(Context)?;
    if !coordinate_valid(location) {
        return Ok(());
    }
    let owner = c.unit_owner(unit).map_err(Context)?;
    let (players, end) = c.alive_players().map_err(Context)?;
    for index in 0..end {
        let player = c.alive_player(players, index).map_err(Context)?;
        let viewer = c.current_player_id(player).map_err(Context)?;
        if viewer == owner {
            continue;
        }
        let viewer = c.current_player_id(player).map_err(Context)?;
        if c.plot_visible(viewer, location).map_err(Context)? {
            let viewer = c.current_player_id(player).map_err(Context)?;
            visibility_contents::rules_view_unit_basic(c, viewer, unit)
                .map_err(UnitsMovedError::View)?;
        }
        if c.current_player_barbarian(player).map_err(Context)?
            && c.clans_active().map_err(Context)?
        {
            let current = c.unit_location(unit).map_err(Context)?;
            neighborhood(current, 1, |location| {
                if let Some(plot) = c.plot_at(location).map_err(Context)? {
                    if c.barbarian_camp(plot).map_err(Context)? {
                        c.foreign_unit_moved_near_camp(unit, plot)
                            .map_err(Context)?;
                    }
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}

pub const BARBARIAN_APPROACH: u32 = 0x2a43004b;
pub trait BarbarianApproachContext {
    type Error;
    type Map: Copy;
    type AlivePlayers: Copy;
    type Player: Copy;
    type Cities: Copy;
    type Cursor: Copy;
    type City: Copy;
    fn unit_is_barbarian(&mut self) -> Result<bool, Self::Error>;
    fn current_turn(&mut self) -> Result<i32, Self::Error>;
    fn map(&mut self) -> Result<Self::Map, Self::Error>;
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error>;
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error>;
    fn cities_begin(
        &mut self,
        player: Self::Player,
    ) -> Result<(Self::Cities, Self::Cursor), Self::Error>;
    /// Refresh Cities.End and compare the retained cursor before resolving it.
    fn city_at(
        &mut self,
        cities: Self::Cities,
        cursor: Self::Cursor,
    ) -> Result<Option<Self::City>, Self::Error>;
    fn advance_city(
        &mut self,
        cities: Self::Cities,
        cursor: &mut Self::Cursor,
    ) -> Result<(), Self::Error>;
    fn city_location(&mut self, city: Self::City) -> Result<PlotCoord, Self::Error>;
    fn distance(&mut self, map: Self::Map, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error>;
    fn last_invasion(&mut self, city: Self::City) -> Result<i32, Self::Error>;
    fn set_last_invasion(&mut self, city: Self::City, turn: i32) -> Result<(), Self::Error>;
    /// Actual Reporting.Notification.Send(0x2a43004b,current city owner,new)
    /// including reachable subscribers/RNG. Localized strings carry this city's
    /// current name; they are not policy decisions or invented city rewards.
    fn notify_approach(&mut self, city: Self::City, location: PlotCoord)
        -> Result<(), Self::Error>;
}
/// NotifyBarbarianApproach 0x2bef4b0. The ordinary-unit guard precedes all turn,
/// registry and city access. A barbarian caches turn/map/alive end, refreshes
/// the city cursor after each callback and writes the cooldown before Send.
pub fn notify_barbarian_approach<C: BarbarianApproachContext>(
    c: &mut C,
    location: PlotCoord,
) -> Result<(), C::Error> {
    if !c.unit_is_barbarian()? {
        return Ok(());
    }
    let turn = c.current_turn()?;
    let map = c.map()?;
    let (players, end) = c.alive_players()?;
    for index in 0..end {
        let player = c.alive_player(players, index)?;
        let (cities, mut cursor) = c.cities_begin(player)?;
        while let Some(city) = c.city_at(cities, cursor)? {
            let current_location = c.city_location(city)?;
            if c.distance(map, location, current_location)? <= 2 {
                let first = c.last_invasion(city)?;
                if first == -1 || turn >= c.last_invasion(city)?.wrapping_add(10) {
                    c.set_last_invasion(city, turn)?;
                    c.notify_approach(city, location)?;
                }
            }
            c.advance_city(cities, &mut cursor)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Bonus {
        unit: Unit,
        visited: Vec<i32>,
        reports: Vec<SpreadChargesReport>,
        fail: bool,
        change_deferred: bool,
    }
    impl BonusContext for Bonus {
        type Error = &'static str;
        type Map = ();
        type Plot = i32;
        fn map(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn adjacent(
            &mut self,
            _: (),
            _: PlotCoord,
            direction: i32,
        ) -> Result<Option<i32>, Self::Error> {
            self.visited.push(direction);
            Ok(Some(direction))
        }
        fn natural_wonder(&mut self, plot: i32) -> Result<bool, Self::Error> {
            Ok(plot < 2)
        }
        fn preflight_spread_report(&self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn spread_report(&mut self, report: SpreadChargesReport) -> Result<(), Self::Error> {
            assert_eq!(self.unit.k_religion.i_spread_charges, report.current);
            self.reports.push(report);
            if self.fail {
                return Err("subscriber");
            }
            if self.change_deferred && self.reports.len() == 1 {
                self.unit.k_religion.i_nw_deferred_charges = 5;
            }
            Ok(())
        }
    }
    fn bonus(charges: i32) -> Bonus {
        let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
        unit.k_religion.i_nw_deferred_charges = charges;
        unit.k_religion.i_spread_charges = i32::MAX;
        Bonus {
            unit,
            visited: vec![],
            reports: vec![],
            fail: false,
            change_deferred: false,
        }
    }
    #[test]
    fn bonus_rereads_after_callback_and_continues_six_directions() {
        let mut c = bonus(3);
        c.change_deferred = true;
        award_post_move_bonuses(&mut c, PlotCoord { x: 0, y: 0 }).unwrap();
        assert_eq!(c.visited, [0, 1, 2, 3, 4, 5]);
        assert_eq!(
            c.reports
                .iter()
                .map(|r| (r.previous, r.current))
                .collect::<Vec<_>>(),
            [(i32::MAX, i32::MIN + 2), (i32::MIN + 2, i32::MIN + 4)]
        );
        assert_eq!(c.unit.k_religion.i_nw_deferred_charges, 0);
    }
    #[test]
    fn zero_after_first_wonder_still_scans_but_does_not_send_zero_delta() {
        let mut c = bonus(1);
        award_post_move_bonuses(&mut c, PlotCoord { x: 0, y: 0 }).unwrap();
        assert_eq!(c.visited.len(), 6);
        assert_eq!(c.reports.len(), 1);
        assert_eq!(c.unit.k_religion.i_nw_deferred_charges, 0);
        for charges in [0, -1] {
            let mut c = bonus(charges);
            award_post_move_bonuses(&mut c, PlotCoord { x: 0, y: 0 }).unwrap();
            assert!(c.visited.is_empty());
        }
    }
    #[test]
    fn bonus_report_failure_preserves_charge_write_before_deferred_debit() {
        let mut c = bonus(3);
        c.fail = true;
        assert_eq!(
            award_post_move_bonuses(&mut c, PlotCoord { x: 0, y: 0 }),
            Err("subscriber")
        );
        assert_eq!(c.unit.k_religion.i_spread_charges, i32::MIN + 2);
        assert_eq!(c.unit.k_religion.i_nw_deferred_charges, 3);
        assert_eq!(c.visited, [0]);
    }
    #[test]
    fn neighborhood_native_order_includes_center_and_all_rings() {
        let center = PlotCoord { x: 20, y: 21 };
        let mut seen = vec![];
        neighborhood(center, 3, |p| {
            seen.push(p);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(seen.len(), 37);
        assert_eq!(
            &seen[..7],
            &crate::improvement_movement::neighborhood(center)
        );
        let mut unique = seen.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 37);
        assert_eq!(seen[7], PlotCoord { x: 21, y: 23 });
        assert_eq!(seen[8], PlotCoord { x: 20, y: 23 });
    }
}
