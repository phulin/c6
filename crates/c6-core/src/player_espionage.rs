//! Bounded Linux XP2 Player.Espionage v15/v17 wire transport, with strict nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 17;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Espionage wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    allocated: usize,
    limits: Limits,
    version_component: &'static str,
}
impl Reader<'_> {
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes - self.pos,
            max_items: self.limits.max_items - self.items,
        }
    }
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn allocate<T>(&mut self, n: usize) -> Result<(), Error> {
        self.allocated = self
            .allocated
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        if self.allocated > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self, component: &'static str) -> Result<(), Error> {
        self.version_component = component;
        let actual = u32::read(self)?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(())
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn raw(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self
            .data
            .len()
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(b);
        Ok(())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&self.to_le_bytes())
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as Self)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        (*self as u32).write(w)
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.charge(n)?;
        let needed = n.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if needed > r.remaining().max_bytes {
            return Err(Error::ResourceLimit);
        }
        if needed > r.data.len().saturating_sub(r.pos) {
            return Err(Error::Truncated);
        }
        r.allocate::<T>(n)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(self.len())?;
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        for x in self {
            x.write(w)?
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version { r.version(stringify!($name))?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version { VERSION.write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}
record!(CapturedSpy, true, {
    owning_player_ordinal:i32,
    unit_type_hash:u32,
    name_index:i32,
    level:i32,
    promotion_hash:u32,
    promotion_hashes:Vec<u32>,
});
/// Independently tagged OffMapSpy wire data. All fields are present at15/17;
/// hashes and the AI controller bits are retained without native index guesses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffMapSpy {
    pub wire_version: u32,
    pub unit_type_hash: u32,
    pub name_index: i32,
    pub level: i32,
    pub experience_points: i32,
    pub return_turn: i32,
    pub return_x: i32,
    pub return_y: i32,
    pub return_city_owner_ordinal: i32,
    pub ai_controller: u32,
    pub promotion_hashes: Vec<u32>,
    pub old_unit_id: u32,
}
impl Wire for OffMapSpy {
    const MIN: usize = 4
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <Vec<u32> as Wire>::MIN
        + <u32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version_component = "OffMapSpy";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 15 | 17) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            unit_type_hash: <u32 as Wire>::read(r)?,
            name_index: <i32 as Wire>::read(r)?,
            level: <i32 as Wire>::read(r)?,
            experience_points: <i32 as Wire>::read(r)?,
            return_turn: <i32 as Wire>::read(r)?,
            return_x: <i32 as Wire>::read(r)?,
            return_y: <i32 as Wire>::read(r)?,
            return_city_owner_ordinal: <i32 as Wire>::read(r)?,
            ai_controller: <u32 as Wire>::read(r)?,
            promotion_hashes: <Vec<u32> as Wire>::read(r)?,
            old_unit_id: <u32 as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.unit_type_hash.write(w)?;
        self.name_index.write(w)?;
        self.level.write(w)?;
        self.experience_points.write(w)?;
        self.return_turn.write(w)?;
        self.return_x.write(w)?;
        self.return_y.write(w)?;
        self.return_city_owner_ordinal.write(w)?;
        self.ai_controller.write(w)?;
        self.promotion_hashes.write(w)?;
        self.old_unit_id.write(w)?;
        Ok(())
    }
}

record!(CityWithSources, true, {
    player_ordinal:i32,
    city_id:u32,
    expire_turn:i32,
});
/// Independently tagged mission-history wire data. The operation remains a
/// wire hash before native definition lookup; result enum bits are preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionHistoryRecord {
    pub wire_version: u32,
    pub name_index: i32,
    pub operation_hash: u32,
    pub target_city_id: u32,
    pub target_player_ordinal: i32,
    pub target_plot_index: i32,
    pub completion_turn: i32,
    pub initial_result_ordinal: i32,
    pub escape_result_ordinal: i32,
    pub level_after_mission: i32,
    pub loot_info: i32,
}
impl Wire for MissionHistoryRecord {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version_component = "MissionHistoryRecord";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 15 | 17) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            name_index: <i32 as Wire>::read(r)?,
            operation_hash: <u32 as Wire>::read(r)?,
            target_city_id: <u32 as Wire>::read(r)?,
            target_player_ordinal: <i32 as Wire>::read(r)?,
            target_plot_index: <i32 as Wire>::read(r)?,
            completion_turn: <i32 as Wire>::read(r)?,
            initial_result_ordinal: <i32 as Wire>::read(r)?,
            escape_result_ordinal: <i32 as Wire>::read(r)?,
            level_after_mission: <i32 as Wire>::read(r)?,
            loot_info: <i32 as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.name_index.write(w)?;
        self.operation_hash.write(w)?;
        self.target_city_id.write(w)?;
        self.target_player_ordinal.write(w)?;
        self.target_plot_index.write(w)?;
        self.completion_turn.write(w)?;
        self.initial_result_ordinal.write(w)?;
        self.escape_result_ordinal.write(w)?;
        self.level_after_mission.write(w)?;
        self.loot_info.write(w)?;
        Ok(())
    }
}

record!(CounterSpyYieldAward, true, {
    yield_ordinal:i32,
    amount:i32,
    per_x_level:i32,
});
record!(TargetCitySpyYieldPercent, true, {
    yield_ordinal:i32,
    percent:i32,
});
/// Player.Espionage15/17 wire projection. Empty omitted15 vectors represent
/// no wire entries, not an empty native gameplay list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Espionage {
    pub wire_version: u32,
    pub spy_capacity: i32,
    pub captured_spies: Vec<CapturedSpy>,
    pub off_map_spies: Vec<OffMapSpy>,
    pub cities_with_sources: Vec<CityWithSources>,
    pub global_offense_bonus: i32,
    pub global_defense_bonus: i32,
    pub extra_steal_tech_boosts: i32,
    pub mission_histories: Vec<MissionHistoryRecord>,
    pub counter_spy_yield_awards: Vec<CounterSpyYieldAward>,
    pub target_city_spy_yield_percents: Vec<TargetCitySpyYieldPercent>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedEspionageState {
    UnknownRetainedBaseline,
}
impl Espionage {
    /// Applies to counter_spy_yield_awards and target_city_spy_yield_percents.
    /// Native15 self-default copies do not reset or alter either prior list.
    pub fn omitted_read_state(&self) -> Option<OmittedEspionageState> {
        (self.wire_version == 15).then_some(OmittedEspionageState::UnknownRetainedBaseline)
    }
}
impl Wire for Espionage {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <Vec<CapturedSpy> as Wire>::MIN
        + <Vec<OffMapSpy> as Wire>::MIN
        + <Vec<CityWithSources> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<MissionHistoryRecord> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version_component = "Espionage";
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 15 | 17) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            spy_capacity: <i32 as Wire>::read(r)?,
            captured_spies: <Vec<CapturedSpy> as Wire>::read(r)?,
            off_map_spies: <Vec<OffMapSpy> as Wire>::read(r)?,
            cities_with_sources: <Vec<CityWithSources> as Wire>::read(r)?,
            global_offense_bonus: <i32 as Wire>::read(r)?,
            global_defense_bonus: <i32 as Wire>::read(r)?,
            extra_steal_tech_boosts: <i32 as Wire>::read(r)?,
            mission_histories: <Vec<MissionHistoryRecord> as Wire>::read(r)?,
            counter_spy_yield_awards: if wire_version == 17 {
                <Vec<CounterSpyYieldAward> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            target_city_spy_yield_percents: if wire_version == 17 {
                <Vec<TargetCitySpyYieldPercent> as Wire>::read(r)?
            } else {
                Vec::new()
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        VERSION.write(w)?;
        self.spy_capacity.write(w)?;
        self.captured_spies.write(w)?;
        self.off_map_spies.write(w)?;
        self.cities_with_sources.write(w)?;
        self.global_offense_bonus.write(w)?;
        self.global_defense_bonus.write(w)?;
        self.extra_steal_tech_boosts.write(w)?;
        self.mission_histories.write(w)?;
        self.counter_spy_yield_awards.write(w)?;
        self.target_city_spy_yield_percents.write(w)?;
        Ok(())
    }
}

/// Exact Rust reader cursor before refusal. Direct version errors include
/// their four-byte tag; this is not an original native execution failure PC.
#[derive(Debug, Eq, PartialEq)]
pub struct Boundary {
    pub consumed: usize,
    pub component: &'static str,
    pub error: Error,
}
impl Espionage {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_boundary(bytes, limits).map_err(|b| b.error)
    }
    pub fn decode_prefix_with_boundary(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Boundary> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits,
            version_component: "Espionage",
        };
        match Self::read(&mut r) {
            Ok(v) => Ok((v, r.pos, r.items)),
            Err(error) => Err(Boundary {
                consumed: r.pos,
                component: if matches!(error, Error::Version { .. }) {
                    r.version_component
                } else {
                    "Espionage field"
                },
                error,
            }),
        }
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((v, n))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n) = Self::decode_prefix(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        self.write(&mut w)?;
        Ok((w.data, w.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            max_bytes: 1 << 20,
            max_items: 10000,
        }
    }
    fn specimen() -> Espionage {
        Espionage {
            wire_version: VERSION,
            spy_capacity: -7,
            captured_spies: vec![CapturedSpy {
                owning_player_ordinal: -1,
                unit_type_hash: 0xffffffff,
                name_index: -8,
                level: i32::MAX,
                promotion_hash: 0xf1234567,
                promotion_hashes: vec![0xf1234567, 0xf1234567, 0xffffffff],
            }],
            off_map_spies: vec![OffMapSpy {
                wire_version: VERSION,
                unit_type_hash: 0x80000000,
                name_index: 17,
                level: -3,
                experience_points: i32::MIN,
                return_turn: 38,
                return_x: -91,
                return_y: 43,
                return_city_owner_ordinal: 62,
                ai_controller: 0xfedcba98,
                promotion_hashes: vec![0, 0xffffffff],
                old_unit_id: 0xffffffee,
            }],
            cities_with_sources: vec![CityWithSources {
                player_ordinal: 62,
                city_id: 0x87654321,
                expire_turn: -29,
            }],
            global_offense_bonus: i32::MIN,
            global_defense_bonus: i32::MAX,
            extra_steal_tech_boosts: -19,
            mission_histories: vec![MissionHistoryRecord {
                wire_version: VERSION,
                name_index: 41,
                operation_hash: 0xf1234567,
                target_city_id: 0xffffffff,
                target_player_ordinal: -1,
                target_plot_index: -38,
                completion_turn: 777,
                initial_result_ordinal: -7,
                escape_result_ordinal: 99,
                level_after_mission: 13,
                loot_info: i32::MIN,
            }],
            counter_spy_yield_awards: vec![CounterSpyYieldAward {
                yield_ordinal: -19,
                amount: i32::MAX,
                per_x_level: -3,
            }],
            target_city_spy_yield_percents: vec![TargetCitySpyYieldPercent {
                yield_ordinal: 99,
                percent: i32::MIN,
            }],
        }
    }
    #[test]
    fn source_carrier_wire_words() {
        // Independent current target writer order, including every count and tag.
        let words: &[u32] = &[
            17,
            (-7i32) as u32,
            1,
            17,
            u32::MAX,
            u32::MAX,
            (-8i32) as u32,
            i32::MAX as u32,
            0xf1234567,
            3,
            0xf1234567,
            0xf1234567,
            u32::MAX,
            1,
            17,
            0x80000000,
            17,
            (-3i32) as u32,
            i32::MIN as u32,
            38,
            (-91i32) as u32,
            43,
            62,
            0xfedcba98,
            2,
            0,
            u32::MAX,
            0xffffffee,
            1,
            17,
            62,
            0x87654321,
            (-29i32) as u32,
            i32::MIN as u32,
            i32::MAX as u32,
            (-19i32) as u32,
            1,
            17,
            41,
            0xf1234567,
            u32::MAX,
            u32::MAX,
            (-38i32) as u32,
            777,
            (-7i32) as u32,
            99,
            13,
            i32::MIN as u32,
            1,
            17,
            (-19i32) as u32,
            i32::MAX as u32,
            (-3i32) as u32,
            1,
            17,
            99,
            i32::MIN as u32,
        ];
        let bytes: Vec<u8> = words.iter().flat_map(|x| x.to_le_bytes()).collect();
        let value = specimen();
        assert_eq!(value.encode(limits()).unwrap(), bytes);
        assert_eq!(Espionage::decode(&bytes, limits()).unwrap(), value);
        assert_eq!(value.encode_with_items(limits()).unwrap().1, 11);
    }
    #[test]
    fn mission_history15_keeps_all_signed_fields_and_operation_hash() {
        let mut bytes = Vec::new();
        for word in [
            15u32,
            (-17i32) as u32,
            0xfedcba98,
            0xab010007,
            u32::MAX,
            123,
            (-257i32) as u32,
            i32::MIN as u32,
            i32::MAX as u32,
            9,
            (-7i32) as u32,
        ] {
            bytes.extend(word.to_le_bytes());
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 0,
        };
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: exact,
            version_component: "Espionage",
        };
        let v = MissionHistoryRecord::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (44, 0));
        assert_eq!(v.wire_version, 15);
        assert_eq!(
            (v.name_index, v.operation_hash, v.target_city_id),
            (-17, 0xfedcba98, 0xab010007)
        );
        assert_eq!(
            (
                v.target_player_ordinal,
                v.target_plot_index,
                v.completion_turn
            ),
            (-1, 123, -257)
        );
        assert_eq!(
            (v.initial_result_ordinal, v.escape_result_ordinal),
            (i32::MIN, i32::MAX)
        );
        assert_eq!((v.level_after_mission, v.loot_info), (9, -7));
        let mut w = Writer {
            data: vec![0xaa],
            items: 0,
            limits: exact,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert_eq!(w.data, [0xaa]);
        for n in 0..end {
            let mut r = Reader {
                data: &bytes[..n],
                pos: 0,
                items: 0,
                allocated: 0,
                limits: exact,
                version_component: "Espionage",
            };
            assert!(MissionHistoryRecord::read(&mut r).is_err(), "{n}");
        }
        let mut bad = bytes.clone();
        bad[..4].copy_from_slice(&16u32.to_le_bytes());
        let mut r = Reader {
            data: &bad,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: exact,
            version_component: "Espionage",
        };
        assert_eq!(
            MissionHistoryRecord::read(&mut r),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
    }
    fn off_map_spy15_literal() -> Vec<u8> {
        let mut bytes = Vec::new();
        // Own tag; unit hash, name, level, XP, return turn, x/y, owner,
        // raw AI controller, then ordered promotion hashes and old full Unit ID.
        for value in [
            15u32,
            0xfedcba98,
            (-17i32) as u32,
            3,
            (-257i32) as u32,
            51,
            u32::MAX,
            77,
            62,
            0x87654321,
            2,
            u32::MAX,
            u32::MAX,
            0xab010007,
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn off_map_spy15_retains_all_fields_in_source_wire_order() {
        let mut bytes = off_map_spy15_literal();
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 2,
        };
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: exact,
            version_component: "Espionage",
        };
        let v = OffMapSpy::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (end, 2));
        assert_eq!(v.wire_version, 15);
        assert_eq!(
            (v.unit_type_hash, v.ai_controller, v.old_unit_id),
            (0xfedcba98, 0x87654321, 0xab010007)
        );
        assert_eq!(
            (v.name_index, v.level, v.experience_points, v.return_turn),
            (-17, 3, -257, 51)
        );
        assert_eq!(
            (v.return_x, v.return_y, v.return_city_owner_ordinal),
            (-1, 77, 62)
        );
        assert_eq!(v.promotion_hashes, vec![u32::MAX, u32::MAX]);
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: exact,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        for n in 0..end {
            let mut r = Reader {
                data: &bytes[..n],
                pos: 0,
                items: 0,
                allocated: 0,
                limits: exact,
                version_component: "Espionage",
            };
            assert!(OffMapSpy::read(&mut r).is_err(), "{n}");
        }
        let mut bad = bytes.clone();
        bad[..4].copy_from_slice(&16u32.to_le_bytes());
        let mut r = Reader {
            data: &bad,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: exact,
            version_component: "Espionage",
        };
        assert_eq!(
            OffMapSpy::read(&mut r),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
        let mut count = bytes.clone();
        count[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut r = Reader {
            data: &count,
            pos: 0,
            items: 0,
            allocated: 0,
            limits: exact,
            version_component: "Espionage",
        };
        assert_eq!(OffMapSpy::read(&mut r), Err(Error::ResourceLimit));
    }
    #[test]
    fn espionage15_preserves_unknown_omitted_lists_and_present_history() {
        let mut b = Vec::new();
        for v in [15i32, 7, 0, 1] {
            b.extend(v.to_le_bytes());
        }
        b.extend(off_map_spy15_literal());
        for v in [0i32, -1, 2, -3, 1] {
            b.extend(v.to_le_bytes());
        }
        let history_tag = b.len();
        for v in [15i32, -4, -1, -5, -6, -7, -8, -9, -10, -11, -12] {
            b.extend(v.to_le_bytes());
        }
        let end = b.len();
        b.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 4,
        };
        let (v, used, items) = Espionage::decode_prefix_with_items(&b, exact).unwrap();
        assert_eq!((used, items, v.wire_version), (end, 4, 15));
        assert_eq!(v.spy_capacity, 7);
        assert_eq!(v.off_map_spies[0].wire_version, 15);
        assert_eq!(
            (
                v.global_offense_bonus,
                v.global_defense_bonus,
                v.extra_steal_tech_boosts
            ),
            (-1, 2, -3)
        );
        assert_eq!(v.mission_histories[0].operation_hash, u32::MAX);
        assert_eq!(v.mission_histories[0].loot_info, -12);
        assert_eq!(
            v.omitted_read_state(),
            Some(OmittedEspionageState::UnknownRetainedBaseline)
        );
        assert!(
            v.counter_spy_yield_awards.is_empty() && v.target_city_spy_yield_percents.is_empty()
        );
        assert_eq!(v.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(Espionage::decode_prefix(&b[..n], exact).is_err(), "{n}");
        }
        assert_eq!(
            Espionage::decode_prefix(
                &b,
                Limits {
                    max_items: 0,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut nested = b.clone();
        nested[history_tag..history_tag + 4].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            Espionage::decode_prefix(&nested, exact),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
        let boundary = Espionage::decode_prefix_with_boundary(&nested, exact).unwrap_err();
        assert_eq!(
            (boundary.component, boundary.consumed),
            ("MissionHistoryRecord", history_tag + 4)
        );
        assert_eq!(
            boundary.error,
            Error::Version {
                expected: 17,
                actual: 16
            }
        );
        let mut unknown = b.clone();
        unknown[..4].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            Espionage::decode_prefix(&unknown, exact),
            Err(Error::Version {
                expected: 17,
                actual: 16
            })
        );
    }
    #[test]
    fn nonempty_roundtrip_and_prefix() {
        let v = specimen();
        let (b, i) = v.encode_with_items(limits()).unwrap();
        let (d, n, j) = Espionage::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!(d, v);
        assert_eq!((n, j), (b.len(), i));
        let mut tail = b.clone();
        tail.push(2);
        assert_eq!(Espionage::decode(&tail, limits()), Err(Error::Trailing));
        assert_eq!(
            Espionage::decode_prefix(&tail, limits()).unwrap().1,
            b.len()
        );
        assert_eq!(v.encode(limits()).unwrap(), b);
    }
    #[test]
    fn every_truncation() {
        let b = specimen().encode(limits()).unwrap();
        for n in 0..b.len() {
            assert!(Espionage::decode(&b[..n], limits()).is_err(), "{n}");
        }
    }
    #[test]
    fn all_nested_versions() {
        let b = specimen().encode(limits()).unwrap();
        let mut offsets = vec![0];
        let mut p = 12;
        offsets.push(p);
        p += 28 + 12;
        assert_eq!(u32::from_le_bytes(b[p..p + 4].try_into().unwrap()), 1);
        p += 4;
        offsets.push(p);
        p += 48 + 8;
        p += 4;
        offsets.push(p);
        p += 16 + 12 + 4;
        offsets.push(p);
        p += 44 + 4;
        offsets.push(p);
        p += 16 + 4;
        offsets.push(p);
        for p in offsets {
            assert_eq!(&b[p..p + 4], &17u32.to_le_bytes());
            let mut bad = b.clone();
            bad[p] = 16;
            assert_eq!(
                Espionage::decode(&bad, limits()),
                Err(Error::Version {
                    expected: 17,
                    actual: 16
                })
            );
        }
    }
    #[test]
    fn exact_budgets() {
        let v = specimen();
        let (b, i) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: i,
        };
        assert_eq!(Espionage::decode(&b, exact).unwrap(), v);
        assert_eq!(v.encode(exact).unwrap(), b);
        for low in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: i - 1,
                ..exact
            },
        ] {
            assert!(v.encode(low).is_err());
            assert!(Espionage::decode(&b, low).is_err());
        }
    }
    #[test]
    fn allocation_preflight() {
        let mut b = specimen().encode(limits()).unwrap();
        b[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Espionage::decode(&b, limits()), Err(Error::ResourceLimit));
        let mut r = Reader {
            data: &[1, 0, 0, 0, 17, 0, 0, 0],
            pos: 0,
            items: 0,
            allocated: 0,
            limits: limits(),
            version_component: "Espionage",
        };
        assert_eq!(Vec::<CapturedSpy>::read(&mut r), Err(Error::Truncated));
    }
}
