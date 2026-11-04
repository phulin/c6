//! Live Linux delayed Unit death and Placement.Remove. Reached unsupported
//! synchronous consumers retain their native state prefix and return an error.
pub mod effects;
pub mod objects;

use crate::{loaded_upkeep, unit_placement_services as services, unit_sight_state as sight};
use c6_core::{
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord, Unit},
    unit_placement_transition::{PlacementPayload, PlacementReport},
};

type EffectError<E> = <E as sight::Effects>::Error;
#[derive(Debug)]
pub enum Error<E, P = E> {
    Missing(&'static str),
    EffectFields(c6_core::unit_kill::effects::Error),
    CleanupQuery(crate::melee_attack::backend::QueryError),
    ObjectCleanup(Box<c6_core::unit_kill::objects::Error<Error<E, P>>>),
    CommonEffect(crate::unit_modifier_effect::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Occupancy(c6_core::unit_occupancy::OccupancyError),
    Sight(c6_core::unit_sight::SightError<sight::Error<P>>),
    Services(services::Error<P>),
    Effect(E),
    Provider(P),
    Cost(crate::formation_movement_cost::Error),
    Placement(crate::movement_placement_state::Error),
    Upkeep(loaded_upkeep::Error<P>),
}
pub trait Providers:
    sight::Effects
    + services::Services<Error = EffectError<Self>>
    + loaded_upkeep::OperationReports<Error = EffectError<Self>>
{
}
impl<T> Providers for T where
    T: sight::Effects
        + services::Services<Error = EffectError<T>>
        + loaded_upkeep::OperationReports<Error = EffectError<T>>
{
}
pub trait Effects: Providers {
    fn signal_ability_lost(
        &mut self,
        _state: &mut LoadedState,
        _runtime: &mut sight::Runtime,
        _actor: (i32, u32),
        _ability: i32,
    ) -> Result<(), Error<EffectError<Self>>> {
        Err(Error::Missing(
            "current GameSignals+1578 ability lost consumers",
        ))
    }
    fn signal_unit_removed(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _actor: (i32, u32),
    ) -> Result<(), Error<EffectError<Self>>> {
        Err(Error::Missing(
            "current GameSignals+1470 UnitRemoved consumers",
        ))
    }
    fn remove_pairing(
        &mut self,
        _state: &mut LoadedState,
        _parent: ComponentId,
        _child: ComponentId,
    ) -> Result<(), Error<EffectError<Self>>> {
        Err(Error::Missing("ObjectPairing.RemoveChild nonnull parent"))
    }
    fn destroy_immediately(
        &mut self,
        _state: &mut LoadedState,
        _runtime: &mut sight::Runtime,
        _actor: (i32, u32),
    ) -> Result<(), Error<EffectError<Self>>> {
        Err(Error::Missing(
            "Player.Units.Destroy immediate roster/destructor path",
        ))
    }
    fn split_dying_unit(
        &mut self,
        _state: &mut LoadedState,
        _actor: (i32, u32),
    ) -> Result<(), Error<EffectError<Self>>> {
        Err(Error::Missing(
            "RemoveUnit/Create singleton dying Formation",
        ))
    }
}
/// Lifecycle services may borrow separate fields of the same persistent
/// backend. Report/sight providers keep their existing error and GAT types.
pub trait Continuations<P: Providers> {
    type Error;
    fn removed(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _actor: (i32, u32),
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("current UnitRemoved dispatch"))
    }
    fn ability_lost(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _actor: (i32, u32),
        _ability: i32,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("current ability-revoked dispatch"))
    }
    fn split(
        &mut self,
        _state: &mut LoadedState,
        _providers: &mut P,
        _actor: (i32, u32),
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("dying Formation split"))
    }
    fn unpair(
        &mut self,
        _state: &mut LoadedState,
        _providers: &mut P,
        _parent: ComponentId,
        _child: ComponentId,
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("nonnull parent unpair"))
    }
    fn immediate(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _providers: &mut P,
        _actor: (i32, u32),
    ) -> Result<(), Error<Self::Error, EffectError<P>>> {
        Err(Error::Missing("immediate Unit destruction"))
    }
}
struct Legacy;
impl<P: Effects> Continuations<P> for Legacy {
    type Error = EffectError<P>;
    fn removed(
        &mut self,
        s: &mut LoadedState,
        v: &mut sight::Runtime,
        p: &mut P,
        a: (i32, u32),
    ) -> Result<(), Error<Self::Error>> {
        p.signal_unit_removed(s, v, a)
    }
    fn ability_lost(
        &mut self,
        s: &mut LoadedState,
        v: &mut sight::Runtime,
        p: &mut P,
        a: (i32, u32),
        i: i32,
    ) -> Result<(), Error<Self::Error>> {
        p.signal_ability_lost(s, v, a, i)
    }
    fn split(
        &mut self,
        s: &mut LoadedState,
        p: &mut P,
        a: (i32, u32),
    ) -> Result<(), Error<Self::Error>> {
        p.split_dying_unit(s, a)
    }
    fn unpair(
        &mut self,
        s: &mut LoadedState,
        p: &mut P,
        a: ComponentId,
        b: ComponentId,
    ) -> Result<(), Error<Self::Error>> {
        p.remove_pairing(s, a, b)
    }
    fn immediate(
        &mut self,
        s: &mut LoadedState,
        v: &mut sight::Runtime,
        p: &mut P,
        a: (i32, u32),
    ) -> Result<(), Error<Self::Error>> {
        p.destroy_immediately(s, v, a)
    }
}
fn unit<E, P>(state: &LoadedState, actor: (i32, u32)) -> Result<&Unit, Error<E, P>> {
    state
        .world()
        .player(actor.0)
        .map_err(Error::World)?
        .unit(actor.1)
        .map_err(Error::World)?
        .map(|v| v.record)
        .ok_or(Error::Missing("retained dying Unit"))
}
fn unit_mut<E, P>(state: &mut LoadedState, actor: (i32, u32)) -> Result<&mut Unit, Error<E, P>> {
    state.unit_mut(actor.0, actor.1).map_err(Error::State)
}
fn identity(unit: &Unit) -> ComponentId {
    ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    }
}
fn send<E: Providers, C>(
    state: &mut LoadedState,
    effects: &mut E,
    report: PlacementReport,
) -> Result<(), Error<C, EffectError<E>>> {
    let actor = loaded_upkeep::UnitHandle::resolve(
        state,
        i32::from(report.identity.owner),
        report.identity.id,
    )
    .map_err(Error::World)?
    .ok_or(Error::Missing("kill report retained actor"))?;
    effects
        .preflight_placement_report(state, actor, report.event_hash)
        .map_err(Error::Provider)?;
    effects
        .placement_report(state, report)
        .map_err(Error::Provider)
}

/// Native Placement.Remove2c42bb0. All-layer membership removal precedes sight;
/// invalid coordinate/report writes occur only after a successful removal.
/// This helper retains the Unit in its roster and does not destroy its Formation.
fn remove_placement_inner<E: Providers, C>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<(), Error<C, EffectError<E>>> {
    let old = unit(state, actor)?.k_location;
    if !c6_core::skip_turn::placement::coordinate_valid(old) {
        return Ok(());
    }
    let id = identity(unit(state, actor)?);
    let Some(index) = services::plot_index_at(state, old).map_err(Error::Services)? else {
        return Err(Error::Missing("Placement.Remove old Map plot"));
    };
    let base = state
        .world()
        .plot_units(0, index as usize)
        .map_err(Error::World)?
        .contains(&c6_core::map::UnitReference::from_component_id(id));
    let removed = state
        .world_mut()
        .map_err(Error::State)?
        .remove_unit_occupancy(id, old, u32::MAX)
        .map_err(Error::Occupancy)?;
    if removed == u32::MAX {
        return Ok(());
    }
    if base {
        let plot = state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(index as usize)
            .ok_or(Error::Missing("retained old plot"))?;
        plot.unit_count = plot.unit_count.wrapping_sub(1);
    }
    sight::Context {
        state,
        runtime,
        effects,
        sight_definitions: definitions,
        actor,
    }
    .update_map(old, false)
    .map_err(Error::Sight)?;
    unit_mut(state, actor)?.k_location = PlotCoord { x: -1, y: -1 };
    send(
        state,
        effects,
        PlacementReport {
            event_hash: 0xd8adfd71,
            identity: id,
            payload: PlacementPayload::Identity,
        },
    )?;
    // Native queries the OLD plot after the first report, so callbacks' current
    // city/district changes are visible here.
    if services::plot_is_city(state, effects, index).map_err(Error::Services)? {
        let city = services::owning_city(state, effects, index)
            .map_err(Error::Services)?
            .ok_or(Error::Missing("old city identity"))?;
        send(
            state,
            effects,
            PlacementReport {
                event_hash: 0x8ee31e6f,
                identity: id,
                payload: PlacementPayload::OtherIdentity(ComponentId {
                    owner: city.0 as i16,
                    component_type: 0,
                    id: city.1,
                }),
            },
        )?;
    } else if let Some((owner, id_)) =
        services::plot_district(state, effects, index).map_err(Error::Services)?
    {
        send(
            state,
            effects,
            PlacementReport {
                event_hash: 0x4ce3627a,
                identity: id,
                payload: PlacementPayload::OtherIdentity(ComponentId {
                    owner: owner as i16,
                    component_type: 2,
                    id: id_,
                }),
            },
        )?;
    }
    Ok(())
}

struct Context<'a, P, C> {
    state: &'a mut LoadedState,
    runtime: &'a mut sight::Runtime,
    effects: &'a mut P,
    continuations: &'a mut C,
    definitions: &'a crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
}
impl<P: Providers, C: Continuations<P>> c6_core::unit_kill::Context for Context<'_, P, C> {
    type Error = Error<C::Error, EffectError<P>>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        unit(self.state, self.actor)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        unit_mut(self.state, self.actor)
    }
    fn signal_removed(&mut self) -> Result<(), Self::Error> {
        self.continuations
            .removed(self.state, self.runtime, self.effects, self.actor)
    }
    fn formation_unit_count(&self) -> Result<Option<usize>, Self::Error> {
        let unit = self.unit()?;
        Ok(self
            .state
            .world()
            .player(unit.e_owner)
            .map_err(Error::World)?
            .formation(unit.formation_id)
            .map_err(Error::World)?
            .map(|f| f.record.unit_ids.len()))
    }
    fn split_dying_unit(&mut self) -> Result<(), Self::Error> {
        self.continuations
            .split(self.state, self.effects, self.actor)
    }
    fn remove_placement(&mut self) -> Result<(), Self::Error> {
        remove_placement_inner(
            self.state,
            self.runtime,
            self.effects,
            self.definitions,
            self.actor,
        )
    }
}
/// Player.Units.DelayedDestroy stage over the real retained Unit. Manager.Kill's
/// preceding ZOC/pairing/siege work must execute before this entry.
pub fn delayed_destroy<E: Effects>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<(), Error<EffectError<E>>> {
    delayed_destroy_with_continuations(state, runtime, effects, &mut Legacy, definitions, actor)
}
pub fn remove_placement<P: Effects>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    providers: &mut P,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<(), Error<EffectError<P>>> {
    remove_placement_inner(state, runtime, providers, definitions, actor)
}
pub fn delayed_destroy_with_continuations<P: Providers, C: Continuations<P>>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut P,
    continuations: &mut C,
    definitions: &crate::loaded_actions::city_range_attack::Definitions,
    actor: (i32, u32),
) -> Result<(), Error<C::Error, EffectError<P>>> {
    c6_core::unit_kill::delayed_destroy(&mut Context {
        state,
        runtime,
        effects,
        continuations,
        definitions,
        actor,
    })
}

/// Native stock Manager.DestroyUnit26af000 reached through Manager.Kill. Does
/// not erase a delayed Unit: subsequent combat XP still uses this same object.
/// `killed_by` is accepted for the Manager.Kill ABI; this stock DestroyUnit target
/// does not read it. A Rules override requires its own source-qualified route.
#[allow(clippy::too_many_arguments)]
pub fn kill_with_continuations<E, C>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    continuations: &mut C,
    definitions: &crate::unit_placement_state::Definitions<'_, '_>,
    queries: &crate::unit_placement_state::Queries<'_>,
    actor: (i32, u32),
    delayed: bool,
    _killed_by: i32,
) -> Result<bool, Error<C::Error, EffectError<E>>>
where
    E: Providers + loaded_upkeep::FormationRestoreEffects<Error = EffectError<E>>,
    C: Continuations<E>,
{
    let Some(found) = state
        .world()
        .player(actor.0)
        .map_err(Error::World)?
        .unit(actor.1)
        .map_err(Error::World)?
    else {
        return Ok(false);
    };
    let layer = c6_core::skip_turn::placement::unit_layer(state.world(), found.record)
        .map_err(|_| Error::Missing("dying Unit location layer"))?
        .unwrap_or(u32::MAX);
    for direction in 0..6 {
        let old = unit(state, actor)?.k_location;
        let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
            state,
            (old.x, old.y),
            direction,
        ) else {
            continue;
        };
        let Some(index) =
            services::plot_index_at(state, PlotCoord { x, y }).map_err(Error::Services)?
        else {
            continue;
        };
        let others = state
            .world()
            .plot_units(layer, index as usize)
            .map_err(Error::World)?
            .to_vec();
        for other in others {
            let Some(other) = state.world().unit(other).map_err(Error::World)? else {
                continue;
            };
            let owner = other.record.e_owner;
            let formation_id = other.record.formation_id;
            let Some(formation) = state
                .world()
                .player(owner)
                .map_err(Error::World)?
                .formation(formation_id)
                .map_err(Error::World)?
            else {
                continue;
            };
            if !formation.record.has_moved_into_zoc {
                continue;
            }
            let value = {
                let placement = queries
                    .placement_context(state, definitions.placement)
                    .map_err(Error::Placement)?;
                crate::formation_movement_cost::slowed_by_zoc(
                    state,
                    definitions.movement,
                    &placement,
                    formation.record,
                    formation.record.location,
                    None,
                    Some(runtime),
                )
                .map_err(Error::Cost)?
            };
            // Native setter is unconditional for the reached old=true branch.
            loaded_upkeep::set_formation_has_moved_into_zoc(
                state,
                owner,
                formation_id,
                effects,
                value,
            )
            .map_err(Error::Upkeep)?;
        }
    }
    let id = identity(unit(state, actor)?);
    let children = state
        .object_pairing()
        .map_err(Error::State)?
        .get_children(id)
        .to_vec();
    for child in children {
        // Native selects the parent's CURRENT owner roster, regardless child type/owner.
        let owner = unit(state, actor)?.e_owner;
        if state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit(child.id)
            .map_err(Error::World)?
            .is_some()
        {
            kill_with_continuations(
                state,
                runtime,
                effects,
                continuations,
                definitions,
                queries,
                (owner, child.id),
                true,
                -1,
            )?;
        }
    }
    let current = identity(unit(state, actor)?);
    let parent = state
        .object_pairing()
        .map_err(Error::State)?
        .parent_entries()
        .iter()
        .find(|(child, _)| *child == current)
        .map(|(_, parent)| *parent)
        .unwrap_or(c6_core::object_pairing::NULL_COMPONENT);
    if parent != c6_core::object_pairing::NULL_COMPONENT {
        continuations.unpair(state, effects, parent, current)?;
    }
    let current = unit(state, actor)?;
    services::siege::run(state, runtime, effects, definitions, current.k_location, current.e_owner)
        .map_err(Error::Services)?;
    if !(delayed && unit(state, actor)?.b_delayed_death) {
        if delayed {
            delayed_destroy_with_continuations(
                state,
                runtime,
                effects,
                continuations,
                definitions.sight,
                actor,
            )?;
        } else {
            continuations.immediate(state, runtime, effects, actor)?;
        }
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn kill<P>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    providers: &mut P,
    definitions: &crate::unit_placement_state::Definitions<'_, '_>,
    queries: &crate::unit_placement_state::Queries<'_>,
    actor: (i32, u32),
    delayed: bool,
    killed_by: i32,
) -> Result<bool, Error<EffectError<P>>>
where
    P: Effects + loaded_upkeep::FormationRestoreEffects<Error = EffectError<P>>,
{
    kill_with_continuations(
        state,
        runtime,
        providers,
        &mut Legacy,
        definitions,
        queries,
        actor,
        delayed,
        killed_by,
    )
}
