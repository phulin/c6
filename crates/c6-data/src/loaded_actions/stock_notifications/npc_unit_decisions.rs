//! External choices at the source NPC deactivation policy calls.
//! The provider supplies choices; this boundary never selects native AI orders
//! or completes units merely because their player requested EndTurn.
use super::Runtime;
use c6_core::{game_update::Error, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Tactical,
    Homeland,
    CityBuild,
    CultureGovernmentPolicies,
    Governors,
    Economic,
    EconomicTrade,
}

/// A source-positioned invocation, created only by the stock deactivation
/// dispatcher. Decisions may affect this owner's units at this stage.
pub struct Context {
    owner: i32,
    stage: Stage,
    priority: i32,
    turn: i32,
    city_ai_id: Option<u32>,
}
impl Context {
    /// Current CityBuild identity at the per-city activation boundary.
    pub fn city_ai_id(&self) -> Option<u32> {
        self.city_ai_id
    }
    pub fn owner(&self) -> i32 {
        self.owner
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    pub fn priority(&self) -> i32 {
        self.priority
    }
    pub fn turn(&self) -> i32 {
        self.turn
    }
}

/// Synchronous external decision service over the same state and operation
/// owner. Implementations can use `npc_unit_operations` to apply explicit
/// requests and completion choices. No provider preserves the no-choice path.
pub trait Provider<P> {
    fn provide(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        context: &Context,
    ) -> Result<(), String>;
    /// New activation sites require an explicit implementation. A provider for
    /// existing unit stages cannot accidentally defer unrelated decisions.
    fn provide_activation(
        &mut self,
        _state: &mut LoadedState,
        _backend: &mut Runtime<'_, P>,
        _context: &Context,
    ) -> Result<(), String> {
        Err("external activation policy response is not bound".into())
    }
}

impl<'a, P> Runtime<'a, P> {
    pub fn bind_npc_unit_decisions(
        &mut self,
        state: &LoadedState,
        provider: Box<dyn Provider<P> + 'a>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC unit decision binding"))?;
        if self.npc_unit_decisions.is_some() {
            return Err(Error::Unsupported(
                "NPC unit decision provider already bound",
            ));
        }
        self.npc_unit_decisions = Some(provider);
        Ok(())
    }

    pub(super) fn npc_unit_decisions(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        stage: Stage,
        priority: i32,
    ) -> Result<(), Error> {
        self.policy_decisions(state, owner, stage, priority, None, false)
    }

    /// Activation requires an explicitly supplied policy, including an explicit
    /// defer response (`Ok(())` without actions). Absence is unsupported.
    pub(super) fn activation_policy_decisions(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        stage: Stage,
        city_ai_id: Option<u32>,
    ) -> Result<(), Error> {
        let priority =
            c6_core::loaded_state::game_update::human_activation::current_priority(state, owner)?;
        self.policy_decisions(state, owner, stage, priority, city_ai_id, true)
    }

    fn policy_decisions(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        stage: Stage,
        priority: i32,
        city_ai_id: Option<u32>,
        required: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC unit decision runtime"))?;
        let Some(mut provider) = self.npc_unit_decisions.take() else {
            return if required {
                Err(Error::Unsupported(
                    "NPC activation requires explicit external policy response",
                ))
            } else {
                Ok(())
            };
        };
        let context = Context {
            owner,
            stage,
            priority,
            turn: state.retained().core.current_turn,
            city_ai_id,
        };
        let result = if required {
            provider.provide_activation(state, self, &context)
        } else {
            provider.provide(state, self, &context)
        };
        self.npc_unit_decisions = Some(provider);
        result.map_err(|cause| {
            self.last_error = Some(format!("external NPC unit decision: {cause}"));
            Error::Unsupported("external NPC unit decision provider")
        })
    }
}
