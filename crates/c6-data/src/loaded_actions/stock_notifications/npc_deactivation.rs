//! Stock empty-asset NPC deactivation on the persistent action owner. Native
//! policy caches are omitted; current control/knowledge writes and ordering are
//! retained. Reached nonempty policy/gameplay continuations remain explicit.
use super::Runtime;
pub use c6_core::game_update::Error;
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_components::RetainedPlayer,
    loaded_state::{game_update as loaded, LoadedState},
};
pub use loaded::npc_activation::Kind;
pub use loaded::npc_deactivation::{Frame, Progress};
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNpcExternalPoliciesNoAddedConsumers,
}
pub struct Inputs {
    binding: WireBinding,
    pub owner: i32,
    pub readiness: Vec<Kind>,
    pub priorities: Vec<Kind>,
    pub end: Vec<Kind>,
    /// Initialized plain/configured priority fields, not readiness answers.
    pub configured: Vec<(Kind, i32)>,
    pub civilization_level: u32,
    pub planning_instance_count: Option<usize>,
}
impl Inputs {
    #[allow(clippy::too_many_arguments)]
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        owner: i32,
        readiness: Vec<Kind>,
        priorities: Vec<Kind>,
        end: Vec<Kind>,
        configured: Vec<(Kind, i32)>,
        civilization_level: u32,
        planning_instance_count: Option<usize>,
        _: Scope,
    ) -> Result<Self, Error> {
        if state.binding() != binding {
            return Err(Error::Unsupported("NPC deactivation input binding"));
        }
        player(state, owner)?;
        Ok(Self {
            binding: binding.clone(),
            owner,
            readiness,
            priorities,
            end,
            configured,
            civilization_level,
            planning_instance_count,
        })
    }
    fn configured(&self, kind: Kind) -> Result<i32, Error> {
        self.configured
            .iter()
            .find(|r| r.0 == kind)
            .map(|r| r.1)
            .ok_or(Error::Unsupported(
                "current NPC deactivation configured priority",
            ))
    }
}
pub(crate) struct State {
    inputs: Inputs,
    frame: Frame,
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current NPC deactivation player"))
}
fn no_units(state: &LoadedState, owner: i32) -> Result<(), Error> {
    if state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("current NPC units"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("current NPC Unit roster"))?
        .is_empty()
    {
        Ok(())
    } else {
        Err(Error::Unsupported("NPC recruitment/action continuation"))
    }
}
fn no_cities(state: &LoadedState, owner: i32) -> Result<(), Error> {
    let p = player(state, owner)?;
    if p.cities.records.entries.is_empty()
        && p.ai_city_manager
            .as_ref()
            .ok_or(Error::Unsupported("current NPC CityManager"))?
            .city_ais
            .entries
            .is_empty()
    {
        Ok(())
    } else {
        Err(Error::Unsupported(
            "NPC CityBuild deactivation continuation",
        ))
    }
}
fn no_trees(state: &LoadedState, owner: i32) -> Result<(), Error> {
    if player(state, owner)?
        .ai_behavior_tree
        .as_ref()
        .ok_or(Error::Unsupported("current NPC behavior trees"))?
        .behavior_trees
        .entries
        .is_empty()
    {
        Ok(())
    } else {
        Err(Error::Unsupported(
            "NPC executable behavior tree continuation",
        ))
    }
}
struct Effects<'s, 'a, 'i, P> {
    backend: &'s mut Runtime<'a, P>,
    inputs: &'i Inputs,
    visibility: &'i crate::unit_sight_state::Runtime,
    units: &'i crate::registry::LinuxDefinitionRegistry,
}
impl<P> loaded::npc_deactivation::Effects for Effects<'_, '_, '_, P> {
    fn ready(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
    ) -> Result<bool, Error> {
        let current = loaded::human_activation::current_priority(state, owner)?;
        match kind {
            Kind::Targets => {
                if player(state, owner)?
                    .ai_targets
                    .as_ref()
                    .ok_or(Error::Unsupported("current NPC Targets"))?
                    .new_war_started
                    .0
                    != 0
                {
                    return Err(Error::Unsupported("NPC new war operation continuation"));
                }
            }
            Kind::City => no_cities(state, owner)?,
            Kind::BehaviorTree => no_trees(state, owner)?,
            Kind::Religious => {
                if current == self.inputs.configured(kind)?
                    && self.inputs.civilization_level == 0x253718b0
                {
                    return Err(Error::Unsupported(
                        "major NPC Religious deactivation continuation",
                    ));
                }
                // Nonmajor returns before contracts, purchase or selection.
                // Its cleared desire entry is an omitted native policy score.
            }
            Kind::Homeland => {
                let additional = player(state, owner)?
                    .ai_homeland
                    .as_ref()
                    .ok_or(Error::Unsupported("current NPC Homeland"))?
                    .additional_priority;
                if current == self.inputs.configured(kind)? || current == additional {
                    no_units(state, owner)?;
                    no_cities(state, owner)?;
                }
            }
            Kind::Tactical => {
                if current == self.inputs.configured(kind)? {
                    no_units(state, owner)?;
                    no_cities(state, owner)?;
                }
            }
            _ => return Err(Error::Unsupported("unknown NPC readiness consumer")),
        }
        Ok(true)
    }
    fn priority_end(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
    ) -> Result<(), Error> {
        let id = match kind {
            Kind::Homeland => {
                player(state, owner)?
                    .ai_homeland
                    .as_ref()
                    .ok_or(Error::Unsupported("current NPC Homeland"))?
                    .control_instance_id
            }
            Kind::Tactical => {
                player(state, owner)?
                    .ai_tactical
                    .as_ref()
                    .ok_or(Error::Unsupported("current NPC Tactical"))?
                    .control_instance_id
            }
            Kind::BehaviorTree => return no_trees(state, owner),
            _ => return Err(Error::Unsupported("unknown NPC priority-end consumer")),
        };
        loaded::npc_deactivation::release_controller(state, owner, id)
    }
    fn end(
        &mut self,
        state: &mut LoadedState,
        _: &mut loaded::Runtime,
        owner: i32,
        kind: Kind,
        _: i32,
    ) -> Result<(), Error> {
        match kind {
            Kind::Planning => {
                if self.inputs.planning_instance_count != Some(0) {
                    return Err(Error::Unsupported("current NPC planning instance teardown"));
                }
                // Clear(empty) has no destructor. SetAiPlanner only occurs in
                // the current Unit loop; no Unit pointer is silently dropped.
                no_units(state, owner)
            }
            Kind::City => no_cities(state, owner),
            Kind::Research => {
                let techs = &player(state, owner)?.techs;
                if techs
                    .research_queue_hashes
                    .first()
                    .is_none_or(|v| *v == u32::MAX)
                {
                    return Err(Error::ExternalDecisionRequired(
                        "technology selection during NPC deactivation",
                    ));
                }
                // UpdateTreeSearchInfo only refreshes native recommendation
                // data; the already selected current research is preserved.
                Ok(())
            }
            Kind::Tactical => no_units(state, owner),
            Kind::Targets => super::human_activation::knowledge::observe_visible_units(
                state,
                owner,
                self.units,
                self.visibility,
            ),
            _ => Err(Error::Unsupported("unknown NPC end consumer")),
        }
    }
    fn cleanup(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        let control = player(state, owner)?
            .ai_control
            .as_ref()
            .ok_or(Error::Unsupported("current NPC Control Manager"))?;
        if !control.controls.instances.is_empty() {
            return Err(Error::Unsupported("current Control ShouldDestroy fields"));
        }
        Ok(())
    }
    fn turn_complete(
        &mut self,
        _: &mut LoadedState,
        _: &mut loaded::Runtime,
        _: i32,
    ) -> Result<(), Error> {
        // Native TurnComplete is profiling timer work only; processing depth
        // has already changed in the Core owner before this boundary.
        self.backend
            .reached
            .push("stock NPC control deactivation complete");
        Ok(())
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_npc_deactivation(
        &mut self,
        state: &LoadedState,
        inputs: Inputs,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC deactivation backend binding"))?;
        if inputs.binding != *state.binding()
            || self
                .npc_deactivation
                .as_ref()
                .is_some_and(|b| !b.frame.is_complete())
        {
            return Err(Error::Unsupported(
                "NPC deactivation inputs already bound/differ",
            ));
        }
        let frame = Frame::new(
            state,
            inputs.owner,
            true,
            inputs.readiness.clone(),
            inputs.priorities.clone(),
            inputs.end.clone(),
        )?;
        self.npc_deactivation = Some(State { inputs, frame });
        Ok(())
    }
    pub fn deactivate_npc_control(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
        visibility: &crate::unit_sight_state::Runtime,
        units: &crate::registry::LinuxDefinitionRegistry,
    ) -> Result<Progress, Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC deactivation backend binding"))?;
        let mut owned = self
            .npc_deactivation
            .take()
            .ok_or(Error::Unsupported("current NPC deactivation bindings"))?;
        if owned.inputs.owner != owner {
            self.npc_deactivation = Some(owned);
            return Err(Error::Unsupported("NPC deactivation current owner differs"));
        }
        let result = owned.frame.advance(
            state,
            runtime,
            &mut Effects {
                backend: self,
                inputs: &owned.inputs,
                visibility,
                units,
            },
        );
        self.npc_deactivation = Some(owned);
        result
    }
}
