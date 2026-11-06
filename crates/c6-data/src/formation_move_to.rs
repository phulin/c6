//! Linux XP2 Formation MoveTo composition on canonical loaded gameplay state.
//! Path selection, cost, Placement, expenditure and terminal callbacks share one
//! live World. Unsupported reached branches fail explicitly; this bounded action
//! entry does not establish complete gameplay actions or turns.
use crate::{formation_path_state as path, unit_movement_state as unit_moves};
use c6_core::{
    formation_path_cache::{self as cache, GenerationContext},
    loaded_state::LoadedState,
    movement::{MoveBudget, MovementPoints},
    movement_shore::PlacementRequest,
    pathfinder::TwoLayer,
    skip_turn::placement as unit_placement,
    unit::{Formation, PlotCoord},
};

fn formation(state: &LoadedState, identity: (i32, u32)) -> Result<&Formation, path::Error> {
    state
        .world()
        .player(identity.0)
        .map_err(path::Error::World)?
        .formation(identity.1)
        .map_err(path::Error::World)?
        .map(|f| f.record)
        .ok_or(path::Error::Missing("MoveTo Formation"))
}

/// Native mutable TestMoveTo (0x29482c0). This queries actual placement and
/// visibility, then computes/reuses the real path cache. Its integer result is
/// the destination node's turn count; false visibility is never placement success.
#[allow(clippy::too_many_arguments)]
pub fn test_move_to<'a, 'r>(
    state: &mut LoadedState,
    definitions: &'a path::Definitions<'a, 'r>,
    runtime: path::Runtime<'a>,
    search: &mut TwoLayer,
    identity: (i32, u32),
    destination: PlotCoord,
    flags: u32,
) -> Result<i32, path::Error> {
    let mut context = path::Context::new(state, definitions, runtime, identity)?;
    if GenerationContext::formation(&context).location == destination {
        return Ok(0);
    }
    if !GenerationContext::is_plot(&mut context, destination)? {
        return Ok(0);
    }
    let owner = GenerationContext::formation(&context).owner;
    let id = GenerationContext::formation(&context).id;
    if path::includes_unit_of_domain(context.state, GenerationContext::formation(&context), 1)? {
        let result = context
            .placement()?
            .formation_placement(
                owner,
                id,
                destination,
                PlacementRequest::from_validate_options(1),
            )
            .map_err(path::Error::Placement)?;
        if result != 1 {
            return Ok(0);
        }
        cache::clear_path_cache(GenerationContext::formation_mut(&mut context)?);
        return Ok(1);
    }
    if flags & 0x10 == 0 && GenerationContext::is_visible(&mut context, destination)? {
        let result = context
            .placement()?
            .formation_placement(
                owner,
                id,
                destination,
                PlacementRequest::from_validate_options(0),
            )
            .map_err(path::Error::Placement)?;
        if result == 0 {
            cache::clear_path_cache(GenerationContext::formation_mut(&mut context)?);
            return Ok(0);
        }
    }
    if !context.update_path_cache(search, destination, flags)? {
        return Ok(0);
    }
    Ok(GenerationContext::formation(&context)
        .last_path
        .first()
        .map_or(0, |n| n.data2))
}

/// GetPathFirstPlot (0x293d910) indexes the computed destination-first path.
fn first_step(formation: &Formation) -> Option<PlotCoord> {
    let index = formation.last_path.len().saturating_sub(2);
    formation.last_path.get(index).map(|n| PlotCoord {
        x: i32::from(n.x),
        y: i32::from(n.y),
    })
}

/// Native GetFormationsMoves<int> folds cache the member end and skip null Find.
/// The domain-taking Formation wrapper (0x293d130) passes NO_DOMAIN (-1), even
/// though its public signature and MoveTo caller carry another domain value.
fn movement_budget(
    state: &LoadedState,
    definitions: &path::Definitions<'_, '_>,
    placement: &crate::movement_placement_state::Context<'_, '_, '_>,
    identity: (i32, u32),
) -> Result<MoveBudget, path::Error> {
    let record = formation(state, identity)?;
    let player = state
        .world()
        .player(record.owner)
        .map_err(path::Error::World)?;
    let inputs = unit_moves::Inputs {
        definitions: &definitions.units,
    };
    let fold = |base: bool| -> Result<i32, path::Error> {
        let mut minimum = 9001i32;
        let mut escort_maximum = 0;
        for &id in &record.unit_ids {
            let Some(found) = player.unit(id).map_err(path::Error::World)? else {
                continue;
            };
            let unit = found.record;
            let moves = if base {
                unit_moves::unit_base_moves_with_can_embark(state, unit, -1, &inputs, || {
                    placement.can_embark(unit)
                })
                .map_err(path::Error::UnitMoves)?
            } else {
                unit_moves::unit_max_moves(state, unit, &inputs).map_err(path::Error::UnitMoves)?
            };
            minimum = minimum.min(moves);
            if unit.k_movement_modifiers.b_escort_mobility {
                escort_maximum = escort_maximum.max(moves)
            }
        }
        Ok(minimum.max(escort_maximum))
    };
    Ok(MoveBudget {
        base_moves: fold(true)?,
        max_moves: MovementPoints::integer(fold(false)?),
        remaining: MovementPoints(record.moves_remaining_raw),
        exit_moves: MovementPoints(0),
    })
}

/// Current owned configuration references used by both path and placement
/// queries. These are configuration/state fields, never per-action answers.
pub struct Configuration<'a> {
    pub civilization_levels:
        Option<&'a crate::movement_placement_state::CurrentCivilizationLevels<'a>>,
    pub city_domains: Option<&'a path::CurrentCityDomains<'a>>,
    pub execution: path::ExecutionContext,
}
impl Configuration<'_> {
    fn runtime<'a>(
        &'a self,
        state: &LoadedState,
        visibility: &'a crate::unit_sight_state::Runtime,
        identity: (i32, u32),
    ) -> Result<path::Runtime<'a>, path::Error> {
        Ok(path::Runtime {
            visibility,
            civilization_levels: self.civilization_levels.map(|v| {
                crate::movement_placement_state::CurrentCivilizationLevels {
                    binding: v.binding,
                    values: v.values,
                }
            }),
            city_domains: self.city_domains.map(|v| path::CurrentCityDomains {
                binding: v.binding,
                stamp: v.stamp,
                masks: v.masks,
            }),
            execution: self.execution,
            is_evaluating_operation: state
                .formation_runtime(identity.0, identity.1)
                .map_err(path::Error::State)?
                .evaluating_operation(),
        })
    }
}

#[derive(Debug)]
pub enum Error<E, P> {
    Query(path::Error),
    Placement(P),
    Upkeep(crate::loaded_upkeep::Error<E>),
    Attributes(c6_core::formation_upkeep::FormationUpkeepError<crate::loaded_upkeep::Error<E>>),
    Expenditure(
        c6_core::movement_execution::FormationMovementError<crate::loaded_upkeep::Error<E>>,
    ),
    Unsupported(&'static str),
    Capture(String),
}

fn preflight_children(state: &LoadedState, identity: (i32, u32)) -> Result<(), path::Error> {
    let record = formation(state, identity)?;
    let player = state
        .world()
        .player(record.owner)
        .map_err(path::Error::World)?;
    for &id in &record.unit_ids {
        let Some(member) = player.unit(id).map_err(path::Error::World)? else {
            continue;
        };
        let unit = member.record;
        let parent = c6_core::unit::ComponentId {
            owner: unit.e_owner as i16,
            component_type: 1,
            id: unit.id,
        };
        let pairs = state.object_pairing().map_err(path::Error::State)?;
        for child in pairs.get_children(parent) {
            if child.component_type != 1 {
                continue;
            }
            // Source uses the PARENT Unit's owner collection, not child.owner.
            let children = state
                .world()
                .player(unit.e_owner)
                .map_err(path::Error::World)?;
            if children
                .unit(child.id)
                .map_err(path::Error::World)?
                .is_some()
            {
                return Err(path::Error::Missing(
                    "MoveChildren live child Placement flags10/escort continuation",
                ));
            }
        }
    }
    Ok(())
}

/// Native post-Placement capture scan over the selected Plot and current
/// Formation layer (2947add..2947e1d). The empty-candidate continuation includes
/// the moving combat Unit itself. Noncombat candidates use the supplied current
/// capture services; unavailable or unsupported continuations remain errors.
fn capture_scan<E, T, P, C: crate::civilian_capture::Capture<E>>(
    state: &mut LoadedState,
    definitions: &path::Definitions<'_, '_>,
    identity: (i32, u32),
    target: PlotCoord,
    handler: &mut C,
    mut continuation: Option<(&mut crate::unit_sight_state::Runtime, &mut E)>,
) -> Result<(), Error<T, P>> {
    let record = formation(state, identity).map_err(Error::Query)?;
    if !record.can_capture {
        return Ok(());
    }
    let layer = unit_placement::formation_layer(state.world(), record)
        .map_err(|e| Error::Query(path::Error::PlacementQuery(e)))?
        .ok_or(Error::Unsupported(
            "MoveTo capture scan missing current layer",
        ))?;
    let count = unit_placement::units_at(state.world(), Some(layer), target)
        .map_err(|e| Error::Query(path::Error::PlacementQuery(e)))?
        .len();
    for index in (0..count).rev() {
        let reference = *unit_placement::units_at(state.world(), Some(layer), target)
            .map_err(|e| Error::Query(path::Error::PlacementQuery(e)))?
            .get(index)
            .ok_or(Error::Unsupported("capture plot-unit cursor invalidated"))?;
        let Some(unit) = state
            .world()
            .unit(reference)
            .map_err(|e| Error::Query(path::Error::World(e)))?
        else {
            continue;
        };
        let unit = unit.record;
        if unit.b_delayed_death {
            continue;
        }
        // Unit.IsCombat20dee90 short-circuits melee, ranged, then bombard.
        // These existing admitted getters are read-only and draw no RNG.
        if definitions
            .formation
            .combat(state, unit)
            .map_err(|e| Error::Query(path::Error::Rules(e)))?
            > 0
            || definitions
                .formation
                .ranged(state, unit)
                .map_err(|e| Error::Query(path::Error::Rules(e)))?
                > 0
            || definitions
                .formation
                .bombard(state, unit)
                .map_err(|e| Error::Query(path::Error::Rules(e)))?
                > 0
        {
            continue;
        }
        let victim = (unit.e_owner, unit.id);
        if let Some((visibility, effects)) = continuation.as_mut() {
            handler
                .capture(state, visibility, effects, identity, victim)
                .map_err(Error::Capture)?;
        } else {
            handler
                .preflight(state, identity, victim)
                .map_err(Error::Capture)?;
        }
    }
    Ok(())
}

fn placement_flags(flags: u32) -> u32 {
    ((flags & 4) << 1) | ((flags >> 4) & 3) | ((flags >> 6) & 0x80)
}

// The private step function is joined to the concrete Data Unit Placement below.
// Its closure permits the Formation member loop to retain the same live E/state
// borrows while the existing Unit adapter executes each member in native order.
#[allow(clippy::too_many_arguments)]
fn execute<'a, 'r, E, P, C: crate::civilian_capture::Capture<E>>(
    state: &mut LoadedState,
    definitions: &'a path::Definitions<'a, 'r>,
    configuration: &Configuration<'_>,
    visibility: &mut crate::unit_sight_state::Runtime,
    search: &mut TwoLayer,
    identity: (i32, u32),
    destination: PlotCoord,
    flags: u32,
    effects: &mut E,
    capture: &mut C,
    mut place: impl FnMut(
        &mut LoadedState,
        &mut crate::unit_sight_state::Runtime,
        &mut E,
        (i32, u32),
        PlotCoord,
        u32,
    ) -> Result<i32, P>,
) -> Result<i32, Error<<E as crate::loaded_upkeep::Effects>::Error, P>>
where
    E: crate::loaded_upkeep::FormationRestoreEffects
        + crate::loaded_upkeep::OperationReports<Error = <E as crate::loaded_upkeep::Effects>::Error>,
{
    use crate::{formation_movement_cost as cost, loaded_upkeep as upkeep};
    use c6_core::movement::ZocValue;
    // Unknown effectful branches refuse before path/cache or membership writes.
    if flags & 1 != 0 {
        return Err(Error::Unsupported(
            "MoveTo split/member MakeRequest continuation",
        ));
    }
    if path::includes_unit_of_domain(state, formation(state, identity).map_err(Error::Query)?, 1)
        .map_err(Error::Query)?
    {
        return Err(Error::Unsupported("MoveTo air Formation Placement"));
    }
    // Reject the current known noncombat-candidate branch before mutation.
    // Placement callbacks can change membership; the native scan is repeated
    // below in its actual post-Placement position over the selected step.
    capture_scan(state, definitions, identity, destination, capture, None)?;
    preflight_children(state, identity).map_err(Error::Query)?;
    upkeep::preflight_formation_movement(
        state,
        identity.0,
        identity.1,
        &unit_moves::Inputs {
            definitions: &definitions.units,
        },
        effects,
    )
    .map_err(Error::Upkeep)?;
    let test = test_move_to(
        state,
        definitions,
        configuration
            .runtime(state, visibility, identity)
            .map_err(Error::Query)?,
        search,
        identity,
        destination,
        flags,
    )
    .map_err(Error::Query)?;
    if test == 0 {
        return Ok(0);
    }
    let from = formation(state, identity).map_err(Error::Query)?.location;
    let Some(to) = first_step(formation(state, identity).map_err(Error::Query)?) else {
        return Ok(0);
    };
    let count = formation(state, identity)
        .map_err(Error::Query)?
        .last_path
        .len();
    if count > 1 && formation(state, identity).map_err(Error::Query)?.last_path[count - 2].data2 > 1
    {
        return Ok(-1);
    }
    if count != 0 {
        let runtime = configuration
            .runtime(state, visibility, identity)
            .map_err(Error::Query)?;
        let placement =
            path::placement_context(state, definitions, &runtime).map_err(Error::Query)?;
        let record = formation(state, identity).map_err(Error::Query)?;
        if flags & 0x10 == 0 && record.last_path[0].data2 == 1 {
            let result = placement
                .formation_placement(
                    record.owner,
                    record.id,
                    destination,
                    PlacementRequest::from_validate_options(0),
                )
                .map_err(|e| Error::Query(path::Error::Placement(e)))?;
            if result != 1 {
                // The source next asks defender flags0x101, enemy city, then
                // retry flags1. No combat-answer substitute is introduced.
                return Err(Error::Unsupported(
                    "MoveTo destination defender/retry placement branch",
                ));
            }
        }
        let validation = (flags >> 4) & 1;
        if placement
            .formation_placement(
                record.owner,
                record.id,
                to,
                PlacementRequest::from_validate_options(validation),
            )
            .map_err(|e| Error::Query(path::Error::Placement(e)))?
            != 1
            && record.moves_remaining_raw > 0
            && path::includes_unit_of_domain(state, record, 0).map_err(Error::Query)?
        {
            let (_, target) = c6_core::movement_world::plot_at(state.world().map(), to)
                .map_err(|e| Error::Query(path::Error::Cost(cost::Error::Movement(e))))?;
            if !placement
                .water(target)
                .map_err(|e| Error::Query(path::Error::Placement(e)))?
            {
                let retry = placement
                    .formation_placement(
                        record.owner,
                        record.id,
                        to,
                        PlacementRequest::from_validate_options(validation | 0x80),
                    )
                    .map_err(|e| Error::Query(path::Error::Placement(e)))?;
                if retry == 1 {
                    return Err(Error::Unsupported("MoveTo disembark Formation split"));
                }
            }
        }
    }
    let cached_cost = {
        let runtime = configuration
            .runtime(state, visibility, identity)
            .map_err(Error::Query)?;
        let placement =
            path::placement_context(state, definitions, &runtime).map_err(Error::Query)?;
        let can_attack = formation(state, identity).map_err(Error::Query)?.can_attack;
        let (_, target) = c6_core::movement_world::plot_at(state.world().map(), to)
            .map_err(|e| Error::Query(path::Error::Cost(cost::Error::Movement(e))))?;
        // Both cost branches read target IsWater before Formation::GetBaseMoves.
        // The wrapper itself passes -1 to every Unit getter (0x293d150).
        let _requested_domain = i32::from(
            placement
                .water(target)
                .map_err(|e| Error::Query(path::Error::Placement(e)))?,
        ) - 1;
        let budget =
            movement_budget(state, definitions, &placement, identity).map_err(Error::Query)?;
        let record = formation(state, identity).map_err(Error::Query)?;
        let result = if can_attack {
            cost::movement_cost_no_zoc(
                state,
                &definitions.movement,
                &placement,
                record,
                from,
                to,
                budget,
            )
        } else {
            cost::movement_cost(
                state,
                &definitions.movement,
                &placement,
                record,
                from,
                to,
                Some(destination),
                budget,
                ZocValue::Query,
                flags,
                Some(visibility),
            )
        }
        .map_err(|e| Error::Query(path::Error::Cost(e)))?;
        result.final_cost().0
    };
    if formation(state, identity).map_err(Error::Query)?.has_moved
        && cached_cost
            > formation(state, identity)
                .map_err(Error::Query)?
                .moves_remaining_raw
    {
        upkeep::complete_formation_movement(state, identity.0, identity.1, effects)
            .map_err(Error::Upkeep)?;
        return Ok(-1);
    }
    let validate = placement_flags(flags) | if to == destination { 0 } else { 8 };
    let result =
        place(state, visibility, effects, identity, to, validate).map_err(Error::Placement)?;
    if result == 1 {
        capture_scan(
            state,
            definitions,
            identity,
            to,
            capture,
            Some((visibility, effects)),
        )?;
        upkeep::change_formation_moves(
            state,
            identity.0,
            identity.1,
            &unit_moves::Inputs {
                definitions: &definitions.units,
            },
            effects,
            cached_cost.wrapping_neg(),
        )
        .map_err(Error::Expenditure)?;
        let complete = to == destination
            || cached_cost
                > formation(state, identity)
                    .map_err(Error::Query)?
                    .moves_remaining_raw;
        // MoveChildren repeats current member lookup after expenditure callbacks.
        preflight_children(state, identity).map_err(Error::Query)?;
        upkeep::update_formation(
            state,
            identity.0,
            identity.1,
            &unit_moves::Inputs {
                definitions: &definitions.units,
            },
            effects,
        )
        .map_err(Error::Attributes)?;
        upkeep::set_formation_has_moved(
            state,
            identity.0,
            identity.1,
            &unit_moves::Inputs {
                definitions: &definitions.units,
            },
            effects,
            true,
        )
        .map_err(Error::Attributes)?;
        let zoc = {
            let placement = crate::movement_placement_state::Context::from_definitions(
                state,
                &definitions.placement,
            )
            .map_err(|e| Error::Query(path::Error::Placement(e)))?;
            let record = formation(state, identity).map_err(Error::Query)?;
            cost::slowed_by_zoc(
                state,
                &definitions.movement,
                &placement,
                record,
                record.location,
                None,
                Some(visibility),
            )
            .map_err(|e| Error::Query(path::Error::Cost(e)))?
        };
        if zoc
            != formation(state, identity)
                .map_err(Error::Query)?
                .has_moved_into_zoc
        {
            upkeep::set_formation_has_moved_into_zoc(state, identity.0, identity.1, effects, zoc)
                .map_err(Error::Upkeep)?;
        }
        if complete {
            upkeep::complete_formation_movement(state, identity.0, identity.1, effects)
                .map_err(Error::Upkeep)?
        }
    }
    Ok(cache::finish_movement_path(
        state
            .formation_mut(identity.0, identity.1)
            .map_err(|e| Error::Query(path::Error::State(e)))?,
        result,
    ))
}

#[derive(Debug)]
pub enum PlacementError<E> {
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    MissingFormation,
    InvalidatedMemberCursor { index: usize },
    Unit(crate::unit_placement_state::Error<E>),
}

/// Formation::Movement::MoveTo (0x2947680) for supported initialized normal
/// movement. The real Data Unit Placement adapter executes every member. The
/// return retains native path-turn/false/deferred semantics used by MoveTo.Update.
/// Reached air, noncombat capture candidates, live-child and splitting/request
/// branches remain explicit refusals; no action success is returned for an unfinished continuation.
#[allow(clippy::too_many_arguments)]
pub fn move_to<'a, 'r, E>(
    state: &mut LoadedState,
    definitions: &'a path::Definitions<'a, 'r>,
    placement_definitions: &crate::unit_placement_state::Definitions<'_, '_>,
    queries: &crate::unit_placement_state::Queries<'_>,
    visibility: &mut crate::unit_sight_state::Runtime,
    search: &mut TwoLayer,
    identity: (i32, u32),
    destination: PlotCoord,
    flags: u32,
    execution: path::ExecutionContext,
    effects: &mut E,
) -> Result<
    i32,
    Error<
        crate::unit_placement_state::EffectsError<E>,
        PlacementError<crate::unit_placement_state::EffectsError<E>>,
    >,
>
where
    E: crate::unit_placement_state::Effects
        + crate::loaded_upkeep::FormationRestoreEffects<
            Error = crate::unit_placement_state::EffectsError<E>,
        >,
{
    move_to_with_capture(
        state,
        definitions,
        placement_definitions,
        queries,
        visibility,
        search,
        identity,
        destination,
        flags,
        execution,
        effects,
        &mut crate::civilian_capture::Unavailable,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn move_to_with_capture<'a, 'r, E, C: crate::civilian_capture::Capture<E>>(
    state: &mut LoadedState,
    definitions: &'a path::Definitions<'a, 'r>,
    placement_definitions: &crate::unit_placement_state::Definitions<'_, '_>,
    queries: &crate::unit_placement_state::Queries<'_>,
    visibility: &mut crate::unit_sight_state::Runtime,
    search: &mut TwoLayer,
    identity: (i32, u32),
    destination: PlotCoord,
    flags: u32,
    execution: path::ExecutionContext,
    effects: &mut E,
    capture: &mut C,
) -> Result<
    i32,
    Error<
        crate::unit_placement_state::EffectsError<E>,
        PlacementError<crate::unit_placement_state::EffectsError<E>>,
    >,
>
where
    E: crate::unit_placement_state::Effects
        + crate::loaded_upkeep::FormationRestoreEffects<
            Error = crate::unit_placement_state::EffectsError<E>,
        >,
{
    placement_definitions
        .validate_binding(state)
        .map_err(|e| Error::Query(path::Error::Rules(e)))?;
    // Reject an unavailable synchronous Placement profile before even path
    // queries edit the Formation cache. Providers must cover callbacks and the
    // selected intermediate step, not merely the requested endpoint.
    {
        let record = formation(state, identity).map_err(Error::Query)?;
        let owner = record.owner;
        let player = state
            .world()
            .player(owner)
            .map_err(|e| Error::Query(path::Error::World(e)))?;
        for &id in &record.unit_ids {
            if player
                .unit(id)
                .map_err(|e| Error::Query(path::Error::World(e)))?
                .is_some()
            {
                crate::unit_placement_state::Effects::preflight_normal(
                    effects,
                    state,
                    visibility,
                    (owner, id),
                    destination,
                )
                .map_err(|e| Error::Placement(PlacementError::Unit(e)))?;
            }
        }
    }
    // Copy the small immutable binding identity so its borrow cannot pin the
    // mutable World during actual placement/callbacks. No gameplay state clones.
    let binding = state.binding().clone();
    let levels = queries.civilization_levels.map(|values| {
        crate::movement_placement_state::CurrentCivilizationLevels {
            binding: &binding,
            values,
        }
    });
    let domains = queries.city_domains.map(|masks| path::CurrentCityDomains {
        binding: &binding,
        stamp: state.action_stamp(),
        masks,
    });
    let configuration = Configuration {
        civilization_levels: levels.as_ref(),
        city_domains: domains.as_ref(),
        execution,
    };
    execute(
        state,
        definitions,
        &configuration,
        visibility,
        search,
        identity,
        destination,
        flags,
        effects,
        capture,
        |state, visibility, effects, identity, destination, validation_flags| {
            let record = state
                .world()
                .player(identity.0)
                .map_err(PlacementError::World)?
                .formation(identity.1)
                .map_err(PlacementError::World)?
                .ok_or(PlacementError::MissingFormation)?
                .record;
            let owner = record.owner;
            let count = record.unit_ids.len();
            state.world().player(owner).map_err(PlacementError::World)?;
            for index in 0..count {
                let record = state
                    .world()
                    .player(identity.0)
                    .map_err(PlacementError::World)?
                    .formation(identity.1)
                    .map_err(PlacementError::World)?
                    .ok_or(PlacementError::MissingFormation)?
                    .record;
                let id = *record
                    .unit_ids
                    .get(index)
                    .ok_or(PlacementError::InvalidatedMemberCursor { index })?;
                if crate::loaded_upkeep::UnitHandle::resolve(state, owner, id)
                    .map_err(PlacementError::World)?
                    .is_none()
                {
                    continue;
                }
                let result = crate::unit_placement_state::set(
                    state,
                    placement_definitions,
                    queries,
                    visibility,
                    effects,
                    crate::unit_placement_state::Request {
                        actor: (owner, id),
                        destination,
                        validation_flags,
                    },
                )
                .map_err(PlacementError::Unit)?;
                if result != 1 {
                    return Ok(result);
                }
            }
            Ok(1)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_step_uses_destination_first_path_without_seeding_a_selected_edge() {
        let mut wire = vec![0; 52];
        wire[0] = 1;
        let mut formation = Formation::decode(&wire, c6_core::unit::Limits::default()).unwrap();
        assert_eq!(first_step(&formation), None);
        for x in [10, 20, 30] {
            formation.last_path.push(c6_core::unit::PathNode {
                data1_raw: 0,
                data2: 1,
                flags: 0,
                x,
                y: 5,
            });
            let wanted = if formation.last_path.len() < 3 {
                10
            } else {
                20
            };
            assert_eq!(first_step(&formation), Some(PlotCoord { x: wanted, y: 5 }));
        }
    }
    #[test]
    fn movement_options_preserve_exact_native_placement_bit_mapping() {
        for (source, wanted) in [
            (0, 0),
            (4, 8),
            (0x10, 1),
            (0x20, 2),
            (0x2000, 0x80),
            (0x2034, 0x8b),
            (0x8000_0801, 0),
        ] {
            assert_eq!(placement_flags(source), wanted);
        }
    }
}
