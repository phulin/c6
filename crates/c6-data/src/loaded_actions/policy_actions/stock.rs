//! Source-ordered stock PolicyEnacted/PolicyRetracted consumers.
//! Native addresses are decoded into these bindings by initialization only.
use super::{modifiers, Definitions};
use crate::loaded_actions::{
    session::policy::Services,
    stock_notifications::{self, Runtime},
    stock_script,
};
use c6_core::{government_policy::Event, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Listener {
    GameEffects,
    Script,
    Gossip,
    Achievement,
}
/// Fixed stock source consumers. Additional/custom listeners require their
/// concrete gameplay implementation before a caller can initialize this scope.
pub struct Stock<'a> {
    pub enacted: &'a [Listener],
    pub retracted: &'a [Listener],
    pub modifiers: &'a modifiers::Definitions,
}
#[derive(Debug)]
pub enum Error {
    Modifiers(stock_notifications::policy_modifiers::Error),
    Script(stock_script::Error),
    Unsupported(&'static str),
}
struct YieldReports;
impl crate::city_production_turn::modifier_effects::Effects for YieldReports {
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: crate::city_production_turn::modifier_effects::Report,
    ) -> Result<(), crate::city_production_turn::Error> {
        // Same stock ef1e2348 cache-only closure as ordinary-building effects.
        // Current yields read canonical changes; no native DTO cache is needed.
        Ok(())
    }
}
impl<'a, P> Services<'a, P> for Stock<'_> {
    type Error = Error;
    fn policy_event(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'a, P>,
        owner: i32,
        event: Event,
        defs: &Definitions,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), Error> {
        let (policy, added, listeners) = match event {
            Event::Enacted(p) => (p, true, self.enacted),
            Event::Retracted(p) => (p, false, self.retracted),
            // 44855c91 and d559f80d are source stock cache publications; their
            // numeric/read-only consumers cannot change gameplay or draw RNG.
            Event::PolicyChanged(_) | Event::BeginBatch | Event::EndBatch | Event::BatchChanged => {
                return Ok(())
            }
        };
        for listener in listeners {
            match listener {
                Listener::GameEffects => backend
                    .policy_modifier_event(
                        state,
                        owner,
                        policy,
                        defs.policy_hash(policy)
                            .ok_or(Error::Unsupported("current Policy definition"))?,
                        added,
                        self.modifiers,
                        &mut YieldReports,
                    )
                    .map_err(Error::Modifiers)?,
                Listener::Script => {
                    // Native PolicyChanged has owner/policy and two bools:
                    // enact=(true,false), retract=(false,false). Empty actual
                    // groups execute the same registry lookup/indexer path;
                    // nonempty callbacks are rejected by call_empty.
                    backend
                        .script
                        .call_empty(state, "PolicyChanged", &[owner, policy])
                        .map_err(Error::Script)?;
                }
                Listener::Gossip => {
                    // Native AddGossipItem29b25d0 checks the live history extent
                    // before archive/delivery. Never substitute BEFORE emptiness.
                    if !history.history.is_empty() {
                        return Err(Error::Unsupported("policy current Gossip delivery"));
                    }
                }
                // 29002d0 only checks static policy/achievement eligibility and
                // submits an external achievement; no rules/RNG mutation.
                Listener::Achievement => {}
            }
        }
        Ok(())
    }
}
