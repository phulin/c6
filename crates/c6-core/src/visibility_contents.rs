//! Linux ViewPlotContents discovery and diplomacy-view ordering.
//!
//! This executes typed discovery mutations. Contact/Meet, reporting, signals,
//! unit-view rules and relic creation are required gameplay providers; they may
//! not be dropped or replaced by native AI choices. Their full effects and RNG
//! must be available before the enclosing visibility transition is admitted.
use crate::unit::{ComponentId, PlotCoord};
use crate::visibility_change::ViewContents;

/// Runtime enum-indexed player discovery flags, with explicit complete arrays.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovery {
    pub continents: Vec<bool>,
    pub features: Vec<bool>,
    pub circumnavigated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signal<U> {
    PlayerContinent {
        player: i32,
        continent: i32,
    },
    UnitContinent {
        unit: U,
        continent: i32,
    },
    PlayerWonder {
        player: i32,
        feature: i32,
        plot: PlotCoord,
    },
    UnitWonder {
        unit: U,
        feature: i32,
        plot: PlotCoord,
    },
    Circumnavigated {
        player: i32,
        plot: PlotCoord,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Notification {
    Continent(i32),
    Goody,
    NaturalWonder(i32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    MissingPlot(u32),
}

pub trait Context {
    type Error;
    type Plot: Copy;
    type Unit: Copy;
    type UnitList: Copy;
    type Capital: Copy;
    fn preflight_contents(&self, event: ViewContents) -> Result<(), Self::Error>;
    fn map_present(&mut self) -> Result<bool, Self::Error>;
    fn plot(&mut self, index: i32) -> Result<Option<Self::Plot>, Self::Error>;
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn continent(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn goody_notify(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn natural_wonder(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn player_valid(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn human(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn is_revealed(&mut self, player: i32, plot: u32) -> Result<bool, Self::Error>;
    fn discovery(&self, player: i32) -> Result<&Discovery, Self::Error>;
    fn discovery_mut(&mut self, player: i32) -> Result<&mut Discovery, Self::Error>;
    fn found_continent(&self, player: i32, index: i32) -> Result<bool, Self::Error> {
        Ok(self
            .discovery(player)?
            .continents
            .get(index as usize)
            .copied()
            .unwrap_or(false))
    }
    fn set_found_continent(&mut self, player: i32, index: i32) -> Result<(), Self::Error> {
        if let Some(flag) = self
            .discovery_mut(player)?
            .continents
            .get_mut(index as usize)
        {
            *flag = true;
        }
        Ok(())
    }
    fn found_feature(&self, player: i32, index: i32) -> Result<bool, Self::Error> {
        Ok(self
            .discovery(player)?
            .features
            .get(index as usize)
            .copied()
            .unwrap_or(false))
    }
    fn set_found_feature(&mut self, player: i32, index: i32) -> Result<(), Self::Error> {
        if let Some(flag) = self.discovery_mut(player)?.features.get_mut(index as usize) {
            *flag = true;
        }
        Ok(())
    }
    fn circumnavigated(&self, player: i32) -> Result<bool, Self::Error> {
        Ok(self.discovery(player)?.circumnavigated)
    }
    fn set_circumnavigated(&mut self, player: i32) -> Result<(), Self::Error> {
        self.discovery_mut(player)?.circumnavigated = true;
        Ok(())
    }

    fn find_unit(&mut self, reference: ComponentId) -> Result<Option<Self::Unit>, Self::Error>;
    fn unit_owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn unit_layer_count(&mut self) -> Result<i32, Self::Error>;
    fn unit_list(&mut self, plot: Self::Plot, layer: i32) -> Result<Self::UnitList, Self::Error>;
    fn unit_list_count(&mut self, list: Self::UnitList) -> Result<usize, Self::Error>;
    fn unit_reference(
        &mut self,
        list: Self::UnitList,
        index: usize,
    ) -> Result<ComponentId, Self::Error>;
    /// Complete Player::Manager::Meet, including flags, movement completion,
    /// callback order, rewards/RNG and external-policy decision boundaries.
    fn meet(
        &mut self,
        first: i32,
        second: i32,
        plot: Option<Self::Plot>,
        source: ComponentId,
    ) -> Result<(), Self::Error>;
    fn view_unit_rules(
        &mut self,
        player: i32,
        unit: Self::Unit,
        origin: i32,
        first_reveal: bool,
    ) -> Result<(), Self::Error>;
    fn signal(&mut self, event: Signal<Self::Unit>) -> Result<(), Self::Error>;
    /// Full Reporting::Notification::Send and any reachable gameplay handlers.
    fn notify(
        &mut self,
        player: i32,
        plot: Self::Plot,
        kind: Notification,
    ) -> Result<(), Self::Error>;
    fn natural_wonder_relic_count(&mut self, player: i32) -> Result<i32, Self::Error>;
    fn capital(&mut self, player: i32) -> Result<Option<Self::Capital>, Self::Error>;
    fn capital_location(&mut self, city: Self::Capital) -> Result<PlotCoord, Self::Error>;
    /// Exact Religion::CreateRelic with original RNG streams/draw ordering.
    fn create_relic(
        &mut self,
        player: i32,
        location: PlotCoord,
        source: u32,
    ) -> Result<(), Self::Error>;
    fn grid_width(&mut self) -> Result<i32, Self::Error>;
    fn grid_height(&mut self) -> Result<i32, Self::Error>;
    fn revealed_xy(&mut self, player: i32, x: i32, y: i32) -> Result<bool, Self::Error>;
}

/// Rules::Diplomacy::ViewPlot. First-reveal bool is unused by this leaf.
/// Human landowners initiate the meeting only when the origin is revealed to
/// them (or negative); nonhuman landowners are the second participant.
pub fn diplomacy_view_plot<C: Context>(
    c: &mut C,
    event: ViewContents,
    plot: Option<C::Plot>,
) -> Result<(), C::Error> {
    let Some(plot) = plot else {
        return Ok(());
    };
    if event.revealed_state == 0 {
        return Ok(());
    }
    let owner = c.plot_owner(plot)?;
    let valid = c.player_valid(owner)?;
    if owner == event.player || !valid {
        return Ok(());
    }
    if c.human(owner)? {
        if event.origin >= 0 && !c.is_revealed(owner, event.origin as u32)? {
            return Ok(());
        }
        c.meet(owner, event.player, Some(plot), event.source)?;
    } else {
        c.meet(event.player, owner, Some(plot), event.source)?;
    }
    Ok(())
}

/// Rules::Diplomacy::ViewUnit: human owners with nonnegative origins meet at
/// the origin plot. With a negative origin they meet at the viewed plot with a
/// default ComponentID (owner=-1, component_type=0, id=UINT_MAX).
pub fn diplomacy_view_unit<C: Context>(
    c: &mut C,
    event: ViewContents,
    unit: C::Unit,
    plot: C::Plot,
) -> Result<(), C::Error> {
    let owner = c.unit_owner(unit)?;
    if c.human(owner)? {
        if event.origin >= 0 {
            if !c.is_revealed(owner, event.origin as u32)? {
                return Ok(());
            }
            let origin_plot = c.plot(event.origin)?;
            c.meet(owner, event.player, origin_plot, event.source)?;
        } else {
            c.meet(
                owner,
                event.player,
                Some(plot),
                ComponentId {
                    owner: -1,
                    component_type: 0,
                    id: u32::MAX,
                },
            )?;
        }
    } else {
        c.meet(event.player, owner, Some(plot), event.source)?;
    }
    Ok(())
}

/// Concrete Linux ViewPlotContents. Retains the source unit and capital handles
/// across callbacks, re-reads mutable flags/reward counts at native call sites,
/// and dispatches all synchronous effects in original order.
pub fn view_plot_contents<C: Context>(
    c: &mut C,
    event: ViewContents,
) -> Result<(), Error<C::Error>> {
    use Error::Context;
    c.preflight_contents(event).map_err(Context)?;
    if !c.map_present().map_err(Context)? {
        return Ok(());
    }
    // Native map+0x48 must return a valid plot for this admitted transition.
    let plot = c.plot(event.plot as i32).map_err(Context)?;
    let plot = plot.ok_or(Error::MissingPlot(event.plot))?;
    c.plot_owner(plot).map_err(Context)?;
    let source_unit = c.find_unit(event.source).map_err(Context)?;
    diplomacy_view_plot(c, event, Some(plot)).map_err(Context)?;
    if event.revealed_state == 2 {
        let mut layer = 0;
        while layer < c.unit_layer_count().map_err(Context)? {
            let list = c.unit_list(plot, layer).map_err(Context)?;
            let mut index = 0;
            while index < c.unit_list_count(list).map_err(Context)? {
                let reference = c.unit_reference(list, index).map_err(Context)?;
                let owner_id = c.player_id(i32::from(reference.owner)).map_err(Context)?;
                if owner_id != event.player {
                    if let Some(unit) = c.find_unit(reference).map_err(Context)? {
                        diplomacy_view_unit(c, event, unit, plot).map_err(Context)?;
                        c.view_unit_rules(event.player, unit, event.origin, event.first_reveal)
                            .map_err(Context)?;
                    }
                }
                index += 1;
            }
            layer += 1;
        }
        let continent = c.continent(plot).map_err(Context)?;
        if continent != -1 {
            // Native getters return false outside the captured complete array;
            // setters ignore those indices. That does not suppress signals.
            let found = c
                .found_continent(event.player, continent)
                .map_err(Context)?;
            if !found {
                c.set_found_continent(event.player, continent)
                    .map_err(Context)?;
                c.signal(Signal::PlayerContinent {
                    player: event.player,
                    continent,
                })
                .map_err(Context)?;
                if let Some(unit) = source_unit {
                    let continent = c.continent(plot).map_err(Context)?;
                    c.signal(Signal::UnitContinent { unit, continent })
                        .map_err(Context)?;
                }
                c.notify(event.player, plot, Notification::Continent(continent))
                    .map_err(Context)?;
            }
        }
    } else {
        c.continent(plot).map_err(Context)?;
    }
    let goody = c.goody_notify(plot).map_err(Context)?;
    let first_nonhidden = event.revealed_state != 0 && event.first_reveal;
    if first_nonhidden && goody {
        c.notify(event.player, plot, Notification::Goody)
            .map_err(Context)?;
    }
    let wonder = c.natural_wonder(plot).map_err(Context)?;
    if first_nonhidden && wonder {
        let feature = c.feature(plot).map_err(Context)?;
        let known = c.found_feature(event.player, feature).map_err(Context)?;
        if !known {
            let feature = c.feature(plot).map_err(Context)?;
            c.set_found_feature(event.player, feature)
                .map_err(Context)?;
            let feature = c.feature(plot).map_err(Context)?;
            let location = c.location(plot).map_err(Context)?;
            c.signal(Signal::PlayerWonder {
                player: event.player,
                feature,
                plot: location,
            })
            .map_err(Context)?;
            if let Some(unit) = source_unit {
                let feature = c.feature(plot).map_err(Context)?;
                let location = c.location(plot).map_err(Context)?;
                c.signal(Signal::UnitWonder {
                    unit,
                    feature,
                    plot: location,
                })
                .map_err(Context)?;
            }
            let feature = c.feature(plot).map_err(Context)?;
            c.notify(event.player, plot, Notification::NaturalWonder(feature))
                .map_err(Context)?;
            if c.natural_wonder_relic_count(event.player)
                .map_err(Context)?
                > 0
            {
                if let Some(capital) = c.capital(event.player).map_err(Context)? {
                    let mut index = 0;
                    while index
                        < c.natural_wonder_relic_count(event.player)
                            .map_err(Context)?
                    {
                        let location = c.capital_location(capital).map_err(Context)?;
                        c.create_relic(event.player, location, 0x28c7_ddf5)
                            .map_err(Context)?;
                        index += 1;
                    }
                }
            }
        }
    }
    let circumnavigated = c.circumnavigated(event.player).map_err(Context)?;
    if event.revealed_state == 2 && event.first_reveal && !circumnavigated {
        let mut x = 0;
        while x < c.grid_width().map_err(Context)? {
            if c.grid_height().map_err(Context)? <= 0 {
                return Ok(());
            }
            let mut y = 0;
            while !c.revealed_xy(event.player, x, y).map_err(Context)? {
                y += 1;
                if y >= c.grid_height().map_err(Context)? {
                    return Ok(());
                }
            }
            x += 1;
        }
        c.set_circumnavigated(event.player).map_err(Context)?;
        let location = c.location(plot).map_err(Context)?;
        c.signal(Signal::Circumnavigated {
            player: event.player,
            plot: location,
        })
        .map_err(Context)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        discovery: Discovery,
        log: Vec<String>,
        owner: i32,
        human: bool,
        origin_revealed: bool,
        wonder: bool,
        relic_count: i32,
        columns: Vec<bool>,
        encountered: Vec<bool>,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                discovery: Discovery {
                    continents: vec![false],
                    features: vec![false],
                    circumnavigated: false,
                },
                log: vec![],
                owner: -1,
                human: false,
                origin_revealed: false,
                wonder: true,
                relic_count: 2,
                columns: vec![true, true],
                encountered: vec![false],
            }
        }
    }
    fn event() -> ViewContents {
        ViewContents {
            player: 0,
            plot: 0,
            origin: 1,
            first_reveal: true,
            revealed_state: 2,
            source: ComponentId {
                owner: 0,
                component_type: 0,
                id: 4,
            },
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type Plot = i32;
        type Unit = u32;
        type UnitList = ();
        type Capital = ();
        fn preflight_contents(&self, _: ViewContents) -> Result<(), Self::Error> {
            Ok(())
        }
        fn map_present(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn plot(&mut self, i: i32) -> Result<Option<i32>, Self::Error> {
            Ok(Some(i))
        }
        fn plot_owner(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(self.owner)
        }
        fn location(&mut self, p: i32) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: p, y: 0 })
        }
        fn continent(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn feature(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn goody_notify(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn natural_wonder(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(self.wonder)
        }
        fn player_valid(&mut self, p: i32) -> Result<bool, Self::Error> {
            Ok(p >= 0)
        }
        fn human(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(self.human)
        }
        fn is_revealed(&mut self, _: i32, _: u32) -> Result<bool, Self::Error> {
            Ok(self.origin_revealed)
        }
        fn discovery(&self, _: i32) -> Result<&Discovery, Self::Error> {
            Ok(&self.discovery)
        }
        fn discovery_mut(&mut self, _: i32) -> Result<&mut Discovery, Self::Error> {
            Ok(&mut self.discovery)
        }
        fn find_unit(&mut self, r: ComponentId) -> Result<Option<u32>, Self::Error> {
            Ok(Some(r.id))
        }
        fn unit_owner(&mut self, _: u32) -> Result<i32, Self::Error> {
            Ok(1)
        }
        fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error> {
            Ok(owner)
        }
        fn unit_layer_count(&mut self) -> Result<i32, Self::Error> {
            Ok(1)
        }
        fn unit_list(&mut self, _: i32, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn unit_list_count(&mut self, _: ()) -> Result<usize, Self::Error> {
            Ok(1)
        }
        fn unit_reference(&mut self, _: (), _: usize) -> Result<ComponentId, Self::Error> {
            Ok(ComponentId {
                owner: 1,
                component_type: 0,
                id: 8,
            })
        }
        fn meet(
            &mut self,
            first: i32,
            second: i32,
            plot: Option<i32>,
            source: ComponentId,
        ) -> Result<(), Self::Error> {
            self.log.push(format!(
                "meet {first} {second} {plot:?} {} {} {}",
                source.owner, source.component_type, source.id
            ));
            Ok(())
        }
        fn view_unit_rules(
            &mut self,
            _: i32,
            unit: u32,
            _: i32,
            _: bool,
        ) -> Result<(), Self::Error> {
            self.log.push(format!("view unit {unit}"));
            Ok(())
        }
        fn signal(&mut self, e: Signal<u32>) -> Result<(), Self::Error> {
            self.log.push(format!("signal {e:?}"));
            Ok(())
        }
        fn notify(&mut self, _: i32, _: i32, n: Notification) -> Result<(), Self::Error> {
            self.log.push(format!("notify {n:?}"));
            Ok(())
        }
        fn natural_wonder_relic_count(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(self.relic_count)
        }
        fn capital(&mut self, _: i32) -> Result<Option<()>, Self::Error> {
            Ok(Some(()))
        }
        fn capital_location(&mut self, _: ()) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: 9, y: 0 })
        }
        fn create_relic(
            &mut self,
            _: i32,
            location: PlotCoord,
            source: u32,
        ) -> Result<(), Self::Error> {
            assert_eq!(source, 0x28c7_ddf5);
            assert_eq!(location.x, 9);
            self.log.push("relic".into());
            // Synchronous effects can change the reward count: native re-reads.
            self.relic_count = 1;
            Ok(())
        }
        fn grid_width(&mut self) -> Result<i32, Self::Error> {
            Ok(self.columns.len() as i32)
        }
        fn grid_height(&mut self) -> Result<i32, Self::Error> {
            Ok(1)
        }
        fn revealed_xy(&mut self, _: i32, x: i32, _: i32) -> Result<bool, Self::Error> {
            Ok(self.columns[x as usize])
        }
    }
    impl UnitViewContext for Fixture {
        fn unit_type(&mut self, _: u32) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn encountered_units(&self, _: i32) -> Result<&[bool], Self::Error> {
            Ok(&self.encountered)
        }
        fn encountered_units_mut(&mut self, _: i32) -> Result<&mut [bool], Self::Error> {
            Ok(&mut self.encountered)
        }
        fn player_barbarian(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn clans_active(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn signal_encountered_unit(&mut self, _: i32, _: i32) -> Result<(), Self::Error> {
            assert!(self.encountered[0]);
            self.log.push("encountered signal".into());
            Ok(())
        }
        fn clan_view_foreign_unit(&mut self, _: u32, plot: i32) -> Result<(), Self::Error> {
            self.log.push(format!("clan view {plot}"));
            Ok(())
        }
    }
    #[test]
    fn unit_rules_write_encounter_flag_before_signal_and_clan_callback() {
        let mut f = Fixture::new();
        rules_view_unit(&mut f, 0, 8, 5).unwrap();
        assert_eq!(f.encountered, [true]);
        assert_eq!(f.log, ["encountered signal", "clan view 5"]);
        f.log.clear();
        rules_view_unit(&mut f, 0, 8, 5).unwrap();
        assert_eq!(f.log, ["clan view 5"]);
    }
    #[test]
    fn basic_view_overload_does_not_run_clan_effects() {
        let mut f = Fixture::new();
        rules_view_unit_basic(&mut f, 0, 8).unwrap();
        assert_eq!(f.encountered, [true]);
        assert_eq!(f.log, ["encountered signal"]);
        f.log.clear();
        rules_view_unit_basic(&mut f, 0, 8).unwrap();
        assert!(f.log.is_empty());
    }
    #[test]
    fn missing_unit_type_flag_fails_without_clan_callback_or_flag_mutation() {
        let mut f = Fixture::new();
        f.encountered.clear();
        assert_eq!(
            rules_view_unit(&mut f, 0, 8, 5),
            Err(UnitViewError::MissingUnitType(0))
        );
        assert!(f.log.is_empty());
    }

    #[test]
    fn discoveries_contact_rewards_and_signals_run_in_native_order() {
        let mut f = Fixture::new();
        view_plot_contents(&mut f, event()).unwrap();
        assert_eq!(
            f.log,
            [
                "meet 0 1 Some(0) 0 0 4",
                "view unit 8",
                "signal PlayerContinent { player: 0, continent: 0 }",
                "signal UnitContinent { unit: 4, continent: 0 }",
                "notify Continent(0)",
                "notify Goody",
                "signal PlayerWonder { player: 0, feature: 0, plot: PlotCoord { x: 0, y: 0 } }",
                "signal UnitWonder { unit: 4, feature: 0, plot: PlotCoord { x: 0, y: 0 } }",
                "notify NaturalWonder(0)",
                "relic",
                "signal Circumnavigated { player: 0, plot: PlotCoord { x: 0, y: 0 } }",
            ]
        );
        assert!(f.discovery.continents[0]);
        assert!(f.discovery.features[0]);
        assert!(f.discovery.circumnavigated);
    }
    #[test]
    fn revealed_only_has_wonder_effects_but_no_continent_or_foreign_unit_contact() {
        let mut f = Fixture::new();
        let mut e = event();
        e.revealed_state = 1;
        view_plot_contents(&mut f, e).unwrap();
        assert!(!f.discovery.continents[0]);
        assert!(f.discovery.features[0]);
        assert!(!f.discovery.circumnavigated);
        assert_eq!(f.log.first().unwrap(), "notify Goody");
        assert_eq!(f.log.last().unwrap(), "relic");
    }
    #[test]
    fn human_plot_owner_must_have_origin_revealed_and_initiates_meet() {
        let mut f = Fixture::new();
        f.owner = 1;
        f.human = true;
        diplomacy_view_plot(&mut f, event(), Some(5)).unwrap();
        assert!(f.log.is_empty());
        f.origin_revealed = true;
        diplomacy_view_plot(&mut f, event(), Some(5)).unwrap();
        assert_eq!(f.log, ["meet 1 0 Some(5) 0 0 4"]);
    }
    #[test]
    fn negative_origin_human_unit_contact_uses_default_source_identity() {
        let mut f = Fixture::new();
        f.human = true;
        let mut e = event();
        e.origin = -1;
        diplomacy_view_unit(&mut f, e, 8, 5).unwrap();
        assert_eq!(f.log, ["meet 1 0 Some(5) -1 0 4294967295"]);
    }
    #[test]
    fn circumnavigation_requires_one_revealed_cell_in_every_column() {
        let mut f = Fixture::new();
        f.columns[1] = false;
        view_plot_contents(&mut f, event()).unwrap();
        assert!(!f.discovery.circumnavigated);
        assert!(!f.log.iter().any(|s| s.contains("Circumnavigated")));
    }
    #[test]
    fn native_out_of_range_discovery_flags_do_not_suppress_effects() {
        let mut f = Fixture::new();
        f.discovery.continents.clear();
        f.discovery.features.clear();
        view_plot_contents(&mut f, event()).unwrap();
        assert!(f.discovery.continents.is_empty());
        assert!(f.discovery.features.is_empty());
        assert!(f.log.iter().any(|s| s == "notify Continent(0)"));
        assert!(f.log.iter().any(|s| s == "relic"));
    }
}

/// Required live state/effects for the concrete Rules.Units::ViewUnit leaves.
pub trait UnitViewContext: Context {
    fn unit_type(&mut self, unit: Self::Unit) -> Result<i32, Self::Error>;
    fn encountered_units(&self, player: i32) -> Result<&[bool], Self::Error>;
    fn encountered_units_mut(&mut self, player: i32) -> Result<&mut [bool], Self::Error>;
    fn player_barbarian(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn clans_active(&mut self) -> Result<bool, Self::Error>;
    /// Exact GameSignals+0x5e8 dispatch, after the encountered flag write.
    fn signal_encountered_unit(&mut self, player: i32, unit_type: i32) -> Result<(), Self::Error>;
    /// Complete ClansManager::OnViewForeignUnit gameplay/RNG effects.
    fn clan_view_foreign_unit(
        &mut self,
        unit: Self::Unit,
        origin: Self::Plot,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitViewError<E> {
    Context(E),
    MissingUnitType(i32),
}

/// Two-argument Rules.Units::ViewUnit (Linux 0x2bf4940).
/// This overload only updates encounter state and dispatches its signal.
/// Native unit-type flag indexing is unguarded; invalid exported arrays fail
/// explicitly here instead of inventing an unseen/encountered value.
pub fn rules_view_unit_basic<C: UnitViewContext>(
    c: &mut C,
    player: i32,
    unit: C::Unit,
) -> Result<(), UnitViewError<C::Error>> {
    use UnitViewError::Context;
    if c.player_valid(player).map_err(Context)? {
        let unit_type = c.unit_type(unit).map_err(Context)?;
        let encountered = *c
            .encountered_units(player)
            .map_err(Context)?
            .get(unit_type as usize)
            .ok_or(UnitViewError::MissingUnitType(unit_type))?;
        if !encountered {
            let flag = c
                .encountered_units_mut(player)
                .map_err(Context)?
                .get_mut(unit_type as usize)
                .ok_or(UnitViewError::MissingUnitType(unit_type))?;
            *flag = true;
            c.signal_encountered_unit(player, unit_type)
                .map_err(Context)?;
        }
    }
    Ok(())
}

/// Four-argument ViewUnit (Linux 0x2bf49d0). The first-reveal bool is unused.
/// Its barbarian-clan continuation follows the basic encounter operation.
pub fn rules_view_unit<C: UnitViewContext>(
    c: &mut C,
    player: i32,
    unit: C::Unit,
    origin: i32,
) -> Result<(), UnitViewError<C::Error>> {
    use UnitViewError::Context;
    rules_view_unit_basic(c, player, unit)?;
    if c.unit_owner(unit).map_err(Context)? != player
        && c.player_barbarian(player).map_err(Context)?
        && c.clans_active().map_err(Context)?
    {
        if let Some(plot) = c.plot(origin).map_err(Context)? {
            c.clan_view_foreign_unit(unit, plot).map_err(Context)?;
        }
    }
    Ok(())
}
