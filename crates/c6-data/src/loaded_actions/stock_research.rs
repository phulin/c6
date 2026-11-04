//! Research turn composition borrowing the SAME stock action backend.
//! Canonical science/progress calculation stays in `research::Context`.
use super::Runtime;
use crate::loaded_actions::{choose_tech, culture_notifications, notifications::processing};
use crate::research;
use c6_core::loaded_state::LoadedState;
#[derive(Debug)]
pub enum Error {
    Choice(choose_tech::Error),
    Culture(culture_notifications::Error),
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
/// Current raw owner data and admitted culture query definitions. Each optional
/// field is required only by the source branch that actually reads it.
pub struct CultureInputs<'a> {
    pub binding: c6_core::core_player::initialization::WireBinding,
    pub definitions: &'a culture_notifications::Definitions,
    pub graph: Option<&'a culture_notifications::GameCulture>,
    pub starting: Option<&'a culture_notifications::StartingContext>,
    pub national_park_count: Option<usize>,
    pub dark_ages: Option<&'a [u8]>,
    pub golden_ages: Option<&'a [u8]>,
    pub current_era_start_turn: Option<i32>,
    pub ever_alive_major_count: Option<usize>,
}
/// Borrowed composition; no ScriptState, policy, World or research runtime is
/// cloned. Definitions and optional current graphs are retained by the caller.
/// Unknown positive acquisition/query effects fail only when actually reached.
pub struct Effects<'s, 'a, P> {
    pub backend: &'s mut Runtime<'a, P>,
    pub runtime: &'s mut research::Runtime,
    pub choices: &'s choose_tech::Definitions,
    pub choice_graph: Option<&'s choose_tech::GameTechs>,
    pub culture: Option<&'s CultureInputs<'s>>,
}
fn current_age_flag(values: Option<&[u8]>, owner: i32) -> Option<i32> {
    if owner == -1 {
        return Some(0);
    }
    let index = usize::try_from(owner).ok()?;
    Some(values?.get(index).is_some_and(|value| *value != 0) as i32)
}
impl<P> Effects<'_, '_, P> {
    fn culture_can_notify(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        if !self
            .backend
            .facts
            .manager_initialized
            .ok_or(Error::Notification(processing::Error::Unsupported(
                "culture notification manager initialization",
            )))?
        {
            return Ok(false);
        }
        if self
            .backend
            .facts
            .game_uninitializing
            .ok_or(Error::Notification(processing::Error::Unsupported(
                "culture notification game-uninitializing",
            )))?
        {
            return Ok(false);
        }
        Ok(state
            .partial_player_controls(owner)
            .map_err(|e| {
                Error::Notification(processing::Error::Gameplay(super::super::Error::State(e)))
            })?
            .slot_status
            == 3)
    }
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
        definitions: &research::Definitions,
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
            research::Effect::ChooseCivic(owner) => {
                if !self.culture_can_notify(state, owner)? {
                    return Ok(());
                }
                let culture = self.culture.ok_or(Error::Effect(effect.clone()))?;
                if culture.binding != *state.binding() {
                    return Err(Error::Effect(effect));
                }
                let needed = culture
                    .definitions
                    .player(state, owner, culture.graph, None, culture.starting)
                    .map_err(Error::Culture)?
                    .is_needed(culture_notifications::Kind::ChooseCivic, true)
                    .map_err(Error::Culture)?;
                if needed {
                    self.backend
                        .publish_decision(state, owner, 0xe1d3d3ac, [-1, -1], None)
                        .map_err(Error::Notification)?;
                }
                self.backend
                    .reached
                    .push("research current choose-civic predicate");
                Ok(())
            }
            research::Effect::GovernmentNotifications(owner) => {
                if !self.culture_can_notify(state, owner)? {
                    return Ok(());
                }
                let culture = self.culture.ok_or(Error::Effect(effect.clone()))?;
                if culture.binding != *state.binding() {
                    return Err(Error::Effect(effect));
                }
                for kind in [
                    culture_notifications::Kind::ConsiderGovernmentChange,
                    culture_notifications::Kind::FillCivicSlot,
                ] {
                    let needed = culture
                        .definitions
                        .player(state, owner, culture.graph, None, culture.starting)
                        .map_err(Error::Culture)?
                        .is_needed(kind, true)
                        .map_err(Error::Culture)?;
                    if needed {
                        self.backend
                            .publish_decision(state, owner, kind.type_hash(), [-1, -1], None)
                            .map_err(Error::Notification)?;
                    }
                }
                self.backend
                    .reached
                    .push("research current government-policy predicates");
                Ok(())
            }
            research::Effect::Report(ref report)
                if matches!(report.event, 0xc5613042 | 0xac2769a3 | 0x80f1366d) =>
            {
                research::Context {
                    state,
                    definitions,
                    effects: self,
                }
                .stock_civic_publication_yields(report)
                .map_err(|_| Error::Effect(effect))?;
                self.backend
                    .reached
                    .push("stock culture publication current yield queries");
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
        if matches!(query, research::Query::PlayerAge { owner: -1, .. }) {
            return Ok(0);
        }
        let Some(culture) = self.culture else {
            return Err(Error::Query(query));
        };
        if culture.binding != *state.binding() {
            return Err(Error::Query(query));
        }
        match query {
            research::Query::NationalParkCount => culture
                .national_park_count
                .and_then(|n| i32::try_from(n).ok())
                .ok_or(Error::Query(query)),
            research::Query::CurrentEraStartTurn => {
                culture.current_era_start_turn.ok_or(Error::Query(query))
            }
            research::Query::EverAliveMajorsCount => culture
                .ever_alive_major_count
                .and_then(|n| i32::try_from(n).ok())
                .ok_or(Error::Query(query)),
            research::Query::PlayerAge { owner, golden } => current_age_flag(
                if golden {
                    culture.golden_ages
                } else {
                    culture.dark_ages
                },
                owner,
            )
            .ok_or(Error::Query(query)),
            _ => Err(Error::Query(query)),
        }
    }
}

#[cfg(test)]
mod current_age_tests {
    use super::current_age_flag;
    #[test]
    fn known_empty_age_arrays_differ_from_missing_inputs() {
        assert_eq!(current_age_flag(None, 0), None);
        assert_eq!(current_age_flag(Some(&[]), 0), Some(0));
        assert_eq!(current_age_flag(None, -1), Some(0));
        assert_eq!(current_age_flag(Some(&[0, 2]), 1), Some(1));
        assert_eq!(current_age_flag(Some(&[1]), 8), Some(0));
        assert_eq!(current_age_flag(Some(&[]), -2), None);
    }
}
