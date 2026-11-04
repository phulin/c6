//! Ordered Linux Placement::Set map-membership transition (0x2c4160b..2c4180a).
//!
//! Consumes a destination already selected by the original validation branch.
//! This is a shared part of placement, not complete Placement::Set or MoveTo.
//! The returned continuation records the native cached values required by its
//! remaining visibility, reporting, embarkation, contact and rules callbacks.
use crate::map::Plot;
use crate::unit::{ComponentId, PlotCoord, Unit};
use crate::unit_occupancy::{self, OccupancyError};
use crate::unit_turn_upkeep::ACTIVITY_HOLD;
use crate::world::World;

pub const EMBARK_EVENT: u32 = 0x0fbb4f61;
pub const DISEMBARK_EVENT: u32 = 0x23c8f020;

pub trait PlacementMembershipContext {
    type Error;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    /// The retained actual Map/UnitContainer receiver, not a copy of occupancy.
    fn world_mut(&mut self) -> Result<&mut World, Self::Error>;
    /// Pure support check for all reached queries and continuations before the
    /// first membership write, including later full Placement effects. Contexts
    /// cannot claim a successful gameplay move by running only this component.
    fn preflight_membership(&self, destination: PlotCoord) -> Result<(), Self::Error>;
    fn local_plot_visible(&mut self, old: PlotCoord) -> Result<bool, Self::Error>;
    fn local_unit_visible(&mut self) -> Result<bool, Self::Error>;
    /// Actual Map GetPlot mapping for the retained receiver; unknown is Err.
    fn plot_mut(&mut self, location: PlotCoord) -> Result<&mut Plot, Self::Error>;
    /// Full Sight.Manager.UpdateMap(unit, old, false), only on removal to an
    /// invalid destination after successful old membership removal.
    fn remove_sight(&mut self, old: PlotCoord) -> Result<(), Self::Error>;
    /// Resolve current EditFormation, when present resolve again and assign its
    /// actual location. An absent formation is allowed; unknown is an error.
    fn set_current_formation_location(&mut self, destination: PlotCoord)
        -> Result<(), Self::Error>;
    fn destination_is_water(&mut self, destination: PlotCoord) -> Result<bool, Self::Error>;
    fn destination_has_city_attributes(
        &mut self,
        destination: PlotCoord,
    ) -> Result<bool, Self::Error>;
    fn domain(&mut self) -> Result<i32, Self::Error>;
    fn destination_city_domain_valid(
        &mut self,
        destination: PlotCoord,
        domain: i32,
    ) -> Result<bool, Self::Error>;
    fn can_embark(&mut self) -> Result<bool, Self::Error>;
    /// Actual Rules.Placement virtual+0x50 layer choice, called only when the
    /// removal result is UINTMAX. This is not a guessed layer-zero default.
    fn placement_layer(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlacementMembershipError<E> {
    EffectsUnavailable(E),
    Context(E),
    Occupancy(OccupancyError),
}

/// Cached native locals at the next Placement continuation. Identity and old
/// visibility/embarked are captured before removal, not reread after callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementMembership {
    pub old_location: PlotCoord,
    pub destination: PlotCoord,
    pub identity: ComponentId,
    pub old_locally_visible: bool,
    pub was_embarked: bool,
    /// Last successfully removed layer, or actual placement-layer fallback.
    pub layer: u32,
    /// UINTMAX means no embark/disembark event is selected at this point.
    pub embark_event: u32,
    /// Native invalid-destination branch returns immediately after location/
    /// formation writes. Valid placement still requires the remaining tail.
    pub destination_valid: bool,
}

/// Execute real membership/position changes in original order. The caller runs
/// the earlier IsValidLocation/alternate-coordinate branch when requested, then
/// this component, then the rest of Placement::Set before movement expenditure.
/// Synchronous failures retain completed writes. No coordinate-only action or
/// no-op sight/contact/report implementation is supplied.
pub fn set_membership<C: PlacementMembershipContext>(
    c: &mut C,
    destination: PlotCoord,
) -> Result<PlacementMembership, PlacementMembershipError<C::Error>> {
    use PlacementMembershipError::{Context, Occupancy};
    c.preflight_membership(destination)
        .map_err(PlacementMembershipError::EffectsUnavailable)?;
    let u = c.unit().map_err(Context)?;
    let old = u.k_location;
    let identity = ComponentId {
        owner: u.e_owner as i16,
        component_type: 1,
        id: u.id,
    };
    let was_embarked = u.e_activity != ACTIVITY_HOLD && u.b_embarked;
    let old_valid = crate::skip_turn::placement::coordinate_valid(old);
    let destination_valid = crate::skip_turn::placement::coordinate_valid(destination);
    let visible = old_valid
        && c.local_plot_visible(old).map_err(Context)?
        && c.local_unit_visible().map_err(Context)?;
    let mut layer = if old_valid {
        let removed = c
            .world_mut()
            .map_err(Context)?
            .remove_unit_occupancy(identity, old, unit_occupancy::ALL_LAYERS)
            .map_err(Occupancy)?;
        if removed != unit_occupancy::ALL_LAYERS {
            if removed == 0 {
                let plot = c.plot_mut(old).map_err(Context)?;
                plot.unit_count = plot.unit_count.wrapping_sub(1);
            }
            if !destination_valid {
                c.remove_sight(old).map_err(Context)?;
            }
        }
        removed
    } else {
        unit_occupancy::ALL_LAYERS
    };
    c.unit_mut().map_err(Context)?.k_location = destination;
    c.set_current_formation_location(destination)
        .map_err(Context)?;
    // Native checks current Unit location again after the setter continuation.
    let current_valid =
        crate::skip_turn::placement::coordinate_valid(c.unit().map_err(Context)?.k_location);
    let mut embark_event = u32::MAX;
    if current_valid {
        let embark_branch = if c.destination_is_water(destination).map_err(Context)? {
            if c.destination_has_city_attributes(destination)
                .map_err(Context)?
            {
                let domain = c.domain().map_err(Context)?;
                !c.destination_city_domain_valid(destination, domain)
                    .map_err(Context)?
            } else {
                true
            }
        } else {
            false
        };
        embark_event = if embark_branch {
            let can_embark = c.can_embark().map_err(Context)?;
            if !was_embarked && can_embark {
                EMBARK_EVENT
            } else {
                u32::MAX
            }
        } else if was_embarked {
            DISEMBARK_EVENT
        } else {
            u32::MAX
        };
        if layer == unit_occupancy::ALL_LAYERS {
            layer = c.placement_layer().map_err(Context)?;
        }
        // Native AddUnit's returned bool is ignored. Layer-zero Plot count is
        // incremented regardless; a failed malformed caller is not normalized.
        c.world_mut()
            .map_err(Context)?
            .add_unit_occupancy(identity, destination, layer)
            .map_err(Occupancy)?;
        if layer == 0 {
            let plot = c.plot_mut(destination).map_err(Context)?;
            plot.unit_count = plot.unit_count.wrapping_add(1);
        }
    }
    Ok(PlacementMembership {
        old_location: old,
        destination,
        identity,
        old_locally_visible: visible,
        was_embarked,
        layer,
        embark_event,
        destination_valid: current_valid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Context {
        unit: Unit,
        world: World,
        formation_location: PlotCoord,
        calls: Vec<&'static str>,
        admitted: bool,
        visible: bool,
        water: bool,
        city: bool,
        domain_valid: bool,
        can_embark: bool,
        fallback_layer: u32,
        fail_formation: bool,
    }
    const OLD: PlotCoord = PlotCoord { x: 0, y: 0 };
    const NEW: PlotCoord = PlotCoord { x: 1, y: 0 };
    impl Context {
        fn new() -> Self {
            let mut map = crate::world::tests::map();
            map.units.layers.clear();
            let mut world = World::new(map, vec![]).unwrap();
            let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
            unit.id = 9;
            unit.e_owner = 0;
            unit.k_location = OLD;
            unit.e_activity = 0;
            unit.b_embarked = false;
            world
                .add_unit_occupancy(
                    ComponentId {
                        owner: 0,
                        component_type: 1,
                        id: 9,
                    },
                    OLD,
                    0,
                )
                .unwrap();
            world.plot_mut(0).unwrap().unit_count = 1;
            Self {
                unit,
                world,
                formation_location: OLD,
                calls: vec![],
                admitted: true,
                visible: true,
                water: false,
                city: false,
                domain_valid: false,
                can_embark: true,
                fallback_layer: 3,
                fail_formation: false,
            }
        }
    }
    impl PlacementMembershipContext for Context {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            self.calls.push("unit location write");
            Ok(&mut self.unit)
        }
        fn world_mut(&mut self) -> Result<&mut World, Self::Error> {
            self.calls.push("membership");
            Ok(&mut self.world)
        }
        fn preflight_membership(&self, _: PlotCoord) -> Result<(), Self::Error> {
            if self.admitted {
                Ok(())
            } else {
                Err("unknown placement effects")
            }
        }
        fn local_plot_visible(&mut self, _: PlotCoord) -> Result<bool, Self::Error> {
            self.calls.push("plot visible");
            Ok(self.visible)
        }
        fn local_unit_visible(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("unit visible");
            Ok(true)
        }
        fn plot_mut(&mut self, p: PlotCoord) -> Result<&mut Plot, Self::Error> {
            self.calls.push(if p == OLD {
                "old plot count"
            } else {
                "new plot count"
            });
            self.world
                .plot_mut(p.x as usize)
                .ok_or("plot mapping absent")
        }
        fn remove_sight(&mut self, _: PlotCoord) -> Result<(), Self::Error> {
            self.calls.push("remove sight");
            Ok(())
        }
        fn set_current_formation_location(&mut self, p: PlotCoord) -> Result<(), Self::Error> {
            self.calls.push("formation location write");
            if self.fail_formation {
                return Err("formation inaccessible");
            }
            self.formation_location = p;
            Ok(())
        }
        fn destination_is_water(&mut self, _: PlotCoord) -> Result<bool, Self::Error> {
            self.calls.push("water");
            Ok(self.water)
        }
        fn destination_has_city_attributes(&mut self, _: PlotCoord) -> Result<bool, Self::Error> {
            self.calls.push("city attrs");
            Ok(self.city)
        }
        fn domain(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("domain");
            Ok(2)
        }
        fn destination_city_domain_valid(
            &mut self,
            _: PlotCoord,
            _: i32,
        ) -> Result<bool, Self::Error> {
            self.calls.push("city domain");
            Ok(self.domain_valid)
        }
        fn can_embark(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("can embark");
            Ok(self.can_embark)
        }
        fn placement_layer(&mut self) -> Result<u32, Self::Error> {
            self.calls.push("fallback layer");
            Ok(self.fallback_layer)
        }
    }
    #[test]
    fn normal_membership_updates_unit_formation_and_both_plot_counts_in_order() {
        let mut c = Context::new();
        let p = set_membership(&mut c, NEW).unwrap();
        assert_eq!(
            c.calls,
            [
                "plot visible",
                "unit visible",
                "membership",
                "old plot count",
                "unit location write",
                "formation location write",
                "water",
                "membership",
                "new plot count"
            ]
        );
        assert_eq!((c.unit.k_location, c.formation_location), (NEW, NEW));
        assert!(c.world.plot_units(0, 0).unwrap().is_empty());
        assert_eq!(c.world.plot_units(0, 1).unwrap()[0].id, 9);
        assert_eq!(
            (
                c.world.map().plots[0].unit_count,
                c.world.map().plots[1].unit_count
            ),
            (0, 1)
        );
        assert_eq!(
            (p.layer, p.embark_event, p.old_locally_visible),
            (0, u32::MAX, true)
        );
        assert_eq!(c.unit.x_moves_remaining, 0); // expenditure is a later step.
    }
    #[test]
    fn last_removed_layer_controls_plot_counter_branch_and_reinsertion() {
        let mut c = Context::new();
        c.world
            .add_unit_occupancy(
                ComponentId {
                    owner: 0,
                    component_type: 1,
                    id: 9,
                },
                OLD,
                3,
            )
            .unwrap();
        let p = set_membership(&mut c, NEW).unwrap();
        assert_eq!(p.layer, 3);
        // Native RemoveUnit removes both but returns3; caller decrements only
        // when that return is0. Preserve this non-normal duplicate-layer case.
        assert_eq!(c.world.map().plots[0].unit_count, 1);
        assert_eq!(c.world.map().plots[1].unit_count, 0);
        assert!(c.world.plot_units(0, 0).unwrap().is_empty());
        assert_eq!(c.world.plot_units(3, 1).unwrap()[0].id, 9);
    }
    #[test]
    fn invalid_destination_removes_sight_before_location_and_skips_destination_queries() {
        let mut c = Context::new();
        let invalid = PlotCoord { x: -9999, y: -9999 };
        let p = set_membership(&mut c, invalid).unwrap();
        assert_eq!(
            c.calls,
            [
                "plot visible",
                "unit visible",
                "membership",
                "old plot count",
                "remove sight",
                "unit location write",
                "formation location write"
            ]
        );
        assert!(!p.destination_valid);
        assert_eq!(c.unit.k_location, invalid);
    }
    #[test]
    fn water_city_exception_and_embark_event_do_not_set_embarked_early() {
        let mut c = Context::new();
        c.water = true;
        let p = set_membership(&mut c, NEW).unwrap();
        assert_eq!(p.embark_event, EMBARK_EVENT);
        assert!(!c.unit.b_embarked);
        assert!(c.calls.contains(&"can embark"));
        let mut c = Context::new();
        c.water = true;
        c.city = true;
        c.domain_valid = true;
        c.unit.b_embarked = true;
        let p = set_membership(&mut c, NEW).unwrap();
        assert_eq!(p.embark_event, DISEMBARK_EVENT);
        assert!(!c.calls.contains(&"can embark"));
    }
    #[test]
    fn invisible_old_plot_shortcircuits_unit_visibility_and_refusal_is_inert() {
        let mut c = Context::new();
        c.visible = false;
        let p = set_membership(&mut c, NEW).unwrap();
        assert!(!p.old_locally_visible);
        assert!(!c.calls.contains(&"unit visible"));
        let mut c = Context::new();
        c.admitted = false;
        assert!(matches!(
            set_membership(&mut c, NEW),
            Err(PlacementMembershipError::EffectsUnavailable(_))
        ));
        assert_eq!(c.unit.k_location, OLD);
        assert_eq!(c.world.plot_units(0, 0).unwrap().len(), 1);
        assert!(c.calls.is_empty());
    }
    #[test]
    fn later_failure_retains_detachment_and_unit_location_prefix() {
        let mut c = Context::new();
        c.fail_formation = true;
        assert!(matches!(
            set_membership(&mut c, NEW),
            Err(PlacementMembershipError::Context("formation inaccessible"))
        ));
        assert_eq!(c.unit.k_location, NEW);
        assert!(c.world.plot_units(0, 0).unwrap().is_empty());
        assert!(c.world.plot_units(0, 1).unwrap().is_empty());
        assert_eq!(c.world.map().plots[0].unit_count, 0);
    }
    #[test]
    fn same_tile_reinserts_at_end_and_plot_counts_wrap_as_bytes() {
        let mut c = Context::new();
        c.world
            .add_unit_occupancy(
                ComponentId {
                    owner: 0,
                    component_type: 1,
                    id: 10,
                },
                OLD,
                0,
            )
            .unwrap();
        set_membership(&mut c, OLD).unwrap();
        assert_eq!(
            c.world
                .plot_units(0, 0)
                .unwrap()
                .iter()
                .map(|u| u.id)
                .collect::<Vec<_>>(),
            [10, 9]
        );
        assert_eq!(c.world.map().plots[0].unit_count, 1);
        let mut c = Context::new();
        c.world.plot_mut(0).unwrap().unit_count = i8::MIN;
        c.world.plot_mut(1).unwrap().unit_count = i8::MAX;
        set_membership(&mut c, NEW).unwrap();
        assert_eq!(
            (
                c.world.map().plots[0].unit_count,
                c.world.map().plots[1].unit_count
            ),
            (i8::MAX, i8::MIN)
        );
    }

    #[test]
    fn missing_old_membership_uses_actual_layer_provider_after_domain_queries() {
        let mut c = Context::new();
        c.world
            .remove_unit_occupancy(
                ComponentId {
                    owner: 0,
                    component_type: 1,
                    id: 9,
                },
                OLD,
                u32::MAX,
            )
            .unwrap();
        let p = set_membership(&mut c, NEW).unwrap();
        assert_eq!(p.layer, 3);
        assert_eq!(
            &c.calls[c.calls.len() - 3..],
            ["water", "fallback layer", "membership"]
        );
        assert_eq!(c.world.plot_units(3, 1).unwrap()[0].id, 9);
        assert_eq!(c.world.map().plots[0].unit_count, 1);
    }
}
