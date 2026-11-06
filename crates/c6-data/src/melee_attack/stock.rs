//! Public stock composition of ordinary melee queries, resolution and Unit death.
//! Retain this adapter's referenced definitions and the stock Runtime between
//! requests. No callback or legality answers are supplied by the fixture layer.
use super::transition::{Damage as DamageProvider, Error};
fn error(stage: &str, e: impl std::fmt::Debug) -> Error {
    Error {
        stage: super::transition::Stage::Damage,
        detail: format!("{stage}: {e:?}"),
    }
}
use crate::{
    loaded_actions::{move_to::combat::Combat, stock_notifications::Runtime},
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
        formation: &crate::formation_rules::Definitions,
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
        .with_influence(self.influence);
        melee::resolve::apply_damage_with_continuations(
            state,
            sight,
            effects,
            &mut callbacks,
            definitions,
            queries,
            formation,
            pending,
        )
        .map_err(|e| error("stock death", e))
    }
}

/// Pure current-state movement queries retained by the combat owner.
pub trait MovementQueries {
    fn cost(
        &self,
        state: &LoadedState,
        sight: &sight::Runtime,
        attacker: UnitReference,
        at: PlotCoord,
    ) -> Result<i32, crate::loaded_actions::stock_notifications::movement::Error>;
    fn visible(
        &self,
        state: &LoadedState,
        sight: &sight::Runtime,
        attacker: UnitReference,
        at: PlotCoord,
    ) -> Result<(bool, bool), crate::loaded_actions::stock_notifications::movement::Error>;
    fn district(
        &self,
        state: &LoadedState,
        attacker: UnitReference,
        at: PlotCoord,
    ) -> Result<bool, crate::loaded_actions::stock_notifications::movement::Error>;
}
pub struct Movement<'a, 'd, 'r> {
    pub definitions: &'a crate::formation_path_state::Definitions<'d, 'r>,
    pub queries: &'a placement::Queries<'d>,
}
impl MovementQueries for Movement<'_, '_, '_> {
    fn cost(
        &self,
        s: &LoadedState,
        v: &sight::Runtime,
        u: UnitReference,
        p: PlotCoord,
    ) -> Result<i32, crate::loaded_actions::stock_notifications::movement::Error> {
        let placement = self
            .queries
            .placement_context(s, &self.definitions.placement)
            .map_err(query_error)?
            .with_combat_definitions(&self.definitions.formation)
            .with_district_definitions(&self.definitions.movement);
        melee::resolve::surviving::combat_movement_cost(
            s,
            &self.definitions.movement,
            &placement,
            &crate::unit_movement_state::Inputs {
                definitions: &self.definitions.units,
            },
            v,
            u,
            p,
        )
        .map_err(query_error)
    }
    fn visible(
        &self,
        s: &LoadedState,
        v: &sight::Runtime,
        u: UnitReference,
        p: PlotCoord,
    ) -> Result<(bool, bool), crate::loaded_actions::stock_notifications::movement::Error> {
        let unit = melee::actor(s, u).map_err(query_error)?;
        let attachment = s
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(query_error)?
            .formation
            .ok_or_else(|| query_error("combat Formation"))?;
        let formation = s
            .world()
            .player(attachment.owner)
            .and_then(|p| p.formation(attachment.requested_id))
            .map_err(query_error)?
            .ok_or_else(|| query_error("combat Formation"))?
            .record;
        let layer = c6_core::skip_turn::placement::formation_layer(s.world(), formation)
            .map_err(query_error)?
            .unwrap_or(u32::MAX);
        crate::formation_path_state::has_visible_enemy_unit(
            s,
            &self.definitions.formation,
            v,
            formation,
            layer,
            p,
        )
        .map_err(query_error)
    }
    fn district(
        &self,
        s: &LoadedState,
        u: UnitReference,
        p: PlotCoord,
    ) -> Result<bool, crate::loaded_actions::stock_notifications::movement::Error> {
        melee::target_selection::enemy_defensible_district_with_definitions(
            s,
            Some(&self.definitions.movement),
            u.player(),
            p,
        )
        .map(|v| v.is_some())
        .map_err(query_error)
    }
}
fn query_error(
    e: impl std::fmt::Debug,
) -> crate::loaded_actions::stock_notifications::movement::Error {
    crate::loaded_actions::stock_notifications::movement::Error::Requirement(format!("{e:?}"))
}
/// Ordinary queued MoveTo attack using canonical current state and modifiers.
pub struct Melee<'a, 'd, 'r, D> {
    pub targets: &'a melee::target_selection::Definitions,
    pub inputs: &'a melee::transition::Inputs<'a, 'd, 'r>,
    pub movement: &'a Movement<'a, 'd, 'r>,
    pub damage: D,
}
fn unit(actor: (i32, u32)) -> Result<UnitReference, Error> {
    Ok(UnitReference::from_component_id(ComponentId {
        owner: actor.0.try_into().map_err(|e| error("owner", e))?,
        component_type: 1,
        id: actor.1,
    }))
}
impl<'a, D> Combat<Runtime<'a>> for Melee<'_, '_, '_, D>
where
    D: DamageProvider<Runtime<'a>, melee::backend::requirements::Runtime>,
{
    type Error = Error;
    fn test(
        &mut self,
        s: &mut LoadedState,
        v: &sight::Runtime,
        u: (i32, u32),
        p: PlotCoord,
        e: &Runtime<'a>,
    ) -> Result<bool, Error> {
        let inputs = melee::target_selection::Inputs {
            combat: self.inputs.generation.formation,
            movement: self.inputs.movement.definitions,
            sight: v,
            services: e
                .placement_service_runtime()
                .map_err(|e| error("services", e))?,
            districts: Some(self.inputs.generation.maximum_damage),
        };
        let placement = self
            .movement
            .queries
            .placement_context(s, &self.movement.definitions.placement)
            .map_err(|e| error("placement", e))?
            .with_district_definitions(self.inputs.generation.maximum_damage)
            .with_combat_definitions(self.inputs.generation.formation);
        melee::target_selection::test_attack_into_with_placement(
            self.targets,
            &inputs,
            &placement,
            unit(u)?,
            p,
        )
        .map(|v| matches!(v, melee::target_selection::Outcome::RequiresAttackInto(_)))
        .map_err(|e| error("test", e))
    }
    fn attack(
        &mut self,
        s: &mut LoadedState,
        v: &mut sight::Runtime,
        u: (i32, u32),
        p: PlotCoord,
        e: &mut Runtime<'a>,
    ) -> Result<bool, Error> {
        e.melee_attack(s, v, &mut self.damage, self.inputs, unit(u)?, p)
    }
}
