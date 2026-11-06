//! Ordered Player.Eras.SetEra callbacks on the current stock runtime.
//! Positive random boosts, owned-route upgrades and Gossip visibility delivery
//! remain explicit; the supported cold branches read their live owners.
use super::{processing, turn_reports, Runtime};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerSubscriber {
    EventManager,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameSubscriber {
    Gossip,
}
pub struct Inputs {
    binding: WireBinding,
    players: Vec<(i32, Vec<PlayerSubscriber>)>,
    game: Vec<GameSubscriber>,
}
impl Inputs {
    pub fn from_initial(
        state: &LoadedState,
        players: Vec<(i32, Vec<PlayerSubscriber>)>,
        game: Vec<GameSubscriber>,
    ) -> Result<Self, processing::Error> {
        for (index, (owner, _)) in players.iter().enumerate() {
            state
                .world()
                .player(*owner)
                .map_err(|_| processing::Error::Unsupported("era callback player"))?;
            if players[..index].iter().any(|p| p.0 == *owner) {
                return Err(processing::Error::Unsupported(
                    "duplicate era callback owner",
                ));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            players,
            game,
        })
    }
}
pub(super) struct State {
    inputs: Inputs,
    history_empty: Option<bool>,
}
impl<P> Runtime<'_, P> {
    pub fn bind_era_changes(
        &mut self,
        state: &LoadedState,
        inputs: Inputs,
    ) -> Result<(), processing::Error> {
        self.check(state)?;
        if inputs.binding != *state.binding() {
            return Err(processing::Error::Unsupported("era callback input binding"));
        }
        self.era_changes = Some(State {
            inputs,
            history_empty: None,
        });
        Ok(())
    }
    /// Query the SAME Session history for this activation scope. No initial
    /// history answer is retained; the prior scope is restored on every result.
    pub fn with_era_gossip_history<T>(
        &mut self,
        history: &[c6_core::city_power_activation::gossip_delivery::TurnVisibility],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self
            .era_changes
            .as_mut()
            .map(|s| s.history_empty.replace(history.is_empty()));
        let result = f(self);
        if let (Some(s), Some(previous)) = (&mut self.era_changes, previous) {
            s.history_empty = previous;
        }
        result
    }
    /// Current activation's borrowed Session history. This is deliberately
    /// unavailable outside that scope; an initial empty answer is not retained.
    pub(super) fn current_gossip_history_empty(&self) -> Option<bool> {
        self.era_changes
            .as_ref()
            .and_then(|state| state.history_empty)
    }
    pub(super) fn player_era_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        old: i32,
        new: i32,
    ) -> Result<(), &'static str> {
        self.check(state).map_err(|_| "era runtime binding")?;
        let current = self
            .era_changes
            .take()
            .ok_or("current era callback registrations")?;
        let result = (|| {
            if current.inputs.binding != *state.binding() {
                return Err("era current owner binding");
            }
            let listeners = &current
                .inputs
                .players
                .iter()
                .find(|p| p.0 == owner)
                .ok_or("era current player registrations")?
                .1;
            for subscriber in listeners {
                match subscriber {
                    PlayerSubscriber::EventManager => {
                        // EventManager2b01650 calls Techs, Culture, Trade in order.
                        let p = state
                            .retained()
                            .players
                            .iter()
                            .find(|p| p.header.owner == owner)
                            .ok_or("era retained player")?;
                        if p.techs.num_random_tech_boosts_on_new_era > 0 {
                            return Err("era random technology boosts");
                        }
                        if p.culture.num_random_civic_boosts_on_new_era > 0 {
                            return Err("era random civic boosts");
                        }
                        // UpgradePlayerRoutes2bce120 selects owned route plots
                        // before any best-route query, setter, signal or RNG.
                        if state.world().map().plots.iter().any(|p| {
                            p.route_type >= 0
                                && p.city_attributes
                                    .as_ref()
                                    .is_some_and(|a| i32::from(a.player) == owner)
                        }) {
                            return Err("era current owned-route upgrade");
                        }
                        self.reached
                            .push("era player Techs Culture Trade cold callbacks complete");
                    }
                }
            }
            for subscriber in &current.inputs.game {
                match subscriber {
                    GameSubscriber::Gossip => match current.history_empty {
                        // AddGossipItem29b25d0 returns before archive/report when
                        // the current TurnVisibility vector is empty. Occurrence
                        // construction only writes the temporary type/owner.
                        Some(true) => {}
                        Some(false) => return Err("era Gossip visibility delivery"),
                        None => return Err("era current Session Gossip history scope"),
                    },
                }
            }
            // City publications are output only, but their current City query
            // path is not admitted yet on this reached cityless transition.
            if !state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or("era report player")?
                .cities
                .records
                .entries
                .is_empty()
            {
                return Err("era current City report path");
            }
            // SetEra's final8152ea report feeds the stock EventStream DTO, with
            // no synchronous gameplay setters/signals or RNG on this route.
            self.deliver_round_report(
                state,
                0x008152ea,
                owner,
                new,
                old,
                turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )
            .map_err(|_| "era changed report")?;
            Ok(())
        })();
        self.era_changes = Some(current);
        result
    }
}
