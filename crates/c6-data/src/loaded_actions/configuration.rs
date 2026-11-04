//! Checked typed source configuration inputs. These do not prove later runtime equivalence.
use c6_core::core_player::initialization::WireBinding;
use std::ops::Range;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    SourceIdentity,
    Truncated(usize),
    Framing(&'static str),
    Limit,
    UnsupportedFlags { offset: usize, flags: u32 },
    DuplicateKey(u32),
    MissingKey(u32),
    UnsupportedScalar(u32),
    Player(i32),
    RuntimeConfigurationUnverified,
    Missing(&'static str),
    Binding,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux source configuration: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Raw getter inputs. Type IDs/hashes are retained as their original 32 bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawGameConfiguration {
    pub game_mode: u32,
    pub start_year: Option<i32>,
    pub calendar_type: u32,
    pub game_speed_type: u32,
    pub start_era: u32,
}
impl RawGameConfiguration {
    /// Original IsNetworkMultiplayer 0x28e48b0 compares these four enums.
    pub fn is_network_multiplayer(self) -> bool {
        matches!(
            self.game_mode,
            0xe75806e8 | 0x0138409a | 0x07cd5e0a | 0x453f23e2
        )
    }
    /// Original IsPlayByCloud 0x28e4da0 compares this independent mode.
    pub fn is_play_by_cloud(self) -> bool {
        self.game_mode == 0x10ff95a6
    }
}

/// All input bytes remain borrowed, including unknown/unconsumed metadata and
/// compressed gameplay. Parsing configuration does not authorize advancing it.
pub struct Configuration<'a> {
    source: Option<&'a [u8]>,
    game: GameFields,
    last_starting_game: Option<GameFields>,
    levels: Vec<(i32, u32)>,
    binding: Option<WireBinding>,
    active_root: Range<usize>,
    last_starting_root: Range<usize>,
    gameplay_start: usize,
}

/// Retained source ranges and typed getter projections supplied by a decoder.
pub struct SourceInputs {
    pub game: RawGameConfiguration,
    pub last_starting_game: RawGameConfiguration,
    pub civilization_levels: Vec<(i32, u32)>,
    pub active_root: Range<usize>,
    pub last_starting_root: Range<usize>,
    pub gameplay_start: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigurationSource {
    SourceRead,
    RuntimeSnapshot,
}
/// Independently observed semantic fields. None means unobserved, including
/// start_year=None versus Some(None), the latter a known absent source key.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GameFields {
    pub game_mode: Option<u32>,
    pub start_year: Option<Option<i32>>,
    pub calendar_type: Option<u32>,
    pub game_speed_type: Option<u32>,
    pub start_era: Option<u32>,
}
impl From<RawGameConfiguration> for GameFields {
    fn from(v: RawGameConfiguration) -> Self {
        Self {
            game_mode: Some(v.game_mode),
            start_year: Some(v.start_year),
            calendar_type: Some(v.calendar_type),
            game_speed_type: Some(v.game_speed_type),
            start_era: Some(v.start_era),
        }
    }
}
impl GameFields {
    fn complete(self) -> Result<RawGameConfiguration, Error> {
        Ok(RawGameConfiguration {
            game_mode: self.game_mode.ok_or(Error::Missing("game_mode"))?,
            start_year: self
                .start_year
                .ok_or(Error::Missing("start_year presence"))?,
            calendar_type: self.calendar_type.ok_or(Error::Missing("calendar_type"))?,
            game_speed_type: self
                .game_speed_type
                .ok_or(Error::Missing("game_speed_type"))?,
            start_era: self.start_era.ok_or(Error::Missing("start_era"))?,
        })
    }
}
pub struct RuntimeSnapshot {
    pub game: GameFields,
    pub last_starting_game: Option<GameFields>,
    /// Only known slots need be supplied; absence is never civilization level0.
    pub civilization_levels: Vec<(i32, u32)>,
}
impl<'a> Configuration<'a> {
    /// Validate ranges and the complete 64-slot getter projection. Unknown source
    /// bytes remain retained. This is a consistency check, not native authentication.
    pub fn from_source_inputs(source: &'a [u8], inputs: SourceInputs) -> Result<Self, Error> {
        if inputs.active_root.start >= inputs.active_root.end
            || inputs.active_root.end > inputs.last_starting_root.start
            || inputs.last_starting_root.start >= inputs.last_starting_root.end
            || inputs.last_starting_root.end > inputs.gameplay_start
            || inputs.gameplay_start > source.len()
        {
            return Err(Error::Framing("source root ranges"));
        }
        if inputs.civilization_levels.len() != 64
            || inputs
                .civilization_levels
                .iter()
                .enumerate()
                .any(|(i, v)| v.0 != i as i32)
        {
            return Err(Error::Framing("complete civilization level slots"));
        }
        Ok(Self {
            source: Some(source),
            game: inputs.game.into(),
            last_starting_game: Some(inputs.last_starting_game.into()),
            levels: inputs.civilization_levels,
            binding: None,
            active_root: inputs.active_root,
            last_starting_root: inputs.last_starting_root,
            gameplay_start: inputs.gameplay_start,
        })
    }
    pub fn from_runtime_snapshot(
        binding: &WireBinding,
        inputs: RuntimeSnapshot,
    ) -> Result<Self, Error> {
        let levels = inputs.civilization_levels;
        for (index, &(owner, _)) in levels.iter().enumerate() {
            if !(0..64).contains(&owner) {
                return Err(Error::Player(owner));
            }
            if levels[..index].iter().any(|v| v.0 == owner) {
                return Err(Error::Framing("duplicate runtime player"));
            }
        }
        Ok(Self {
            source: None,
            game: inputs.game,
            last_starting_game: inputs.last_starting_game,
            levels,
            binding: Some(binding.clone()),
            active_root: 0..0,
            last_starting_root: 0..0,
            gameplay_start: 0,
        })
    }
    pub fn require_binding(&self, binding: &WireBinding) -> Result<(), Error> {
        if self.binding.as_ref().is_some_and(|v| v != binding) {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    pub fn source(&self) -> ConfigurationSource {
        if self.binding.is_some() {
            ConfigurationSource::RuntimeSnapshot
        } else {
            ConfigurationSource::SourceRead
        }
    }
    pub fn game(&self) -> Result<RawGameConfiguration, Error> {
        self.game.complete()
    }
    pub fn game_fields(&self) -> GameFields {
        self.game
    }
    pub fn last_starting_game(&self) -> Result<RawGameConfiguration, Error> {
        self.last_starting_game
            .ok_or(Error::Missing("last_starting_game"))?
            .complete()
    }
    pub fn is_play_by_cloud(&self, binding: &WireBinding) -> Result<bool, Error> {
        self.require_binding(binding)?;
        Ok(self.game.game_mode.ok_or(Error::Missing("game_mode"))? == 0x10ff95a6)
    }
    /// Native IsAnyMultiplayer28e4cf0 reads only the current GameMode key.
    pub fn is_any_multiplayer(&self, binding: &WireBinding) -> Result<bool, Error> {
        self.require_binding(binding)?;
        Ok(self.game.game_mode.ok_or(Error::Missing("game_mode"))? != 0xe9685ee7)
    }
    pub fn is_network_multiplayer(&self, binding: &WireBinding) -> Result<bool, Error> {
        self.require_binding(binding)?;
        Ok(matches!(
            self.game.game_mode.ok_or(Error::Missing("game_mode"))?,
            0xe75806e8 | 0x0138409a | 0x07cd5e0a | 0x453f23e2
        ))
    }
    pub fn civilization_level_type_id(&self, player: i32) -> Result<u32, Error> {
        self.levels
            .iter()
            .find(|v| v.0 == player)
            .map(|v| v.1)
            .ok_or(Error::Player(player))
    }
    /// Known slots only on runtime snapshots; use the checked per-player getter
    /// when a consumer requires a slot that may not have been observed.
    pub fn civilization_level_types(&self) -> &[(i32, u32)] {
        &self.levels
    }
    pub fn original_bytes(&self) -> Result<&'a [u8], Error> {
        self.source
            .ok_or(Error::Missing("serialized configuration source"))
    }
    pub fn active_root_bytes(&self) -> Result<&'a [u8], Error> {
        Ok(&self.original_bytes()?[self.active_root.clone()])
    }
    pub fn last_starting_root_bytes(&self) -> Result<&'a [u8], Error> {
        Ok(&self.original_bytes()?[self.last_starting_root.clone()])
    }
    pub fn unconsumed_gameplay_bytes(&self) -> Result<&'a [u8], Error> {
        Ok(&self.original_bytes()?[self.gameplay_start..])
    }
    /// No source-only decoder proves subsequent native runtime writes absent.
    pub fn require_runtime_equivalence(&self) -> Result<(), Error> {
        Err(Error::RuntimeConfigurationUnverified)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_mode_comparisons() {
        let game = RawGameConfiguration {
            game_mode: 0,
            start_year: None,
            calendar_type: 0,
            game_speed_type: 0,
            start_era: 0,
        };
        for mode in [0xe75806e8, 0x0138409a, 0x07cd5e0a, 0x453f23e2] {
            assert!((RawGameConfiguration {
                game_mode: mode,
                ..game
            })
            .is_network_multiplayer());
        }
        assert!(!(RawGameConfiguration {
            game_mode: 3915931367,
            ..game
        })
        .is_network_multiplayer());
        let cloud = RawGameConfiguration {
            game_mode: 0x10ff95a6,
            ..game
        };
        assert!(cloud.is_play_by_cloud());
        assert!(!cloud.is_network_multiplayer());
    }
    #[test]
    fn runtime_mode_is_lazy_and_identity_checked() {
        let binding = super::super::choose_tech::current_tests::binding();
        let config = Configuration::from_runtime_snapshot(
            &binding,
            RuntimeSnapshot {
                game: GameFields {
                    game_mode: Some(0x10ff95a6),
                    ..GameFields::default()
                },
                last_starting_game: None,
                civilization_levels: vec![(7, 0)],
            },
        )
        .unwrap();
        assert_eq!(config.source(), ConfigurationSource::RuntimeSnapshot);
        assert_eq!(config.is_play_by_cloud(&binding), Ok(true));
        assert_eq!(config.is_network_multiplayer(&binding), Ok(false));
        assert_eq!(config.game(), Err(Error::Missing("start_year presence")));
        assert_eq!(
            config.last_starting_game(),
            Err(Error::Missing("last_starting_game"))
        );
        assert_eq!(config.civilization_level_type_id(7), Ok(0));
        assert_eq!(config.civilization_level_type_id(0), Err(Error::Player(0)));
        assert_eq!(
            config.original_bytes(),
            Err(Error::Missing("serialized configuration source"))
        );
        let mut other = binding.clone();
        other.inputs.rules_inputs_sha256[0] ^= 1;
        assert_eq!(config.is_play_by_cloud(&other), Err(Error::Binding));
        let missing = Configuration::from_runtime_snapshot(
            &binding,
            RuntimeSnapshot {
                game: GameFields::default(),
                last_starting_game: None,
                civilization_levels: vec![],
            },
        )
        .unwrap();
        assert_eq!(
            missing.is_play_by_cloud(&binding),
            Err(Error::Missing("game_mode"))
        );
    }
    #[test]
    fn checked_source_ranges_and_slots() {
        let game = RawGameConfiguration {
            game_mode: 0,
            start_year: None,
            calendar_type: 0,
            game_speed_type: 0,
            start_era: 0,
        };
        let inputs = || SourceInputs {
            game,
            last_starting_game: game,
            civilization_levels: (0..64).map(|p| (p, 0)).collect(),
            active_root: 0..2,
            last_starting_root: 3..5,
            gameplay_start: 6,
        };
        let bytes = [9; 8];
        let config = Configuration::from_source_inputs(&bytes, inputs()).unwrap();
        assert_eq!(config.unconsumed_gameplay_bytes().unwrap(), &[9, 9]);
        assert_eq!(config.source(), ConfigurationSource::SourceRead);
        assert_eq!(
            config.require_runtime_equivalence(),
            Err(Error::RuntimeConfigurationUnverified)
        );
        let mut bad = inputs();
        bad.civilization_levels[1].0 = 0;
        assert!(Configuration::from_source_inputs(&bytes, bad).is_err());
        let mut bad = inputs();
        bad.last_starting_root = 1..5;
        assert!(Configuration::from_source_inputs(&bytes, bad).is_err());
    }
}
