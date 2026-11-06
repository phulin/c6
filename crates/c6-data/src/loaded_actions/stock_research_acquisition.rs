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
    discovered: Option<Vec<ResourceDiscoveredSubscriber>>,
    discovered_checks: Option<Vec<super::super::resource_revelation::ResourceBoostCheck>>,
    pub discovered_dispatches: Vec<(i32, i32, u32)>,
}
/// The actual Boost.Manager PlayerResourceCheck subscriber. Its definition
/// guards and unowned-plot branch perform no mutation or RNG draw; an owned
/// plot still requires the positive improvement/boost continuation.
#[derive(Clone, Copy, Debug)]
pub enum ResourceDiscoveredSubscriber {
    PlayerResourceCheck,
}
#[derive(Clone, Debug)]
struct ResourceUnlockDefinition {
    index: i32,
    hash: u32,
    technology: Option<i32>,
    civic: Option<i32>,
    compatible_improvements: Vec<u32>,
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
                compatible_improvements: rules
                    .gameplay()
                    .table("Improvement_ValidResources")
                    .ok_or("Improvement_ValidResources")?
                    .rows()
                    .filter(|row| {
                        row.text("ResourceType").ok().map(str::as_bytes)
                            == Some(definition.type_name.as_slice())
                    })
                    .map(|row| {
                        row.text("ImprovementType")
                            .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
                            .map_err(|e| e.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?,
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
            discovered: None,
            discovered_checks: None,
            discovered_dispatches: Vec::new(),
        });
        Ok(())
    }
    /// Complete current Game.ResourceDiscovered registration order, distinct
    /// from ResourceUnlocked and ResourceRevealed. Unknown subscribers must be
    /// rejected by the runtime importer rather than silently dropped.
    pub fn bind_research_resource_discovery(
        &mut self,
        state: &LoadedState,
        subscribers: Vec<ResourceDiscoveredSubscriber>,
    ) -> Result<(), String> {
        self.check(state).map_err(|e| e.to_string())?;
        let inputs = self
            .research_acquisition
            .as_mut()
            .filter(|binding| binding.binding == *state.binding())
            .and_then(|binding| binding.resource_unlocks.as_mut())
            .ok_or("research resource unlock binding")?;
        if inputs.discovered.is_some() {
            return Err("research resource discovery already bound".into());
        }
        inputs.discovered = Some(subscribers);
        Ok(())
    }
    /// Definition row identities come from each actual current callable's
    /// captured boost reference, in complete ResourceDiscovered ring order.
    pub fn bind_research_resource_discovery_definitions(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        subscribers: &[(u64, i64)],
    ) -> Result<(), String> {
        let checks = subscribers
            .iter()
            .map(|&(receiver, row)| {
                super::super::resource_revelation::resource_boost_check(rules, receiver, row)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.bind_research_resource_discovery(
            state,
            vec![ResourceDiscoveredSubscriber::PlayerResourceCheck; checks.len()],
        )?;
        self.research_acquisition
            .as_mut()
            .unwrap()
            .resource_unlocks
            .as_mut()
            .unwrap()
            .discovered_checks = Some(checks);
        Ok(())
    }
    pub fn research_resource_discovery_dispatches(&self) -> &[(i32, i32, u32)] {
        self.research_acquisition
            .as_ref()
            .and_then(|binding| binding.resource_unlocks.as_ref())
            .map_or(&[], |inputs| inputs.discovered_dispatches.as_slice())
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
impl<'a, P> Runtime<'a, P> {
    /// Shared original ChangeResourceAmount services; no second resource ledger.
    pub fn bind_research_resource_amount(
        &mut self,
        state: &LoadedState,
        definitions: &'a crate::loaded_actions::upgrade_actions::resources::Definitions,
        signals: &'a crate::loaded_actions::upgrade_actions::resources::Signals,
    ) -> Result<(), String> {
        self.check(state).map_err(|e| e.to_string())?;
        if self.research_resource_amount.is_some() {
            return Err("research resource amount already bound".into());
        }
        self.research_resource_amount = Some((definitions, signals));
        Ok(())
    }
}
impl<P> Effects<'_, '_, P> {
    fn technology_resource_unlock(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        index: i32,
        research_definitions: &research::Definitions,
    ) -> Result<(), String> {
        use c6_core::yields::{PrerequisiteKind, PrerequisiteState};
        let binding = self
            .backend
            .research_acquisition
            .as_ref()
            .ok_or("research acquisition binding")?;
        if binding.binding != *state.binding() {
            return Err("research acquisition binding differs".into());
        }
        // Immutable rule identities only; mutable runtime owners remain shared.
        let definitions = binding
            .resource_unlocks
            .as_ref()
            .ok_or("current resource unlock definitions and signal")?
            .definitions
            .clone();
        for resource in &definitions {
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
            let inputs = self
                .backend
                .research_acquisition
                .as_mut()
                .unwrap()
                .resource_unlocks
                .as_mut()
                .unwrap();
            // OnPlayerTechResearched dispatches this before iterating plots.
            inputs.dispatches.push((player, resource.index));
            if let Some(listener) = inputs.subscribers.first() {
                return Err(format!(
                    "Game.ResourceUnlocked subscriber {listener} for resource {}",
                    resource.index
                ));
            }
            let plots: Vec<u32> = state
                .world()
                .map()
                .plots
                .iter()
                .enumerate()
                .filter(|(_, plot)| plot.resource_hash == resource.hash)
                .map(|(index, _)| index as u32)
                .collect();
            for plot in plots {
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
                // Native UpdateRevealedAt always precedes the caller's own
                // GetRevealedState and mask4 refresh, even for hidden plots.
                let runtime = &mut *self.runtime;
                let choices = self.choices;
                let culture = self.culture;
                let choice_graph = self.choice_graph;
                let effect_sight = self.sight;
                self.backend
                    .update_revealed_resource_with(
                        state,
                        sight,
                        player,
                        plot,
                        |backend, state, player, boost| {
                            let mut effects = Effects {
                                backend,
                                runtime,
                                choices,
                                culture,
                                choice_graph,
                                sight: effect_sight,
                            };
                            let mut context = research::Context {
                                state,
                                definitions: research_definitions,
                                effects: &mut effects,
                            };
                            c6_core::relic_creation::boost::trigger_boost(
                                &mut context,
                                Some(player),
                                boost,
                            )
                            .map_err(|e| {
                                super::super::movement::Error::Requirement(format!(
                                    "resource reveal boost: {e:?}"
                                ))
                            })
                        },
                    )
                    .map_err(|e| e.to_string())?;
                let normal = sight
                    .shared(player)?
                    .0
                    .iter()
                    .find(|layer| layer.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
                    .ok_or("current research normal shared layer")?;
                let visible = *normal
                    .values
                    .get(plot as usize)
                    .ok_or("research shared visibility cell")?
                    > 0;
                let revealed = *state
                    .world()
                    .visibility(player)
                    .ok_or("research retained visibility")?
                    .revealed
                    .get(plot as usize)
                    .ok_or("research revealed cell")?;
                if revealed > 1 {
                    return Err("noncanonical resource reveal byte".into());
                }
                let revealed_state = if visible { 2 } else { i32::from(revealed) };
                if revealed_state == 0 {
                    continue;
                }
                if sight.local_observer.ok_or("research local observer")? == player
                    && !sight.messages_disabled
                {
                    self.backend
                        .propagate_sight(
                            state,
                            c6_core::visibility_change::Propagation {
                                player,
                                plot,
                                revealed_state,
                                mask: 4,
                                first_reveal: false,
                                origin: -1,
                                sight_range: 1,
                                source: c6_core::unit::ComponentId {
                                    owner: -1,
                                    component_type: 0,
                                    id: u32::MAX,
                                },
                            },
                        )
                        .map_err(|e| e.to_string())?;
                }
                let owner = crate::movement_placement_state::plot_owner(
                    state
                        .world()
                        .map()
                        .plot_by_index(plot)
                        .ok_or("research resource plot")?,
                )
                .map_err(|e| format!("{e:?}"))?;
                // GetResourceExtractableAt returns NO_RESOURCE for another
                // owner's plot before resource visibility/improvement checks.
                if owner == player && self.resource_extractable(state, player, plot, resource)? {
                    let count = i32::from(
                        state
                            .world()
                            .map()
                            .plot_by_index(plot)
                            .unwrap()
                            .resource_count,
                    );
                    let (definitions, signals) = self
                        .backend
                        .research_resource_amount
                        .ok_or("current resource amount services")?;
                    crate::loaded_actions::upgrade_actions::resources::change_owned_amount(
                        state,
                        self.backend,
                        definitions,
                        signals,
                        player,
                        resource.hash,
                        count,
                    )?;
                }
                let inputs = self
                    .backend
                    .research_acquisition
                    .as_mut()
                    .unwrap()
                    .resource_unlocks
                    .as_mut()
                    .unwrap();
                let subscribers = inputs
                    .discovered
                    .as_ref()
                    .ok_or("current Game.ResourceDiscovered registry")?;
                inputs
                    .discovered_dispatches
                    .push((player, resource.index, plot));
                let subscriber_count = subscribers.len();
                let checks = inputs.discovered_checks.clone();
                if let Some(checks) = checks {
                    if checks.len() != subscriber_count {
                        return Err("resource discovery binding order".into());
                    }
                    for check in checks {
                        for boost in super::super::resource_revelation::resource_boost_matches(
                            state,
                            &check,
                            player,
                            resource.index,
                            plot,
                        )
                        .map_err(|e| e.to_string())?
                        {
                            let mut context = research::Context {
                                state,
                                definitions: research_definitions,
                                effects: self,
                            };
                            c6_core::relic_creation::boost::trigger_boost(
                                &mut context,
                                Some(player),
                                boost,
                            )
                            .map_err(|e| format!("resource discovery boost: {e:?}"))?;
                        }
                    }
                } else if owner == player && subscriber_count != 0 {
                    return Err("current owned ResourceDiscovered boost definitions".into());
                }
                if owner == player {
                    // Original Send(ResourceDiscovered,PlotCoord,localized strings)
                    // is exposed as an informational headless gameplay notice.
                    self.backend
                        .publish_notice(
                            state,
                            super::super::Notice::ResourceDiscovered {
                                owner: player,
                                resource: resource.index,
                                plot,
                            },
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        self.backend
            .reached
            .push("stock technology resource unlock and plot revelation");
        Ok(())
    }
    fn resource_extractable(
        &self,
        state: &LoadedState,
        owner: i32,
        index: u32,
        resource: &ResourceUnlockDefinition,
    ) -> Result<bool, String> {
        use c6_core::map::PlotFlag;
        let plot = state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or("resource extraction plot")?;
        if plot.resource_count == 0 {
            return Ok(false);
        }
        let visible = self
            .backend
            .resource_revelation
            .as_ref()
            .ok_or("resource extraction definitions")?
            .definitions
            .visible_resource(state, owner, plot.resource_hash as i32)
            .map_err(|e| format!("{e:?}"))?;
        if visible == -1 {
            return Ok(false);
        }
        if let Some(attributes) = &plot.city_attributes {
            if attributes.district != u32::MAX {
                let player = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or("resource extraction district owner")?;
                if let Some(district) = player
                    .districts
                    .records
                    .entries
                    .iter()
                    .find(|e| e.value.id == attributes.district)
                    .map(|e| &e.value)
                {
                    if district.is_complete && !district.pillaged {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(!plot.flags.get(PlotFlag::ImprovementPillaged)
            && resource
                .compatible_improvements
                .contains(&plot.improvement_hash))
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
                                self.technology_resource_unlock(state, player, index, definitions)
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
