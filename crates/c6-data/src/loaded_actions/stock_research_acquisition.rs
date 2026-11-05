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
    pub resource_unlocks: Option<ResourceUnlocks>,
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
#[derive(Clone, Debug)]
pub struct ResourceUnlocks {
    definitions: Vec<ResourceUnlockDefinition>,
    subscribers: Vec<u64>,
    pub dispatches: Vec<(i32, i32)>,
}
#[derive(Clone, Debug)]
struct ResourceUnlockDefinition {
    index: i32,
    hash: u32,
    technology: Option<i32>,
    civic: Option<i32>,
}
impl<P> Runtime<'_, P> {
    /// Current Resource definition order and complete Game.ResourceUnlocked
    /// registration order; no scalar/reset values are synthesized.
    pub fn bind_research_resource_unlocks(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        subscribers: Vec<u64>,
    ) -> Result<(), String> {
        self.check(state).map_err(|e| e.to_string())?;
        use crate::unit_initialization_definitions::collection;
        let resources =
            collection(rules, "Resources", "ResourceType").map_err(|e| format!("{e:?}"))?;
        let techs =
            collection(rules, "Technologies", "TechnologyType").map_err(|e| format!("{e:?}"))?;
        let civics = collection(rules, "Civics", "CivicType").map_err(|e| format!("{e:?}"))?;
        let table = rules
            .gameplay()
            .table("Resources")
            .ok_or("research Resources table")?;
        let mut definitions = Vec::new();
        for (index, definition) in resources.entries().iter().enumerate() {
            let row = table
                .row_by_id(definition.database_rowid)
                .ok_or("research Resource row")?;
            let reference = |column,
                             registry: &crate::registry::LinuxDefinitionRegistry|
             -> Result<Option<i32>, String> {
                match row.get(column).map_err(|e| e.to_string())? {
                    crate::SqlValue::Null => Ok(None),
                    crate::SqlValue::Text(name) => Ok(registry
                        .find_index(crate::registry::linux_type_hash(name))
                        .map(|i| i as i32)),
                    _ => Err(format!("research resource {column}")),
                }
            };
            definitions.push(ResourceUnlockDefinition {
                index: index as i32,
                hash: crate::registry::linux_type_hash(&definition.type_name),
                technology: reference("PrereqTech", &techs)?,
                civic: reference("PrereqCivic", &civics)?,
            });
        }
        let binding = self
            .research_acquisition
            .as_mut()
            .ok_or("research acquisition binding")?;
        if binding.binding != *state.binding() || binding.resource_unlocks.is_some() {
            return Err("research resource unlock binding differs or already installed".into());
        }
        binding.resource_unlocks = Some(ResourceUnlocks {
            definitions,
            subscribers,
            dispatches: Vec::new(),
        });
        Ok(())
    }
    pub fn research_resource_unlock_dispatches(&self) -> &[(i32, i32)] {
        self.research_acquisition
            .as_ref()
            .and_then(|binding| binding.resource_unlocks.as_ref())
            .map_or(&[], |inputs| inputs.dispatches.as_slice())
    }
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
    fn technology_resource_unlock(
        &mut self,
        state: &LoadedState,
        player: i32,
        index: i32,
    ) -> Result<(), String> {
        use c6_core::yields::{PrerequisiteKind, PrerequisiteState};
        let binding = self
            .backend
            .research_acquisition
            .as_mut()
            .ok_or("research acquisition binding")?;
        if binding.binding != *state.binding() {
            return Err("research acquisition binding differs".into());
        }
        let inputs = binding
            .resource_unlocks
            .as_mut()
            .ok_or("current resource unlock definitions and signal")?;
        for resource in &inputs.definitions {
            if resource.technology != Some(index) {
                continue;
            }
            if let Some(civic) = resource.civic {
                let mask = state
                    .world()
                    .acquired_mask(player, PrerequisiteKind::Civic)
                    .map_err(|e| format!("{e:?}"))?;
                if !mask.get(civic as usize).copied().unwrap_or(false)
                    || !state
                        .world()
                        .globally_available(PrerequisiteKind::Civic, civic)
                        .map_err(|e| format!("{e:?}"))?
                {
                    continue;
                }
            }
            // OnPlayerTechResearched dispatches this before iterating plots.
            inputs.dispatches.push((player, resource.index));
            if let Some(listener) = inputs.subscribers.first() {
                return Err(format!(
                    "Game.ResourceUnlocked subscriber {listener} for resource {}",
                    resource.index
                ));
            }
            for (plot_index, plot) in state.world().map().plots.iter().enumerate() {
                if plot.resource_hash != resource.hash {
                    continue;
                }
                let shared = if self.sight.is_none() {
                    self.backend
                        .research_queries
                        .map(|queries| queries.visibility.try_borrow())
                        .transpose()
                        .map_err(|_| "current research shared visibility already borrowed")?
                } else {
                    None
                };
                let sight = self
                    .sight
                    .or(shared.as_deref())
                    .ok_or("current research shared visibility")?;
                let normal = sight
                    .shared(player)?
                    .0
                    .iter()
                    .find(|layer| layer.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
                    .ok_or("current research normal shared layer")?;
                let visible = *normal
                    .values
                    .get(plot_index)
                    .ok_or("research shared visibility cell")?
                    > 0;
                let own = state
                    .world()
                    .visibility(player)
                    .ok_or("research retained visibility")?;
                let revealed = *own
                    .revealed
                    .get(plot_index)
                    .ok_or("research revealed cell")?;
                if revealed > 1 {
                    return Err("noncanonical resource reveal byte".into());
                }
                // UpdateRevealedAt and its caller both return for state zero.
                // No resource amounts, layers, caches, signals or RNG are
                // changed by either source method on this branch.
                if visible || revealed != 0 {
                    return Err(format!(
                        "positive resource reveal continuation resource {} plot {}",
                        resource.index, plot_index
                    ));
                }
            }
        }
        self.backend
            .reached
            .push("stock technology resource unlock signal and unrevealed plot guards");
        Ok(())
    }
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
                                return Err(source_failure(
                                    "positive civic random-agenda initialization".into(),
                                ));
                            }
                            filters.1
                        } else {
                            definitions
                                .technology_has_resource_unlock(index)
                                .map_err(|e| source_failure(format!("{e:?}")))?
                        };
                        if unlock {
                            if kind == research::Kind::Technology {
                                self.technology_resource_unlock(state, player, index)
                                    .map_err(source_failure)?;
                            } else {
                                return Err(source_failure(
                                    "positive civic resource unlock continuation".into(),
                                ));
                            }
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
                        let level = self
                            .runtime
                            .civilization_levels
                            .iter()
                            .find(|row| row.0 == player)
                            .ok_or_else(|| {
                                source_failure("current history player civilization level".into())
                            })?
                            .1;
                        // History::Manager::HandleMoment (29da970) rejects
                        // nonmajor actors at 29da9d3 before history/EraScore
                        // writes, signals or RNG. The dispatcher's preceding
                        // era/technology-count reads are pure; any Moment it
                        // constructs is temporary data only.
                        if level == 0x253718b0 {
                            return Err(source_failure(
                                "major research history moment and EraScore continuation".into(),
                            ));
                        }
                        self.backend
                            .reached
                            .push("stock research history nonmajor moment guard");
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
