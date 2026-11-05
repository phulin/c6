//! Human control activation on the same persistent stock action owner.
//! Native policy scores, job scheduling, planner pointers and target DTOs are
//! excluded from the external-policy arena. Gameplay ownership and role writes
//! run at their actual off-turn/priority boundaries.
pub mod knowledge;
use super::{maintenance, Runtime};
use crate::game_update_state::activation::maintenance::ScoutInputs;
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_components::RetainedPlayer,
    loaded_state::{game_update::human_activation as core, LoadedState},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    City,
    BehaviorTree,
    Planning,
    Diplomatic,
    Religious,
    Targets,
    GrandStrategic,
    Governors,
    Research,
    Economic,
    Espionage,
    Homeland,
    Culture,
    Military,
    Tactical,
}
/// Explicit profile for the stock human callbacks. Added subscribers, existing
/// executable control contracts and automatic policy execution are not admitted.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockHumanExternalPoliciesNoAddedPlanningConsumers,
}
pub struct Inputs {
    binding: WireBinding,
    pub owner: i32,
    /// Current registration order; no sorting or assumed constructor order.
    pub off_turn: Vec<Kind>,
    pub priorities: Vec<(Kind, i32)>,
    /// Unsaved Religious+100 request flag. Required only at that callback.
    pub pending_pantheon: Option<bool>,
    pub religious_contract_count: Option<usize>,
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
            return Err(Error::Unsupported("human activation initial binding"));
        }
        Ok(Self {
            binding: binding.clone(),
            owner,
            off_turn,
            priorities,
            pending_pantheon,
            religious_contract_count,
        })
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("human activation current player"))
}
struct Effects<'s, 'a, 'i, P> {
    backend: &'s mut Runtime<'a, P>,
    inputs: &'i Inputs,
    definitions: &'i maintenance::Definitions,
    units: &'i [u32],
    scout: &'i ScoutInputs,
    visibility: &'i crate::unit_sight_state::Runtime,
    major: bool,
    initialized: bool,
}
impl<P> Effects<'_, '_, '_, P> {
    fn fail(&mut self, error: impl std::fmt::Debug) -> Error {
        self.backend.last_error = Some(format!("human activation: {error:?}"));
        Error::Unsupported("stock human activation; inspect backend.last_error")
    }
    fn city_callbacks(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        let p = player(state, owner)?;
        if p.cities.records.entries.is_empty() {
            return Ok(());
        }
        let manager = p
            .ai_city_manager
            .as_ref()
            .ok_or(Error::Unsupported("current human CityBuild manager"))?;
        for entry in &manager.city_ais.entries {
            let city = &entry.city_build;
            // UpdateWorkingContract1fda930 may cancel a contract and pop the
            // actual production queue. An absent handle returns immediately.
            if city.managed_working_contract.player_id >= 0
                && city.managed_working_contract.id != u32::MAX
            {
                return Err(Error::Unsupported("human CityBuild working contract"));
            }
        }
        // 1fdd181..1fdd2c8 refreshes plot/district/food/defender policy
        // evaluations, using current getters and arithmetic without RNG.
        // The actual human guard at1fdd2d0 returns before CheckGrowth,
        // contract requests and QueueNextBuild. Those are external decisions.
        // Off-turn analyzer jobs and these derived scores are not gameplay.
        // 1fdd220..1fdd23a only writes MAX into stale CityBuild-local tree
        // references. It never releases or modifies the actual tree/control;
        // those references are consumed by the native production policies
        // below the human return. Mark this cache unavailable with the rest.
        state
            .omit_city_build_planning(owner)
            .map_err(|_| Error::Unsupported("human CityBuild planning owner"))?;
        Ok(())
    }
    fn controller_callbacks(state: &LoadedState, owner: i32, kind: Kind) -> Result<(), Error> {
        let p = player(state, owner)?;
        match kind {
            Kind::Diplomatic => {
                let d = p
                    .ai_diplomatic
                    .as_ref()
                    .ok_or(Error::Unsupported("current human diplomatic controls"))?;
                if d.managed_contracts.iter().any(|c| c.id != u32::MAX) {
                    return Err(Error::Unsupported(
                        "existing diplomatic contract activation",
                    ));
                }
            }
            Kind::Research => {
                let r = p
                    .ai_research
                    .as_ref()
                    .ok_or(Error::Unsupported("current human research controls"))?;
                if r.managed_contracts.iter().any(|c| c.id != u32::MAX) {
                    return Err(Error::Unsupported("existing research contract activation"));
                }
            }
            Kind::Culture => {
                let c = p
                    .ai_culture
                    .as_ref()
                    .ok_or(Error::Unsupported("current human culture controls"))?;
                if c.managed_contracts.iter().any(|c| c.id != u32::MAX) {
                    return Err(Error::Unsupported("existing culture contract activation"));
                }
            }
            Kind::BehaviorTree => {
                let b = p
                    .ai_behavior_tree
                    .as_ref()
                    .ok_or(Error::Unsupported("current human behavior trees"))?;
                if !b.behavior_trees.entries.is_empty() {
                    return Err(Error::Unsupported("existing behavior tree Reset"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
impl<P> core::Effects for Effects<'_, '_, '_, P> {
    fn wait_for_jobs(&mut self, state: &mut LoadedState, _: i32) -> Result<(), Error> {
        self.backend.check(state).map_err(|e| self.fail(e))?;
        // The Rust execution context is synchronous. Native worker joins and
        // start/end profiling/presentation signals are not gameplay jobs.
        Ok(())
    }
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error> {
        for &kind in &self.inputs.off_turn {
            match kind {
                Kind::City => Self::city_callbacks(state, owner)?,
                Kind::Military => {
                    // UpdateOffTurn214a198: AnalyzeForces precedes defenses
                    // and new-operation score evaluation. Preserve its roles.
                    maintenance::Context {
                        state,
                        backend: self.backend,
                        definitions: self.definitions,
                    }
                    .roles(owner, self.units, self.major, self.initialized, self.scout)
                    .map_err(|e| self.fail(e))?;
                }
                Kind::Culture
                | Kind::Diplomatic
                | Kind::Economic
                | Kind::Governors
                | Kind::GrandStrategic
                | Kind::Research
                | Kind::Espionage
                | Kind::Religious => {
                    // These off-turn consumers produce policy analyses only;
                    // the stock human profile has no added consumers of them.
                    // The source arithmetic and score-job paths consume no
                    // gameplay RNG. Their scheduling statuses can be collapsed
                    // after completing the retained gameplay work in this ring.
                }
                _ => return Err(Error::Unsupported("unsupported off-turn callback kind")),
            }
        }
        Ok(false)
    }
    fn priority_callbacks(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        full_turn: bool,
    ) -> Result<(), Error> {
        // Native captures the vector's original end for each priority pass,
        // rereading current priority before each individual callback.
        for &(kind, configured) in &self.inputs.priorities {
            let priority = core::current_priority(state, owner)?;
            if matches!(kind, Kind::City | Kind::BehaviorTree) {
                if full_turn {
                    if kind == Kind::City {
                        Self::city_callbacks(state, owner)?;
                    } else {
                        Self::controller_callbacks(state, owner, kind)?;
                    }
                }
                continue;
            }
            if priority != configured {
                continue;
            }
            if kind != Kind::Planning && kind != Kind::Governors && !full_turn {
                continue;
            }
            Self::controller_callbacks(state, owner, kind)?;
            match kind {
                Kind::Religious => {
                    if self
                        .inputs
                        .pending_pantheon
                        .ok_or(Error::Unsupported("current religious pantheon request"))?
                    {
                        return Err(Error::ExternalDecisionRequired(
                            "pending pantheon belief selection",
                        ));
                    }
                    if self
                        .inputs
                        .religious_contract_count
                        .ok_or(Error::Unsupported("current religious contract count"))?
                        != 0
                    {
                        return Err(Error::Unsupported("existing religious contract activation"));
                    }
                    // IsHuman blocks belief acquisition, religious unit delete
                    // and PurchaseGreatPeople; missionary targets are planning.
                }
                Kind::Military => {
                    let m = player(state, owner)?
                        .ai_military
                        .as_ref()
                        .ok_or(Error::Unsupported("current human Military operations"))?;
                    if !m.active_operations.entries.is_empty() {
                        return Err(Error::Unsupported("existing Military operation activation"));
                    }
                    // UpgradeUnits evaluates candidates, then IsHuman guards
                    // the actual TryUpgradeUnit. Veteran naming is also under
                    // !IsHuman. External policies own both choices.
                }
                Kind::Targets => {
                    knowledge::observe_visible_units(
                        state,
                        owner,
                        self.definitions.maintenance.unit_registry(),
                        self.visibility,
                        self.backend
                            .movement()
                            .ok()
                            .map(|m| m.definitions.formation),
                    )?;
                    let t = player(state, owner)?
                        .ai_targets
                        .as_ref()
                        .ok_or(Error::Unsupported("current human Targets"))?;
                    if t.new_war_started.0 != 0 {
                        return Err(Error::Unsupported("Targets pending war operations"));
                    }
                }
                // Governors and Espionage return on current IsHuman. Other
                // callbacks maintain policy scores/pointers/targets; no native
                // policy request is executed in this stock human scope.
                _ => {}
            }
        }
        Ok(())
    }
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error> {
        self.backend.check(state).map_err(|e| self.fail(e))?;
        if event != 0xa235ddf7 {
            return Err(Error::Unsupported("human control report type"));
        }
        let _ = player(state, owner)?;
        // a235ddf7 registers City.OnAiAdvisorUpdated27171f0, which
        // traverses Cities via2717700 to fill advisor presentation output.
        // Host3210ae0 queues Player event176. These derived DTOs are omitted;
        // city gameplay/contract work is handled at its priority boundary.
        self.backend
            .reached
            .push("stock human control activation report");
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
pub fn run<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    definitions: &maintenance::Definitions,
    inputs: &Inputs,
    units: &[u32],
    major: bool,
    initialized: bool,
    scout: &ScoutInputs,
    full_turn: bool,
    visibility: &crate::unit_sight_state::Runtime,
) -> Result<(), Error> {
    if state.binding() != &inputs.binding {
        return Err(Error::Unsupported("human activation runtime binding"));
    }
    core::activate(
        state,
        inputs.owner,
        units,
        full_turn,
        &mut Effects {
            backend,
            inputs,
            definitions,
            units,
            scout,
            visibility,
            major,
            initialized,
        },
    )
}
