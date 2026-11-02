//! Deterministic simulator kernel. Gameplay beyond declared coverage fails closed.
pub mod codec;
pub mod geometry;
pub mod rng;
use rng::RandomState;
use std::collections::BTreeMap;

pub const SNAPSHOT_VERSION: u32 = 1;

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
    pub sections: BTreeMap<String, Vec<u8>>,
}
impl Snapshot {
    pub fn new(random: RandomState) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            identity: Identity::default(),
            random,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
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
        Ok(Self { snapshot })
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    /// Unsupported operations leave every field unchanged.
    pub fn apply(&mut self, action: &Action) -> Result<Outcome, Error> {
        match action {
            Action::RandomDraw { bound } => Ok(Outcome::Random(self.snapshot.random.draw(*bound))),
            Action::RandomReset { seed } => {
                self.snapshot.random.reset(*seed);
                Ok(Outcome::Applied)
            }
            Action::RandomInitialize { seed } => {
                self.snapshot.random.initialize(*seed);
                Ok(Outcome::Applied)
            }
            Action::RandomLock => {
                self.snapshot.random.lock();
                Ok(Outcome::Applied)
            }
            Action::RandomUnlock => {
                self.snapshot.random.unlock();
                Ok(Outcome::Applied)
            }
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
    fn unknown_snapshot_version_rejected() {
        let mut snapshot = Snapshot::new(RandomState::new(0));
        snapshot.version += 1;
        assert!(matches!(
            Engine::new(snapshot),
            Err(Error::SnapshotVersion(2))
        ));
    }
}
