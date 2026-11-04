//! Bounded post-Map Core saved-wire composition, with a context-free first-entry
//! prefix and a separately attributed conditional whole-Manager path; no gameplay.
use crate::player_prerequisites::{self, IndexedBools};
use crate::player_tail;
use std::sync::Arc;
#[path = "player_initialization.rs"]
pub mod initialization;
use crate::unit::{self, Formation, Limits, PlotCoord};
use crate::{
    city, map, player_components as pc, player_culture, player_diplomacy, player_districts,
    player_name_manager,
};
use crate::{
    player_ai_behavior_tree, player_ai_military, player_ai_research, player_ai_tactical,
    player_ai_targets,
};
use crate::{
    player_ai_city_manager, player_ai_culture, player_ai_diplomatic, player_ai_economic,
    player_ai_governors, player_ai_grand_strategic, player_ai_homeland,
};
use crate::{player_ai_control, player_ai_identity, player_ai_influence_map};
use crate::{player_cultural_identity, player_heroes};
use crate::{
    player_eras, player_great_people_points, player_influence, player_quests, player_trade,
};
use crate::{player_espionage, player_religion, player_resources, player_stats, player_techs};
use crate::{player_governors, player_improvements, player_treasury, player_units, player_wmds};

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
    InvalidContainer,
    InvalidPrefix,
    EntryIndex(u32),
    MissingInitializationSnapshot(initialization::PlayerIdentity),
    Initialization(initialization::Error),
    MilitaryContextMismatch,
    AIMilitary(player_ai_military::Error),
    AIResearch(player_ai_research::Error),
    AITactical(player_ai_tactical::Error),
    AITargets(player_ai_targets::Error),
    AIBehaviorTree(player_ai_behavior_tree::Error),
    OwnerMismatch { requested: i32, record: i32 },
    Formation(unit::Error),
    Component(pc::Error),
    City(city::Error),
    Diplomacy(player_diplomacy::Error),
    Districts(player_districts::Error),
    NameManager(player_name_manager::Error),
    Culture(player_culture::Error),
    Espionage(player_espionage::Error),
    Religion(player_religion::Error),
    Resources(player_resources::Error),
    Stats(player_stats::Error),
    Techs(player_techs::Error),
    Treasury(player_treasury::Error),
    Units(player_units::Error),
    Eras(player_eras::Error),
    Trade(player_trade::Error),
    GreatPeoplePoints(player_great_people_points::Error),
    Influence(player_influence::Error),
    Quests(player_quests::Error),
    Wmds(player_wmds::Error),
    Improvements(player_improvements::Error),
    Governors(player_governors::Error),
    CulturalIdentity(player_cultural_identity::Error),
    Heroes(player_heroes::Error),
    IndexedBools(player_prerequisites::Error),
    AIControl(player_ai_control::Error),
    AIInfluenceMap(player_ai_influence_map::Error),
    AIPlanning(player_ai_identity::Error),
    Tail(player_tail::Error),
    AICityManager(player_ai_city_manager::Error),
    AICulture(player_ai_culture::Error),
    AIDiplomatic(player_ai_diplomatic::Error),
    AIEconomic(player_ai_economic::Error),
    AIGovernors(player_ai_governors::Error),
    AIGrandStrategic(player_ai_grand_strategic::Error),
    AIHomeland(player_ai_homeland::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux post-Map prefix: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Eq, PartialEq)]
pub struct ReligionData {
    pub religion_hash: u32,
    pub founder: i32,
    pub founded_turn: i32,
    pub holy_city: PlotCoord,
    pub custom_name: Vec<u8>,
    pub belief_hashes: Vec<u32>,
    pub building_hashes: Vec<u32>,
    pub unit_hashes: Vec<u32>,
    pub anyone_condemns_favor: i32,
    pub condemn_prohibited: bool,
    pub auto_spreads_to_new_cities: bool,
    pub awards_envoy: bool,
    pub awards_envoy_religious_pressure: i32,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Religion {
    pub minimum_faith_for_next_pantheon: i32,
    pub next_pantheon_founder: i32,
    pub records: Vec<ReligionData>,
}
#[derive(Debug, Eq, PartialEq)]
pub struct PlayerHeader {
    pub owner: i32,
    pub starting_plot: PlotCoord,
    /// Fifteen original ordered i32/enum words; no inferred semantic names.
    pub ordered_scalars: [i32; 15],
    pub ordered_flags: [bool; 6],
    pub handicap_hash: u32,
    pub district_hash: u32,
    pub project_hash: u32,
    pub final_scalar: i32,
}
impl PlayerHeader {
    /// Saved Player47 m_bIsAlive (first ordered bool); this does not establish
    /// initialized validity, configuration, team/human status or Read fixups.
    /// Exact Linux source contract: docs/linux-player-header-control.md.
    pub fn saved_is_alive(&self) -> bool {
        self.ordered_flags[0]
    }
    /// Saved Player47 m_bWasEverAlive, controlling the post-Heroes wire branch.
    /// It does not establish initialized validity, configuration or AI policy.
    pub fn saved_was_ever_alive(&self) -> bool {
        self.ordered_flags[1]
    }
}
/// Bounded saved header only. Version38 support does not authorize reading its
/// unframed Player body. See docs/linux-player-header-versions.md.
#[derive(Debug, Eq, PartialEq)]
pub struct VersionedPlayerHeader {
    pub version: u32,
    /// Original scalar wire values; absent v38 scalars [11] and [13] default to 0.
    /// Hashes retain wire identities, without native definition-index remapping.
    pub saved: PlayerHeader,
}
impl VersionedPlayerHeader {
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let version = r.u32()?;
        if version != 38 && version != 47 {
            return Err(Error::Version {
                expected: 47,
                actual: version,
            });
        }
        let saved = r.header_body(version)?;
        Ok((Self { version, saved }, r.pos))
    }
    /// Proven native Read scalar assignments only, before runtime fixups.
    /// v38 pillage modifiers use x86 wrapping 32-bit arithmetic: 100*x - 100.
    pub fn read_applied_scalars(&self) -> [i32; 15] {
        let mut values = self.saved.ordered_scalars;
        if self.version == 38 {
            for value in &mut values[1..3] {
                *value = value.wrapping_mul(100).wrapping_sub(100);
            }
        }
        values
    }
}
/// Diagnostic traversal from the post-Map boundary to the first Formations.
/// No Formations or other Player body bytes are consumed.
#[derive(Debug, Eq, PartialEq)]
pub struct FirstPlayerHeaderPrefix {
    pub religion: Religion,
    pub player_count: u32,
    pub requested_owner: i32,
    pub header: VersionedPlayerHeader,
}
impl FirstPlayerHeaderPrefix {
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        r.first_player_header()
    }
}
/// Typed saved body before Cities, shared by the strict Player47 reader and
/// the bounded Player38/47 traversal. No runtime initialization is inferred.
#[derive(Debug, Eq, PartialEq)]
pub struct PlayerPreCities {
    pub formations: map::Recycled<Formation>,
    pub goody_hut: pc::GoodyHut,
    pub bonuses: pc::Bonuses,
    pub congress: pc::Congress,
}
/// Explicit failure at an unconsumed nested component. Offset is relative to
/// the supplied post-Map bytes; nested errors never authorize skipping bytes.
#[derive(Debug, Eq, PartialEq)]
pub struct PlayerPrefixBoundary {
    pub offset: usize,
    pub component: &'static str,
    pub error: Error,
}
#[derive(Debug, Eq, PartialEq)]
pub struct FirstPlayerCitiesPrefix {
    pub header_prefix: FirstPlayerHeaderPrefix,
    pub body: PlayerPreCities,
}
impl FirstPlayerCitiesPrefix {
    /// Stops before Cities, retaining the saved version and Read projection.
    /// Uses the same nested readers and shared budget as strict Player47.
    pub fn decode_prefix(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize), PlayerPrefixBoundary> {
        Self::decode_prefix_with_items(bytes, limits).map(|(value, bytes, _)| (value, bytes))
    }
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), PlayerPrefixBoundary> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let (header_prefix, _) = r
            .first_player_header()
            .map_err(|error| PlayerPrefixBoundary {
                offset: r.pos,
                component: "Player header",
                error,
            })?;
        let body = r.pre_cities()?;
        Ok((
            Self {
                header_prefix,
                body,
            },
            r.pos,
            r.items,
        ))
    }
    /// Decode a supported Cities component without asserting Player38 body or
    /// initialized gameplay-state completeness. Input begins at Cities.
    pub fn decode_cities_prefix(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Cities, usize), PlayerPrefixBoundary> {
        Self::decode_cities_prefix_with_items(bytes, limits).map(|(value, size, _)| (value, size))
    }
    /// Also return the actual aggregate dynamic-element charge, so a bounded
    /// continuation can subtract both bytes and items before its next child.
    /// This does not admit a complete Player38 body or runtime initialization.
    pub fn decode_cities_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Cities, usize, usize), PlayerPrefixBoundary> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let cities = r.cities().map_err(|error| PlayerPrefixBoundary {
            offset: r.pos,
            component: "Cities",
            error,
        })?;
        Ok((cities, r.pos, r.items))
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct Cities {
    pub wire_version: u32,
    /// False means Read leaves the prior native vector unchanged. The empty
    /// transport vector then establishes no constructor/runtime baseline.
    pub building_build_overrides_wire_present: bool,
    pub records: map::Recycled<city::City>,
    pub fresh_water_housing_bonus: bool,
    pub urban_defenses: i32,
    pub home_continent_hash: u32,
    pub settled_continent_hashes: Vec<u32>,
    pub peak_cities: u32,
    pub extra_starting_population_off_home_continent: i32,
    pub population_after_conquest: i32,
    pub banned_building_hashes: Vec<u32>,
    pub banned_district_hashes: Vec<u32>,
    pub banned_district_building_hashes: Vec<u32>,
    pub banned_project_hashes: Vec<u32>,
    pub building_build_overrides: Vec<[u32; 2]>,
}
/// Present exactly when the saved Player47 WasEverAlive flag is true.
/// Definition-table wire order/hash identities are preserved without remapping.
#[derive(Debug, Eq, PartialEq)]
pub struct PostHeroesArrays {
    pub unit_definitions: IndexedBools,
    pub feature_definitions: IndexedBools,
    pub continent_definitions: IndexedBools,
}
#[derive(Debug, Eq, PartialEq)]
pub struct FirstPlayer {
    /// Explicit Manager wire ID, retained separately from the embedded ID.
    pub requested_owner: i32,
    pub header: PlayerHeader,
    pub formations: map::Recycled<Formation>,
    pub goody_hut: pc::GoodyHut,
    pub bonuses: pc::Bonuses,
    pub congress: pc::Congress,
    pub cities: Cities,
    pub diplomacy: player_diplomacy::Diplomacy,
    pub districts: player_districts::Districts,
    pub name_manager: player_name_manager::NameManager,
    pub culture: player_culture::Culture,
    pub espionage: player_espionage::Espionage,
    pub religion: player_religion::Religion,
    pub resources: player_resources::Resources,
    pub stats: player_stats::Stats,
    pub techs: player_techs::Techs,
    pub treasury: player_treasury::Treasury,
    pub units: player_units::Units,
    pub eras: player_eras::Eras,
    pub trade: player_trade::Trade,
    pub great_people_points: player_great_people_points::GreatPeoplePoints,
    pub influence: player_influence::Influence,
    pub quests: player_quests::Quests,
    pub wmds: player_wmds::Wmds,
    pub improvements: player_improvements::Improvements,
    pub governors: player_governors::Governors,
    pub cultural_identity: player_cultural_identity::CulturalIdentity,
    pub heroes: player_heroes::Heroes,
    pub post_heroes_arrays: Option<PostHeroesArrays>,
    pub ai_control: Option<player_ai_control::ControlManager>,
    pub ai_influence_map: Option<player_ai_influence_map::InfluenceMap>,
    pub ai_planning: Option<player_ai_identity::PlanningManagerIdentity>,
    pub ai_city_manager: Option<player_ai_city_manager::CityManager>,
    pub ai_culture: Option<player_ai_culture::CultureAi>,
    pub ai_diplomatic: Option<player_ai_diplomatic::Diplomatic>,
    pub ai_economic: Option<player_ai_economic::Economic>,
    pub ai_governors: Option<player_ai_governors::Governors>,
    pub ai_grand_strategic: Option<player_ai_grand_strategic::GrandStrategic>,
    pub ai_homeland: Option<player_ai_homeland::Homeland>,

    pub initialization_snapshot: Option<Arc<initialization::InitializationSnapshot>>,
    pub ai_military: Option<player_ai_military::Military>,
    pub ai_research: Option<player_ai_research::Research>,
    pub ai_tactical: Option<player_ai_tactical::Tactical>,
    pub ai_targets: Option<player_ai_targets::Targets>,
    pub ai_behavior_tree: Option<player_ai_behavior_tree::BehaviorTreeManager>,
    /// Complete suffix for a closed false entry or fully contextual true entry.
    pub tail: Option<player_tail::Tail>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    /// Manager has no entries; later Game components remain unsupported.
    AfterEmptyPlayerManager,
    /// The first entry is partial. Remaining includes that entry's unfinished tail.
    /// WasEverAlive=true: all supported ordered AI wire records consumed; Military context is still required.
    AIMilitary,
    /// Represented wire entries are closed; next explicit requested ID is unread.
    NextPlayerEntry,
    /// Every represented Manager entry is closed; later Game components are unread.
    AfterPlayerManager,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Prefix {
    pub religion: Religion,
    pub player_count: u32,
    pub first_player: Option<FirstPlayer>,
    /// Additional ordered Manager entries, without cloning the first entry.
    pub additional_players: Vec<FirstPlayer>,
    /// Explicit attributed Core role; source-only bindings exist only in tests.
    pub initialization_binding: Option<initialization::WireBinding>,
    pub boundary: Boundary,
    /// Counts unfinished Manager wire entries, derived from actual represented
    /// records; context-free true-branch imports remain partial.
    pub unfinished_player_entries: u32,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn first_player_header(&mut self) -> Result<(FirstPlayerHeaderPrefix, usize), Error> {
        let religion = self.religion()?;
        self.version(1)?;
        let player_count = self.u32()?;
        if player_count == 0 {
            return Err(Error::InvalidPrefix);
        }
        if player_count as usize > self.remaining().max_items {
            return Err(Error::ResourceLimit);
        }
        self.charge(0, 1)?;
        let requested_owner = self.i32()?;
        let (header, consumed) =
            VersionedPlayerHeader::decode_prefix(&self.bytes[self.pos..], self.remaining())?;
        self.charge(consumed, 0)?;
        if requested_owner != header.saved.owner {
            return Err(Error::OwnerMismatch {
                requested: requested_owner,
                record: header.saved.owner,
            });
        }
        Ok((
            FirstPlayerHeaderPrefix {
                religion,
                player_count,
                requested_owner,
                header,
            },
            self.pos,
        ))
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
    fn charge(&mut self, bytes: usize, items: usize) -> Result<(), Error> {
        let pos = self.pos.checked_add(bytes).ok_or(Error::ResourceLimit)?;
        let count = self.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        if pos > self.limits.max_bytes || count > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        self.pos = pos;
        self.items = count;
        Ok(())
    }
    fn raw(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let start = self.pos;
        self.charge(size, 0)?;
        self.bytes.get(start..self.pos).ok_or(Error::Truncated)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(Error::InvalidBool(v)),
        }
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = self.u32()?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::Version { expected, actual })
        }
    }
    fn coord(&mut self) -> Result<PlotCoord, Error> {
        Ok(PlotCoord {
            x: self.i32()?,
            y: self.i32()?,
        })
    }
    fn vec<T>(&mut self, count: usize, min: usize) -> Result<Vec<T>, Error> {
        self.charge(0, count)?;
        if count > self.bytes.len().saturating_sub(self.pos) / min {
            return Err(Error::Truncated);
        }
        if count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        Ok(out)
    }
    fn list<T>(
        &mut self,
        min: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.u32()? as usize;
        let mut out = self.vec(n, min)?;
        for _ in 0..n {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn indexed_bools(&mut self) -> Result<IndexedBools, Error> {
        let start = self.pos;
        let count = self.u32()? as usize;
        self.charge(0, count)?;
        self.raw(count.checked_mul(5).ok_or(Error::ResourceLimit)?)?;
        let wire = self.pos - start;
        let storage = count
            .checked_mul(std::mem::size_of::<unit::HashValue<bool>>())
            .filter(|b| *b <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        // Wire bytes/items have already spent the shared budget. IndexedBools'
        // separate typed-vector check may require more bytes than its 5B pair.
        let (value, used) = IndexedBools::decode_prefix(
            &self.bytes[start..self.pos],
            Limits {
                max_bytes: wire.max(storage),
                max_items: count,
            },
        )
        .map_err(Error::IndexedBools)?;
        debug_assert_eq!(used, wire);
        Ok(value)
    }
    fn hashes(&mut self) -> Result<Vec<u32>, Error> {
        self.list(4, Self::u32)
    }
    fn component<T, E>(
        &mut self,
        read: impl FnOnce(&[u8], Limits) -> Result<(T, usize, usize), E>,
        error: impl FnOnce(E) -> Error,
    ) -> Result<T, Error> {
        let (v, n, i) = read(&self.bytes[self.pos..], self.remaining()).map_err(error)?;
        self.charge(n, i)?;
        Ok(v)
    }
    fn religion_data(&mut self) -> Result<ReligionData, Error> {
        self.version(7)?;
        Ok(ReligionData {
            religion_hash: self.u32()?,
            founder: self.i32()?,
            founded_turn: self.i32()?,
            holy_city: self.coord()?,
            custom_name: self.list(1, |r| Ok(r.raw(1)?[0]))?,
            belief_hashes: self.hashes()?,
            building_hashes: self.hashes()?,
            unit_hashes: self.hashes()?,
            anyone_condemns_favor: self.i32()?,
            condemn_prohibited: self.boolean()?,
            auto_spreads_to_new_cities: self.boolean()?,
            awards_envoy: self.boolean()?,
            awards_envoy_religious_pressure: self.i32()?,
        })
    }
    fn religion(&mut self) -> Result<Religion, Error> {
        self.version(7)?;
        Ok(Religion {
            minimum_faith_for_next_pantheon: self.i32()?,
            next_pantheon_founder: self.i32()?,
            records: self.list(51, Self::religion_data)?,
        })
    }
    fn header(&mut self) -> Result<PlayerHeader, Error> {
        self.version(47)?;
        self.header_body(47)
    }
    fn header_body(&mut self, version: u32) -> Result<PlayerHeader, Error> {
        let owner = self.i32()?;
        let starting_plot = self.coord()?;
        let mut ordered_scalars = [0; 15];
        for (index, value) in ordered_scalars.iter_mut().enumerate() {
            if version == 38 && (index == 11 || index == 13) {
                // ReadVersioned since39/since45 explicitly assigns default0.
                continue;
            }
            *value = self.i32()?;
        }
        let mut ordered_flags = [false; 6];
        for v in &mut ordered_flags {
            *v = self.boolean()?;
        }
        Ok(PlayerHeader {
            owner,
            starting_plot,
            ordered_scalars,
            ordered_flags,
            handicap_hash: self.u32()?,
            district_hash: self.u32()?,
            project_hash: self.u32()?,
            final_scalar: self.i32()?,
        })
    }
    fn formation(&mut self) -> Result<Formation, Error> {
        // Exact current fixed header, counted PathNode1 records, coordinate, IDs.
        // Strict existing codec validates every field after structural sizing.
        let start = self.pos;
        let limits = self.remaining();
        self.raw(36)?;
        let paths = self.u32()? as usize;
        self.charge(0, paths)?;
        self.raw(paths.checked_mul(20).ok_or(Error::ResourceLimit)?)?;
        self.raw(8)?;
        let units = self.u32()? as usize;
        self.charge(0, units)?;
        self.raw(units.checked_mul(4).ok_or(Error::ResourceLimit)?)?;
        Formation::decode(&self.bytes[start..self.pos], limits).map_err(Error::Formation)
    }
    fn recycled<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<map::Recycled<T>, Error> {
        self.version(65537)?;
        if self.u32()? != 16 {
            return Err(Error::InvalidContainer);
        }
        let capacity = self.u32()?;
        let n = self.u32()? as usize;
        if n > capacity as usize {
            return Err(Error::InvalidContainer);
        }
        let mut entries = self.vec::<map::Slot<T>>(n, 4)?;
        // A sorted auxiliary slot list is bounded by the same item count. Slots
        // themselves retain their native serialized order in `entries`.
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            let index = self.i32()?;
            if index < 0 || index as u32 >= capacity {
                return Err(Error::InvalidContainer);
            }
            slots.push(index);
            entries.push(map::Slot {
                index,
                value: read(self)?,
            });
        }
        slots.sort_unstable();
        if slots.windows(2).any(|s| s[0] == s[1]) {
            return Err(Error::InvalidContainer);
        }
        let current_id_bits = self.u32()?;
        let free_count = self.u32()? as usize;
        let free_head = self.u32()?;
        let free_tail = self.u32()?;
        let mut free_links = self.vec(free_count, 4)?;
        for _ in 0..free_count {
            free_links.push(self.u32()?);
        }
        Ok(map::Recycled {
            capacity,
            entries,
            current_id_bits,
            free_head,
            free_tail,
            free_links,
        })
    }
    fn cities(&mut self) -> Result<Cities, Error> {
        // Target Cities.Read 0x2a57ac0: only these source-closed branches.
        let wire_version = self.u32()?;
        if !matches!(wire_version, 12 | 15) {
            return Err(Error::Version {
                expected: 15,
                actual: wire_version,
            });
        }
        let records =
            self.recycled(|r| r.component(city::City::decode_prefix_with_items, Error::City))?;
        Ok(Cities {
            wire_version,
            building_build_overrides_wire_present: wire_version == 15,
            records,
            fresh_water_housing_bonus: self.boolean()?,
            urban_defenses: self.i32()?,
            home_continent_hash: self.u32()?,
            settled_continent_hashes: self.hashes()?,
            peak_cities: self.u32()?,
            extra_starting_population_off_home_continent: self.i32()?,
            population_after_conquest: if wire_version >= 13 { self.i32()? } else { 0 },
            banned_building_hashes: self.hashes()?,
            banned_district_hashes: self.hashes()?,
            banned_district_building_hashes: self.hashes()?,
            banned_project_hashes: self.hashes()?,
            building_build_overrides: if wire_version == 15 {
                self.list(8, |r| Ok([r.u32()?, r.u32()?]))?
            } else {
                Vec::new()
            },
        })
    }
}
impl Reader<'_> {
    fn pre_cities(&mut self) -> Result<PlayerPreCities, PlayerPrefixBoundary> {
        // Player.Read 0x2b4f9c2..0x2b4fa65: both supported versions read
        // Formations, GoodyHut, Bonuses and Congress (since36), then Cities.
        macro_rules! read {
            ($name:literal, $read:expr) => {{
                let offset = self.pos;
                $read.map_err(|error| PlayerPrefixBoundary {
                    offset,
                    component: $name,
                    error,
                })?
            }};
        }
        read!("Formations", self.version(1));
        let formations = read!("Formations", self.recycled(Reader::formation));
        let goody_hut = read!(
            "GoodyHut",
            self.component(pc::GoodyHut::decode_prefix_with_items, Error::Component)
        );
        let bonuses = read!(
            "Bonuses",
            self.component(pc::Bonuses::decode_prefix_with_items, Error::Component)
        );
        let congress = read!(
            "Congress",
            self.component(pc::Congress::decode_prefix_with_items, Error::Component)
        );
        Ok(PlayerPreCities {
            formations,
            goody_hut,
            bonuses,
            congress,
        })
    }
    fn player_prefix(&mut self) -> Result<FirstPlayer, Error> {
        self.charge(0, 1)?;
        let requested_owner = self.i32()?;
        let header = self.header()?;
        if requested_owner != header.owner {
            return Err(Error::OwnerMismatch {
                requested: requested_owner,
                record: header.owner,
            });
        }
        let PlayerPreCities {
            formations,
            goody_hut,
            bonuses,
            congress,
        } = self.pre_cities().map_err(|boundary| boundary.error)?;
        let cities = self.cities()?;
        let diplomacy = self.component(
            player_diplomacy::Diplomacy::decode_prefix_with_items,
            Error::Diplomacy,
        )?;
        let districts = self.component(
            player_districts::Districts::decode_prefix_with_items,
            Error::Districts,
        )?;
        let name_manager = self.component(
            player_name_manager::NameManager::decode_prefix_with_items,
            Error::NameManager,
        )?;
        let culture = self.component(
            player_culture::Culture::decode_prefix_with_items,
            Error::Culture,
        )?;
        let espionage = self.component(
            player_espionage::Espionage::decode_prefix_with_items,
            Error::Espionage,
        )?;
        let religion = self.component(
            player_religion::Religion::decode_prefix_with_items,
            Error::Religion,
        )?;
        let resources = self.component(
            player_resources::Resources::decode_prefix_with_items,
            Error::Resources,
        )?;
        let stats = self.component(player_stats::Stats::decode_prefix_with_items, Error::Stats)?;
        let techs = self.component(player_techs::Techs::decode_prefix_with_items, Error::Techs)?;
        let treasury = self.component(
            player_treasury::Treasury::decode_prefix_with_items,
            Error::Treasury,
        )?;
        let units = self.component(player_units::Units::decode_prefix_with_items, Error::Units)?;
        let eras = self.component(player_eras::Eras::decode_prefix_with_items, Error::Eras)?;
        let trade = self.component(player_trade::Trade::decode_prefix_with_items, Error::Trade)?;
        let great_people_points = self.component(
            player_great_people_points::GreatPeoplePoints::decode_prefix_with_items,
            Error::GreatPeoplePoints,
        )?;
        let influence = self.component(
            player_influence::Influence::decode_prefix_with_items,
            Error::Influence,
        )?;
        let quests = self.component(
            player_quests::Quests::decode_prefix_with_items,
            Error::Quests,
        )?;
        let wmds = self.component(player_wmds::Wmds::decode_prefix_with_items, Error::Wmds)?;
        let improvements = self.component(
            player_improvements::Improvements::decode_prefix_with_items,
            Error::Improvements,
        )?;
        let governors = self.component(
            player_governors::Governors::decode_prefix_with_items,
            Error::Governors,
        )?;
        let cultural_identity = self.component(
            player_cultural_identity::CulturalIdentity::decode_prefix_with_items,
            Error::CulturalIdentity,
        )?;
        let heroes = self.component(
            player_heroes::Heroes::decode_prefix_with_items,
            Error::Heroes,
        )?;
        let post_heroes_arrays = if header.saved_was_ever_alive() {
            Some(PostHeroesArrays {
                unit_definitions: self.indexed_bools()?,
                feature_definitions: self.indexed_bools()?,
                continent_definitions: self.indexed_bools()?,
            })
        } else {
            None
        };
        let (ai_control, ai_influence_map, ai_planning) = if header.saved_was_ever_alive() {
            (
                Some(self.component(
                    player_ai_control::ControlManager::decode_prefix_with_items,
                    Error::AIControl,
                )?),
                Some(self.component(
                    player_ai_influence_map::InfluenceMap::decode_prefix_with_items,
                    Error::AIInfluenceMap,
                )?),
                Some(self.component(
                    player_ai_identity::PlanningManagerIdentity::decode_prefix_with_items,
                    Error::AIPlanning,
                )?),
            )
        } else {
            (None, None, None)
        };
        let ai_city_manager = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_city_manager::CityManager::decode_prefix_with_items,
                Error::AICityManager,
            )?)
        } else {
            None
        };
        let ai_culture = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_culture::CultureAi::decode_prefix_with_items,
                Error::AICulture,
            )?)
        } else {
            None
        };
        let ai_diplomatic = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_diplomatic::Diplomatic::decode_prefix_with_items,
                Error::AIDiplomatic,
            )?)
        } else {
            None
        };
        let ai_economic = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_economic::Economic::decode_prefix_with_items,
                Error::AIEconomic,
            )?)
        } else {
            None
        };
        let ai_governors = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_governors::Governors::decode_prefix_with_items,
                Error::AIGovernors,
            )?)
        } else {
            None
        };
        let ai_grand_strategic = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_grand_strategic::GrandStrategic::decode_prefix_with_items,
                Error::AIGrandStrategic,
            )?)
        } else {
            None
        };
        let ai_homeland = if header.saved_was_ever_alive() {
            Some(self.component(
                player_ai_homeland::Homeland::decode_prefix_with_items,
                Error::AIHomeland,
            )?)
        } else {
            None
        };
        let tail = if header.saved_was_ever_alive() {
            None
        } else {
            Some(self.component(
                |bytes, limits| {
                    player_tail::Tail::decode_prefix_with_items(
                        bytes,
                        player_tail::Context {
                            player_version: 47,
                            was_ever_alive: false,
                        },
                        limits,
                    )
                },
                Error::Tail,
            )?)
        };
        Ok(FirstPlayer {
            requested_owner,
            header,
            formations,
            goody_hut,
            bonuses,
            congress,
            cities,
            diplomacy,
            districts,
            name_manager,
            culture,
            espionage,
            religion,
            resources,
            stats,
            techs,
            treasury,
            units,
            eras,
            trade,
            great_people_points,
            influence,
            quests,
            wmds,
            improvements,
            governors,
            cultural_identity,
            heroes,
            post_heroes_arrays,
            ai_control,
            ai_influence_map,
            ai_planning,
            ai_city_manager,
            ai_culture,
            ai_diplomatic,
            ai_economic,
            ai_governors,
            ai_grand_strategic,
            ai_homeland,

            initialization_snapshot: None,
            ai_military: None,
            ai_research: None,
            ai_tactical: None,
            ai_targets: None,
            ai_behavior_tree: None,
            tail,
        })
    }
    fn complete_player(
        &mut self,
        player: &mut FirstPlayer,
        index: u32,
        binding: &initialization::WireBinding,
        contexts: &impl PlayerInitializations,
        core_origin: usize,
    ) -> Result<(), Error> {
        if !player.header.saved_was_ever_alive() {
            return Ok(());
        }
        let identity = initialization::PlayerIdentity {
            manager_index: index,
            requested_owner: player.requested_owner,
            record_owner: player.header.owner,
        };
        let absolute_cursor = core_origin
            .checked_add(self.pos)
            .ok_or(Error::ResourceLimit)?;
        let snapshot = contexts
            .for_player_at(identity, absolute_cursor)
            .ok_or(Error::MissingInitializationSnapshot(identity))?;
        let context = snapshot
            .context_for(identity, binding)
            .map_err(Error::Initialization)?;
        snapshot
            .check_cursor(absolute_cursor)
            .map_err(Error::Initialization)?;
        let military = self.component(
            |bytes, limits| {
                player_ai_military::Military::decode_prefix_with_items_in_context(
                    bytes, limits, context,
                )
            },
            Error::AIMilitary,
        )?;
        snapshot
            .check_saved_player(military.player_id, binding)
            .map_err(Error::Initialization)?;
        player.ai_military = Some(military);
        player.ai_research = Some(self.component(
            player_ai_research::Research::decode_prefix_with_items,
            Error::AIResearch,
        )?);
        player.ai_tactical = Some(self.component(
            player_ai_tactical::Tactical::decode_prefix_with_items,
            Error::AITactical,
        )?);
        player.ai_targets = Some(self.component(
            player_ai_targets::Targets::decode_prefix_with_items,
            Error::AITargets,
        )?);
        player.ai_behavior_tree = Some(self.component(
            player_ai_behavior_tree::BehaviorTreeManager::decode_prefix_with_items,
            Error::AIBehaviorTree,
        )?);
        player.tail = Some(self.component(
            |bytes, limits| {
                player_tail::Tail::decode_prefix_with_items(
                    bytes,
                    player_tail::Context {
                        player_version: 47,
                        was_ever_alive: true,
                    },
                    limits,
                )
            },
            Error::Tail,
        )?);
        player.initialization_snapshot = Some(snapshot);
        Ok(())
    }
}

/// A read-only metadata provider. Arc cloning shares immutable snapshot inputs;
/// it never clones a Player/world or consumes provider state on parse failure.
pub trait PlayerInitializations {
    fn for_player(
        &self,
        identity: initialization::PlayerIdentity,
    ) -> Option<Arc<initialization::InitializationSnapshot>>;
    /// Absolute Military cursor is source-derived, never a Manager ordinal.
    fn for_player_at(
        &self,
        identity: initialization::PlayerIdentity,
        _absolute_military_cursor: usize,
    ) -> Option<Arc<initialization::InitializationSnapshot>> {
        self.for_player(identity)
    }
}
impl<F> PlayerInitializations for F
where
    F: Fn(initialization::PlayerIdentity) -> Option<Arc<initialization::InitializationSnapshot>>,
{
    fn for_player(
        &self,
        identity: initialization::PlayerIdentity,
    ) -> Option<Arc<initialization::InitializationSnapshot>> {
        self(identity)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InitializationBasis {
    ContextFreeWire,
    SuppliedSnapshotInputs,
    #[cfg(test)]
    SourceOnlySnapshotInputs,
}

impl Prefix {
    /// Input begins immediately after the complete Map wire record. Any suffix
    /// is left untouched at `boundary`; counts are not a complete roster claim.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let religion = r.religion()?;
        r.version(1)?;
        let player_count = r.u32()?;
        // Count is resource-gated even though unsupported later entries are not allocated.
        if player_count as usize > r.remaining().max_items {
            return Err(Error::ResourceLimit);
        }
        let first_player = if player_count == 0 {
            None
        } else {
            Some(r.player_prefix()?)
        };
        let boundary = match first_player.as_ref() {
            Some(p) if p.header.saved_was_ever_alive() => Boundary::AIMilitary,
            Some(_) if player_count > 1 => Boundary::NextPlayerEntry,
            Some(_) => Boundary::AfterPlayerManager,
            None => Boundary::AfterEmptyPlayerManager,
        };
        let completed = u32::from(
            first_player
                .as_ref()
                .is_some_and(|p| !p.header.saved_was_ever_alive()),
        );
        Ok((
            Self {
                religion,
                player_count,
                first_player,
                additional_players: Vec::new(),
                initialization_binding: None,
                boundary,
                unfinished_player_entries: player_count - completed,
            },
            r.pos,
            r.items,
        ))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(bytes, limits).map(|(v, n, _)| (v, n))
    }
}

impl Prefix {
    /// Conditional all-Player wire path. Requires attributed snapshots for every
    /// actual true branch. This never reconstructs native initialization.
    pub fn decode_player_manager_conditional(
        bytes: &[u8],
        limits: Limits,
        binding: initialization::WireBinding,
        contexts: &impl PlayerInitializations,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_player_manager_conditional_at(bytes, limits, binding, contexts, 0)
    }
    /// `core_origin` is the actual absolute cursor where this post-Map slice
    /// begins. Role-admitted snapshots bind their Military start to that cursor.
    pub fn decode_player_manager_conditional_at(
        bytes: &[u8],
        limits: Limits,
        binding: initialization::WireBinding,
        contexts: &impl PlayerInitializations,
        core_origin: usize,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_player_manager_conditional_reserving(
            bytes,
            limits,
            binding,
            contexts,
            core_origin,
            |records, additional| {
                records
                    .try_reserve_exact(additional)
                    .map_err(|_| Error::ResourceLimit)
            },
        )
    }
    fn decode_player_manager_conditional_reserving(
        bytes: &[u8],
        limits: Limits,
        binding: initialization::WireBinding,
        contexts: &impl PlayerInitializations,
        core_origin: usize,
        mut reserve: impl FnMut(&mut Vec<FirstPlayer>, usize) -> Result<(), Error>,
    ) -> Result<(Self, usize, usize), Error> {
        let (mut prefix, consumed, items) = Self::decode_prefix_with_items(bytes, limits)?;
        let mut reader = Reader {
            bytes,
            pos: consumed,
            items,
            limits,
        };
        if let Some(first) = prefix.first_player.as_mut() {
            reader.complete_player(first, 0, &binding, contexts, core_origin)?;
        }
        let count = prefix.player_count.saturating_sub(1) as usize;
        // Preflight entry count, available header wire and addressable storage.
        // Requested ID (4B) + current Player47 header (98B) is a conservative
        // structural minimum, never an observed fixture stride. Item-derived
        // storage is separate from wire bytes, as in the existing unit policy.
        // Each entry spends its one roster item when player_prefix reads it.
        if count > reader.remaining().max_items
            || count > bytes.len().saturating_sub(reader.pos) / 102
            || count > reader.remaining().max_bytes / 102
            || count
                .checked_mul(std::mem::size_of::<FirstPlayer>())
                .is_none_or(|n| n > isize::MAX as usize)
        {
            return Err(Error::ResourceLimit);
        }
        for index in 1..prefix.player_count {
            let mut player = reader.player_prefix()?;
            reader.complete_player(&mut player, index, &binding, contexts, core_origin)?;
            // Reserve only for a fully decoded record. An advertised count
            // never becomes a typed-storage allocation request.
            reserve(&mut prefix.additional_players, 1)?;
            prefix.additional_players.push(player);
        }
        prefix.initialization_binding = Some(binding);
        prefix.unfinished_player_entries = 0;
        prefix.boundary = if prefix.player_count == 0 {
            Boundary::AfterEmptyPlayerManager
        } else {
            Boundary::AfterPlayerManager
        };
        prefix.validate_model()?;
        Ok((prefix, reader.pos, reader.items))
    }
    /// Includes the explicit requested ID before Player47. Derive record cuts by
    /// excluding those first four bytes; no fixture offsets are consulted.
    pub fn encode_player_entry_prefix_with_items(
        &self,
        index: u32,
        limits: Limits,
    ) -> Result<(Vec<u8>, usize), Error> {
        let p = self
            .players()
            .nth(index as usize)
            .ok_or(Error::EntryIndex(index))?;
        p.encode_entry_prefix_with_items(limits, index, self.initialization_binding.as_ref())
    }
}
impl FirstPlayer {
    fn validate_entry(
        &self,
        index: u32,
        binding: Option<&initialization::WireBinding>,
    ) -> Result<bool, Error> {
        if self.requested_owner != self.header.owner {
            return Err(Error::OwnerMismatch {
                requested: self.requested_owner,
                record: self.header.owner,
            });
        }
        let was = self.header.saved_was_ever_alive();
        if [
            self.post_heroes_arrays.is_some(),
            self.ai_control.is_some(),
            self.ai_influence_map.is_some(),
            self.ai_planning.is_some(),
            self.ai_city_manager.is_some(),
            self.ai_culture.is_some(),
            self.ai_diplomatic.is_some(),
            self.ai_economic.is_some(),
            self.ai_governors.is_some(),
            self.ai_grand_strategic.is_some(),
            self.ai_homeland.is_some(),
        ]
        .iter()
        .any(|present| *present != was)
        {
            return Err(Error::InvalidPrefix);
        }
        let later = [
            self.ai_military.is_some(),
            self.ai_research.is_some(),
            self.ai_tactical.is_some(),
            self.ai_targets.is_some(),
            self.ai_behavior_tree.is_some(),
        ];
        if !was {
            if later.iter().any(|v| *v)
                || self.initialization_snapshot.is_some()
                || self.tail.as_ref().is_none_or(|tail| tail.alive.is_some())
            {
                return Err(Error::InvalidPrefix);
            }
            return Ok(true);
        }
        if later.iter().all(|v| !*v) {
            if self.initialization_snapshot.is_some() || self.tail.is_some() {
                return Err(Error::InvalidPrefix);
            }
            return Ok(false);
        }
        if later.iter().any(|v| !*v) || self.tail.as_ref().is_none_or(|tail| tail.alive.is_none()) {
            return Err(Error::InvalidPrefix);
        }
        let snapshot = self
            .initialization_snapshot
            .as_ref()
            .ok_or(Error::InvalidPrefix)?;
        let binding = binding.ok_or(Error::InvalidPrefix)?;
        let identity = initialization::PlayerIdentity {
            manager_index: index,
            requested_owner: self.requested_owner,
            record_owner: self.header.owner,
        };
        let context = snapshot
            .context_for(identity, binding)
            .map_err(Error::Initialization)?;
        let military = self.ai_military.as_ref().unwrap();
        snapshot
            .check_saved_player(military.player_id, binding)
            .map_err(Error::Initialization)?;
        if military.operation_types.is_some() != context.operation_type_arrays_present {
            return Err(Error::MilitaryContextMismatch);
        }
        Ok(true)
    }
    pub fn encode_entry_prefix_with_items(
        &self,
        limits: Limits,
        manager_index: u32,
        binding: Option<&initialization::WireBinding>,
    ) -> Result<(Vec<u8>, usize), Error> {
        self.validate_entry(manager_index, binding)?;
        let mut writer = Writer {
            bytes: Vec::new(),
            items: 0,
            limits,
        };
        writer.player_prefix(self)?;
        Ok((writer.bytes, writer.items))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    /// Complete arbitrary source-only wire carrier for consuming adapter tests;
    /// not an initialized native Player or a runtime constructor default.
    pub(crate) fn source_first_player() -> FirstPlayer {
        complete_source_player(0, true, true).0
    }

    fn words(out: &mut Vec<u8>, values: &[u32]) {
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    fn empty_container(out: &mut Vec<u8>) {
        words(out, &[65537, 16, 8, 0, 0x10000, 2, 7, 3, 7, 3]);
    }
    // Synthetic supported wire values, not native initialization defaults.
    fn diplomacy_carrier() -> player_diplomacy::Diplomacy {
        let mut board_bytes = 13u32.to_le_bytes().to_vec();
        board_bytes.resize(4 + 4 * 4 + 3 * 4 + 9 * 64 * 4, 0);
        let blackboard =
            crate::player_diplomacy_blackboard::Blackboard::decode(&board_bytes, Limits::default())
                .unwrap();
        let mut value = player_diplomacy::Diplomacy {
            wire_version: 50,
            legacy_available_congress_discussions: None,
            had_friendly_meeting: [false; 64],
            has_delegation: [false; 64],
            has_embassy: [false; 64],
            open_borders_from_count: [0; 64],
            open_borders_ended_turn: [0; 64],
            can_enforce_borders: false,
            denounced: [0; 64],
            cities_razed: [0; 64],
            cities_liberated: [0; 64],
            tiles_culture_bombed: [0; 64],
            nukes_used_against_you: [0; 64],
            converted_city: [false; 64],
            occupies_original_city: [false; 64],
            offered_deal: [0; 64],
            war_weariness: [0; 64],
            warmonger_modifier: [0; 64],
            era_player_met: [0; 64],
            yield_per_delegation: Vec::new(),
            yield_per_embassy: Vec::new(),
            war_and_alliance_modifiers: [0; 10],
            warmonger_scaling_raw: 0,
            no_occupation_penalties: false,
            blackboard,
            city_war_weariness: Vec::new(),
            casus_belli: [player_diplomacy::CasusBelliData::default(); 64],
            yield_modifiers: Vec::new(),
            movement_modifiers: Vec::new(),
            combat_modifiers: Vec::new(),
            action_prereq_overrides: Vec::new(),
            banned_action_civilizations: Vec::new(),
            last_war_and_city_turns: [0; 10],
            all_alliances_shared_visibility: false,
            grievance_generation_modifier: 0,
            grievance_decay_modifier: 0,
            grievance_decay_disabled: false,
            alliance_and_favor_modifiers: [0; 9],
            visibility: Vec::new(),
            visibility_source_hashes: Vec::new(),
            visibility_additions: Vec::new(),
            gossip_reports: Vec::new(),
            held_messages: Vec::new(),
            promises_made: Vec::new(),
            incursions_made: Vec::new(),
            last_promises_made: Vec::new(),
            units_blocked_entry: Vec::new(),
            alliance_points_per_player: Vec::new(),
            banned_action_counts: Vec::new(),
        };
        value.promises_made = vec![(
            64,
            player_diplomacy::TypeMap {
                entries: vec![(
                    -1,
                    player_diplomacy::PromiseData {
                        wire_version: 50,
                        turn_made: i32::MIN,
                        skipped: true,
                        broken: false,
                        broken_notification_sent: true,
                        report_repeat_offense: false,
                        turn_broken: i32::MAX,
                    },
                )],
            },
        )];
        value.incursions_made = vec![(
            -1,
            player_diplomacy::TypeMap {
                entries: vec![(64, vec![i32::MIN, 1, i32::MAX])],
            },
        )];
        value.visibility_source_hashes = vec![u32::MAX, 0x80000000];
        value
    }
    // Arbitrary explicit source-wire fields; no runtime constructor defaults.
    fn culture_carrier() -> player_culture::Culture {
        use crate::player_culture as pculture;
        pculture::Culture {
            wire_version: 45,
            current_government_hash: 3735879680,
            post_anarchy_government_hash: 3735879681,
            anarchy_end_turn: -9998,
            turn_adopted: -9997,
            modified_boost: -9996,
            government_change_considered: true,
            government_unlocked: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            government_usage_count: vec![
                unit::HashValue {
                    definition_hash: 0x80000000,
                    value: i32::MIN,
                },
                unit::HashValue {
                    definition_hash: 0x80000000,
                    value: i32::MAX,
                },
            ],
            last_government_used: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: 0x80000000,
            }],
            policy_unlocked: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            policy_purchased: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            policy_has_been_enacted: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            policy_is_in_place: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            policy_banned: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            active_policies: pculture::GovernmentSlots {
                entries: vec![pculture::GovernmentSlot {
                    policy_hash: u32::MAX,
                    slot_ordinal: -3,
                }],
            },
            modified_slots: pculture::GovernmentSlots {
                entries: vec![pculture::GovernmentSlot {
                    policy_hash: u32::MAX,
                    slot_ordinal: -3,
                }],
            },
            modified_removed_slots: pculture::GovernmentSlots {
                entries: vec![pculture::GovernmentSlot {
                    policy_hash: u32::MAX,
                    slot_ordinal: -3,
                }],
            },
            has_civic: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            boost_triggered: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            cultural_progress_bits: vec![
                unit::HashValue {
                    definition_hash: 0x80000000,
                    value: i32::MIN,
                },
                unit::HashValue {
                    definition_hash: 0x80000000,
                    value: i32::MAX,
                },
            ],
            civic_queue_hashes: vec![u32::MAX, 0x80000000, 0],
            cultural_overflow_bits: -9979,
            culture_rate_change_bits: -9978,
            civic_completed_this_turn: true,
            civic_completed_this_turn_hash: 3735879704,
            turn_last_policy_change: -9975,
            num_random_civic_boosts_on_new_era: -9974,
            free_civic_boost_wonder_era: -9973,
            tourism_on_bits: vec![i32::MIN, -1, 0, i32::MAX],
            dominant_over: vec![true, false, true],
            turns_until_dominant: vec![i32::MIN, -1, 0, i32::MAX],
            tourism_modifier: -9969,
            cached_alliance_tourism_bonus_bits: -9968,
            cached_alliance_culture_bonus_bits: -9967,
            religious_tourism_reduction: -9966,
            overall_tourism_reduction: -9965,
            always_full_religious_tourism: -9964,
            lifetime_culture_bits: -9963,
            staycationers: -9962,
            last_turn_staycationers: -9961,
            last_turn_tourists_to: -9960,
            tourists_from: vec![i32::MIN, -1, 0, i32::MAX],
            extra_trade_route_tourism_modifier: -9958,
            auto_themed_building_hash: 3735879723,
            tourism_locations: vec![pculture::TourismLocation {
                plot_index: -9,
                lifetime_tourism: i32::MAX,
                tourist_counts: vec![
                    pculture::TouristCount {
                        player_ordinal: 64,
                        count: -11,
                    },
                    pculture::TouristCount {
                        player_ordinal: 64,
                        count: 12,
                    },
                ],
            }],
            culture_bomb_triggers: vec![pculture::CultureBombTrigger {
                building_hash: 0x80000000,
                district_hash: u32::MAX,
                improvement_hash: 3,
                captures_owned_territory: true,
            }],
            tourism_into_favor: vec![pculture::TourismIntoFavor {
                tourism: -101,
                favor: 102,
            }],
            culture_bomb_converts_city: true,
            unlock_features: vec![pculture::FeatureUnlock {
                feature_hash: u32::MAX,
                civic_hash: 0x80000000,
            }],
            tourism_national_parks_modifier_bits: -9951,
            great_work_object_tourism_modifiers: vec![pculture::GreatWorkObjectTourismModifier {
                great_work_object_ordinal: -1,
                tourism_modifier: i32::MIN,
                no_tourism: true,
            }],
            policy_favor_list: vec![pculture::PolicyFavor {
                policy_hash: 0xdeadbeef,
                favor_per_turn: -13,
            }],
            building_favor_list: vec![pculture::BuildingFavor {
                building_hash: u32::MAX,
                favor_per_turn: 14,
            }],
            other_government_intolerance_multiplier_bits: -9947,
            other_government_intolerance_modifier: -9946,
            previous_government_policies: pculture::GovernmentSlots {
                entries: vec![pculture::GovernmentSlot {
                    policy_hash: u32::MAX,
                    slot_ordinal: -3,
                }],
            },
            num_civics_repeated: -9944,
            auto_themed_buildings_with_x_slots: -9943,
            auto_themed_wonders_with_x_slots: -9942,
            all_great_works_tourism_modifier: -9941,
            all_great_works_yield_modifier: -9940,
            replaced_slots: vec![pculture::GovernmentSlotReplacement {
                replaced_slot_ordinal: -1,
                added_slot_ordinal: 7,
                replaces_all: true,
            }],
            government_type_modified_slots: vec![pculture::GovernmentTypeSlotModifier {
                government_hash: 0xdeadbeef,
                slot_ordinal: -2,
                delta: i32::MIN,
            }],
        }
    }
    // Explicit arbitrary current-wire values, without runtime defaults.
    fn tracked_carrier() -> crate::tracked_value::TrackedValue {
        use crate::tracked_value::{Step, TrackedValue};
        let leaf = TrackedValue {
            value_q8: i32::MIN,
            include_steps: false,
            min_q8: Some(i32::MIN),
            max_q8: Some(i32::MAX),
            steps: vec![],
        };
        let label = map::LocalizedText {
            base: vec![0, 255],
            arguments: vec![
                map::LocalizedArgument::Long64(i64::MIN),
                map::LocalizedArgument::Float32Bits(0x7fc00042),
            ],
            named_arguments: vec![map::NamedArgument {
                argument_index: 0,
                name: vec![128],
            }],
        };
        let branch = TrackedValue {
            value_q8: 7,
            include_steps: false,
            min_q8: None,
            max_q8: Some(9),
            steps: vec![Step {
                priority: -1,
                id: u32::MAX,
                label: label.clone(),
                value: leaf,
            }],
        };
        TrackedValue {
            value_q8: i32::MAX,
            include_steps: false,
            min_q8: Some(-10),
            max_q8: None,
            steps: vec![Step {
                priority: i32::MIN,
                id: 0x80000000,
                label,
                value: branch,
            }],
        }
    }
    fn espionage_carrier() -> player_espionage::Espionage {
        use crate::player_espionage as pmod;
        pmod::Espionage {
            wire_version: 17,
            spy_capacity: -999,
            captured_spies: vec![pmod::CapturedSpy {
                owning_player_ordinal: -998,
                unit_type_hash: 3735879683,
                name_index: -996,
                level: -995,
                promotion_hash: 3735879686,
                promotion_hashes: vec![3735879687],
            }],
            off_map_spies: vec![pmod::OffMapSpy {
                wire_version: 17,
                unit_type_hash: 3735879683,
                name_index: -996,
                level: -995,
                experience_points: -994,
                return_turn: -993,
                return_x: -992,
                return_y: -991,
                return_city_owner_ordinal: -990,
                ai_controller: 3735879691,
                promotion_hashes: vec![3735879692],
                old_unit_id: 3735879693,
            }],
            cities_with_sources: vec![pmod::CityWithSources {
                player_ordinal: -996,
                city_id: 3735879685,
                expire_turn: -994,
            }],
            global_offense_bonus: -995,
            global_defense_bonus: -994,
            extra_steal_tech_boosts: -993,
            mission_histories: vec![pmod::MissionHistoryRecord {
                wire_version: 17,
                name_index: -992,
                operation_hash: 3735879689,
                target_city_id: 3735879690,
                target_player_ordinal: -989,
                target_plot_index: -988,
                completion_turn: -987,
                initial_result_ordinal: -986,
                escape_result_ordinal: -985,
                level_after_mission: -984,
                loot_info: -983,
            }],
            counter_spy_yield_awards: vec![pmod::CounterSpyYieldAward {
                yield_ordinal: -991,
                amount: -990,
                per_x_level: -989,
            }],
            target_city_spy_yield_percents: vec![pmod::TargetCitySpyYieldPercent {
                yield_ordinal: -990,
                percent: -989,
            }],
        }
    }
    fn religion_carrier() -> player_religion::Religion {
        use crate::player_religion as pmod;
        pmod::Religion {
            wire_version: 20,
            faith_balance_q8: -999,
            faith_rate_change_q8: -998,
            pantheon_hash: 3735879683,
            num_beliefs_earned: -996,
            holy_city_coord: pmod::PlotCoord { x: -995, y: -994 },
            belief_yields: vec![pmod::BeliefYield {
                belief_yield_ordinal: -994,
                yield_ordinal: -993,
                amount: -992,
                per_x_items: -991,
                district_hash: 3735879690,
            }],
            building_yield_modifiers: vec![pmod::ReligionBuildingYieldModifier {
                yield_ordinal: -993,
                multiplier: -992,
                founder_only: true,
            }],
            religion_building_discount: -992,
            spread_distance: -991,
            combat_loss: -990,
            inquisition: -989,
            gains_all_follower_beliefs: true,
            founder_belief_of_majority_religion: true,
            faith_per_peaceful_religion_founder: -986,
            add_faith_harvest_modifier: -985,
            add_faith_chop_modifier: -984,
            add_faith_barbarian_plunder_modifier: -983,
            adopts_ally_founded_religions: -982,
            treat_ally_capital_as_holy_city: -981,
            treat_holy_sites_as_holy_city: -980,
            spread_multiplier: -979,
            spread_enhancing_tech_hash: 3735879702,
            tech_enabled_spread_multiplier: -977,
            natural_wonder_relic: -976,
            faith_from_dispersal: -975,
            religious_pressure_per_great_person_activated: -974,
            enable_wall_attack_whole_game_same_religion: -973,
            enable_wall_attack_whole_game_same_religion_promotion_class: vec![unit::HashValue {
                definition_hash: 0x80000000,
                value: i32::MIN,
            }],
            start_inquisition_charges: -971,
            spread_trade_route_pressure_destination_modifier: -970,
            spread_trade_route_pressure_origin_modifier: -969,
            great_person_founder_hash: 3735879712,
            units_religion_strength_by_religion: vec![-967],
        }
    }
    fn resources_carrier() -> player_resources::Resources {
        use crate::player_resources as pmod;
        pmod::Resources {
            wire_version: 16,
            free_resource_visibility: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: true,
            }],
            extra_amenities_per_owned_luxury_resource: -998,
            extra_amenities_per_owned_bonus_resource: -997,
            full_access_one_strategic: true,
            prevent_harvest_resource: -995,
            resource_ownership: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: vec![-994],
            }],
            accumulated_resource_per_turn_q8: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -993,
            }],
            luxury_allocations: vec![pmod::CityResource {
                city_id: 3735879688,
                resource_hash: 3735879689,
            }],
            reserved_resources: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -991,
            }],
            banned_resources: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -990,
            }],
            no_cap_resources: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -989,
            }],
            export_resource_extracted: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -988,
            }],
            import_resource_extracted: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -987,
            }],
            free_resources_extracted: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -986,
            }],
            extra_accumulation_modifier: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -985,
            }],
            resource_stockpile_cap_modifier: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -984,
            }],
            unit_resource_demand_per_turn_q8: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -983,
            }],
            power_resource_demand_per_turn_q8: vec![pmod::ResourceEntry {
                resource_hash: u32::MAX,
                value: -982,
            }],
        }
    }
    fn stats_carrier() -> player_stats::Stats {
        use crate::player_stats as pmod;
        pmod::Stats {
            wire_version: 27,
            legacy_diplomatic_victory_points: None,
            max_melee_strength_trained: -999,
            max_ranged_strength_trained: -998,
            max_bombard_strength_trained: -997,
            military_strength: -996,
            military_strength_from_gold: -995,
            air_military_strength: -994,
            land_military_strength: -993,
            naval_military_strength: -992,
            barbarians_killed: -991,
            barbarian_camps_cleared: -990,
            tiles_pillaged: -989,
            wmd_city_hits: -988,
            cities_revolted: -987,
            cities_annexed: -986,
            total_cities_lost_to_disloyalty: -985,
            total_cities_gained_from_disloyalty: -984,
            total_number_of_alliances_formed: -983,
            total_number_of_emergency_alliances_formed: -982,
            gold_spent_levying: -981,
            emergencies_joined: -980,
            total_governors_appointed: -979,
            total_governor_assignments: -978,
            diplomatic_victory_points: tracked_carrier(),
            last_turn_favor: -976,
            current_turn_favor: -975,
            lifetime_favor: -974,
            science_victory_points_q8: -973,
            science_victory_points_per_turn_q8: -972,
            num_units_constructed: vec![unit::HashValue {
                definition_hash: 0x80000000,
                value: i32::MIN,
            }],
            num_projects_advanced: vec![unit::HashValue {
                definition_hash: 0x80000000,
                value: i32::MIN,
            }],
            wonders_lost_plots: vec![-969],
            cached_yields_q8: vec![unit::HashValue {
                definition_hash: 0x80000000,
                value: i32::MIN,
            }],
            removed_feature_counts: [i32::MIN; 64],
            national_park_counts: [i32::MIN; 64],
            placed_forest_counts: [i32::MIN; 64],
            known_wmd_counts: [i32::MIN; 64],
            cleared_bandit_camp_counts: [i32::MIN; 64],
            city_state_war_counts: [i32::MIN; 64],
            players_revived: [i32::MIN; 64],
            fertility_added_by_droughts: -960,
            tiles_damaged_by_droughts: -959,
            population_lost_by_droughts: -958,
            units_killed_by_droughts: -957,
            fertility_added_by_floods: -956,
            tiles_damaged_by_floods: -955,
            population_lost_by_floods: -954,
            units_killed_by_floods: -953,
            fertility_added_by_storms: -952,
            tiles_damaged_by_storms: -951,
            population_lost_by_storms: -950,
            units_killed_by_storms: -949,
            fertility_added_by_volcanos: -948,
            tiles_damaged_by_volcanos: -947,
            population_lost_by_volcanos: -946,
            units_killed_by_volcanos: -945,
        }
    }
    fn techs_carrier() -> player_techs::Techs {
        use crate::player_techs as pmod;
        pmod::Techs {
            wire_version: 6,
            research_queue_hashes: vec![3735879681],
            research_overflow_q8: -998,
            science_rate_change_q8: -997,
            modified_boost: -996,
            has_tech: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            boost_triggered: crate::player_prerequisites::IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: u32::MAX,
                        value: false,
                    },
                ],
            },
            research_progress_q8: vec![unit::HashValue {
                definition_hash: 0x80000000,
                value: i32::MIN,
            }],
            num_random_tech_boosts_on_new_era: -992,
            num_techs_repeated: -991,
            free_tech_boost_wonder_era: -990,
        }
    }
    // Explicit wire carrier; no runtime initialization defaults.
    fn treasury_carrier() -> player_treasury::Treasury {
        use crate::player_treasury as pmod;
        pmod::Treasury {
            wire_version: 10,
            gold_balance_q8: -1999,
            gold_rate_change_q8: -1998,
            cached_net_income_excluding_trade_q8: -1997,
            bankruptcy_amenities: -1996,
            maintenance_discount_per_unit: -1995,
            unit_upgrade_discount_percent: -1994,
            maintenance_discount_per_wmd_modifier: -1993,
            levied_unit_upgrade_discount_percent: -1992,
            cached_net_income_q8: -1991,
            gold_interest_percent_q8: -1990,
            gold_dispersal_change_q8: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            gold_diplomatic_deals: vec![pmod::DiplomaticDeal {
                first: 3203334156,
                second: 3203334157,
            }],
        }
    }
    // Explicit wire carrier; no runtime initialization defaults.
    fn units_carrier() -> player_units::Units {
        use crate::player_units as pmod;
        pmod::Units {
            wire_version: 22,
            legacy_wall_attack_promotion_class_values: None,
            records: pmod::UnitTable {
                capacity: 257,
                entries: vec![],
                current_id_bits: 0xffff0000,
                free_head: 0x12345678,
                free_tail: u32::MAX,
                free_links: vec![99, 0xffff0001],
            },
            current_unit_cost: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            unit_production_multiplier_q8: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            extra_unit_copy: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            enable_wall_attack_whole_game_promotion_class: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            unit_build_disabled: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: true,
            }],
            no_settling_cities: true,
            no_foreign_heal: true,
            no_neutral_heal: true,
            add_on_district_founding: vec![pmod::DistrictFoundingUnits {
                key: 3203334154,
                values: vec![pmod::MapPair {
                    key: 3203334155,
                    value: 3203334156,
                }],
            }],
            unit_free_pass_hashes: vec![3203334155],
            unit_embarkation_pass_hashes: vec![3203334156],
            extra_build_charges: -1987,
            extra_build_charges_by_unit: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            extra_spread_charges: -1985,
            embarked_movement_modifier: -1984,
            upgrade_military_formation_on_city_conquest_count: -1983,
            promotion_selections: vec![pmod::RandomPromotionSelection {
                wire_version: 22,
                promotion_hash: 3203334162,
                times_selected: -1981,
            }],
            proposed_promotions: vec![pmod::ProposedPromotion {
                wire_version: 22,
                promotion_hash: 3203334163,
                unit_id: 3203334164,
            }],
            base_promotions: vec![pmod::BasePromotion {
                unit_hash: 3203334164,
                promotion_hash: 3203334165,
                count: -1978,
            }],
            unlimited_promotion_choice_unit_hashes: vec![3203334165],
            land_corps_unlock_civic_hash: 3203334166,
            land_army_unlock_civic_hash: 3203334167,
            naval_corps_unlock_civic_hash: 3203334168,
            naval_army_unlock_civic_hash: 3203334169,
            land_corps_unlock_tech_hash: 3203334170,
            land_army_unlock_tech_hash: 3203334171,
            naval_corps_unlock_tech_hash: 3203334172,
            naval_army_unlock_tech_hash: 3203334173,
            peak_units: 3203334174,
            land_corps_modified_strength: -1969,
            land_army_modified_strength: -1968,
            naval_corps_modified_strength: -1967,
            naval_army_modified_strength: -1966,
            enable_wall_attack_whole_game: -1965,
            rock_band_album_sales_modifier: -1964,
            banned_production_yield: -1963,
            favored_production_yield: -1962,
            favored_production_modifier_q8: -1961,
            upgrade_cost_modifier_q8: -1960,
            current_unit_resource_cost: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
            unit_fuel_demands: vec![unit::HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN,
            }],
        }
    }
    // Arbitrary complete Trade35 wire values; no runtime defaults/remapping.
    fn trade_carrier() -> player_trade::Trade {
        player_trade::Trade {
            wire_version: 35,
            player: -3000,
            outgoing_route_capacity: -2999,
            notified_outgoing_route_capacity: -2998,
            extra_route_range: -2997,
            improved_route_level: -2996,
            immediate_trading_post_count: -2995,
            gain_tiles_en_route_radius: -2994,
            anyone_plunder_favor: -2993,
            anyone_trade_to_favor: -2992,
            extra_land_range_refuel: -2991,
            extra_water_range_refuel: -2990,
            free_tech_boost_first_trading_post_each_civ: -2989,
            free_civic_boost_first_trading_post_each_civ: -2988,
            trade_route_capacity_increase_on_meeting: -2987,
            forbid_land_route_domestic: -2986,
            forbid_land_route_international: -2985,
            civ_received_civic_boost_first_trading_post: vec![true, false, true],
            civ_received_tech_boost_first_trading_post: vec![true, false, true],
            flat_yield_bonus: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_domestic: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_city_state: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_international: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_intercontinental_domestic: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_yield_bonus_for_intercontinental_international: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_yield_bonus_for_cultural_dominance: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_for_districts_city_state: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_for_ally_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_for_ally_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_for_suzerain_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_for_suzerain_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_to_others: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            techs_civics_per_yield: -2969,
            per_specialty_district_yield_bonus_for_domestic: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            yield_bonus_per_trading_post_in_foreign_city: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            yield_bonus_per_trading_post_in_own_city: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_per_path_tile: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_per_follower: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            international_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_per_player: std::array::from_fn(|i| vec![(i as u32, i as i32 - 32)]),
            flat_origin_yield_per_improvement_at_destination: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_improvement_at_destination: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_international_yield_per_improvement_at_origin: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            per_specialty_district_yield_bonus_for_international: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_origin_yield_per_terrain_for_domestic: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_terrain_for_domestic: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_origin_yield_per_terrain_for_international: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_terrain_for_international: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            international_majors_trade_disabled_count: -2951,
            international_minors_trade_disabled_count: -2950,
            domestic_trade_disabled_count: -2949,
        }
    }
    // Explicit arbitrary saved-wire fields, not native initialization defaults.
    fn great_people_points_carrier() -> player_great_people_points::GreatPeoplePoints {
        player_great_people_points::GreatPeoplePoints {
            wire_version: 13,
            player: -4000,
            points_total: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            points_earned_this_turn: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            bonus_points_per_turn: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            free_great_person_points: -3996,
            points_refund_percent: -3995,
            no_patronage_faith: true,
            no_patronage_gold: true,
            patronage_discount_percent: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            bonus_points_per_turn_percent: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            yield_modifier_per_earned_great_person: vec![
                (u32::MAX, i32::MIN),
                (u32::MAX, i32::MAX),
            ],
            yield_change_per_earned_great_person_class: vec![
                player_great_people_points::YieldChangePerEarnedGreatPersonClassOnResource {
                    yield_hash: u32::MAX,
                    great_person_class_hash: 0x80000000,
                    resource_hash: 7,
                    amount: i32::MIN,
                },
            ],
            great_person_boost_list: vec![player_great_people_points::GreatPersonEarnedBoost {
                great_person_class_hash: 0x80000000,
                tech_boost: true,
                other_players: false,
            }],
            great_person_class_no_points: vec![u32::MAX, 0x80000000],
        }
    }
    // Explicit arbitrary saved-wire fields, not native initialization defaults.
    fn influence_carrier() -> player_influence::Influence {
        player_influence::Influence {
            player: -4000,
            points_earned: -3999,
            base_points_per_turn: -3998,
            not_earning_points: true,
            points_threshold: -3996,
            tokens_per_threshold: -3995,
            tokens_to_give: -3994,
            duplicate_first_tokens: -3993,
            tokens_on_first_to_meet: -3992,
            tokens_on_trade_route_started: -3991,
            tokens_received: vec![i32::MIN, -1, i32::MAX],
            virtual_tokens_received: vec![i32::MIN, -1, i32::MAX],
            disallow_tokens_from: vec![true, false, true],
            yield_per_given_token: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            suzerain: -3986,
            suzerain_changed_count: -3985,
            suzerain_unique_bonus_disabled_count: -3984,
            levy_turn_counter: -3983,
            yield_modifier_per_tributary: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            yield_per_tributary: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            open_borders_count: vec![i32::MIN, -1, i32::MAX],
            levy_discount_percent_by_player: vec![i32::MIN, -1, i32::MAX],
            copy_luxuries_count: vec![i32::MIN, -1, i32::MAX],
            copy_strategics_count: vec![i32::MIN, -1, i32::MAX],
            duplicate_tokens_when_rival_government: -3976,
            duplicate_tokens_when_trade_route_to: -3975,
            duplicate_tokens_when_same_religion: -3974,
            suzerain_visibility_counter: -3973,
            levy_discount_percent: -3972,
            tokens_received_modifier: vec![i32::MIN, -1, i32::MAX],
            max_tokens_received: vec![i32::MIN, -1, i32::MAX],
        }
    }
    // Explicit arbitrary saved-wire fields, not native initialization defaults.
    fn quests_carrier() -> player_quests::Quests {
        player_quests::Quests { player: -4000 }
    }
    // Arbitrary complete current wire fields; no runtime defaults/normalization.
    fn wmds_carrier() -> player_wmds::Wmds {
        use crate::player_wmds as pmod;
        pmod::Wmds {
            weapons: vec![pmod::WeaponCount {
                definition_hash: 3735879680,
                count: -4999,
            }],
        }
    }
    // Arbitrary complete current wire fields; no runtime defaults/normalization.
    fn improvements_carrier() -> player_improvements::Improvements {
        use crate::player_improvements as pmod;
        pmod::Improvements {
            plot_indices: vec![i32::MIN, -1, i32::MAX],
            improvement_array_size: 3735879681,
        }
    }
    // Arbitrary complete current wire fields; no runtime defaults/normalization.
    fn governors_carrier() -> player_governors::Governors {
        use crate::player_governors as pmod;
        pmod::Governors {
            governor_points: -5000,
            capacity: 8,
            entries: vec![pmod::Slot {
                index: 2,
                governor: pmod::Governor {
                    id: -4997,
                    definition_hash: 3735879684,
                    name: vec![0, 255, 128],
                    assigned_city: pmod::CityId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: 3735879688,
                    },
                    owning_player: -4993,
                    turns_on_site: -4992,
                    cultural_pressure: -4991,
                    domestic_identity_pressure: -4990,
                    foreign_identity_pressure: -4989,
                    domestic_identity_pressure_once_per_city: -4988,
                    foreign_identity_pressure_once_per_city: -4987,
                    established: true,
                    neutralized_turns: -4985,
                    neutralized_indefinitely: -4984,
                    identity_per_title: -4983,
                    alliance_points: -4982,
                    ignore_identity_pressure: -4981,
                    feature_no_improvement_appeal_modifier: -4980,
                    prevent_structural_damage: -4979,
                    assign_to_major: true,
                    resource_power_provided_change: -4977,
                    faith_purchase_districts: -4976,
                    extra_heal: -4975,
                    established_cities: vec![pmod::EstablishedCity {
                        city: pmod::CityId {
                            owner: i16::MIN,
                            component_type: i16::MIN,
                            id: 3735879708,
                        },
                        value: -4973,
                    }],
                    promotions: vec![pmod::Promotion {
                        definition_hash: 3735879707,
                        earned: true,
                    }],
                    govern_city_state: true,
                    grievances: vec![pmod::Grievance {
                        score: -4971,
                        turns: -4970,
                    }],
                },
            }],
            current_id_bits: 3735879683,
            free_head: 3735879684,
            free_tail: 3735879685,
            free_links: vec![9, 9, u32::MAX],
            neutralized_indefinitely: -4993,
            title_considered: true,
            favored_governor_hash: 3735879689,
            diplomatic_favor: -4990,
        }
    }
    // All fourteen signed current-wire fields, with no runtime defaults.
    fn cultural_identity_carrier() -> player_cultural_identity::CulturalIdentity {
        player_cultural_identity::CulturalIdentity {
            extra_capital_pressure_radius: i32::MIN,
            identity_per_turn_for_domestic_trade_route_origin_raw: -5999,
            martial_law_modifier: -5998,
            post_combat_foreign_loyalty: -5997,
            post_combat_local_loyalty: -5996,
            additional_golden_age_post_combat_loyalty: -5995,
            post_pillage_foreign_loyalty: -5994,
            post_pillage_local_loyalty: -5993,
            get_skip_free_city_step: -5992,
            foreign_identity_per_turn_from_nearby_great_works_raw: -5991,
            domestic_identity_per_turn_from_nearby_great_works_raw: -5990,
            foreign_identity_per_turn_from_nearby_wonders_raw: -5989,
            domestic_identity_per_turn_from_nearby_wonders_raw: -5988,
            always_loyal_coastal_home_continent_count: -5987,
        }
    }
    // Arbitrary full current saved Heroes fields, without runtime defaults.
    fn heroes_carrier() -> player_heroes::Heroes {
        use crate::player_heroes as pmod;
        pmod::Heroes {
            discovered: vec![pmod::DiscoveredHero {
                hero_class_hash: 3203334144,
                discovered: true,
            }],
            goody_hut_discovery_chance: -6999,
            player_influence_chances: vec![i32::MIN, -1, i32::MAX],
            player_suzerain_count: vec![i32::MIN, -1, i32::MAX],
            discovery_order: vec![pmod::DiscoveryWeight {
                hero_class_hash: 3203334148,
                weight: -6995,
            }],
            claims: vec![pmod::Claim {
                hero_class_hash: 3203334149,
                claim_era_hash: 3203334150,
                claim_turn: -6993,
                last_appearance_era_hash: 3203334152,
            }],
        }
    }
    fn post_heroes_arrays_carrier() -> PostHeroesArrays {
        fn table(hash: u32) -> IndexedBools {
            IndexedBools {
                pairs: vec![
                    unit::HashValue {
                        definition_hash: hash,
                        value: true,
                    },
                    unit::HashValue {
                        definition_hash: hash,
                        value: false,
                    },
                ],
            }
        }
        PostHeroesArrays {
            unit_definitions: table(u32::MAX),
            feature_definitions: table(0x80000000),
            continent_definitions: table(7),
        }
    }
    fn after_cultural_identity_bytes(p: &FirstPlayer) -> usize {
        let mut bytes = p.heroes.encode(Limits::default()).unwrap().len();
        if let Some(a) = &p.post_heroes_arrays {
            for table in [
                &a.unit_definitions,
                &a.feature_definitions,
                &a.continent_definitions,
            ] {
                bytes += table.encode(Limits::default()).unwrap().len();
            }
        }
        bytes
            + after_post_heroes_arrays_bytes(p)
            + p.tail.as_ref().map_or(0, |value| {
                value
                    .encode(
                        player_tail::Context {
                            player_version: 47,
                            was_ever_alive: false,
                        },
                        Limits::default(),
                    )
                    .unwrap()
                    .len()
            })
    }
    // Complete arbitrary current control contracts; no native initialization.
    fn ai_control_carrier() -> player_ai_control::ControlManager {
        use crate::player_ai_control::*;
        fn contract(data: ContractData) -> Option<Contract> {
            Some(Contract {
                base: ContractBase {
                    status: -1,
                    player_id: i32::MIN,
                    priority: 7,
                    time_limit: -2,
                    time_estimate: 3,
                    id: 0x80000000,
                    owner: u32::MAX,
                    extra_present_value: i32::MIN,
                },
                data,
            })
        }
        ControlManager {
            player_id: -7100,
            priority: i32::MAX,
            controls: RecycledInstances {
                capacity: 8,
                instances: vec![Instance {
                    index: 2,
                    id: 0xffff0002,
                    priority: i32::MIN,
                    recruit_priority: 11,
                }],
                current_id_bits: 0xffff0000,
                first_free: 7,
                last_free: 3,
                free_indices: vec![7, 3],
            },
            has_out_of_turn_processing: true,
            running_as_human: false,
            contracts: vec![
                None,
                contract(ContractData::UnitRequest {
                    unit_hash: u32::MAX,
                    building_hash: 0x80000000,
                    district_hash: 1,
                    location: [-3, 4],
                    required_city: 0xffff0001,
                    keep_control: true,
                }),
                contract(ContractData::TechResearch {
                    technology_hashes: [u32::MAX, 1],
                    best_controller: 0x80000000,
                }),
                contract(ContractData::CivicResearch {
                    civic_hashes: [7, u32::MAX],
                }),
                contract(ContractData::PurchasePlot {
                    location: [i32::MIN, i32::MAX],
                }),
                contract(ContractData::DiplomaticAction {
                    player_id: -1,
                    action_hash: 0xdeadbeef,
                }),
            ],
        }
    }
    fn ai_influence_map_carrier() -> player_ai_influence_map::InfluenceMap {
        player_ai_influence_map::InfluenceMap {
            influence: vec![i16::MIN, -1, i16::MAX],
            religious_influence: vec![i16::MAX, i16::MIN],
        }
    }
    fn ai_planning_carrier() -> player_ai_identity::PlanningManagerIdentity {
        player_ai_identity::PlanningManagerIdentity {
            player_id: i32::MIN,
        }
    }
    fn after_post_heroes_arrays_bytes(p: &FirstPlayer) -> usize {
        p.ai_control
            .as_ref()
            .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_influence_map
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_planning
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_city_manager
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_culture
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + after_culture_ai_bytes(p)
    }
    // Common false-branch saved tail; all fields explicit, no runtime defaults.
    fn false_tail_carrier() -> player_tail::Tail {
        player_tail::Tail {
            alive: None,
            properties: unit::variants::TypedVariantMap {
                header: unit::variants::KeyedHeader::Count,
                entries: vec![],
            },
            improvement_replacements: vec![
                unit::HashValue {
                    definition_hash: u32::MAX,
                    value: 0x80000000,
                },
                unit::HashValue {
                    definition_hash: u32::MAX,
                    value: 7,
                },
            ],
            random_event_hashes: vec![u32::MAX, 0x80000000],
            terrain_work_impassable: vec![i32::MIN, -1],
            random_event_no_unit_damage: vec![i32::MAX],
            random_event_modified_damage_opposing_player: vec![-7],
        }
    }
    // Explicit complete saved fields; BoolByte values retain raw bytes.
    fn ai_city_manager_carrier() -> player_ai_city_manager::CityManager {
        use crate::player_ai_city_manager as pmod;
        pmod::CityManager {
            player_id: -8000,
            city_ais: pmod::RecycledCityBuilds {
                capacity: 16,
                entries: vec![pmod::CityBuildEntry {
                    index: 3,
                    city_build: pmod::CityBuild {
                        city: pmod::ComponentId {
                            owner: i16::MIN,
                            component_type: i16::MIN,
                            id: 3203334149,
                        },
                        city_id: 3203334148,
                        player_owner: -7995,
                        city_ai_id: 3203334150,
                        current_yields: vec![pmod::HashValue {
                            hash: 3203334151,
                            value: -7992,
                        }],
                        current_base_yields: vec![pmod::HashValue {
                            hash: 3203334152,
                            value: -7991,
                        }],
                        current_gpps: vec![pmod::HashValue {
                            hash: 3203334153,
                            value: -7990,
                        }],
                        best_improvement_hash: 3203334154,
                        city_threat: -7989,
                        under_attack: pmod::BoolByte(197),
                        desired_defenders: -7987,
                        defenders_needed: -7986,
                        reservations: vec![pmod::PlotReservation {
                            controller: 3203334159,
                            value: -7984,
                            tag: 3203334161,
                        }],
                        event_trees: vec![u32::MAX, 0x80000000],
                        managed_working_contract: pmod::ControlHandle {
                            player_id: -7983,
                            id: 3203334162,
                        },
                        managed_contracts: vec![pmod::ControlHandle {
                            player_id: -7982,
                            id: 3203334163,
                        }],
                        done_off_turn: pmod::BoolByte(197),
                        best_plot_to_clear: [i32::MIN, i32::MAX],
                        best_plot_to_buy: [i32::MIN, i32::MAX],
                        food_loss_amounts: vec![i32::MIN, -1, i32::MAX],
                        food_loss_turns: vec![i32::MIN, -1, i32::MAX],
                        food_loss_populations: vec![i32::MIN, -1, i32::MAX],
                        city_unit_efficiencies: vec![pmod::UnitEfficiency {
                            unit_hash: 3203334169,
                            max_damage: -7974,
                            is_obsolete: pmod::BoolByte(197),
                            is_ranged: pmod::BoolByte(197),
                            is_naval: pmod::BoolByte(197),
                            has_zone_of_control: pmod::BoolByte(197),
                            civ_unique: pmod::BoolByte(197),
                            exclude_from_efficiency_calculation: pmod::BoolByte(197),
                            can_build: pmod::BoolByte(197),
                            speed: -7966,
                            range: -7965,
                            main_combat_strength: -7964,
                            defense: -7963,
                            num_in_use: -7962,
                            efficiency_against_all_enemies: -7961,
                            efficiency_against_everyone: -7960,
                            efficiency_against_player: vec![i32::MIN, -1, i32::MAX],
                            raw_efficiency: -7958,
                            maintenance_resource_hash: 3203334187,
                            num_resource_needed: -7956,
                        }],
                        city_trade: pmod::TradeCity {
                            city: pmod::ComponentId {
                                owner: i16::MIN,
                                component_type: i16::MIN,
                                id: 3203334172,
                            },
                            current_routes: -7973,
                            current_units: -7972,
                            connected_cities: vec![pmod::TradeConnection {
                                city: pmod::ComponentId {
                                    owner: i16::MIN,
                                    component_type: i16::MIN,
                                    id: 3203334175,
                                },
                                rail_connection: pmod::BoolByte(197),
                                rail_value: -7969,
                            }],
                            known_failed_connections: vec![pmod::ComponentId {
                                owner: i16::MIN,
                                component_type: i16::MIN,
                                id: 3203334176,
                            }],
                        },
                        yield_preferences: vec![i32::MIN, -1, i32::MAX],
                        plot_evaluations: vec![pmod::PlotEvaluation {
                            index: 3,
                            plot: [i32::MIN, i32::MAX],
                            evaluate_wonders: pmod::BoolByte(197),
                            city: pmod::ComponentId {
                                owner: i16::MIN,
                                component_type: i16::MIN,
                                id: 3203334177,
                            },
                            resource_hash: 3203334176,
                            feature_hash: 3203334177,
                            location: [i32::MIN, i32::MAX],
                            neighbors: pmod::Neighbors([i16::MIN, -1, 0, 1, 2, i16::MAX]),
                            has_improvement: pmod::BoolByte(197),
                            has_district: pmod::BoolByte(197),
                            has_pillaged_improvement: pmod::BoolByte(197),
                            feature_removable: pmod::BoolByte(197),
                            can_build_improvement: pmod::BoolByte(197),
                            can_build_district: pmod::BoolByte(197),
                            can_place_building: pmod::BoolByte(197),
                            can_purchase: pmod::BoolByte(197),
                            neighbor_can_have_district: pmod::BoolByte(197),
                            revealed: pmod::BoolByte(197),
                            owned: pmod::BoolByte(197),
                            worked: pmod::BoolByte(197),
                        }],
                        evaluation_layers: pmod::EvaluationLayers(vec![pmod::EvaluationLayer {
                            buildings: vec![pmod::HashValue {
                                hash: 3203334173,
                                value: -7970,
                            }],
                            districts: vec![pmod::HashValue {
                                hash: 3203334174,
                                value: -7969,
                            }],
                            improvements: vec![pmod::HashValue {
                                hash: 3203334175,
                                value: -7968,
                            }],
                        }]),
                        unit_evaluations: vec![pmod::HashValue {
                            hash: 3203334174,
                            value: -7969,
                        }],
                        cached_unit_evaluations: vec![pmod::HashValue {
                            hash: 3203334175,
                            value: -7968,
                        }],
                        project_evaluations: vec![pmod::HashValue {
                            hash: 3203334176,
                            value: -7967,
                        }],
                        power_surplus: -7967,
                        default_build_present_value: -7966,
                        unimproved_resources: vec![pmod::HashValue {
                            hash: 3203334179,
                            value: -7964,
                        }],
                    },
                }],
                current_id_bits: 3203334147,
                first_free: 3203334148,
                last_free: 3203334149,
                free_indices: vec![u32::MAX, 0x80000000],
            },
        }
    }
    // Explicit complete saved fields; BoolByte values retain raw bytes.
    fn ai_culture_carrier() -> player_ai_culture::CultureAi {
        use crate::player_ai_culture as pmod;
        pmod::CultureAi {
            player_id: -8000,
            managed_contracts: vec![pmod::ControlHandle {
                player_id: -7999,
                id: 3203334146,
            }],
            boost_manager: pmod::BoostManager {
                player_id: -7998,
                allowed_active_boosts: 3203334147,
                active_boosts: vec![pmod::ActiveBoost {
                    behavior_tree_control_id: 3203334148,
                    boost_uniqueness_hash: -7995,
                }],
            },
            done_off_turn: pmod::BoolByte(197),
            congress_pseudo_yield_preferences: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -7996,
            }],
            pseudo_yield_preferences: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -7995,
            }],
            best_slot_policy_values: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -7994,
            }],
            civic_analyzers: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::CivicAnalysis {
                    player_id: -7993,
                    civic_hash: 3203334152,
                    next_civic_needed_hash: 3203334153,
                    is_owned: pmod::BoolByte(197),
                    is_favored: pmod::BoolByte(197),
                    has_boost: pmod::BoolByte(197),
                    is_repeatable: pmod::BoolByte(197),
                    solo_value_q8: -7986,
                    full_value_q8: -7985,
                    turns_to_unlock: -7984,
                },
            }],
            policy_analyzers: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::PolicyAnalysis {
                    player_id: -7992,
                    policy_hash: 3203334153,
                    slot_type_hash: 3203334154,
                    dark_age_policy: pmod::BoolByte(197),
                    golden_age_policy: pmod::BoolByte(197),
                    min_era_chronology: -7987,
                    max_era_chronology: -7986,
                    value_q8: -7985,
                },
            }],
            resolution_analyzers: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::ResolutionAnalysis {
                    player_id: -7991,
                    resolution_hash: 3203334154,
                    value_q8: -7989,
                    option_one_value_q8: -7988,
                    option_two_value_q8: -7987,
                    best_option: -7986,
                },
            }],
            govt_values: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -7990,
            }],
            rockband_analyzers: vec![pmod::RockBandAnalysis {
                player_id: -7989,
                base_value_q8: -7988,
                location: pmod::PlotCoord { x: -7987, y: -7986 },
                district_hash: 3203334158,
                improvement_hash: 3203334159,
                national_park: pmod::BoolByte(197),
            }],
            great_work_manager: pmod::GreatWorks {
                player_id: -7988,
                great_work_values_q8: vec![i32::MIN, -1, i32::MAX],
                working_great_work_values_q8: vec![i32::MIN, -1, i32::MAX],
            },
            national_park_spots: vec![i32::MIN, -1, i32::MAX],
            num_naturalists: -7986,
            must_change_policies: pmod::BoolByte(197),
        }
    }
    // Explicit complete current saved wire; raw byte fields are not bools.
    fn ai_diplomatic_carrier() -> player_ai_diplomatic::Diplomatic {
        use crate::player_ai_diplomatic as pmod;
        pmod::Diplomatic {
            player_id: -9000,
            min_force_to_notice_q8: -8999,
            min_troubling_force_q8: -8998,
            diplomatic_scores_q8: vec![i32::MIN, -1, i32::MAX],
            base_diplo_scores_q8: vec![i32::MIN, -1, i32::MAX],
            visible_scores_q8: vec![vec![i32::MIN, -1, i32::MAX]],
            diplomatic_states: pmod::StateVector {
                states: vec![pmod::DiplomaticState {
                    owner: -8994,
                    opponent: -8993,
                    state_name: vec![0, 255, 128],
                    is_updating: pmod::BoolByte(197),
                    is_active: pmod::BoolByte(197),
                    has_changed_state: pmod::BoolByte(197),
                    diplomatic_bonus_q8: -8988,
                    state_hash: 3735879693,
                    allowed_action_hashes: vec![u32::MAX, 0x80000000],
                    response_action_hashes: vec![u32::MAX, 0x80000000],
                    desired_action_hashes: vec![u32::MAX, 0x80000000],
                    transitions: vec![pmod::DiplomaticTransition {
                        player_diplomacy_player: -8983,
                        opponent_player: -8982,
                        current_score_q8: -8981,
                        transition_index: -8980,
                        next_state: vec![0, 255, 128],
                        next_state_index: -8978,
                        has_required_min: pmod::BoolByte(197),
                        required_min: -8976,
                        has_required_max: pmod::BoolByte(197),
                        required_max: -8974,
                        has_allowed_min: pmod::BoolByte(197),
                        allowed_min: -8972,
                        has_allowed_max: pmod::BoolByte(197),
                        allowed_max: -8970,
                        must_transition: pmod::BoolByte(197),
                        transition_score_q8: -8968,
                        throttle_turns: -8967,
                        condition_hash: 3735879714,
                        transition_hash: 3735879715,
                        transition_action_hashes: vec![u32::MAX, 0x80000000],
                        actions_gained_hashes: vec![u32::MAX, 0x80000000],
                        actions_lost_hashes: vec![u32::MAX, 0x80000000],
                    }],
                    triggers: vec![pmod::TriggeredTransition {
                        next_state_name: vec![0, 255, 128],
                        next_state_index: -8981,
                        trigger: -8980,
                    }],
                }],
                is_updated: pmod::BoolByte(197),
            },
            diplo_action_evaluator: pmod::ActionEvalVector {
                handlers: vec![pmod::DiplomaticActionHandler {
                    owner: 2,
                    opponent: 3,
                    desirability: Some(vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }]),
                    cost: Some(vec![pmod::HashEntry {
                        hash: 7,
                        value: i32::MAX,
                    }]),
                    alliance_value_q8: -1,
                    war_value_q8: i32::MIN,
                    war_value_penalties_q8: i32::MAX,
                    is_initialized: pmod::BoolByte(197),
                }],
                is_updated: pmod::BoolByte(197),
            },
            num_majors_met: -8992,
            player_threats_q8: vec![i32::MIN, -1, i32::MAX],
            player_major_threats: vec![pmod::BoolByte(197)],
            player_trusts_q8: vec![i32::MIN, -1, i32::MAX],
            managed_contracts: vec![pmod::ControlHandle {
                player_id: -8988,
                id: 3735879693,
            }],
            best_potential_friend: -8987,
            partner_action_preference: vec![vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8986,
            }]],
            partner_action_worth_q8: vec![vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8985,
            }]],
            closed_partner_action: vec![vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::BoolByte(197),
            }]],
            requested_items: vec![pmod::TradeItem {
                game_turn: -8983,
                other_player: -8982,
                item_type: 3735879699,
                item_sub_type: 3735879700,
                item_value: 3735879701,
            }],
            excluded_items: vec![pmod::TradeItem {
                game_turn: -8982,
                other_player: -8981,
                item_type: 3735879700,
                item_sub_type: 3735879701,
                item_value: 3735879702,
            }],
            favor_traded_this_turn: -8981,
            done_off_turn: pmod::BoolByte(197),
        }
    }
    // Explicit complete current saved wire; raw byte fields are not bools.
    fn ai_economic_carrier() -> player_ai_economic::Economic {
        use crate::player_ai_economic as pmod;
        pmod::Economic {
            player_id: -9000,
            current_balance_q8: -8999,
            city_income_q8: -8998,
            religious_income_q8: -8997,
            diplomatic_income_q8: -8996,
            percentage_bonuses_income_q8: -8995,
            city_expenses_q8: -8994,
            turns_with_negative_income: -8993,
            total_saving_weight: -8992,
            saving_weights: vec![i32::MIN, -1, i32::MAX],
            savings_q8: vec![i32::MIN, -1, i32::MAX],
            yield_priority_boost_q8: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8989,
            }],
            managed_contracts: vec![pmod::ControlHandle {
                player_id: -8988,
                id: 3735879693,
            }],
            trade_manager: pmod::TradePlayer {
                player_id: -8987,
                control_id: 3735879694,
                best_tech_driven_road_hash: 3735879695,
                turns_since_railroad_tree: -8984,
            },
        }
    }
    // Explicit complete current saved wire; raw byte fields are not bools.
    fn ai_governors_carrier() -> player_ai_governors::Governors {
        use crate::player_ai_governors as pmod;
        pmod::Governors {
            player_id: -9000,
            done_off_turn: 197,
            governor_analyses: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::GovernorAnalysis {
                    player_id: -8998,
                    governor_hash: 3735879683,
                    city_ids: vec![pmod::CityId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: 3735879686,
                    }],
                    values_q8: vec![i32::MIN, -1, i32::MAX],
                },
            }],
            promotion_analyses: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: pmod::GovernorPromotionAnalysis {
                    player_id: -8997,
                    promotion_hash: 3735879684,
                    for_governor_hashes: vec![u32::MAX, 0x80000000],
                    value_q8: -8994,
                },
            }],
        }
    }
    // Explicit complete current saved wire; raw byte fields are not bools.
    fn ai_grand_strategic_carrier() -> player_ai_grand_strategic::GrandStrategic {
        use crate::player_ai_grand_strategic as pmod;
        pmod::GrandStrategic {
            player_id: -9000,
            city_planner: pmod::CityPlanner {
                player_id: -8999,
                expansion_score: -8998,
                turns_since_city_founding: -8997,
            },
            primary_regions: vec![u32::MAX, 0x80000000],
            expansion_regions: vec![u32::MAX, 0x80000000],
            tertiary_regions: vec![u32::MAX, 0x80000000],
            settle_cities: vec![pmod::RegionEvaluator {
                player_id: -8995,
                region_id: 3735879686,
                evaluations: vec![pmod::CitySettleSiteEvaluation {
                    x: -8993,
                    y: -8992,
                    score_q8: -8991,
                }],
                intermediate_data: pmod::PlotEvaluation {
                    x: -8992,
                    y: -8991,
                    my_nearest_city: -8990,
                    nearest_enemy_city: -8989,
                    danger: -8988,
                    cultural_pressure: -8987,
                    is_coastal_raw: 197,
                    has_fresh_water_raw: 197,
                    is_valid_raw: 197,
                    new_resources_in_inner_ring: -8983,
                    plot_score_q8: -8982,
                    inner_ring_yield: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8981,
                    }],
                    full_city_yield: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8980,
                    }],
                    num_terrains: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8979,
                    }],
                    num_resources_inner_ring: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8978,
                    }],
                    num_resources: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8977,
                    }],
                    num_features: vec![pmod::HashEntry {
                        hash: u32::MAX,
                        value: -8976,
                    }],
                },
                has_positive_eval_raw: 197,
            }],
            done_island_check_raw: 197,
            is_on_island_raw: 197,
            commemorate_era_raw: 197,
            victory_manager: pmod::VictoryStrategy {
                player_id: -8991,
                pursuing_strategies: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: 197,
                }],
                forbidden_strategies: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: 197,
                }],
                must_change_state: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: 197,
                }],
                meets_exclusive_condition: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: 197,
                }],
                next_turn_change_allowed: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: -8986,
                }],
                yield_priorities_q8: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: -8985,
                }],
                pseudo_yield_priorities_saved_as_yield_hash_q8: vec![pmod::HashEntry {
                    hash: u32::MAX,
                    value: -8984,
                }],
            },
            pseudo_yield_priority_boost_q8: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8990,
            }],
            pseudo_yield_adjustments_q8: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8989,
            }],
            unit_production: pmod::UnitProduction {
                player_id: -8988,
                offense_units: vec![pmod::UnitEfficiencyInfo {
                    unit_hash: 3735879693,
                    max_damage: -8986,
                    is_obsolete_raw: 197,
                    is_ranged_raw: 197,
                    is_naval_raw: 197,
                    has_zone_of_control_raw: 197,
                    civ_unique_raw: 197,
                    exclude_from_efficiency_calculation_raw: 197,
                    can_build_raw: 197,
                    speed: -8978,
                    range: -8977,
                    main_combat_strength: -8976,
                    defense: -8975,
                    num_in_use: -8974,
                    efficiency_against_all_enemies_q8: -8973,
                    efficiency_against_everyone_q8: -8972,
                    efficiency_against_player_q8: vec![i32::MIN, -1, i32::MAX],
                    raw_efficiency_q8: -8970,
                    maintenance_resource_hash: 3735879711,
                    num_resource_needed: -8968,
                }],
                has_targets_raw: 197,
            },
            yield_sensitivity_boost_q8: vec![pmod::HashEntry {
                hash: u32::MAX,
                value: -8987,
            }],
        }
    }
    // Explicit complete current saved wire; raw byte fields are not bools.
    fn ai_homeland_carrier() -> player_ai_homeland::Homeland {
        use crate::player_ai_homeland as pmod;
        pmod::Homeland {
            player_id: -9000,
            control_instance_id: 3735879681,
            additional_priority: -8998,
            prioritized_moves: pmod::MoveList {
                base_moves: vec![i32::MIN, -1, i32::MAX],
                moves: vec![pmod::Move {
                    allowed_move_hash: 3735879684,
                    priority: -8995,
                }],
            },
        }
    }
    fn after_culture_ai_bytes(p: &FirstPlayer) -> usize {
        p.ai_diplomatic
            .as_ref()
            .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_economic
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_governors
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_grand_strategic
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
            + p.ai_homeland
                .as_ref()
                .map_or(0, |v| v.encode(Limits::default()).unwrap().len())
    }
    // Arbitrary source-only wire values; Default below means an empty saved
    // test carrier, never a claim about a native constructor or Initialize.
    fn complete_source_player(
        index: u32,
        was: bool,
        arrays: bool,
    ) -> (
        FirstPlayer,
        Option<Arc<initialization::InitializationSnapshot>>,
        initialization::WireBinding,
    ) {
        let (prefix, _) = Prefix::decode_prefix(&carrier(), Limits::default()).unwrap();
        let mut p = prefix.first_player.unwrap();
        p.requested_owner = index as i32;
        p.header.owner = index as i32;
        let identity = initialization::PlayerIdentity {
            manager_index: index,
            requested_owner: p.requested_owner,
            record_owner: p.header.owner,
        };
        let (snapshot, binding, _) = initialization::source_case(identity, arrays, !arrays);
        if !was {
            p.header.ordered_flags[1] = false;
            p.post_heroes_arrays = None;
            p.ai_control = None;
            p.ai_influence_map = None;
            p.ai_planning = None;
            p.ai_city_manager = None;
            p.ai_culture = None;
            p.ai_diplomatic = None;
            p.ai_economic = None;
            p.ai_governors = None;
            p.ai_grand_strategic = None;
            p.ai_homeland = None;
            p.tail = Some(false_tail_carrier());
            return (p, None, binding);
        }
        let snapshot = Arc::new(snapshot);
        p.initialization_snapshot = Some(Arc::clone(&snapshot));
        p.ai_military = Some(player_ai_military::Military {
            player_id: index as i32,
            on_war_footing: player_ai_military::BoolByte(197),
            operation_types: arrays.then(|| player_ai_military::OperationTypes {
                allowed: vec![player_ai_military::HashValue {
                    hash: u32::MAX,
                    value: -7,
                }],
                per_war: vec![],
                current: vec![],
            }),
            ..player_ai_military::Military::default()
        });
        p.ai_research = Some(player_ai_research::Research {
            player_id: -9,
            research_target_hash: u32::MAX,
            turns_to_research_unit: vec![],
            favored_techs: vec![],
            tree_analysis: vec![],
            done_off_turn: player_ai_research::BoolByte(197),
            managed_contracts: vec![],
            boost_manager: player_ai_culture::BoostManager {
                player_id: -8,
                allowed_active_boosts: 3,
                active_boosts: vec![],
            },
            turns_remaining_on_current_research: -1,
        });
        p.ai_tactical = Some(player_ai_tactical::Tactical {
            player_id: -7,
            control_instance_id: u32::MAX,
            prioritized_moves: player_ai_homeland::MoveList {
                base_moves: vec![-1],
                moves: vec![],
            },
            zombie_type: i32::MIN,
        });
        use player_ai_targets as t;
        let id = || t::ComponentId {
            owner: -1,
            component_type: -2,
            id: u32::MAX,
        };
        p.ai_targets = Some(t::Targets {
            player_id: -6,
            desired_targets: vec![],
            new_war_started: t::BoolByte(197),
            knowledge_base: t::CivKnowledge {
                owner: -5,
                known_cities: vec![],
                barbarians: vec![],
                player_knowledge: vec![],
                total_hostile_armies_of_type: vec![],
                war_knowledge: vec![],
                player_armies: t::Armies(std::array::from_fn(|_| t::ArmyKnowledge {
                    army_size: t::Conditional {
                        present: t::BoolByte(0),
                        value: None,
                    },
                })),
                trade_knowledge: t::TradeKnowledge {
                    owner: -4,
                    from_city: id(),
                    to_city: id(),
                    origin_yield_values_q8: vec![-1],
                    destination_yield_values_q8: vec![],
                    best_yield_destination: -3,
                    best_yield_origin: -2,
                    total_value_q8: i32::MIN,
                },
                yield_average_q8: vec![],
                yield_error_q8: vec![],
            },
            operation_start_points: vec![],
            release_turn: -1,
            map_area_visibility: vec![],
            map_visibility: vec![],
            targets: t::TargetList { targets: vec![] },
        });
        p.ai_behavior_tree = Some(player_ai_behavior_tree::BehaviorTreeManager {
            player_id: -3,
            blackboard: player_ai_behavior_tree::Blackboard {
                job_index: -1,
                ..player_ai_behavior_tree::Blackboard::default()
            },
            behavior_trees: player_ai_behavior_tree::RecycledBehaviorTrees {
                first_free: u32::MAX,
                last_free: u32::MAX,
                ..player_ai_behavior_tree::RecycledBehaviorTrees::default()
            },
        });
        let mut tail = false_tail_carrier();
        tail.alive = Some(player_tail::AliveTail {
            agendas: vec![u32::MAX],
            ..player_tail::AliveTail::default()
        });
        p.tail = Some(tail);
        (p, Some(snapshot), binding)
    }
    fn source_manager(
        branches: &[(bool, bool)],
    ) -> (Prefix, Vec<Arc<initialization::InitializationSnapshot>>) {
        let (mut value, _) = Prefix::decode_prefix(&carrier(), Limits::default()).unwrap();
        value.first_player = None;
        value.additional_players.clear();
        value.player_count = branches.len() as u32;
        if branches.is_empty() {
            value.religion.records.clear();
        }
        value.unfinished_player_entries = 0;
        value.boundary = if branches.is_empty() {
            Boundary::AfterEmptyPlayerManager
        } else {
            Boundary::AfterPlayerManager
        };
        let mut snapshots = vec![];
        for (index, &(was, arrays)) in branches.iter().enumerate() {
            let (p, snapshot, binding) = complete_source_player(index as u32, was, arrays);
            value.initialization_binding = Some(binding);
            if let Some(snapshot) = snapshot {
                snapshots.push(snapshot);
            }
            if index == 0 {
                value.first_player = Some(p);
            } else {
                value.additional_players.push(p);
            }
        }
        (value, snapshots)
    }
    #[test]
    fn conditional_source_manager_mixed_roster_exact_caps_entries_and_suffix() {
        for branches in [
            vec![],
            vec![(false, false)],
            vec![(true, true), (false, false), (true, false)],
        ] {
            let (value, snapshots) = source_manager(&branches);
            let fallback = initialization::source_case(
                initialization::PlayerIdentity {
                    manager_index: 0,
                    requested_owner: 0,
                    record_owner: 0,
                },
                false,
                false,
            )
            .1;
            let binding = value.initialization_binding.clone().unwrap_or(fallback);
            let provider = |identity: initialization::PlayerIdentity| {
                snapshots
                    .iter()
                    .find(|s| s.facts().owner == identity.record_owner)
                    .map(Arc::clone)
            };
            let (bytes, items) = value.encode_prefix_with_items(Limits::default()).unwrap();
            let exact = Limits {
                max_bytes: bytes.len(),
                max_items: items,
            };
            let mut input = bytes.clone();
            input.extend([91, 92, 93]);
            let (decoded, used, charged) = Prefix::decode_player_manager_conditional(
                &input,
                exact,
                binding.clone(),
                &provider,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "branches {branches:?}, bytes {}, items {items}: {e:?}",
                    bytes.len()
                )
            });
            assert_eq!((used, charged), (bytes.len(), items));
            assert_eq!(&input[used..], &[91, 92, 93]);
            assert!(decoded.complete_player_manager_wire());
            assert!(!decoded.initialization_reconstructed());
            assert_eq!(decoded.players().count(), branches.len());
            assert_eq!(
                decoded.encode_prefix_with_items(exact).unwrap(),
                (bytes.clone(), items)
            );
            let mut entry_total = 0;
            for index in 0..branches.len() {
                let (entry, _) = decoded
                    .encode_player_entry_prefix_with_items(index as u32, exact)
                    .unwrap();
                assert_eq!(&entry[..4], &(index as i32).to_le_bytes());
                entry_total += entry.len();
            }
            assert!(entry_total <= bytes.len());
            let mut shorts = vec![Limits {
                max_bytes: bytes.len() - 1,
                max_items: items,
            }];
            if items > 0 {
                shorts.push(Limits {
                    max_bytes: bytes.len(),
                    max_items: items - 1,
                });
            }
            for short in shorts {
                assert!(Prefix::decode_player_manager_conditional(
                    &bytes,
                    short,
                    binding.clone(),
                    &provider
                )
                .is_err());
                assert!(decoded.encode_prefix_with_items(short).is_err());
            }
            if !snapshots.is_empty() {
                assert_eq!(
                    decoded.initialization_basis(),
                    InitializationBasis::SourceOnlySnapshotInputs
                );
                let (standard, standard_used) = Prefix::decode_prefix(&bytes, exact).unwrap();
                assert_eq!(standard.boundary, Boundary::AIMilitary);
                assert!(standard_used < bytes.len());
                let missing = |_| None;
                assert!(matches!(
                    Prefix::decode_player_manager_conditional(
                        &bytes,
                        exact,
                        binding.clone(),
                        &missing
                    ),
                    Err(Error::MissingInitializationSnapshot(_))
                ));
                let mut wrong = binding.clone();
                wrong.core_sha256 = [99; 32];
                assert!(matches!(
                    Prefix::decode_player_manager_conditional(&bytes, exact, wrong, &provider),
                    Err(Error::Initialization(initialization::Error::InputBinding))
                ));
                // Every cut of the actual continuation and remaining entries must fail.
                for end in standard_used..bytes.len() {
                    assert!(
                        Prefix::decode_player_manager_conditional(
                            &bytes[..end],
                            exact,
                            binding.clone(),
                            &provider
                        )
                        .is_err(),
                        "cut {end}"
                    );
                }
            }
        }
    }
    #[test]
    fn conditional_source_43_entry_roster_uses_each_snapshot_once_per_parse() {
        let branches: Vec<_> = (0..43).map(|i| (i % 3 != 1, i % 2 == 0)).collect();
        let (value, snapshots) = source_manager(&branches);
        let binding = value.initialization_binding.clone().unwrap();
        let calls = std::cell::RefCell::new(Vec::new());
        let provider = |identity: initialization::PlayerIdentity| {
            calls.borrow_mut().push(identity);
            snapshots
                .iter()
                .find(|s| s.facts().owner == identity.record_owner)
                .map(Arc::clone)
        };
        let (bytes, items) = value.encode_prefix_with_items(Limits::default()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        let (decoded, n, i) =
            Prefix::decode_player_manager_conditional(&bytes, exact, binding.clone(), &provider)
                .unwrap();
        assert_eq!((n, i), (bytes.len(), items));
        assert_eq!(decoded.additional_players.len(), 42);
        assert!(decoded.complete_player_manager_wire());
        assert_eq!(calls.borrow().len(), snapshots.len());
        assert_eq!(
            decoded.encode_prefix_with_items(exact).unwrap(),
            (bytes.clone(), items)
        );
        for short in [
            Limits {
                max_bytes: bytes.len() - 1,
                max_items: items,
            },
            Limits {
                max_bytes: bytes.len(),
                max_items: items - 1,
            },
        ] {
            assert!(Prefix::decode_player_manager_conditional(
                &bytes,
                short,
                binding.clone(),
                &provider
            )
            .is_err());
            assert!(decoded.encode_prefix_with_items(short).is_err());
        }
        // Reusing immutable metadata after a failed parse remains valid.
        assert!(
            Prefix::decode_player_manager_conditional(&bytes, exact, binding, &provider)
                .unwrap()
                .0
                .complete_player_manager_wire()
        );
    }
    #[test]
    fn malformed_advertised_roster_does_not_reserve_unparsed_records() {
        let (mut prefix, _) = source_manager(&[(false, false)]);
        prefix.player_count = 600_001;
        prefix.unfinished_player_entries = 600_000;
        prefix.boundary = Boundary::NextPlayerEntry;
        let binding = prefix.initialization_binding.clone().unwrap();
        let (mut bytes, _) = prefix.encode_prefix_with_items(Limits::default()).unwrap();
        // Enough source-minimum wire for the advertised count; the next
        // requested ID is followed by version0, so it must fail before reserve.
        bytes.resize(bytes.len() + 600_000 * 102, 0);
        let limits = Limits {
            max_bytes: bytes.len(),
            max_items: 1_000_000,
        };
        let provider = |_| None;
        let mut requests = Vec::new();
        let result = Prefix::decode_player_manager_conditional_reserving(
            &bytes,
            limits,
            binding,
            &provider,
            0,
            |_, additional| {
                requests.push(additional);
                // Safe allocator seam: no request reaches the real allocator.
                Err(Error::ResourceLimit)
            },
        );
        assert!(matches!(
            result,
            Err(Error::Version {
                expected: 47,
                actual: 0
            })
        ));
        assert!(requests.is_empty());
    }
    #[test]
    fn conditional_model_rejects_wrong_owner_context_and_branch_before_output() {
        let (mut value, _) = source_manager(&[(true, true), (false, false)]);
        value
            .first_player
            .as_mut()
            .unwrap()
            .ai_military
            .as_mut()
            .unwrap()
            .operation_types = None;
        assert_eq!(
            value.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::MilitaryContextMismatch)
        );
        let (mut value, _) = source_manager(&[(true, true)]);
        value
            .first_player
            .as_mut()
            .unwrap()
            .ai_military
            .as_mut()
            .unwrap()
            .player_id = -99;
        assert!(matches!(
            value.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::Initialization(initialization::Error::OwnerBinding))
        ));
        let (mut value, _) = source_manager(&[(true, true)]);
        value.first_player.as_mut().unwrap().ai_targets = None;
        assert_eq!(
            value.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
    }
    fn carrier() -> Vec<u8> {
        let mut out = Vec::new();
        words(&mut out, &[7, 101, 102, 1, 7, 0xfeed1234, 4, 5, 6, 7, 3]);
        out.extend_from_slice(b"\xff\0x");
        words(
            &mut out,
            &[2, 0xffffffff, 0x80000000, 0, 1, 0x12345678, 0x80000000],
        );
        out.extend_from_slice(&[1, 0, 1]);
        words(&mut out, &[0x7fffffff]);
        words(&mut out, &[1, 2, 64, 47, 64, 1, 2]);
        words(
            &mut out,
            &[
                0x80000000, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0xffffffff,
            ],
        );
        out.extend_from_slice(&[0, 1, 0, 1, 0, 1]);
        words(&mut out, &[0xdeadbeef, 7, 8, 9, 1]);
        empty_container(&mut out);
        // Goody, Bonuses, Congress, all complete with empty dynamic vectors.
        words(
            &mut out,
            &[
                1, 0, 4, 0, 12, 64, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 6, 7, 8, 9, 0,
            ],
        );
        out.push(1);
        words(&mut out, &[10, 15]);
        empty_container(&mut out);
        out.push(1);
        words(
            &mut out,
            &[0x80000000, 0xdeadbeef, 2, 0x12345678, 0xffffffff, 9, 10, 11],
        );
        words(&mut out, &[1, 123, 0, 1, 456, 0, 2, 111, 222, 111, 333]);
        out.extend(diplomacy_carrier().encode(Limits::default()).unwrap());
        // Districts8, empty actual instance roster with nonempty allocator tail.
        words(&mut out, &[8]);
        empty_container(&mut out);
        words(&mut out, &[0x80000000, 99]);
        // Five indexed-int arrays; first contains duplicate hash pairs.
        words(&mut out, &[2, 0xdeadbeef, 7, 0xdeadbeef, 8, 0, 0, 0, 0]);
        // CreationYield1 with raw Yield index and byte bool fields.
        words(&mut out, &[1, 1, 0x12345678, 0xffffffff, 0x80000000]);
        out.extend([1, 0]);
        words(&mut out, &[0xfedcba98]);
        // Remaining creation/unit/tech/civic collections are empty.
        words(&mut out, &[0, 0, 0, 0]);
        // NameManager2 has equal explicit signed and array lengths, then i16s.
        words(&mut out, &[2, 3, 3]);
        for v in [i16::MIN, -1, i16::MAX] {
            out.extend(v.to_le_bytes());
        }
        out.extend(culture_carrier().encode(Limits::default()).unwrap());
        out.extend(espionage_carrier().encode(Limits::default()).unwrap());
        out.extend(religion_carrier().encode(Limits::default()).unwrap());
        out.extend(resources_carrier().encode(Limits::default()).unwrap());
        out.extend(stats_carrier().encode(Limits::default()).unwrap());
        out.extend(techs_carrier().encode(Limits::default()).unwrap());
        out.extend(treasury_carrier().encode(Limits::default()).unwrap());
        out.extend(units_carrier().encode(Limits::default()).unwrap());
        words(&mut out, &[2, 0x80000000, u32::MAX]);
        out.extend(trade_carrier().encode(Limits::default()).unwrap());
        out.extend(
            great_people_points_carrier()
                .encode(Limits::default())
                .unwrap(),
        );
        out.extend(influence_carrier().encode(Limits::default()).unwrap());
        out.extend(quests_carrier().encode(Limits::default()).unwrap());
        out.extend(wmds_carrier().encode(Limits::default()).unwrap());
        out.extend(improvements_carrier().encode(Limits::default()).unwrap());
        out.extend(governors_carrier().encode(Limits::default()).unwrap());
        out.extend(
            cultural_identity_carrier()
                .encode(Limits::default())
                .unwrap(),
        );
        out.extend(heroes_carrier().encode(Limits::default()).unwrap());
        let arrays = post_heroes_arrays_carrier();
        for table in [
            &arrays.unit_definitions,
            &arrays.feature_definitions,
            &arrays.continent_definitions,
        ] {
            out.extend(table.encode(Limits::default()).unwrap());
        }
        out.extend(ai_control_carrier().encode(Limits::default()).unwrap());
        out.extend(
            ai_influence_map_carrier()
                .encode(Limits::default())
                .unwrap(),
        );
        out.extend(ai_planning_carrier().encode(Limits::default()).unwrap());
        out.extend(ai_city_manager_carrier().encode(Limits::default()).unwrap());
        out.extend(ai_culture_carrier().encode(Limits::default()).unwrap());
        out.extend(ai_diplomatic_carrier().encode(Limits::default()).unwrap());
        out.extend(ai_economic_carrier().encode(Limits::default()).unwrap());
        out.extend(ai_governors_carrier().encode(Limits::default()).unwrap());
        out.extend(
            ai_grand_strategic_carrier()
                .encode(Limits::default())
                .unwrap(),
        );
        out.extend(ai_homeland_carrier().encode(Limits::default()).unwrap());

        out
    }
    #[test]
    fn typed_empty_rosters_nested_components_and_aggregate_budgets() {
        let bytes = carrier();
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b"untouched AIMilitary");
        let (value, n, items) =
            Prefix::decode_prefix_with_items(&trailing, Limits::default()).unwrap();
        assert_eq!(n, bytes.len());
        assert_eq!(value.boundary, Boundary::AIMilitary);
        assert_eq!(value.unfinished_player_entries, 2);
        assert_eq!(value.religion.records[0].custom_name, b"\xff\0x");
        assert_eq!(
            value.encode_prefix_with_items(Limits::default()).unwrap(),
            (bytes.clone(), items)
        );
        let exact = Limits {
            max_bytes: n,
            max_items: items,
        };
        assert_eq!(
            value.encode_prefix_with_items(exact).unwrap(),
            (bytes.clone(), items)
        );
        assert!(value
            .encode_prefix(Limits {
                max_bytes: n - 1,
                ..exact
            })
            .is_err());
        assert!(matches!(
            value.encode_prefix(Limits {
                max_items: items - 1,
                ..exact
            }),
            Err(Error::AIHomeland(player_ai_homeland::Error::ResourceLimit))
        ));
        let p = value.first_player.unwrap();
        assert_eq!(p.requested_owner, 64);
        assert_eq!(p.header.owner, 64);
        assert_eq!(p.header.ordered_scalars[0], i32::MIN);
        assert_eq!(p.formations.free_links, [7, 3]);
        assert!(p.cities.records.entries.is_empty());
        assert_eq!(p.cities.records.free_links, [7, 3]);
        assert_eq!(
            p.diplomacy.promises_made[0].1.entries[0].1.turn_made,
            i32::MIN
        );
        assert_eq!(
            p.diplomacy.incursions_made[0].1.entries[0].1,
            [i32::MIN, 1, i32::MAX]
        );
        assert!(p.districts.records.entries.is_empty());
        assert_eq!(p.districts.records.free_links, [7, 3]);
        assert_eq!(p.districts.current_district_cost.len(), 2);
        assert_eq!(p.districts.creation_yields[0].yield_type, -1);
        assert!(!p.header.saved_is_alive());
        assert_eq!(
            p.name_manager.citizen_name_usage_counts,
            [i16::MIN, -1, i16::MAX]
        );
        assert_eq!(p.culture.tourism_locations[0].tourist_counts[1].count, 12);
        assert_eq!(p.culture.government_type_modified_slots[0].delta, i32::MIN);
        assert!(!p.stats.diplomatic_victory_points.include_steps);
        assert_eq!(
            p.stats.diplomatic_victory_points.steps[0].value.steps[0]
                .value
                .min_q8,
            Some(i32::MIN)
        );
        assert!(p.techs.has_tech.pairs[0].value);
        assert!(!p.techs.boost_triggered.pairs[1].value);
        assert!(p.units.records.entries.is_empty());
        assert_eq!(p.units.records.free_links, [99, 0xffff0001]);
        assert_eq!(p.units.proposed_promotions.len(), 1);
        assert_eq!(p.units.unit_fuel_demands[0].value, i32::MIN);
        assert_eq!(p.treasury.gold_diplomatic_deals.len(), 1);
        assert_eq!(p.eras.era_hash, 0x80000000);
        assert_eq!(p.eras.game_era_hash, u32::MAX);
        assert_eq!(p.trade.player, -3000);
        assert_eq!(p.trade.flat_yield_bonus_per_player[63], [(63, 31)]);
        assert!(p.great_people_points.great_person_boost_list[0].tech_boost);
        assert_eq!(
            p.influence.levy_discount_percent_by_player,
            [i32::MIN, -1, i32::MAX]
        );
        assert_eq!(p.quests.player, -4000);
        assert_eq!(p.wmds.weapons.len(), 1);
        assert_eq!(p.improvements.plot_indices, [i32::MIN, -1, i32::MAX]);
        assert_ne!(
            p.improvements.improvement_array_size as usize,
            p.improvements.plot_indices.len()
        );
        assert_eq!(p.governors.entries[0].index, 2);
        assert_eq!(p.governors.entries[0].governor.name, [0, 255, 128]);
        assert_eq!(p.governors.free_links, [9, 9, u32::MAX]);
        assert!(p.header.saved_was_ever_alive());
        assert_eq!(p.heroes.claims.len(), 1);
        assert_eq!(p.ai_control.as_ref().unwrap().contracts.len(), 6);
        assert_eq!(
            p.ai_influence_map.as_ref().unwrap().influence,
            [i16::MIN, -1, i16::MAX]
        );
        assert_eq!(p.ai_planning.as_ref().unwrap().player_id, i32::MIN);
        assert_eq!(
            p.ai_city_manager.as_ref().unwrap().city_ais.entries[0]
                .city_build
                .under_attack
                .0,
            197
        );
        assert_eq!(
            p.ai_city_manager.as_ref().unwrap().city_ais.entries[0]
                .city_build
                .plot_evaluations[0]
                .owned
                .0,
            197
        );
        assert_eq!(p.ai_culture.as_ref().unwrap().done_off_turn.0, 197);
        assert_eq!(p.ai_diplomatic.as_ref().unwrap().done_off_turn.0, 197);
        assert_eq!(p.ai_governors.as_ref().unwrap().done_off_turn, 197);
        assert_eq!(
            p.ai_grand_strategic.as_ref().unwrap().done_island_check_raw,
            197
        );
        assert_eq!(
            p.ai_homeland
                .as_ref()
                .unwrap()
                .prioritized_moves
                .moves
                .len(),
            1
        );
        assert_eq!(
            p.ai_culture.as_ref().unwrap().civic_analyzers[0]
                .value
                .is_owned
                .0,
            197
        );
        assert_eq!(
            p.post_heroes_arrays
                .as_ref()
                .unwrap()
                .unit_definitions
                .pairs[0]
                .definition_hash,
            u32::MAX
        );
        assert!(
            !p.post_heroes_arrays
                .as_ref()
                .unwrap()
                .unit_definitions
                .pairs[1]
                .value
        );
        assert_eq!(p.cultural_identity.extra_capital_pressure_radius, i32::MIN);
        assert_eq!(
            p.cultural_identity
                .foreign_identity_per_turn_from_nearby_great_works_raw,
            -5991
        );
        assert_eq!(
            p.cultural_identity
                .encode_with_items(Limits {
                    max_bytes: 60,
                    max_items: 0
                })
                .unwrap()
                .1,
            0
        );
        assert_eq!(
            p.quests
                .encode_with_items(Limits {
                    max_bytes: 8,
                    max_items: 0
                })
                .unwrap()
                .1,
            0
        );
        assert_eq!(
            p.trade.flat_origin_yield_per_terrain_for_domestic[0].1,
            [(9, -257), (9, 513)]
        );
        assert_eq!(
            p.eras
                .encode_with_items(Limits {
                    max_bytes: 12,
                    max_items: 0
                })
                .unwrap()
                .1,
            0
        );
        assert_eq!(p.cities.building_build_overrides, [[111, 222], [111, 333]]);
        let exact = Limits {
            max_bytes: n,
            max_items: items,
        };
        assert!(Prefix::decode_prefix_with_items(&trailing, exact).is_ok());
        assert!(Prefix::decode_prefix_with_items(
            &trailing,
            Limits {
                max_bytes: n - 1,
                ..exact
            }
        )
        .is_err());
        assert!(matches!(
            Prefix::decode_prefix_with_items(
                &trailing,
                Limits {
                    max_items: items - 1,
                    ..exact
                }
            ),
            Err(Error::AIHomeland(player_ai_homeland::Error::ResourceLimit))
        ));
        for end in 0..n {
            assert!(
                Prefix::decode_prefix(&bytes[..end], Limits::default()).is_err(),
                "end={end}"
            );
        }
    }
    #[test]
    fn saved_alive_is_only_first_flag_and_nested_current_versions_are_required() {
        let bytes = carrier();
        let (mut value, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let player = value.first_player.as_mut().unwrap();
        assert!(!player.header.saved_is_alive());
        player.header.ordered_flags[0] = true;
        assert!(player.header.saved_is_alive());
        player.header.ordered_flags[2..].fill(false);
        assert!(player.header.saved_is_alive());
        let names = player.name_manager.encode(Limits::default()).unwrap();
        let culture = player.culture.encode(Limits::default()).unwrap();
        let later_bytes = player.espionage.encode(Limits::default()).unwrap().len()
            + player.religion.encode(Limits::default()).unwrap().len()
            + player.resources.encode(Limits::default()).unwrap().len()
            + player.stats.encode(Limits::default()).unwrap().len()
            + player.techs.encode(Limits::default()).unwrap().len()
            + player.treasury.encode(Limits::default()).unwrap().len()
            + player.units.encode(Limits::default()).unwrap().len()
            + player.eras.encode(Limits::default()).unwrap().len()
            + player.trade.encode(Limits::default()).unwrap().len()
            + player
                .great_people_points
                .encode(Limits::default())
                .unwrap()
                .len()
            + player.influence.encode(Limits::default()).unwrap().len()
            + player.quests.encode(Limits::default()).unwrap().len()
            + player.wmds.encode(Limits::default()).unwrap().len()
            + player.improvements.encode(Limits::default()).unwrap().len()
            + player.governors.encode(Limits::default()).unwrap().len()
            + player
                .cultural_identity
                .encode(Limits::default())
                .unwrap()
                .len()
            + after_cultural_identity_bytes(player);

        let names_start = n - later_bytes - culture.len() - names.len();
        let culture_start = n - later_bytes - culture.len();
        let changed = value.encode_prefix(Limits::default()).unwrap();
        assert!(Prefix::decode_prefix(&changed, Limits::default())
            .unwrap()
            .0
            .first_player
            .unwrap()
            .header
            .saved_is_alive());
        let mut malformed = bytes.clone();
        malformed[names_start + 8..names_start + 12].copy_from_slice(&4u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::NameManager(
                player_name_manager::Error::InvalidContainer
            ))
        ));
        let mut malformed = bytes;
        malformed[culture_start..culture_start + 4].copy_from_slice(&46u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Culture(player_culture::Error::Version {
                expected: 45,
                actual: 46
            }))
        ));
    }
    #[test]
    fn rejects_malformed_stats_fixed_count_and_techs_version() {
        let bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let techs = p.techs.encode(Limits::default()).unwrap();
        let stats = p.stats.encode(Limits::default()).unwrap();
        let after_techs = p.treasury.encode(Limits::default()).unwrap().len()
            + p.units.encode(Limits::default()).unwrap().len()
            + p.eras.encode(Limits::default()).unwrap().len()
            + p.trade.encode(Limits::default()).unwrap().len()
            + p.great_people_points
                .encode(Limits::default())
                .unwrap()
                .len()
            + p.influence.encode(Limits::default()).unwrap().len()
            + p.quests.encode(Limits::default()).unwrap().len()
            + p.wmds.encode(Limits::default()).unwrap().len()
            + p.improvements.encode(Limits::default()).unwrap().len()
            + p.governors.encode(Limits::default()).unwrap().len()
            + p.cultural_identity.encode(Limits::default()).unwrap().len()
            + after_cultural_identity_bytes(&p);

        let stats_start = n - after_techs - techs.len() - stats.len();
        // Stats27 source order: 22 scalar words, TrackedValue, 5 scalar
        // words, four counted vectors, then the first WriteArray<int,64>.
        let tracked_bytes = p
            .stats
            .diplomatic_victory_points
            .encode(Limits::default())
            .unwrap()
            .len();
        let fixed_count = stats_start
            + 4
            + 22 * 4
            + tracked_bytes
            + 5 * 4
            + 4
            + 8 * p.stats.num_units_constructed.len()
            + 4
            + 8 * p.stats.num_projects_advanced.len()
            + 4
            + 4 * p.stats.wonders_lost_plots.len()
            + 4
            + 8 * p.stats.cached_yields_q8.len();
        assert_eq!(&bytes[fixed_count..fixed_count + 4], &64u32.to_le_bytes());
        let mut malformed = bytes.clone();
        malformed[fixed_count..fixed_count + 4].copy_from_slice(&63u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Stats(player_stats::Error::InvalidContainer))
        ));
        let mut malformed = bytes;
        malformed[n - after_techs - techs.len()..n - after_techs - techs.len() + 4]
            .copy_from_slice(&7u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Techs(player_techs::Error::Version {
                expected: 6,
                actual: 7
            }))
        ));
    }
    #[test]
    fn rejects_units_nested_container_version_and_live_count() {
        let bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let units = p.units.encode(Limits::default()).unwrap();
        let start = n
            - after_cultural_identity_bytes(&p)
            - p.cultural_identity.encode(Limits::default()).unwrap().len()
            - p.wmds.encode(Limits::default()).unwrap().len()
            - p.improvements.encode(Limits::default()).unwrap().len()
            - p.governors.encode(Limits::default()).unwrap().len()
            - p.great_people_points
                .encode(Limits::default())
                .unwrap()
                .len()
            - p.influence.encode(Limits::default()).unwrap().len()
            - p.quests.encode(Limits::default()).unwrap().len()
            - p.trade.encode(Limits::default()).unwrap().len()
            - p.eras.encode(Limits::default()).unwrap().len()
            - units.len();
        let mut malformed = bytes.clone();
        malformed[start + 4..start + 8].copy_from_slice(&65538u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Units(player_units::Error::Version {
                expected: 65537,
                actual: 65538
            }))
        ));
        let mut malformed = bytes;
        malformed[start + 16..start + 20]
            .copy_from_slice(&(p.units.records.capacity + 1).to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Units(player_units::Error::InvalidContainer))
        ));
    }

    #[test]
    fn eras_scalar_tail_requires_current_version_without_item_charge() {
        let bytes = carrier();
        let (v, n, items) = Prefix::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        let player = v.first_player.as_ref().unwrap();
        let trade_bytes = player.trade.encode(Limits::default()).unwrap().len()
            + player
                .great_people_points
                .encode(Limits::default())
                .unwrap()
                .len()
            + player.influence.encode(Limits::default()).unwrap().len()
            + player.quests.encode(Limits::default()).unwrap().len()
            + player.wmds.encode(Limits::default()).unwrap().len()
            + player.improvements.encode(Limits::default()).unwrap().len()
            + player.governors.encode(Limits::default()).unwrap().len()
            + player
                .cultural_identity
                .encode(Limits::default())
                .unwrap()
                .len()
            + after_cultural_identity_bytes(player);

        let eras = player.eras;
        let (encoded, era_items) = eras
            .encode_with_items(Limits {
                max_bytes: 12,
                max_items: 0,
            })
            .unwrap();
        assert_eq!((encoded.len(), era_items), (12, 0));
        assert_eq!(
            player_eras::Eras::decode_prefix_with_items(
                &encoded,
                Limits {
                    max_bytes: 12,
                    max_items: 0
                }
            )
            .unwrap()
            .2,
            0
        );
        assert!(items > 0); // whole-prefix one-short item checks never subtract from Era's zero.
        let mut malformed = bytes;
        malformed[n - trade_bytes - encoded.len()..n - trade_bytes - encoded.len() + 4]
            .copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&malformed, Limits::default()),
            Err(Error::Eras(player_eras::Error::Version {
                expected: 2,
                actual: 3
            }))
        ));
    }
    #[test]
    fn complete_trade_tail_requires_current_version() {
        let mut bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let trade = p.trade.encode(Limits::default()).unwrap();
        let later = p
            .great_people_points
            .encode(Limits::default())
            .unwrap()
            .len()
            + p.influence.encode(Limits::default()).unwrap().len()
            + p.quests.encode(Limits::default()).unwrap().len()
            + p.wmds.encode(Limits::default()).unwrap().len()
            + p.improvements.encode(Limits::default()).unwrap().len()
            + p.governors.encode(Limits::default()).unwrap().len()
            + p.cultural_identity.encode(Limits::default()).unwrap().len()
            + after_cultural_identity_bytes(&p);

        bytes[n - later - trade.len()..n - later - trade.len() + 4]
            .copy_from_slice(&36u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::Trade(player_trade::Error::Version {
                expected: 35,
                actual: 36
            }))
        ));
    }
    #[test]
    fn quests_zero_item_tail_requires_current_version() {
        let mut bytes = carrier();
        let (v, n, items) = Prefix::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let later = p.wmds.encode(Limits::default()).unwrap().len()
            + p.improvements.encode(Limits::default()).unwrap().len()
            + p.governors.encode(Limits::default()).unwrap().len()
            + p.cultural_identity.encode(Limits::default()).unwrap().len()
            + after_cultural_identity_bytes(&p);
        let quests = p.quests;
        let (encoded, quest_items) = quests
            .encode_with_items(Limits {
                max_bytes: 8,
                max_items: 0,
            })
            .unwrap();
        assert_eq!((encoded.len(), quest_items), (8, 0));
        assert!(items > 0);
        bytes[n - later - encoded.len()..n - later - encoded.len() + 4]
            .copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::Quests(player_quests::Error::Version {
                expected: 1,
                actual: 2
            }))
        ));
    }
    #[test]
    fn governor_nested_version_is_not_skipped() {
        let mut bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let later = p.cultural_identity.encode(Limits::default()).unwrap().len()
            + after_cultural_identity_bytes(&p);
        let governors = p.governors.encode(Limits::default()).unwrap();
        // Governors6 header: points, recycled version/bits/capacity/live count,
        // then first explicit slot followed immediately by Governor16.
        let first_instance = n - later - governors.len() + 28;
        assert_eq!(
            &bytes[first_instance..first_instance + 4],
            &16u32.to_le_bytes()
        );
        bytes[first_instance..first_instance + 4].copy_from_slice(&17u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::Governors(player_governors::Error::Version {
                expected: 16,
                actual: 17
            }))
        ));
    }
    #[test]
    fn cultural_identity_zero_item_scalar_tail_requires_current_version() {
        let mut bytes = carrier();
        let (v, n, items) = Prefix::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let later = after_cultural_identity_bytes(&p);
        let identity = p.cultural_identity;
        let (encoded, identity_items) = identity
            .encode_with_items(Limits {
                max_bytes: 60,
                max_items: 0,
            })
            .unwrap();
        assert_eq!((encoded.len(), identity_items), (60, 0));
        assert!(items > 0);
        bytes[n - later - encoded.len()..n - later - encoded.len() + 4]
            .copy_from_slice(&11u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::CulturalIdentity(
                player_cultural_identity::Error::Version {
                    expected: 10,
                    actual: 11
                }
            ))
        ));
    }
    #[test]
    fn saved_was_ever_alive_controls_both_branches_and_model_consistency() {
        let bytes = carrier();
        let (mut v, n, items) =
            Prefix::decode_prefix_with_items(&bytes, Limits::default()).unwrap();
        let p = v.first_player.as_mut().unwrap();
        assert!(p.header.saved_was_ever_alive());
        let arrays = p.post_heroes_arrays.take().unwrap();
        let ai_control = p.ai_control.take().unwrap();
        let ai_map = p.ai_influence_map.take().unwrap();
        let ai_planning = p.ai_planning.take().unwrap();
        let ai_city_manager = p.ai_city_manager.take().unwrap();
        let ai_culture = p.ai_culture.take().unwrap();
        let ai_diplomatic = p.ai_diplomatic.take().unwrap();
        let ai_economic = p.ai_economic.take().unwrap();
        let ai_governors = p.ai_governors.take().unwrap();
        let ai_grand_strategic = p.ai_grand_strategic.take().unwrap();
        let ai_homeland = p.ai_homeland.take().unwrap();

        let ai_encoded = [
            ai_control.encode_with_items(Limits::default()).unwrap(),
            ai_map.encode_with_items(Limits::default()).unwrap(),
            ai_planning.encode_with_items(Limits::default()).unwrap(),
            ai_city_manager
                .encode_with_items(Limits::default())
                .unwrap(),
            ai_culture.encode_with_items(Limits::default()).unwrap(),
            ai_diplomatic.encode_with_items(Limits::default()).unwrap(),
            ai_economic.encode_with_items(Limits::default()).unwrap(),
            ai_governors.encode_with_items(Limits::default()).unwrap(),
            ai_grand_strategic
                .encode_with_items(Limits::default())
                .unwrap(),
            ai_homeland.encode_with_items(Limits::default()).unwrap(),
        ];
        let ai_bytes: usize = ai_encoded.iter().map(|(bytes, _)| bytes.len()).sum();
        let ai_items: usize = ai_encoded.iter().map(|(_, items)| *items).sum();
        let tables = [
            &arrays.unit_definitions,
            &arrays.feature_definitions,
            &arrays.continent_definitions,
        ];
        let array_bytes: usize = tables
            .iter()
            .map(|a| a.encode(Limits::default()).unwrap().len())
            .sum();
        let array_items: usize = tables.iter().map(|a| a.pairs.len()).sum();
        let tail = false_tail_carrier();
        let (tail_bytes, tail_items) = tail
            .encode_with_items(
                player_tail::Context {
                    player_version: 47,
                    was_ever_alive: false,
                },
                Limits::default(),
            )
            .unwrap();
        p.tail = Some(tail);
        p.header.ordered_flags[1] = false;
        assert!(!p.header.saved_was_ever_alive());
        v.boundary = Boundary::NextPlayerEntry;
        v.unfinished_player_entries = v.player_count - 1;
        assert!(v.first_player_complete());
        let (encoded, charged) = v.encode_prefix_with_items(Limits::default()).unwrap();
        assert_eq!(
            (encoded.len(), charged),
            (
                n - array_bytes - ai_bytes + tail_bytes.len(),
                items - array_items - ai_items + tail_items
            )
        );
        let mut trailing = encoded.clone();
        words(&mut trailing, &[7, 47]); // next requested ID + Player version remain unread
        let exact = Limits {
            max_bytes: encoded.len(),
            max_items: charged,
        };
        for short in [
            Limits {
                max_bytes: encoded.len() - 1,
                ..exact
            },
            Limits {
                max_items: charged - 1,
                ..exact
            },
        ] {
            assert!(Prefix::decode_prefix_with_items(&trailing, short).is_err());
            assert!(v.encode_prefix(short).is_err());
        }
        let (decoded, consumed, used) = Prefix::decode_prefix_with_items(&trailing, exact).unwrap();
        assert!(decoded.first_player_complete());
        assert_eq!(
            &trailing[consumed..],
            &[7u32.to_le_bytes(), 47u32.to_le_bytes()].concat()
        );
        assert_eq!((decoded, consumed, used), (v, encoded.len(), charged));
        let (mut v, _, _) = Prefix::decode_prefix_with_items(&encoded, exact).unwrap();
        v.first_player.as_mut().unwrap().post_heroes_arrays = Some(arrays);
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
        let arrays = v
            .first_player
            .as_mut()
            .unwrap()
            .post_heroes_arrays
            .take()
            .unwrap();
        v.first_player.as_mut().unwrap().header.ordered_flags[1] = true;
        v.boundary = Boundary::AIMilitary;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
        v.unfinished_player_entries = v.player_count;
        let p = v.first_player.as_mut().unwrap();
        p.tail = None;
        p.post_heroes_arrays = Some(arrays);
        p.ai_control = Some(ai_control);
        p.ai_influence_map = Some(ai_map);
        p.ai_planning = Some(ai_planning);
        p.ai_city_manager = Some(ai_city_manager);
        p.ai_culture = Some(ai_culture);
        p.ai_diplomatic = Some(ai_diplomatic);
        p.ai_economic = Some(ai_economic);
        p.ai_governors = Some(ai_governors);
        p.ai_grand_strategic = Some(ai_grand_strategic);
        p.ai_homeland = Some(ai_homeland);

        assert_eq!(v.encode_prefix(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn true_branch_bool_hash_tables_validate_counts_and_canonical_bools() {
        let bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.unwrap();
        let ai_bytes = after_post_heroes_arrays_bytes(&p);
        let a = p.post_heroes_arrays.unwrap();
        let array_bytes = a.unit_definitions.encode(Limits::default()).unwrap().len()
            + a.feature_definitions
                .encode(Limits::default())
                .unwrap()
                .len()
            + a.continent_definitions
                .encode(Limits::default())
                .unwrap()
                .len();
        let mut invalid_bool = bytes.clone();
        invalid_bool[n - ai_bytes - 1] = 2;
        assert!(matches!(
            Prefix::decode_prefix(&invalid_bool, Limits::default()),
            Err(Error::IndexedBools(
                player_prerequisites::Error::InvalidBool
            ))
        ));
        let mut excessive = bytes;
        excessive[n - ai_bytes - array_bytes..n - ai_bytes - array_bytes + 4]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&excessive, Limits::default()),
            Err(Error::ResourceLimit)
        ));
    }
    #[test]
    fn gated_ai_components_require_current_versions_and_present_models() {
        let bytes = carrier();
        let (mut v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.as_ref().unwrap();
        let ai_bytes = after_post_heroes_arrays_bytes(p);
        let control_bytes = p
            .ai_control
            .as_ref()
            .unwrap()
            .encode(Limits::default())
            .unwrap()
            .len();
        let planning_bytes = p
            .ai_planning
            .as_ref()
            .unwrap()
            .encode(Limits::default())
            .unwrap()
            .len();
        let after_planning = p
            .ai_city_manager
            .as_ref()
            .unwrap()
            .encode(Limits::default())
            .unwrap()
            .len()
            + p.ai_culture
                .as_ref()
                .unwrap()
                .encode(Limits::default())
                .unwrap()
                .len()
            + after_culture_ai_bytes(p);
        let mut bad_control = bytes.clone();
        bad_control[n - ai_bytes..n - ai_bytes + 4].copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bad_control, Limits::default()),
            Err(Error::AIControl(player_ai_control::Error::Version {
                expected: 2,
                actual: 3
            }))
        ));
        let mut bad_map = bytes.clone();
        bad_map[n - ai_bytes + control_bytes + 4..n - ai_bytes + control_bytes + 8]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bad_map, Limits::default()),
            Err(Error::AIInfluenceMap(
                player_ai_influence_map::Error::ResourceLimit
            ))
        ));
        let mut bad_planning = bytes;
        bad_planning[n - after_planning - planning_bytes..n - after_planning - planning_bytes + 4]
            .copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bad_planning, Limits::default()),
            Err(Error::AIPlanning(player_ai_identity::Error::Version {
                expected: 1,
                actual: 2
            }))
        ));
        v.first_player.as_mut().unwrap().ai_influence_map = None;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
    }
    #[test]
    fn complete_false_player_closes_single_entry_manager_and_checks_context() {
        let (mut v, _) = Prefix::decode_prefix(&carrier(), Limits::default()).unwrap();
        let p = v.first_player.as_mut().unwrap();
        p.header.ordered_flags[1] = false;
        p.post_heroes_arrays = None;
        p.ai_control = None;
        p.ai_influence_map = None;
        p.ai_planning = None;
        p.ai_city_manager = None;
        p.ai_culture = None;
        p.ai_diplomatic = None;
        p.ai_economic = None;
        p.ai_governors = None;
        p.ai_grand_strategic = None;
        p.ai_homeland = None;

        p.tail = Some(false_tail_carrier());
        v.player_count = 1;
        v.unfinished_player_entries = 0;
        v.boundary = Boundary::AfterPlayerManager;
        assert!(v.first_player_complete());
        let (encoded, items) = v.encode_prefix_with_items(Limits::default()).unwrap();
        let exact = Limits {
            max_bytes: encoded.len(),
            max_items: items,
        };
        let mut trailing = encoded.clone();
        trailing.extend([91, 92, 93]);
        let (decoded, used, charged) = Prefix::decode_prefix_with_items(&trailing, exact).unwrap();
        assert_eq!((used, charged), (encoded.len(), items));
        assert_eq!(&trailing[used..], &[91, 92, 93]);
        assert_eq!(decoded.boundary, Boundary::AfterPlayerManager);
        assert_eq!(decoded.unfinished_player_entries, 0);
        assert!(decoded.first_player_complete());
        assert_eq!(
            decoded.encode_prefix_with_items(exact).unwrap(),
            (encoded.clone(), items)
        );
        let tail_bytes = v
            .first_player
            .as_ref()
            .unwrap()
            .tail
            .as_ref()
            .unwrap()
            .encode(
                player_tail::Context {
                    player_version: 47,
                    was_ever_alive: false,
                },
                Limits::default(),
            )
            .unwrap()
            .len();
        for end in encoded.len() - tail_bytes..encoded.len() {
            assert!(Prefix::decode_prefix(&encoded[..end], Limits::default()).is_err());
        }
        v.unfinished_player_entries = 1;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
        v.unfinished_player_entries = 0;
        v.boundary = Boundary::NextPlayerEntry;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
        v.boundary = Boundary::AfterPlayerManager;
        // Deliberately invalid false-context model: a true-only tail section.
        v.first_player
            .as_mut()
            .unwrap()
            .tail
            .as_mut()
            .unwrap()
            .alive = Some(player_tail::AliveTail::default());
        assert!(!v.first_player_complete());
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
    }
    #[test]
    fn city_manager_and_culture_ai_require_current_versions_and_gated_models() {
        let bytes = carrier();
        let (mut v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let p = v.first_player.as_ref().unwrap();
        let city = p
            .ai_city_manager
            .as_ref()
            .unwrap()
            .encode(Limits::default())
            .unwrap();
        let culture = p
            .ai_culture
            .as_ref()
            .unwrap()
            .encode(Limits::default())
            .unwrap();
        let later = after_culture_ai_bytes(p);
        let mut bad_city = bytes.clone();
        bad_city
            [n - later - culture.len() - city.len()..n - later - culture.len() - city.len() + 4]
            .copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bad_city, Limits::default()),
            Err(Error::AICityManager(
                player_ai_city_manager::Error::Version {
                    expected: 2,
                    actual: 3
                }
            ))
        ));
        let mut bad_culture = bytes;
        bad_culture[n - later - culture.len()..n - later - culture.len() + 4]
            .copy_from_slice(&16u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bad_culture, Limits::default()),
            Err(Error::AICulture(player_ai_culture::Error::Version {
                expected: 15,
                actual: 16
            }))
        ));
        v.first_player.as_mut().unwrap().ai_culture = None;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
    }
    #[test]
    fn context_free_ai_chain_checks_versions_and_stops_before_military() {
        let bytes = carrier();
        let (mut v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        assert_eq!(v.boundary, Boundary::AIMilitary);
        let p = v.first_player.as_ref().unwrap();
        let cases = [
            (
                p.ai_diplomatic
                    .as_ref()
                    .unwrap()
                    .encode(Limits::default())
                    .unwrap()
                    .len(),
                18,
                Error::AIDiplomatic(player_ai_diplomatic::Error::Version {
                    expected: 18,
                    actual: 19,
                }),
            ),
            (
                p.ai_economic
                    .as_ref()
                    .unwrap()
                    .encode(Limits::default())
                    .unwrap()
                    .len(),
                4,
                Error::AIEconomic(player_ai_economic::Error::Version {
                    expected: 4,
                    actual: 5,
                }),
            ),
            (
                p.ai_governors
                    .as_ref()
                    .unwrap()
                    .encode(Limits::default())
                    .unwrap()
                    .len(),
                3,
                Error::AIGovernors(player_ai_governors::Error::Version {
                    expected: 3,
                    actual: 4,
                }),
            ),
            (
                p.ai_grand_strategic
                    .as_ref()
                    .unwrap()
                    .encode(Limits::default())
                    .unwrap()
                    .len(),
                12,
                Error::AIGrandStrategic(player_ai_grand_strategic::Error::Version {
                    expected: 12,
                    actual: 13,
                }),
            ),
            (
                p.ai_homeland
                    .as_ref()
                    .unwrap()
                    .encode(Limits::default())
                    .unwrap()
                    .len(),
                1,
                Error::AIHomeland(player_ai_homeland::Error::Version {
                    expected: 1,
                    actual: 2,
                }),
            ),
        ];
        let mut at = n - cases.iter().map(|(n, _, _)| n).sum::<usize>();
        for (size, version, error) in cases {
            let mut malformed = bytes.clone();
            malformed[at..at + 4].copy_from_slice(&(version + 1u32).to_le_bytes());
            assert_eq!(
                Prefix::decode_prefix(&malformed, Limits::default()).unwrap_err(),
                error
            );
            at += size;
        }
        assert_eq!(at, n);
        v.first_player.as_mut().unwrap().ai_homeland = None;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
    }
    #[test]
    fn empty_manager_stops_and_preserves_suffix() {
        let mut bytes = Vec::new();
        words(&mut bytes, &[7, 1, 2, 0, 1, 0, 99]);
        let (v, n, i) = Prefix::decode_prefix_with_items(
            &bytes,
            Limits {
                max_bytes: 24,
                max_items: 0,
            },
        )
        .unwrap();
        assert_eq!((n, i), (24, 0));
        assert_eq!(v.boundary, Boundary::AfterEmptyPlayerManager);
        assert_eq!(v.unfinished_player_entries, 0);
        assert_eq!(
            v.encode_prefix_with_items(Limits {
                max_bytes: 24,
                max_items: 0
            })
            .unwrap(),
            (bytes[..24].to_vec(), 0)
        );
        assert!(v.first_player.is_none());
    }
    #[test]
    fn rejects_owner_mismatch_and_noncanonical_flags() {
        let mut bytes = carrier();
        // Independently read Religion framing to obtain the Manager cursor.
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        r.religion().unwrap();
        let owner = r.pos + 8;
        bytes[owner..owner + 4].copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::OwnerMismatch {
                requested: 3,
                record: 64
            })
        ));
        let mut bytes = carrier();
        bytes[owner + 4 + 4 + 4 + 8 + 60] = 2;
        assert!(matches!(
            Prefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::InvalidBool(2))
        ));
    }
    #[test]
    fn formation_structural_sizing_delegates_validation_and_charges_counts() {
        let mut bytes = Vec::new();
        words(&mut bytes, &[1, 0x10001, 1, 2, 3, 4, 5]);
        bytes.extend_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);
        words(
            &mut bytes,
            &[1, 1, 2, 3, 4, 0xffff8000, 5, 6, 2, 0xffffffff, 0x12345678],
        );
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        let value = r.formation().unwrap();
        assert_eq!(r.pos, bytes.len());
        assert_eq!(r.items, 3);
        assert_eq!(value.last_path[0].x, i16::MIN);
        assert_eq!(value.unit_ids, [u32::MAX, 0x12345678]);
        assert_eq!(value.encode(Limits::default()).unwrap(), bytes);
        bytes[28] = 2;
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert!(matches!(
            r.formation(),
            Err(Error::Formation(unit::Error::NoncanonicalBool(2)))
        ));
    }
    #[test]
    fn recycled_metadata_slots_and_duplicate_rejection() {
        let mut bytes = Vec::new();
        words(
            &mut bytes,
            &[65537, 16, 3, 2, 2, 100, 0, 101, 0x70000, 1, 1, 1, 1],
        );
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        let v = r.recycled(Reader::u32).unwrap();
        assert_eq!(v.entries[0].index, 2);
        assert_eq!(v.entries[1].index, 0);
        assert_eq!(v.current_id_bits, 0x70000);
        assert_eq!(v.free_links, [1]);
        assert_eq!(r.pos, bytes.len());
        bytes[24..28].copy_from_slice(&2u32.to_le_bytes());
        let mut r = Reader {
            bytes: &bytes,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert!(matches!(
            r.recycled(Reader::u32),
            Err(Error::InvalidContainer)
        ));
    }
}

struct Writer {
    bytes: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.bytes.len(),
            max_items: self.limits.max_items - self.items,
        }
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        let items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        self.items = items;
        Ok(())
    }
    fn raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn word(&mut self, v: u32) -> Result<(), Error> {
        self.raw(&v.to_le_bytes())
    }
    fn int(&mut self, v: i32) -> Result<(), Error> {
        self.word(v as u32)
    }
    fn boolean(&mut self, v: bool) -> Result<(), Error> {
        self.raw(&[u8::from(v)])
    }
    fn coord(&mut self, v: PlotCoord) -> Result<(), Error> {
        self.int(v.x)?;
        self.int(v.y)
    }
    fn count(&mut self, n: usize) -> Result<(), Error> {
        self.charge(n)?;
        self.word(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn hashes(&mut self, v: &[u32]) -> Result<(), Error> {
        self.count(v.len())?;
        for h in v {
            self.word(*h)?;
        }
        Ok(())
    }
    fn component<T, E>(
        &mut self,
        v: &T,
        encode: impl FnOnce(&T, Limits) -> Result<(Vec<u8>, usize), E>,
        error: impl FnOnce(E) -> Error,
    ) -> Result<(), Error> {
        let (bytes, items) = encode(v, self.remaining()).map_err(error)?;
        self.charge(items)?;
        self.raw(&bytes)
    }
    fn religion(&mut self, v: &Religion) -> Result<(), Error> {
        self.word(7)?;
        self.int(v.minimum_faith_for_next_pantheon)?;
        self.int(v.next_pantheon_founder)?;
        self.count(v.records.len())?;
        for v in &v.records {
            self.word(7)?;
            self.word(v.religion_hash)?;
            self.int(v.founder)?;
            self.int(v.founded_turn)?;
            self.coord(v.holy_city)?;
            self.count(v.custom_name.len())?;
            self.raw(&v.custom_name)?;
            self.hashes(&v.belief_hashes)?;
            self.hashes(&v.building_hashes)?;
            self.hashes(&v.unit_hashes)?;
            self.int(v.anyone_condemns_favor)?;
            self.boolean(v.condemn_prohibited)?;
            self.boolean(v.auto_spreads_to_new_cities)?;
            self.boolean(v.awards_envoy)?;
            self.int(v.awards_envoy_religious_pressure)?;
        }
        Ok(())
    }
    fn header(&mut self, v: &PlayerHeader) -> Result<(), Error> {
        self.word(47)?;
        self.int(v.owner)?;
        self.coord(v.starting_plot)?;
        for v in &v.ordered_scalars {
            self.int(*v)?;
        }
        for v in &v.ordered_flags {
            self.boolean(*v)?;
        }
        self.word(v.handicap_hash)?;
        self.word(v.district_hash)?;
        self.word(v.project_hash)?;
        self.int(v.final_scalar)
    }
    fn formation(&mut self, v: &Formation) -> Result<(), Error> {
        self.component(
            v,
            |v, l| {
                let n = v
                    .last_path
                    .len()
                    .checked_add(v.unit_ids.len())
                    .ok_or(unit::Error::ResourceLimit)?;
                Ok((v.encode(l)?, n))
            },
            Error::Formation,
        )
    }
    fn recycled<T>(
        &mut self,
        v: &map::Recycled<T>,
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if v.entries.len() > v.capacity as usize {
            return Err(Error::InvalidContainer);
        }
        self.word(65537)?;
        self.word(16)?;
        self.word(v.capacity)?;
        self.count(v.entries.len())?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(v.entries.len())
            .map_err(|_| Error::ResourceLimit)?;
        for entry in &v.entries {
            if entry.index < 0 || entry.index as u32 >= v.capacity {
                return Err(Error::InvalidContainer);
            }
            slots.push(entry.index);
            self.int(entry.index)?;
            write(self, &entry.value)?;
        }
        slots.sort_unstable();
        if slots.windows(2).any(|s| s[0] == s[1]) {
            return Err(Error::InvalidContainer);
        }
        self.word(v.current_id_bits)?;
        self.count(v.free_links.len())?;
        self.word(v.free_head)?;
        self.word(v.free_tail)?;
        for link in &v.free_links {
            self.word(*link)?;
        }
        Ok(())
    }
    fn cities(&mut self, v: &Cities) -> Result<(), Error> {
        if v.wire_version != 15 {
            return Err(Error::Version {
                expected: 15,
                actual: v.wire_version,
            });
        }
        if !v.building_build_overrides_wire_present {
            return Err(Error::InvalidContainer);
        }
        self.word(15)?;
        self.recycled(&v.records, |w, v| {
            w.component(v, city::City::encode_with_items, Error::City)
        })?;
        self.boolean(v.fresh_water_housing_bonus)?;
        self.int(v.urban_defenses)?;
        self.word(v.home_continent_hash)?;
        self.hashes(&v.settled_continent_hashes)?;
        self.word(v.peak_cities)?;
        self.int(v.extra_starting_population_off_home_continent)?;
        self.int(v.population_after_conquest)?;
        self.hashes(&v.banned_building_hashes)?;
        self.hashes(&v.banned_district_hashes)?;
        self.hashes(&v.banned_district_building_hashes)?;
        self.hashes(&v.banned_project_hashes)?;
        self.count(v.building_build_overrides.len())?;
        for pair in &v.building_build_overrides {
            self.word(pair[0])?;
            self.word(pair[1])?;
        }
        Ok(())
    }
}
impl Writer {
    fn player_prefix(&mut self, p: &FirstPlayer) -> Result<(), Error> {
        if p.requested_owner != p.header.owner {
            return Err(Error::OwnerMismatch {
                requested: p.requested_owner,
                record: p.header.owner,
            });
        }
        self.charge(1)?;
        self.int(p.requested_owner)?;
        self.header(&p.header)?;
        self.word(1)?;
        self.recycled(&p.formations, Writer::formation)?;
        self.component(
            &p.goody_hut,
            pc::GoodyHut::encode_with_items,
            Error::Component,
        )?;
        self.component(&p.bonuses, pc::Bonuses::encode_with_items, Error::Component)?;
        self.component(
            &p.congress,
            pc::Congress::encode_with_items,
            Error::Component,
        )?;
        self.cities(&p.cities)?;
        self.component(
            &p.diplomacy,
            player_diplomacy::Diplomacy::encode_with_items,
            Error::Diplomacy,
        )?;
        self.component(
            &p.districts,
            player_districts::Districts::encode_with_items,
            Error::Districts,
        )?;
        self.component(
            &p.name_manager,
            player_name_manager::NameManager::encode_with_items,
            Error::NameManager,
        )?;
        self.component(
            &p.culture,
            player_culture::Culture::encode_with_items,
            Error::Culture,
        )?;
        self.component(
            &p.espionage,
            player_espionage::Espionage::encode_with_items,
            Error::Espionage,
        )?;
        self.component(
            &p.religion,
            player_religion::Religion::encode_with_items,
            Error::Religion,
        )?;
        self.component(
            &p.resources,
            player_resources::Resources::encode_with_items,
            Error::Resources,
        )?;
        self.component(
            &p.stats,
            player_stats::Stats::encode_with_items,
            Error::Stats,
        )?;
        self.component(
            &p.techs,
            player_techs::Techs::encode_with_items,
            Error::Techs,
        )?;
        self.component(
            &p.treasury,
            player_treasury::Treasury::encode_with_items,
            Error::Treasury,
        )?;
        self.component(
            &p.units,
            player_units::Units::encode_with_items,
            Error::Units,
        )?;
        self.component(&p.eras, player_eras::Eras::encode_with_items, Error::Eras)?;
        self.component(
            &p.trade,
            player_trade::Trade::encode_with_items,
            Error::Trade,
        )?;
        self.component(
            &p.great_people_points,
            player_great_people_points::GreatPeoplePoints::encode_with_items,
            Error::GreatPeoplePoints,
        )?;
        self.component(
            &p.influence,
            player_influence::Influence::encode_with_items,
            Error::Influence,
        )?;
        self.component(
            &p.quests,
            player_quests::Quests::encode_with_items,
            Error::Quests,
        )?;
        self.component(&p.wmds, player_wmds::Wmds::encode_with_items, Error::Wmds)?;
        self.component(
            &p.improvements,
            player_improvements::Improvements::encode_with_items,
            Error::Improvements,
        )?;
        self.component(
            &p.governors,
            player_governors::Governors::encode_with_items,
            Error::Governors,
        )?;
        self.component(
            &p.cultural_identity,
            player_cultural_identity::CulturalIdentity::encode_with_items,
            Error::CulturalIdentity,
        )?;
        self.component(
            &p.heroes,
            player_heroes::Heroes::encode_with_items,
            Error::Heroes,
        )?;
        if let Some(arrays) = &p.post_heroes_arrays {
            for table in [
                &arrays.unit_definitions,
                &arrays.feature_definitions,
                &arrays.continent_definitions,
            ] {
                self.component(
                    table,
                    |value, limits| Ok((value.encode(limits)?, value.pairs.len())),
                    Error::IndexedBools,
                )?;
            }
        }
        if let Some(value) = &p.ai_control {
            self.component(
                value,
                player_ai_control::ControlManager::encode_with_items,
                Error::AIControl,
            )?;
        }
        if let Some(value) = &p.ai_influence_map {
            self.component(
                value,
                player_ai_influence_map::InfluenceMap::encode_with_items,
                Error::AIInfluenceMap,
            )?;
        }
        if let Some(value) = &p.ai_planning {
            self.component(
                value,
                player_ai_identity::PlanningManagerIdentity::encode_with_items,
                Error::AIPlanning,
            )?;
        }
        if let Some(value) = &p.ai_city_manager {
            self.component(
                value,
                player_ai_city_manager::CityManager::encode_with_items,
                Error::AICityManager,
            )?;
        }
        if let Some(value) = &p.ai_culture {
            self.component(
                value,
                player_ai_culture::CultureAi::encode_with_items,
                Error::AICulture,
            )?;
        }
        if let Some(value) = &p.ai_diplomatic {
            self.component(
                value,
                player_ai_diplomatic::Diplomatic::encode_with_items,
                Error::AIDiplomatic,
            )?;
        }
        if let Some(value) = &p.ai_economic {
            self.component(
                value,
                player_ai_economic::Economic::encode_with_items,
                Error::AIEconomic,
            )?;
        }
        if let Some(value) = &p.ai_governors {
            self.component(
                value,
                player_ai_governors::Governors::encode_with_items,
                Error::AIGovernors,
            )?;
        }
        if let Some(value) = &p.ai_grand_strategic {
            self.component(
                value,
                player_ai_grand_strategic::GrandStrategic::encode_with_items,
                Error::AIGrandStrategic,
            )?;
        }
        if let Some(value) = &p.ai_homeland {
            self.component(
                value,
                player_ai_homeland::Homeland::encode_with_items,
                Error::AIHomeland,
            )?;
        }

        if let Some(value) = &p.ai_military {
            self.component(
                value,
                player_ai_military::Military::encode_with_items,
                Error::AIMilitary,
            )?;
        }
        if let Some(value) = &p.ai_research {
            self.component(
                value,
                player_ai_research::Research::encode_with_items,
                Error::AIResearch,
            )?;
        }
        if let Some(value) = &p.ai_tactical {
            self.component(
                value,
                player_ai_tactical::Tactical::encode_with_items,
                Error::AITactical,
            )?;
        }
        if let Some(value) = &p.ai_targets {
            self.component(
                value,
                player_ai_targets::Targets::encode_with_items,
                Error::AITargets,
            )?;
        }
        if let Some(value) = &p.ai_behavior_tree {
            self.component(
                value,
                player_ai_behavior_tree::BehaviorTreeManager::encode_with_items,
                Error::AIBehaviorTree,
            )?;
        }
        if let Some(value) = &p.tail {
            self.component(
                value,
                |value, limits| {
                    value.encode_with_items(
                        player_tail::Context {
                            player_version: 47,
                            was_ever_alive: p.header.saved_was_ever_alive(),
                        },
                        limits,
                    )
                },
                Error::Tail,
            )?;
        }
        Ok(())
    }
}
impl Prefix {
    /// Completeness of the first saved-wire entry, never live initialization.
    pub fn first_player_complete(&self) -> bool {
        self.first_player.as_ref().is_some_and(|p| {
            p.validate_entry(0, self.initialization_binding.as_ref())
                .is_ok_and(|complete| complete)
        })
    }
    pub fn players(&self) -> impl Iterator<Item = &FirstPlayer> {
        self.first_player
            .iter()
            .chain(self.additional_players.iter())
    }
    pub const fn initialization_reconstructed(&self) -> bool {
        false
    }
    pub fn initialization_basis(&self) -> InitializationBasis {
        for p in self.players() {
            if let Some(snapshot) = &p.initialization_snapshot {
                return match snapshot.origin() {
                    initialization::Origin::SuppliedConsistency => {
                        InitializationBasis::SuppliedSnapshotInputs
                    }
                    #[cfg(test)]
                    initialization::Origin::SourceOnly => {
                        InitializationBasis::SourceOnlySnapshotInputs
                    }
                };
            }
        }
        InitializationBasis::ContextFreeWire
    }
    pub fn complete_player_manager_wire(&self) -> bool {
        self.validate_model().is_ok()
            && self.unfinished_player_entries == 0
            && matches!(
                self.boundary,
                Boundary::AfterPlayerManager | Boundary::AfterEmptyPlayerManager
            )
    }
    fn validate_model(&self) -> Result<(), Error> {
        let stored = self
            .additional_players
            .len()
            .checked_add(usize::from(self.first_player.is_some()))
            .ok_or(Error::ResourceLimit)?;
        if (self.player_count == 0) != (stored == 0)
            || (self.first_player.is_none() && !self.additional_players.is_empty())
            || stored > self.player_count as usize
        {
            return Err(Error::InvalidPrefix);
        }
        let mut completed = 0u32;
        let mut last_complete = false;
        for (index, p) in self.players().enumerate() {
            let complete = p.validate_entry(index as u32, self.initialization_binding.as_ref())?;
            if !complete && index + 1 != stored {
                return Err(Error::InvalidPrefix);
            }
            completed += u32::from(complete);
            last_complete = complete;
        }
        let boundary = if stored == 0 {
            Boundary::AfterEmptyPlayerManager
        } else if !last_complete {
            Boundary::AIMilitary
        } else if completed < self.player_count {
            Boundary::NextPlayerEntry
        } else {
            Boundary::AfterPlayerManager
        };
        if self.boundary != boundary
            || self.unfinished_player_entries != self.player_count - completed
        {
            return Err(Error::InvalidPrefix);
        }
        Ok(())
    }
    /// Encode only represented wire entries; validation precedes output allocation.
    pub fn encode_prefix_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        self.validate_model()?;
        let mut w = Writer {
            bytes: Vec::new(),
            items: 0,
            limits,
        };
        w.religion(&self.religion)?;
        w.word(1)?;
        w.word(self.player_count)?;
        if self.player_count as usize > w.remaining().max_items {
            return Err(Error::ResourceLimit);
        }
        for p in self.players() {
            w.player_prefix(p)?;
        }
        Ok((w.bytes, w.items))
    }
    pub fn encode_prefix(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_prefix_with_items(limits)
            .map(|(bytes, _)| bytes)
    }
}

#[cfg(test)]
mod encode_tests {
    use super::*;
    #[test]
    fn encoder_rejects_inconsistent_partial_boundary() {
        let mut v = Prefix {
            religion: Religion {
                minimum_faith_for_next_pantheon: 0,
                next_pantheon_founder: 0,
                records: vec![],
            },
            player_count: 0,
            first_player: None,
            additional_players: Vec::new(),
            initialization_binding: None,
            boundary: Boundary::AfterEmptyPlayerManager,
            unfinished_player_entries: 0,
        };
        assert_eq!(
            v.encode_prefix_with_items(Limits {
                max_bytes: 24,
                max_items: 0
            })
            .unwrap()
            .0
            .len(),
            24
        );
        assert!(matches!(
            v.encode_prefix(Limits {
                max_bytes: 23,
                max_items: 0
            }),
            Err(Error::ResourceLimit)
        ));
        v.boundary = Boundary::AIMilitary;
        assert_eq!(
            v.encode_prefix(Limits::default()),
            Err(Error::InvalidPrefix)
        );
        v.boundary = Boundary::AfterEmptyPlayerManager;
        v.player_count = 1;
        assert_eq!(
            v.encode_prefix(Limits::default()),
            Err(Error::InvalidPrefix)
        );
    }
}

#[cfg(test)]
pub(crate) mod header_versions_tests {
    use super::*;
    fn wire(version: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for word in [version, 7, 11, 12] {
            bytes.extend(word.to_le_bytes());
        }
        for index in 0..15i32 {
            if version == 38 && (index == 11 || index == 13) {
                continue;
            }
            let value = match index {
                1 => i32::MAX,
                2 => i32::MIN,
                _ => index,
            };
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([1, 0, 1, 0, 1, 0]);
        for word in [0xdeadbeefu32, 17, 19, 23] {
            bytes.extend(word.to_le_bytes());
        }
        bytes
    }
    pub(crate) fn cities_prefix_wire(version: u32) -> (Vec<u8>, usize) {
        let mut bytes = Vec::new();
        for word in [7u32, 0, 0, 0, 1, 43, 7] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(wire(version));
        let player = tests::source_first_player();
        let mut writer = Writer {
            bytes: Vec::new(),
            items: 0,
            limits: Limits::default(),
        };
        writer.word(1).unwrap();
        writer
            .recycled(&player.formations, Writer::formation)
            .unwrap();
        writer
            .component(
                &player.goody_hut,
                pc::GoodyHut::encode_with_items,
                Error::Component,
            )
            .unwrap();
        writer
            .component(
                &player.bonuses,
                pc::Bonuses::encode_with_items,
                Error::Component,
            )
            .unwrap();
        writer
            .component(
                &player.congress,
                pc::Congress::encode_with_items,
                Error::Component,
            )
            .unwrap();
        bytes.extend(writer.bytes);
        let end = bytes.len();
        bytes.extend(12345u32.to_le_bytes());
        (bytes, end)
    }
    #[test]
    fn shared_pre_cities_versions_preserve_fields_and_exact_boundary() {
        for version in [38, 47] {
            let (bytes, end) = cities_prefix_wire(version);
            let (prefix, used) = FirstPlayerCitiesPrefix::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: end,
                    max_items: 4096,
                },
            )
            .unwrap();
            assert_eq!(used, end);
            assert_eq!(prefix.header_prefix.header.version, version);
            let player = tests::source_first_player();
            assert_eq!(prefix.body.formations, player.formations);
            assert_eq!(prefix.body.goody_hut, player.goody_hut);
            assert_eq!(prefix.body.bonuses, player.bonuses);
            assert_eq!(prefix.body.congress, player.congress);
            let failure =
                FirstPlayerCitiesPrefix::decode_cities_prefix(&bytes[end..], Limits::default())
                    .unwrap_err();
            assert_eq!(
                failure.error,
                Error::Version {
                    expected: 15,
                    actual: 12345
                }
            );
            for length in 0..end {
                assert!(
                    FirstPlayerCitiesPrefix::decode_prefix(&bytes[..length], Limits::default())
                        .is_err(),
                    "{version}/{length}"
                );
            }
            assert!(FirstPlayerCitiesPrefix::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: end - 1,
                    max_items: 4096
                }
            )
            .is_err());
            assert!(FirstPlayerCitiesPrefix::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: end,
                    max_items: 0
                }
            )
            .is_err());
            // Refuse the first unsupported nested version with its input offset.
            let mut invalid = bytes.clone();
            let formations_at = 28 + if version == 38 { 90 } else { 98 };
            invalid[formations_at..formations_at + 4].copy_from_slice(&2u32.to_le_bytes());
            let failure =
                FirstPlayerCitiesPrefix::decode_prefix(&invalid, Limits::default()).unwrap_err();
            assert_eq!(failure.offset, formations_at);
            assert_eq!(failure.component, "Formations");
            assert_eq!(
                failure.error,
                Error::Version {
                    expected: 1,
                    actual: 2
                }
            );
        }
    }
    #[test]
    fn header_versions_preserve_wire_defaults_and_wrapping_read_assignments() {
        for (version, size) in [(38, 90), (47, 98)] {
            let mut bytes = wire(version);
            assert_eq!(bytes.len(), size);
            bytes.extend(1234u32.to_le_bytes());
            let (header, used) = VersionedPlayerHeader::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: size,
                    max_items: 0,
                },
            )
            .unwrap();
            assert_eq!(used, size);
            assert_eq!(header.version, version);
            assert_eq!(header.saved.owner, 7);
            assert_eq!(header.saved.handicap_hash, 0xdeadbeef);
            assert_eq!(header.saved.ordered_scalars[1], i32::MAX);
            let applied = header.read_applied_scalars();
            if version == 38 {
                assert_eq!(header.saved.ordered_scalars[11], 0);
                assert_eq!(header.saved.ordered_scalars[13], 0);
                assert_eq!(applied[1], -200);
                assert_eq!(applied[2], -100);
            } else {
                assert_eq!(applied, header.saved.ordered_scalars);
                assert_eq!(applied[11], 11);
                assert_eq!(applied[13], 13);
            }
        }
    }
    #[test]
    fn header_versions_truncation_budget_boolean_and_strict_body_refusal() {
        for version in [38, 47] {
            let bytes = wire(version);
            for end in 0..bytes.len() {
                assert_eq!(
                    VersionedPlayerHeader::decode_prefix(&bytes[..end], Limits::default()),
                    Err(Error::Truncated)
                );
            }
            for budget in 0..bytes.len() {
                assert_eq!(
                    VersionedPlayerHeader::decode_prefix(
                        &bytes,
                        Limits {
                            max_bytes: budget,
                            max_items: 0
                        }
                    ),
                    Err(Error::ResourceLimit)
                );
            }
            let mut invalid = bytes.clone();
            invalid[bytes.len() - 22] = 2;
            assert_eq!(
                VersionedPlayerHeader::decode_prefix(&invalid, Limits::default()),
                Err(Error::InvalidBool(2))
            );
        }
        let old = wire(38);
        let mut r = Reader {
            bytes: &old,
            pos: 0,
            items: 0,
            limits: Limits::default(),
        };
        assert_eq!(
            r.header(),
            Err(Error::Version {
                expected: 47,
                actual: 38
            })
        );
        assert_eq!(r.pos, 4);
        for version in [0u32, 37, 39, 46, 48, u32::MAX] {
            assert_eq!(
                VersionedPlayerHeader::decode_prefix(&version.to_le_bytes(), Limits::default()),
                Err(Error::Version {
                    expected: 47,
                    actual: version
                })
            );
        }
    }
    #[test]
    fn header_prefix_stops_before_formations_and_charges_first_entry() {
        let mut bytes = Vec::new();
        for word in [7u32, 0, 0, 0, 1, 43, 7] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(wire(38));
        let expected = bytes.len();
        bytes.extend(1u32.to_le_bytes());
        let (prefix, used) = FirstPlayerHeaderPrefix::decode_prefix(
            &bytes,
            Limits {
                max_bytes: expected,
                max_items: 43,
            },
        )
        .unwrap();
        assert_eq!(used, expected);
        assert_eq!(prefix.player_count, 43);
        assert_eq!(prefix.header.version, 38);
        assert_eq!(
            FirstPlayerHeaderPrefix::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: expected,
                    max_items: 0
                }
            ),
            Err(Error::ResourceLimit)
        );
        bytes[24..28].copy_from_slice(&8i32.to_le_bytes());
        assert_eq!(
            FirstPlayerHeaderPrefix::decode_prefix(&bytes, Limits::default()),
            Err(Error::OwnerMismatch {
                requested: 8,
                record: 7
            })
        );
    }
}

#[cfg(test)]
mod cities_versions_tests {
    use super::*;
    fn wire(version: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Empty recycled City roster, including allocator state and free links.
        for word in [version, 65537, 16, 0, 0, 123, 0, u32::MAX, u32::MAX] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.push(1);
        for word in [0x80000000u32, 0xdeadbeef, 2, 11, 11, 7, 0xffffffff] {
            bytes.extend(word.to_le_bytes());
        }
        if version == 15 {
            bytes.extend(55u32.to_le_bytes());
        }
        for word in [1u32, 21, 1, 22, 1, 23, 1, 24] {
            bytes.extend(word.to_le_bytes());
        }
        if version == 15 {
            for word in [2u32, 31, 32, 31, 33] {
                bytes.extend(word.to_le_bytes());
            }
        }
        bytes
    }
    #[test]
    fn cities_charged_api_keeps_tail_and_exposes_exact_shared_budget() {
        for version in [12, 15] {
            let bytes = wire(version);
            // Two settled hashes, four banned hashes, and (only15) two overrides.
            let expected_items = if version == 12 { 6 } else { 8 };
            let limits = Limits {
                max_bytes: bytes.len(),
                max_items: expected_items,
            };
            let mut carrier = bytes.clone();
            carrier.extend([0xaa, 0xbb]);
            let (value, size, items) =
                FirstPlayerCitiesPrefix::decode_cities_prefix_with_items(&carrier, limits).unwrap();
            assert_eq!((size, items), (bytes.len(), expected_items));
            assert_eq!(
                FirstPlayerCitiesPrefix::decode_cities_prefix(&carrier, limits).unwrap(),
                (value, size)
            );
            assert!(FirstPlayerCitiesPrefix::decode_cities_prefix_with_items(
                &carrier,
                Limits {
                    max_items: expected_items - 1,
                    ..limits
                }
            )
            .is_err());
        }
    }
    #[test]
    fn cities12_and15_exact_grammar_and_limits() {
        for version in [12, 15] {
            let bytes = wire(version);
            let (cities, used) =
                FirstPlayerCitiesPrefix::decode_cities_prefix(&bytes, Limits::default()).unwrap();
            assert_eq!(used, bytes.len());
            assert_eq!(cities.wire_version, version);
            assert_eq!(
                cities.population_after_conquest,
                if version == 15 { 55 } else { 0 }
            );
            assert_eq!(cities.building_build_overrides_wire_present, version == 15);
            assert_eq!(
                cities.building_build_overrides.len(),
                if version == 15 { 2 } else { 0 }
            );
            assert_eq!(cities.settled_continent_hashes, [11, 11]);
            assert_eq!(cities.banned_district_building_hashes, [23]);
            let mut reader = Reader {
                bytes: &bytes,
                pos: 0,
                items: 0,
                limits: Limits::default(),
            };
            reader.cities().unwrap();
            let items = reader.items;
            assert!(FirstPlayerCitiesPrefix::decode_cities_prefix(
                &bytes,
                Limits {
                    max_bytes: bytes.len(),
                    max_items: items
                }
            )
            .is_ok());
            for limits in [
                Limits {
                    max_bytes: bytes.len() - 1,
                    max_items: items,
                },
                Limits {
                    max_bytes: bytes.len(),
                    max_items: items - 1,
                },
            ] {
                assert!(FirstPlayerCitiesPrefix::decode_cities_prefix(&bytes, limits).is_err());
            }
            for length in 0..bytes.len() {
                assert!(FirstPlayerCitiesPrefix::decode_cities_prefix(
                    &bytes[..length],
                    Limits::default()
                )
                .is_err());
            }
            let mut writer = Writer {
                bytes: Vec::new(),
                items: 0,
                limits: Limits::default(),
            };
            if version == 15 {
                writer.cities(&cities).unwrap();
                assert_eq!(writer.bytes, bytes);
            } else {
                assert!(writer.cities(&cities).is_err());
                assert!(writer.bytes.is_empty());
            }
        }
        for version in [0, 11, 13, 14, 16, u32::MAX] {
            assert!(
                matches!(FirstPlayerCitiesPrefix::decode_cities_prefix(&version.to_le_bytes(),Limits::default()).unwrap_err().error, Error::Version {actual,..} if actual==version)
            );
        }
    }
}
