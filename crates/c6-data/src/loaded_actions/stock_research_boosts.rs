//! Source-qualified current civic boost consumers for the stock backend.
//! Rates and quest data stay with their canonical current owners. Presentation
//! DTOs, achievements and telemetry do not run native host/UI machinery.
use super::{Effects, Error};
use crate::loaded_actions::{notifications::processing, stock_notifications::Runtime};
use crate::{game_update_state::round_quests, research};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
use std::cell::RefCell;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CivicConsumer {
    Eras,
    Quests,
    Achievements,
}
pub struct Inputs<'a> {
    binding: WireBinding,
    consumers: Vec<CivicConsumer>,
    quests: &'a RefCell<round_quests::Runtime>,
    culture_report: bool,
    civic_notice: bool,
}
impl<'a> Inputs<'a> {
    /// Ordered observed GameSignals+4b0 subscribers and finite stock report /
    /// notification bindings, admitted from the same current BEFORE.
    pub fn from_initial(
        state: &LoadedState,
        consumers: Vec<CivicConsumer>,
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
            culture_report,
            civic_notice,
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
        if self.research_boosts.is_some() || inputs.binding != *state.binding() {
            return Err(processing::Error::Unsupported(
                "current research boost binding",
            ));
        }
        self.research_boosts = Some(inputs);
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
            .ok_or_else(|| Error::Effect(effect.clone()))?;
        if inputs.binding != *state.binding() || kind != research::Kind::Civic {
            return Err(Error::Effect(effect));
        }
        for consumer in &inputs.consumers {
            match consumer {
                CivicConsumer::Eras => {
                    let rate = state
                        .game_era_boost_rates(player)
                        .map_err(|_| Error::Effect(effect.clone()))?
                        .map(|v| v[1]);
                    if rate.is_some_and(|v| v != 0) {
                        return Err(Error::Acquisition {
                            effect,
                            reason: "positive civic-boost EraScore continuation".into(),
                        });
                    }
                }
                CivicConsumer::Quests => {
                    let quests = inputs.quests.borrow();
                    quests
                        .validate(state)
                        .map_err(|_| Error::Effect(effect.clone()))?;
                    // 2c99150 returns before reading the quest type/owner/civic
                    // filters on an empty current roster. Positive matching
                    // quest completion remains explicit, never discarded.
                    if !quests.current.is_empty() {
                        return Err(Error::Acquisition {
                            effect,
                            reason: "current civic boost quest completion".into(),
                        });
                    }
                }
                CivicConsumer::Achievements => {
                    // 28f8680 updates achievement-only counters/unlocks. No
                    // gameplay owner, callback or RNG is touched by that path.
                }
            }
        }
        self.backend
            .reached
            .push("stock civic boost current Era/quest subscribers");
        Ok(())
    }
    pub(super) fn boost_report(
        &mut self,
        state: &mut LoadedState,
        definitions: &research::Definitions,
        report: &c6_core::relic_creation::boost::Report,
    ) -> Result<(), Error> {
        let effect = research::Effect::Report(*report);
        if !self
            .backend
            .research_boosts
            .as_ref()
            .is_some_and(|v| v.binding == *state.binding() && v.culture_report)
        {
            return Err(Error::Effect(effect));
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
            .is_some_and(|v| v.binding == *state.binding() && v.civic_notice)
        {
            return Err(Error::Effect(effect));
        }
        // BoostCivic inherits empty Instance.OnAdd2d2e4b0. Duplicate detection,
        // localized message formatting and native pool/cache objects are UI.
        if self.culture_can_notify(state, notice.player)? {
            self.backend
                .publish_notice(
                    state,
                    crate::loaded_actions::stock_notifications::Notice::CivicBoosted {
                        owner: notice.player,
                        civic: notice.index,
                        old_progress: notice
                            .old_progress
                            .ok_or_else(|| Error::Effect(effect.clone()))?,
                        source: notice.source.ok_or_else(|| Error::Effect(effect.clone()))?,
                        percent: notice
                            .percent
                            .ok_or_else(|| Error::Effect(effect.clone()))?,
                    },
                )
                .map_err(Error::Notification)?;
        }
        Ok(())
    }
}
