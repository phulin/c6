//! Research turn composition borrowing the SAME stock action backend.
//! Canonical science/progress calculation stays in `research::Context`.
#[path = "stock_research_acquisition.rs"]
pub mod acquisition;
#[path = "stock_research_boosts.rs"]
pub mod boosts;
#[path = "stock_research_units.rs"]
pub mod units;
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
    Acquisition {
        effect: research::Effect,
        reason: String,
    },
    Query(research::Query),
    CityYield(crate::city_turn::Error),
    UnitUpdates(crate::research::units::Error<super::movement::Error>),
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
/// Read-only current query views shared by every ordinary research callback.
#[derive(Clone, Copy)]
pub struct Queries<'a> {
    pub graph: Option<&'a choose_tech::GameTechs>,
    pub visibility: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_research_queries(
        &mut self,
        state: &LoadedState,
        inputs: Queries<'a>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if self.research_queries.is_some()
            || inputs
                .graph
                .is_some_and(|graph| graph.binding() != state.binding())
        {
            return Err(processing::Error::Unsupported(
                "current research query binding",
            ));
        }
        self.research_queries = Some(inputs);
        Ok(())
    }
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
    pub sight: Option<&'s crate::unit_sight_state::Runtime>,
}
impl<'s, 'a, P> Effects<'s, 'a, P> {
    pub fn new(
        backend: &'s mut Runtime<'a, P>,
        runtime: &'s mut research::Runtime,
        choices: &'s choose_tech::Definitions,
        culture: Option<&'s CultureInputs<'s>>,
    ) -> Self {
        let choice_graph = backend.research_queries.and_then(|queries| queries.graph);
        Self {
            backend,
            runtime,
            choices,
            choice_graph,
            culture,
            sight: None,
        }
    }
    pub fn with_sight(mut self, sight: Option<&'s crate::unit_sight_state::Runtime>) -> Self {
        self.sight = sight;
        self
    }
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
impl<P> Runtime<'_, P> {
    /// Admit the stock owner-only culture-yield report after the caller has
    /// qualified its current cache delegate, host translator, Utility provider
    /// and absence of synchronous host hooks from the same initial state.
    pub fn bind_civic_overflow_report(
        &mut self,
        state: &LoadedState,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if self.civic_overflow_report.is_some() {
            return Err(processing::Error::Unsupported(
                "current civic overflow report already bound",
            ));
        }
        self.civic_overflow_report = Some(state.binding().clone());
        Ok(())
    }

    /// A successfully selected queue satisfies this semantic choice. Native
    /// presentation-object expiration is independent of pending arena decisions.
    pub(crate) fn resolve_research_choice(&mut self, owner: i32, kind: research::Kind) {
        resolve_research_decision(&mut self.decisions, owner, kind);
    }

    /// Bind an explicit external DEFER for technology selection requests made
    /// during automatic acquisition callbacks. Ordinary player choices remain
    /// separate and the canonical research queue is unchanged.
    pub fn defer_technology_acquisition_choices(
        &mut self,
        state: &LoadedState,
        player: i32,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        state
            .research_components(player)
            .map_err(|_| processing::Error::Unsupported("automatic research policy player"))?;
        if !self.acquisition_policies.defer_players.contains(&player) {
            self.acquisition_policies.defer_players.push(player);
        }
        Ok(())
    }
    /// Apply the same explicit policy response at a source-qualified NPC
    /// activation selection boundary. This records legal choices and changes
    /// no canonical research queue; ordinary player actions remain external.
    pub fn deferred_research_response(
        &mut self,
        state: &LoadedState,
        player: i32,
        legal_indices: &[i32],
    ) -> Result<research::policy::AcquisitionResponse, processing::Error> {
        self.check(state)?;
        Ok(self.acquisition_policies.respond(player, legal_indices))
    }
    pub fn deferred_acquisition_decisions(&self) -> &[research::policy::DeferredAcquisition] {
        &self.acquisition_policies.decisions
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
    fn acquisition_research_response(
        &mut self,
        state: &LoadedState,
        player: i32,
        legal_indices: &[i32],
    ) -> Result<research::policy::AcquisitionResponse, Self::Error> {
        self.backend
            .deferred_research_response(state, player, legal_indices)
            .map_err(Error::Notification)
    }
    fn effect(
        &mut self,
        state: &mut LoadedState,
        definitions: &research::Definitions,
        effect: research::Effect,
    ) -> Result<(), Error> {
        self.backend.check(state).map_err(Error::Notification)?;
        match effect {
            research::Effect::BoostSignal {
                player,
                kind,
                index,
            } => self.boost_signal(state, player, kind, index),
            research::Effect::Report(ref report)
                if matches!(report.event, 0x3feaad02 | 0xbf48fda8)
                    && report.specialization.is_some()
                    && report.index.is_none()
                    && report.old_progress.is_some()
                    && report.source == Some(0) =>
            {
                self.boost_report(state, definitions, report)
            }
            research::Effect::Notification(ref notice)
                if matches!(
                    (notice.kind, notice.notification_type),
                    (research::Kind::Civic, 0xf94c970d) | (research::Kind::Technology, 0x223fbe46)
                ) && notice.old_progress.is_some()
                    && notice.source == Some(0)
                    && notice.percent.is_some() =>
            {
                self.boost_notice(state, notice)
            }
            research::Effect::BoostLog { .. } | research::Effect::BoostTelemetry { .. } => {
                if self.backend.research_boosts.is_none() {
                    return Err(Error::Effect(effect));
                }
                self.backend
                    .reached
                    .push("stock boost logging and telemetry presentation boundary");
                Ok(())
            }
            research::Effect::Notification(ref notice)
                if ((notice.kind == research::Kind::Technology
                    && notice.notification_type == 0x715a6581)
                    || (notice.kind == research::Kind::Civic
                        && notice.notification_type == 0xb73826b0))
                    && notice.old_progress.is_none()
                    && notice.source.is_none()
                    && notice.percent.is_none() =>
            {
                // Stock TechDiscovered/CivicDiscovered OnAdd is empty; the
                // downstream cache/host delivery is presentation-only. Keep the
                // typed notice on the same backend instead of native UI pools.
                if self.culture_can_notify(state, notice.player)? {
                    self.backend
                        .publish_notice(
                            state,
                            match notice.kind {
                                research::Kind::Technology => super::Notice::TechnologyCompleted {
                                    owner: notice.player,
                                    technology: notice.index,
                                },
                                research::Kind::Civic => super::Notice::CivicCompleted {
                                    owner: notice.player,
                                    civic: notice.index,
                                },
                            },
                        )
                        .map_err(Error::Notification)?;
                }
                self.backend
                    .reached
                    .push("stock research completion notice manager guards");
                Ok(())
            }
            research::Effect::GameAcquired {
                player,
                kind,
                index,
            } => self.game_acquired(state, definitions, player, kind, index),
            research::Effect::GainAnalysis { player, kind } => {
                self.gain_analysis(state, player, kind)
            }
            research::Effect::DistrictCosts(player) => {
                research::Context {
                    state,
                    definitions,
                    effects: self,
                }
                .recompute_ordinary_district_costs(player)
                .map_err(|_| Error::Effect(research::Effect::DistrictCosts(player)))?;
                self.backend
                    .reached
                    .push("stock research current non-specialty district costs");
                Ok(())
            }
            research::Effect::UpdateUnitsForTech(player) => {
                self.backend.update_research_units(state, player)
            }
            research::Effect::PlayerAcquired {
                player,
                kind,
                index,
            } => self.player_acquired(state, definitions, player, kind, index),
            research::Effect::ChooseTechnology(owner) => {
                let needed = self
                    .choices
                    .player(
                        state,
                        owner,
                        self.choice_graph
                            .or(self.backend.research_queries.and_then(|q| q.graph)),
                    )
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
                let technology = self
                    .choices
                    .player(state, owner, self.choice_graph)
                    .map_err(Error::Choice)?;
                let needed = culture
                    .definitions
                    .player(
                        state,
                        owner,
                        culture.graph,
                        Some(&technology),
                        culture.starting,
                    )
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
                let technology = self
                    .choices
                    .player(state, owner, self.choice_graph)
                    .map_err(Error::Choice)?;
                for kind in [
                    culture_notifications::Kind::ConsiderGovernmentChange,
                    culture_notifications::Kind::FillCivicSlot,
                ] {
                    let needed = culture
                        .definitions
                        .player(
                            state,
                            owner,
                            culture.graph,
                            Some(&technology),
                            culture.starting,
                        )
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
                if matches!(report.event, 0x48062f97 | 0x4378a305 | 0xf621c5ac) =>
            {
                research::Context {
                    state,
                    definitions,
                    effects: self,
                }
                .stock_technology_publication_yields(report)
                .map_err(|e| Error::Acquisition {
                    effect,
                    reason: format!("{e:?}"),
                })?;
                self.backend
                    .reached
                    .push("stock technology publication current yield queries");
                Ok(())
            }
            research::Effect::Report(ref report) if report.event == 0x8aba2360 => {
                if self.backend.civic_overflow_report.as_ref() != Some(state.binding())
                    || report.specialization.is_some()
                    || report.index.is_some()
                    || report.old_progress.is_some()
                    || report.source.is_some()
                {
                    return Err(Error::Effect(effect));
                }
                // The canonical overflow store already happened. The stock
                // OnCultureYieldChanged delegate uses the same current query
                // path as other civic publications; retain those effects while
                // omitting its discarded native cache and host UI payloads.
                research::Context {
                    state,
                    definitions,
                    effects: self,
                }
                .stock_civic_publication_yields(report)
                .map_err(|_| Error::Effect(effect))?;
                self.backend
                    .reached
                    .push("stock civic overflow report current yield queries");
                Ok(())
            }
            research::Effect::Report(ref report)
                if matches!(
                    report.event,
                    0xc5613042 | 0xac2769a3 | 0x87667464 | 0x80f1366d
                ) =>
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
        if let research::Query::CityYield { city, yield_index } = query {
            return self
                .backend
                .current_city_yield(state, i32::from(city.owner), city.id, yield_index, 0)
                .map_err(Error::CityYield);
        }
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

// A semantic global research or civic policy choice has no physical target. Native invalid
// location sentinels are presentation hints, not separate policy requests.
pub(super) fn insert_pending_decision(
    decisions: &mut Vec<super::Decision>,
    decision: super::Decision,
) {
    if decisions
        .iter()
        .any(|old| old == &decision || same_global_progression_choice(old, &decision))
    {
        return;
    }
    decisions.push(decision);
}
pub(super) fn same_global_progression_choice(a: &super::Decision, b: &super::Decision) -> bool {
    a.component.is_none()
        && b.component.is_none()
        && a.owner == b.owner
        && a.type_hash == b.type_hash
        && matches!(
            a.type_hash,
            choose_tech::TYPE_HASH | 0xe1d3_d3ac | 0xb6bf_b191 | 0x6331_a2b6
        )
}
fn resolve_research_decision(
    decisions: &mut Vec<super::Decision>,
    owner: i32,
    kind: research::Kind,
) {
    let selected = match kind {
        research::Kind::Technology => choose_tech::TYPE_HASH,
        research::Kind::Civic => culture_notifications::Kind::ChooseCivic.type_hash(),
    };
    decisions.retain(|d| d.owner != owner || d.type_hash != selected);
}
#[cfg(test)]
mod decision_tests {
    use super::*;
    #[test]
    fn global_progression_sentinels_do_not_duplicate_a_policy_choice() {
        let d = |owner, type_hash, location, component| super::super::Decision {
            owner,
            type_hash,
            location,
            component,
        };
        let mut pending = vec![];
        let tech = d(0, choose_tech::TYPE_HASH, [-1, -1], None);
        assert!(same_global_progression_choice(
            &tech,
            &d(0, choose_tech::TYPE_HASH, [-9999, -9999], None)
        ));
        insert_pending_decision(&mut pending, tech.clone());
        insert_pending_decision(
            &mut pending,
            d(0, choose_tech::TYPE_HASH, [-9999, -9999], None),
        );
        let other = d(1, choose_tech::TYPE_HASH, [-9999, -9999], None);
        insert_pending_decision(&mut pending, other.clone());
        let civic = d(
            0,
            culture_notifications::Kind::ChooseCivic.type_hash(),
            [-9999, -9999],
            None,
        );
        insert_pending_decision(&mut pending, civic.clone());
        let targeted = d(
            0,
            choose_tech::TYPE_HASH,
            [-1, -1],
            Some(c6_core::notifications::ComponentId {
                owner: 0,
                component_type: 2,
                id: 65536,
            }),
        );
        insert_pending_decision(&mut pending, targeted.clone());
        let policy = d(
            0,
            culture_notifications::Kind::FillCivicSlot.type_hash(),
            [-1, -1],
            None,
        );
        insert_pending_decision(&mut pending, policy.clone());
        insert_pending_decision(
            &mut pending,
            d(
                0,
                culture_notifications::Kind::FillCivicSlot.type_hash(),
                [-9999, -9999],
                None,
            ),
        );
        assert_eq!(pending, vec![tech, other, civic, targeted, policy]);
    }
    #[test]
    fn selected_research_preserves_other_players_civics_and_government_choices() {
        let d = |owner, type_hash| super::super::Decision {
            owner,
            type_hash,
            location: [-1, -1],
            component: None,
        };
        let next_civic = d(0, culture_notifications::Kind::ChooseCivic.type_hash());
        let other = d(1, choose_tech::TYPE_HASH);
        let government = d(0, culture_notifications::Kind::FillCivicSlot.type_hash());
        let mut pending = vec![
            d(0, choose_tech::TYPE_HASH),
            next_civic.clone(),
            other.clone(),
            government.clone(),
        ];
        resolve_research_decision(&mut pending, 0, research::Kind::Technology);
        assert_eq!(pending, vec![next_civic, other.clone(), government.clone()]);
        resolve_research_decision(&mut pending, 0, research::Kind::Civic);
        assert_eq!(pending, vec![other, government]);
    }
}
