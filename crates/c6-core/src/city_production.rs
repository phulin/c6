//! Bounded Linux XP2 City BuildQueue (v14/v16, node v2) and Buildings (v8/v10) wire.
//! Legacy14 omits non-wonder multiplier (native Q8 raw256) and salvage (raw0);
//! nested FeatureOverride/District modifier versions are independent14/16.
//!
//! Target SHA-256: a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! Original outer writer/read: 0x284c220/0x284c6e0; node: 0x284c5d0/0x284d070.
//! Nested FeatureOverride: 0x284c120/0x284c160; DistrictBuildingProductionModifier:
//! 0x284c1a0/0x284c1e0. Both nested versions are 16, not 1.
//!
//! This preserves ordered wire data, including unknown/duplicate definition
//! hashes and the two node type hashes. It does not perform registry remapping,
//! Reset, queue insertion, dirty tracking, production, or player decisions.
//! Omitted live fields are not established defaults or planning exclusions.
pub mod execution;

use crate::unit::{HashValue, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    TrailingBytes,
    ResourceLimit,
    Version {
        expected: i32,
        actual: i32,
    },
    NoncanonicalBool(u8),
    /// Tags 0..=3 require their corresponding typed payload.
    InvalidPayloadTag(i32),
    Localization(crate::map::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City production wire: {self:?}")
    }
}
impl std::error::Error for Error {}

/// OrderTypes and MilitaryFormationTypes are raw i32 providers at 0x2d500b0
/// and 0x2d4fc90. Unit/Building/District/Project providers instead write hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Order {
    Train {
        unit_hash: u32,
        military_formation: i32,
    },
    Construct {
        building_hash: u32,
    },
    Zone {
        district_hash: u32,
    },
    Advance {
        project_hash: u32,
    },
    /// Both original switch defaults serialize no order-specific payload,
    /// including NO_ORDER (-1), other known orders and unknown raw values.
    Other(i32),
}
impl Order {
    pub fn tag(self) -> i32 {
        match self {
            Self::Train { .. } => 0,
            Self::Construct { .. } => 1,
            Self::Zone { .. } => 2,
            Self::Advance { .. } => 3,
            Self::Other(tag) => tag,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueEntry {
    /// Outer dispatch hash; native Read consumes it without checking it.
    pub type_hash: u32,
    /// Inner node hash; native Read also consumes it without checking it.
    pub node_type_hash: u32,
    pub order: Order,
    pub flags: i32,
    pub request_id: u32,
    pub location: [i32; 2],
}
/// Original canonical writer emits this hash in both positions. The wire
/// codec preserves supplied hashes and does not silently canonicalize them.
pub const BUILD_TYPE_HASH: u32 = 0x2c0f4a46;

impl QueueEntry {
    /// Original MakeRequest(UnitTypes)2822300: ordinary single formation.
    pub fn train_request(unit_hash: u32) -> Self {
        Self {
            type_hash: BUILD_TYPE_HASH,
            node_type_hash: BUILD_TYPE_HASH,
            order: Order::Train {
                unit_hash,
                military_formation: 0,
            },
            flags: 0,
            request_id: 0,
            location: [-9999, -9999],
        }
    }
    /// External building selection without opaque native operation parameters.
    /// MakeRequest2822180 sets only the building/type. ClassParameters28229b0,
    /// BuildQueueNode285ed00 and CopyFromOperation284ddd0 establish these exact
    /// scalar and invalid-coordinate values; no native AI contract is created.
    pub fn construct_request(building_hash: u32) -> Self {
        Self {
            type_hash: BUILD_TYPE_HASH,
            node_type_hash: BUILD_TYPE_HASH,
            order: Order::Construct { building_hash },
            flags: 0,
            request_id: 0,
            location: [-9999, -9999],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureOverride {
    pub wire_version: i32,
    pub district_hash: u32,
    pub feature_hash: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictBuildingProductionModifier {
    pub wire_version: i32,
    pub district_hash: u32,
    pub modifier: i32,
}

/// FixedPoint fields retain their exact signed 8-fractional-bit storage.
/// Indexed arrays retain wire count/order; their counts are not registry sizes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildQueue {
    pub wire_version: i32,
    pub entries: Vec<QueueEntry>,
    /// YieldTypes uses raw i32 at 0x2d4fb70.
    pub yield_type: i32,
    pub wonder_production_multiplier: i32,
    pub river_district_production_multiplier: i32,
    pub river_wonder_production_multiplier: i32,
    pub building_production_modifier: i32,
    pub building_non_wonder_production_modifier: i32,
    pub city_unit_production_modifier: i32,
    pub district_production_modifier: i32,
    pub adjacent_city_river_building_production_multiplier: i32,
    pub adjacent_city_river_district_production_multiplier: i32,
    pub district_adjacent_natural_wonder_production_modifier: Vec<HashValue<i32>>,
    pub wonder_adjacent_natural_wonder_production_modifier: Vec<HashValue<i32>>,
    pub building_progress: Vec<HashValue<i32>>,
    pub construction_plot: Vec<HashValue<i16>>,
    pub unit_progress: Vec<HashValue<i32>>,
    pub district_progress: Vec<HashValue<i32>>,
    pub project_progress: Vec<HashValue<i32>>,
    pub zoning_plot: Vec<HashValue<i16>>,
    pub building_production_multiplier: Vec<HashValue<i32>>,
    pub unit_production_multiplier: Vec<HashValue<i32>>,
    pub district_production_multiplier: Vec<HashValue<i32>>,
    pub extra_unit_copy: Vec<HashValue<i32>>,
    pub features_valid_wonders: Vec<HashValue<bool>>,
    pub production_overflow: i32,
    pub production_salvage: i32,
    pub feature_overrides: Vec<FeatureOverride>,
    pub corps_army_discount: Vec<HashValue<i32>>,
    pub project_production_multiplier: Vec<HashValue<i32>>,
    pub previous_production_type_hash: u32,
    pub ignore_strategic_resource_requirements: bool,
    pub strategic_resource_requirement_modifier: i32,
    pub district_building_production_modifiers: Vec<DistrictBuildingProductionModifier>,
}

struct Budget {
    limits: Limits,
    items: usize,
}
impl Budget {
    fn count<T>(&mut self, count: usize) -> Result<Vec<T>, Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items
            || count
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
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    budget: Budget,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.budget.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let data = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(data.try_into().unwrap())
    }
    fn buildings_version(&mut self) -> Result<i32, Error> {
        let actual = i32::read(self)?;
        match actual {
            8 | 10 => Ok(actual),
            _ => Err(Error::Version {
                expected: 10,
                actual,
            }),
        }
    }
    fn build_queue_version(&mut self) -> Result<i32, Error> {
        let actual = i32::read(self)?;
        match actual {
            14 | 16 => Ok(actual),
            _ => Err(Error::Version {
                expected: 16,
                actual,
            }),
        }
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = i32::read(self)?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::Version { expected, actual })
        }
    }
    fn vector<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let count = u32::read(self)? as usize;
        let mut out = self.budget.count(count)?;
        for _ in 0..count {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn pairs<T: Scalar>(&mut self) -> Result<Vec<HashValue<T>>, Error> {
        self.vector(|r| {
            Ok(HashValue {
                definition_hash: u32::read(r)?,
                value: T::read(r)?,
            })
        })
    }
}
struct Writer {
    bytes: Vec<u8>,
    budget: Budget,
}
impl Writer {
    fn put(&mut self, data: &[u8]) -> Result<(), Error> {
        let size = self
            .bytes
            .len()
            .checked_add(data.len())
            .ok_or(Error::ResourceLimit)?;
        if size > self.budget.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.bytes
            .try_reserve(data.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.bytes.extend_from_slice(data);
        Ok(())
    }
    fn vector<T>(
        &mut self,
        values: &[T],
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        // Count validation uses no temporary allocation on encoding.
        self.budget.items = self
            .budget
            .items
            .checked_add(values.len())
            .ok_or(Error::ResourceLimit)?;
        if self.budget.items > self.budget.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        u32::try_from(values.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(self)?;
        for value in values {
            write(self, value)?;
        }
        Ok(())
    }
    fn pairs<T: Scalar>(&mut self, values: &[HashValue<T>]) -> Result<(), Error> {
        self.vector(values, |w, p| {
            p.definition_hash.write(w)?;
            p.value.write(w)
        })
    }
}
trait Scalar: Sized + Copy {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(self, w: &mut Writer) -> Result<(), Error>;
}
macro_rules! scalar {
    ($t:ty, $n:expr) => {
        impl Scalar for $t {
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_le_bytes(r.take::<$n>()?))
            }
            fn write(self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
        }
    };
}
scalar!(i32, 4);
scalar!(u32, 4);
scalar!(i16, 2);
impl Scalar for bool {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.take::<1>()?[0] {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(Error::NoncanonicalBool(b)),
        }
    }
    fn write(self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[u8::from(self)])
    }
}
impl QueueEntry {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let type_hash = u32::read(r)?;
        r.version(2)?;
        let node_type_hash = u32::read(r)?;
        let tag = i32::read(r)?;
        let flags = i32::read(r)?;
        let order = match tag {
            0 => Order::Train {
                unit_hash: u32::read(r)?,
                military_formation: i32::read(r)?,
            },
            1 => Order::Construct {
                building_hash: u32::read(r)?,
            },
            2 => Order::Zone {
                district_hash: u32::read(r)?,
            },
            3 => Order::Advance {
                project_hash: u32::read(r)?,
            },
            tag => Order::Other(tag),
        };
        Ok(Self {
            type_hash,
            node_type_hash,
            order,
            flags,
            request_id: u32::read(r)?,
            location: [i32::read(r)?, i32::read(r)?],
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.type_hash.write(w)?;
        2i32.write(w)?;
        self.node_type_hash.write(w)?;
        self.order.tag().write(w)?;
        self.flags.write(w)?;
        match self.order {
            Order::Train {
                unit_hash,
                military_formation,
            } => {
                unit_hash.write(w)?;
                military_formation.write(w)?;
            }
            Order::Construct { building_hash } => building_hash.write(w)?,
            Order::Zone { district_hash } => district_hash.write(w)?,
            Order::Advance { project_hash } => project_hash.write(w)?,
            Order::Other(tag) if (0..=3).contains(&tag) => {
                return Err(Error::InvalidPayloadTag(tag))
            }
            Order::Other(_) => {}
        }
        self.request_id.write(w)?;
        self.location[0].write(w)?;
        self.location[1].write(w)
    }
}

impl BuildQueue {
    pub const VERSION: i32 = 16;
    /// Decode only at an independently established structural boundary.
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, consumed, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, consumed))
    }
    /// Returns the actual aggregate dynamic-item charge for enclosing codecs.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            budget: Budget { limits, items: 0 },
        };
        let wire_version = r.build_queue_version()?;
        let value = Self {
            wire_version,
            entries: r.vector(QueueEntry::read)?,
            yield_type: i32::read(&mut r)?,
            wonder_production_multiplier: i32::read(&mut r)?,
            river_district_production_multiplier: i32::read(&mut r)?,
            river_wonder_production_multiplier: i32::read(&mut r)?,
            building_production_modifier: i32::read(&mut r)?,
            building_non_wonder_production_modifier: if wire_version == 16 {
                i32::read(&mut r)?
            } else {
                256
            },
            city_unit_production_modifier: i32::read(&mut r)?,
            district_production_modifier: i32::read(&mut r)?,
            adjacent_city_river_building_production_multiplier: i32::read(&mut r)?,
            adjacent_city_river_district_production_multiplier: i32::read(&mut r)?,
            district_adjacent_natural_wonder_production_modifier: r.pairs()?,
            wonder_adjacent_natural_wonder_production_modifier: r.pairs()?,
            building_progress: r.pairs()?,
            construction_plot: r.pairs()?,
            unit_progress: r.pairs()?,
            district_progress: r.pairs()?,
            project_progress: r.pairs()?,
            zoning_plot: r.pairs()?,
            building_production_multiplier: r.pairs()?,
            unit_production_multiplier: r.pairs()?,
            district_production_multiplier: r.pairs()?,
            extra_unit_copy: r.pairs()?,
            features_valid_wonders: r.pairs()?,
            production_overflow: i32::read(&mut r)?,
            production_salvage: if wire_version == 16 {
                i32::read(&mut r)?
            } else {
                0
            },
            feature_overrides: r.vector(|r| {
                let wire_version = r.build_queue_version()?;
                Ok(FeatureOverride {
                    wire_version,
                    district_hash: u32::read(r)?,
                    feature_hash: u32::read(r)?,
                })
            })?,
            corps_army_discount: r.pairs()?,
            project_production_multiplier: r.pairs()?,
            previous_production_type_hash: u32::read(&mut r)?,
            ignore_strategic_resource_requirements: bool::read(&mut r)?,
            strategic_resource_requirement_modifier: i32::read(&mut r)?,
            district_building_production_modifiers: r.vector(|r| {
                let wire_version = r.build_queue_version()?;
                Ok(DistrictBuildingProductionModifier {
                    wire_version,
                    district_hash: u32::read(r)?,
                    modifier: i32::read(r)?,
                })
            })?,
        };
        Ok((value, r.pos, r.budget.items))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, consumed) = Self::decode_prefix(bytes, limits)?;
        if consumed != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        Ok(value)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
    /// Returns bytes and their actual aggregate dynamic-item charge together.
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if self.wire_version != Self::VERSION
            || self
                .feature_overrides
                .iter()
                .any(|v| v.wire_version != Self::VERSION)
            || self
                .district_building_production_modifiers
                .iter()
                .any(|v| v.wire_version != Self::VERSION)
        {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let mut w = Writer {
            bytes: Vec::new(),
            budget: Budget { limits, items: 0 },
        };
        Self::VERSION.write(&mut w)?;
        w.vector(&self.entries, |w, v| v.write(w))?;
        self.yield_type.write(&mut w)?;
        self.wonder_production_multiplier.write(&mut w)?;
        self.river_district_production_multiplier.write(&mut w)?;
        self.river_wonder_production_multiplier.write(&mut w)?;
        self.building_production_modifier.write(&mut w)?;
        self.building_non_wonder_production_modifier.write(&mut w)?;
        self.city_unit_production_modifier.write(&mut w)?;
        self.district_production_modifier.write(&mut w)?;
        self.adjacent_city_river_building_production_multiplier
            .write(&mut w)?;
        self.adjacent_city_river_district_production_multiplier
            .write(&mut w)?;
        w.pairs(&self.district_adjacent_natural_wonder_production_modifier)?;
        w.pairs(&self.wonder_adjacent_natural_wonder_production_modifier)?;
        w.pairs(&self.building_progress)?;
        w.pairs(&self.construction_plot)?;
        w.pairs(&self.unit_progress)?;
        w.pairs(&self.district_progress)?;
        w.pairs(&self.project_progress)?;
        w.pairs(&self.zoning_plot)?;
        w.pairs(&self.building_production_multiplier)?;
        w.pairs(&self.unit_production_multiplier)?;
        w.pairs(&self.district_production_multiplier)?;
        w.pairs(&self.extra_unit_copy)?;
        w.pairs(&self.features_valid_wonders)?;
        self.production_overflow.write(&mut w)?;
        self.production_salvage.write(&mut w)?;
        w.vector(&self.feature_overrides, |w, v| {
            16i32.write(w)?;
            v.district_hash.write(w)?;
            v.feature_hash.write(w)
        })?;
        w.pairs(&self.corps_army_discount)?;
        w.pairs(&self.project_production_multiplier)?;
        self.previous_production_type_hash.write(&mut w)?;
        self.ignore_strategic_resource_requirements.write(&mut w)?;
        self.strategic_resource_requirement_modifier.write(&mut w)?;
        w.vector(&self.district_building_production_modifiers, |w, v| {
            16i32.write(w)?;
            v.district_hash.write(w)?;
            v.modifier.write(w)
        })?;
        Ok((w.bytes, w.budget.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Independently assembled source grammar; no native execution is claimed.
    fn word(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        fixture_versions(16, 16)
    }
    fn fixture_versions(version: u32, nested: u32) -> Vec<u8> {
        let mut b = Vec::new();
        word(&mut b, version);
        word(&mut b, 7);
        for (n, tag) in [0i32, 1, 2, 3, -1, 4, i32::MIN].into_iter().enumerate() {
            word(&mut b, BUILD_TYPE_HASH.wrapping_add(n as u32));
            word(&mut b, 2);
            word(&mut b, 0xf0000000 | n as u32);
            word(&mut b, tag as u32);
            word(&mut b, i32::MIN as u32);
            if (0..=3).contains(&tag) {
                word(&mut b, u32::MAX - n as u32);
            }
            if tag == 0 {
                word(&mut b, i32::MIN as u32);
            }
            word(&mut b, 0x80000000 | n as u32);
            word(&mut b, u32::MAX);
            word(&mut b, 0x7fffffff);
        }
        for i in 0..10 {
            if version == 14 && i == 5 {
                continue;
            }
            word(&mut b, 0x80000000 + i);
        }
        // All 13 initial indexed arrays populated, preserving duplicate keys.
        for array in 0..13 {
            word(&mut b, 2);
            for value in [0u32, 1] {
                word(&mut b, 0xdeadbeef);
                match array {
                    3 | 7 => b.extend_from_slice(&(0x8000u16 + value as u16).to_le_bytes()),
                    12 => b.push(value as u8),
                    _ => word(&mut b, 0x80000000 + value),
                }
            }
        }
        word(&mut b, 0x80000001); // overflow
        if version == 16 {
            word(&mut b, 0x7fffffff);
        } // salvage
        word(&mut b, 1); // FeatureOverride vector
        for value in [nested, u32::MAX, 0x12345678] {
            word(&mut b, value);
        }
        for _ in 0..2 {
            // CorpsArmyDiscount and Project multiplier
            word(&mut b, 1);
            word(&mut b, 0xdeadbeef);
            word(&mut b, u32::MAX);
        }
        word(&mut b, 0xfedcba98);
        b.push(1);
        word(&mut b, 0x87654321);
        word(&mut b, 1);
        for value in [nested, 0xdeadbeef, 0x80000000] {
            word(&mut b, value);
        }
        b
    }
    #[test]
    fn buildqueue14_q8_one_and_salvage_zero_preserve_remaining_grammar() {
        let mut bytes = fixture_versions(14, 14);
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 37,
        };
        let (value, used, items) = BuildQueue::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 37));
        assert_eq!(value.wire_version, 14);
        assert_eq!(value.building_non_wonder_production_modifier, 256);
        assert_eq!(value.production_salvage, 0);
        let current = BuildQueue::decode(&fixture(), Limits::default()).unwrap();
        assert_eq!(value.entries, current.entries);
        assert_eq!(value.building_progress, current.building_progress);
        assert_eq!(value.production_overflow, current.production_overflow);
        assert_eq!(
            value.strategic_resource_requirement_modifier,
            current.strategic_resource_requirement_modifier
        );
        assert_eq!(value.feature_overrides[0].wire_version, 14);
        assert_eq!(
            value.district_building_production_modifiers[0].wire_version,
            14
        );
        assert_eq!(value.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(
                BuildQueue::decode_prefix(&bytes[..n], limits).is_err(),
                "{n}"
            );
        }
        assert_eq!(
            BuildQueue::decode_prefix(
                &bytes,
                Limits {
                    max_items: 36,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        for version in [13u32, 15, 17] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                BuildQueue::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 16,
                    actual: version as i32
                })
            );
        }
        for (outer, nested) in [(14, 16), (16, 14)] {
            let v =
                BuildQueue::decode(&fixture_versions(outer, nested), Limits::default()).unwrap();
            assert_eq!(v.wire_version, outer as i32);
            assert_eq!(v.feature_overrides[0].wire_version, nested as i32);
            assert_eq!(
                v.district_building_production_modifiers[0].wire_version,
                nested as i32
            );
        }
    }
    #[test]
    fn charged_prefix_budget_composition() {
        let bytes = fixture();
        let mut carrier = bytes.clone();
        carrier.extend_from_slice(&[99; 20]);
        let limits = Limits {
            max_bytes: bytes.len(),
            max_items: 37,
        };
        let (value, consumed, items) =
            BuildQueue::decode_prefix_with_items(&carrier, limits).unwrap();
        assert_eq!((consumed, items), (bytes.len(), 37));
        assert_eq!(
            value.encode_with_items(limits).unwrap(),
            (bytes.clone(), 37)
        );
        let exhausted = Limits {
            max_items: 37 - 1,
            ..limits
        };
        assert!(BuildQueue::decode_prefix_with_items(&carrier, exhausted).is_err());
        assert!(value.encode_with_items(exhausted).is_err());
    }
    #[test]
    fn current_source_grammar_and_raw_branches() {
        let bytes = fixture();
        let value = BuildQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(value.entries.len(), 7);
        assert_eq!(
            value.entries[0].order,
            Order::Train {
                unit_hash: u32::MAX,
                military_formation: i32::MIN
            }
        );
        assert_eq!(
            value.entries[1].order,
            Order::Construct {
                building_hash: u32::MAX - 1
            }
        );
        assert_eq!(
            value.entries[2].order,
            Order::Zone {
                district_hash: u32::MAX - 2
            }
        );
        assert_eq!(
            value.entries[3].order,
            Order::Advance {
                project_hash: u32::MAX - 3
            }
        );
        assert_eq!(value.entries[6].order, Order::Other(i32::MIN));
        assert_eq!(value.entries[0].location, [-1, i32::MAX]);
        assert_eq!(value.yield_type, i32::MIN);
        assert_eq!(value.construction_plot[1].value, -32767);
        assert!(!value.features_valid_wonders[0].value);
        assert!(value.features_valid_wonders[1].value);
        assert_eq!(value.feature_overrides[0].feature_hash, 0x12345678);
        assert_eq!(
            value.district_building_production_modifiers[0].modifier,
            i32::MIN
        );
        assert_eq!(value.encode(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn truncation_versions_and_prefix() {
        let bytes = fixture();
        for length in 0..bytes.len() {
            assert!(
                BuildQueue::decode(&bytes[..length], Limits::default()).is_err(),
                "{length}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(99);
        assert_eq!(
            BuildQueue::decode(&trailing, Limits::default()),
            Err(Error::TrailingBytes)
        );
        assert_eq!(
            BuildQueue::decode_prefix(&trailing, Limits::default())
                .unwrap()
                .1,
            bytes.len()
        );
        for (offset, expected) in [(0, 16), (12, 2), (bytes.len() - 12, 16)] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&1i32.to_le_bytes());
            assert_eq!(
                BuildQueue::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 1
                })
            );
        }
    }
    #[test]
    fn malformed_bool_payload_and_bounded_resources() {
        let bytes = fixture();
        let mut bad = bytes.clone();
        let offset = bad.len() - 21;
        bad[offset] = 2;
        assert_eq!(
            BuildQueue::decode(&bad, Limits::default()),
            Err(Error::NoncanonicalBool(2))
        );
        let mut value = BuildQueue::decode(&bytes, Limits::default()).unwrap();
        value.entries[0].order = Order::Other(0);
        assert_eq!(
            value.encode(Limits::default()),
            Err(Error::InvalidPayloadTag(0))
        );
        let value = BuildQueue::decode(&bytes, Limits::default()).unwrap();
        for limits in [
            Limits {
                max_items: 6,
                max_bytes: 100000,
            },
            Limits {
                max_items: 100000,
                max_bytes: bytes.len() - 1,
            },
        ] {
            assert_eq!(
                BuildQueue::decode(&bytes, limits),
                Err(Error::ResourceLimit)
            );
            assert_eq!(value.encode(limits), Err(Error::ResourceLimit));
        }
        // A tiny wire record cannot force an unbounded allocation.
        let mut bad = vec![];
        word(&mut bad, 16);
        word(&mut bad, u32::MAX);
        assert_eq!(
            BuildQueue::decode(&bad, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}

/// Buildings v10's explicit building/yield tuples use a Yield definition hash,
/// unlike BuildQueue.yield_type and YieldChangeFromFeatures.yield_type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildingYieldModifier {
    pub building_hash: u32,
    pub yield_hash: u32,
    pub amount: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreatWorkSlot {
    pub wire_version: i32,
    /// GreatWorkSlotTypes uses raw i32 (0x2d4ff30), not a definition hash.
    pub slot_type: i32,
    /// GreatWorkListIndex uses raw i32 (0x2d4fff0).
    pub great_work_index: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatWorkBuilding {
    pub building_hash: u32,
    pub slots: Vec<GreatWorkSlot>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtraGreatWorkSlots {
    pub building_hash: u32,
    pub slot_type: i32,
    pub count: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YieldChangeFromFeatures {
    pub feature_hash: u32,
    /// This nested provider writes a raw YieldTypes i32, unlike yield tuples.
    pub yield_type: i32,
    pub amount: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingName {
    pub building_hash: u32,
    pub name: crate::map::LocalizedText,
}
/// Original W/R 0x2862860/0x2862c70. Read invokes Reset and performs registry
/// mapping/filtering; this codec preserves complete wire data separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Buildings {
    pub wire_version: i32,
    pub building_locations: Vec<HashValue<i16>>,
    pub pillaged: Vec<HashValue<bool>>,
    /// Original vector<short> indexed by Building; values are raw i16.
    pub era_created: Vec<HashValue<i16>>,
    pub num_units_supported: Vec<HashValue<i16>>,
    pub yield_modifiers: Vec<BuildingYieldModifier>,
    pub yield_changes: Vec<BuildingYieldModifier>,
    pub yield_changes_city_states: Vec<BuildingYieldModifier>,
    pub wonder_yield_changes: Vec<HashValue<i32>>,
    pub great_work_buildings: Vec<GreatWorkBuilding>,
    pub extra_great_works: Vec<ExtraGreatWorkSlots>,
    pub yield_change_from_features: Vec<YieldChangeFromFeatures>,
    pub building_name_overrides: Vec<BuildingName>,
}
fn localization_limits(limits: Limits, bytes: usize, items: usize) -> crate::map::Limits {
    crate::map::Limits {
        max_bytes: limits.max_bytes.saturating_sub(bytes),
        max_items: limits.max_items.saturating_sub(items),
        max_allocation: limits.max_bytes,
        max_depth: 64,
    }
}
impl Reader<'_> {
    fn building_yields(&mut self) -> Result<Vec<BuildingYieldModifier>, Error> {
        self.vector(|r| {
            Ok(BuildingYieldModifier {
                building_hash: u32::read(r)?,
                yield_hash: u32::read(r)?,
                amount: i32::read(r)?,
            })
        })
    }
    fn localized_text(&mut self) -> Result<crate::map::LocalizedText, Error> {
        let limits = localization_limits(self.budget.limits, self.pos, self.budget.items);
        let end = self
            .bytes
            .len()
            .min(self.pos.saturating_add(limits.max_bytes));
        let (value, consumed, items) =
            crate::map::LocalizedText::decode_prefix_with_items(&self.bytes[self.pos..end], limits)
                .map_err(Error::Localization)?;
        self.pos = self.pos.checked_add(consumed).ok_or(Error::ResourceLimit)?;
        self.budget.items = self
            .budget
            .items
            .checked_add(items)
            .ok_or(Error::ResourceLimit)?;
        if self.budget.items > self.budget.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(value)
    }
}
impl Writer {
    fn building_yields(&mut self, values: &[BuildingYieldModifier]) -> Result<(), Error> {
        self.vector(values, |w, v| {
            v.building_hash.write(w)?;
            v.yield_hash.write(w)?;
            v.amount.write(w)
        })
    }
    fn localized_text(&mut self, value: &crate::map::LocalizedText) -> Result<(), Error> {
        let limits = localization_limits(self.budget.limits, self.bytes.len(), self.budget.items);
        let (bytes, items) = value
            .encode_with_items(limits)
            .map_err(Error::Localization)?;
        self.budget.items = self
            .budget
            .items
            .checked_add(items)
            .ok_or(Error::ResourceLimit)?;
        if self.budget.items > self.budget.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        self.put(&bytes)
    }
}
/// Native Reset clears these version-omitted vectors; no constructor or
/// arbitrary prior-state baseline is inferred from their absence on the wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildingsOmittedListEffect {
    Present,
    EmptyAfterNativeReset,
}
impl Buildings {
    pub fn omitted_lists_effect(&self) -> BuildingsOmittedListEffect {
        if self.wire_version == 8 {
            BuildingsOmittedListEffect::EmptyAfterNativeReset
        } else {
            BuildingsOmittedListEffect::Present
        }
    }
    pub const VERSION: i32 = 10;
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, consumed, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, consumed))
    }
    /// Returns the actual aggregate dynamic-item charge for enclosing codecs.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            budget: Budget { limits, items: 0 },
        };
        let wire_version = r.buildings_version()?;
        let value = Self {
            wire_version,
            building_locations: r.pairs()?,
            pillaged: r.pairs()?,
            era_created: r.pairs()?,
            num_units_supported: r.pairs()?,
            yield_modifiers: r.building_yields()?,
            yield_changes: r.building_yields()?,
            yield_changes_city_states: r.building_yields()?,
            wonder_yield_changes: r.pairs()?,
            great_work_buildings: r.vector(|r| {
                Ok(GreatWorkBuilding {
                    building_hash: u32::read(r)?,
                    slots: r.vector(|r| {
                        let wire_version = r.buildings_version()?;
                        Ok(GreatWorkSlot {
                            wire_version,
                            slot_type: i32::read(r)?,
                            great_work_index: i32::read(r)?,
                        })
                    })?,
                })
            })?,
            extra_great_works: r.vector(|r| {
                Ok(ExtraGreatWorkSlots {
                    building_hash: u32::read(r)?,
                    slot_type: i32::read(r)?,
                    count: i32::read(r)?,
                })
            })?,
            yield_change_from_features: if wire_version == 8 {
                Vec::new()
            } else {
                r.vector(|r| {
                    r.version(Self::VERSION)?;
                    Ok(YieldChangeFromFeatures {
                        feature_hash: u32::read(r)?,
                        yield_type: i32::read(r)?,
                        amount: i32::read(r)?,
                    })
                })?
            },
            building_name_overrides: if wire_version == 8 {
                Vec::new()
            } else {
                r.vector(|r| {
                    r.version(Self::VERSION)?;
                    Ok(BuildingName {
                        building_hash: u32::read(r)?,
                        name: r.localized_text()?,
                    })
                })?
            },
        };
        Ok((value, r.pos, r.budget.items))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, consumed) = Self::decode_prefix(bytes, limits)?;
        if consumed != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        Ok(value)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
    /// Returns bytes and their actual aggregate dynamic-item charge together.
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if self.wire_version != Self::VERSION
            || self
                .great_work_buildings
                .iter()
                .flat_map(|v| &v.slots)
                .any(|v| v.wire_version != Self::VERSION)
        {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let mut w = Writer {
            bytes: Vec::new(),
            budget: Budget { limits, items: 0 },
        };
        Self::VERSION.write(&mut w)?;
        w.pairs(&self.building_locations)?;
        w.pairs(&self.pillaged)?;
        w.pairs(&self.era_created)?;
        w.pairs(&self.num_units_supported)?;
        w.building_yields(&self.yield_modifiers)?;
        w.building_yields(&self.yield_changes)?;
        w.building_yields(&self.yield_changes_city_states)?;
        w.pairs(&self.wonder_yield_changes)?;
        w.vector(&self.great_work_buildings, |w, v| {
            v.building_hash.write(w)?;
            w.vector(&v.slots, |w, v| {
                Self::VERSION.write(w)?;
                v.slot_type.write(w)?;
                v.great_work_index.write(w)
            })
        })?;
        w.vector(&self.extra_great_works, |w, v| {
            v.building_hash.write(w)?;
            v.slot_type.write(w)?;
            v.count.write(w)
        })?;
        w.vector(&self.yield_change_from_features, |w, v| {
            Self::VERSION.write(w)?;
            v.feature_hash.write(w)?;
            v.yield_type.write(w)?;
            v.amount.write(w)
        })?;
        w.vector(&self.building_name_overrides, |w, v| {
            Self::VERSION.write(w)?;
            v.building_hash.write(w)?;
            w.localized_text(&v.name)
        })?;
        Ok((w.bytes, w.budget.items))
    }
}

#[cfg(test)]
mod buildings_tests {
    use super::*;
    fn word(b: &mut Vec<u8>, v: u32) {
        b.extend_from_slice(&v.to_le_bytes());
    }
    fn localized(b: &mut Vec<u8>, nested: bool) {
        word(b, u32::MAX);
        b.push(1);
        word(b, 3);
        b.extend_from_slice(b"a\0\xff");
        b.push(if nested { 1 } else { 0 });
        if nested {
            b.push(5);
            localized(b, false);
        }
        b.push(1);
        word(b, 0);
        b.push(1);
        word(b, 1);
        b.push(0xff);
    }
    // Hand-written source grammar, including each nonempty nested branch.
    fn fixture() -> (Vec<u8>, usize, usize) {
        let mut b = Vec::new();
        word(&mut b, 10);
        for index in 0..4 {
            word(&mut b, 2);
            for value in [0, 1] {
                word(&mut b, 0xdeadbeef);
                if index == 1 {
                    b.push(value);
                } else {
                    b.extend_from_slice(&(0x8000u16 + value as u16).to_le_bytes());
                }
            }
        }
        for _ in 0..3 {
            word(&mut b, 1);
            word(&mut b, u32::MAX);
            word(&mut b, 0xfedcba98);
            word(&mut b, 0x80000000);
        }
        word(&mut b, 1);
        word(&mut b, 0x87654321);
        word(&mut b, 0x7fffffff);
        word(&mut b, 1);
        word(&mut b, 0xdeadbeef);
        word(&mut b, 2);
        let slot_version = b.len();
        for index in [0x80000000, u32::MAX] {
            word(&mut b, 10);
            word(&mut b, 0x80000000);
            word(&mut b, index);
        }
        word(&mut b, 1);
        word(&mut b, 0xdeadbeef);
        word(&mut b, 0x80000001);
        word(&mut b, u32::MAX);
        word(&mut b, 1);
        word(&mut b, 10);
        word(&mut b, 0xfedcba98);
        word(&mut b, 0x80000000);
        word(&mut b, 0x7fffffff);
        word(&mut b, 2);
        let name_version = b.len();
        for _ in 0..2 {
            word(&mut b, 10);
            word(&mut b, 0xdeadbeef);
            localized(&mut b, true);
        }
        (b, slot_version, name_version)
    }
    #[test]
    fn buildings8_omitted_vectors_are_empty_after_reset_and_slots_independent() {
        let (mut bytes, slot, names) = fixture();
        // Literal fixture's final fields: one16-byte feature-yield record with
        // count, then the name count before `names`. Both are absent in8.
        bytes.truncate(names - 24);
        bytes[..4].copy_from_slice(&8i32.to_le_bytes());
        for at in [slot, slot + 12] {
            bytes[at..at + 4].copy_from_slice(&8i32.to_le_bytes());
        }
        let end = bytes.len();
        bytes.extend([0xaa, 0xbb]);
        let limits = Limits {
            max_bytes: end,
            max_items: 16,
        };
        let (value, used, items) = Buildings::decode_prefix_with_items(&bytes, limits).unwrap();
        assert_eq!((used, items), (end, 16));
        assert_eq!(value.wire_version, 8);
        assert!(value.yield_change_from_features.is_empty());
        assert!(value.building_name_overrides.is_empty());
        assert_eq!(
            value.omitted_lists_effect(),
            BuildingsOmittedListEffect::EmptyAfterNativeReset
        );
        assert_eq!(value.great_work_buildings[0].slots[0].wire_version, 8);
        assert_eq!(
            value.great_work_buildings[0].slots[0].great_work_index,
            i32::MIN
        );
        assert_eq!(value.extra_great_works[0].count, -1);
        assert_eq!(value.encode(limits), Err(Error::UnsupportedLegacyWrite));
        for n in 0..end {
            assert!(
                Buildings::decode_prefix(&bytes[..n], limits).is_err(),
                "{n}"
            );
        }
        assert_eq!(
            Buildings::decode_prefix(
                &bytes,
                Limits {
                    max_items: 15,
                    ..limits
                }
            ),
            Err(Error::ResourceLimit)
        );
        for at in [slot, slot + 12] {
            bytes[at..at + 4].copy_from_slice(&10i32.to_le_bytes());
        }
        let v = Buildings::decode_prefix(&bytes, limits).unwrap().0;
        assert_eq!(v.wire_version, 8);
        assert_eq!(v.great_work_buildings[0].slots[0].wire_version, 10);
        for version in [7i32, 9, 11] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Buildings::decode_prefix(&bad, limits),
                Err(Error::Version {
                    expected: 10,
                    actual: version
                })
            );
        }
    }
    #[test]
    fn charged_prefix_budget_composition() {
        let bytes = fixture().0;
        let mut carrier = bytes.clone();
        carrier.extend_from_slice(&[99; 20]);
        let limits = Limits {
            max_bytes: bytes.len(),
            max_items: 41,
        };
        let (value, consumed, items) =
            Buildings::decode_prefix_with_items(&carrier, limits).unwrap();
        assert_eq!((consumed, items), (bytes.len(), 41));
        assert_eq!(
            value.encode_with_items(limits).unwrap(),
            (bytes.clone(), 41)
        );
        let exhausted = Limits {
            max_items: 41 - 1,
            ..limits
        };
        assert!(Buildings::decode_prefix_with_items(&carrier, exhausted).is_err());
        assert!(value.encode_with_items(exhausted).is_err());
    }
    #[test]
    fn complete_source_buildings_nonempty_grammar() {
        let (bytes, _, _) = fixture();
        let value = Buildings::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(value.building_locations[0].value, i16::MIN);
        assert_eq!(value.era_created[1].value, -32767);
        assert!(!value.pillaged[0].value);
        assert!(value.pillaged[1].value);
        assert_eq!(value.yield_modifiers[0].yield_hash, 0xfedcba98);
        assert_eq!(
            value.great_work_buildings[0].slots[0].great_work_index,
            i32::MIN
        );
        assert_eq!(value.yield_change_from_features[0].yield_type, i32::MIN);
        assert_eq!(value.extra_great_works[0].slot_type, i32::MIN + 1);
        assert_eq!(value.building_name_overrides[0].name.base, b"a\0\xff");
        assert_eq!(value.encode(Limits::default()).unwrap(), bytes);
        for length in 0..bytes.len() {
            assert!(
                Buildings::decode(&bytes[..length], Limits::default()).is_err(),
                "{length}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            Buildings::decode_prefix(&trailing, Limits::default())
                .unwrap()
                .1,
            bytes.len()
        );
        assert_eq!(
            Buildings::decode(&trailing, Limits::default()),
            Err(Error::TrailingBytes)
        );
    }
    #[test]
    fn buildings_versions_and_aggregate_budget() {
        let (bytes, slot_version, name_version) = fixture();
        for offset in [0, slot_version, name_version] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&1i32.to_le_bytes());
            assert_eq!(
                Buildings::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 10,
                    actual: 1
                })
            );
        }
        let value = Buildings::decode(&bytes, Limits::default()).unwrap();
        // Overall budget crosses recursive localized names; each individual
        // component remains small enough to pass this policy in isolation.
        let limits = Limits {
            max_bytes: 100000,
            max_items: 25,
        };
        assert!(Buildings::decode(&bytes, limits).is_err());
        assert!(value.encode(limits).is_err());
        let limits = Limits {
            max_bytes: bytes.len() - 1,
            max_items: 100000,
        };
        assert!(Buildings::decode(&bytes, limits).is_err());
        assert!(value.encode(limits).is_err());
        let mut bad = vec![];
        word(&mut bad, 10);
        word(&mut bad, u32::MAX);
        assert_eq!(
            Buildings::decode(&bad, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
