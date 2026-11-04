//! Connected initialized-map Placement::Set with native movement flags3.
//! Path selection belongs to Formation/MoveTo, not this placement component.
//! Required leaf providers invoke their concrete source implementations on the
//! same actor/world. No report, callback, contact, policy or RNG is discarded.
use super::*;
use crate::{
    improvement_movement,
    unit_sight::{self, SightContext},
};
type E<C> = <C as PlacementMembershipContext>::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validation {
    Rejected,
    Exact,
    Alternate(PlotCoord),
}

pub trait Context:
    PlacementMembershipContext
    + PlacementStateContext<Error = <Self as PlacementMembershipContext>::Error>
    + SightContext<Error = <Self as PlacementMembershipContext>::Error>
    + improvement_movement::Context<Error = <Self as PlacementMembershipContext>::Error>
{
    type PlacementPlot: Copy;
    fn validate_location(
        &mut self,
        destination: PlotCoord,
        flags: u32,
    ) -> Result<Validation, E<Self>>;
    /// Validate current actor/destination and required initial provider inputs.
    /// Later unsupported effects return errors with the completed native prefix
    /// retained; this hook is not a whole-program callback approval gate.
    fn preflight_normal_placement(&self, destination: PlotCoord) -> Result<(), E<Self>>;
    fn local_player(&mut self) -> Result<i32, E<Self>>;
    fn outgoing_visibility_to(&mut self, owner: i32, viewer: i32) -> Result<bool, E<Self>>;
    fn outgoing_visibility_blocked(&mut self, owner: i32, viewer: i32) -> Result<bool, E<Self>>;
    fn local_revealed_state(&mut self, destination: PlotCoord) -> Result<i32, E<Self>>;
    fn visibility_messages_disabled(&mut self) -> Result<bool, E<Self>>;
    fn plot_distance(&mut self, old: PlotCoord, new: PlotCoord) -> Result<i32, E<Self>>;
    /// Full stock Rules.Placement::EmbarkedStateChange.
    fn embarked_state_change(&mut self, target: bool, old_valid: bool) -> Result<(), E<Self>>;
    /// Retained map's actual GetPlot; unknown must not become None.
    fn placement_plot(&mut self, at: PlotCoord) -> Result<Option<Self::PlacementPlot>, E<Self>>;
    fn plot_improvement(&mut self, plot: Self::PlacementPlot) -> Result<u32, E<Self>>;
    fn improvement_fortification(&mut self, kind: u32) -> Result<i32, E<Self>>;
    fn plot_improvement_pillaged(&mut self, plot: Self::PlacementPlot) -> Result<bool, E<Self>>;
    fn buildings_at_plot(&mut self, plot: Self::PlacementPlot) -> Result<Vec<u32>, E<Self>>;
    fn building_fortification(&mut self, kind: u32) -> Result<i32, E<Self>>;
    /// Full GameSignals+16e0 dispatch(unit, cached old, cached destination).
    fn signal_unit_moved(&mut self, old: PlotCoord, new: PlotCoord) -> Result<(), E<Self>>;
    fn plot_is_city(&mut self, plot: Self::PlacementPlot) -> Result<bool, E<Self>>;
    fn plot_district_type(&mut self, plot: Self::PlacementPlot) -> Result<u32, E<Self>>;
    fn owning_city(&mut self, plot: Self::PlacementPlot) -> Result<ComponentId, E<Self>>;
    fn plot_district(&mut self, plot: Self::PlacementPlot) -> Result<ComponentId, E<Self>>;
    /// Resolve actual EditCity (null skips), then real DoCityUnitChanged.
    fn city_unit_changed(&mut self, city: ComponentId) -> Result<(), E<Self>>;
    fn check_siege_near(&mut self, location: PlotCoord, owner: i32) -> Result<(), E<Self>>;
    fn award_post_move_bonuses(&mut self, destination: PlotCoord) -> Result<(), E<Self>>;
    fn notify_barbarian_approach(&mut self, destination: PlotCoord) -> Result<(), E<Self>>;
    fn diplomacy_unit_moved(&mut self, old: PlotCoord) -> Result<(), E<Self>>;
    fn rules_unit_moved(&mut self, old: PlotCoord) -> Result<(), E<Self>>;
}

/// Extra pure queries reached only for invalid old coordinates.
pub trait InitialContext: Context {
    fn definition_team_visibility(&mut self) -> Result<bool, E<Self>>;
    fn local_observer(&mut self) -> Result<i32, E<Self>>;
    fn same_team(&mut self, observer: i32, owner: i32) -> Result<bool, E<Self>>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error<T> {
    Context(T),
    UnsupportedInitialLocation,
    Membership(PlacementMembershipError<T>),
    State(PlacementStateError<T>),
    Sight(unit_sight::SightError<T>),
    Improvement(improvement_movement::Error<T>),
}

fn unit<C: Context>(c: &C) -> Result<&Unit, Error<E<C>>> {
    PlacementMembershipContext::unit(c).map_err(Error::Context)
}
fn edit<C: Context>(c: &mut C) -> Result<&mut Unit, Error<E<C>>> {
    PlacementMembershipContext::unit_mut(c).map_err(Error::Context)
}
fn report<C: Context>(
    c: &mut C,
    event: u32,
    id: ComponentId,
    payload: PlacementPayload,
) -> Result<(), Error<E<C>>> {
    c.send_placement_report(PlacementReport {
        event_hash: event,
        identity: id,
        payload,
    })
    .map_err(Error::Context)
}
fn complete<C: Context>(c: &mut C) -> Result<(), Error<E<C>>> {
    complete_movement(c).map_err(Error::State)?;
    Ok(())
}
fn fortify<C: Context>(c: &mut C, turns: i32) -> Result<(), Error<E<C>>> {
    edit(c)?.i_fortify_turns = turns.max(0);
    Ok(())
}
fn changed_city<C: Context>(
    c: &mut C,
    p: C::PlacementPlot,
    id: ComponentId,
    entering: bool,
) -> Result<(), Error<E<C>>> {
    let city = c.plot_is_city(p).map_err(Error::Context)?;
    if city || c.plot_district_type(p).map_err(Error::Context)? != u32::MAX {
        let owner = c.owning_city(p).map_err(Error::Context)?;
        c.city_unit_changed(owner).map_err(Error::Context)?;
        // Native rereads the retained attributes after the cultural callback.
        let target = if city {
            c.owning_city(p)
        } else {
            c.plot_district(p)
        }
        .map_err(Error::Context)?;
        let hash = match (city, entering) {
            (true, true) => 0x89facdc3,
            (false, true) => 0xc1f57315,
            (true, false) => 0x8ee31e6f,
            (false, false) => 0x4ce3627a,
        };
        report(c, hash, id, PlacementPayload::OtherIdentity(target))?;
    }
    Ok(())
}

/// Native flags3 = validate plus normal adjacent-movement reporting. This
/// executes through the final Placement/Units/Diplomacy continuations; it does
/// not select a path, spend Formation movement, or complete an operation queue.
/// Return is native validation0/1/2; alternate placement can mutate then return2.
pub fn set<C: Context>(
    c: &mut C,
    destination: PlotCoord,
    validation_flags: u32,
) -> Result<i32, Error<E<C>>> {
    set_impl(c, destination, validation_flags, false, |_, _, _| Ok(()))
}
/// Source flags1 and invalid old location, retaining the complete shared tail.
pub fn set_initial<C: InitialContext>(
    c: &mut C, destination: PlotCoord, validation_flags: u32,
) -> Result<i32, Error<E<C>>> {
    set_impl(c, destination, validation_flags, true, |c, membership, maximum| {
        let team_visible = if c.definition_team_visibility().map_err(Error::Context)? {
            let observer = InitialContext::local_observer(c).map_err(Error::Context)?;
            let owner = unit(c)?.e_owner;
            c.same_team(observer, owner).map_err(Error::Context)?
        } else { true };
        let location = unit(c)?.k_location;
        report(c, 0xb8515a41, membership.identity, PlacementPayload::Initial {
            location, layer: membership.layer as i32, maximum_charges: maximum,
            embark_event: membership.embark_event, team_visible,
        })
    })
}
fn set_impl<C: Context>(
    c: &mut C, destination: PlotCoord, validation_flags: u32, initial: bool,
    mut initial_report: impl FnMut(&mut C, &PlacementMembership, i32) -> Result<(), Error<E<C>>>,
) -> Result<i32, Error<E<C>>> {
    let validation = c
        .validate_location(destination, validation_flags)
        .map_err(Error::Context)?;
    let (destination, result) = match validation {
        Validation::Rejected => return Ok(0),
        Validation::Exact => (destination, 1),
        Validation::Alternate(p) => (p, 2),
    };
    if initial == crate::skip_turn::placement::coordinate_valid(unit(c)?.k_location) {
        return Err(Error::UnsupportedInitialLocation);
    }
    c.preflight_normal_placement(destination)
        .map_err(Error::Context)?;
    let m = set_membership(c, destination).map_err(Error::Membership)?;
    if !m.destination_valid {
        return Ok(result);
    }
    // Charge getters are pure; native repeats the chosen getter. Preserve the
    // same reads even though only the initial-placement report uses this max.
    let build = unit(c)?.i_build_charges;
    let spreads = unit(c)?.k_religion.i_spread_charges;
    let maximum = if build > spreads {
        unit(c)?.i_build_charges
    } else {
        unit(c)?.k_religion.i_spread_charges
    };
    if initial { initial_report(c, &m, maximum)?; }
    let owner = i32::from(m.identity.owner);
    let viewer = c.local_player().map_err(Error::Context)?;
    let shared = viewer == owner
        || (c
            .outgoing_visibility_to(owner, viewer)
            .map_err(Error::Context)?
            && !c
                .outgoing_visibility_blocked(owner, viewer)
                .map_err(Error::Context)?);
    let mut visible = shared;
    let mut revealed = if shared { 2 } else { 0 };
    if !shared
        && c.local_plot_visible(destination).map_err(Error::Context)?
        && c.local_unit_visible().map_err(Error::Context)?
    {
        revealed = c
            .local_revealed_state(destination)
            .map_err(Error::Context)?;
        visible = true;
    }
    if visible != m.old_locally_visible && visible {
        complete(c)?;
        if !c.visibility_messages_disabled().map_err(Error::Context)? {
            report(
                c,
                0xc1300ee7,
                m.identity,
                PlacementPayload::Visibility {
                    state: revealed,
                    initial_placement: Some(initial),
                },
            )?;
        }
    }
    apply_embark_change(c, m.embark_event, m.old_location, |c, target, old| {
        c.embarked_state_change(target, old)
    })
    .map_err(Error::State)?;
    let air = PlacementMembershipContext::domain(c).map_err(Error::Context)? == 1;
    if !initial && (air
        || c.plot_distance(m.old_location, unit(c)?.k_location)
            .map_err(Error::Context)? == 1)
    {
        edit(c)?.b_needs_movement_completion = true;
        let position = unit(c)?.k_location;
        // ComponentID ctor1fe21f0: owner−1/type0/idUINTMAX.
        let parent = ComponentId {
            owner: -1,
            component_type: 0,
            id: u32::MAX,
        };
        report(
            c,
            0xbe121408,
            m.identity,
            PlacementPayload::Moved {
                location: position,
                locally_visible: m.old_locally_visible || visible,
                newly_locally_visible: visible,
                embark_event: m.embark_event,
                parent,
            },
        )?;
    } else {
        complete(c)?;
        let position = unit(c)?.k_location;
        report(
            c,
            0x9deb82e7,
            m.identity,
            PlacementPayload::Coordinate(position),
        )?;
    }
    if !visible && visible != m.old_locally_visible {
        complete(c)?;
        if !c.visibility_messages_disabled().map_err(Error::Context)? {
            report(
                c,
                0xc1300ee7,
                m.identity,
                PlacementPayload::Visibility {
                    state: revealed,
                    initial_placement: None,
                },
            )?;
        }
    } else if m.embark_event != u32::MAX {
        complete(c)?;
    }
    unit_sight::update_map(c, destination, true).map_err(Error::Sight)?;
    improvement_movement::unit_moved(c, m.old_location).map_err(Error::Improvement)?;
    fortify(c, 0)?;
    let plot = c.placement_plot(destination).map_err(Error::Context)?;
    if let Some(plot) = plot {
        let kind = c.plot_improvement(plot).map_err(Error::Context)?;
        if kind != u32::MAX {
            let grant = c.improvement_fortification(kind).map_err(Error::Context)?;
            if grant > 0 && !c.plot_improvement_pillaged(plot).map_err(Error::Context)? {
                let grant = c.improvement_fortification(kind).map_err(Error::Context)?;
                fortify(c, grant)?;
            }
        } else {
            for building in c.buildings_at_plot(plot).map_err(Error::Context)? {
                if c.building_fortification(building).map_err(Error::Context)? > 0 {
                    let grant = c.building_fortification(building).map_err(Error::Context)?;
                    fortify(c, grant)?;
                }
            }
        }
    }
    c.signal_unit_moved(m.old_location, destination)
        .map_err(Error::Context)?;
    if let Some(plot) = plot {
        changed_city(c, plot, m.identity, true)?;
        let location = unit(c)?.k_location;
        let owner = unit(c)?.e_owner;
        c.check_siege_near(location, owner)
            .map_err(Error::Context)?;
    }
    if let Some(plot) = c.placement_plot(m.old_location).map_err(Error::Context)? {
        changed_city(c, plot, m.identity, false)?;
    }
    c.award_post_move_bonuses(destination)
        .map_err(Error::Context)?;
    c.notify_barbarian_approach(destination)
        .map_err(Error::Context)?;
    c.diplomacy_unit_moved(m.old_location)
        .map_err(Error::Context)?;
    c.rules_unit_moved(m.old_location).map_err(Error::Context)?;
    Ok(result)
}
