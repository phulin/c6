//! Stock NPC deactivation on the persistent action owner. Native
//! policy caches are omitted; current control/knowledge writes and ordering are
//! retained. Decisions remain external; unimplemented gameplay continuations
//! fail explicitly after their actual preceding stores.
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
/// Tactical.OnTurnDeactivate2196060 can displace or delete an overstacked
/// ordinary unit. With fewer than two resolved layered candidates its friendly
/// count cannot reach two, regardless of the pure definition/type predicates.
/// This guard deliberately leaves the reached positive placement branch explicit.
fn tactical_end(state: &LoadedState, owner: i32) -> Result<(), Error> {
    if state
        .player_slot_status(owner)
        .map_err(|_| Error::Unsupported("NPC tactical control role"))?
        == 3
    {
        return Ok(());
    }
    let units = state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("NPC tactical player"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("NPC tactical unit roster"))?;
    for slot in units {
        let unit = &slot.value;
        if unit.b_delayed_death || unit.k_location.x == -9999 || unit.k_location.y == -9999 {
            continue;
        }
        let candidates = c6_core::skip_turn::placement::layered_units(state.world(), unit)
            .map_err(|_| Error::Unsupported("NPC tactical current layered units"))?;
        let mut resolved = 0usize;
        for &candidate in candidates {
            if state
                .world()
                .unit(candidate)
                .map_err(|_| Error::Unsupported("NPC tactical current candidate"))?
                .is_some()
            {
                resolved += 1;
            }
        }
        if resolved >= 2 {
            return Err(Error::Unsupported(
                "NPC tactical stacked-unit placement continuation",
            ));
        }
    }
    Ok(())
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
    visibility: &'i std::cell::RefCell<crate::unit_sight_state::Runtime>,
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
                    self.backend.npc_unit_decisions(
                        state,
                        owner,
                        super::npc_unit_decisions::Stage::Homeland,
                        current,
                    )?;
                    // DoTurn2135330/2195f60 returns true after native
                    // RecruitUnits/AssignMoves. Those calls choose controller
                    // allocations, formation changes and unit commands; they
                    // belong to external policy, including its selection RNG.
                    // An explicit EndTurn does not authorize native choices.
                    self.backend
                        .reached
                        .push("external NPC unit action selection omitted");
                }
            }
            Kind::Tactical => {
                if current == self.inputs.configured(kind)? {
                    self.backend.npc_unit_decisions(
                        state,
                        owner,
                        super::npc_unit_decisions::Stage::Tactical,
                        current,
                    )?;
                    // DoTurn2135330/2195f60 returns true after native
                    // RecruitUnits/AssignMoves. Those calls choose controller
                    // allocations, formation changes and unit commands; they
                    // belong to external policy, including its selection RNG.
                    // An explicit EndTurn does not authorize native choices.
                    self.backend
                        .reached
                        .push("external NPC unit action selection omitted");
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
                // Clear(empty) has no destructor. Each current Unit then gets
                // SetAiPlanner(null)21b9e20: a plain pointer store, with no
                // report, command or RNG. These planner links and the matching
                // activation21b9280 are omitted external-policy storage (same
                // contract as npc_leaves::Planning), not gameplay owners.
                Ok(())
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
            Kind::Tactical => tactical_end(state, owner),
            // +228 invokes Targets::UpdateKnowledgeBase21aa9b0, which
            // refreshes visible Unit/camp records without IncrementTurn.
            Kind::Targets => super::human_activation::knowledge::refresh_visible_units(
                state,
                owner,
                self.units,
                &*self
                    .visibility
                    .try_borrow()
                    .map_err(|_| Error::Unsupported("NPC knowledge sight borrow"))?,
                self.backend
                    .movement()
                    .ok()
                    .map(|m| m.definitions.formation),
            ),
            _ => Err(Error::Unsupported("unknown NPC end consumer")),
        }
    }
    fn cleanup(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        loaded::npc_deactivation::cleanup_controllers(state, owner)
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
    /// Retain current metadata and gameplay owners; only the completed
    /// continuation cursor is renewed at the new activation boundary.
    pub(super) fn prepare_npc_deactivation_activation(
        &mut self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<(), Error> {
        if let Some(bound) = self
            .npc_deactivation
            .iter_mut()
            .find(|b| b.inputs.owner == owner)
        {
            bound.frame.prepare_new_activation(state)?;
        }
        Ok(())
    }
    pub fn bind_npc_deactivation(
        &mut self,
        state: &LoadedState,
        inputs: Inputs,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC deactivation backend binding"))?;
        if inputs.binding != *state.binding() {
            return Err(Error::Unsupported("NPC deactivation input binding differs"));
        }
        let existing = self
            .npc_deactivation
            .iter()
            .position(|b| b.inputs.owner == inputs.owner);
        if existing.is_some_and(|at| !self.npc_deactivation[at].frame.is_complete()) {
            return Err(Error::Unsupported("NPC deactivation owner already pending"));
        }
        let frame = Frame::new(
            state,
            inputs.owner,
            true,
            inputs.readiness.clone(),
            inputs.priorities.clone(),
            inputs.end.clone(),
        )?;
        let bound = State { inputs, frame };
        if let Some(at) = existing {
            self.npc_deactivation[at] = bound;
        } else {
            self.npc_deactivation.push(bound);
        }
        Ok(())
    }
    pub fn deactivate_npc_control(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        owner: i32,
        visibility: &std::cell::RefCell<crate::unit_sight_state::Runtime>,
        units: &crate::registry::LinuxDefinitionRegistry,
    ) -> Result<Progress, Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC deactivation backend binding"))?;
        let at = self
            .npc_deactivation
            .iter()
            .position(|b| b.inputs.owner == owner)
            .ok_or(Error::Unsupported(
                "current NPC deactivation owner bindings",
            ))?;
        // Lease exactly this owner's real frame; other owners and their order
        // stay intact throughout the callback pass, including an error/pause.
        let mut owned = self.npc_deactivation.remove(at);
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
        self.npc_deactivation.insert(at, owned);
        result
    }
}
