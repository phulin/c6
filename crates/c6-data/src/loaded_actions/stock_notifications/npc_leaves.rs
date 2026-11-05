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
fn no_units<P>(
    state: &LoadedState,
    backend: &mut Runtime<'_, P>,
    owner: i32,
    kind: Kind,
) -> Result<(), Error> {
    if !state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("NPC current unit owner"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("NPC current unit roster"))?
        .is_empty()
    {
        backend.last_error = Some(format!(
            "NPC {kind:?} nonempty unit gameplay continuation for player {owner}"
        ));
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
    managers: Option<&crate::game_update_state::activation::cityless_managers::Runtime>,
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
            let current = managers
                .ok_or(Error::Unsupported("NPC current Congress/Emergency owner"))?
                .congress(state)?;
            // 2c4b2b8 FindDiscussionOptions walks Discussion definitions and
            // resolves their Emergency instances. An observed empty current
            // instance list produces no candidates, including for met majors.
            if !current.emergencies.is_empty() {
                return Err(Error::Unsupported(
                    "NPC Congress proposal decision continuation",
                ));
            }
        }
    }
    Ok(())
}
/// Same-owner camp composition, invoked only after the real automatic research
/// grants. It must perform CreateCamps, current tribe activation, then Clans.
pub trait Camps<P> {
    fn visibility(&self) -> &crate::unit_sight_state::Runtime;
    fn run(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut c6_core::loaded_state::game_update::Runtime,
        owner: i32,
    ) -> Result<(), Error>;
}
enum Visibility<'i, P> {
    Borrowed(&'i crate::unit_sight_state::Runtime),
    Camps(&'i mut dyn Camps<P>),
}
pub struct Leaves<'i, P = c6_core::unit::OperationParameters> {
    inputs: &'i Inputs,
    definitions: &'i maintenance::Definitions,
    visibility: Visibility<'i, P>,
    off_turn: Option<&'i mut inactive_control::Inputs>,
    managers: Option<&'i crate::game_update_state::activation::cityless_managers::Runtime>,
    requirements: Option<&'i mut c6_core::melee_attack::proposals::subject_sets::Runtime>,
}
impl<'i, P> Leaves<'i, P> {
    pub fn new(
        inputs: &'i Inputs,
        definitions: &'i maintenance::Definitions,
        visibility: &'i crate::unit_sight_state::Runtime,
    ) -> Self {
        Self {
            inputs,
            definitions,
            visibility: Visibility::Borrowed(visibility),
            off_turn: None,
            managers: None,
            requirements: None,
        }
    }
    pub fn new_with_camps(
        inputs: &'i Inputs,
        definitions: &'i maintenance::Definitions,
        camps: &'i mut dyn Camps<P>,
    ) -> Self {
        Self {
            inputs,
            definitions,
            visibility: Visibility::Camps(camps),
            off_turn: None,
            managers: None,
            requirements: None,
        }
    }
    fn visibility(&self) -> &crate::unit_sight_state::Runtime {
        match &self.visibility {
            Visibility::Borrowed(value) => value,
            Visibility::Camps(camps) => camps.visibility(),
        }
    }
    /// Reuse the initialized off-turn owner across policy pauses. This is the
    /// same canonical runtime input used by inactive Control.DoTurn; do not
    /// replace it with a freshly decoded readiness snapshot on resume.
    pub fn with_off_turn(mut self, inputs: &'i mut inactive_control::Inputs) -> Self {
        self.off_turn = Some(inputs);
        self
    }
    pub fn with_managers(
        mut self,
        managers: &'i crate::game_update_state::activation::cityless_managers::Runtime,
    ) -> Self {
        self.managers = Some(managers);
        self
    }
    /// Borrow the current turn's single requirement owner only while camp
    /// creation dispatches the reentrant factory effects.
    pub fn with_requirements(
        mut self,
        requirements: &'i mut c6_core::melee_attack::proposals::subject_sets::Runtime,
    ) -> Self {
        self.requirements = Some(requirements);
        self
    }
}
impl<P> npc_activation::Leaves<P> for Leaves<'_, P> {
    fn research_sight(&self) -> Option<&crate::unit_sight_state::Runtime> {
        Some(self.visibility())
    }
    fn callbacks(&self) -> &[Kind] {
        &self.inputs.callbacks
    }
    fn barbarian_reached(
        &self,
        state: &LoadedState,
        owner: i32,
        full: bool,
    ) -> Result<bool, Error> {
        self.inputs.reached(state, owner, Kind::Barbarian, full)
    }
    fn barbarian_after_research(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        runtime: &mut c6_core::loaded_state::game_update::Runtime,
        owner: i32,
    ) -> Result<(), Error> {
        let manager = &state.retained().barbarian;
        if manager.player_ordinal != owner {
            return Err(Error::Unsupported("NPC Barbarian current owner"));
        }
        if let Visibility::Camps(camps) = &mut self.visibility {
            let requirements = self
                .requirements
                .as_deref_mut()
                .ok_or(Error::Unsupported("NPC camp current requirement owner"))?;
            return backend
                .with_modifier_requirement_sets(requirements, |backend| {
                    camps.run(state, backend, runtime, owner)
                })
                .map_err(|error| {
                    backend.last_error = Some(format!("camp shared requirements: {error:?}"));
                    Error::Unsupported("camp shared requirement ownership")
                })?;
        }
        if manager.improvement_hash != u32::MAX && manager.value_e0 < manager.value_c8 {
            return Err(Error::Unsupported("NPC Barbarian CreateCamps"));
        }
        if !manager.instances.is_empty() {
            return Err(Error::Unsupported("NPC Barbarian tribe OnTurnActivate"));
        }
        let inputs = backend
            .city_query_inputs()
            .ok_or(Error::Unsupported("NPC current capability membership"))?;
        if inputs
            .current
            .capabilities
            .has(c6_core::game_capabilities::BARBARIAN_CLANS)
        {
            return Err(Error::Unsupported("NPC Barbarian Clans activation"));
        }
        Ok(())
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
            Kind::Barbarian => {
                if state.retained().barbarian.player_ordinal != owner {
                    return Err(Error::Unsupported("NPC Barbarian current player binding"));
                }
                // The wrapper executes acquisition then the actual camp suffix.
                Ok(())
            }
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
                    // CivKnowledge::IncrementTurn 20009f0..20009fb skips the
                    // unassigned record before consulting current relations.
                    if war.opponent == -1 {
                        continue;
                    }
                    if war.is_active.0 != 0
                        || state
                            .game_relations()
                            .map_err(|_| Error::Unsupported("NPC Targets current relations"))?
                            .is_at_war(owner, war.opponent)
                            .map_err(|error| {
                                backend.last_error = Some(format!(
                                    "NPC Targets current war {owner}/{}: {error:?}",
                                    war.opponent
                                ));
                                Error::Unsupported("NPC Targets current war")
                            })?
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
                    self.visibility(),
                    backend.movement().ok().map(|m| m.definitions.formation),
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
                // 2121e72..2121fd5 visits units only for great-person
                // analysis. IsGreatPerson203b890 requires both identifiers;
                // controlled people also leave before the AiEvent dispatch.
                // Ordinary units have no writes, callbacks or RNG here.
                for unit in state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("GrandStrategic current unit owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("GrandStrategic current unit roster"))?
                {
                    if unit.value.k_great_person.e_individual != u32::MAX
                        && unit.value.k_great_person.e_class != u32::MAX
                        && unit.value.ai_controller == u32::MAX
                    {
                        return Err(Error::Unsupported(
                            "GrandStrategic great-person analysis continuation",
                        ));
                    }
                }
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
                no_units(state, backend, owner, kind)?;
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
                no_units(state, backend, owner, kind)?;
                // 20b2040's city/discount vectors are policy analyses and
                // logging. Actual promotion, AiEvent requests and processed
                // stores are inside the current unit roster traversal.
                Ok(())
            }
            Kind::Military => {
                no_units(state, backend, owner, kind)?;
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
                no_units(state, backend, owner, kind)?;
                // 2135230 prepares move priorities/EstablishPriorities;
                // it does not inspect or execute control_instance_id. Preserve
                // that owner for the separate control/deactivation path.
                // External policies own actual new Homeland move selection.
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
                empty_congress_choices(state, backend, owner, self.managers)?;
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
                no_units(state, backend, owner, kind)?;
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
            Kind::Barbarian,
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
