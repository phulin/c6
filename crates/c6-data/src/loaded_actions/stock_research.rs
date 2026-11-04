//! Research turn composition borrowing the SAME stock action backend.
//! Canonical science/progress calculation stays in `research::Context`.
use super::Runtime;
use crate::loaded_actions::{choose_tech, notifications::processing};
use crate::research;
use c6_core::loaded_state::LoadedState;
#[derive(Debug)]
pub enum Error {
    Choice(choose_tech::Error),
    Notification(processing::Error),
    Effect(research::Effect),
    Query(research::Query),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock research: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Borrowed composition; no ScriptState, policy, World or research runtime is
/// cloned. Definitions and optional current graphs are retained by the caller.
/// Unknown positive acquisition/query effects fail only when actually reached.
pub struct Effects<'s, 'a, P> {
    pub backend: &'s mut Runtime<'a, P>,
    pub runtime: &'s mut research::Runtime,
    pub choices: &'s choose_tech::Definitions,
    pub choice_graph: Option<&'s choose_tech::GameTechs>,
}
impl<P> research::Effects for Effects<'_, '_, P> {
    type Error = Error;
    fn runtime(&self) -> &research::Runtime {
        self.runtime
    }
    fn runtime_mut(&mut self) -> &mut research::Runtime {
        self.runtime
    }
    fn effect(
        &mut self,
        state: &mut LoadedState,
        _: &research::Definitions,
        effect: research::Effect,
    ) -> Result<(), Error> {
        self.backend.check(state).map_err(Error::Notification)?;
        match effect {
            research::Effect::ChooseTechnology(owner) => {
                let needed = self
                    .choices
                    .player(state, owner, self.choice_graph)
                    .map_err(Error::Choice)?
                    .is_needed(true)
                    .map_err(Error::Choice)?;
                if needed {
                    self.backend
                        .publish_decision(state, owner, choose_tech::TYPE_HASH, [-1, -1], None)
                        .map_err(Error::Notification)?;
                }
                self.backend
                    .reached
                    .push("research current choose-technology predicate");
                Ok(())
            }
            other => Err(Error::Effect(other)),
        }
    }
    fn query(
        &mut self,
        state: &mut LoadedState,
        _: &research::Definitions,
        query: research::Query,
    ) -> Result<i32, Error> {
        self.backend.check(state).map_err(Error::Notification)?;
        Err(Error::Query(query))
    }
}
