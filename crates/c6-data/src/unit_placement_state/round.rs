//! Reuse current Placement damage/operations services for end-of-round units.
use super::*;
use c6_core::unit_healing::{self, UnitHealingContext};

pub trait Effects: super::Effects {
    fn set_unit_activity(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        activity: i32,
    ) -> Result<(), EffectsError<Self>>;
    /// Actual Game unit-healed signal and every reached gameplay/RNG consumer.
    fn unit_healed(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        signal: UnitHealedSignal,
    ) -> Result<(), EffectsError<Self>>;
}

pub struct Inputs<'a, 'd, 'r> {
    pub definitions: &'a Definitions<'d, 'r>,
    pub queries: &'a Queries<'d>,
}
fn context<'a, 'd, 'r, E: super::Effects>(
    state: &'a mut LoadedState,
    inputs: &'a Inputs<'a, 'd, 'r>,
    runtime: &'a mut sight::Runtime,
    effects: &'a mut E,
    actor: (i32, u32),
) -> Context<'a, 'd, 'r, E> {
    Context {
        sight: sight::Context {
            state,
            runtime,
            effects,
            sight_definitions: inputs.definitions.sight,
            actor,
        },
        definitions: inputs.definitions,
        queries: inputs.queries,
        creation_tags: None,
    }
}
/// Actual UnitTakeMovementDamage(current location); this still calls the
/// existing ChangeDamage(0) path on inert improvements, rather than omitting it.
pub fn improvement_damage<E: super::Effects>(
    state: &mut LoadedState,
    inputs: &Inputs<'_, '_, '_>,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    actor: (i32, u32),
) -> Result<i32, c6_core::improvement_movement::Error<Error<EffectsError<E>>>> {
    inputs
        .definitions
        .validate_binding(state)
        .map_err(|e| c6_core::improvement_movement::Error::Context(Error::Query(e)))?;
    let mut c = context(state, inputs, runtime, effects, actor);
    let old = c
        .actor()
        .map_err(c6_core::improvement_movement::Error::Context)?
        .k_location;
    c6_core::improvement_movement::take_movement_damage(Some(&mut c), old)
}
struct Healing<'a, 'd, 'r, E> {
    context: Context<'a, 'd, 'r, E>,
    world: HealingWorldState,
    rules: HealingRules,
}
impl<E: Effects> UnitHealingContext for Healing<'_, '_, '_, E> {
    type Error = Error<EffectsError<E>>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.context.actor()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.context.edit()
    }
    fn healing_world_state(&self) -> Result<HealingWorldState, Self::Error> {
        Ok(self.world)
    }
    fn healing_rules(&self) -> Result<HealingRules, Self::Error> {
        Ok(self.rules)
    }
    fn preflight_healing_effects(&self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn combat_max_hit_points(&self) -> Result<i32, Self::Error> {
        self.context.combat_max_hit_points()
    }
    fn maximum_damage_for_report(&self) -> Result<i32, Self::Error> {
        self.context.maximum_damage_for_report()
    }
    fn send_damage_changed(&mut self, r: DamageChangedReport) -> Result<(), Self::Error> {
        self.context.send_damage_changed(r)
    }
    fn clear_current_operations(&mut self) -> Result<(), Self::Error> {
        self.context.clear_current_operations()
    }
    fn dispatch_unit_healed(&mut self, signal: UnitHealedSignal) -> Result<(), Self::Error> {
        let c = &mut self.context.sight;
        c.effects
            .unit_healed(c.state, c.actor, signal)
            .map_err(Error::Effect)
    }
}
/// Full shared HealUnit amount/damage/report/operation/healed-signal continuation
/// for the concrete current world inputs built by round_units::healing_world.
pub fn heal<E: Effects>(
    state: &mut LoadedState,
    inputs: &Inputs<'_, '_, '_>,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    actor: (i32, u32),
    world: HealingWorldState,
    rules: HealingRules,
) -> Result<unit_healing::UnitHealingOutcome, unit_healing::UnitHealingError<Error<EffectsError<E>>>>
{
    inputs
        .definitions
        .validate_binding(state)
        .map_err(|e| unit_healing::UnitHealingError::InputsUnavailable(Error::Query(e)))?;
    let context = context(state, inputs, runtime, effects, actor);
    unit_healing::heal_unit(Some(&mut Healing {
        context,
        world,
        rules,
    }))
}
