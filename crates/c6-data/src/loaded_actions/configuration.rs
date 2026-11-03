//! Checked typed source configuration inputs. These do not prove later runtime equivalence.
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
    source: &'a [u8],
    game: RawGameConfiguration,
    last_starting_game: RawGameConfiguration,
    levels: Vec<(i32, u32)>,
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
            source,
            game: inputs.game,
            last_starting_game: inputs.last_starting_game,
            levels: inputs.civilization_levels,
            active_root: inputs.active_root,
            last_starting_root: inputs.last_starting_root,
            gameplay_start: inputs.gameplay_start,
        })
    }
    pub fn source(&self) -> ConfigurationSource {
        ConfigurationSource::SourceRead
    }
    pub fn game(&self) -> RawGameConfiguration {
        self.game
    }
    pub fn last_starting_game(&self) -> RawGameConfiguration {
        self.last_starting_game
    }
    pub fn civilization_level_type_id(&self, player: i32) -> Result<u32, Error> {
        self.levels
            .iter()
            .find(|v| v.0 == player)
            .map(|v| v.1)
            .ok_or(Error::Player(player))
    }
    pub fn civilization_level_types(&self) -> &[(i32, u32)] {
        &self.levels
    }
    pub fn original_bytes(&self) -> &'a [u8] {
        self.source
    }
    pub fn active_root_bytes(&self) -> &'a [u8] {
        &self.source[self.active_root.clone()]
    }
    pub fn last_starting_root_bytes(&self) -> &'a [u8] {
        &self.source[self.last_starting_root.clone()]
    }
    pub fn unconsumed_gameplay_bytes(&self) -> &'a [u8] {
        &self.source[self.gameplay_start..]
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
        assert_eq!(config.unconsumed_gameplay_bytes(), &[9, 9]);
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
