//! Concrete stock nonhuman subscriber guards. This is a partial activation
//! backend: an unimplemented reached gameplay/RNG continuation is an error,
//! including on an owner with no cities or units.
use super::{human_activation::knowledge, inactive_control, maintenance, npc_activation, Runtime};
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_components::RetainedPlayer,
    loaded_state::{game_update::human_activation::current_priority, LoadedState},
};
pub use npc_activation::Kind;

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNpcExternalPoliciesNoAddedPlanningConsumers,
}
/// Current native registration order and unsaved Religious request inputs.
/// These inputs do not qualify an unknown callback as a successful no-op.
pub struct Inputs {
    binding: WireBinding,
    pub owner: i32,
    pub off_turn: Vec<Kind>,
    pub priorities: Vec<(Kind, i32)>,
    pub pending_pantheon: Option<bool>,
    pub religious_contract_count: Option<usize>,
    /// Unsaved CultureAi_GreatWorks request vector, required at Culture.
    pub great_work_request_count: Option<usize>,
    callbacks: Vec<Kind>,
}
impl Inputs {
    #[allow(clippy::too_many_arguments)]
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        owner: i32,
        off_turn: Vec<Kind>,
        priorities: Vec<(Kind, i32)>,
        pending_pantheon: Option<bool>,
        religious_contract_count: Option<usize>,
        _: Scope,
    ) -> Result<Self, Error> {
        if state.binding() != binding {
            return Err(Error::Unsupported("NPC leaves initial binding"));
        }
        player(state, owner)?;
        let callbacks = priorities.iter().map(|&(kind, _)| kind).collect();
        Ok(Self {
            binding: binding.clone(),
            owner,
            off_turn,
            priorities,
            pending_pantheon,
            religious_contract_count,
            great_work_request_count: None,
            callbacks,
        })
    }
    /// Bind the current unsaved GreatWorks request vector length. A missing
    /// count remains unknown until ExecuteRequests is actually reached.
    pub fn with_great_work_request_count(mut self, count: Option<usize>) -> Self {
        self.great_work_request_count = count;
        self
    }
    fn check(&self, state: &LoadedState, owner: i32) -> Result<(), Error> {
        if state.binding() != &self.binding || owner != self.owner {
            return Err(Error::Unsupported("NPC leaves current binding/owner"));
        }
        player(state, owner)?;
        Ok(())
    }
    fn reached(
        &self,
        state: &LoadedState,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<bool, Error> {
        self.check(state, owner)?;
        if matches!(kind, Kind::City | Kind::BehaviorTree) {
            return Ok(full);
        }
        let configured = self
            .priorities
            .iter()
            .find(|&&(k, _)| k == kind)
            .ok_or(Error::Unsupported("NPC callback registration"))?
            .1;
        Ok(callback_enabled(
            kind,
            current_priority(state, owner)?,
            configured,
            full,
        ))
    }
}
fn npc_kind(kind: super::human_activation::Kind) -> Kind {
    use super::human_activation::Kind as Human;
    match kind {
        Human::City => Kind::City,
        Human::BehaviorTree => Kind::BehaviorTree,
        Human::Planning => Kind::Planning,
        Human::Diplomatic => Kind::Diplomatic,
        Human::Religious => Kind::Religious,
        Human::Targets => Kind::Targets,
        Human::GrandStrategic => Kind::GrandStrategic,
        Human::Governors => Kind::Governors,
        Human::Research => Kind::Research,
        Human::Economic => Kind::Economic,
        Human::Espionage => Kind::Espionage,
        Human::Homeland => Kind::Homeland,
        Human::Culture => Kind::Culture,
        Human::Military => Kind::Military,
        Human::Tactical => Kind::Tactical,
    }
}
fn callback_enabled(kind: Kind, priority: i32, configured: i32, full: bool) -> bool {
    priority == configured && (full || matches!(kind, Kind::Planning | Kind::Governors))
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("NPC leaves current player"))
}
fn no_cities(state: &LoadedState, owner: i32) -> Result<(), Error> {
    if !player(state, owner)?.cities.records.entries.is_empty() {
        return Err(Error::Unsupported("NPC City activation callbacks"));
    }
    Ok(())
}
fn no_units(state: &LoadedState, owner: i32) -> Result<(), Error> {
    if !state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("NPC current unit owner"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("NPC current unit roster"))?
        .is_empty()
    {
        return Err(Error::Unsupported(
            "NPC nonempty unit gameplay continuation",
        ));
    }
    Ok(())
}
fn empty_free_cities(state: &LoadedState) -> Result<(), Error> {
    // DoTurnActivate294e1f0 reaches OnTurnActivate only within this vector.
    if !state.retained().free_cities.instances.is_empty() {
        return Err(Error::Unsupported("NPC FreeCities OnTurnActivate"));
    }
    Ok(())
}
fn major<P>(backend: &Runtime<'_, P>, owner: i32) -> Result<bool, Error> {
    let level = backend
        .player_services
        .as_ref()
        .and_then(|services| services.players.iter().find(|p| p.owner == owner))
        .and_then(|p| p.civilization_level)
        .ok_or(Error::Unsupported("NPC current civilization level"))?;
    Ok(level == c6_core::diplomacy_meeting::MAJOR_LEVEL)
}
fn empty_congress_choices<P>(
    state: &LoadedState,
    backend: &Runtime<'_, P>,
    owner: i32,
) -> Result<(), Error> {
    // 2c4b1c0 visits alive majors; a candidate reaches FindDiscussionOptions
    // only if it has met the owner, or is the owner itself.
    for p in &state.retained().players {
        let other = p.header.owner;
        if !state
            .player_alive(other)
            .map_err(|_| Error::Unsupported("NPC Congress alive player"))?
            || !major(backend, other)?
        {
            continue;
        }
        if other == owner
            || state
                .game_relations()
                .map_err(|_| Error::Unsupported("NPC Congress formal relations"))?
                .has_met(other, owner)
                .map_err(|_| Error::Unsupported("NPC Congress formal contact"))?
        {
            return Err(Error::Unsupported(
                "NPC Congress proposal decision continuation",
            ));
        }
    }
    Ok(())
}
pub struct Leaves<'i> {
    inputs: &'i Inputs,
    definitions: &'i maintenance::Definitions,
    visibility: &'i crate::unit_sight_state::Runtime,
    off_turn: Option<&'i mut inactive_control::Inputs>,
}
impl<'i> Leaves<'i> {
    pub fn new(
        inputs: &'i Inputs,
        definitions: &'i maintenance::Definitions,
        visibility: &'i crate::unit_sight_state::Runtime,
    ) -> Self {
        Self {
            inputs,
            definitions,
            visibility,
            off_turn: None,
        }
    }
    /// Reuse the initialized off-turn owner across policy pauses. This is the
    /// same canonical runtime input used by inactive Control.DoTurn; do not
    /// replace it with a freshly decoded readiness snapshot on resume.
    pub fn with_off_turn(mut self, inputs: &'i mut inactive_control::Inputs) -> Self {
        self.off_turn = Some(inputs);
        self
    }
}
impl<P> npc_activation::Leaves<P> for Leaves<'_> {
    fn callbacks(&self) -> &[Kind] {
        &self.inputs.callbacks
    }
    fn research_reached(
        &self,
        state: &LoadedState,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<bool, Error> {
        if !matches!(kind, Kind::Research | Kind::Culture) {
            return Err(Error::Unsupported("NPC research boundary kind"));
        }
        self.inputs.reached(state, owner, kind, full)
    }
    fn wait(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
    ) -> Result<(), Error> {
        self.inputs.check(state, owner)?;
        backend
            .check(state)
            .map_err(|_| Error::Unsupported("NPC leaves backend binding"))?;
        // The Rust backend executes synchronous gameplay work; joining native
        // worker threads and profiling signals has no independent gameplay job.
        Ok(())
    }
    fn off_turn(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
    ) -> Result<bool, Error> {
        self.inputs.check(state, owner)?;
        let inputs = self
            .off_turn
            .as_deref_mut()
            .ok_or(Error::Unsupported("NPC current off-turn owner"))?;
        if inputs.owner != owner
            || inputs.off_turn.len() != self.inputs.off_turn.len()
            || inputs
                .off_turn
                .iter()
                .zip(&self.inputs.off_turn)
                .any(|(&human, &npc)| npc_kind(human) != npc)
        {
            return Err(Error::Unsupported("NPC off-turn registration differs"));
        }
        inactive_control::dispatch_off_turn(state, backend, inputs)
    }
    fn callback(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        kind: Kind,
        full: bool,
    ) -> Result<(), Error> {
        if !self.inputs.reached(state, owner, kind, full)? {
            return Ok(());
        }
        let p = player(state, owner)?;
        match kind {
            Kind::City => no_cities(state, owner),
            Kind::FreeCities => empty_free_cities(state),
            Kind::BehaviorTree => {
                let b = p
                    .ai_behavior_tree
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC BehaviorTree owner"))?;
                if !b.behavior_trees.entries.is_empty() {
                    return Err(Error::Unsupported("NPC existing BehaviorTree Reset"));
                }
                Ok(())
            }
            Kind::Targets => {
                let targets = p
                    .ai_targets
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Targets owner"))?;
                if targets.new_war_started.0 != 0 {
                    return Err(Error::Unsupported("NPC Targets pending new war"));
                }
                for war in &targets.knowledge_base.war_knowledge {
                    if war.is_active.0 != 0
                        || state
                            .game_relations()
                            .map_err(|_| Error::Unsupported("NPC Targets current relations"))?
                            .is_at_war(owner, war.opponent)
                            .map_err(|_| Error::Unsupported("NPC Targets current war"))?
                    {
                        return Err(Error::Unsupported("NPC Targets war knowledge decay"));
                    }
                }
                // 21aaeb0 increments knowledge before observing current plots.
                // The shared observer requires empty historical City/barbarian/
                // UnitInfo stores, so their aging loops have no retained work.
                // Player/yield/trade estimates and CollectTargets are policy
                // analyses in this no-added-planning-consumers scope.
                knowledge::observe_visible_units(
                    state,
                    owner,
                    self.definitions.maintenance.unit_registry(),
                    self.visibility,
                )?;
                state
                    .combat_ai_targets_mut(owner)
                    .map_err(|_| Error::Unsupported("NPC Targets current write"))?
                    .new_war_started
                    .0 = 0;
                Ok(())
            }
            Kind::Diplomatic => {
                let d = p
                    .ai_diplomatic
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Diplomatic owner"))?;
                if !d.managed_contracts.is_empty() {
                    return Err(Error::Unsupported("NPC Diplomatic contract activation"));
                }
                if d.diplomatic_states.states.iter().any(|s| {
                    !s.response_action_hashes.is_empty()
                        || !s.desired_action_hashes.is_empty()
                        || !s.transitions.is_empty()
                        || !s.triggers.is_empty()
                }) {
                    return Err(Error::Unsupported("NPC Diplomatic action/state transition"));
                }
                // 20961d0: these empty state vectors never dispatch scripts,
                // DealValue, state changes or RNG. Evaluator readiness is
                // planning output excluded by this explicit scope; formal
                // Player.Diplomacy relations are retained unchanged.
                Ok(())
            }
            Kind::GrandStrategic => {
                no_cities(state, owner)?;
                no_units(state, owner)?;
                let g = p
                    .ai_grand_strategic
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC GrandStrategic owner"))?;
                if g.commemorate_era_raw != 0 {
                    return Err(Error::ExternalDecisionRequired(
                        "era commemoration selection",
                    ));
                }
                // 21217a0 otherwise updates settlement/victory/production
                // policy scores. +2b8 dispatches AiEvent DESIRE_SETTLE with a
                // stack GameModifierAnalysis; stock Analyze consumers produce
                // policy outputs, not Effect.Apply gameplay operations.
                Ok(())
            }
            Kind::Planning => {
                no_units(state, owner)?;
                if p.ai_planning.is_none() {
                    return Err(Error::Unsupported("NPC Planning owner"));
                }
                // 21b9280 creates planners and writes units only within the
                // current unit roster; RunPlanningJobs evaluates policy jobs.
                Ok(())
            }
            Kind::Economic => {
                let current_level = backend
                    .player_services
                    .as_ref()
                    .and_then(|services| services.players.iter().find(|p| p.owner == owner))
                    .and_then(|p| p.civilization_level)
                    .ok_or(Error::Unsupported("NPC Economic civilization level"))?;
                // 20a5ace checks IsFreeCities before Treasury/budget/purchases.
                if current_level != 0x04e6_b0a0 {
                    return Err(Error::Unsupported(
                        "NPC non-FreeCities Economic continuation",
                    ));
                }
                Ok(())
            }
            Kind::Governors => {
                no_cities(state, owner)?;
                if !p.governors.entries.is_empty() {
                    return Err(Error::Unsupported("NPC existing Governor assignments"));
                }
                // 2117120 chooses appointments/promotions and then knapsack
                // city assignments. These new choices belong to external
                // PlayerActions; no native policy command is executed.
                Ok(())
            }
            Kind::Espionage => {
                no_units(state, owner)?;
                // 20b2040's city/discount vectors are policy analyses and
                // logging. Actual promotion, AiEvent requests and processed
                // stores are inside the current unit roster traversal.
                Ok(())
            }
            Kind::Military => {
                no_units(state, owner)?;
                let m = p
                    .ai_military
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Military owner"))?;
                if !m.active_operations.entries.is_empty() {
                    return Err(Error::Unsupported("NPC Military existing operations"));
                }
                // UpdateOps has no existing operation; UpgradeUnits and
                // veteran name/RNG loops have no units. Emergency operation
                // selection is a new native policy decision, externalized
                // with the remaining player actions.
                Ok(())
            }
            Kind::Homeland => {
                no_units(state, owner)?;
                let h = p
                    .ai_homeland
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Homeland owner"))?;
                if h.control_instance_id != u32::MAX {
                    return Err(Error::Unsupported(
                        "NPC Homeland existing control ownership",
                    ));
                }
                // 2135230 prepares move priorities/EstablishPriorities;
                // external policies own actual new Homeland move selection.
                Ok(())
            }
            Kind::Research => {
                let r = p
                    .ai_research
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Research owner"))?;
                if !r.managed_contracts.is_empty() {
                    return Err(Error::Unsupported("NPC Research contract activation"));
                }
                // 2185aca caches GetTurnsLeft for policy analysis. The
                // subsequent full-value and task-bias calculations are policy
                // inputs; selection itself is owned by npc_activation.
                Ok(())
            }
            Kind::Culture => {
                let c = p
                    .ai_culture
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Culture owner"))?;
                if !c.managed_contracts.is_empty() {
                    return Err(Error::Unsupported("NPC Culture contract activation"));
                }
                if self
                    .inputs
                    .great_work_request_count
                    .ok_or(Error::Unsupported("NPC GreatWorks request queue"))?
                    != 0
                {
                    return Err(Error::Unsupported("NPC GreatWorks queued PlayerOperations"));
                }
                if major(backend, owner)? {
                    return Err(Error::Unsupported(
                        "NPC government/policy external decisions",
                    ));
                }
                empty_congress_choices(state, backend, owner)?;
                if !state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("NPC Culture units owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("NPC Culture unit roster"))?
                    .is_empty()
                {
                    return Err(Error::Unsupported("NPC Culture pre-choice unit policies"));
                }
                // Government policy selection2030100 returns for nonmajors;
                // must_change_policies is a native policy-readiness cache.
                // Empty Congress choices rejoin202f4c8 via202fa52. Park
                // candidates and num_naturalists are policy analysis; no unit
                // exists to generate or execute a naturalist policy request.
                Ok(())
            }
            Kind::Religious => {
                if self
                    .inputs
                    .pending_pantheon
                    .ok_or(Error::Unsupported("NPC pending pantheon input"))?
                {
                    return Err(Error::ExternalDecisionRequired(
                        "pending pantheon belief selection",
                    ));
                }
                if self
                    .inputs
                    .religious_contract_count
                    .ok_or(Error::Unsupported("NPC religious contracts input"))?
                    != 0
                {
                    return Err(Error::Unsupported("NPC Religious contract activation"));
                }
                no_units(state, owner)?;
                // No religious Unit exists for the Delete command traversal.
                // PurchaseGreatPeople2175d90 evaluates CanPatronize and
                // GetActivationPlots before choosing a new PlayerOperation;
                // external PlayerActions own that choice and any faith spend.
                // Belief rankings and cached AI religion are policy analyses.
                Ok(())
            }
            _ => Err(Error::Unsupported(
                "NPC reached gameplay/RNG subscriber continuation",
            )),
        }
    }
    fn research_suffix(
        &mut self,
        state: &mut LoadedState,
        _: &mut Runtime<'_, P>,
        owner: i32,
        kind: Kind,
    ) -> Result<(), Error> {
        self.inputs.check(state, owner)?;
        match kind {
            Kind::Research => {
                let r = player(state, owner)?
                    .ai_research
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Research suffix owner"))?;
                if !r.boost_manager.active_boosts.is_empty() {
                    return Err(Error::Unsupported("NPC Research existing boost ownership"));
                }
                // 2185eb5 has already emitted 48062f97 through the wrapper.
                // The remaining stable sorts feed ManageBoosts1fc9be0. With
                // no existing boost ownership to reconcile, that method only
                // chooses and registers new AI controls/operations: external
                // policies own those decisions. Score logging is presentation.
                Ok(())
            }
            Kind::Culture => {
                let p = player(state, owner)?;
                let c = p
                    .ai_culture
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Culture suffix owner"))?;
                if !c.boost_manager.active_boosts.is_empty() {
                    return Err(Error::Unsupported("NPC Culture existing boost ownership"));
                }
                if !state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("NPC Culture units owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("NPC Culture unit roster"))?
                    .is_empty()
                {
                    return Err(Error::Unsupported("NPC Culture unit policy suffix"));
                }
                // QueueNextCivic returns at203179b to202f673. Its suffix
                // sorts analyses, manages new boost policies and walks units
                // for naturalist/rock-band policy requests. Congress202f6d2
                // is a separate predecessor branch, not reached from this
                // QueueNextCivic continuation.
                Ok(())
            }
            _ => Err(Error::Unsupported("NPC research suffix kind")),
        }
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        event: u32,
    ) -> Result<(), Error> {
        self.inputs.check(state, owner)?;
        backend
            .check(state)
            .map_err(|_| Error::Unsupported("NPC report backend binding"))?;
        if event != 0xa235ddf7 {
            return Err(Error::Unsupported("NPC control report type"));
        }
        // a235ddf7 -> City.OnAiAdvisorUpdated27171f0 ->2717700 walks
        // current owned Cities; the empty roster reaches no Fill/query.
        // Host3210ae0 queues176 for advisor presentation only.
        no_cities(state, owner)?;
        backend.reached.push("stock NPC control activation report");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guards_require_current_priority_and_full_activation() {
        for kind in [
            Kind::Research,
            Kind::Culture,
            Kind::FreeCities,
            Kind::Targets,
        ] {
            assert!(callback_enabled(kind, 3, 3, true));
            assert!(!callback_enabled(kind, 2, 3, true));
            assert!(!callback_enabled(kind, 3, 3, false));
        }
        for kind in [Kind::Planning, Kind::Governors] {
            assert!(callback_enabled(kind, 2, 2, false));
            assert!(!callback_enabled(kind, 3, 2, false));
        }
    }
}
