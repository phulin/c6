//! CityDestroyed subscribers over the ongoing modifier/requirement/collection
//! owners. Registration order is the live signal order, never grouped by kind.
use crate::{
    found_city::city_created_callbacks::Current,
    melee_attack::backend::{
        self,
        city_creation::{self, collection},
        EffectRuntime,
    },
};
use c6_core::{
    loaded_state::LoadedState, melee_attack::proposals::collections, unit::ComponentId,
    unit_moved_signal::ListenerRing,
};

#[derive(Clone, Copy, Debug)]
pub enum Listener {
    GameEffects,
    Trade,
    PlayerRules { owner: i32 },
    Targets { owner: i32 },
    PlayerCities { owner: i32, collection: u64 },
    AllCities { collection: u64 },
    Requirement(super::requirements::Binding),
    Unsupported(u64),
}

/// Authenticated current Unit.MakeTradeRoute definition values. Unit identity
/// remains in the canonical roster and is queried anew during the callback.
pub struct TradeDefinitions {
    binding: c6_core::core_player::initialization::WireBinding,
    units: crate::registry::LinuxDefinitionRegistry,
    make_route: Vec<bool>,
}
impl TradeDefinitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &crate::Rules,
        bound: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, String> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|e| format!("{e:?}"))?;
        if state.binding() != bound.destination_binding() {
            return Err("CityDestroyed trade definition binding".into());
        }
        let units = crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(|e| format!("{e:?}"))?;
        let table = rules.gameplay().table("Units").ok_or("Units")?;
        let make_route = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table.row_by_id(entry.database_rowid).ok_or("Units row")?;
                let value = row.get("MakeTradeRoute").map_err(|e| format!("{e:?}"))?;
                crate::game_parameters::sql_integer(value)
                    .map(|v| v != 0)
                    .ok_or_else(|| "Unit.MakeTradeRoute".to_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            make_route,
        })
    }
    /// Trade.Manager.CityDestroyed2c0c2d0. An actual empty graph and absence of
    /// any trader at this location leave gameplay/RNG unchanged. Positive route
    /// cleanup or trader relocation remains explicit at the reached operation.
    pub fn destroyed(
        &self,
        state: &LoadedState,
        city: ComponentId,
        uninitializing: bool,
    ) -> Result<(), String> {
        if uninitializing {
            return Ok(());
        }
        if &self.binding != state.binding() {
            return Err("CityDestroyed trade rules owner".into());
        }
        if state
            .trade_graph()
            .map_err(|e| format!("{e:?}"))?
            .graph()
            .origins()
            .iter()
            .any(|origin| !origin.destinations.is_empty())
        {
            return Err("CityDestroyed route removal/cleanup requires live graph mutation".into());
        }
        let current = state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| format!("{e:?}"))?;
        let location = c6_core::unit::PlotCoord {
            x: current.header.location_x,
            y: current.header.location_y,
        };
        let owner = state
            .world()
            .player(i32::from(city.owner))
            .map_err(|e| format!("{e:?}"))?;
        for entry in owner.unit_records().map_err(|e| format!("{e:?}"))? {
            let unit = &entry.value;
            if unit.k_location != location {
                continue;
            }
            let index = self
                .units
                .find_index(unit.e_type_index)
                .ok_or("current Unit definition")?;
            if self.make_route[index] {
                return Err("CityDestroyed trader relocation/operation continuation".into());
            }
        }
        Ok(())
    }
}

pub struct Context<'a, E> {
    pub state: &'a mut LoadedState,
    pub effects: &'a mut EffectRuntime,
    pub requirements: &'a mut backend::requirements::Runtime,
    pub collections: &'a mut collections::Runtime,
    pub listeners: &'a mut ListenerRing<Listener>,
    pub queries: &'a Current<'a>,
    pub features: &'a [u32],
    pub logging_priority: i32,
    /// Actual Game+2ae, also used by Context.IsShuttingDown.
    pub uninitializing: bool,
    pub trade: &'a TradeDefinitions,
    pub factory: &'a mut dyn collection::RequirementFactory,
    pub leaves: &'a mut E,
}
impl<E: crate::city_production_turn::modifier_effects::Effects> Context<'_, E> {
    pub fn dispatch(&mut self, city: ComponentId) -> Result<(), String> {
        let mut cursor = self
            .listeners
            .cursor_start()
            .map_err(|e| format!("{e:?}"))?;
        loop {
            let listener = self
                .listeners
                .cursor_route(cursor)
                .map_err(|e| format!("{e:?}"))?;
            if let Some(listener) = listener {
                if let Err(error) = self.one(listener, city) {
                    self.listeners
                        .cursor_release(cursor)
                        .map_err(|e| format!("{e:?}"))?;
                    return Err(error);
                }
            }
            match self
                .listeners
                .cursor_next(cursor)
                .map_err(|e| format!("{e:?}"))?
            {
                Some(next) => cursor = next,
                None => return Ok(()),
            }
        }
    }
    pub fn one(&mut self, listener: Listener, city: ComponentId) -> Result<(), String> {
        self.state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| format!("{e:?}"))?;
        match listener {
            Listener::GameEffects => city_creation::destruction::detach_city(
                self.state,
                self.effects,
                self.requirements,
                city,
                self.logging_priority,
                self.features,
                self.factory,
                self.leaves,
            )
            .map_err(|e| format!("{e:?}")),
            Listener::Trade => self.trade.destroyed(self.state, city, self.uninitializing),
            // Player.OnCityDestroyed2b30110 -> Rules.Players2bf1450 only
            // reads WorldBuilder.IsActive; no mutation, callback or RNG.
            Listener::PlayerRules { owner } => self
                .state
                .world()
                .player(owner)
                .map(|_| ())
                .map_err(|e| format!("{e:?}")),
            Listener::Targets { owner } => {
                super::targets_city_destroyed(self.state, owner, city).map_err(|e| format!("{e:?}"))
            }
            Listener::Requirement(binding) => super::requirements::city_destroyed(
                self.state,
                self.requirements,
                self.queries,
                binding,
                city,
            )
            .map_err(|e| format!("{e:?}")),
            Listener::PlayerCities { owner, collection } => {
                if i32::from(city.owner) != owner {
                    return Ok(());
                }
                self.remove_collection(collection, city)
            }
            Listener::AllCities { collection } => self.remove_collection(collection, city),
            Listener::Unsupported(receiver) => Err(format!(
                "unsupported live CityDestroyed subscriber {receiver}"
            )),
        }
    }
    fn remove_collection(&mut self, collection: u64, city: ComponentId) -> Result<(), String> {
        let subject = self
            .effects
            .make_city_object(self.state, city)
            .map_err(|e| format!("{e:?}"))?;
        collections::remove_typed_item(self, collection, subject).map_err(|e| format!("{e:?}"))
    }
}
impl<E: crate::city_production_turn::modifier_effects::Effects> collections::Context<u64>
    for Context<'_, E>
{
    type Error = String;
    fn collections(&self) -> Result<&collections::Runtime, String> {
        Ok(self.collections)
    }
    fn collections_mut(&mut self) -> Result<&mut collections::Runtime, String> {
        Ok(self.collections)
    }
    fn item_changed(
        &mut self,
        receiver: u64,
        subject: &mut u64,
        added: bool,
    ) -> Result<(), String> {
        collection::item_changed(
            self.state,
            self.effects,
            self.requirements,
            collection::Invocation {
                receiver,
                subject: *subject,
                added,
                maximum_loyalty: None,
                features: self.features,
            },
            self.factory,
            self.leaves,
        )
        .map_err(|e| format!("{e:?}"))
    }
}
