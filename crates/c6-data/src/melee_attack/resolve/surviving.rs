//! Linux inner melee continuation when both combatants survive. This returns
//! only the outer Resolve boundary; it does not complete an Attack action.
use super::{RequiresCombatAftermath, RequiresOuterResolution};
use crate::{formation_movement_cost as cost, loaded_upkeep as upkeep};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    movement::{MoveBudget, MovementPoints, ZocValue},
    unit::PlotCoord,
};
type ProviderError<E> = <E as upkeep::Effects>::Error;

#[derive(Clone, Copy, Debug)]
pub struct Status {
    pub attacker: UnitReference,
    pub defender: UnitReference,
    pub location: PlotCoord,
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub status_id: u32,
}
/// Ordered attacker/defender statuses, then the multiplayer notification when
/// configured. Providers preserve reached gameplay subscribers and RNG.
pub trait Effects: upkeep::Effects + upkeep::OperationReports<Error = ProviderError<Self>> {
    fn both_alive_statuses(
        &mut self,
        state: &mut LoadedState,
        request: Status,
    ) -> Result<(), ProviderError<Self>>;
    /// Delegate to `combat_movement_cost` with current admitted definitions,
    /// placement queries and sight; no imported cost answer is accepted.
    fn combat_movement_cost(
        &mut self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<i32, ProviderError<Self>>;
    /// Native HasVisibleEnemyUnit's return and output flag, in that order.
    /// Delegate to the current Formation's canonical visibility query.
    fn visible_enemy_at(
        &mut self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<(bool, bool), ProviderError<Self>>;
    /// Called only when positive advance permission is not blocked by the
    /// visible-enemy return/output pair. Read the attacker's current owner.
    fn enemy_defensible_district(
        &mut self,
        state: &LoadedState,
        attacker: UnitReference,
        location: PlotCoord,
    ) -> Result<bool, ProviderError<Self>>;
}
#[derive(Debug)]
pub enum Error<E> {
    Actor(super::super::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Layer(c6_core::skip_turn::placement::Error),
    MovementFormation(c6_core::movement_execution::FormationMovementError<upkeep::Error<E>>),
    Maximum(cost::Error),
    Effect(E),
    Clear(Box<upkeep::Error<E>>),
    Movement(c6_core::movement_execution::UnitMovementError<upkeep::Error<E>>),
    Sync(c6_core::formation_upkeep::FormationUpkeepError<upkeep::Error<E>>),
    DeadAttacker,
    DefenderDeath,
    MissingFormation,
    MissingNotificationFacts,
    Advance,
    UnsupportedAdvanceUnit(UnitReference),
    Placement(Box<crate::unit_placement_state::Error<E>>),
    PlacementRejected(i32),
}

/// Native 28d6ccb -> 28d6808 -> 28d69de -> 28d6a24. A failed
/// continuation retains preceding callbacks and mutations in the owned state.
pub fn finish_both_alive<
    E: Effects
        + crate::unit_placement_state::Effects
        + upkeep::Effects<Error = <E as crate::unit_sight_state::Effects>::Error>,
>(
    state: &mut LoadedState,
    maximum: &cost::Definitions<'_>,
    inputs: &crate::unit_movement_state::Inputs<'_>,
    sight: &mut crate::unit_sight_state::Runtime,
    effects: &mut E,
    pending: RequiresCombatAftermath,
    status_id: u32,
) -> Result<RequiresOuterResolution, Error<ProviderError<E>>> {
    let members = pending.members;
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let dead = |state: &LoadedState, actor| {
        let current = super::super::actor(state, actor).map_err(Error::Actor)?;
        Ok::<_, Error<ProviderError<E>>>(
            current.i_damage >= maximum.maximum_damage(current).map_err(Error::Maximum)?,
        )
    };
    if dead(state, attacker)? {
        return Err(Error::DeadAttacker);
    }
    if dead(state, defender)? {
        return Err(Error::DefenderDeath);
    }
    effects
        .both_alive_statuses(
            state,
            Status {
                attacker,
                defender,
                location: members.location,
                attacker_damage: members.damage.attacker_received,
                defender_damage: members.damage.defender_received,
                status_id,
            },
        )
        .map_err(Error::Effect)?;
    finish_attacker(state, maximum, inputs, sight, effects, members, None)
}

/// Continue the living attacker after the defender-death callbacks and proposal
/// destruction. Headless combat generation retains the native false advanced
/// visualization field, so advance uses the Formation flags3 Set path.
#[allow(clippy::too_many_arguments)]
pub fn finish_winner<
    E: Effects
        + crate::unit_placement_state::Effects
        + upkeep::Effects<Error = <E as crate::unit_sight_state::Effects>::Error>,
>(
    state: &mut LoadedState,
    maximum: &cost::Definitions<'_>,
    inputs: &crate::unit_movement_state::Inputs<'_>,
    definitions: &crate::unit_placement_state::Definitions<'_, '_>,
    queries: &crate::unit_placement_state::Queries<'_>,
    sight: &mut crate::unit_sight_state::Runtime,
    effects: &mut E,
    pending: RequiresCombatAftermath,
) -> Result<RequiresOuterResolution, Error<ProviderError<E>>> {
    finish_attacker(
        state,
        maximum,
        inputs,
        sight,
        effects,
        pending.members,
        Some((definitions, queries)),
    )
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn finish_attacker<
    E: Effects
        + crate::unit_placement_state::Effects
        + upkeep::Effects<Error = <E as crate::unit_sight_state::Effects>::Error>,
>(
    state: &mut LoadedState,
    maximum: &cost::Definitions<'_>,
    inputs: &crate::unit_movement_state::Inputs<'_>,
    sight: &mut crate::unit_sight_state::Runtime,
    effects: &mut E,
    members: super::super::execution::GeneratedMembers,
    advance: Option<(
        &crate::unit_placement_state::Definitions<'_, '_>,
        &crate::unit_placement_state::Queries<'_>,
    )>,
) -> Result<RequiresOuterResolution, Error<ProviderError<E>>> {
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let dead = |state: &LoadedState, actor| {
        let current = super::super::actor(state, actor).map_err(Error::Actor)?;
        Ok::<_, Error<ProviderError<E>>>(
            current.i_damage >= maximum.maximum_damage(current).map_err(Error::Maximum)?,
        )
    };
    upkeep::clear_unit_operations(state, attacker.player(), attacker.id, effects)
        .map_err(|e| Error::Clear(Box::new(e)))?;
    upkeep::clear_unit_operations(state, defender.player(), defender.id, effects)
        .map_err(|e| Error::Clear(Box::new(e)))?;
    if dead(state, attacker)? {
        return Ok(RequiresOuterResolution { members });
    }
    // Pinned Linux IsSuicide (28e09a0) is `xor eax,eax; ret`.
    let current = super::super::actor(state, attacker).map_err(Error::Actor)?;
    let formation = state
        .unit_runtime(current.e_owner, current.id)
        .map_err(Error::State)?
        .formation
        .ok_or(Error::MissingFormation)?;
    let movement_cost = effects
        .combat_movement_cost(state, sight, attacker, members.location)
        .map_err(Error::Effect)?;
    // Generated permission does not establish an advance. Native 28d699f
    // blocks it only when BOTH the HasVisibleEnemyUnit return and output are
    // true; otherwise the fresh enemy-district query is still mandatory.
    if members.attacker_advances {
        let (enemy, combat) = effects
            .visible_enemy_at(state, sight, attacker, members.location)
            .map_err(Error::Effect)?;
        if !(enemy && combat)
            && !effects
                .enemy_defensible_district(state, attacker, members.location)
                .map_err(Error::Effect)?
        {
            let Some((definitions, queries)) = advance else {
                return Err(Error::Advance);
            };
            upkeep::change_formation_moves(
                state,
                formation.owner,
                formation.requested_id,
                inputs,
                effects,
                movement_cost.wrapping_neg(),
            )
            .map_err(Error::MovementFormation)?;
            // Native scans the retained destination IDs backwards. The ordinary
            // uncaptured combat branch skips delayed-death objects. Civilian
            // capture and nonempty live stacks remain explicit continuations.
            let map = state.world().map();
            let index = (members.location.y as u32)
                .wrapping_mul(map.units.width)
                .wrapping_add(members.location.x as u32);
            let layer = c6_core::skip_turn::placement::formation_layer(
                state.world(),
                state
                    .world()
                    .player(formation.owner)
                    .map_err(Error::World)?
                    .formation(formation.requested_id)
                    .map_err(Error::World)?
                    .ok_or(Error::MissingFormation)?
                    .record,
            )
            .map_err(Error::Layer)?
            .unwrap_or(u32::MAX);
            let targets = map
                .units
                .layers
                .iter()
                .find(|v| v.layer_id == layer)
                .ok_or(Error::MissingFormation)?
                .cells
                .iter()
                .find(|v| v.plot_index == index)
                .map(|v| v.units.clone())
                .unwrap_or_default();
            for target in targets.into_iter().rev() {
                let Some(found) = state.world().unit(target).map_err(Error::World)? else {
                    continue;
                };
                if !found.record.b_delayed_death {
                    return Err(Error::UnsupportedAdvanceUnit(target));
                }
            }
            let record = state
                .world()
                .player(formation.owner)
                .map_err(Error::World)?
                .formation(formation.requested_id)
                .map_err(Error::World)?
                .ok_or(Error::MissingFormation)?
                .record;
            let owner = record.owner;
            let count = record.unit_ids.len();
            for index in 0..count {
                let record = state
                    .world()
                    .player(formation.owner)
                    .map_err(Error::World)?
                    .formation(formation.requested_id)
                    .map_err(Error::World)?
                    .ok_or(Error::MissingFormation)?
                    .record;
                let id = *record.unit_ids.get(index).ok_or(Error::MissingFormation)?;
                if upkeep::UnitHandle::resolve(state, owner, id)
                    .map_err(Error::World)?
                    .is_none()
                {
                    continue;
                }
                let result = crate::unit_placement_state::set(
                    state,
                    definitions,
                    queries,
                    sight,
                    effects,
                    crate::unit_placement_state::Request {
                        actor: (owner, id),
                        destination: members.location,
                        validation_flags: 1,
                    },
                )
                .map_err(|e| Error::Placement(Box::new(e)))?;
                if result != 1 {
                    return Err(Error::PlacementRejected(result));
                }
            }
            upkeep::complete_formation_movement(
                state,
                formation.owner,
                formation.requested_id,
                effects,
            )
            .map_err(|e| Error::Clear(Box::new(e)))?;
            return eligible(state, attacker, members);
        }
    }
    upkeep::change_unit_moves(
        state,
        attacker.player(),
        attacker.id,
        inputs,
        effects,
        movement_cost.wrapping_neg(),
    )
    .map_err(Error::Movement)?;
    // Native retains this Formation across ChangeMovesRemaining subscribers.
    upkeep::update_formation(
        state,
        formation.owner,
        formation.requested_id,
        inputs,
        effects,
    )
    .map_err(Error::Sync)?;
    eligible(state, attacker, members)
}
fn eligible<E>(
    state: &mut LoadedState,
    attacker: UnitReference,
    members: super::super::execution::GeneratedMembers,
) -> Result<RequiresOuterResolution, Error<E>> {
    let can_heal = super::super::actor(state, attacker)
        .map_err(Error::Actor)?
        .k_health_modifiers
        .b_can_heal_after_action;
    state
        .unit_mut(attacker.player(), attacker.id)
        .map_err(Error::State)?
        .b_eligible_to_heal = can_heal;
    Ok(RequiresOuterResolution { members })
}

/// Actual MovementCost overload reached at 28d6927. Base/max moves fold all
/// current Formation members; domain -1 is the native Formation getter's member
/// argument, even though its caller supplies combat-plot water domain.
#[allow(clippy::too_many_arguments)]
pub fn combat_movement_cost(
    state: &LoadedState,
    definitions: &cost::Definitions<'_>,
    placement: &crate::movement_placement_state::Context<'_, '_, '_>,
    inputs: &crate::unit_movement_state::Inputs<'_>,
    sight: &crate::unit_sight_state::Runtime,
    attacker: UnitReference,
    destination: PlotCoord,
) -> Result<i32, crate::formation_path_state::Error> {
    use crate::{formation_path_state::Error as QueryError, unit_movement_state as moves};
    let unit =
        super::super::actor(state, attacker).map_err(|_| QueryError::Missing("combat attacker"))?;
    let attachment = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(QueryError::State)?
        .formation
        .ok_or(QueryError::Missing("combat Formation"))?;
    let player = state
        .world()
        .player(attachment.owner)
        .map_err(QueryError::World)?;
    let formation = player
        .formation(attachment.requested_id)
        .map_err(QueryError::World)?
        .ok_or(QueryError::Missing("combat Formation"))?
        .record;
    let fold = |base: bool| -> Result<i32, QueryError> {
        let mut minimum = 9001i32;
        let mut escort_maximum = 0;
        for &id in &formation.unit_ids {
            let Some(found) = player.unit(id).map_err(QueryError::World)? else {
                continue;
            };
            let member = found.record;
            let n = if base {
                moves::unit_base_moves_with_can_embark(state, member, -1, inputs, || {
                    placement.can_embark(member)
                })
                .map_err(QueryError::UnitMoves)?
            } else {
                moves::unit_max_moves(state, member, inputs).map_err(QueryError::UnitMoves)?
            };
            minimum = minimum.min(n);
            if member.k_movement_modifiers.b_escort_mobility {
                escort_maximum = escort_maximum.max(n);
            }
        }
        Ok(minimum.max(escort_maximum))
    };
    let (_, target) = c6_core::movement_world::plot_at(state.world().map(), destination)
        .map_err(|e| QueryError::Cost(cost::Error::Movement(e)))?;
    let _domain = i32::from(placement.water(target).map_err(QueryError::Placement)?) - 1;
    let budget = MoveBudget {
        base_moves: fold(true)?,
        max_moves: MovementPoints::integer(fold(false)?),
        remaining: MovementPoints(0),
        exit_moves: MovementPoints(0),
    };
    cost::movement_cost(
        state,
        definitions,
        placement,
        formation,
        unit.k_location,
        destination,
        Some(destination),
        budget,
        ZocValue::Query,
        0,
        Some(sight),
    )
    .map(|v| v.final_cost().0)
    .map_err(QueryError::Cost)
}

/// Reporting.Notification.Send asks AcceptsNotifications (2d2f000) before
/// allocating or sending anything. Resolve the fresh defender owner first,
/// then preserve manager -> uninitializing -> current human-configuration order.
/// Returning true still requires the real creation/publication continuation.
pub fn accepts_combat_notification(
    state: &LoadedState,
    initialized: bool,
    uninitializing: bool,
    defender: UnitReference,
) -> Result<bool, super::super::Error> {
    let owner = super::super::actor(state, defender)?.e_owner;
    if !initialized || uninitializing {
        return Ok(false);
    }
    Ok(state
        .player_slot_status(owner)
        .map_err(super::super::Error::State)?
        == 3)
}

/// Source multiplayer gate followed by the real notification recipient guard.
/// False performs no allocation. True requires notification creation and Send;
/// it is not itself successful delivery. Facts are actual manager/runtime data.
pub fn combat_notification_required(
    state: &LoadedState,
    configuration: &crate::loaded_actions::configuration::RawGameConfiguration,
    manager_facts: Option<(bool, bool)>,
    defender: UnitReference,
) -> Result<bool, Error<std::convert::Infallible>> {
    if !configuration.is_any_multiplayer() {
        return Ok(false);
    }
    let (initialized, uninitializing) = manager_facts.ok_or(Error::MissingNotificationFacts)?;
    accepts_combat_notification(state, initialized, uninitializing, defender).map_err(Error::Actor)
}
