//! Bounded post-Map Core wire prefix through the first Player's complete Culture.
//! Stops before unframed Espionage; this is not a Player/world/gameplay importer.
use crate::unit::{self, Formation, Limits, PlotCoord};
use crate::{
    city, map, player_components as pc, player_culture, player_diplomacy, player_districts,
    player_name_manager,
};

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
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    /// Manager has no entries; later Game components remain unsupported.
    AfterEmptyPlayerManager,
    /// The first entry is partial. Remaining includes that entry's unfinished tail.
    PlayerEspionage,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Prefix {
    pub religion: Religion,
    pub player_count: u32,
    pub first_player: Option<FirstPlayer>,
    pub boundary: Boundary,
    /// No complete Player entry has been consumed at PlayerEspionage.
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
            })
        };
        let boundary = if first_player.is_some() {
            Boundary::PlayerEspionage
        } else {
            Boundary::AfterEmptyPlayerManager
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
        out
    }
    #[test]
    fn typed_empty_rosters_nested_components_and_aggregate_budgets() {
        let bytes = carrier();
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b"untouched Espionage");
        let (value, n, items) =
            Prefix::decode_prefix_with_items(&trailing, Limits::default()).unwrap();
        assert_eq!(n, bytes.len());
        assert_eq!(value.boundary, Boundary::PlayerEspionage);
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
            Err(Error::Culture(player_culture::Error::ResourceLimit))
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
            Err(Error::Culture(player_culture::Error::ResourceLimit))
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
        player.header.ordered_flags[1..].fill(false);
        assert!(player.header.saved_is_alive());
        let names = player.name_manager.encode(Limits::default()).unwrap();
        let culture = player.culture.encode(Limits::default()).unwrap();
        let names_start = n - culture.len() - names.len();
        let culture_start = n - culture.len();
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
        if self.unfinished_player_entries != self.player_count
            || !matches!(
                (self.player_count, self.first_player.as_ref(), self.boundary),
                (0, None, Boundary::AfterEmptyPlayerManager)
                    | (1.., Some(_), Boundary::PlayerEspionage)
            )
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
        v.boundary = Boundary::PlayerEspionage;
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
