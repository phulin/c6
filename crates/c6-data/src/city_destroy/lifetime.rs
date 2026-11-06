//! Actual City/CityBuild signal and factory lifetime. These signal rings are
//! borrowed ongoing owners, not copies reconstructed for each destruction.
use crate::melee_attack::backend::{requirements, EffectRuntime};
use c6_core::{
    loaded_state::LoadedState,
    unit::ComponentId,
    unit_moved_signal::{ListenerRing, Subscription},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signal {
    PlayerControl(i32),
    PlayerDistrict(i32),
    GameBuilding,
    GameCityPair,
    GamePlayersCityPair,
    GameDiplomacy,
    GameGreatPerson,
    GameResourceCities,
}
#[derive(Clone, Copy, Debug)]
pub struct Handle {
    pub signal: Signal,
    pub token: Subscription,
}
/// Source callable identities remain in the rings even when this bounded
/// action only unsubscribes them. Future dispatch requires their concrete
/// consumers; an unknown callable is not a successful empty callback.
pub struct Signals {
    pub rings: Vec<(Signal, ListenerRing<u64>)>,
}
impl Signals {
    pub fn from_initial(rings: Vec<(Signal, ListenerRing<u64>)>) -> Result<Self, String> {
        if rings
            .iter()
            .enumerate()
            .any(|(i, r)| rings[..i].iter().any(|p| p.0 == r.0))
        {
            return Err("duplicate lifetime signal owner".into());
        }
        Ok(Self { rings })
    }
    /// Resolve a captured handle to its actual current ring token once during
    /// initial materialization; no extra listener is appended.
    pub fn handle(&self, signal: Signal, receiver: u64) -> Result<Handle, String> {
        let ring = &self
            .rings
            .iter()
            .find(|r| r.0 == signal)
            .ok_or("lifetime signal owner")?
            .1;
        let token = ring
            .subscription_for(|r| r == receiver)
            .map_err(|e| format!("{e:?}"))?;
        Ok(Handle { signal, token })
    }
    /// Register a source-created callable in this ongoing signal. The caller's
    /// logical callable owner retains its typed City/CityBuild receiver.
    pub fn subscribe(&mut self, signal: Signal, receiver: u64) -> Result<Handle, String> {
        let ring = &mut self
            .rings
            .iter_mut()
            .find(|r| r.0 == signal)
            .ok_or("current lifetime signal owner")?
            .1;
        let token = ring.subscribe(receiver).map_err(|e| format!("{e:?}"))?;
        Ok(Handle { signal, token })
    }
    fn unsubscribe(&mut self, handle: Handle) -> Result<(), String> {
        self.rings
            .iter_mut()
            .find(|r| r.0 == handle.signal)
            .ok_or("current lifetime signal owner")?
            .1
            .unsubscribe(handle.token)
            .map_err(|e| format!("{e:?}"))
    }
}
/// Complete source handle fields, including actual nulls. Neither a missing
/// City nor unknown handle is inferred empty. Source constructors may register
/// fresh rows into this same owner as new Cities are created.
pub struct City {
    pub city: ComponentId,
    pub signals: [Option<Handle>; 3],
    pub event_manager: Option<Handle>,
    pub city_build: Option<Build>,
}
pub struct Build {
    pub id: u32,
    /// Current Handle+0 cache pointer presence, distinct from saved owner/ID.
    pub working_contract_cached: bool,
    pub managed_contracts_cached: Vec<bool>,
    /// Player+270, Player+f0, Game+1710, Game+1338, Game+1350.
    pub signals: [Option<Handle>; 5],
}
pub struct Runtime {
    stamp: c6_core::loaded_state::ActionStamp,
    pub cities: Vec<City>,
}
impl Runtime {
    /// Called by canonical City/CityBuild initialization AFTER their actual
    /// subscriptions have been registered in the same Signals owner.
    pub fn register_city(&mut self, state: &LoadedState, city: City) -> Result<(), String> {
        if !self.stamp.same_instance(state.action_stamp()) {
            return Err("City lifetime state owner".into());
        }
        state
            .city(i32::from(city.city.owner), city.city.id)
            .map_err(|e| format!("{e:?}"))?;
        if self.cities.iter().any(|old| old.city == city.city) {
            return Err("duplicate City lifetime".into());
        }
        self.cities.push(city);
        Ok(())
    }
    pub fn from_initial(state: &LoadedState, cities: Vec<City>) -> Result<Self, String> {
        if !state.is_pristine() {
            return Err("initial City lifetime after gameplay".into());
        }
        for (index, row) in cities.iter().enumerate() {
            state
                .city(i32::from(row.city.owner), row.city.id)
                .map_err(|e| format!("{e:?}"))?;
            if cities[..index].iter().any(|old| old.city == row.city) {
                return Err("duplicate City lifetime".into());
            }
        }
        Ok(Self {
            stamp: state.action_stamp(),
            cities,
        })
    }
}
/// CityD1 ordering: unsubscription; Resources/Religion verified RETs; event
/// manager unsubscription; cached modifier release; validation; CityBuild D1.
/// The canonical City slot is released only by the enclosing transition.
#[allow(clippy::too_many_arguments)]
pub fn destroy(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    signals: &mut Signals,
    effects: &mut EffectRuntime,
    requirements: &requirements::Runtime,
    city: ComponentId,
    logging_priority: i32,
    shutting_down: bool,
) -> Result<(), String> {
    if !runtime.stamp.same_instance(state.action_stamp()) {
        return Err("City lifetime state owner".into());
    }
    let index = runtime
        .cities
        .iter()
        .position(|r| r.city == city)
        .ok_or("current City lifetime metadata")?;
    for at in 0..3 {
        if let Some(handle) = runtime.cities[index].signals[at] {
            signals.unsubscribe(handle)?;
            runtime.cities[index].signals[at] = None;
        }
    }
    if let Some(handle) = runtime.cities[index].event_manager {
        signals.unsubscribe(handle)?;
        runtime.cities[index].event_manager = None;
    }
    effects
        .release_destroyed_city_cache(state, requirements, city)
        .map_err(|e| format!("{e:?}"))?;
    // Validation.Check_NoCityReferences only enters its diagnostic object scan
    // at priority>=3. Enabled diagnostic allocation is not guessed away.
    if logging_priority >= 3 {
        return Err("City destruction diagnostic reference validation".into());
    }
    if let Some(build) = runtime.cities[index].city_build.as_mut() {
        for at in 0..5 {
            if let Some(handle) = build.signals[at] {
                signals.unsubscribe(handle)?;
                build.signals[at] = None;
            }
        }
        let owner = i32::from(city.owner);
        let current = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| p.ai_city_manager.as_ref())
            .ok_or("current CityBuild manager")?
            .city_ais
            .entries
            .iter()
            .find(|e| e.city_build.city_ai_id == build.id)
            .ok_or("current CityBuild instance")?;
        if current
            .city_build
            .event_trees
            .iter()
            .any(|id| *id != u32::MAX)
        {
            return Err("CityBuild existing event-tree destruction".into());
        }
        let contract = &current.city_build.managed_working_contract;
        // Handle.get returns its current cache first, otherwise FindContract
        // scans the current canonical manager by full native contract ID.
        let working_live = if build.working_contract_cached {
            true
        } else if contract.player_id == -1 {
            false
        } else {
            state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == contract.player_id)
                .and_then(|p| p.ai_control.as_ref())
                .ok_or("CityBuild working-contract manager")?
                .contracts
                .iter()
                .flatten()
                .any(|r| r.base.id == contract.id)
        };
        if working_live && !shutting_down {
            return Err("CityBuild live working-contract FailContract".into());
        }
        // Release uses the actual cached pointer. A nonnull cached contract
        // can RemoveHandle and destroy its last owner even when shutting down.
        if build.working_contract_cached {
            return Err("CityBuild working-contract cached owner release".into());
        }
        if build.managed_contracts_cached.len() != current.city_build.managed_contracts.len() {
            return Err("CityBuild managed-contract cache projection".into());
        }
        if build
            .managed_contracts_cached
            .iter()
            .any(|present| *present)
        {
            return Err("CityBuild managed-contract cached owner release".into());
        }
        state
            .release_city_build_after_destruction(owner, build.id)
            .map_err(|e| format!("{e:?}"))?;
    }
    runtime.cities.remove(index);
    Ok(())
}
