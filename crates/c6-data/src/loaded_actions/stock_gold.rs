//! Gold balance notification on the same persistent stock backend.
//! Treasury snapshot/host playback is presentation-only in this fixed stock
//! scope. The following gameplay signal requires current registrations.
use super::{turn_reports, Runtime};
use crate::game_update_state::activation::treasury_deals;
use c6_core::{
    core_player::initialization::WireBinding,
    game_update,
    loaded_state::{ActionStamp, LoadedState},
};
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessExternalPoliciesNoAddedReportConsumers,
}
/// Current ordered GameSignals+9f0 registration tokens, not callback answers.
/// Nonempty lists are retained and explicitly unsupported until their actual
/// callback semantics are connected. An unavailable list is distinct from [].
pub struct Signals {
    binding: WireBinding,
    instance: ActionStamp,
    listeners: Vec<u64>,
}
impl Signals {
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        listeners: Vec<u64>,
    ) -> Result<Self, Error> {
        if state.binding() != binding {
            return Err(Error::Binding);
        }
        Ok(Self {
            binding: binding.clone(),
            instance: state.action_stamp(),
            listeners,
        })
    }
}
#[derive(Debug)]
pub enum Error {
    Binding,
    MissingRegistrations,
    CurrentPlayer,
    AlreadyBound,
    UnsupportedListener {
        token: u64,
        owner: i32,
        whole_gold: i32,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gold balance notification: {self:?}")
    }
}
impl std::error::Error for Error {}
impl<P> Runtime<'_, P> {
    pub fn bind_gold_signals(
        &mut self,
        state: &LoadedState,
        signals: Signals,
    ) -> Result<(), Error> {
        self.check(state).map_err(|_| Error::Binding)?;
        if state.binding() != &signals.binding
            || !signals.instance.same_instance(state.action_stamp())
        {
            return Err(Error::Binding);
        }
        if self.gold_signals.is_some() {
            return Err(Error::AlreadyBound);
        }
        self.gold_signals = Some(signals);
        Ok(())
    }
    /// Called only after canonical SetGoldBalance has stored its new value.
    /// Report20f2116e precedes GameSignals+9f0 even when a later listener fails.
    pub fn deliver_gold_changed(
        &mut self,
        state: &LoadedState,
        owner: i32,
        _scope: Scope,
    ) -> Result<(), Error> {
        self.check(state).map_err(|_| Error::Binding)?;
        let current = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::CurrentPlayer)?;
        self.turn_report_records.push(turn_reports::Record::Report {
            event: 0x20f2116e,
            player: owner,
        });
        // Stock PlayerTreasury.OnTreasuryChanged→Add/Fill→Publish only builds
        // derived presentation. Fill27f0e10 reads balance and maintenance
        // getters; no gameplay setters/signals/RNG. Its enabled/suspended
        // branches alter presentation only and need no native cache layout.
        self.reached
            .push("stock gold balance report presentation boundary");
        let signals = self
            .gold_signals
            .as_ref()
            .ok_or(Error::MissingRegistrations)?;
        if state.binding() != &signals.binding
            || !signals.instance.same_instance(state.action_stamp())
        {
            return Err(Error::Binding);
        }
        let whole_gold = current.treasury.gold_balance_q8 >> 8;
        if let Some(&token) = signals.listeners.first() {
            return Err(Error::UnsupportedListener {
                token,
                owner,
                whole_gold,
            });
        }
        self.reached
            .push("current GoldChanged registration list completed");
        Ok(())
    }
}
impl<P> treasury_deals::Effects for Runtime<'_, P> {
    fn gold_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), game_update::Error> {
        self.deliver_gold_changed(
            state,
            owner,
            Scope::StockPlainHeadlessExternalPoliciesNoAddedReportConsumers,
        )
        .map_err(|e| {
            self.last_error = Some(format!("{e:?}"));
            game_update::Error::Unsupported("current GoldChanged gameplay signal")
        })
    }
}
