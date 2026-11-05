//! Source-qualified capture subscribers over the same canonical owners.
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

/// Actual CityTransferredMomentDispatcher29e5010 guards for the first combat
/// capture. No moment is constructed when the original owner differs from the
/// new owner and the old major has multiple cities and loses no original capital.
pub fn history_pre_transfer(
    state: &LoadedState,
    city: ComponentId,
    new_owner: i32,
    transfer_type: u32,
) -> Result<(), String> {
    let old_owner = i32::from(city.owner);
    let value = state
        .city(old_owner, city.id)
        .map_err(|e| format!("{e:?}"))?;
    if transfer_type != 0xd3fb_88ac && value.header.original_owner == new_owner {
        return Err("city-transfer recapture historic moment".into());
    }
    let old = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == old_owner)
        .ok_or("history old city owner")?;
    if (value.header.original_capital
        && value.header.original_owner == old_owner
        && old_owner != new_owner)
        || old.cities.records.entries.len() <= 1
    {
        return Err("city-transfer original-capital/final-city historic moment".into());
    }
    Ok(())
}

/// Stats.OnCityTransferred2b7f910 mutates only cultural-identity transfers.
/// The ordinary combat transfer exits before player/City queries or RNG.
pub fn stats_pre_transfer(transfer_type: u32) -> Result<(), String> {
    if transfer_type == 0xd3fb_88ac {
        return Err("cultural-identity city-transfer Stats update".into());
    }
    Ok(())
}

/// FreeCities.Manager.OnCityTransferred294e290 compares the actual parent
/// player with both old and new owners. Major-to-major transfer reaches neither
/// list insertion nor marking a departing FreeCity instance dead.
pub fn free_cities_pre_transfer(
    city: ComponentId,
    new_owner: i32,
    manager_owner: i32,
) -> Result<(), String> {
    if i32::from(city.owner) == manager_owner || new_owner == manager_owner {
        return Err("FreeCities transfer instance lifetime".into());
    }
    Ok(())
}

/// AI.Military.OnCityConquered21490a0 preserves this gameplay/history write
/// even for externally controlled players. The signal's first argument is the
/// previous owner; callback receiver identity determines the matching list.
pub fn military_city_conquered(
    state: &mut LoadedState,
    receiver: i32,
    previous_owner: i32,
) -> Result<(), String> {
    if receiver != previous_owner {
        return Ok(());
    }
    let turn = state.retained().core.current_turn;
    let military = state
        .player_ai_military_mut(receiver)
        .map_err(|e| format!("{e:?}"))?
        .as_mut()
        .ok_or("current capture Military receiver")?;
    military
        .turns_cities_taken
        .try_reserve(1)
        .map_err(|_| "capture Military turn allocation")?;
    military.turns_cities_taken.push(turn);
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreTransfer {
    History,
    Stats,
    FreeCities { manager_owner: i32 },
}
/// Invoke one source-classified live pre-transfer listener. This lets the
/// caller retain its mutation-safe listener ring and actual receiver identity.
pub fn dispatch_pre_transfer(
    state: &LoadedState,
    listener: PreTransfer,
    city: ComponentId,
    new_owner: i32,
    transfer_type: u32,
) -> Result<(), String> {
    match listener {
        PreTransfer::History => history_pre_transfer(state, city, new_owner, transfer_type),
        PreTransfer::Stats => stats_pre_transfer(transfer_type),
        PreTransfer::FreeCities { manager_owner } => {
            free_cities_pre_transfer(city, new_owner, manager_owner)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Conquered {
    Summary,
    GeneralPlayerSummary,
    Lua,
    DiplomaticLog,
    Gossip,
    EmergencyStart,
    EmergencyGoal,
    Military { receiver: i32 },
    Achievements,
}

/// The public Emergency adapter owns its real temporary-object/requirement
/// implementation; the caller loans that adapter alongside the other owners.
pub trait EmergencyCallbacks {
    fn start_military(
        &mut self,
        state: &mut LoadedState,
        previous_owner: i32,
        city: ComponentId,
    ) -> Result<(), String>;
    fn goal(
        &mut self,
        state: &mut LoadedState,
        previous_owner: i32,
        city: ComponentId,
    ) -> Result<(), String>;
}

/// These are borrowed ongoing owners. They must be the same Script, Gossip and
/// Emergency instances used by founding, ordinary actions and turn upkeep.
pub struct Context<'s, T, E> {
    pub state: &'s mut LoadedState,
    pub script: &'s mut crate::loaded_actions::transition::ScriptState,
    pub synchronization: &'s mut crate::loaded_actions::stock_script::Synchronization,
    pub gossip: &'s mut c6_core::city_power_activation::gossip_delivery::Manager<T>,
    pub emergency: &'s mut E,
}
impl<T, E: EmergencyCallbacks> Context<'_, T, E> {
    pub fn dispatch_one(
        &mut self,
        listener: Conquered,
        city: ComponentId,
        previous_owner: i32,
    ) -> Result<(), String> {
        self.state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| format!("{e:?}"))?;
        match listener {
            // Both source helpers only adjust Summary repository datasets.
            // The file logger only reads turn/mayhem/visibility and writes a
            // formatted log line. No gameplay state, signal or RNG is reached.
            Conquered::Summary | Conquered::GeneralPlayerSummary | Conquered::DiplomaticLog => {
                Ok(())
            }
            Conquered::Lua => crate::loaded_actions::stock_script::on_city_conquered(
                self.script,
                self.synchronization,
                self.state,
                previous_owner,
                city,
            )
            .map_err(|e| format!("{e:?}")),
            Conquered::Gossip => conquer_city_gossip_empty(self.gossip),
            Conquered::EmergencyStart => {
                self.emergency
                    .start_military(self.state, previous_owner, city)
            }
            Conquered::EmergencyGoal => self.emergency.goal(self.state, previous_owner, city),
            Conquered::Military { receiver } => {
                military_city_conquered(self.state, receiver, previous_owner)
            }
            // These two capture achievement methods only read canonical game
            // queries, mutate achievement-progress counters, and call the
            // platform achievement output helper. Inspected native helpers
            // contain no gameplay writes, signals, policy calls or RNG draws.
            // Their platform presentation is omitted at the arena boundary.
            Conquered::Achievements => Ok(()),
        }
    }
}

/// PreCityConquered has the source-qualified Achievements.OnPreCityConquered
/// output listener. Its complete reached source reads definitions/promotions/
/// plots and calls only platform achievement output; no gameplay/RNG effects.
pub fn pre_city_conquered_achievements() {}

/// ConquerCity.Handler constructs occurrence(type0e3a97a2,new_owner,old_owner,
/// city_id). AddGossipItem29b25d0 returns BEFORE retaining the occurrence or
/// setting turn when the actual current TurnVisibility history is empty.
pub fn conquer_city_gossip_empty<T>(
    manager: &c6_core::city_power_activation::gossip_delivery::Manager<T>,
) -> Result<(), String> {
    if manager.history.is_empty() {
        Ok(())
    } else {
        Err("capture gossip occurrence/report/delivery continuation".into())
    }
}
