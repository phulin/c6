//! Source-qualified acquisition consumers from the current loaded game.
//! Registration bindings are runtime input, distinct from saved research state.
use super::{Effects, Error, Runtime};
use crate::research;
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Clone, Debug)]
pub struct Binding {
    pub binding: WireBinding,
    pub players: Vec<Player>,
    pub game_technology: Option<Vec<GameSubscriber>>,
    pub game_civic: Option<Vec<GameSubscriber>>,
}
#[derive(Clone, Debug)]
pub struct Player {
    pub owner: i32,
    /// The sole acquired subscriber is ResearchAI::ResearchCompleted, with its
    /// actual receiver bound to this player and an unadjusted member callable.
    pub technology: Option<TechnologySubscriber>,
    pub civic: Option<CivicSubscriber>,
    pub gain_technology: Option<AnalysisSubscriber>,
    pub gain_civic: Option<AnalysisSubscriber>,
}
#[derive(Clone, Copy, Debug)]
pub enum TechnologySubscriber {
    ResearchCompletedWithoutContracts,
}
#[derive(Clone, Copy, Debug)]
pub enum CivicSubscriber {
    CivicAi,
}
/// Ordered Targets::OnAiEvent(NULL) then BehaviorTree::OnAiEvent.
#[derive(Clone, Copy, Debug)]
pub enum AnalysisSubscriber {
    /// The event is -1/out of range or both configured tree names are empty.
    NoTriggeredTree,
    /// Military name is empty and CreateOneUseBehaviorTree returns for humans.
    HumanTreeGuard,
}
/// Source order is preserved, including repeated player and boost callbacks.
#[derive(Clone, Debug)]
pub enum GameSubscriber {
    Diplomacy { owner: i32 },
    NoAttachedModifiers,
    PlayerCatchup { owner: i32 },
    EmptyQuests,
    BoostReference { index: Option<i32> },
    EmptyGossipVisibility,
    ZeroEraScore { owners: Vec<i32> },
    StartEraHistory,
    EmptyScriptRegistry,
    FewerThanEightEverMajors { count: usize },
}
impl<P> Runtime<'_, P> {
    pub fn bind_research_acquisition(
        &mut self,
        state: &LoadedState,
        binding: Binding,
    ) -> Result<(), super::processing::Error> {
        self.check(state)?;
        if binding.binding != *state.binding() {
            return Err(super::processing::Error::Unsupported(
                "research acquisition binding",
            ));
        }
        for (i, player) in binding.players.iter().enumerate() {
            state.research_components(player.owner).map_err(|_| {
                super::processing::Error::Unsupported("research acquisition player")
            })?;
            if binding.players[..i].iter().any(|p| p.owner == player.owner) {
                return Err(super::processing::Error::Unsupported(
                    "duplicate research acquisition player",
                ));
            }
        }
        self.research_acquisition = Some(binding);
        Ok(())
    }
}
impl<P> Effects<'_, '_, P> {
    pub(super) fn game_acquired(
        &mut self,
        state: &mut LoadedState,
        definitions: &research::Definitions,
        player: i32,
        kind: research::Kind,
        index: i32,
    ) -> Result<(), Error> {
        let effect = research::Effect::GameAcquired {
            player,
            kind,
            index,
        };
        let binding =
            self.backend
                .research_acquisition
                .as_ref()
                .ok_or_else(|| Error::Acquisition {
                    effect: effect.clone(),
                    reason: "missing current game acquisition binding or callback list".into(),
                })?;
        if binding.binding != *state.binding() {
            return Err(Error::Acquisition {
                effect,
                reason: "game acquisition binding identity differs".into(),
            });
        }
        let callbacks = match kind {
            research::Kind::Technology => &binding.game_technology,
            research::Kind::Civic => &binding.game_civic,
        }
        .as_ref()
        .ok_or_else(|| Error::Acquisition {
            effect: effect.clone(),
            reason: "missing current game acquisition binding or callback list".into(),
        })?
        .clone();
        for (position, callback) in callbacks.into_iter().enumerate() {
            let callback_name = format!("callback {position} {callback:?}");
            let failure = || Error::Acquisition {
                effect: effect.clone(),
                reason: callback_name.clone(),
            };
            let source_failure = |reason: String| Error::Acquisition {
                effect: effect.clone(),
                reason: format!("{callback_name}: {reason}"),
            };
            match callback {
                GameSubscriber::Diplomacy { owner } => {
                    if owner == player {
                        if kind == research::Kind::Civic {
                            if definitions
                                .civic_acquisition_filters(index)
                                .map_err(|e| source_failure(format!("{e:?}")))?
                                .3
                            {
                                return Err(failure());
                            }
                        } else {
                            let alive = self
                                .runtime
                                .alive_majors
                                .as_ref()
                                .ok_or_else(failure)?
                                .clone();
                            for other in alive {
                                if other != owner {
                                    research::diplomacy::recheck(
                                        state,
                                        definitions.diplomatic_visibility(),
                                        &self.runtime.civilization_levels,
                                        owner,
                                        other,
                                        |_, _, _| Err(()),
                                    )
                                    .map_err(|e| source_failure(format!("{e:?}")))?;
                                }
                            }
                        }
                    }
                }
                GameSubscriber::NoAttachedModifiers => {
                    let attached = if kind == research::Kind::Civic {
                        definitions
                            .civic_acquisition_filters(index)
                            .map_err(|e| source_failure(format!("{e:?}")))?
                            .0
                    } else {
                        definitions
                            .technology_has_attached_modifiers(index)
                            .map_err(|e| source_failure(format!("{e:?}")))?
                    };
                    if attached {
                        return Err(failure());
                    }
                }
                GameSubscriber::PlayerCatchup { owner } => {
                    let mut context = research::Context {
                        state,
                        definitions,
                        effects: self,
                    };
                    if kind == research::Kind::Civic {
                        context.stock_player_civic_catchup(owner, player, index)
                    } else {
                        context.stock_player_technology_catchup(owner, player, index)
                    }
                    .map_err(|e| source_failure(format!("{e:?}")))?;
                    if owner == player {
                        let unlock = if kind == research::Kind::Civic {
                            let filters = definitions
                                .civic_acquisition_filters(index)
                                .map_err(|e| source_failure(format!("{e:?}")))?;
                            if filters.2 {
                                return Err(failure());
                            }
                            filters.1
                        } else {
                            definitions
                                .technology_has_resource_unlock(index)
                                .map_err(|e| source_failure(format!("{e:?}")))?
                        };
                        if unlock {
                            return Err(failure());
                        }
                    }
                }
                GameSubscriber::EmptyQuests | GameSubscriber::EmptyGossipVisibility => {}
                GameSubscriber::BoostReference { index: reference } => {
                    if reference == Some(index) {
                        return Err(failure());
                    }
                }
                GameSubscriber::ZeroEraScore { owners } => {
                    if !owners.contains(&player) {
                        return Err(failure());
                    }
                }
                GameSubscriber::StartEraHistory => {
                    let era = if kind == research::Kind::Civic {
                        definitions.civic_era_hash(index)
                    } else {
                        definitions.technology_era_hash(index)
                    }
                    .map_err(|e| source_failure(format!("{e:?}")))?;
                    if era != self.runtime.start_era_hash {
                        return Err(failure());
                    }
                }
                GameSubscriber::EmptyScriptRegistry => {
                    self.backend
                        .script
                        .call_two_empty(
                            state,
                            if kind == research::Kind::Civic {
                                "OnCivicCulturevated"
                            } else {
                                "OnTechnologyResearched"
                            },
                            [player, index],
                        )
                        .map_err(|e| source_failure(format!("{e:?}")))?;
                }
                GameSubscriber::FewerThanEightEverMajors { count } => {
                    if count >= 8 {
                        return Err(failure());
                    }
                }
            }
        }
        self.backend
            .reached
            .push("stock game acquired ordered current callbacks");
        Ok(())
    }
    pub(super) fn gain_analysis(
        &mut self,
        state: &LoadedState,
        player: i32,
        kind: research::Kind,
    ) -> Result<(), Error> {
        let effect = research::Effect::GainAnalysis { player, kind };
        let binding = self
            .backend
            .research_acquisition
            .as_ref()
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        if binding.binding != *state.binding() {
            return Err(Error::Effect(effect));
        }
        let owner = binding
            .players
            .iter()
            .find(|p| p.owner == player)
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        let subscriber = match kind {
            research::Kind::Technology => owner.gain_technology,
            research::Kind::Civic => owner.gain_civic,
        }
        .ok_or_else(|| Error::Effect(effect.clone()))?;
        if matches!(subscriber, AnalysisSubscriber::HumanTreeGuard)
            && state
                .player_slot_status(player)
                .map_err(|_| Error::Effect(effect.clone()))?
                != 3
        {
            return Err(Error::Effect(effect));
        }
        self.backend
            .reached
            .push("stock gain analysis current null-analysis and tree guards");
        Ok(())
    }
    pub(super) fn player_acquired(
        &mut self,
        state: &mut LoadedState,
        definitions: &research::Definitions,
        player: i32,
        kind: research::Kind,
        index: i32,
    ) -> Result<(), Error> {
        let effect = research::Effect::PlayerAcquired {
            player,
            kind,
            index,
        };
        let binding = self
            .backend
            .research_acquisition
            .as_ref()
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        if binding.binding != *state.binding() {
            return Err(Error::Effect(effect));
        }
        let owner = binding
            .players
            .iter()
            .find(|p| p.owner == player)
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        match kind {
            research::Kind::Technology => {
                match owner
                    .technology
                    .ok_or_else(|| Error::Effect(effect.clone()))?
                {
                    TechnologySubscriber::ResearchCompletedWithoutContracts => {
                        research::Context {
                            state,
                            definitions,
                            effects: self,
                        }
                        .stock_research_completed_without_contracts(player, index)
                        .map_err(|e| Error::Acquisition {
                            effect,
                            reason: format!("{e:?}"),
                        })?;
                        self.backend
                            .reached
                            .push("stock research acquired current member callback");
                        Ok(())
                    }
                }
            }
            research::Kind::Civic => {
                match owner.civic.ok_or_else(|| Error::Effect(effect.clone()))? {
                    CivicSubscriber::CivicAi => {
                        if state
                            .player_slot_status(player)
                            .map_err(|_| Error::Effect(effect.clone()))?
                            != 3
                        {
                            return Err(Error::Effect(effect));
                        }
                        self.backend
                            .reached
                            .push("stock civic acquired human policy guards");
                        Ok(())
                    }
                }
            }
        }
    }
}
