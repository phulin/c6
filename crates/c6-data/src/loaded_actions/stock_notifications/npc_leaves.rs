//! Concrete stock nonhuman subscriber guards. This is a partial activation
//! backend: an unimplemented reached gameplay/RNG continuation is an error,
//! including on an owner with no cities or units.
use super::{maintenance, npc_activation, Runtime};
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
            callbacks,
        })
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
fn empty_free_cities(state: &LoadedState) -> Result<(), Error> {
    // DoTurnActivate294e1f0 reaches OnTurnActivate only within this vector.
    if !state.retained().free_cities.instances.is_empty() {
        return Err(Error::Unsupported("NPC FreeCities OnTurnActivate"));
    }
    Ok(())
}
pub struct Leaves<'i> {
    inputs: &'i Inputs,
    _definitions: &'i maintenance::Definitions,
    _visibility: &'i crate::unit_sight_state::Runtime,
}
impl<'i> Leaves<'i> {
    pub fn new(
        inputs: &'i Inputs,
        definitions: &'i maintenance::Definitions,
        visibility: &'i crate::unit_sight_state::Runtime,
    ) -> Self {
        Self {
            inputs,
            _definitions: definitions,
            _visibility: visibility,
        }
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
        _: &mut Runtime<'_, P>,
        owner: i32,
    ) -> Result<bool, Error> {
        self.inputs.check(state, owner)?;
        for &kind in &self.inputs.off_turn {
            match kind {
                Kind::City => no_cities(state, owner)?,
                _ => return Err(Error::Unsupported("NPC off-turn gameplay/RNG continuation")),
            }
        }
        Ok(false)
    }
    fn callback(
        &mut self,
        state: &mut LoadedState,
        _: &mut Runtime<'_, P>,
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
                // Nonhuman UpdateKnowledgeBase/CollectTargets also reaches
                // other knowledge stores and potential RNG. Empty own assets
                // alone do not close that continuation.
                Err(Error::Unsupported(
                    "NPC Targets knowledge/CollectTargets continuation",
                ))
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
            Kind::Research => {
                let r = p
                    .ai_research
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Research owner"))?;
                if !r.managed_contracts.is_empty() {
                    return Err(Error::Unsupported("NPC Research contract activation"));
                }
                // 2185aca stores GetTurnsLeft before contracts and policy.
                Err(Error::Unsupported(
                    "NPC Research turns-left/analysis prefix",
                ))
            }
            Kind::Culture => {
                let c = p
                    .ai_culture
                    .as_ref()
                    .ok_or(Error::Unsupported("NPC Culture owner"))?;
                if !c.managed_contracts.is_empty() {
                    return Err(Error::Unsupported("NPC Culture contract activation"));
                }
                Err(Error::Unsupported(
                    "NPC Culture pre-policy gameplay continuation",
                ))
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
                Err(Error::Unsupported(
                    "NPC Religious acquisition/purchase continuation",
                ))
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
            // After 48062f97, 2186022 calls ManageBoosts even with no assets.
            Kind::Research => Err(Error::Unsupported("NPC Research ManageBoosts suffix")),
            Kind::Culture => Err(Error::Unsupported(
                "NPC Culture government/policy/Congress suffix",
            )),
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
        if event != 0xa235ddf7 {
            return Err(Error::Unsupported("NPC control report type"));
        }
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
