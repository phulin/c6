//! Capture callbacks using the persistent stock Runtime's existing canonical
//! modifier, requirement, collection, Script and reporting owners.
use crate::{
    loaded_actions::stock_notifications::{self as stock, movement},
    unit_kill, unit_sight_state as sight,
};
use c6_core::loaded_state::LoadedState;

/// Reuses the shared immutable lifecycle inputs already used by Builder and
/// upgrade removal. Each callback loans the same mutable owners temporarily;
/// creation immediately afterwards sees all death changes on those owners.
pub struct Death<'a, 'd, 'r> {
    pub input: &'a crate::builder_improvement::death::Inputs<'d, 'r>,
    pub default_defeat_bindings: &'a [crate::unit_kill::stock::default_defeat::Binding],
}
impl<P> unit_kill::Continuations<stock::Runtime<'_, P>> for Death<'_, '_, '_> {
    type Error = unit_kill::stock::Error;
    fn removed(
        &mut self,
        state: &mut LoadedState,
        visibility: &mut sight::Runtime,
        backend: &mut stock::Runtime<'_, P>,
        actor: (i32, u32),
    ) -> Result<(), unit_kill::Error<Self::Error, movement::Error>> {
        let input = self.input;
        let bindings = self.default_defeat_bindings;
        // Native -1 remains the all-types sentinel; nonnegative values are
        // cached native definition indices, not Unit type hashes.
        let unit_types = input
            .metadata
            .death_listeners
            .iter()
            .filter_map(|listener| {
                if let unit_kill::stock::Listener::DefaultDefeatFilter {
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
        backend
            .with_modifier_owners(|backend, owners| {
                let mut influence =
                    |state: &mut LoadedState,
                     visibility: &sight::Runtime,
                     services: &crate::unit_placement_services::Runtime,
                     owner: i32| {
                        let alive = services.alive_players.as_ref().ok_or(
                            unit_kill::stock::Error::Input("current Influence alive order"),
                        )?;
                        crate::influence_map_update::update(
                            state,
                            input.influence,
                            &crate::influence_map_update::Queries {
                                alive_players: alive,
                                capabilities: Some(input.capabilities),
                                visibility,
                                movement: input.movement,
                                formation: input.formation,
                                city_range: input.cities,
                                path: None,
                                path_runtime: None,
                                strength: None,
                            },
                            owner,
                        )
                        .map_err(|e| unit_kill::stock::Error::Influence(format!("{e:?}")))
                    };
                let mut requirements = unit_kill::stock::default_defeat::OwnerWithUnits {
                    runtime: &mut owners.requirements,
                    bindings,
                    unit_types: &unit_types,
                    movement: input.movement,
                };
                let mut callbacks = unit_kill::stock::Stock::new(
                    &mut owners.effects,
                    &mut requirements,
                    input.unit_definitions,
                    input.metadata,
                )
                .map_err(unit_kill::Error::Effect)?
                .with_collections(&mut owners.collections)
                .with_unit_definitions(input.units)
                .with_sight_definitions(input.sight_definitions)
                .with_influence(&mut influence);
                unit_kill::Continuations::removed(&mut callbacks, state, visibility, backend, actor)
            })
            .map_err(unit_kill::Error::Provider)?
    }
    fn ability_lost(
        &mut self,
        state: &mut LoadedState,
        visibility: &mut sight::Runtime,
        backend: &mut stock::Runtime<'_, P>,
        actor: (i32, u32),
        ability: i32,
    ) -> Result<(), unit_kill::Error<Self::Error, movement::Error>> {
        let input = self.input;
        let bindings = self.default_defeat_bindings;
        // Native -1 remains the all-types sentinel; nonnegative values are
        // cached native definition indices, not Unit type hashes.
        let unit_types = input
            .metadata
            .death_listeners
            .iter()
            .filter_map(|listener| {
                if let unit_kill::stock::Listener::DefaultDefeatFilter {
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
        backend
            .with_modifier_owners(|backend, owners| {
                let mut requirements = unit_kill::stock::default_defeat::OwnerWithUnits {
                    runtime: &mut owners.requirements,
                    bindings,
                    unit_types: &unit_types,
                    movement: input.movement,
                };
                let mut callbacks = unit_kill::stock::Stock::new(
                    &mut owners.effects,
                    &mut requirements,
                    input.unit_definitions,
                    input.metadata,
                )
                .map_err(unit_kill::Error::Effect)?
                .with_collections(&mut owners.collections)
                .with_unit_definitions(input.units)
                .with_sight_definitions(input.sight_definitions);
                unit_kill::Continuations::ability_lost(
                    &mut callbacks,
                    state,
                    visibility,
                    backend,
                    actor,
                    ability,
                )
            })
            .map_err(unit_kill::Error::Provider)?
    }
}

use c6_core::{
    core_player::initialization::WireBinding,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

/// Actual ordered source handlers of PlayerSignals+2b8 for the admitted profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisConsumer {
    Targets,
    Behavior,
}
/// Current source-qualified stock routes. Gameplay state, human configuration
/// and city membership are reread from the Session on every callback.
pub struct Events<'a> {
    pub generation: super::generation::Inputs<'a>,
    pub binding: WireBinding,
    pub initialized_script_only: bool,
    pub analysis: Vec<AnalysisConsumer>,
    pub notice_scope: Option<stock::Scope>,
}
impl Events<'_> {
    fn check(&self, state: &LoadedState, owner: i32) -> Result<(), String> {
        if state.binding() != &self.binding {
            return Err("capture callback binding".into());
        }
        if !self.initialized_script_only
            || self.analysis != [AnalysisConsumer::Targets, AnalysisConsumer::Behavior]
            || self.notice_scope.is_none()
        {
            return Err("capture stock callback registrations".into());
        }
        if state.player_slot_status(owner).map_err(super::debug)? != 3 {
            return Err("capture nonhuman military-operation policy continuation".into());
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("capture callback owner")?;
        if !player.cities.records.entries.is_empty() {
            return Err("capture Targets city-value query continuation".into());
        }
        Ok(())
    }
}
impl<P> super::Events<stock::Runtime<'_, P>> for Events<'_> {
    fn generate(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        actor: UnitReference,
        defender: UnitReference,
        at: PlotCoord,
    ) -> Result<(), String> {
        self.generation
            .generate(state, backend, actor, defender, at)
    }
    fn preflight(
        &self,
        state: &LoadedState,
        _victim: ComponentId,
        captor: ComponentId,
    ) -> Result<(), String> {
        self.check(state, i32::from(captor.owner))
    }
    fn captured(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        new: ComponentId,
    ) -> Result<(), String> {
        self.check(state, i32::from(new.owner))?;
        // GameSignals+1440 is shared with City.CreateUnit; capture supplies an
        // actual null City. The source-qualified sole Script callback is reused.
        crate::unit_creation::signals::script_initialized(
            state,
            &mut backend.script,
            UnitReference::from_component_id(new),
            None,
        )
        .map_err(super::debug)
    }
    fn analysis(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock::Runtime<'_, P>,
        owner: i32,
        _units: &[u32],
    ) -> Result<(), String> {
        self.check(state, owner)?;
        // Targets21ab060 computes a local value over current Cities; this
        // admitted empty collection reaches no City getters or writes.
        // Behavior26e4fc0 handles AI_EVENT_UNIT_CAPTURED(2), whose nonempty
        // military plan calls StartMilitaryOperation2147a50. IsHuman at2147a72
        // returns NULL before evaluators/allocation/RNG; no policy is selected.
        Ok(())
    }
    fn notification(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        victim_owner: i32,
        captor: ComponentId,
        new: ComponentId,
        at: PlotCoord,
    ) -> Result<(), String> {
        self.check(state, i32::from(captor.owner))?;
        backend
            .publish_notice(
                state,
                stock::Notice::CivilianCaptured {
                    owner: victim_owner,
                    captor_owner: i32::from(captor.owner),
                    unit: c6_core::notifications::ComponentId {
                        owner: new.owner,
                        component_type: new.component_type,
                        id: new.id,
                    },
                    location: [at.x, at.y],
                },
            )
            .map_err(super::debug)
    }
    fn processed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        new: ComponentId,
    ) -> Result<(), String> {
        self.check(state, i32::from(new.owner))?;
        // Same native SetProcessedTurn and qualified current snapshot route as
        // upgrade creation; this retains callback-visible state before reporting.
        backend.upgrade_processed_turn(state, new)
    }
}
