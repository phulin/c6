//! Source-qualified current technology and civic boost consumers for the stock backend.
//! Rates and quest data stay with their canonical current owners. Presentation
//! DTOs, achievements and telemetry do not run native host/UI machinery.
use super::{Effects, Error};
use crate::loaded_actions::{notifications::processing, stock_notifications::Runtime};
use crate::{game_update_state::round_quests, research};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
use std::cell::RefCell;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoostConsumer {
    Eras,
    Quests,
    Achievements,
}
/// Compatibility name for callers binding civic inspirations.
pub type CivicConsumer = BoostConsumer;
pub struct Inputs<'a> {
    binding: WireBinding,
    consumers: Vec<BoostConsumer>,
    quests: &'a RefCell<round_quests::Runtime>,
    kind: research::Kind,
    stock_report: bool,
    stock_notice: bool,
    companion: Option<Box<Inputs<'a>>>,
}
impl<'a> Inputs<'a> {
    fn for_kind(&self, kind: research::Kind) -> Option<&Self> {
        if self.kind == kind {
            Some(self)
        } else {
            self.companion.as_deref().filter(|input| input.kind == kind)
        }
    }

    /// Ordered observed GameSignals+4b0 subscribers and finite stock report /
    /// notification bindings, admitted from the same current BEFORE.
    pub fn from_initial(
        state: &LoadedState,
        consumers: Vec<BoostConsumer>,
        quests: &'a RefCell<round_quests::Runtime>,
        culture_report: bool,
        civic_notice: bool,
    ) -> Result<Self, Error> {
        quests.borrow().validate(state).map_err(|_| {
            Error::Effect(research::Effect::BoostSignal {
                player: -1,
                kind: research::Kind::Civic,
                index: -1,
            })
        })?;
        Ok(Self {
            binding: state.binding().clone(),
            consumers,
            quests,
            kind: research::Kind::Civic,
            stock_report: culture_report,
            stock_notice: civic_notice,
            companion: None,
        })
    }
    /// Ordered current GameSignals+1308 subscribers. `stock_report` admits
    /// only a boost report with no native cache subscribers and the stock host
    /// translator with no synchronous pre/post/prepublication consumers.
    /// `stock_notice` qualifies the stock BoostTech factory / empty OnAdd.
    pub fn from_technology_initial(
        state: &LoadedState,
        consumers: Vec<BoostConsumer>,
        quests: &'a RefCell<round_quests::Runtime>,
        stock_report: bool,
        stock_notice: bool,
    ) -> Result<Self, Error> {
        quests.borrow().validate(state).map_err(|_| {
            Error::Effect(research::Effect::BoostSignal {
                player: -1,
                kind: research::Kind::Technology,
                index: -1,
            })
        })?;
        Ok(Self {
            binding: state.binding().clone(),
            consumers,
            quests,
            kind: research::Kind::Technology,
            stock_report,
            stock_notice,
            companion: None,
        })
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_research_boosts(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if inputs.binding != *state.binding()
            || self
                .research_boosts
                .as_ref()
                .is_some_and(|current| current.for_kind(inputs.kind).is_some())
        {
            return Err(processing::Error::Unsupported(
                "current research boost binding",
            ));
        }
        if let Some(current) = self.research_boosts.as_mut() {
            current.companion = Some(Box::new(inputs));
        } else {
            self.research_boosts = Some(inputs);
        }
        Ok(())
    }
}
impl<P> Effects<'_, '_, P> {
    pub(super) fn boost_signal(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        kind: research::Kind,
        index: i32,
    ) -> Result<(), Error> {
        let effect = research::Effect::BoostSignal {
            player,
            kind,
            index,
        };
        let inputs = self
            .backend
            .research_boosts
            .as_ref()
            .and_then(|input| input.for_kind(kind))
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        if inputs.binding != *state.binding() || kind != inputs.kind {
            return Err(Error::Effect(effect));
        }
        for consumer in &inputs.consumers {
            match consumer {
                BoostConsumer::Eras => {
                    let rate = state
                        .game_era_boost_rates(player)
                        .map_err(|_| Error::Effect(effect.clone()))?
                        .map(|v| v[usize::from(kind == research::Kind::Civic)]);
                    if rate.is_some_and(|v| v != 0) {
                        return Err(Error::Acquisition {
                            effect,
                            reason: format!("positive {kind:?}-boost EraScore continuation"),
                        });
                    }
                }
                BoostConsumer::Quests => {
                    let quests = inputs.quests.borrow();
                    quests
                        .validate(state)
                        .map_err(|_| Error::Effect(effect.clone()))?;
                    // 2c99150 (civic) / 2c99a80 (technology) return before
                    // reading quest filters on an empty current roster. Matching
                    // quest completion remains explicit, never discarded.
                    if !quests.current.is_empty() {
                        return Err(Error::Acquisition {
                            effect,
                            reason: format!("current {kind:?} boost quest completion"),
                        });
                    }
                }
                BoostConsumer::Achievements => {
                    // 28f8680 (civic) / 28f8740 (technology) update
                    // achievement-only counters/unlocks. No
                    // gameplay owner, callback or RNG is touched by that path.
                }
            }
        }
        self.backend
            .reached
            .push("stock research boost current Era/quest subscribers");
        Ok(())
    }
    pub(super) fn boost_report(
        &mut self,
        state: &mut LoadedState,
        definitions: &research::Definitions,
        report: &c6_core::relic_creation::boost::Report,
    ) -> Result<(), Error> {
        let effect = research::Effect::Report(*report);
        let kind = match report.event {
            0xbf48_fda8 => research::Kind::Technology,
            0x3fea_ad02 => research::Kind::Civic,
            _ => return Err(Error::Effect(effect)),
        };
        if !self
            .backend
            .research_boosts
            .as_ref()
            .and_then(|input| input.for_kind(kind))
            .is_some_and(|input| input.binding == *state.binding() && input.stock_report)
        {
            return Err(Error::Effect(effect));
        }
        if kind == research::Kind::Technology {
            if report.event != 0xbf48_fda8
                || report.specialization.is_none()
                || report.index.is_some()
                || report.old_progress.is_none()
                || report.source.is_none()
            {
                return Err(Error::Effect(effect));
            }
            // The qualified current bf48fda8 registration list is empty.
            // The stock host translator3210b70 copies arguments and queues
            // event178, with no synchronous consumers. Unlike civic boost
            // publication this path does not query science/culture yields.
            self.backend
                .reached
                .push("stock technology boost publication");
            return Ok(());
        }
        research::Context {
            state,
            definitions,
            effects: self,
        }
        .stock_civic_publication_yields(report)
        .map_err(|e| Error::Acquisition {
            effect,
            reason: format!("{e:?}"),
        })?;
        self.backend
            .reached
            .push("stock civic boost culture publication queries");
        Ok(())
    }
    pub(super) fn boost_notice(
        &mut self,
        state: &mut LoadedState,
        notice: &c6_core::relic_creation::boost::Notification,
    ) -> Result<(), Error> {
        let effect = research::Effect::Notification(*notice);
        if !self
            .backend
            .research_boosts
            .as_ref()
            .and_then(|input| input.for_kind(notice.kind))
            .is_some_and(|v| {
                v.binding == *state.binding() && v.stock_notice && v.kind == notice.kind
            })
        {
            return Err(Error::Effect(effect));
        }
        // BoostCivic and BoostTech inherit empty Instance.OnAdd2d2e4b0. Duplicate detection,
        // localized message formatting and native pool/cache objects are UI.
        if self.culture_can_notify(state, notice.player)? {
            use crate::loaded_actions::stock_notifications::Notice;
            let old_progress = notice
                .old_progress
                .ok_or_else(|| Error::Effect(effect.clone()))?;
            let source = notice.source.ok_or_else(|| Error::Effect(effect.clone()))?;
            let percent = notice
                .percent
                .ok_or_else(|| Error::Effect(effect.clone()))?;
            let semantic = match notice.kind {
                research::Kind::Technology => Notice::TechnologyBoosted {
                    owner: notice.player,
                    technology: notice.index,
                    old_progress,
                    source,
                    percent,
                },
                research::Kind::Civic => Notice::CivicBoosted {
                    owner: notice.player,
                    civic: notice.index,
                    old_progress,
                    source,
                    percent,
                },
            };
            self.backend
                .publish_notice(state, semantic)
                .map_err(Error::Notification)?;
        }
        Ok(())
    }
}

impl<P> Runtime<'_, P> {
    /// Synchronous population continuation on the already loaned research owner.
    pub(crate) fn population_research_boost(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        definition: usize,
    ) -> Result<(), crate::city_turn::Error> {
        use crate::city_turn::{religion_callbacks, Error as CityError};
        use crate::melee_attack::resolve::completion::boosts::{ResearchTrigger, Trigger};
        let inputs = self
            .unit_creation
            .as_ref()
            .and_then(|input| input.research)
            .ok_or(CityError::Unsupported(
                "population canonical research definitions",
            ))?;
        let mut runtime = self
            .action_research
            .take()
            .ok_or(CityError::Unsupported("population canonical research loan"))?;
        let result = {
            let mut effects = Effects::new(self, &mut runtime, inputs.choices, inputs.culture);
            ResearchTrigger {
                definitions: inputs.definitions,
                effects: &mut effects,
            }
            .trigger(state, owner, definition)
            .map_err(|error| {
                CityError::ReligionCallback(religion_callbacks::Error::BoostContinuation(format!(
                    "population boost: {error:?}"
                )))
            })
        };
        self.action_research = Some(runtime);
        result
    }
}
