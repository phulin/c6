//! Ordinary melee resolution over the current owned Units. Partial failures keep
//! the native prefix; generated damage is never reported as a resolved attack.
pub mod budget;
pub mod completion;
pub mod kill_awards;
pub mod post_kill;
pub mod post_yields;
pub mod unit_era;

use super::execution::{GeneratedMembers, RequiresResolve};
use crate::{
    loaded_upkeep, unit_kill, unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference};
type EffectsError<E> = <E as sight::Effects>::Error;
#[derive(Debug)]
pub enum Error<E, C = E> {
    Damage(c6_core::unit_healing::UnitHealingError<placement::Error<E>>),
    Actor(super::Error),
    Combat(crate::loaded_actions::Error),
    Missing(&'static str),
    Maximum(crate::formation_movement_cost::Error),
    Kill(Box<unit_kill::Error<C, E>>),
}
/// HP changes and delayed death have completed. XP, joint-war awards and the
/// remaining combat aftermath must still run before an action can complete.
#[must_use]
pub struct RequiresExperience {
    pub members: GeneratedMembers,
}
#[allow(clippy::too_many_arguments)]
fn change_damage<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    unit: UnitReference,
    delta: i32,
    killed_by: i32,
) -> Result<(), Error<EffectsError<E>>>
where
    E: placement::Effects
        + unit_kill::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<E>>,
{
    placement::change_damage(
        state,
        definitions,
        queries,
        sight,
        effects,
        (unit.player(), unit.id),
        delta,
    )
    .map_err(Error::Damage)?;
    // Report subscribers may have changed damage, max HP, placement or queues.
    let damage = super::actor(state, unit).map_err(Error::Actor)?.i_damage;
    let maximum = definitions
        .movement
        .maximum_damage(super::actor(state, unit).map_err(Error::Actor)?)
        .map_err(Error::Maximum)?;
    if damage >= maximum {
        unit_kill::kill(
            state,
            sight,
            effects,
            definitions,
            queries,
            (unit.player(), unit.id),
            true,
            killed_by,
        )
        .map_err(|e| Error::Kill(Box::new(e)))?;
    }
    Ok(())
}
/// ResolveMeleeUnitVsUnitCombat28d64ec: defender damage/kill precedes attacker
/// damage/kill. Both retained Units remain available for subsequent XP queries.
pub fn apply_damage<E>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    formation: &crate::formation_rules::Definitions,
    pending: RequiresResolve,
) -> Result<RequiresExperience, Error<EffectsError<E>>>
where
    E: placement::Effects
        + unit_kill::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<E>>,
{
    let members = pending.members;
    let d = &members.damage;
    super::actor(state, d.attacker).map_err(Error::Actor)?;
    let defender = super::actor(state, d.defender).map_err(Error::Actor)?;
    let location = if state
        .world()
        .map()
        .plot_at(members.location.x, members.location.y)
        .is_some()
    {
        members.location
    } else {
        defender.k_location
    };
    if state
        .world()
        .map()
        .plot_at(location.x, location.y)
        .is_none()
    {
        return Err(Error::Missing("Resolve current combat plot"));
    }
    if !(formation.combat(state, defender).map_err(Error::Combat)? > 0
        || formation.ranged(state, defender).map_err(Error::Combat)? > 0
        || formation.bombard(state, defender).map_err(Error::Combat)? > 0)
    {
        return Err(Error::Missing("noncombat defender Resolve branch"));
    }
    change_damage(
        state,
        sight,
        effects,
        definitions,
        queries,
        d.defender,
        d.defender_received,
        d.attacker.player(),
    )?;
    change_damage(
        state,
        sight,
        effects,
        definitions,
        queries,
        d.attacker,
        d.attacker_received,
        d.defender.player(),
    )?;
    Ok(RequiresExperience { members })
}

/// Split-provider variant for one persistent combat backend: sight/report
/// providers and lifecycle continuations borrow disjoint fields of that owner.
#[allow(clippy::too_many_arguments)]
pub fn apply_damage_with_continuations<P, C>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    providers: &mut P,
    continuations: &mut C,
    definitions: &placement::Definitions<'_, '_>,
    queries: &placement::Queries<'_>,
    formation: &crate::formation_rules::Definitions,
    pending: RequiresResolve,
) -> Result<RequiresExperience, Error<EffectsError<P>, C::Error>>
where
    P: placement::Effects + loaded_upkeep::FormationRestoreEffects<Error = EffectsError<P>>,
    C: unit_kill::Continuations<P>,
{
    let members = pending.members;
    let d = &members.damage;
    super::actor(state, d.attacker).map_err(Error::Actor)?;
    let defender = super::actor(state, d.defender).map_err(Error::Actor)?;
    let location = if state
        .world()
        .map()
        .plot_at(members.location.x, members.location.y)
        .is_some()
    {
        members.location
    } else {
        defender.k_location
    };
    if state
        .world()
        .map()
        .plot_at(location.x, location.y)
        .is_none()
    {
        return Err(Error::Missing("Resolve current combat plot"));
    }
    if !(formation.combat(state, defender).map_err(Error::Combat)? > 0
        || formation.ranged(state, defender).map_err(Error::Combat)? > 0
        || formation.bombard(state, defender).map_err(Error::Combat)? > 0)
    {
        return Err(Error::Missing("noncombat defender Resolve branch"));
    }
    for (unit, delta, killed_by) in [
        (d.defender, d.defender_received, d.attacker.player()),
        (d.attacker, d.attacker_received, d.defender.player()),
    ] {
        placement::change_damage(
            state,
            definitions,
            queries,
            sight,
            providers,
            (unit.player(), unit.id),
            delta,
        )
        .map_err(Error::Damage)?;
        let damage = super::actor(state, unit).map_err(Error::Actor)?.i_damage;
        let maximum = definitions
            .movement
            .maximum_damage(super::actor(state, unit).map_err(Error::Actor)?)
            .map_err(Error::Maximum)?;
        if damage >= maximum {
            unit_kill::kill_with_continuations(
                state,
                sight,
                providers,
                continuations,
                definitions,
                queries,
                (unit.player(), unit.id),
                true,
                killed_by,
            )
            .map_err(|e| Error::Kill(Box::new(e)))?;
        }
    }
    Ok(RequiresExperience { members })
}

#[derive(Debug)]
pub enum ExperienceError<E> {
    Change(super::experience::ChangeError<E>),
    Query(super::experience::Error),
    /// Partner discovery is computed; closest-current-unit XP sharing remains
    /// mandatory on this positive branch and is never silently discarded.
    JointWarAward {
        receiving_player: i32,
        partners: super::experience::JointWarPartners,
        amount: i32,
    },
}
#[must_use]
pub struct RequiresCombatAftermath {
    pub members: GeneratedMembers,
}
/// Resolve applies defender XP before attacker XP, then both joint-war awards.
#[allow(clippy::too_many_arguments)]
pub fn apply_experience<E: super::experience::Reports>(
    state: &mut LoadedState,
    definitions: &super::experience::Definitions,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    levels: &[(i32, u32)],
    cached_majors: &[i32],
    effects: &mut E,
    pending: RequiresExperience,
) -> Result<RequiresCombatAftermath, ExperienceError<E::Error>> {
    let members = pending.members;
    let d = &members.damage;
    super::experience::change(
        state,
        definitions,
        maximum,
        levels,
        d.defender,
        members.defender_experience,
        effects,
    )
    .map_err(ExperienceError::Change)?;
    super::experience::change(
        state,
        definitions,
        maximum,
        levels,
        d.attacker,
        members.attacker_experience,
        effects,
    )
    .map_err(ExperienceError::Change)?;
    for (receiving_player, enemy, amount) in [
        (
            d.defender.player(),
            d.attacker.player(),
            members.defender_experience,
        ),
        (
            d.attacker.player(),
            d.defender.player(),
            members.attacker_experience,
        ),
    ] {
        let partners = super::experience::joint_war_partners(state, levels, cached_majors, enemy)
            .map_err(ExperienceError::Query)?;
        if !partners.owners.is_empty() {
            return Err(ExperienceError::JointWarAward {
                receiving_player,
                partners,
                amount,
            });
        }
    }
    Ok(RequiresCombatAftermath { members })
}

/// Semantic effects reached after a combatant dies. Notification delivery owns
/// its real pending gameplay messages; localization/render serialization is not
/// an input to this transition.
pub trait DeathReports: loaded_upkeep::OperationReports {
    /// First status, UnitDied notification, Game.Signals.UnitKilled (+14a0),
    /// then the opponent status, in that order. A headless provider may replace
    /// presentation output with semantic events, but must retain every reached
    /// gameplay subscriber and RNG draw from the signal.
    fn unit_death_notifications(
        &mut self,
        state: &mut LoadedState,
        request: UnitDeath,
    ) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct UnitDeath {
    pub dead: UnitReference,
    pub opponent: UnitReference,
    pub location: c6_core::unit::PlotCoord,
    pub dead_damage: i32,
    pub opponent_damage: i32,
    pub status_id: u32,
}
#[derive(Debug)]
pub enum AftermathError<E> {
    Actor(super::Error),
    Maximum(crate::formation_movement_cost::Error),
    Effect(E),
    Clear(Box<loaded_upkeep::Error<E>>),
    /// The live attacker follows the separate suicide/movement/healing path.
    SurvivingAttacker,
    RelicCreation {
        owner: i32,
        location: c6_core::unit::PlotCoord,
        source: u32,
    },
}
/// The inner melee routine returned. The outer ResolveCombat budget changes,
/// post-combat effects and war weariness are still mandatory.
#[must_use]
pub struct RequiresOuterResolution {
    pub members: GeneratedMembers,
}

/// ResolveMeleeUnitVsUnitCombat's dead-attacker branch. Generated advance
/// permission never bypasses the current survival checks. Callback failures
/// retain their exact prefix; this is not a completed Attack action.
pub fn finish_dead_attacker<E: DeathReports>(
    state: &mut LoadedState,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    effects: &mut E,
    pending: RequiresCombatAftermath,
    status_id: u32,
) -> Result<RequiresOuterResolution, AftermathError<E::Error>> {
    use c6_core::unit_placement_transition::{PlacementPayload, PlacementReport};
    let members = pending.members;
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let current = super::actor(state, attacker).map_err(AftermathError::Actor)?;
    if current.i_damage
        < maximum
            .maximum_damage(current)
            .map_err(AftermathError::Maximum)?
    {
        return Err(AftermathError::SurvivingAttacker);
    }
    // IsRelicUponDeath reads the source enum; the historical bool is not this
    // getter. Only this source creates a relic in this branch.
    let source = current.k_religion.e_relic_source as u32;
    if source == 0xb8cd57a7 {
        return Err(AftermathError::RelicCreation {
            owner: current.e_owner,
            location: current.k_location,
            source,
        });
    }
    let dead = super::actor(state, attacker).map_err(AftermathError::Actor)?;
    let dead_id = c6_core::unit::ComponentId {
        owner: dead.e_owner as i16,
        component_type: 1,
        id: dead.id,
    };
    let other = super::actor(state, defender).map_err(AftermathError::Actor)?;
    let other_id = c6_core::unit::ComponentId {
        owner: other.e_owner as i16,
        component_type: 1,
        id: other.id,
    };
    effects
        .placement_report(
            state,
            PlacementReport {
                event_hash: 0x754cbd1b,
                identity: dead_id,
                payload: PlacementPayload::OtherIdentity(other_id),
            },
        )
        .map_err(AftermathError::Effect)?;
    effects
        .unit_death_notifications(
            state,
            UnitDeath {
                dead: attacker,
                opponent: defender,
                location: members.location,
                dead_damage: members.damage.attacker_received,
                opponent_damage: members.damage.defender_received,
                status_id,
            },
        )
        .map_err(AftermathError::Effect)?;
    loaded_upkeep::clear_unit_operations(state, attacker.player(), attacker.id, effects)
        .map_err(|e| AftermathError::Clear(Box::new(e)))?;
    loaded_upkeep::clear_unit_operations(state, defender.player(), defender.id, effects)
        .map_err(|e| AftermathError::Clear(Box::new(e)))?;
    let current = super::actor(state, attacker).map_err(AftermathError::Actor)?;
    if current.i_damage
        < maximum
            .maximum_damage(current)
            .map_err(AftermathError::Maximum)?
    {
        return Err(AftermathError::SurvivingAttacker);
    }
    Ok(RequiresOuterResolution { members })
}
