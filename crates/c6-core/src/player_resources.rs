//! Bounded Linux XP2 Player.Resources v12/v16 wire transport, including nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 16;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Resources wire: {self:?}")
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
        // Check each allocation's addressable layout; wire bytes are a separate budget.
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        if bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if !matches!(actual, 12 | 16) {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(actual)
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
                if $version { r.version()?; }
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
impl Wire for bool {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match r.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::InvalidBool(x)),
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[u8::from(*self)])
    }
}
/// Ordered definition hash/value pair; duplicates and unknown hashes are retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceEntry<T> {
    pub resource_hash: u32,
    pub value: T,
}
impl<T: Wire> Wire for ResourceEntry<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            resource_hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.resource_hash.write(w)?;
        self.value.write(w)
    }
}
record!(CityResource,false,{city_id:u32,resource_hash:u32});
/// Player.Resources12/16 wire projection. Raw reserved values remain intact;
/// native Read defaults and its clamp are described separately, not applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resources {
    pub wire_version: u32,
    pub free_resource_visibility: Vec<ResourceEntry<bool>>,
    pub extra_amenities_per_owned_luxury_resource: i32,
    pub extra_amenities_per_owned_bonus_resource: i32,
    pub full_access_one_strategic: bool,
    pub prevent_harvest_resource: i32,
    pub resource_ownership: Vec<ResourceEntry<Vec<i32>>>,
    pub accumulated_resource_per_turn_q8: Vec<ResourceEntry<i32>>,
    pub luxury_allocations: Vec<CityResource>,
    pub reserved_resources: Vec<ResourceEntry<i32>>,
    pub banned_resources: Vec<ResourceEntry<i32>>,
    pub no_cap_resources: Vec<ResourceEntry<i32>>,
    pub export_resource_extracted: Vec<ResourceEntry<i32>>,
    pub import_resource_extracted: Vec<ResourceEntry<i32>>,
    pub free_resources_extracted: Vec<ResourceEntry<i32>>,
    pub extra_accumulation_modifier: Vec<ResourceEntry<i32>>,
    pub resource_stockpile_cap_modifier: Vec<ResourceEntry<i32>>,
    pub unit_resource_demand_per_turn_q8: Vec<ResourceEntry<i32>>,
    pub power_resource_demand_per_turn_q8: Vec<ResourceEntry<i32>>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceReadEffect {
    /// Zero at each actual Resource definition collection index; registry/count,
    /// dense ordering and unrepresented slots are not inferred from wire data.
    ZeroAtActualResourceIndexes,
    /// After native hash overlay, clamp negatives across the entire logical
    /// reserved-resource vector. This does not normalize the wire pairs here.
    ClampNegativeReservedEntriesToZero,
}
impl Resources {
    /// At12: extra accumulation, stockpile cap, unit demand and power demand.
    pub fn omitted_resource_arrays_read_effect(&self) -> Option<ResourceReadEffect> {
        (self.wire_version == 12).then_some(ResourceReadEffect::ZeroAtActualResourceIndexes)
    }
    pub fn reserved_resources_read_effect(&self) -> ResourceReadEffect {
        ResourceReadEffect::ClampNegativeReservedEntriesToZero
    }
}
impl Wire for Resources {
    const MIN: usize = 4
        + <Vec<ResourceEntry<bool>> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <bool as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<ResourceEntry<Vec<i32>>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<CityResource> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN
        + <Vec<ResourceEntry<i32>> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = r.version()?;
        Ok(Self {
            wire_version,
            free_resource_visibility: <Vec<ResourceEntry<bool>> as Wire>::read(r)?,
            extra_amenities_per_owned_luxury_resource: <i32 as Wire>::read(r)?,
            extra_amenities_per_owned_bonus_resource: <i32 as Wire>::read(r)?,
            full_access_one_strategic: <bool as Wire>::read(r)?,
            prevent_harvest_resource: <i32 as Wire>::read(r)?,
            resource_ownership: <Vec<ResourceEntry<Vec<i32>>> as Wire>::read(r)?,
            accumulated_resource_per_turn_q8: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            luxury_allocations: <Vec<CityResource> as Wire>::read(r)?,
            reserved_resources: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            banned_resources: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            no_cap_resources: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            export_resource_extracted: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            import_resource_extracted: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            free_resources_extracted: <Vec<ResourceEntry<i32>> as Wire>::read(r)?,
            extra_accumulation_modifier: if wire_version == 16 {
                <Vec<ResourceEntry<i32>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            resource_stockpile_cap_modifier: if wire_version == 16 {
                <Vec<ResourceEntry<i32>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            unit_resource_demand_per_turn_q8: if wire_version == 16 {
                <Vec<ResourceEntry<i32>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            power_resource_demand_per_turn_q8: if wire_version == 16 {
                <Vec<ResourceEntry<i32>> as Wire>::read(r)?
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
        self.free_resource_visibility.write(w)?;
        self.extra_amenities_per_owned_luxury_resource.write(w)?;
        self.extra_amenities_per_owned_bonus_resource.write(w)?;
        self.full_access_one_strategic.write(w)?;
        self.prevent_harvest_resource.write(w)?;
        self.resource_ownership.write(w)?;
        self.accumulated_resource_per_turn_q8.write(w)?;
        self.luxury_allocations.write(w)?;
        self.reserved_resources.write(w)?;
        self.banned_resources.write(w)?;
        self.no_cap_resources.write(w)?;
        self.export_resource_extracted.write(w)?;
        self.import_resource_extracted.write(w)?;
        self.free_resources_extracted.write(w)?;
        self.extra_accumulation_modifier.write(w)?;
        self.resource_stockpile_cap_modifier.write(w)?;
        self.unit_resource_demand_per_turn_q8.write(w)?;
        self.power_resource_demand_per_turn_q8.write(w)?;
        Ok(())
    }
}

impl Resources {
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
        let v = Self::read(&mut r)?;
        Ok((v, r.pos, r.items))
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
    fn specimen() -> Resources {
        Resources {
            wire_version: VERSION,
            free_resource_visibility: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: true,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: false,
                },
            ],
            extra_amenities_per_owned_luxury_resource: i32::MIN,
            extra_amenities_per_owned_bonus_resource: i32::MIN,
            full_access_one_strategic: true,
            prevent_harvest_resource: i32::MIN,
            resource_ownership: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: vec![i32::MIN, -1, i32::MAX],
                },
                ResourceEntry {
                    resource_hash: 0,
                    value: vec![],
                },
            ],
            accumulated_resource_per_turn_q8: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            luxury_allocations: vec![CityResource {
                city_id: 0xffff_fffe,
                resource_hash: 0x8000_0001,
            }],
            reserved_resources: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            banned_resources: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            no_cap_resources: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            export_resource_extracted: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            import_resource_extracted: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            free_resources_extracted: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            extra_accumulation_modifier: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            resource_stockpile_cap_modifier: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            unit_resource_demand_per_turn_q8: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
            power_resource_demand_per_turn_q8: vec![
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MIN,
                },
                ResourceEntry {
                    resource_hash: 0x8000_0001,
                    value: i32::MAX,
                },
            ],
        }
    }
    #[test]
    fn resources12_literal_omits_late_arrays_without_normalizing_wire_entries() {
        fn words(b: &mut Vec<u8>, values: &[i32]) {
            for v in values {
                b.extend(v.to_le_bytes());
            }
        }
        let hash = 0x87654321u32 as i32;
        let mut b = Vec::new();
        words(&mut b, &[12, 2, hash]);
        b.push(1);
        words(&mut b, &[hash]);
        b.push(0);
        words(&mut b, &[-11, 12]);
        b.push(1);
        words(&mut b, &[-13]);
        words(&mut b, &[1, hash, 3, i32::MIN, 0, i32::MAX]); // ownership row
        words(&mut b, &[1, hash, -257]); // accumulated Q8
        words(&mut b, &[1, 0xab010007u32 as i32, hash]); // luxury allocation
        words(&mut b, &[2, hash, -3, hash, 7]); // raw reserved pairs incl negative
        words(&mut b, &[1, hash, -5]); // banned
        words(&mut b, &[0]); // no-cap
        words(&mut b, &[1, hash, 10, 1, hash, -11, 1, hash, 12]); // export/import/free
                                                                  // Four arrays introduced13/14/15 have no wire counts at12.
        let end = b.len();
        b.extend([0xaa, 0xbb]);
        let exact = Limits {
            max_bytes: end,
            max_items: 14,
        };
        let (v, used, items) = Resources::decode_prefix_with_items(&b, exact).unwrap();
        assert_eq!((used, items, v.wire_version), (end, 14, 12));
        assert_eq!(
            (
                v.extra_amenities_per_owned_luxury_resource,
                v.extra_amenities_per_owned_bonus_resource
            ),
            (-11, 12)
        );
        assert!(v.full_access_one_strategic);
        assert_eq!(v.prevent_harvest_resource, -13);
        assert_eq!(v.resource_ownership[0].value, [i32::MIN, 0, i32::MAX]);
        assert_eq!(v.accumulated_resource_per_turn_q8[0].value, -257);
        assert_eq!(v.luxury_allocations[0].city_id, 0xab010007);
        assert_eq!(
            v.reserved_resources
                .iter()
                .map(|p| (p.resource_hash, p.value))
                .collect::<Vec<_>>(),
            vec![(hash as u32, -3), (hash as u32, 7)]
        );
        assert_eq!(v.banned_resources[0].value, -5);
        assert_eq!(
            (
                v.export_resource_extracted[0].value,
                v.import_resource_extracted[0].value,
                v.free_resources_extracted[0].value
            ),
            (10, -11, 12)
        );
        assert!(
            v.extra_accumulation_modifier.is_empty()
                && v.resource_stockpile_cap_modifier.is_empty()
        );
        assert!(
            v.unit_resource_demand_per_turn_q8.is_empty()
                && v.power_resource_demand_per_turn_q8.is_empty()
        );
        assert_eq!(
            v.omitted_resource_arrays_read_effect(),
            Some(ResourceReadEffect::ZeroAtActualResourceIndexes)
        );
        assert_eq!(
            v.reserved_resources_read_effect(),
            ResourceReadEffect::ClampNegativeReservedEntriesToZero
        );
        let mut w = Writer {
            data: vec![0xaa],
            items: 0,
            limits: exact,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert_eq!(w.data, [0xaa]);
        for n in 0..end {
            assert!(Resources::decode_prefix(&b[..n], exact).is_err(), "{n}");
        }
        assert_eq!(
            Resources::decode_prefix(
                &b,
                Limits {
                    max_items: 13,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut bad = b.clone();
        bad[12] = 2;
        assert_eq!(
            Resources::decode_prefix(&bad, exact),
            Err(Error::InvalidBool(2))
        );
        let mut unknown = b.clone();
        unknown[..4].copy_from_slice(&13u32.to_le_bytes());
        assert_eq!(
            Resources::decode_prefix(&unknown, exact),
            Err(Error::Version {
                expected: 16,
                actual: 13
            })
        );
    }
    #[test]
    fn nonempty_roundtrip_and_prefix() {
        let v = specimen();
        let (b, i) = v.encode_with_items(limits()).unwrap();
        let (mut suffix, n, items) = Resources::decode_prefix_with_items(&b, limits()).unwrap();
        assert_eq!(suffix, v);
        assert_eq!(n, b.len());
        assert_eq!(v.wire_version, 16);
        assert_eq!(v.omitted_resource_arrays_read_effect(), None);
        assert_eq!(
            v.reserved_resources_read_effect(),
            ResourceReadEffect::ClampNegativeReservedEntriesToZero
        );
        assert_eq!(i, items);
        assert_eq!(items, 30);
        suffix.reserved_resources[0].value = -23;
        assert_eq!(
            Resources::decode(&suffix.encode(limits()).unwrap(), limits()).unwrap(),
            suffix
        );
        let mut extended = b.clone();
        extended.push(99);
        assert_eq!(Resources::decode(&extended, limits()), Err(Error::Trailing));
        assert_eq!(
            Resources::decode_prefix(&extended, limits()).unwrap(),
            (v, b.len())
        );
    }
    #[test]
    fn every_prefix_truncates() {
        let b = specimen().encode(limits()).unwrap();
        for n in 0..b.len() {
            assert_eq!(
                Resources::decode(&b[..n], limits()),
                Err(Error::Truncated),
                "prefix {n}"
            );
        }
    }
    #[test]
    fn exact_and_one_short_budgets() {
        let v = specimen();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Resources::decode(&b, exact).unwrap(), v);
        assert_eq!(v.encode(exact).unwrap(), b);
        for short in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(Resources::decode(&b, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn strict_version_and_bools() {
        let mut b = specimen().encode(limits()).unwrap();
        b[0] = 0;
        assert!(matches!(
            Resources::decode(&b, limits()),
            Err(Error::Version { .. })
        ));
        b[0] = VERSION as u8;
        b[12] = 2;
        assert_eq!(Resources::decode(&b, limits()), Err(Error::InvalidBool(2)));
        b[12] = 1;
        let scalar = 4 + 4 + 2 * 5 + 8;
        b[scalar] = 255;
        assert_eq!(
            Resources::decode(&b, limits()),
            Err(Error::InvalidBool(255))
        );
    }
    #[test]
    fn typed_allocation_is_not_wire_budget() {
        let mut v = specimen();
        v.resource_ownership = (0..200)
            .map(|resource_hash| ResourceEntry {
                resource_hash,
                value: vec![],
            })
            .collect();
        let (b, items) = v.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert!(
            v.resource_ownership.len() * std::mem::size_of::<ResourceEntry<Vec<i32>>>() > b.len()
        );
        assert_eq!(Resources::decode(&b, exact).unwrap(), v);
    }
    #[test]
    fn hostile_count_rejected_before_allocation() {
        let mut b = VERSION.to_le_bytes().to_vec();
        b.extend(u32::MAX.to_le_bytes());
        assert_eq!(Resources::decode(&b, limits()), Err(Error::ResourceLimit));
    }
}
