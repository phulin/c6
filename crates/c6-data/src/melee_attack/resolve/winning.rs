//! Defender-death prefix of ordinary melee. Capture proposals are created by
//! the caller on the current combat backend between this report and feedback.
use super::{DeathReports, RequiresCombatAftermath, UnitDeath};
use c6_core::{
    loaded_state::LoadedState,
    unit_placement_transition::{PlacementPayload, PlacementReport},
};

pub trait Effects: DeathReports {
    /// Native winner status, loser status, then UnitDied / UnitKilled signal.
    /// Presentation may become semantic output; reached gameplay subscribers
    /// and RNG must still execute before proposal destruction.
    fn defender_death_notifications(
        &mut self,
        state: &mut LoadedState,
        request: UnitDeath,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Actor(super::super::Error),
    Maximum(crate::formation_movement_cost::Error),
    Effect(E),
    LivingDefender,
    RelicCreation {
        owner: i32,
        location: c6_core::unit::PlotCoord,
        source: u32,
    },
}
/// 28d6a46: relic query precedes UnitDefeated; the fresh capture proposal follows
/// the report's synchronous consumers, using the same retained Unit identities.
pub fn report<E: Effects>(
    state: &mut LoadedState,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    effects: &mut E,
    pending: &RequiresCombatAftermath,
) -> Result<(), Error<E::Error>> {
    let d = &pending.members.damage;
    let dead = super::super::actor(state, d.defender).map_err(Error::Actor)?;
    if dead.i_damage < maximum.maximum_damage(dead).map_err(Error::Maximum)? {
        return Err(Error::LivingDefender);
    }
    let source = dead.k_religion.e_relic_source as u32;
    if source == 0xb8cd57a7 {
        return Err(Error::RelicCreation {
            owner: dead.e_owner,
            location: dead.k_location,
            source,
        });
    }
    let identity = c6_core::unit::ComponentId {
        owner: dead.e_owner as i16,
        component_type: 1,
        id: dead.id,
    };
    let other = super::super::actor(state, d.attacker).map_err(Error::Actor)?;
    let other = c6_core::unit::ComponentId {
        owner: other.e_owner as i16,
        component_type: 1,
        id: other.id,
    };
    effects
        .placement_report(
            state,
            PlacementReport {
                event_hash: 0x754cbd1b,
                identity,
                payload: PlacementPayload::OtherIdentity(other),
            },
        )
        .map_err(Error::Effect)
}
pub fn notify<E: Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    pending: &RequiresCombatAftermath,
    status_id: u32,
) -> Result<(), Error<E::Error>> {
    let m = &pending.members;
    effects
        .defender_death_notifications(
            state,
            UnitDeath {
                dead: m.damage.defender,
                opponent: m.damage.attacker,
                location: m.location,
                dead_damage: m.damage.defender_received,
                opponent_damage: m.damage.attacker_received,
                status_id,
            },
        )
        .map_err(Error::Effect)
}
