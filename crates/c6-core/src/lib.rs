//! Deterministic simulator kernel. Gameplay beyond declared coverage fails closed.
pub mod city;
pub mod city_citizens;
pub mod city_district;
pub mod city_economy;
pub mod city_power_activation;
pub mod city_production;
pub mod city_services;
pub mod city_wire;
pub mod codec;
pub mod combat;
pub mod combat_defender;
pub mod combat_random;
pub mod core_player;
pub mod core_stream;
pub mod diplomacy_drain;
pub mod diplomacy_statement;
pub mod formation_initialization;
pub mod formation_path_cache;
pub mod formation_path_callbacks;
pub mod formation_upkeep;
pub mod geometry;
pub mod improvement_movement;
pub mod improvement_yield;
pub mod loaded_components;
pub mod loaded_state;
pub mod map;
pub mod melee_attack;
pub mod messages;
pub mod movement;
pub mod movement_execution;
pub mod movement_placement;
pub mod movement_shore;
pub mod movement_world;
pub mod notification_lifecycle;
pub mod notifications;
pub mod pathfinder;
pub mod player_ai_identity;
pub mod player_components;
pub mod player_cultural_identity;
pub mod player_culture;
pub mod player_diplomacy;
pub mod player_diplomacy_blackboard;
pub mod player_districts;
pub mod player_eras;
pub mod player_espionage;
pub mod player_great_people_points;
pub mod player_improvements;
pub mod player_influence;
pub mod player_name_manager;
pub mod player_prerequisites;
pub mod player_quests;
pub mod player_religion;
pub mod player_resources;
pub mod player_stats;
pub mod player_techs;
pub mod player_trade;
pub mod player_treasury;
pub mod player_turn_stage;
pub mod player_units;
pub mod player_wmds;
pub mod plot_yield;
pub mod random_variants;
pub mod relic_creation;
pub mod requirements;
pub mod research_acquisition;
pub mod rng;
pub mod skip_turn;
pub mod trade;
pub mod unit;
pub mod unit_healing;
pub mod unit_initialization;
pub mod unit_occupancy;
pub mod unit_operations_clear;
pub mod unit_placement_transition;
pub mod unit_placement_tails;
pub mod unit_sight;
pub mod unit_turn_start;
pub mod unit_turn_upkeep;
pub mod visibility_change;
pub mod visibility_contents;
pub mod visibility_propagation;
pub mod world;
pub mod yields;
use rng::RandomState;
use std::collections::BTreeMap;

pub const SNAPSHOT_VERSION: u32 = 2;

/// Identities must pin the DLL and rules database/mod configuration separately.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Identity {
    pub dll_sha256: [u8; 32],
    pub rules_sha256: [u8; 32],
}

/// Opaque sections preserve future state without claiming to understand it.
/// This is a simulator envelope, not a Civilization VI save-file decoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub version: u32,
    pub identity: Identity,
    pub random: RandomState,
    /// Independent caller-identified GameCore::Random instances; ID 0 is reserved.
    pub random_streams: BTreeMap<u32, RandomState>,
    pub sections: BTreeMap<String, Vec<u8>>,
}
impl Snapshot {
    pub fn new(random: RandomState) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            identity: Identity::default(),
            random,
            random_streams: BTreeMap::new(),
            sections: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Subsystem {
    Random,
    World,
    Players,
    Units,
    Cities,
    Diplomacy,
    Economy,
    Combat,
    Turn,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coverage {
    VerifiedPrimitive,
    Unsupported,
}
pub const COVERAGE: &[(Subsystem, Coverage)] = &[
    (Subsystem::Random, Coverage::VerifiedPrimitive),
    (Subsystem::World, Coverage::Unsupported),
    (Subsystem::Players, Coverage::Unsupported),
    (Subsystem::Units, Coverage::Unsupported),
    (Subsystem::Cities, Coverage::Unsupported),
    (Subsystem::Diplomacy, Coverage::Unsupported),
    (Subsystem::Economy, Coverage::Unsupported),
    (Subsystem::Combat, Coverage::Unsupported),
    (Subsystem::Turn, Coverage::Unsupported),
];

/// Numeric IDs are caller assigned; 0 identifies the legacy default instance.
pub type RngStreamId = u32;
pub const DEFAULT_RNG_STREAM: RngStreamId = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomOperation {
    Draw { bound: u16 },
    Reset { seed: u32 },
    Initialize { seed: u32 },
    Lock,
    Unlock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Random {
        stream: RngStreamId,
        operation: RandomOperation,
    },
    RandomDraw {
        bound: u16,
    },
    RandomReset {
        seed: u32,
    },
    RandomInitialize {
        seed: u32,
    },
    RandomLock,
    RandomUnlock,
    EndTurn,
    /// Preserve caller-defined action bytes until the concrete action is supported.
    Game {
        kind: u32,
        payload: Vec<u8>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Random(u16),
    Applied,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    SnapshotVersion(u32),
    InvalidStreamId,
    DuplicateStream(u32),
    MissingStream(u32),
    Unsupported(Subsystem),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct Engine {
    snapshot: Snapshot,
}
impl Engine {
    pub fn new(snapshot: Snapshot) -> Result<Self, Error> {
        if snapshot.version != SNAPSHOT_VERSION {
            return Err(Error::SnapshotVersion(snapshot.version));
        }
        if snapshot.random_streams.contains_key(&DEFAULT_RNG_STREAM) {
            return Err(Error::InvalidStreamId);
        }
        Ok(Self { snapshot })
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    /// Register an independent instance. This does not identify a native owner.
    pub fn insert_random_stream(
        &mut self,
        id: RngStreamId,
        state: RandomState,
    ) -> Result<(), Error> {
        if id == DEFAULT_RNG_STREAM {
            return Err(Error::InvalidStreamId);
        }
        if self.snapshot.random_streams.contains_key(&id) {
            return Err(Error::DuplicateStream(id));
        }
        self.snapshot.random_streams.insert(id, state);
        Ok(())
    }
    pub fn random_state(&self, id: RngStreamId) -> Result<&RandomState, Error> {
        if id == DEFAULT_RNG_STREAM {
            Ok(&self.snapshot.random)
        } else {
            self.snapshot
                .random_streams
                .get(&id)
                .ok_or(Error::MissingStream(id))
        }
    }
    /// Allocation-free stream routing; absent IDs fail before mutating state.
    pub fn apply_random(
        &mut self,
        id: RngStreamId,
        operation: RandomOperation,
    ) -> Result<Outcome, Error> {
        let state = if id == DEFAULT_RNG_STREAM {
            &mut self.snapshot.random
        } else {
            self.snapshot
                .random_streams
                .get_mut(&id)
                .ok_or(Error::MissingStream(id))?
        };
        match operation {
            RandomOperation::Draw { bound } => Ok(Outcome::Random(state.draw(bound))),
            RandomOperation::Reset { seed } => {
                state.reset(seed);
                Ok(Outcome::Applied)
            }
            RandomOperation::Initialize { seed } => {
                state.initialize(seed);
                Ok(Outcome::Applied)
            }
            RandomOperation::Lock => {
                state.lock();
                Ok(Outcome::Applied)
            }
            RandomOperation::Unlock => {
                state.unlock();
                Ok(Outcome::Applied)
            }
        }
    }
    /// Unsupported operations leave every field unchanged.
    pub fn apply(&mut self, action: &Action) -> Result<Outcome, Error> {
        match action {
            Action::Random { stream, operation } => self.apply_random(*stream, *operation),
            Action::RandomDraw { bound } => {
                self.apply_random(0, RandomOperation::Draw { bound: *bound })
            }
            Action::RandomReset { seed } => {
                self.apply_random(0, RandomOperation::Reset { seed: *seed })
            }
            Action::RandomInitialize { seed } => {
                self.apply_random(0, RandomOperation::Initialize { seed: *seed })
            }
            Action::RandomLock => self.apply_random(0, RandomOperation::Lock),
            Action::RandomUnlock => self.apply_random(0, RandomOperation::Unlock),
            Action::EndTurn => Err(Error::Unsupported(Subsystem::Turn)),
            Action::Game { .. } => Err(Error::Unsupported(Subsystem::World)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_state_preserved_and_unsupported_actions_are_atomic() {
        let mut snapshot = Snapshot::new(RandomState::new(37));
        snapshot
            .sections
            .insert("future.units.v9".into(), vec![0, 255, 7]);
        let mut engine = Engine::new(snapshot.clone()).unwrap();
        assert_eq!(
            engine.apply(&Action::EndTurn),
            Err(Error::Unsupported(Subsystem::Turn))
        );
        assert_eq!(engine.snapshot(), &snapshot);
        engine.apply(&Action::RandomDraw { bound: 10 }).unwrap();
        assert_eq!(engine.snapshot().sections, snapshot.sections);
    }
    #[test]
    fn independent_streams_branch_and_fail_atomically() {
        let mut e = Engine::new(Snapshot::new(RandomState::new(1))).unwrap();
        e.insert_random_stream(7, RandomState::new(1)).unwrap();
        let before = e.snapshot().clone();
        assert_eq!(
            e.insert_random_stream(7, RandomState::new(999)),
            Err(Error::DuplicateStream(7))
        );
        assert_eq!(
            e.insert_random_stream(0, RandomState::new(999)),
            Err(Error::InvalidStreamId)
        );
        assert_eq!(
            e.apply_random(9, RandomOperation::Lock),
            Err(Error::MissingStream(9))
        );
        assert_eq!(e.snapshot(), &before);
        let mut branch = e.clone();
        let a = e
            .apply_random(7, RandomOperation::Draw { bound: 65535 })
            .unwrap();
        assert_eq!(
            a,
            branch
                .apply_random(0, RandomOperation::Draw { bound: 65535 })
                .unwrap()
        );
        assert_eq!(e.random_state(7), branch.random_state(0));
        assert_eq!(e.random_state(0), branch.random_state(7));
        e.apply(&Action::Random {
            stream: 7,
            operation: RandomOperation::Reset { seed: 37 },
        })
        .unwrap();
        assert_eq!(e.random_state(0).unwrap().seed, 1);
    }
    #[test]
    fn unknown_snapshot_version_rejected() {
        let mut snapshot = Snapshot::new(RandomState::new(0));
        snapshot.version += 1;
        assert!(matches!(
            Engine::new(snapshot),
            Err(Error::SnapshotVersion(3))
        ));
    }
}

pub mod tracked_value;

pub mod unit_operation_aftermath;

pub mod reporting_dirty_components;

pub mod player_governors;

pub mod player_heroes;

pub mod unit_operation_notification_provider;

pub mod player_ai_control;

pub mod player_tail;

pub mod player_ai_influence_map;

pub mod player_ai_culture;

pub mod player_ai_city_manager;

pub mod player_ai_economic;

pub mod turn_completion;

pub mod player_ai_diplomatic;

pub mod player_ai_governors;

pub mod player_ai_grand_strategic;

pub mod player_ai_homeland;

pub mod player_ai_research;

pub mod player_ai_military;

pub mod player_ai_tactical;

pub mod player_ai_targets;

pub mod player_ai_behavior_tree;

pub mod unit_operation_interruption;

pub mod game_capabilities;
pub mod game_diplomatic_relations;
pub mod game_diplomatic_relations_wire;
pub mod game_gossip_wire;
pub mod game_name_manager_wire;
pub mod congress_wire;
pub mod game_fallout_wire;
pub mod game_culture_wire;
pub mod game_techs_wire;
pub mod game_effects_wire;
pub mod game_effects_modifiers_wire;
pub mod game_global_ai;
pub mod game_great_people;
pub mod game_quests;

pub mod game_barbarian;

pub mod game_free_cities;
pub mod game_random_wire;

pub mod trade_read_order;

pub mod diplomacy_manager;
pub mod diplomacy_meeting;
pub mod diplomacy_request;

pub mod player_notifications_owner;

pub mod trade_graph_state;

pub mod game_techs;

pub mod hero_discovery_order;

pub mod script_event_registry;

pub mod object_pairing;

pub mod unit_moved_signal;

pub mod plot_adjacent_owner_requirement;

pub mod lua_math_random;

pub mod game_update;

pub mod simple_requirement;
pub mod unit_near_religious_city_requirement;
pub mod unit_modifier_effect;
pub mod game_emergency;
pub mod game_climate;
pub mod game_random_events;
pub mod game_national_parks;
pub mod game_stats;
pub mod game_history;
pub mod game_eras;
pub mod game_archaeology;

pub mod found_city;

pub mod game_deals_wire;

pub mod game_summary_wire;
