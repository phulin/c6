//! AwardPostCombatEffects28ca910 through kill-class bonuses and winner healing.
//! Later loyalty, religion, second proposal and weariness belong to the caller.
use super::{budget::RequiresPostCombatEffects, GeneratedMembers};
use crate::{
    formation_rules,
    loaded_actions::configuration::Configuration,
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    unit_placement_state as placement, unit_sight_state as sight, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
    unit::PlotCoord,
};
type ProviderError<E> = <E as sight::Effects>::Error;

#[derive(Debug)]
pub enum Error<E> {
    Actor(Box<super::super::Error>),
    State(c6_core::loaded_state::Error),
    Combat(crate::loaded_actions::Error),
    Maximum(crate::formation_movement_cost::Error),
    Configuration(crate::loaded_actions::configuration::Error),
    Effect(E),
    Visibility(Box<crate::formation_path_state::Error>),
    Damage(Box<c6_core::unit_healing::UnitHealingError<placement::Error<E>>>),
    Missing(&'static str),
}
#[derive(Clone, Copy, Debug)]
pub struct Kill {
    pub winner: UnitReference,
    pub loser: UnitReference,
    /// Native caches the WINNER location before all GP callbacks and healing.
    pub location: PlotCoord,
    pub captured_combat: i32,
    pub religious_strength: i32,
}
#[must_use = "post-combat loyalty/religion and outer Resolve remain mandatory"]
pub struct RequiresKillEffects {
    pub members: GeneratedMembers,
    pub kill: Option<Kill>,
}

pub struct Definitions {
    binding: WireBinding,
    speeds: LinuxDefinitionRegistry,
    speed_cost: Vec<i32>,
    levels: LinuxDefinitionRegistry,
    can_earn: Vec<bool>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, &'static str> {
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|_| "kill awards resource binding")?;
        if authority.destination_binding() != state.binding() {
            return Err("kill awards state binding");
        }
        let registry =
            |table, column| collection(rules, table, column).map_err(|_| "kill awards definitions");
        let speeds = registry("GameSpeeds", "GameSpeedType")?;
        let levels = registry("CivilizationLevels", "CivilizationLevelType")?;
        let values = |registry: &LinuxDefinitionRegistry,
                      table_name,
                      column|
         -> Result<Vec<i32>, &'static str> {
            let table = rules
                .gameplay()
                .table(table_name)
                .ok_or("kill awards static table")?;
            registry
                .entries()
                .iter()
                .map(|entry| {
                    table
                        .row_by_id(entry.database_rowid)
                        .and_then(|r| r.get(column).ok())
                        .and_then(crate::game_parameters::sql_integer)
                        .ok_or("kill awards static field")
                })
                .collect()
        };
        Ok(Self {
            binding: state.binding().clone(),
            speed_cost: values(&speeds, "GameSpeeds", "CostMultiplier")?,
            can_earn: values(&levels, "CivilizationLevels", "CanEarnGreatPeople")?
                .into_iter()
                .map(|v| v != 0)
                .collect(),
            speeds,
            levels,
        })
    }
    pub(crate) fn cost(&self, game_speed: u32) -> i32 {
        self.speeds
            .find_index(game_speed)
            .and_then(|i| self.speed_cost.get(i))
            .copied()
            .unwrap_or(100)
    }
}
pub struct Inputs<'q, 'a, 'r> {
    pub definitions: &'q Definitions,
    pub classes: &'q unit_initialization_definitions::Definitions,
    pub formation: &'q formation_rules::Definitions,
    pub placement: &'q placement::Definitions<'a, 'r>,
    pub placement_queries: &'q placement::Queries<'q>,
    pub civilization_levels: &'q [(i32, u32)],
    pub configuration: &'q Configuration<'q>,
    /// Read only for native localPlayer==-1 fallback.
    pub local_observer: Option<i32>,
}
/// Required reached effects, dispatched only after actual canonical GP writes.
/// Presentation feedback still uses the caller's qualified ordered report path.
pub trait Effects: placement::Effects {
    /// GameSignals+798(owner,classIndex,newQ8,deltaQ8), then report884c48ce(owner).
    fn great_people_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        class_index: i32,
        new_q8: i32,
        delta_q8: i32,
    ) -> Result<(), ProviderError<Self>>;
    /// The helper already checked actual local visibility. Publish source
    /// 45b517d6 semantic reward output through the qualified current route.
    fn kill_reward_feedback(
        &mut self,
        state: &mut LoadedState,
        winner: UnitReference,
        location: PlotCoord,
        class_hash: u32,
        amount: i32,
    ) -> Result<(), ProviderError<Self>>;
}
#[allow(clippy::too_many_arguments)]
fn feedback<E: Effects>(
    state: &mut LoadedState,
    sight: &sight::Runtime,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    winner: UnitReference,
    location: PlotCoord,
    class_hash: u32,
    amount: i32,
) -> Result<(), Error<ProviderError<E>>> {
    // Globals.GetLocalPlayer reads the CURRENT PlayerManager alias. The
    // saved Game scalar is a distinct value and cannot replace this fact.
    let local = inputs
        .placement_queries
        .local_player
        .ok_or(Error::Missing("current local player for reward"))?;
    let viewer = if local == -1 {
        inputs
            .local_observer
            .ok_or(Error::Missing("current local observer for reward"))?
    } else {
        local
    };
    let index = location
        .y
        .wrapping_mul(state.world().map().header.width)
        .wrapping_add(location.x) as u32 as usize;
    if crate::formation_path_state::shared_plot_visible(sight, viewer, index)
        .map_err(|e| Error::Visibility(Box::new(e)))?
    {
        effects
            .kill_reward_feedback(state, winner, location, class_hash, amount)
            .map_err(Error::Effect)?;
    }
    Ok(())
}

fn strength_bonus(bonus: i32, captured_combat: i32, cost_percent: i32) -> i32 {
    let whole = bonus.wrapping_mul(captured_combat) / 100;
    adjust_speed(whole, cost_percent)
}
pub(crate) fn adjust_speed(whole: i32, cost_percent: i32) -> i32 {
    // Economic2be38b0 -> Q8 constructor -> ScaleByPercentageClamped2b9c990
    // -> AsInt1fccdb0. Both constructors wrap BEFORE the 64-bit product.
    let scaled = i64::from(whole.wrapping_shl(8)) * i64::from(cost_percent.wrapping_shl(8)) / 25600;
    (scaled.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32) >> 8
}
fn actor<E>(state: &LoadedState, unit: UnitReference) -> Result<&c6_core::unit::Unit, Error<E>> {
    super::super::actor(state, unit).map_err(|e| Error::Actor(Box::new(e)))
}
fn player<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("kill award Player"))
}
fn can_earn<E>(
    state: &LoadedState,
    inputs: &Inputs<'_, '_, '_>,
    owner: i32,
) -> Result<bool, Error<E>> {
    if !state.player_alive(owner).map_err(Error::State)? {
        return Ok(false);
    }
    let level = inputs
        .civilization_levels
        .iter()
        .find(|r| r.0 == owner)
        .ok_or(Error::Missing("kill award current civilization level"))?
        .1;
    Ok(inputs
        .definitions
        .levels
        .find_index(level)
        .and_then(|i| inputs.definitions.can_earn.get(i))
        .copied()
        .unwrap_or(false))
}
fn change_total<E: Effects>(
    state: &mut LoadedState,
    inputs: &Inputs<'_, '_, '_>,
    effects: &mut E,
    owner: i32,
    class_index: i32,
    class_hash: u32,
    delta_q8: i32,
) -> Result<(), Error<ProviderError<E>>> {
    // ChangePointsTotal checks eligibility only to reject positive increments;
    // negative changes are still allowed when the player cannot earn points.
    if !can_earn(state, inputs, owner)? && delta_q8 > 0 {
        return Ok(());
    }
    let current = player(state, owner)?
        .great_people_points
        .points_total
        .iter()
        .rev()
        .find(|r| r.0 == class_hash)
        .map(|r| r.1)
        .ok_or(Error::Missing("current GP total cell"))?;
    let next = current.wrapping_add(delta_q8).max(0);
    if !can_earn(state, inputs, owner)? && next > current {
        return Ok(());
    }
    if next == current {
        return Ok(());
    }
    state
        .social_components_mut(owner)
        .map_err(Error::State)?
        .great_people
        .points_total
        .iter_mut()
        .rev()
        .find(|r| r.0 == class_hash)
        .ok_or(Error::Missing("current GP total cell"))?
        .1 = next;
    effects
        .great_people_changed(state, owner, class_index, next, next.wrapping_sub(current))
        .map_err(Error::Effect)
}
/// Ordinary melee/unit-versus-unit entry. Original IsDead tests attacker first;
/// a dead attacker selects defender even if both happen to be dead.
pub fn apply<E: Effects>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    inputs: &Inputs<'_, '_, '_>,
    pending: RequiresPostCombatEffects,
) -> Result<RequiresKillEffects, Error<ProviderError<E>>> {
    if state.binding() != &inputs.definitions.binding {
        return Err(Error::Missing("kill award definitions binding"));
    }
    let members = pending.members;
    let attacker = members.damage.attacker;
    let defender = members.damage.defender;
    let dead = |state: &LoadedState, unit| -> Result<bool, Error<ProviderError<E>>> {
        let unit = actor(state, unit)?;
        let maximum = inputs
            .placement
            .movement
            .maximum_damage(unit)
            .map_err(Error::Maximum)?;
        Ok(unit.i_damage >= maximum)
    };
    let dead_attacker = dead(state, attacker)?;
    let (winner, loser) = if dead_attacker {
        (defender, attacker)
    } else if dead(state, defender)? {
        (attacker, defender)
    } else {
        return Ok(RequiresKillEffects {
            members,
            kill: None,
        });
    };
    let location = actor(state, winner)?.k_location;
    let captured_combat = inputs
        .formation
        .combat(state, actor(state, loser)?)
        .map_err(Error::Combat)?;
    let religious_strength = actor(state, loser)?.k_religion.i_religious_strength;
    let winner_owner = actor(state, winner)?.e_owner;
    let loser_owner = actor(state, loser)?.e_owner;
    player::<ProviderError<E>>(state, winner_owner)?;
    player::<ProviderError<E>>(state, loser_owner)?;
    for (index, entry) in inputs
        .classes
        .great_person_classes()
        .entries()
        .iter()
        .enumerate()
    {
        let class_hash = linux_type_hash(&entry.type_name);
        // The dead-defender branch reloads both Player owners each class;
        // dead-attacker retained its winner Player before entering the loop.
        let winner_owner = if dead_attacker {
            winner_owner
        } else {
            actor(state, winner)?.e_owner
        };
        let disabled = player::<ProviderError<E>>(state, winner_owner)?
            .great_people_points
            .great_person_class_no_points
            .contains(&class_hash);
        if !disabled {
            let unit = actor(state, winner)?;
            // Native getters2c29090/2c29120 return zero outside the
            // actual owned array extent; missing UnitRuntime itself is error.
            let bonus = state
                .unit_runtime(unit.e_owner, unit.id)
                .map_err(Error::State)?
                .bonus_points_per_kill
                .get(index)
                .copied()
                .unwrap_or(0);
            if !dead_attacker && bonus > 0 {
                return Err(Error::Missing(
                    "current great-person availability for flat attacker kill bonus",
                ));
            }
            if dead_attacker && bonus != 0 {
                change_total(
                    state,
                    inputs,
                    effects,
                    winner_owner,
                    index as i32,
                    class_hash,
                    bonus.wrapping_shl(8),
                )?;
                feedback(
                    state, sight, inputs, effects, winner, location, class_hash, bonus,
                )?;
            }
        }
        // This second getter is deliberately OUTSIDE the NoPoints guard.
        let unit = actor(state, winner)?;
        let bonus = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .bonus_points_per_kill_strength
            .get(index)
            .copied()
            .unwrap_or(0);
        inputs
            .configuration
            .require_binding(state.binding())
            .map_err(Error::Configuration)?;
        let speed = inputs
            .configuration
            .game_fields()
            .game_speed_type
            .ok_or(Error::Missing("current kill award game speed"))?;
        let amount = strength_bonus(bonus, captured_combat, inputs.definitions.cost(speed));
        if amount > 0 {
            // Native next calls Game.GreatPeople.CanEverEarnClass with the
            // CURRENT winner owner and availability=true. Its live individual
            // pool/retirement state is not yet owned by this adapter.
            return Err(Error::Missing(
                "current great-person availability for strength kill bonus",
            ));
        }
    }
    let heal = actor(state, winner)?.k_combat_modifers.i_post_combat_heal;
    if heal > 0 {
        let current = actor(state, winner)?;
        let actor = (current.e_owner, current.id);
        placement::change_damage(
            state,
            inputs.placement,
            inputs.placement_queries,
            sight,
            effects,
            actor,
            heal.wrapping_neg(),
        )
        .map_err(|e| Error::Damage(Box::new(e)))?;
    }
    Ok(RequiresKillEffects {
        members,
        kill: Some(Kill {
            winner,
            loser,
            location,
            captured_combat,
            religious_strength,
        }),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kill_strength_product_wraps_before_signed_division_and_speed_q8() {
        assert_eq!(strength_bonus(30, 35, 150), 15);
        assert_eq!(strength_bonus(-30, 35, 150), -15);
        assert_eq!(strength_bonus(i32::MAX, 2, 100), 0);
        assert_eq!(adjust_speed(1, 150), 1);
        assert_eq!(adjust_speed(-1, 150), -2); // AsInt floors the negative Q8.
    }
    #[test]
    fn game_speed_clamps_after_wrapped_constructors() {
        assert_eq!(adjust_speed(8_388_607, 200), 8_388_607);
        assert_eq!(adjust_speed(-8_388_608, 200), -8_388_608);
        assert_eq!(adjust_speed(i32::MAX, 100), -1);
    }
}
