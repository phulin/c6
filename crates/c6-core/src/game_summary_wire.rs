//! Reached Linux GameSummary4 / shared repository3 / Repository0 wire transport.
//! Keeps independent guard byte strings, dataset order and tuple coordinates,
//! raw Q8 values, moments and plot-state history. Native reference rebuilding,
//! dataset normalization, presentation and gameplay effects are not applied.
//! ThroughSharedRepository remains a bounded prefix; Instance composes both
//! repositories without applying native pooling or reference reconstruction.
use crate::notifications::{self, LocalizedString};
pub use crate::unit::Limits;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    Localization(notifications::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux GameSummary shared wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn raw(&mut self, size: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(size).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn version(&mut self, expected: u32) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(actual)
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as i32)
    }
}
impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.raw(1)?[0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let count = u32::read(r)? as usize;
        r.items = r.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let minimum = count.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if minimum > r.limits.max_bytes - r.pos {
            return Err(Error::ResourceLimit);
        }
        if minimum > r.data.len() - r.pos {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(T::read(r)?);
        }
        Ok(values)
    }
}

impl Wire for u16 {
    const MIN: usize = 2;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(2)?.try_into().unwrap()))
    }
}
/// The first two tuple coordinates are retained in native serialized order;
/// they are not interpreted as live player/object/turn identifiers here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sample {
    pub first: u32,
    pub second: u32,
    pub value_q8_raw: i32,
}
impl Wire for Sample {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            first: u32::read(r)?,
            second: u32::read(r)?,
            value_q8_raw: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSet {
    pub name: Vec<u8>,
    pub display_name: Vec<u8>,
    pub display_description: Vec<u8>,
    pub visible_raw: u8,
    pub collection_type: u32,
    pub values: Vec<Sample>,
}
impl Wire for DataSet {
    const MIN: usize = 21;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            name: Vec::read(r)?,
            display_name: Vec::read(r)?,
            display_description: Vec::read(r)?,
            visible_raw: u8::read(r)?,
            collection_type: u32::read(r)?,
            values: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Moment {
    pub turn: u32,
    pub moment_type: u32,
    pub text: LocalizedString,
    pub player1: u16,
    pub player2: u16,
    pub plot_index: u32,
    pub data1: i32,
    pub data2: i32,
}
impl Wire for Moment {
    const MIN: usize = 28;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let turn = u32::read(r)?;
        let moment_type = u32::read(r)?;
        let (text, n, i) = LocalizedString::decode_prefix_with_items(
            &r.data[r.pos..],
            notifications::Limits {
                max_bytes: r.limits.max_bytes - r.pos,
                max_items: r.limits.max_items - r.items,
                ..notifications::Limits::default()
            },
        )
        .map_err(Error::Localization)?;
        r.pos += n;
        r.items += i;
        Ok(Self {
            turn,
            moment_type,
            text,
            player1: u16::read(r)?,
            player2: u16::read(r)?,
            plot_index: u32::read(r)?,
            data1: i32::read(r)?,
            data2: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotState {
    pub terrain_type: u32,
    pub feature_type: u32,
    pub resource_type: u32,
    pub construction_type: u32,
    pub owner_id: u16,
    pub pillaged_raw: u8,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotStateRecord {
    pub first: u32,
    pub second: u32,
    pub state: PlotState,
}
impl Wire for PlotStateRecord {
    const MIN: usize = 27;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            first: u32::read(r)?,
            second: u32::read(r)?,
            state: PlotState {
                terrain_type: u32::read(r)?,
                feature_type: u32::read(r)?,
                resource_type: u32::read(r)?,
                construction_type: u32::read(r)?,
                owner_id: u16::read(r)?,
                pillaged_raw: u8::read(r)?,
            },
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedRepository {
    pub opening_guard: Vec<u8>,
    pub wire_version: u32,
    pub datasets: Vec<DataSet>,
    pub moments: Vec<Moment>,
    pub plot_states: Vec<PlotStateRecord>,
    pub closing_guard: Vec<u8>,
}
impl Wire for SharedRepository {
    const MIN: usize = 24;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            opening_guard: Vec::read(r)?,
            wire_version: r.version(3)?,
            datasets: Vec::read(r)?,
            moments: Vec::read(r)?,
            plot_states: Vec::read(r)?,
            closing_guard: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThroughSharedRepository {
    pub wire_version: u32,
    pub shared: SharedRepository,
}
impl ThroughSharedRepository {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let v = Self {
            wire_version: r.version(4)?,
            shared: SharedRepository::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}

/// Native interned strings are full byte strings on wire; native PoolString
/// receives a C string. No pooling, NUL truncation or pointer identity is applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RulesetType {
    pub type_name: Vec<u8>,
    pub kind: Vec<u8>,
    pub name: Vec<u8>,
    pub icon: Vec<u8>,
}
impl Wire for RulesetType {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            type_name: Vec::read(r)?,
            kind: Vec::read(r)?,
            name: Vec::read(r)?,
            icon: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Object {
    pub id: i32,
    pub owner_player: i32,
    pub type_name: Vec<u8>,
    pub name: Vec<u8>,
    pub component: notifications::ComponentId,
    pub plot_index: i32,
    pub extra_data: Vec<u8>,
    pub icon: Vec<u8>,
}
impl Wire for Object {
    const MIN: usize = 36;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: i32::read(r)?,
            owner_player: i32::read(r)?,
            type_name: Vec::read(r)?,
            name: Vec::read(r)?,
            component: notifications::ComponentId {
                owner: u16::read(r)? as i16,
                component_type: u16::read(r)? as i16,
                id: u32::read(r)?,
            },
            plot_index: i32::read(r)?,
            extra_data: Vec::read(r)?,
            icon: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPoint {
    pub id: Vec<u8>,
    pub object_id: i32,
    pub value_object_id: i32,
    /// Native assigns the optional numeric value only for nonzero presence.
    /// The saved numeric word is consumed even for zero, and remains preserved.
    pub numeric_present_raw: u8,
    pub numeric_value_word: i32,
    pub value_string: Vec<u8>,
    pub value_type: Vec<u8>,
}
impl Wire for DataPoint {
    const MIN: usize = 25;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: Vec::read(r)?,
            object_id: i32::read(r)?,
            value_object_id: i32::read(r)?,
            numeric_present_raw: u8::read(r)?,
            numeric_value_word: i32::read(r)?,
            value_string: Vec::read(r)?,
            value_type: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValuePair {
    pub first: i32,
    pub second: i32,
}
impl Wire for ValuePair {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            first: i32::read(r)?,
            second: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataSetRecord {
    pub id: Vec<u8>,
    pub object_id: i32,
    pub type_name: Vec<u8>,
    pub values: Vec<ValuePair>,
}
impl Wire for DataSetRecord {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            id: Vec::read(r)?,
            object_id: i32::read(r)?,
            type_name: Vec::read(r)?,
            values: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Repository {
    pub opening_guard: Vec<u8>,
    pub wire_version: u32,
    pub last_object_id: u32,
    pub ruleset_types: Vec<RulesetType>,
    pub objects: Vec<Object>,
    pub data_points: Vec<DataPoint>,
    pub data_sets: Vec<DataSetRecord>,
    pub closing_guard: Vec<u8>,
}
impl Wire for Repository {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            opening_guard: Vec::read(r)?,
            wire_version: r.version(0)?,
            last_object_id: u32::read(r)?,
            ruleset_types: Vec::read(r)?,
            objects: Vec::read(r)?,
            data_points: Vec::read(r)?,
            data_sets: Vec::read(r)?,
            closing_guard: Vec::read(r)?,
        })
    }
}
macro_rules! root_reader {
    ($ty:ty,$body:expr) => {
        impl $ty {
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
                if n != bytes.len() {
                    return Err(Error::Trailing);
                }
                Ok(v)
            }
            pub fn decode_prefix_with_items(
                bytes: &[u8],
                limits: Limits,
            ) -> Result<(Self, usize, usize), Error> {
                let mut r = Reader {
                    data: bytes,
                    pos: 0,
                    items: 0,
                    limits,
                };
                let v = $body(&mut r)?;
                Ok((v, r.pos, r.items))
            }
        }
    };
}
root_reader!(Repository, Repository::read);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub wire_version: u32,
    pub shared: SharedRepository,
    pub repository: Repository,
}
impl Wire for Instance {
    const MIN: usize = 60;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(4)?,
            shared: SharedRepository::read(r)?,
            repository: Repository::read(r)?,
        })
    }
}
root_reader!(Instance, Instance::read);
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        4, 0, 0, 0, 2, 0, 0, 0, 255, 0, 3, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 97, 2, 0, 0, 0, 98, 99,
        0, 0, 0, 0, 255, 255, 255, 255, 255, 2, 0, 0, 0, 7, 0, 0, 0, 9, 0, 0, 0, 0, 255, 255, 255,
        7, 0, 0, 0, 9, 0, 0, 0, 255, 255, 255, 127, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 255, 2,
        123, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 120, 86, 52, 18, 3, 0, 0, 0, 97,
        0, 255, 255, 255, 62, 0, 255, 255, 255, 255, 253, 255, 255, 255, 4, 0, 0, 0, 1, 0, 0, 0, 7,
        0, 0, 0, 8, 0, 0, 0, 170, 170, 170, 170, 187, 187, 187, 187, 204, 204, 204, 204, 221, 221,
        221, 221, 255, 255, 255, 1, 0, 0, 0, 128,
    ];
    #[test]
    fn ordered_tuples_binary_strings_raw_flags_and_nested_localization_survive() {
        let limits = Limits {
            max_bytes: FIXTURE.len(),
            max_items: 17,
        };
        let (v, n, i) = ThroughSharedRepository::decode_prefix_with_items(FIXTURE, limits).unwrap();
        assert_eq!((n, i), (FIXTURE.len(), 17));
        let s = &v.shared;
        assert_eq!(s.opening_guard, [255, 0]);
        assert_eq!(s.closing_guard, [128]);
        assert_eq!(
            (s.datasets.len(), s.moments.len(), s.plot_states.len()),
            (2, 1, 1)
        );
        assert_eq!(s.datasets[0].visible_raw, 255);
        assert_eq!(s.datasets[1].visible_raw, 2);
        assert_eq!(s.datasets[0].collection_type, u32::MAX);
        assert_eq!(s.datasets[1].display_description, [255]);
        assert_eq!(
            s.datasets[0].values,
            vec![
                Sample {
                    first: 7,
                    second: 9,
                    value_q8_raw: -256
                },
                Sample {
                    first: 7,
                    second: 9,
                    value_q8_raw: i32::MAX
                }
            ]
        );
        assert!(s.datasets[1].values.is_empty());
        let m = &s.moments[0];
        assert_eq!(
            (
                m.turn,
                m.moment_type,
                m.player1,
                m.player2,
                m.plot_index,
                m.data1,
                m.data2
            ),
            (u32::MAX, 0x12345678, 65535, 62, u32::MAX, -3, 4)
        );
        assert_eq!(m.text.base, [b'a', 0, 255]);
        assert_eq!(
            m.text.wire_format,
            notifications::LocalizedWireFormat::LegacyLength
        );
        assert_eq!(
            s.plot_states[0],
            PlotStateRecord {
                first: 7,
                second: 8,
                state: PlotState {
                    terrain_type: 0xaaaaaaaa,
                    feature_type: 0xbbbbbbbb,
                    resource_type: 0xcccccccc,
                    construction_type: 0xdddddddd,
                    owner_id: 65535,
                    pillaged_raw: 255
                }
            }
        );
        let mut tail = FIXTURE.to_vec();
        tail.extend([0x55, 0xaa]);
        assert_eq!(
            ThroughSharedRepository::decode_prefix_with_items(&tail, limits)
                .unwrap()
                .1,
            FIXTURE.len()
        );
        assert_eq!(
            ThroughSharedRepository::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
    }
    #[test]
    fn every_cut_budget_count_and_independent_version_is_bounded() {
        for cut in 0..FIXTURE.len() {
            assert!(ThroughSharedRepository::decode(&FIXTURE[..cut], Limits::default()).is_err());
            assert!(ThroughSharedRepository::decode(
                FIXTURE,
                Limits {
                    max_bytes: cut,
                    max_items: 17
                }
            )
            .is_err());
        }
        for max_items in 0..17 {
            assert!(ThroughSharedRepository::decode(
                FIXTURE,
                Limits {
                    max_bytes: FIXTURE.len(),
                    max_items
                }
            )
            .is_err());
        }
        for (at, expected) in TAGS {
            let mut b = FIXTURE.to_vec();
            b[*at..*at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                ThroughSharedRepository::decode(&b, Limits::default()),
                Err(Error::Version {
                    expected: *expected,
                    actual: 99
                })
            );
        }
        for at in COUNTS {
            let mut b = FIXTURE.to_vec();
            b[*at..*at + 4].copy_from_slice(&0x7fffffffu32.to_le_bytes());
            assert!(ThroughSharedRepository::decode(&b, Limits::default()).is_err());
        }
    }
    const TAGS: &[(usize, u32)] = &[(0, 4), (10, 3)];
    const COUNTS: &[usize] = &[4, 14, 18, 23, 29, 38, 66, 70, 75, 85, 89, 101, 124, 155];
    const REPOSITORY_FIXTURE: &[u8] = &[
        2, 0, 0, 0, 0, 255, 0, 0, 0, 0, 255, 255, 255, 255, 1, 0, 0, 0, 1, 0, 0, 0, 255, 1, 0, 0,
        0, 107, 0, 0, 0, 0, 2, 0, 0, 0, 0, 105, 1, 0, 0, 0, 249, 255, 255, 255, 255, 255, 255, 255,
        1, 0, 0, 0, 84, 2, 0, 0, 0, 255, 78, 255, 255, 0, 128, 255, 255, 255, 255, 247, 255, 255,
        255, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 97, 249, 255, 255, 255, 255, 255,
        255, 255, 0, 133, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 249, 255, 255, 255,
        254, 255, 255, 255, 255, 255, 255, 255, 127, 0, 0, 0, 0, 1, 0, 0, 0, 122, 1, 0, 0, 0, 1, 0,
        0, 0, 100, 249, 255, 255, 255, 1, 0, 0, 0, 116, 2, 0, 0, 0, 255, 255, 255, 255, 4, 0, 0, 0,
        255, 255, 255, 255, 5, 0, 0, 0, 1, 0, 0, 0, 128,
    ];
    #[test]
    fn repository_keeps_saved_strings_optional_numeric_words_and_duplicate_pairs() {
        let limits = Limits {
            max_bytes: REPOSITORY_FIXTURE.len(),
            max_items: 23,
        };
        let (v, n, i) = Repository::decode_prefix_with_items(REPOSITORY_FIXTURE, limits).unwrap();
        assert_eq!((n, i), (REPOSITORY_FIXTURE.len(), 23));
        assert_eq!(v.last_object_id, u32::MAX);
        assert_eq!(v.opening_guard, [0, 255]);
        assert_eq!(v.closing_guard, [128]);
        assert_eq!(
            v.ruleset_types[0],
            RulesetType {
                type_name: vec![255],
                kind: b"k".to_vec(),
                name: vec![],
                icon: vec![0, b'i']
            }
        );
        let o = &v.objects[0];
        assert_eq!((o.id, o.owner_player, o.plot_index), (-7, -1, -9));
        assert_eq!(
            o.component,
            notifications::ComponentId {
                owner: -1,
                component_type: i16::MIN,
                id: u32::MAX
            }
        );
        assert_eq!(o.name, [255, b'N']);
        assert_eq!(o.extra_data, [0]);
        assert_eq!(
            (
                v.data_points[0].numeric_present_raw,
                v.data_points[0].numeric_value_word
            ),
            (0, -123)
        );
        assert_eq!(
            (
                v.data_points[1].numeric_present_raw,
                v.data_points[1].numeric_value_word
            ),
            (255, i32::MAX)
        );
        assert_eq!(
            v.data_sets[0].values,
            vec![
                ValuePair {
                    first: -1,
                    second: 4
                },
                ValuePair {
                    first: -1,
                    second: 5
                }
            ]
        );
        let mut all = FIXTURE.to_vec();
        all.extend(REPOSITORY_FIXTURE);
        let (v, n, i) = Instance::decode_prefix_with_items(
            &all,
            Limits {
                max_bytes: all.len(),
                max_items: 40,
            },
        )
        .unwrap();
        assert_eq!((n, i), (all.len(), 40));
        assert_eq!(v.repository.last_object_id, u32::MAX);
        assert_eq!(v.shared.moments.len(), 1);
        all.push(0x55);
        assert_eq!(
            Instance::decode_prefix_with_items(&all, Limits::default())
                .unwrap()
                .1,
            all.len() - 1
        );
        assert_eq!(
            Instance::decode(&all, Limits::default()),
            Err(Error::Trailing)
        );
    }
    #[test]
    fn repository_and_composition_enforce_every_boundary_and_shared_item_budget() {
        for cut in 0..REPOSITORY_FIXTURE.len() {
            assert!(Repository::decode(&REPOSITORY_FIXTURE[..cut], Limits::default()).is_err());
            assert!(Repository::decode(
                REPOSITORY_FIXTURE,
                Limits {
                    max_bytes: cut,
                    max_items: 23
                }
            )
            .is_err());
        }
        for max_items in 0..23 {
            assert!(Repository::decode(
                REPOSITORY_FIXTURE,
                Limits {
                    max_bytes: REPOSITORY_FIXTURE.len(),
                    max_items
                }
            )
            .is_err());
        }
        for at in REPOSITORY_COUNTS {
            let mut b = REPOSITORY_FIXTURE.to_vec();
            b[*at..*at + 4].copy_from_slice(&0x7fffffffu32.to_le_bytes());
            assert!(Repository::decode(&b, Limits::default()).is_err());
        }
        let mut b = REPOSITORY_FIXTURE.to_vec();
        b[6..10].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Repository::decode(&b, Limits::default()),
            Err(Error::Version {
                expected: 0,
                actual: 99
            })
        );
        let mut all = FIXTURE.to_vec();
        all.extend(REPOSITORY_FIXTURE);
        for max_items in 0..40 {
            assert!(Instance::decode(
                &all,
                Limits {
                    max_bytes: all.len(),
                    max_items
                }
            )
            .is_err());
        }
        for cut in 0..all.len() {
            assert!(Instance::decode(&all[..cut], Limits::default()).is_err());
        }
    }
    const REPOSITORY_COUNTS: &[usize] = &[
        0, 14, 18, 23, 28, 32, 38, 50, 55, 73, 78, 82, 86, 104, 109, 113, 130, 134, 139, 143, 152,
        157, 177,
    ];
}
