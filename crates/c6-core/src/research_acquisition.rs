//! Linux XP2 Player.Techs::SetTech and Player.Culture::SetCivic.
//!
//! Executes acquired flags, repeat counts and the synchronous acquisition order.
//! Reached signal handlers, cost updates and recursive boosts are required
//! providers: this module alone does not establish full gameplay/RNG parity.
pub use crate::yields::PrerequisiteKind as Kind;
use crate::{player_culture::Culture, player_prerequisites::IndexedBools, player_techs::Techs};
pub mod units;

pub const TEAM_BOOST_SOURCE: i32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Definition {
    /// Actual runtime enum-to-hash mapping, never an SQL row ID.
    pub hash: u32,
    /// None represents a captured null definition reference at At(index).
    pub repeatable: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    MissingDefinition(i32),
    MissingAcquiredFlag { player: i32, hash: u32 },
    MissingBoostFlag { player: i32, hash: u32 },
}

pub trait Context {
    type Error;
    /// Validate complete reached state and all synchronous effect providers,
    /// including recursive boosts, before the first mutation. A missing
    /// gameplay/RNG handler must be unsupported, never an empty dispatcher.
    fn preflight_acquisition(
        &self,
        player: i32,
        kind: Kind,
        index: i32,
        value: bool,
    ) -> Result<(), Self::Error>;
    fn definitions(&self, kind: Kind) -> &[Definition];
    fn techs(&self, player: i32) -> Result<&Techs, Self::Error>;
    fn techs_mut(&mut self, player: i32) -> Result<&mut Techs, Self::Error>;
    fn culture(&self, player: i32) -> Result<&Culture, Self::Error>;
    fn culture_mut(&mut self, player: i32) -> Result<&mut Culture, Self::Error>;
    /// Actual live Culture acquired vector size, independently of wire pair
    /// count. The source-read overlay must have been applied to Reset storage.
    fn civic_acquired_count(&self, player: i32) -> Result<usize, Self::Error>;
    /// Actual Game.Techs::HasTech / Game.Culture::HasCivic runtime graph query.
    fn globally_available(&self, kind: Kind, index: i32) -> Result<bool, Self::Error>;
    /// PlayerSignals +0x18 for tech; +0 for civic. Execute all subscribers.
    fn signal_player_acquired(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
    ) -> Result<(), Self::Error>;
    /// PlayerSignals +0x2b8, AI_EVENT_GAIN_TECH/CIVIC and null analysis pointer.
    /// This is a synchronous signal, not authorization for native AI choices.
    fn signal_gain_analysis(&mut self, player: i32, kind: Kind) -> Result<(), Self::Error>;
    fn recompute_unit_costs(&mut self, player: i32) -> Result<(), Self::Error>;
    fn update_units_for_tech(&mut self, player: i32) -> Result<(), Self::Error>;
    fn recompute_district_costs(&mut self, player: i32) -> Result<(), Self::Error>;
    fn early_research_agreement(&mut self, player: i32, tech: i32) -> Result<(), Self::Error>;
    /// Native Manager::GetAliveMajors collection in actual iteration order.
    fn alive_majors(&self) -> Result<Vec<i32>, Self::Error>;
    fn team(&self, player: i32) -> Result<i32, Self::Error>;
    fn trigger_boost(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        source: i32,
    ) -> Result<(), Self::Error>;
    /// GameSignals +0x12f0 for tech; +0x498 for civic, (player,index).
    fn signal_game_acquired(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
    ) -> Result<(), Self::Error>;
}

fn flags<C: Context>(
    c: &C,
    player: i32,
    kind: Kind,
    boosts: bool,
) -> Result<&IndexedBools, C::Error> {
    Ok(match kind {
        Kind::Technology => {
            let state = c.techs(player)?;
            if boosts {
                &state.boost_triggered
            } else {
                &state.has_tech
            }
        }
        Kind::Civic => {
            let state = c.culture(player)?;
            if boosts {
                &state.boost_triggered
            } else {
                &state.has_civic
            }
        }
    })
}

fn flag(flags: &IndexedBools, hash: u32) -> Option<bool> {
    // The last hash pair wins during native ReadIndexHashArray. Earlier
    // duplicates remain wire history and are not additional runtime entries.
    flags
        .pairs
        .iter()
        .rev()
        .find(|p| p.definition_hash == hash)
        .map(|p| p.value)
}

/// SetTech(-1), out-of-range indices and inactive technologies do nothing.
/// SetCivic first normalizes its enum/hash through GetIndex (only unsigned
/// values above 0x7ff are hashes). It writes an in-range acquired flag even
/// when the global civic graph marks that civic inactive.
pub fn set<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    index_or_hash: i32,
    value: bool,
) -> Result<(), Error<C::Error>> {
    use Error::Context;
    let raw = index_or_hash as u32;
    let index = if kind == Kind::Civic && raw as usize >= c.definitions(kind).len() {
        if raw <= 0x7ff {
            return Ok(());
        }
        let Some(index) = c.definitions(kind).iter().rposition(|d| d.hash == raw) else {
            return Ok(());
        };
        index as i32
    } else {
        index_or_hash
    };
    let count = match kind {
        Kind::Technology => c.definitions(kind).len(),
        Kind::Civic => c.civic_acquired_count(player).map_err(Context)?,
    };
    if (index as u32 as usize) >= count {
        return Ok(());
    }
    if kind == Kind::Technology && !c.globally_available(kind, index).map_err(Context)? {
        return Ok(());
    }
    let definition = *c
        .definitions(kind)
        .get(index as usize)
        .ok_or(Error::MissingDefinition(index))?;
    let old = flag(
        flags(c, player, kind, false).map_err(Context)?,
        definition.hash,
    )
    .ok_or(Error::MissingAcquiredFlag {
        player,
        hash: definition.hash,
    })?;
    c.preflight_acquisition(player, kind, index, value)
        .map_err(Context)?;
    let acquired = match kind {
        Kind::Technology => &mut c.techs_mut(player).map_err(Context)?.has_tech,
        Kind::Civic => &mut c.culture_mut(player).map_err(Context)?.has_civic,
    };
    acquired
        .pairs
        .iter_mut()
        .rev()
        .find(|p| p.definition_hash == definition.hash)
        .expect("checked acquired flag")
        .value = value;
    // Civic HasCivic is deliberately queried after the flag write.
    let active = kind == Kind::Technology || c.globally_available(kind, index).map_err(Context)?;
    if !active || !value {
        return Ok(());
    }
    if old && definition.repeatable == Some(true) {
        match kind {
            Kind::Technology => {
                let state = c.techs_mut(player).map_err(Context)?;
                state.num_techs_repeated = state.num_techs_repeated.wrapping_add(1);
            }
            Kind::Civic => {
                let state = c.culture_mut(player).map_err(Context)?;
                state.num_civics_repeated = state.num_civics_repeated.wrapping_add(1);
            }
        }
    }
    c.signal_player_acquired(player, kind, index)
        .map_err(Context)?;
    c.signal_gain_analysis(player, kind).map_err(Context)?;
    c.recompute_unit_costs(player).map_err(Context)?;
    if kind == Kind::Technology {
        c.update_units_for_tech(player).map_err(Context)?;
    }
    c.recompute_district_costs(player).map_err(Context)?;
    if kind == Kind::Technology {
        c.early_research_agreement(player, index).map_err(Context)?;
    }
    for other in c.alive_majors().map_err(Context)? {
        if other != player && c.team(other).map_err(Context)? == c.team(player).map_err(Context)? {
            let boosted = flag(
                flags(c, other, kind, true).map_err(Context)?,
                definition.hash,
            )
            .ok_or(Error::MissingBoostFlag {
                player: other,
                hash: definition.hash,
            })?;
            if !boosted {
                c.trigger_boost(other, kind, index, TEAM_BOOST_SOURCE)
                    .map_err(Context)?;
            }
        }
    }
    c.signal_game_acquired(player, kind, index)
        .map_err(Context)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::{HashValue, Limits};

    struct Fixture {
        definitions: Vec<Definition>,
        techs: Vec<Techs>,
        culture: Vec<Culture>,
        teams: Vec<i32>,
        majors: Vec<i32>,
        active: bool,
        ready: bool,
        events: Vec<(i32, &'static str)>,
    }
    impl Fixture {
        fn new() -> Self {
            let limits = Limits {
                max_bytes: 4096,
                max_items: 100,
            };
            let mut bytes = vec![0; 4096];
            bytes[..4].copy_from_slice(&crate::player_techs::VERSION.to_le_bytes());
            let mut tech = Techs::decode_prefix(&bytes, limits).unwrap().0;
            bytes[..4].copy_from_slice(&crate::player_culture::VERSION.to_le_bytes());
            let mut culture = Culture::decode_prefix(&bytes, limits).unwrap().0;
            let flags = IndexedBools {
                pairs: vec![HashValue {
                    definition_hash: 17,
                    value: false,
                }],
            };
            tech.has_tech = flags.clone();
            tech.boost_triggered = flags.clone();
            culture.has_civic = flags.clone();
            culture.boost_triggered = flags;
            Self {
                definitions: vec![Definition {
                    hash: 17,
                    repeatable: Some(true),
                }],
                techs: vec![tech; 5],
                culture: vec![culture; 5],
                teams: vec![2, 9, 2, 2, 2],
                majors: vec![4, 0, 2, 1, 3],
                active: true,
                ready: true,
                events: vec![],
            }
        }
        fn event(&mut self, player: i32, name: &'static str) -> Result<(), &'static str> {
            self.events.push((player, name));
            Ok(())
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn preflight_acquisition(
            &self,
            _: i32,
            _: Kind,
            _: i32,
            _: bool,
        ) -> Result<(), Self::Error> {
            if self.ready {
                Ok(())
            } else {
                Err("missing gameplay subscriber")
            }
        }
        fn definitions(&self, _: Kind) -> &[Definition] {
            &self.definitions
        }
        fn techs(&self, p: i32) -> Result<&Techs, Self::Error> {
            Ok(&self.techs[p as usize])
        }
        fn techs_mut(&mut self, p: i32) -> Result<&mut Techs, Self::Error> {
            Ok(&mut self.techs[p as usize])
        }
        fn culture(&self, p: i32) -> Result<&Culture, Self::Error> {
            Ok(&self.culture[p as usize])
        }
        fn culture_mut(&mut self, p: i32) -> Result<&mut Culture, Self::Error> {
            Ok(&mut self.culture[p as usize])
        }
        fn civic_acquired_count(&self, _: i32) -> Result<usize, Self::Error> {
            Ok(1)
        }
        fn globally_available(&self, _: Kind, _: i32) -> Result<bool, Self::Error> {
            Ok(self.active)
        }
        fn signal_player_acquired(&mut self, p: i32, k: Kind, _: i32) -> Result<(), Self::Error> {
            assert_eq!(flag(flags(self, p, k, false)?, 17), Some(true));
            self.event(p, "player")
        }
        fn signal_gain_analysis(&mut self, p: i32, _: Kind) -> Result<(), Self::Error> {
            self.event(p, "analysis")
        }
        fn recompute_unit_costs(&mut self, p: i32) -> Result<(), Self::Error> {
            self.event(p, "units")
        }
        fn update_units_for_tech(&mut self, p: i32) -> Result<(), Self::Error> {
            self.event(p, "tech units")
        }
        fn recompute_district_costs(&mut self, p: i32) -> Result<(), Self::Error> {
            self.event(p, "districts")
        }
        fn early_research_agreement(&mut self, p: i32, _: i32) -> Result<(), Self::Error> {
            // Earlier synchronous callbacks can change what the later team
            // loop observes. Do not precompute team boost recipients.
            self.teams[2] = 9;
            self.event(p, "agreement")
        }
        fn alive_majors(&self) -> Result<Vec<i32>, Self::Error> {
            Ok(self.majors.clone())
        }
        fn team(&self, p: i32) -> Result<i32, Self::Error> {
            Ok(self.teams[p as usize])
        }
        fn trigger_boost(
            &mut self,
            p: i32,
            _: Kind,
            _: i32,
            source: i32,
        ) -> Result<(), Self::Error> {
            assert_eq!(source, 6);
            self.event(p, "boost")
        }
        fn signal_game_acquired(&mut self, p: i32, _: Kind, _: i32) -> Result<(), Self::Error> {
            self.event(p, "game")
        }
    }

    #[test]
    fn technology_effect_order_and_live_team_filter() {
        let mut c = Fixture::new();
        c.techs[3].boost_triggered.pairs[0].value = true;
        set(&mut c, 0, Kind::Technology, 0, true).unwrap();
        assert_eq!(
            c.events,
            vec![
                (0, "player"),
                (0, "analysis"),
                (0, "units"),
                (0, "tech units"),
                (0, "districts"),
                (0, "agreement"),
                (4, "boost"),
                (0, "game"),
            ]
        );
        assert_eq!(c.techs[0].num_techs_repeated, 0);
        c.techs[0].num_techs_repeated = i32::MAX;
        c.events.clear();
        set(&mut c, 0, Kind::Technology, 0, true).unwrap();
        assert_eq!(c.techs[0].num_techs_repeated, i32::MIN);
        assert_eq!(c.events.last(), Some(&(0, "game")));
    }

    #[test]
    fn civic_order_repeats_and_clear_without_effects() {
        let mut c = Fixture::new();
        c.culture[2].boost_triggered.pairs[0].value = true;
        c.culture[0].has_civic.pairs[0].value = true;
        c.culture[0].num_civics_repeated = 10;
        set(&mut c, 0, Kind::Civic, 0, true).unwrap();
        assert_eq!(
            c.events,
            vec![
                (0, "player"),
                (0, "analysis"),
                (0, "units"),
                (0, "districts"),
                (4, "boost"),
                (3, "boost"),
                (0, "game"),
            ]
        );
        assert_eq!(c.culture[0].num_civics_repeated, 11);
        c.events.clear();
        set(&mut c, 0, Kind::Civic, 0, false).unwrap();
        assert!(!c.culture[0].has_civic.pairs[0].value);
        assert_eq!(c.culture[0].num_civics_repeated, 11);
        assert!(c.events.is_empty());
    }

    #[test]
    fn inactive_graph_guard_differs_and_invalid_indices_are_noops() {
        let mut c = Fixture::new();
        c.active = false;
        set(&mut c, 0, Kind::Technology, 0, true).unwrap();
        assert!(!c.techs[0].has_tech.pairs[0].value);
        set(&mut c, 0, Kind::Civic, 0, true).unwrap();
        assert!(c.culture[0].has_civic.pairs[0].value);
        for kind in [Kind::Technology, Kind::Civic] {
            for index in [-1, -2, 1, i32::MAX] {
                set(&mut c, 0, kind, index, true).unwrap();
            }
        }
        assert!(c.events.is_empty());
    }

    #[test]
    fn missing_handlers_and_missing_flags_fail_before_mutation() {
        let mut c = Fixture::new();
        c.ready = false;
        assert_eq!(
            set(&mut c, 0, Kind::Technology, 0, true),
            Err(Error::Context("missing gameplay subscriber"))
        );
        assert!(!c.techs[0].has_tech.pairs[0].value);
        c.ready = true;
        c.techs[0].has_tech.pairs.clear();
        assert_eq!(
            set(&mut c, 0, Kind::Technology, 0, true),
            Err(Error::MissingAcquiredFlag {
                player: 0,
                hash: 17
            })
        );
        assert!(c.events.is_empty());
    }

    #[test]
    fn repeated_nonrepeatable_still_dispatches_and_duplicate_last_wins() {
        let mut c = Fixture::new();
        c.definitions[0].repeatable = Some(false);
        c.techs[0].has_tech.pairs.push(HashValue {
            definition_hash: 17,
            value: true,
        });
        set(&mut c, 0, Kind::Technology, 0, true).unwrap();
        assert_eq!(c.techs[0].num_techs_repeated, 0);
        assert!(!c.techs[0].has_tech.pairs[0].value);
        assert_eq!(c.events.first(), Some(&(0, "player")));
    }

    #[test]
    fn civic_hash_normalization_has_unsigned_hash_threshold() {
        let mut c = Fixture::new();
        // Even a matching definition hash is not a valid native hash argument
        // below the HashID::IsHash threshold.
        set(&mut c, 0, Kind::Civic, 17, true).unwrap();
        assert!(!c.culture[0].has_civic.pairs[0].value);
        let hash = 0x8000_0111;
        c.definitions[0].hash = hash;
        for state in &mut c.culture {
            state.has_civic.pairs[0].definition_hash = hash;
            state.boost_triggered.pairs[0].definition_hash = hash;
        }
        // Avoid the fixture signal's fixed-hash assertion; inactive civics
        // still write the resolved acquired slot.
        c.active = false;
        set(&mut c, 0, Kind::Civic, hash as i32, true).unwrap();
        assert!(c.culture[0].has_civic.pairs[0].value);
        set(&mut c, 0, Kind::Technology, hash as i32, true).unwrap();
        assert!(!c.techs[0].has_tech.pairs[0].value);
    }
}
