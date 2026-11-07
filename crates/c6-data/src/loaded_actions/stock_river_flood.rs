//! Same-BEFORE stock river-flood reporting and zero player-damage callbacks.
//! Additional consumers and positive player damage require explicit handlers.
use super::Runtime;
use crate::game_update_state::round_random_events::Error;
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug)]
pub enum WholeDamageConsumer {
    Stats(i32),
    Achievements,
}
#[derive(Clone, Copy, Debug)]
pub enum PlayerDamageConsumer {
    Gossip,
    Emergency,
    Achievements,
}
#[derive(Clone, Copy)]
pub struct DamageInputs<'a> {
    pub definitions: &'a crate::unit_placement_state::Definitions<'a, 'a>,
    pub queries: &'a crate::unit_placement_state::Queries<'a>,
    pub visibility: &'a std::cell::RefCell<crate::unit_sight_state::Runtime>,
}
pub struct Inputs<'a> {
    binding: WireBinding,
    whole: Vec<WholeDamageConsumer>,
    player: Vec<PlayerDamageConsumer>,
    ever_alive: Vec<i32>,
    aid_threshold: i32,
    damage: Option<DamageInputs<'a>>,
}
impl<'a> Inputs<'a> {
    /// The caller supplies the actual ordered GameSignals+c30/c48 listeners,
    /// ever-alive IDs and rules threshold. It must qualify the stock Terrain,
    /// General and YieldMonitor cache consumers and queued-only host reports.
    pub fn from_initial(
        state: &LoadedState,
        whole: Vec<WholeDamageConsumer>,
        player: Vec<PlayerDamageConsumer>,
        ever_alive: Vec<i32>,
        aid_threshold: i32,
    ) -> Result<Self, Error> {
        if aid_threshold <= 0 {
            return Err(Error::Requires("positive flood aid threshold"));
        }
        Ok(Self {
            binding: state.binding().clone(),
            whole,
            player,
            ever_alive,
            aid_threshold,
            damage: None,
        })
    }
    pub fn with_damage(mut self, damage: DamageInputs<'a>) -> Self {
        self.damage = Some(damage);
        self
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_river_flood_reports(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Requires("river flood backend binding"))?;
        if self.river_flood.is_some() || inputs.binding != *state.binding() {
            return Err(Error::Requires("river flood current input binding"));
        }
        self.river_flood = Some(inputs);
        Ok(())
    }
    pub(crate) fn river_flood_report(
        &mut self,
        state: &mut LoadedState,
        event_hash: u32,
        subtype: u32,
        plot: i32,
        _severity: i32,
        mitigated: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Requires("river flood backend binding"))?;
        let inputs = self
            .river_flood
            .as_ref()
            .ok_or(Error::Requires("current river flood report consumers"))?;
        if inputs.binding != *state.binding()
            || mitigated
            || plot < 0
            || plot as usize >= state.world().map().plots.len()
        {
            return Err(Error::Requires("river flood report context"));
        }
        match event_hash {
            0xfc5eac6e if subtype == 0x1c8fa8d7 => {
                // Qualified Terrain consumer fills presentation from the plain
                // canonical plot. No gameplay/requirement consumer is omitted.
                self.reached.push("stock river flood TerrainStateChanged");
            }
            0xe869c8ff => {
                // General summary is output. YieldMonitor's hot observer may
                // query routes; execute its existing canonical domain guard.
                super::reporting::yield_monitor::warm_route_domain(state)
                    .map_err(|_| Error::Requires("river flood current YieldMonitor routes"))?;
                self.reached.push("stock river flood RandomEventOccurred");
            }
            _ => return Err(Error::Requires("river flood report type")),
        }
        Ok(())
    }
    /// Called only by the preflighted unowned/no-unit flood branch: the entire
    /// PlayerRandomDamageArray, including affected-plot vectors, is zero/empty.
    pub(crate) fn river_flood_damage_signals(
        &mut self,
        state: &mut LoadedState,
        _event_hash: u32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Requires("river flood backend binding"))?;
        let inputs = self
            .river_flood
            .as_ref()
            .ok_or(Error::Requires("current river flood damage consumers"))?;
        if inputs.binding != *state.binding() || inputs.aid_threshold <= 0 {
            return Err(Error::Requires("river flood zero-damage scope"));
        }
        for consumer in &inputs.whole {
            match consumer {
                WholeDamageConsumer::Stats(owner) => {
                    if !state
                        .retained()
                        .players
                        .iter()
                        .any(|p| p.header.owner == *owner)
                    {
                        return Err(Error::Requires("river flood Stats owner"));
                    }
                    // FLOODPLAIN adds zero to four integer tracked counters;
                    // the source zero-delta path reaches no Changed callback.
                    self.reached.push("stock river flood zero-damage Stats");
                }
                WholeDamageConsumer::Achievements => self
                    .reached
                    .push("stock river flood empty damage achievements"),
            }
        }
        for owner in &inputs.ever_alive {
            if !state
                .retained()
                .players
                .iter()
                .any(|p| p.header.owner == *owner)
            {
                return Err(Error::Requires("river flood ever-alive owner"));
            }
            for consumer in &inputs.player {
                match consumer {
                    // Gossip requires threshold < damage; AidRequest requires
                    // threshold <= damage. Current positive threshold rejects
                    // both before any gameplay callback or RNG. The reached
                    // FLOODPLAIN name has neither comet nor Vesuvius flags.
                    PlayerDamageConsumer::Gossip => self
                        .reached
                        .push("stock river flood zero-damage Gossip gate"),
                    PlayerDamageConsumer::Emergency => {
                        self.reached.push("stock river flood zero-damage aid gate")
                    }
                    PlayerDamageConsumer::Achievements => self
                        .reached
                        .push("stock river flood zero-damage player achievements"),
                }
            }
        }
        Ok(())
    }
    pub(crate) fn river_flood_maximum_damage(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<i32, String> {
        let inputs = self
            .river_flood
            .as_ref()
            .and_then(|i| i.damage)
            .ok_or("river flood damage providers")?;
        let unit = state
            .world()
            .player(actor.0)
            .map_err(|e| format!("{e:?}"))?
            .unit(actor.1)
            .map_err(|e| format!("{e:?}"))?
            .ok_or("river flood current unit")?
            .record;
        inputs
            .definitions
            .movement
            .maximum_damage(unit)
            .map_err(|e| format!("{e:?}"))
    }
    pub(crate) fn river_flood_change_damage(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        delta: i32,
    ) -> Result<(), String> {
        let inputs = self
            .river_flood
            .as_ref()
            .and_then(|i| i.damage)
            .ok_or("river flood damage providers")?;
        let mut sight = inputs
            .visibility
            .try_borrow_mut()
            .map_err(|e| format!("river flood sight: {e:?}"))?;
        crate::unit_placement_state::change_damage(
            state,
            inputs.definitions,
            inputs.queries,
            &mut sight,
            self,
            actor,
            delta,
        )
        .map(|_| ())
        .map_err(|e| format!("river flood ChangeDamage: {e:?}"))
    }
    pub(crate) fn river_flood_wake_unit(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
    ) -> Result<(), String> {
        crate::loaded_upkeep::clear_unit_operations(state, actor.0, actor.1, self)
            .map_err(|e| format!("river flood Clear: {e:?}"))?;
        crate::loaded_upkeep::set_unit_activity(state, actor.0, actor.1, self, 0x6b25012e)
            .map_err(|e| format!("river flood Awake: {e:?}"))
    }
}
