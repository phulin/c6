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
    /// Unsaved CityBuild+1a0 values keyed by the current recycled City AI id.
    city_priorities: Option<Vec<(u32, i32)>>,
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
            city_priorities: None,
        })
    }
    /// Bind the current unsaved GreatWorks request vector length. A missing
    /// count remains unknown until ExecuteRequests is actually reached.
    pub fn with_great_work_request_count(mut self, count: Option<usize>) -> Self {
        self.great_work_request_count = count;
        self
    }
    pub fn with_city_priorities(mut self, priorities: Vec<(u32, i32)>) -> Result<Self, Error> {
        for (index, &(id, _)) in priorities.iter().enumerate() {
            if priorities[..index].iter().any(|v| v.0 == id) {
                return Err(Error::Unsupported(
                    "duplicate NPC CityBuild priority identity",
                ));
            }
        }
        self.city_priorities = Some(priorities);
        Ok(self)
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
    fn pantheon_requested(
        &self,
        state: &LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        full: bool,
    ) -> Result<bool, Error> {
        if !self.inputs.reached(state, owner, Kind::Religious, full)? {
            return Ok(false);
        }
        backend.pending_pantheon(owner, self.inputs.pending_pantheon)
    }
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
            Kind::City => {
                let manager = p
                    .ai_city_manager
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC current CityBuild manager"))?;
                if manager.city_ais.entries.is_empty() {
                    return Ok(());
                }
                let free_cities = backend
                    .player_services
                    .as_ref()
                    .and_then(|s| s.players.iter().find(|p| p.owner == owner))
                    .and_then(|p| p.civilization_level)
                    .ok_or(Error::Unsupported("NPC City civilization level"))?
                    == 0x04e6_b0a0;
                let mut reached = Vec::new();
                let priority = current_priority(state, owner)?;
                for entry in &manager.city_ais.entries {
                    let city = &entry.city_build;
                    let configured = self
                        .inputs
                        .city_priorities
                        .as_ref()
                        .and_then(|v| v.iter().find(|v| v.0 == city.city_ai_id))
                        .ok_or(Error::Unsupported("NPC current CityBuild priority"))?
                        .1;
                    if configured != priority {
                        continue;
                    }
                    if free_cities {
                        continue;
                    }
                    // UpdateWorkingContract1fda930 precedes all policy sites.
                    // An existing handle can fail/cancel and pop production.
                    if city.managed_working_contract.player_id >= 0
                        && city.managed_working_contract.id != u32::MAX
                    {
                        return Err(Error::Unsupported(
                            "NPC existing CityBuild working contract",
                        ));
                    }
                    if !city.managed_contracts.is_empty() {
                        return Err(Error::Unsupported("NPC existing CityBuild contracts"));
                    }
                    reached.push(city.city_ai_id);
                }
                // Scores/terrain/food/defender analyses are excluded native
                // planning storage, as on the stock human activation path.
                state
                    .omit_city_build_planning(owner)
                    .map_err(|_| Error::Unsupported("NPC CityBuild planning owner"))?;
                for id in reached {
                    backend.activation_policy_decisions(
                        state,
                        owner,
                        super::npc_unit_decisions::Stage::CityBuild,
                        Some(id),
                    )?;
                }
                Ok(())
            }
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
                if p.ai_planning.is_none() {
                    return Err(Error::Unsupported("NPC Planning owner"));
                }
                // 21b9280 attaches native planner pointers; all direct
                // GetAiPlanner consumers are AI policy routines. Execute and
                // BFS fill move/target scores, not Unit orders or Formation
                // paths. Preview simulations21b7ef9/8148 pass (true,true);
                // CalculateUnitVsUnitDamage2bd8c08 stores true as bUseAverage
                // (+44), the existing RNG-free average damage kernel.
                // This external-policy boundary omits that planning storage,
                // balanced job locks and scores, not a gameplay transition.
                Ok(())
            }
            Kind::Economic => {
                let current_level = backend
                    .player_services
                    .as_ref()
                    .and_then(|services| services.players.iter().find(|p| p.owner == owner))
                    .and_then(|p| p.civilization_level)
                    .ok_or(Error::Unsupported("NPC Economic civilization level"))?;
                // 20a5aba/20a5ace return for barbarians and FreeCities
                // before Treasury, contracts, purchases or RNG.
                if matches!(current_level, 0xd959_ec24 | 0x04e6_b0a0) {
                    return Ok(());
                }
                let economic = p
                    .ai_economic
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Economic owner"))?;
                if !economic.managed_contracts.is_empty() {
                    return Err(Error::Unsupported(
                        "NPC Economic existing contract continuation",
                    ));
                }
                // 20a5ad8..60ac updates budget estimates only: all Treasury
                // accesses are getters. External policies supply purchases,
                // tribe payoffs and maintenance disband choices after6191.
                backend.activation_policy_decisions(
                    state,
                    owner,
                    super::npc_unit_decisions::Stage::Economic,
                    None,
                )?;
                // 20a6392 reaches Trade even after a deferred purchase. Route
                // choice and selected-unit SetProcessedTurn are explicit policy
                // actions; no unit is marked merely by reaching this suffix.
                backend.activation_policy_decisions(
                    state,
                    owner,
                    super::npc_unit_decisions::Stage::EconomicTrade,
                    None,
                )
            }
            Kind::Governors => {
                // 2117120 logs analysis, chooses appointment/promotion, then
                // DoKnapsackAssignments21178e0 chooses city assignments.
                // Each selected gameplay operation belongs to the external
                // response; the scoring algorithm and log are omitted.
                backend.activation_policy_decisions(
                    state,
                    owner,
                    super::npc_unit_decisions::Stage::Governors,
                    None,
                )
            }
            Kind::Espionage => {
                for slot in state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("Espionage current unit owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("Espionage current unit roster"))?
                {
                    let unit = &slot.value;
                    if unit.e_type_index == u32::MAX
                        || unit.b_delayed_death
                        || unit.ai_controller != u32::MAX
                    {
                        continue;
                    }
                    let definitions = backend
                        .unit_creation
                        .as_ref()
                        .ok_or(Error::Unsupported("Espionage current unit definitions"))?
                        .definitions;
                    if definitions
                        .is_spy(state, unit)
                        .map_err(|_| Error::Unsupported("Espionage current spy query"))?
                    {
                        return Err(Error::Unsupported("NPC Spy gameplay continuation"));
                    }
                }
                // 20b2040's city/discount vectors are policy analyses and
                // logging. 20b2443..2480 rejects invalid type, delayed death,
                // assigned controller and non-Spy definitions before any
                // promotion, AiEvent request or processed-turn write.
                Ok(())
            }
            Kind::Military => {
                let m = p
                    .ai_military
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Military owner"))?;
                if !m.active_operations.entries.is_empty() {
                    return Err(Error::Unsupported("NPC Military existing operations"));
                }
                // UpdateOps has no existing operation. UpgradeUnits2149ac0
                // selects an upgrade using efficiency and affordability, then
                // issues TryUpgradeUnit; that RNG-free policy choice belongs
                // to external actions, as does starting a new emergency op.
                for slot in state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("Military current unit owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("Military current unit roster"))?
                {
                    let unit = &slot.value;
                    // 2149f8e..2149fd1 names only combat units at level >=3
                    // with an empty veteran name. These two retained guards
                    // can reject before the read-only strength queries.
                    if unit.k_experience.i_level < 3 || !unit.k_experience.s_veteran_name.is_empty()
                    {
                        continue;
                    }
                    let combat = backend
                        .movement()
                        .map_err(|_| Error::Unsupported("Military current combat definitions"))?
                        .definitions
                        .formation;
                    if combat
                        .combat(state, unit)
                        .map_err(|_| Error::Unsupported("Military current combat strength"))?
                        > 0
                        || combat
                            .ranged(state, unit)
                            .map_err(|_| Error::Unsupported("Military current ranged strength"))?
                            > 0
                        || combat
                            .bombard(state, unit)
                            .map_err(|_| Error::Unsupported("Military current bombard strength"))?
                            > 0
                    {
                        return Err(Error::Unsupported(
                            "Military veteran naming/RNG continuation",
                        ));
                    }
                }
                Ok(())
            }
            Kind::Homeland => {
                let level = backend
                    .player_services
                    .as_ref()
                    .and_then(|s| s.players.iter().find(|p| p.owner == owner))
                    .and_then(|p| p.civilization_level)
                    .ok_or(Error::Unsupported("Homeland current civilization level"))?;
                if level != 0xd959_ec24 && player(state, owner)?.cities.records.entries.is_empty() {
                    // Without a city, a matching MoveList policy can request
                    // additional priority3. Its policy list is not yet bound.
                    no_units(state, backend, owner, kind)?;
                    return Ok(());
                }
                // 213526c skips the move/city branch for barbarians and writes
                // -1 through +e8 to m_additionalPriority(+f8), not the global
                // Control manager priority. EstablishPriorities only prepares
                // policy move scores; DoTurn's selection is externalized.
                state
                    .player_ai_homeland_mut(owner)
                    .map_err(|_| Error::Unsupported("Homeland current write"))?
                    .as_mut()
                    .ok_or(Error::Unsupported("NPC Homeland owner"))?
                    .additional_priority = -1;
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
                let requested = c6_core::loaded_state::game_update::npc_activation::take_culture_policy_request(state, owner)?;
                if requested && major(backend, owner)? {
                    backend.activation_policy_decisions(
                        state,
                        owner,
                        super::npc_unit_decisions::Stage::CultureGovernmentPolicies,
                        None,
                    )?;
                }
                empty_congress_choices(state, backend, owner, self.managers)?;
                // Government policy selection2030100 returns for nonmajors;
                // must_change_policies is a native policy-readiness cache.
                // Empty Congress choices rejoin202f4c8 via202fa52. The unit
                // traversal202f5e0..202f645 only reads ParkCharges and counts
                // AI num_naturalists; it writes no Unit, action or RNG. Omit
                // that policy count together with the parkcandidate scores.
                Ok(())
            }
            Kind::Religious => {
                if backend.pending_pantheon(owner, self.inputs.pending_pantheon)? {
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
                let level = backend
                    .player_services
                    .as_ref()
                    .and_then(|s| s.players.iter().find(|p| p.owner == owner))
                    .and_then(|p| p.civilization_level)
                    .ok_or(Error::Unsupported("Religious current civilization level"))?;
                // 2173c48 bypasses the traversal for barbarians. Other
                // units are filtered by ReligiousStrength at2173cdb before
                // religion comparison or command construction. Ordinary
                // Settlers/Warriors therefore require no Delete continuation.
                if level != 0xd959_ec24 {
                    for entry in state
                        .world()
                        .player(owner)
                        .map_err(|_| Error::Unsupported("Religious current unit owner"))?
                        .unit_records()
                        .map_err(|_| Error::Unsupported("Religious current unit roster"))?
                    {
                        if entry.value.k_religion.i_religious_strength > 0 {
                            return Err(Error::Unsupported(
                                "Religious positive-strength unit continuation",
                            ));
                        }
                    }
                }
                // The barbarian guard skips the Delete command traversal.
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
                for slot in state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("NPC Culture units owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("NPC Culture unit roster"))?
                {
                    // 202ff1e tests the current controller before 202fc5b
                    // checks the Archaeology home-city sentinel. Ordinary
                    // camp units leave before city/plot queries or AiEvent.
                    let unit = &slot.value;
                    if unit.ai_controller == u32::MAX
                        && unit.k_archaeology.k_home_city_id != u32::MAX
                    {
                        return Err(Error::Unsupported(
                            "NPC archaeologist analysis continuation",
                        ));
                    }
                }
                // QueueNextCivic returns at203179b to202f673. Its suffix
                // sorts analyses and manages new boost policies. Congress
                // 202f6d2 is a separate predecessor branch, not reached from
                // this QueueNextCivic continuation. No source RNG or gameplay
                // writes occur in the rejected ordinary-unit traversal.
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
        // a235ddf7 -> City.OnAiAdvisorUpdated27171f0 ->2717700 fills
        // advisor presentation DTOs, as in the stock human boundary.
        // Host3210ae0 queues176 for advisor presentation only.
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
