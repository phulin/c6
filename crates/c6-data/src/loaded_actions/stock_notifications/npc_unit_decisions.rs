//! External choices at the source NPC deactivation policy calls.
//! The provider supplies choices; this boundary never selects native AI orders
//! or completes units merely because their player requested EndTurn.
use super::Runtime;
use c6_core::{game_update::Error, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    Tactical,
    Homeland,
}

/// A source-positioned invocation, created only by the stock deactivation
/// dispatcher. Decisions may affect this owner's units at this stage.
pub struct Context {
    owner: i32,
    stage: Stage,
    priority: i32,
    turn: i32,
}
impl Context {
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
        self.check(state)
            .map_err(|_| Error::Unsupported("NPC unit decision runtime"))?;
        let Some(mut provider) = self.npc_unit_decisions.take() else {
            return Ok(());
        };
        let context = Context {
            owner,
            stage,
            priority,
            turn: state.retained().core.current_turn,
        };
        let result = provider.provide(state, self, &context);
        self.npc_unit_decisions = Some(provider);
        result.map_err(|cause| {
            self.last_error = Some(format!("external NPC unit decision: {cause}"));
            Error::Unsupported("external NPC unit decision provider")
        })
    }
}
