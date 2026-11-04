//! Linux improvement activation and movement-damage continuation.
//! Ordinary inert activation and the complete damage dispatch/order are
//! computed here. Camp/removable rewards and reached Kill/discovery effects
//! remain explicit; this is not full Placement or MoveTo support.
use crate::{
    unit::PlotCoord,
    unit_healing::{self, UnitHealingContext, UnitHealingError},
};

pub const ON_IMPROVEMENT_DAMAGE: u32 = 0x7d27db83;
pub const ADJACENT_IMPROVEMENT_DAMAGE: u32 = 0x8634aa0f;
pub const LAST_DAMAGE_PLOT: u32 = 0xa281ca07;

pub trait Context: UnitHealingContext {
    type Map: Copy;
    type Plot: Copy;
    type Diplomacy: Copy;
    type Definition: Copy;
    type AlivePlayers: Copy;
    fn preflight_movement_damage(&self) -> Result<(), Self::Error>;
    fn barbarian_camp_improvement(&mut self) -> Result<u32, Self::Error>;
    fn unit_is_barbarian(&mut self) -> Result<bool, Self::Error>;
    fn clans_active(&mut self) -> Result<bool, Self::Error>;
    fn unit_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error>;
    fn improvement_type(&mut self, plot: Self::Plot) -> Result<u32, Self::Error>;
    fn improvement_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn improvement_pillaged(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn improvement_at(&mut self, kind: u32) -> Result<Self::Definition, Self::Error>;
    fn improvement_find(&mut self, kind: u32) -> Result<Option<Self::Definition>, Self::Error>;
    fn remove_on_entry(&mut self, definition: Self::Definition) -> Result<bool, Self::Error>;
    /// Capture Globals.EditMap, then the current owner's actual Diplomacy.
    fn map_and_diplomacy(&mut self) -> Result<(Self::Map, Self::Diplomacy), Self::Error>;
    fn plot_at(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn plot_location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    fn plot_index(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn at_war(&mut self, diplomacy: Self::Diplomacy, owner: i32) -> Result<bool, Self::Error>;
    /// Actual FindProperty + native TypedVariant integer conversion, not a
    /// guessed SQL default. None is the actual absent property.
    fn property(
        &mut self,
        definition: Self::Definition,
        key: u32,
    ) -> Result<Option<i32>, Self::Error>;
    /// Full HandleImprovementVisUpdates, including reached synchronous effects.
    fn update_improvement_visibility(
        &mut self,
        plot: Self::Plot,
        direction: i32,
    ) -> Result<(), Self::Error>;
    /// Actual Manager.Kill(unit,true,attacker), including all gameplay effects.
    fn kill(&mut self, delayed: bool, attacker: i32) -> Result<(), Self::Error>;
    fn alive_players(&mut self) -> Result<Self::AlivePlayers, Self::Error>;
    fn alive_player_count(&mut self, players: Self::AlivePlayers) -> Result<usize, Self::Error>;
    fn alive_player(
        &mut self,
        players: Self::AlivePlayers,
        index: usize,
    ) -> Result<i32, Self::Error>;
    /// GameSignals+1548 dispatch(unit, improvementOwner, first-pass damage).
    fn signal_improvement_damage(&mut self, owner: i32, amount: i32) -> Result<(), Self::Error>;
    fn set_dynamic_integer(&mut self, key: u32, value: i32) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Context(E),
    Damage(UnitHealingError<E>),
    MissingCurrentPlot,
    InvalidImprovementOwner(i32),
    UnsupportedCampActivation,
    UnsupportedRemovableActivation,
}

/// UnitActivateImprovement's actual nonnull-Unit inert branches. No report,
/// removal or reward is invented for an unsupported camp/removable branch.
pub fn activate_improvement<C: Context>(c: &mut C, plot: C::Plot) -> Result<bool, Error<C::Error>> {
    let camp = c.barbarian_camp_improvement().map_err(Error::Context)?;
    if c.unit_is_barbarian().map_err(Error::Context)? {
        return Ok(false);
    }
    let kind = c.improvement_type(plot).map_err(Error::Context)?;
    if kind == u32::MAX {
        return Ok(false);
    }
    let definition = c.improvement_at(kind).map_err(Error::Context)?;
    let clans = c.clans_active().map_err(Error::Context)?;
    if kind == camp && clans {
        return Err(Error::UnsupportedCampActivation);
    }
    if !c.remove_on_entry(definition).map_err(Error::Context)? {
        return Ok(false);
    }
    Err(Error::UnsupportedRemovableActivation)
}

/// Exact radius-one EnumerateNeighborhood order: center, then the matching
/// Linux host HEXVec_i::TRANSLATIONS array (not Unit direction order).
pub fn neighborhood(center: PlotCoord) -> [PlotCoord; 7] {
    fn half(y: i32) -> i32 {
        if y >= 0 {
            y >> 1
        } else {
            let a = y.wrapping_sub(1);
            a.wrapping_add(((a as u32) >> 31) as i32) >> 1
        }
    }
    let q = center.x.wrapping_sub(half(center.y));
    let offsets = [(0, 0), (0, 1), (-1, 0), (1, -1), (0, -1), (1, 0), (-1, 1)];
    offsets.map(|(x, y)| {
        let row = center.y.wrapping_add(y);
        PlotCoord {
            x: q.wrapping_add(x).wrapping_add(half(row)),
            y: row,
        }
    })
}

fn scan<C: Context>(
    c: &mut C,
    map: C::Map,
    diplomacy: C::Diplomacy,
    center: C::Plot,
    direction: i32,
    total: &mut i32,
    owners: &mut [i32; 64],
) -> Result<(), Error<C::Error>> {
    let location = c.plot_location(center).map_err(Error::Context)?;
    for location in neighborhood(location) {
        let Some(plot) = c.plot_at(map, location).map_err(Error::Context)? else {
            continue;
        };
        let kind = c.improvement_type(plot).map_err(Error::Context)?;
        if kind == u32::MAX {
            continue;
        }
        if c.improvement_owner(plot).map_err(Error::Context)? == -1 {
            continue;
        }
        if c.improvement_pillaged(plot).map_err(Error::Context)? {
            continue;
        }
        let owner = c.improvement_owner(plot).map_err(Error::Context)?;
        if !c.at_war(diplomacy, owner).map_err(Error::Context)? {
            continue;
        }
        let kind = c.improvement_type(plot).map_err(Error::Context)?;
        let Some(definition) = c.improvement_find(kind).map_err(Error::Context)? else {
            continue;
        };
        let own_location = c.plot_location(plot).map_err(Error::Context)?;
        let center_location = c.plot_location(center).map_err(Error::Context)?;
        let key = if own_location == center_location {
            ON_IMPROVEMENT_DAMAGE
        } else {
            ADJACENT_IMPROVEMENT_DAMAGE
        };
        let Some(amount) = c.property(definition, key).map_err(Error::Context)? else {
            continue;
        };
        if amount <= 0 {
            continue;
        }
        *total = total.wrapping_add(amount);
        let owner = c.improvement_owner(plot).map_err(Error::Context)?;
        let index = usize::try_from(owner)
            .ok()
            .filter(|i| *i < owners.len())
            .ok_or(Error::InvalidImprovementOwner(owner))?;
        owners[index] = owners[index].wrapping_add(amount);
        c.update_improvement_visibility(plot, direction)
            .map_err(Error::Context)?;
    }
    Ok(())
}

/// Stock Manager.ChangeDamage: do not skip delta0. IsDead captures current
/// damage BEFORE querying current dynamic maximum and may reach real Kill.
pub fn manager_change_damage<C: Context>(
    c: &mut C,
    delta: i32,
    attacker: i32,
) -> Result<(), Error<C::Error>> {
    unit_healing::change_damage(c, delta).map_err(Error::Damage)?;
    let damage = c.unit().map_err(Error::Context)?.i_damage;
    let maximum = c.maximum_damage_for_report().map_err(Error::Context)?;
    if damage >= maximum {
        c.kill(true, attacker).map_err(Error::Context)?;
    }
    Ok(())
}

/// Rules.Improvements.UnitTakeMovementDamage. Only the FIRST neighborhood sum
/// is applied as damage. Both scans ADD into the returned total; direction−1
/// affects improvement visibility, never subtraction from that sum.
pub fn take_movement_damage<C: Context>(
    c: Option<&mut C>,
    old: PlotCoord,
) -> Result<i32, Error<C::Error>> {
    let Some(c) = c else {
        return Ok(0);
    };
    if c.unit().map_err(Error::Context)?.b_delayed_death {
        return Ok(0);
    }
    if c.unit_plot().map_err(Error::Context)?.is_none() {
        return Ok(0);
    }
    let current = c
        .unit_plot()
        .map_err(Error::Context)?
        .ok_or(Error::MissingCurrentPlot)?;
    let (map, diplomacy) = c.map_and_diplomacy().map_err(Error::Context)?;
    let Some(old) = c.plot_at(map, old).map_err(Error::Context)? else {
        return Ok(0);
    };
    c.preflight_movement_damage().map_err(Error::Context)?;
    let mut total = 0;
    let mut owners = [0_i32; 64];
    scan(c, map, diplomacy, current, 1, &mut total, &mut owners)?;
    manager_change_damage(c, total, -1)?;
    let players = c.alive_players().map_err(Error::Context)?;
    let end = c.alive_player_count(players).map_err(Error::Context)?;
    for index in 0..end {
        let owner = c.alive_player(players, index).map_err(Error::Context)?;
        let slot = usize::try_from(owner)
            .ok()
            .filter(|i| *i < owners.len())
            .ok_or(Error::InvalidImprovementOwner(owner))?;
        if owners[slot] > 0 {
            c.signal_improvement_damage(owner, owners[slot])
                .map_err(Error::Context)?;
        }
    }
    scan(c, map, diplomacy, old, -1, &mut total, &mut owners)?;
    if total != 0 {
        let index = c.plot_index(current).map_err(Error::Context)?;
        c.set_dynamic_integer(LAST_DAMAGE_PLOT, index)
            .map_err(Error::Context)?;
    }
    Ok(total)
}

/// UnitMoved invokes activation before movement damage, on the same actor.
pub fn unit_moved<C: Context>(c: &mut C, old: PlotCoord) -> Result<i32, Error<C::Error>> {
    let plot = c
        .unit_plot()
        .map_err(Error::Context)?
        .ok_or(Error::MissingCurrentPlot)?;
    activate_improvement(c, plot)?;
    take_movement_damage(Some(c), old)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        unit::Unit,
        unit_healing::{DamageChangedReport, HealingRules, HealingWorldState, UnitHealedSignal},
    };
    struct Plot {
        location: PlotCoord,
        owner: i32,
        amount: i32,
    }
    struct Fixture {
        unit: Unit,
        plots: Vec<Plot>,
        alive: Vec<i32>,
        calls: Vec<String>,
        maximum: i32,
        remove: bool,
    }
    const OLD: PlotCoord = PlotCoord { x: 1, y: 1 };
    const NEW: PlotCoord = PlotCoord { x: 4, y: 1 };
    impl Fixture {
        fn new(current: i32, old: i32) -> Self {
            let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
            unit.i_damage = 0;
            unit.b_delayed_death = false;
            unit.k_location = NEW;
            unit.k_operations.nodes.clear();
            Self {
                unit,
                plots: vec![
                    Plot {
                        location: NEW,
                        owner: 1,
                        amount: current,
                    },
                    Plot {
                        location: OLD,
                        owner: 3,
                        amount: old,
                    },
                ],
                alive: vec![3, 1],
                calls: vec![],
                maximum: 100,
                remove: false,
            }
        }
    }
    impl UnitHealingContext for Fixture {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn healing_world_state(&self) -> Result<HealingWorldState, Self::Error> {
            Err("unused heal query")
        }
        fn healing_rules(&self) -> Result<HealingRules, Self::Error> {
            Err("unused heal query")
        }
        fn preflight_healing_effects(&self) -> Result<(), Self::Error> {
            Err("unused heal query")
        }
        fn combat_max_hit_points(&self) -> Result<i32, Self::Error> {
            Ok(100)
        }
        fn maximum_damage_for_report(&self) -> Result<i32, Self::Error> {
            Ok(self.maximum)
        }
        fn send_damage_changed(&mut self, r: DamageChangedReport) -> Result<(), Self::Error> {
            self.calls.push(format!("damage:{}", r.damage));
            Ok(())
        }
        fn clear_current_operations(&mut self) -> Result<(), Self::Error> {
            self.calls.push("clear".into());
            Ok(())
        }
        fn dispatch_unit_healed(&mut self, _: UnitHealedSignal) -> Result<(), Self::Error> {
            panic!("movement never sends healed signal")
        }
    }
    impl Context for Fixture {
        type Map = ();
        type Plot = usize;
        type Diplomacy = ();
        type Definition = usize;
        type AlivePlayers = ();
        fn preflight_movement_damage(&self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn barbarian_camp_improvement(&mut self) -> Result<u32, Self::Error> {
            Ok(99)
        }
        fn unit_is_barbarian(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn clans_active(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn unit_plot(&mut self) -> Result<Option<usize>, Self::Error> {
            Ok(Some(0))
        }
        fn improvement_type(&mut self, p: usize) -> Result<u32, Self::Error> {
            Ok(p as u32)
        }
        fn improvement_owner(&mut self, p: usize) -> Result<i32, Self::Error> {
            Ok(self.plots[p].owner)
        }
        fn improvement_pillaged(&mut self, _: usize) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn improvement_at(&mut self, k: u32) -> Result<usize, Self::Error> {
            Ok(k as usize)
        }
        fn improvement_find(&mut self, k: u32) -> Result<Option<usize>, Self::Error> {
            Ok(Some(k as usize))
        }
        fn remove_on_entry(&mut self, _: usize) -> Result<bool, Self::Error> {
            Ok(self.remove)
        }
        fn map_and_diplomacy(&mut self) -> Result<((), ()), Self::Error> {
            Ok(((), ()))
        }
        fn plot_at(&mut self, _: (), p: PlotCoord) -> Result<Option<usize>, Self::Error> {
            Ok(self.plots.iter().position(|v| v.location == p))
        }
        fn plot_location(&mut self, p: usize) -> Result<PlotCoord, Self::Error> {
            Ok(self.plots[p].location)
        }
        fn plot_index(&mut self, p: usize) -> Result<i32, Self::Error> {
            Ok(p as i32 + 40)
        }
        fn at_war(&mut self, _: (), _: i32) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn property(&mut self, d: usize, _: u32) -> Result<Option<i32>, Self::Error> {
            Ok(Some(self.plots[d].amount))
        }
        fn update_improvement_visibility(&mut self, p: usize, d: i32) -> Result<(), Self::Error> {
            self.calls.push(format!("vis:{p}:{d}"));
            Ok(())
        }
        fn kill(&mut self, d: bool, a: i32) -> Result<(), Self::Error> {
            self.calls.push(format!("kill:{d}:{a}"));
            Ok(())
        }
        fn alive_players(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn alive_player_count(&mut self, _: ()) -> Result<usize, Self::Error> {
            Ok(self.alive.len())
        }
        fn alive_player(&mut self, _: (), i: usize) -> Result<i32, Self::Error> {
            Ok(self.alive[i])
        }
        fn signal_improvement_damage(&mut self, o: i32, n: i32) -> Result<(), Self::Error> {
            self.calls.push(format!("signal:{o}:{n}"));
            Ok(())
        }
        fn set_dynamic_integer(&mut self, k: u32, v: i32) -> Result<(), Self::Error> {
            assert_eq!(k, LAST_DAMAGE_PLOT);
            self.calls.push(format!("property:{v}"));
            Ok(())
        }
    }
    #[test]
    fn current_only_applies_once_then_signals_and_marks_cached_new_plot() {
        let mut c = Fixture::new(7, 0);
        assert_eq!(unit_moved(&mut c, OLD), Ok(7));
        assert_eq!(c.unit.i_damage, 7);
        assert_eq!(
            c.calls,
            vec!["vis:0:1", "damage:7", "signal:1:7", "property:40"]
        );
    }
    #[test]
    fn old_only_adds_returned_total_without_second_damage_or_signal() {
        let mut c = Fixture::new(0, 9);
        assert_eq!(take_movement_damage(Some(&mut c), OLD), Ok(9));
        assert_eq!(c.unit.i_damage, 0);
        assert_eq!(c.calls, vec!["vis:1:-1", "property:40"]);
    }
    #[test]
    fn overlapping_neighborhoods_count_both_but_apply_first_sum_and_signal_alive_order() {
        let mut c = Fixture::new(7, 9);
        c.plots[1].location = PlotCoord { x: 3, y: 1 };
        assert_eq!(
            take_movement_damage(Some(&mut c), PlotCoord { x: 3, y: 1 }),
            Ok(32)
        );
        assert_eq!(c.unit.i_damage, 16);
        assert_eq!(
            c.calls,
            vec![
                "vis:0:1",
                "vis:1:1",
                "damage:16",
                "signal:3:9",
                "signal:1:7",
                "vis:1:-1",
                "vis:0:-1",
                "property:40"
            ]
        );
    }
    #[test]
    fn zero_delta_still_checks_dead_and_kills_and_unsupported_activation_is_inert() {
        let mut c = Fixture::new(0, 0);
        c.maximum = 0;
        assert_eq!(take_movement_damage(Some(&mut c), OLD), Ok(0));
        assert_eq!(c.calls, vec!["kill:true:-1"]);
        c.calls.clear();
        c.remove = true;
        assert_eq!(
            unit_moved(&mut c, OLD),
            Err(Error::UnsupportedRemovableActivation)
        );
        assert!(c.calls.is_empty());
    }
    #[test]
    fn native_center_then_host_translation_order_is_not_direction_order() {
        assert_eq!(
            neighborhood(PlotCoord { x: 4, y: 1 }),
            [
                PlotCoord { x: 4, y: 1 },
                PlotCoord { x: 5, y: 2 },
                PlotCoord { x: 3, y: 1 },
                PlotCoord { x: 5, y: 0 },
                PlotCoord { x: 4, y: 0 },
                PlotCoord { x: 5, y: 1 },
                PlotCoord { x: 4, y: 2 }
            ]
        );
    }
}
