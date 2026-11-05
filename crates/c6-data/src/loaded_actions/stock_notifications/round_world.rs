//! XP2 world upkeep over the existing climate, trade and action owners.
use super::Runtime;
use crate::game_update_state::{
    activation::cityless_managers, round_climate, round_congress, round_diplomatic_relations,
    round_emergency, round_eras, round_fallout, round_great_people, round_map, round_quests,
    round_random_events, round_religion, round_trade,
};
use c6_core::{
    game_capabilities::Membership,
    game_update::Error,
    loaded_state::{
        game_update::round::{self, WorldManager},
        LoadedState,
    },
};

/// Remaining map and world-manager callbacks stay at their native positions.
/// Implementations must retain actual gameplay/RNG effects, or return an error.
pub trait Effects<P> {
    fn manager_turn(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        manager: WorldManager,
    ) -> Result<(), Error>;
}

pub struct Current<'a, 'b, P, E> {
    pub backend: &'a mut Runtime<'b, P>,
    pub map: &'a mut round_map::Runtime,
    pub climate: &'a mut round_climate::Runtime,
    pub cityless: &'a mut cityless_managers::Runtime,
    pub carbon: round_climate::CarbonRules<'a>,
    pub alive_major_ids: &'a [i32],
    pub trade: &'a mut round_trade::Runtime,
    pub random_definitions: &'a round_random_events::Definitions,
    pub random: &'a mut round_random_events::Runtime,
    pub fallout: &'a mut round_fallout::Runtime,
    pub great_people_definitions: &'a round_great_people::Definitions,
    pub great_people: &'a mut round_great_people::Runtime,
    pub quests: &'a mut round_quests::Runtime,
    pub diplomatic_definitions: &'a round_diplomatic_relations::Definitions,
    pub era_definitions: &'a round_eras::Definitions,
    pub congress_definitions: &'a round_congress::Definitions,
    pub favor_definitions: &'a round_congress::favor::Definitions,
    pub alive_ids: &'a [i32],
    pub capabilities: &'a Membership,
    pub effects: &'a mut E,
}

impl<P> round_trade::Effects for Runtime<'_, P> {
    fn temporary_routes_cleared(&mut self, _: &mut LoadedState) -> Result<(), round_trade::Error> {
        Err(round_trade::Error::Requires(
            "temporary-route-cleared signal consumers",
        ))
    }
}
struct EraReports;
struct Favor<'a, 'b, P> {
    backend: &'a mut Runtime<'b, P>,
    definitions: &'a round_congress::favor::Definitions,
    climate: &'a round_climate::Runtime,
    alive: &'a [i32],
    majors: &'a [i32],
}
impl<P> round_congress::Effects for Favor<'_, '_, P> {
    fn favor_per_turn(&mut self, state: &LoadedState, owner: i32) -> Result<i32, Error> {
        round_congress::favor::calculate(
            state,
            self.definitions,
            owner,
            self.climate,
            self.alive,
            self.majors,
        )
    }
    fn favor_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), Error> {
        self.backend
            .deliver_round_report(
                state,
                0x68e7d3da,
                owner,
                amount,
                reason,
                super::turn_reports::Scope::StockPlainHeadlessExternalPoliciesNoAddedConsumers,
            )
            .map_err(|_| Error::Unsupported("Congress favor scalar report"))
    }
}
impl round_eras::Effects for EraReports {
    fn countdown_started(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
    ) -> Result<(), round_eras::Error> {
        Err(round_eras::Error::Requires(
            "era countdown report consumers",
        ))
    }
}

impl<P, E: Effects<P>> round::WorldEffects for Current<'_, '_, P, E> {
    fn map_component_count(&mut self, state: &LoadedState) -> Result<u32, Error> {
        self.map
            .component_count(state)
            .map_err(|_| Error::Unsupported("current Map turn components"))
    }
    fn map_component_turn(&mut self, state: &mut LoadedState, index: u32) -> Result<(), Error> {
        self.map
            .component_turn(state, index, self.backend)
            .map_err(|detail| {
                self.backend.last_error = Some(format!("Map.DoTurn: {detail:?}"));
                Error::Unsupported("Map component turn continuation")
            })
    }
    fn manager_turn(
        &mut self,
        state: &mut LoadedState,
        manager: WorldManager,
    ) -> Result<(), Error> {
        let result = match manager {
            WorldManager::Climate => round_climate::do_turn(
                state,
                self.climate,
                self.cityless,
                self.alive_major_ids,
                state.retained().core.current_turn,
                &self.carbon,
            )
            .map_err(|e| format!("{e:?}")),
            WorldManager::Trade => {
                round_trade::run(state, self.trade, self.backend).map_err(|e| format!("{e:?}"))
            }
            WorldManager::GreatPeople => round_great_people::run(
                state,
                self.great_people_definitions,
                self.great_people,
                self.backend,
            )
            .map_err(|e| format!("{e:?}")),
            WorldManager::Religion => {
                let players = self
                    .backend
                    .player_services
                    .as_ref()
                    .ok_or(Error::Unsupported("round Religion current player services"))?;
                round_religion::run(state, players).map_err(|e| format!("{e:?}"))
            }
            WorldManager::Quests => {
                round_quests::run(state, self.quests).map_err(|e| format!("{e:?}"))
            }
            WorldManager::DiplomaticRelations => {
                let players = self
                    .backend
                    .player_services
                    .as_ref()
                    .ok_or(Error::Unsupported(
                        "round Diplomacy current player services",
                    ))?;
                round_diplomatic_relations::run(
                    state,
                    players,
                    self.diplomatic_definitions,
                    self.random.scheduling.game_speed_hash,
                )
                .map_err(|e| format!("{e:?}"))
            }
            WorldManager::Eras => {
                let players = self
                    .backend
                    .player_services
                    .as_ref()
                    .ok_or(Error::Unsupported("round Eras current player services"))?;
                round_eras::run(
                    state,
                    self.era_definitions,
                    players,
                    self.capabilities,
                    round_eras::Scheduling {
                        game_speed_hash: self.random.scheduling.game_speed_hash,
                        game_end_turn: self.random.scheduling.game_end_turn,
                    },
                    &mut EraReports,
                )
                .map_err(|e| format!("{e:?}"))
            }
            WorldManager::RandomEvents => round_random_events::run(
                state,
                self.random_definitions,
                self.random,
                self.fallout,
                &mut round_random_events::CurrentClimate {
                    runtime: self.climate,
                    cityless: self.cityless,
                },
                self.capabilities,
                self.backend,
            )
            .map_err(|e| format!("{e:?}")),
            WorldManager::Fallout => {
                round_fallout::run(state, self.fallout, self.backend).map_err(|e| format!("{e:?}"))
            }
            WorldManager::Emergency => {
                round_emergency::run(state, self.cityless, self.capabilities)
                    .map_err(|e| format!("{e:?}"))
            }
            // Economic.DoTurn26472b0 -> RecalculateValues26472c0 returns
            // before both resource calculations when IsActive26471c0's
            // actual monopolies capability is disabled.
            WorldManager::Economic if !self.capabilities.has(0x10c40410) => Ok(()),
            // HeroesManager.DoTurn2656140 constructs only stack-owned tracking
            // vectors from const Hero/Stats queries and formats two file-log
            // tables. It destroys both vectors at2656666; there are no manager
            // writes, gameplay signals, commands or random calls in this leaf.
            WorldManager::Heroes => Ok(()),
            WorldManager::Congress => round_congress::run(
                state,
                self.congress_definitions,
                self.cityless,
                self.capabilities,
                self.alive_major_ids,
                &mut Favor {
                    backend: self.backend,
                    definitions: self.favor_definitions,
                    climate: self.climate,
                    alive: self.alive_ids,
                    majors: self.alive_major_ids,
                },
            )
            .map_err(|e| format!("{e:?}")),
            _ => return self.effects.manager_turn(state, self.backend, manager),
        };
        result.map_err(|detail| {
            self.backend.last_error = Some(format!("world {manager:?}: {detail}"));
            Error::Unsupported("world manager continuation")
        })
    }
}
