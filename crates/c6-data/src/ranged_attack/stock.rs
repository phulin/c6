//! Public stock composition of ranged queries, resolution and Unit death.
//! Retain this adapter's referenced definitions and the stock Runtime between
//! requests. No callback or legality answers are supplied by the fixture layer.
use super::{error, Damage as DamageProvider, Error};
use crate::{
    loaded_actions::{self as actions, move_to::combat::Combat, stock_notifications::Runtime},
    loaded_upkeep, melee_attack as melee,
    unit_placement_services::Services,
    unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

/// Current stock damage/death continuation, including live influence dispatch.
pub struct Damage<'a> {
    pub metadata: &'a crate::unit_kill::stock::Metadata,
    pub definitions: &'a crate::unit_initialization_definitions::Definitions,
    pub units: &'a c6_core::movement_world::DefinitionTable<
        'a,
        Option<c6_core::skip_turn::placement::UnitProperties>,
    >,
    pub influence: &'a mut crate::unit_kill::stock::Influence<'a>,
}
impl<E> DamageProvider<E, melee::backend::requirements::Runtime> for Damage<'_>
where
    E: placement::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = <E as sight::Effects>::Error>,
    <E as sight::Effects>::Error: std::fmt::Debug,
{
    type Error = Error;
    fn apply(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        runtime: &mut melee::backend::EffectRuntime,
        requirements: &mut melee::backend::requirements::Runtime,
        definitions: &placement::Definitions<'_, '_>,
        queries: &placement::Queries<'_>,
        pending: melee::execution::RequiresResolve,
    ) -> Result<melee::resolve::RequiresExperience, Error> {
        let mut callbacks = crate::unit_kill::stock::Stock::new(
            runtime,
            requirements,
            self.definitions,
            self.metadata,
        )
        .map_err(|e| error("stock death", e))?
        .with_unit_definitions(self.units)
        .with_sight_definitions(definitions.sight)
        .with_influence(self.influence);
        super::death::apply_with_continuations(
            state,
            sight,
            effects,
            &mut callbacks,
            definitions,
            queries,
            pending,
        )
        .map_err(|e| error("stock death", e))
    }
}
/// Retained stock death composition using the canonical creation collections and
/// live default-defeat requirements alongside the combat modifier owner.
pub struct RetainedDamage<'a, 'd> {
    pub damage: Damage<'a>,
    pub default_defeat: &'a [crate::unit_kill::stock::default_defeat::Binding],
    pub movement: &'a crate::formation_movement_cost::Definitions<'d>,
}
impl DamageProvider<Runtime<'_>, melee::backend::requirements::Runtime> for RetainedDamage<'_, '_> {
    type Error = Error;
    fn apply(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut Runtime<'_>,
        runtime: &mut melee::backend::EffectRuntime,
        requirements: &mut melee::backend::requirements::Runtime,
        definitions: &placement::Definitions<'_, '_>,
        queries: &placement::Queries<'_>,
        pending: melee::execution::RequiresResolve,
    ) -> Result<melee::resolve::RequiresExperience, Error> {
        let unit_types = self
            .damage
            .metadata
            .death_listeners
            .iter()
            .filter_map(|listener| {
                if let crate::unit_kill::stock::Listener::DefaultDefeatFilter {
                    receiver,
                    unit_type,
                    ..
                } = *listener
                {
                    Some((receiver, unit_type as u32))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        effects
            .with_combat_collections(state, |effects, state, collections| {
                let mut requirements = crate::unit_kill::stock::default_defeat::OwnerWithUnits {
                    runtime: requirements,
                    bindings: self.default_defeat,
                    unit_types: &unit_types,
                    movement: self.movement,
                };
                let mut callbacks = crate::unit_kill::stock::Stock::new(
                    runtime,
                    &mut requirements,
                    self.damage.definitions,
                    self.damage.metadata,
                )
                .map_err(|e| error("retained stock death", e))?
                .with_collections(collections)
                .with_unit_definitions(self.damage.units)
                .with_sight_definitions(definitions.sight)
                .with_influence(self.damage.influence);
                super::death::apply_with_continuations(
                    state,
                    sight,
                    effects,
                    &mut callbacks,
                    definitions,
                    queries,
                    pending,
                )
                .map_err(|e| error("retained stock death", e))
            })
            .map_err(|e| error("retained death collections", e))?
    }
}

/// Attach this to `move_to::combat::WithCombat`. All CanStart/Test/execute work
/// uses the existing exact ranged implementation on the current stock owner.
pub struct Ranged<'a, 'd, 'r, D> {
    pub targets: &'a melee::target_selection::Definitions,
    pub inputs: &'a melee::transition::Inputs<'a, 'd, 'r>,
    pub operations: &'a actions::Inputs<'a, 'd>,
    pub damage: D,
}
fn unit(actor: (i32, u32)) -> Result<UnitReference, Error> {
    let owner = actor.0.try_into().map_err(|e| error("actor owner", e))?;
    Ok(UnitReference::from_component_id(ComponentId {
        owner,
        component_type: 1,
        id: actor.1,
    }))
}
impl<'a, D> Combat<Runtime<'a>> for Ranged<'_, '_, '_, D>
where
    D: DamageProvider<Runtime<'a>, melee::backend::requirements::Runtime>,
{
    type Error = Error;
    fn test(
        &mut self,
        _: &mut LoadedState,
        _: &sight::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: &Runtime<'a>,
    ) -> Result<bool, Error> {
        Err(error(
            "dispatch",
            "melee combat not composed by ranged adapter",
        ))
    }
    fn attack(
        &mut self,
        _: &mut LoadedState,
        _: &mut sight::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: &mut Runtime<'a>,
    ) -> Result<bool, Error> {
        Err(error(
            "dispatch",
            "melee combat not composed by ranged adapter",
        ))
    }
    fn range_can_start(
        &mut self,
        state: &LoadedState,
        sight: &sight::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        visible_only: bool,
        effects: &Runtime<'a>,
    ) -> Result<Option<bool>, Error> {
        let target = melee::target_selection::Inputs {
            combat: self.inputs.generation.formation,
            movement: self.inputs.movement.definitions,
            sight,
            services: effects
                .placement_service_runtime()
                .map_err(|e| error("target services", e))?,
            districts: Some(self.inputs.generation.maximum_damage),
        };
        super::query::can_start(
            state,
            self.targets,
            &target,
            self.operations,
            sight,
            unit(actor)?,
            destination,
            visible_only,
        )
        .map(Some)
    }
    fn range_test(
        &mut self,
        state: &mut LoadedState,
        sight: &sight::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &Runtime<'a>,
    ) -> Result<Option<bool>, Error> {
        let target = melee::target_selection::Inputs {
            combat: self.inputs.generation.formation,
            movement: self.inputs.movement.definitions,
            sight,
            services: effects
                .placement_service_runtime()
                .map_err(|e| error("target services", e))?,
            districts: Some(self.inputs.generation.maximum_damage),
        };
        super::query::test(state, self.targets, &target, unit(actor)?, destination).map(Some)
    }
    fn range_attack(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
        effects: &mut Runtime<'a>,
    ) -> Result<Option<bool>, Error> {
        effects
            .ranged_attack(
                state,
                sight,
                &mut self.damage,
                self.inputs,
                unit(actor)?,
                destination,
            )
            .map(Some)
    }
}
