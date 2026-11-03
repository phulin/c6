//! Bounded post-Map Core wire prefix through the first Player's complete Heroes and conditional bool tables.
//! Yields at the exact conditional AIControl or inline-tail boundary; no gameplay.
use crate::player_prerequisites::{self, IndexedBools};
use crate::unit::{self, Formation, Limits, PlotCoord};
use crate::{
    city, map, player_components as pc, player_culture, player_diplomacy, player_districts,
    player_name_manager,
};
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
#[derive(Debug, Eq, PartialEq)]
pub struct Cities {
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
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    /// Manager has no entries; later Game components remain unsupported.
    AfterEmptyPlayerManager,
    /// The first entry is partial. Remaining includes that entry's unfinished tail.
    /// WasEverAlive=true: all three tables consumed; AI Control is next.
    AIControl,
    /// WasEverAlive=false: jump to properties at native writer0x2b4f413.
    /// This is inside the later inline tail, not its true-branch start.
    PlayerInlineTail,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Prefix {
    pub religion: Religion,
    pub player_count: u32,
    pub first_player: Option<FirstPlayer>,
    pub boundary: Boundary,
    /// Neither nonempty boundary closes a complete Player entry.
    pub unfinished_player_entries: u32,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
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
        let owner = self.i32()?;
        let starting_plot = self.coord()?;
        let mut ordered_scalars = [0; 15];
        for v in &mut ordered_scalars {
            *v = self.i32()?;
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
        self.version(15)?;
        let records =
            self.recycled(|r| r.component(city::City::decode_prefix_with_items, Error::City))?;
        Ok(Cities {
            records,
            fresh_water_housing_bonus: self.boolean()?,
            urban_defenses: self.i32()?,
            home_continent_hash: self.u32()?,
            settled_continent_hashes: self.hashes()?,
            peak_cities: self.u32()?,
            extra_starting_population_off_home_continent: self.i32()?,
            population_after_conquest: self.i32()?,
            banned_building_hashes: self.hashes()?,
            banned_district_hashes: self.hashes()?,
            banned_district_building_hashes: self.hashes()?,
            banned_project_hashes: self.hashes()?,
            building_build_overrides: self.list(8, |r| Ok([r.u32()?, r.u32()?]))?,
        })
    }
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
            r.charge(0, 1)?;
            let requested_owner = r.i32()?;
            let header = r.header()?;
            if requested_owner != header.owner {
                return Err(Error::OwnerMismatch {
                    requested: requested_owner,
                    record: header.owner,
                });
            }
            r.version(1)?;
            let formations = r.recycled(Self::read_formation)?;
            let goody_hut =
                r.component(pc::GoodyHut::decode_prefix_with_items, Error::Component)?;
            let bonuses = r.component(pc::Bonuses::decode_prefix_with_items, Error::Component)?;
            let congress = r.component(pc::Congress::decode_prefix_with_items, Error::Component)?;
            let cities = r.cities()?;
            let diplomacy = r.component(
                player_diplomacy::Diplomacy::decode_prefix_with_items,
                Error::Diplomacy,
            )?;
            let districts = r.component(
                player_districts::Districts::decode_prefix_with_items,
                Error::Districts,
            )?;
            let name_manager = r.component(
                player_name_manager::NameManager::decode_prefix_with_items,
                Error::NameManager,
            )?;
            let culture = r.component(
                player_culture::Culture::decode_prefix_with_items,
                Error::Culture,
            )?;
            let espionage = r.component(
                player_espionage::Espionage::decode_prefix_with_items,
                Error::Espionage,
            )?;
            let religion = r.component(
                player_religion::Religion::decode_prefix_with_items,
                Error::Religion,
            )?;
            let resources = r.component(
                player_resources::Resources::decode_prefix_with_items,
                Error::Resources,
            )?;
            let stats = r.component(player_stats::Stats::decode_prefix_with_items, Error::Stats)?;
            let techs = r.component(player_techs::Techs::decode_prefix_with_items, Error::Techs)?;
            let treasury = r.component(
                player_treasury::Treasury::decode_prefix_with_items,
                Error::Treasury,
            )?;
            let units = r.component(player_units::Units::decode_prefix_with_items, Error::Units)?;
            let eras = r.component(player_eras::Eras::decode_prefix_with_items, Error::Eras)?;
            let trade = r.component(player_trade::Trade::decode_prefix_with_items, Error::Trade)?;
            let great_people_points = r.component(
                player_great_people_points::GreatPeoplePoints::decode_prefix_with_items,
                Error::GreatPeoplePoints,
            )?;
            let influence = r.component(
                player_influence::Influence::decode_prefix_with_items,
                Error::Influence,
            )?;
            let quests = r.component(
                player_quests::Quests::decode_prefix_with_items,
                Error::Quests,
            )?;
            let wmds = r.component(player_wmds::Wmds::decode_prefix_with_items, Error::Wmds)?;
            let improvements = r.component(
                player_improvements::Improvements::decode_prefix_with_items,
                Error::Improvements,
            )?;
            let governors = r.component(
                player_governors::Governors::decode_prefix_with_items,
                Error::Governors,
            )?;
            let cultural_identity = r.component(
                player_cultural_identity::CulturalIdentity::decode_prefix_with_items,
                Error::CulturalIdentity,
            )?;
            let heroes = r.component(
                player_heroes::Heroes::decode_prefix_with_items,
                Error::Heroes,
            )?;
            let post_heroes_arrays = if header.saved_was_ever_alive() {
                Some(PostHeroesArrays {
                    unit_definitions: r.indexed_bools()?,
                    feature_definitions: r.indexed_bools()?,
                    continent_definitions: r.indexed_bools()?,
                })
            } else {
                None
            };
            Some(FirstPlayer {
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
            })
        };
        let boundary = match first_player.as_ref() {
            Some(p) if p.header.saved_was_ever_alive() => Boundary::AIControl,
            Some(_) => Boundary::PlayerInlineTail,
            None => Boundary::AfterEmptyPlayerManager,
        };
        Ok((
            Self {
                religion,
                player_count,
                first_player,
                boundary,
                unfinished_player_entries: player_count,
            },
            r.pos,
            r.items,
        ))
    }
    fn read_formation(r: &mut Reader<'_>) -> Result<Formation, Error> {
        r.formation()
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(bytes, limits).map(|(v, n, _)| (v, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                promotion_hash: 3203334162,
                times_selected: -1981,
            }],
            proposed_promotions: vec![pmod::ProposedPromotion {
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

        out
    }
    #[test]
    fn typed_empty_rosters_nested_components_and_aggregate_budgets() {
        let bytes = carrier();
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b"untouched AIControl");
        let (value, n, items) =
            Prefix::decode_prefix_with_items(&trailing, Limits::default()).unwrap();
        assert_eq!(n, bytes.len());
        assert_eq!(value.boundary, Boundary::AIControl);
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
            Err(Error::IndexedBools(
                player_prerequisites::Error::ResourceLimit
            ))
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
            Err(Error::ResourceLimit)
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

    /// Run only from an isolated committed source archive. The supplied raw
    /// Core is a validation input, never an offset or trace source for parsing.
    #[test]
    #[ignore = "requires C6_CORE_PLAYER_PREFIX_FIXTURE raw Core input"]
    fn offline_raw_core_whole_prefix_exact_caps() {
        use std::io::Read;
        let path =
            std::env::var_os("C6_CORE_PLAYER_PREFIX_FIXTURE").expect("raw Core fixture path");
        const MAX: u64 = 256 * 1024 * 1024;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .unwrap()
            .take(MAX + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() as u64 <= MAX);
        let value = crate::core_stream::ThroughFirstPlayer::decode(
            &bytes,
            map::Limits::default(),
            Limits::default(),
        )
        .unwrap();
        let prefix = &value.player_prefix;
        let first = prefix.first_player.as_ref().unwrap();
        let expected = if first.header.saved_was_ever_alive() {
            Boundary::AIControl
        } else {
            Boundary::PlayerInlineTail
        };
        assert_eq!(prefix.boundary, expected);
        assert_eq!(
            first.post_heroes_arrays.is_some(),
            first.header.saved_was_ever_alive()
        );
        let (encoded, items) = prefix.encode_prefix_with_items(Limits::default()).unwrap();
        let post_map = &bytes[value.core.consumed..];
        assert_eq!(encoded, post_map[..encoded.len()]);
        let exact = Limits {
            max_bytes: encoded.len(),
            max_items: items,
        };
        let (decoded, consumed, charged) =
            Prefix::decode_prefix_with_items(post_map, exact).unwrap();
        assert_eq!(
            (&decoded, consumed, charged),
            (prefix, encoded.len(), items)
        );
        assert_eq!(
            prefix.encode_prefix_with_items(exact).unwrap(),
            (encoded.clone(), items)
        );
        for short in [
            Limits {
                max_bytes: encoded.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(Prefix::decode_prefix_with_items(post_map, short).is_err());
            assert!(prefix.encode_prefix(short).is_err());
        }
        let p = prefix.first_player.as_ref().unwrap();
        assert!(!p.units.records.entries.is_empty());
        println!(
            "consumed={} post_map_bytes={} post_map_items={} units={} boundary={:?}",
            value.consumed,
            encoded.len(),
            items,
            p.units.records.entries.len(),
            prefix.boundary
        );
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
        p.header.ordered_flags[1] = false;
        assert!(!p.header.saved_was_ever_alive());
        v.boundary = Boundary::PlayerInlineTail;
        let (encoded, charged) = v.encode_prefix_with_items(Limits::default()).unwrap();
        assert_eq!(
            (encoded.len(), charged),
            (n - array_bytes, items - array_items)
        );
        let mut trailing = encoded.clone();
        trailing.extend([255, 255, 255, 255]);
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
        v.boundary = Boundary::AIControl;
        assert_eq!(
            v.encode_prefix(Limits {
                max_bytes: 0,
                max_items: 0
            }),
            Err(Error::InvalidPrefix)
        );
        v.first_player.as_mut().unwrap().post_heroes_arrays = Some(arrays);
        assert_eq!(v.encode_prefix(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn true_branch_bool_hash_tables_validate_counts_and_canonical_bools() {
        let bytes = carrier();
        let (v, n) = Prefix::decode_prefix(&bytes, Limits::default()).unwrap();
        let a = v.first_player.unwrap().post_heroes_arrays.unwrap();
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
        invalid_bool[n - 1] = 2;
        assert!(matches!(
            Prefix::decode_prefix(&invalid_bool, Limits::default()),
            Err(Error::IndexedBools(
                player_prerequisites::Error::InvalidBool
            ))
        ));
        let mut excessive = bytes;
        excessive[n - array_bytes..n - array_bytes + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Prefix::decode_prefix(&excessive, Limits::default()),
            Err(Error::ResourceLimit)
        ));
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
impl Prefix {
    /// Encode exactly the supported prefix, without an unsupported Player tail.
    /// Inconsistent boundary/count/first-entry combinations are rejected.
    pub fn encode_prefix_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let expected_boundary = match self.first_player.as_ref() {
            Some(p) if p.header.saved_was_ever_alive() => Boundary::AIControl,
            Some(_) => Boundary::PlayerInlineTail,
            None => Boundary::AfterEmptyPlayerManager,
        };
        if self.unfinished_player_entries != self.player_count
            || (self.player_count == 0) != self.first_player.is_none()
            || self.boundary != expected_boundary
            || self
                .first_player
                .as_ref()
                .is_some_and(|p| p.header.saved_was_ever_alive() != p.post_heroes_arrays.is_some())
        {
            return Err(Error::InvalidPrefix);
        }
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
        if let Some(p) = &self.first_player {
            if p.requested_owner != p.header.owner {
                return Err(Error::OwnerMismatch {
                    requested: p.requested_owner,
                    record: p.header.owner,
                });
            }
            w.charge(1)?;
            w.int(p.requested_owner)?;
            w.header(&p.header)?;
            w.word(1)?;
            w.recycled(&p.formations, Writer::formation)?;
            w.component(
                &p.goody_hut,
                pc::GoodyHut::encode_with_items,
                Error::Component,
            )?;
            w.component(&p.bonuses, pc::Bonuses::encode_with_items, Error::Component)?;
            w.component(
                &p.congress,
                pc::Congress::encode_with_items,
                Error::Component,
            )?;
            w.cities(&p.cities)?;
            w.component(
                &p.diplomacy,
                player_diplomacy::Diplomacy::encode_with_items,
                Error::Diplomacy,
            )?;
            w.component(
                &p.districts,
                player_districts::Districts::encode_with_items,
                Error::Districts,
            )?;
            w.component(
                &p.name_manager,
                player_name_manager::NameManager::encode_with_items,
                Error::NameManager,
            )?;
            w.component(
                &p.culture,
                player_culture::Culture::encode_with_items,
                Error::Culture,
            )?;
            w.component(
                &p.espionage,
                player_espionage::Espionage::encode_with_items,
                Error::Espionage,
            )?;
            w.component(
                &p.religion,
                player_religion::Religion::encode_with_items,
                Error::Religion,
            )?;
            w.component(
                &p.resources,
                player_resources::Resources::encode_with_items,
                Error::Resources,
            )?;
            w.component(
                &p.stats,
                player_stats::Stats::encode_with_items,
                Error::Stats,
            )?;
            w.component(
                &p.techs,
                player_techs::Techs::encode_with_items,
                Error::Techs,
            )?;
            w.component(
                &p.treasury,
                player_treasury::Treasury::encode_with_items,
                Error::Treasury,
            )?;
            w.component(
                &p.units,
                player_units::Units::encode_with_items,
                Error::Units,
            )?;
            w.component(&p.eras, player_eras::Eras::encode_with_items, Error::Eras)?;
            w.component(
                &p.trade,
                player_trade::Trade::encode_with_items,
                Error::Trade,
            )?;
            w.component(
                &p.great_people_points,
                player_great_people_points::GreatPeoplePoints::encode_with_items,
                Error::GreatPeoplePoints,
            )?;
            w.component(
                &p.influence,
                player_influence::Influence::encode_with_items,
                Error::Influence,
            )?;
            w.component(
                &p.quests,
                player_quests::Quests::encode_with_items,
                Error::Quests,
            )?;
            w.component(&p.wmds, player_wmds::Wmds::encode_with_items, Error::Wmds)?;
            w.component(
                &p.improvements,
                player_improvements::Improvements::encode_with_items,
                Error::Improvements,
            )?;
            w.component(
                &p.governors,
                player_governors::Governors::encode_with_items,
                Error::Governors,
            )?;
            w.component(
                &p.cultural_identity,
                player_cultural_identity::CulturalIdentity::encode_with_items,
                Error::CulturalIdentity,
            )?;
            w.component(
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
                    w.component(
                        table,
                        |value, limits| Ok((value.encode(limits)?, value.pairs.len())),
                        Error::IndexedBools,
                    )?;
                }
            }
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
        v.boundary = Boundary::AIControl;
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
