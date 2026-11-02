//! Bounded Linux variant wire transport from static contracts, not gameplay
//! semantics or native load/resave normalization. Unsupported branches fail closed.
//! See `docs/linux-unit-wire-variants.md`; native differential parity is pending.
//! Typed support is limited to modifier zero and declared types 0..=5, 8..=11,
//! 20..=22. Strings accept canonical NUL-terminated char8 constructor buffers;
//! arbitrary raw arrays, other declared types, negative map counts, conflicting
//! branches, and native normalization cases are rejected rather than approximated.

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_items: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_items: 1_000_000,
            max_depth: 64,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariantError {
    Truncated,
    Limit,
    NegativeCount,
    InvalidOrder,
    InvalidHeader,
    InvalidMetadata,
    UnsupportedType(u16),
    UnsupportedModifiers(u16),
    UnsupportedBranch,
}
impl std::fmt::Display for VariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for VariantError {}

/// Aggregate budget can be shared by successive prefix decodes. A failed decode
/// leaves this budget unchanged. Bytes and entries are charged independently.
#[derive(Clone, Debug)]
pub struct DecodeBudget {
    limits: Limits,
    bytes: usize,
    items: usize,
}
impl DecodeBudget {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            bytes: 0,
            items: 0,
        }
    }
    pub fn remaining_items(&self) -> usize {
        self.limits.max_items - self.items
    }
    pub fn items_used(&self) -> usize {
        self.items
    }
    pub fn bytes_used(&self) -> usize {
        self.bytes
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VariantMap {
    /// Separate ascending unsigned-key trees. Values retain all union bits.
    pub scalars: Vec<(u32, u32)>,
    pub arrays: Vec<(u32, ArrayVariant)>,
}
/// Actual serialized keyed prefix. Preserve accepted noncanonical prefix bits;
/// this is transport metadata, not a manager/options reconstruction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyedHeader {
    #[default]
    Count,
    Prefix(u32),
}
impl KeyedHeader {
    fn names(self) -> bool {
        matches!(self, Self::Prefix(x) if x & 1 != 0)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedEntry {
    pub key: u32,
    /// None when names are absent; Some(empty) retains a zero-length wire name.
    pub name: Option<Vec<u8>>,
    pub value: TypedVariant,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypedVariantMap {
    pub header: KeyedHeader,
    pub entries: Vec<TypedEntry>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypedVariantVectorMap {
    pub header: KeyedHeader,
    pub entries: Vec<TypedEntry>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedVariant {
    pub type_bits: u16,
    pub modifier_bits: u16,
    pub value: ArrayVariant,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayVariant {
    pub count_flags: u32,
    pub element_size: u32,
    pub payload: VariantPayload,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariantPayload {
    Inline32(u32),
    Inline64(u64),
    /// Classified char8 buffer only; arbitrary untyped raw buffers are rejected.
    Char8(Vec<u8>),
    VariantMap(Box<VariantMap>),
    /// The native writer serializes no vector members in this branch.
    EmptyVariantVector,
    TypedVariantMap(Box<TypedVariantMap>),
    TypedVariantVector(Vec<TypedVariant>),
    TypedVariantVectorMap(Box<TypedVariantVectorMap>),
}
const OWN: u32 = 0x0100_0000;
const CHILDREN: u32 = 0x5e00_0000;
const LONG: u32 = 0x8000_0000;
const ARRAY: u32 = 0x2000_0000;

struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
    budget: DecodeBudget,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], VariantError> {
        let end = self.pos.checked_add(n).ok_or(VariantError::Limit)?;
        let charged = self
            .budget
            .bytes
            .checked_add(n)
            .ok_or(VariantError::Limit)?;
        if charged > self.budget.limits.max_bytes {
            return Err(VariantError::Limit);
        }
        let bytes = self
            .input
            .get(self.pos..end)
            .ok_or(VariantError::Truncated)?;
        self.pos = end;
        self.budget.bytes = charged;
        Ok(bytes)
    }
    fn u16(&mut self) -> Result<u16, VariantError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, VariantError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, VariantError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn count(&mut self, n: u32, minimum: usize) -> Result<usize, VariantError> {
        let n = usize::try_from(n).map_err(|_| VariantError::Limit)?;
        if n > self.budget.remaining_items() {
            return Err(VariantError::Limit);
        }
        if n.checked_mul(minimum).ok_or(VariantError::Limit)? > self.input.len() - self.pos {
            return Err(VariantError::Truncated);
        }
        self.budget.items += n;
        Ok(n)
    }
    fn depth(&self, depth: usize) -> Result<(), VariantError> {
        if depth > self.budget.limits.max_depth {
            Err(VariantError::Limit)
        } else {
            Ok(())
        }
    }
    fn map(&mut self, depth: usize) -> Result<VariantMap, VariantError> {
        self.depth(depth)?;
        let mut map = VariantMap::default();
        let n = self.u32()?;
        if n & 0x8000_0000 != 0 {
            return Err(VariantError::NegativeCount);
        }
        for _ in 0..self.count(n, 8)? {
            map.scalars.push((self.u32()?, self.u32()?));
        }
        ordered(map.scalars.iter().map(|x| x.0))?;
        let n = self.u32()?;
        if n & 0x8000_0000 != 0 {
            return Err(VariantError::NegativeCount);
        }
        for _ in 0..self.count(n, 12)? {
            map.arrays.push((self.u32()?, self.array(depth + 1, None)?));
        }
        ordered(map.arrays.iter().map(|x| x.0))?;
        Ok(map)
    }
    fn keyed(
        &mut self,
        depth: usize,
        sorted: bool,
    ) -> Result<(KeyedHeader, Vec<TypedEntry>), VariantError> {
        self.depth(depth)?;
        let first = self.u32()?;
        let (header, n) = if first & 0x8000_0000 != 0 {
            (KeyedHeader::Prefix(first), self.u32()?)
        } else {
            (KeyedHeader::Count, first)
        };
        let mut entries = Vec::new();
        for _ in 0..self.count(n, if header.names() { 18 } else { 16 })? {
            let key = self.u32()?;
            let name = if header.names() {
                let n = self.u16()?;
                Some(self.take(usize::from(n))?.to_vec())
            } else {
                None
            };
            entries.push(TypedEntry {
                key,
                name,
                value: self.typed(depth + 1)?,
            });
        }
        if sorted {
            ordered(entries.iter().map(|x| x.key))?;
        } else {
            let mut keys: Vec<_> = entries.iter().map(|x| x.key).collect();
            keys.sort_unstable();
            ordered(keys.into_iter())?;
        }
        Ok((header, entries))
    }
    fn typed(&mut self, depth: usize) -> Result<TypedVariant, VariantError> {
        self.depth(depth)?;
        let type_bits = self.u16()?;
        let modifier_bits = self.u16()?;
        if modifier_bits != 0 {
            return Err(VariantError::UnsupportedModifiers(modifier_bits));
        }
        if !matches!(type_bits, 0..=5 | 8..=11 | 20..=22) {
            return Err(VariantError::UnsupportedType(type_bits));
        }
        let value = self.array(depth + 1, Some(type_bits))?;
        Ok(TypedVariant {
            type_bits,
            modifier_bits,
            value,
        })
    }
    fn array(&mut self, depth: usize, declared: Option<u16>) -> Result<ArrayVariant, VariantError> {
        self.depth(depth)?;
        let count_flags = self.u32()?;
        let element_size = self.u32()?;
        let child = count_flags & CHILDREN;
        // Conflicting branch flags and ValueIsArray load normalization are not modeled.
        if child.count_ones() > 1
            || (count_flags & ARRAY != 0 && (child != 0 || count_flags & OWN == 0))
            || (child != 0 && count_flags & LONG != 0)
        {
            return Err(VariantError::InvalidMetadata);
        }
        if child != 0 && (element_size != 0 || count_flags & 0x00ff_ffff != 0) {
            return Err(VariantError::InvalidMetadata);
        }
        let payload = match child {
            0x0200_0000 => VariantPayload::VariantMap(Box::new(self.map(depth + 1)?)),
            0x0800_0000 => VariantPayload::EmptyVariantVector,
            0x0400_0000 => {
                let (header, entries) = self.keyed(depth + 1, true)?;
                VariantPayload::TypedVariantMap(Box::new(TypedVariantMap { header, entries }))
            }
            0x1000_0000 => {
                let n = self.u32()?;
                let mut values = Vec::new();
                for _ in 0..self.count(n, 12)? {
                    values.push(self.typed(depth + 1)?);
                }
                VariantPayload::TypedVariantVector(values)
            }
            0x4000_0000 => {
                let (header, entries) = self.keyed(depth + 1, false)?;
                VariantPayload::TypedVariantVectorMap(Box::new(TypedVariantVectorMap {
                    header,
                    entries,
                }))
            }
            0 => {
                if count_flags & OWN != 0 {
                    if declared != Some(5) {
                        return Err(VariantError::UnsupportedBranch);
                    }
                    let count = count_flags & 0x00ff_ffff;
                    let n = count.checked_mul(element_size).ok_or(VariantError::Limit)?;
                    if element_size != 1 || count == 0 || count_flags & LONG != 0 {
                        return Err(VariantError::InvalidMetadata);
                    }
                    let bytes = self.take(n as usize)?.to_vec();
                    if bytes.last() != Some(&0) || bytes[..bytes.len() - 1].contains(&0) {
                        return Err(VariantError::InvalidMetadata);
                    }
                    VariantPayload::Char8(bytes)
                } else if count_flags & LONG != 0 {
                    VariantPayload::Inline64(self.u64()?)
                } else {
                    VariantPayload::Inline32(self.u32()?)
                }
            }
            _ => return Err(VariantError::UnsupportedBranch),
        };
        let value = ArrayVariant {
            count_flags,
            element_size,
            payload,
        };
        if let Some(t) = declared {
            value.validate_type(t)?;
        }
        Ok(value)
    }
}
fn ordered(keys: impl Iterator<Item = u32>) -> Result<(), VariantError> {
    let mut previous = None;
    for key in keys {
        if previous.is_some_and(|p| p >= key) {
            return Err(VariantError::InvalidOrder);
        }
        previous = Some(key);
    }
    Ok(())
}
impl ArrayVariant {
    fn validate_type(&self, t: u16) -> Result<(), VariantError> {
        let valid = match (&self.payload, t) {
            (VariantPayload::Inline32(0), 0 | 8..=11 | 22) => {
                self.count_flags == 0 && self.element_size == 0
            }
            (VariantPayload::Inline32(_), 1..=4) => self.count_flags == 0 && self.element_size == 0,
            (VariantPayload::Inline32(0), 5) => self.count_flags == 0 && self.element_size == 0,
            (VariantPayload::Inline64(_), 20 | 21) => {
                self.count_flags == LONG && self.element_size == 0
            }
            (VariantPayload::Char8(_), 5) => true,
            (VariantPayload::VariantMap(_), 8)
            | (VariantPayload::EmptyVariantVector, 9)
            | (VariantPayload::TypedVariantMap(_), 10)
            | (VariantPayload::TypedVariantVector(_), 11)
            | (VariantPayload::TypedVariantVectorMap(_), 22) => true,
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(VariantError::InvalidMetadata)
        }
    }
}

struct Writer {
    bytes: Vec<u8>,
    limits: Limits,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), VariantError> {
        if bytes.len() > self.limits.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(VariantError::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn u16(&mut self, n: u16) -> Result<(), VariantError> {
        self.put(&n.to_le_bytes())
    }
    fn u32(&mut self, n: u32) -> Result<(), VariantError> {
        self.put(&n.to_le_bytes())
    }
    fn count(&mut self, n: usize) -> Result<(), VariantError> {
        if n > self.limits.max_items {
            return Err(VariantError::Limit);
        }
        self.u32(u32::try_from(n).map_err(|_| VariantError::Limit)?)
    }
    fn depth(&self, depth: usize) -> Result<(), VariantError> {
        if depth > self.limits.max_depth {
            Err(VariantError::Limit)
        } else {
            Ok(())
        }
    }
    fn map(&mut self, map: &VariantMap, depth: usize) -> Result<(), VariantError> {
        self.depth(depth)?;
        self.count(map.scalars.len())?;
        for (key, bits) in &map.scalars {
            self.u32(*key)?;
            self.u32(*bits)?;
        }
        self.count(map.arrays.len())?;
        for (key, value) in &map.arrays {
            self.u32(*key)?;
            self.array(value, depth + 1)?;
        }
        Ok(())
    }
    fn keyed(
        &mut self,
        header: KeyedHeader,
        entries: &[TypedEntry],
        depth: usize,
    ) -> Result<(), VariantError> {
        self.depth(depth)?;
        match header {
            KeyedHeader::Count => {
                if entries.len() >= 0x8000_0000 {
                    return Err(VariantError::InvalidHeader);
                }
            }
            KeyedHeader::Prefix(prefix) => {
                if prefix & 0x8000_0000 == 0 {
                    return Err(VariantError::InvalidHeader);
                }
                self.u32(prefix)?;
            }
        }
        self.count(entries.len())?;
        for entry in entries {
            self.u32(entry.key)?;
            match (&entry.name, header.names()) {
                (Some(bytes), true) => {
                    self.u16(u16::try_from(bytes.len()).map_err(|_| VariantError::Limit)?)?;
                    self.put(bytes)?;
                }
                (None, false) => {}
                _ => return Err(VariantError::InvalidMetadata),
            }
            self.typed(&entry.value, depth + 1)?;
        }
        Ok(())
    }
    fn typed(&mut self, value: &TypedVariant, depth: usize) -> Result<(), VariantError> {
        self.depth(depth)?;
        self.u16(value.type_bits)?;
        self.u16(value.modifier_bits)?;
        self.array(&value.value, depth + 1)
    }
    fn array(&mut self, value: &ArrayVariant, depth: usize) -> Result<(), VariantError> {
        self.depth(depth)?;
        let branch = value.count_flags & CHILDREN;
        let expected = match &value.payload {
            VariantPayload::VariantMap(_) => 0x0200_0000,
            VariantPayload::EmptyVariantVector => 0x0800_0000,
            VariantPayload::TypedVariantMap(_) => 0x0400_0000,
            VariantPayload::TypedVariantVector(_) => 0x1000_0000,
            VariantPayload::TypedVariantVectorMap(_) => 0x4000_0000,
            _ => 0,
        };
        if branch != expected {
            return Err(VariantError::InvalidMetadata);
        }
        match &value.payload {
            VariantPayload::Inline32(_) if value.count_flags & (OWN | LONG) != 0 => {
                return Err(VariantError::InvalidMetadata)
            }
            VariantPayload::Inline64(_)
                if value.count_flags & OWN != 0 || value.count_flags & LONG == 0 =>
            {
                return Err(VariantError::InvalidMetadata)
            }
            VariantPayload::Char8(bytes)
                if value.count_flags & OWN == 0
                    || value.count_flags & LONG != 0
                    || value.element_size != 1
                    || bytes.len() != (value.count_flags & 0xffffff) as usize =>
            {
                return Err(VariantError::InvalidMetadata)
            }
            _ => {}
        }
        self.u32(value.count_flags)?;
        self.u32(value.element_size)?;
        match &value.payload {
            VariantPayload::Inline32(bits) => self.u32(*bits),
            VariantPayload::Inline64(bits) => self.put(&bits.to_le_bytes()),
            VariantPayload::Char8(bytes) => self.put(bytes),
            VariantPayload::VariantMap(map) => self.map(map, depth + 1),
            VariantPayload::EmptyVariantVector => Ok(()),
            VariantPayload::TypedVariantMap(map) => self.keyed(map.header, &map.entries, depth + 1),
            VariantPayload::TypedVariantVector(values) => {
                self.count(values.len())?;
                for value in values {
                    self.typed(value, depth + 1)?;
                }
                Ok(())
            }
            VariantPayload::TypedVariantVectorMap(map) => {
                self.keyed(map.header, &map.entries, depth + 1)
            }
        }
    }
}
macro_rules! root_codec {
    ($ty:ty, $read:expr, $write:expr) => {
        impl $ty {
            /// Decode one framed prefix, committing aggregate budget only on success.
            pub fn decode_prefix(
                input: &[u8],
                budget: &mut DecodeBudget,
            ) -> Result<(Self, usize), VariantError> {
                let mut reader = Reader {
                    input,
                    pos: 0,
                    budget: budget.clone(),
                };
                let value = ($read)(&mut reader)?;
                *budget = reader.budget;
                Ok((value, reader.pos))
            }
            /// Return bytes and aggregate item charge. Validates semantic support
            /// and framing using the same bounded decoder before returning bytes.
            pub fn encode(&self, limits: Limits) -> Result<(Vec<u8>, usize), VariantError> {
                let mut writer = Writer {
                    bytes: Vec::new(),
                    limits,
                };
                ($write)(&mut writer, self)?;
                let mut budget = DecodeBudget::new(limits);
                let (_, consumed) = Self::decode_prefix(&writer.bytes, &mut budget)?;
                if consumed != writer.bytes.len() {
                    return Err(VariantError::InvalidMetadata);
                }
                Ok((writer.bytes, budget.items_used()))
            }
        }
    };
}
root_codec!(
    VariantMap,
    |r: &mut Reader<'_>| r.map(0),
    |w: &mut Writer, v: &VariantMap| w.map(v, 0)
);
root_codec!(
    TypedVariantMap,
    |r: &mut Reader<'_>| {
        let (header, entries) = r.keyed(0, true)?;
        Ok(TypedVariantMap { header, entries })
    },
    |w: &mut Writer, v: &TypedVariantMap| w.keyed(v.header, &v.entries, 0)
);
root_codec!(
    TypedVariantVectorMap,
    |r: &mut Reader<'_>| {
        let (header, entries) = r.keyed(0, false)?;
        Ok(TypedVariantVectorMap { header, entries })
    },
    |w: &mut Writer, v: &TypedVariantVectorMap| w.keyed(v.header, &v.entries, 0)
);
root_codec!(
    TypedVariant,
    |r: &mut Reader<'_>| r.typed(0),
    |w: &mut Writer, v: &TypedVariant| w.typed(v, 0)
);

#[cfg(test)]
mod tests {
    use super::*;
    fn decode<T>(
        bytes: &[u8],
        read: impl FnOnce(&[u8], &mut DecodeBudget) -> Result<(T, usize), VariantError>,
    ) -> T {
        let (value, consumed) = read(bytes, &mut DecodeBudget::new(Limits::default())).unwrap();
        assert_eq!(consumed, bytes.len());
        value
    }
    #[test]
    fn literal_unsigned_keys_and_union_bits() {
        let wire = [1, 0, 0, 0, 255, 255, 255, 255, 0, 0, 192, 127, 0, 0, 0, 0];
        let map = decode(&wire, VariantMap::decode_prefix);
        assert_eq!(map.scalars, [(u32::MAX, 0x7fc00000)]);
        assert_eq!(map.encode(Limits::default()).unwrap(), (wire.to_vec(), 1));
    }
    #[test]
    fn named_header_prefix_name_and_bool_upper_bytes() {
        let wire = [
            3, 0, 0, 128, 1, 0, 0, 0, 7, 0, 0, 0, 2, 0, 255, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 2, 3, 4,
        ];
        let map = decode(&wire, TypedVariantMap::decode_prefix);
        assert_eq!(map.header, KeyedHeader::Prefix(0x80000003));
        assert_eq!(map.entries[0].name, Some(vec![255, 0]));
        assert_eq!(
            map.entries[0].value.value.payload,
            VariantPayload::Inline32(0x04030201)
        );
        assert_eq!(map.encode(Limits::default()).unwrap().0, wire);
    }
    #[test]
    fn literal_nested_vector_map_preserves_order() {
        // TypedVectorMap (type22), owns child, two null typed entries in vector order.
        let wire = [
            22, 0, 0, 0, 0, 0, 0, 65, 0, 0, 0, 0, 2, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let value = decode(&wire, TypedVariant::decode_prefix);
        assert_eq!(value.encode(Limits::default()).unwrap(), (wire.to_vec(), 2));
        let mut budget = DecodeBudget::new(Limits {
            max_items: 1,
            ..Limits::default()
        });
        assert_eq!(
            TypedVariant::decode_prefix(&wire, &mut budget),
            Err(VariantError::Limit)
        );
        assert_eq!(budget.items_used(), 0);
    }
    #[test]
    fn string_float_and_int64_bits() {
        for wire in [
            vec![5, 0, 0, 0, 2, 0, 0, 33, 1, 0, 0, 0, b'x', 0],
            vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 128, 127],
            vec![
                20, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
            ],
        ] {
            let v = decode(&wire, TypedVariant::decode_prefix);
            assert_eq!(v.encode(Limits::default()).unwrap().0, wire);
        }
    }
    #[test]
    fn fail_closed_types_modifiers_flags_counts_and_truncation() {
        let base = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
        for length in 0..base.len() {
            assert!(TypedVariant::decode_prefix(
                &base[..length],
                &mut DecodeBudget::new(Limits::default())
            )
            .is_err());
        }
        for (offset, byte, expected) in [
            (0, 6, VariantError::UnsupportedType(6)),
            (2, 1, VariantError::UnsupportedModifiers(1)),
            (7, 32, VariantError::InvalidMetadata),
        ] {
            let mut wire = base;
            wire[offset] = byte;
            assert_eq!(
                TypedVariant::decode_prefix(&wire, &mut DecodeBudget::new(Limits::default())),
                Err(expected)
            );
        }
        assert_eq!(
            VariantMap::decode_prefix(
                &[255, 255, 255, 255],
                &mut DecodeBudget::new(Limits::default())
            ),
            Err(VariantError::NegativeCount)
        );
    }
    #[test]
    fn prefix_aggregate_and_depth_limits() {
        let wire = [1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 99];
        let mut budget = DecodeBudget::new(Limits {
            max_items: 1,
            ..Limits::default()
        });
        assert_eq!(VariantMap::decode_prefix(&wire, &mut budget).unwrap().1, 16);
        assert_eq!(budget.bytes_used(), 16);
        assert_eq!(
            VariantMap::decode_prefix(&wire, &mut budget),
            Err(VariantError::Limit)
        );
        let typed = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            TypedVariant::decode_prefix(
                &typed,
                &mut DecodeBudget::new(Limits {
                    max_depth: 0,
                    ..Limits::default()
                })
            ),
            Err(VariantError::Limit)
        );
        assert_eq!(
            VariantMap::decode_prefix(
                &wire,
                &mut DecodeBudget::new(Limits {
                    max_bytes: 15,
                    ..Limits::default()
                })
            ),
            Err(VariantError::Limit)
        );
    }
    #[test]
    fn reject_map_duplicates_and_payload_mismatch() {
        let wire = [
            2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(
            VariantMap::decode_prefix(&wire, &mut DecodeBudget::new(Limits::default())),
            Err(VariantError::InvalidOrder)
        );
        let invalid = TypedVariant {
            type_bits: 2,
            modifier_bits: 0,
            value: ArrayVariant {
                count_flags: LONG,
                element_size: 0,
                payload: VariantPayload::Inline32(1),
            },
        };
        assert_eq!(
            invalid.encode(Limits::default()),
            Err(VariantError::InvalidMetadata)
        );
    }
    #[test]
    fn null_children_and_empty_named_header_are_wire_metadata() {
        for type_id in [0, 8, 9, 10, 11, 22] {
            let mut wire = [0; 16];
            wire[0] = type_id;
            let v = decode(&wire, TypedVariant::decode_prefix);
            assert_eq!(v.encode(Limits::default()).unwrap().0, wire);
        }
        let named_empty = [1, 0, 0, 128, 0, 0, 0, 0];
        let map = decode(&named_empty, TypedVariantMap::decode_prefix);
        assert_eq!(map.encode(Limits::default()).unwrap().0, named_empty);
    }
    #[test]
    fn nested_map_and_typed_vector_frame_exactly() {
        // A typed map owns one typed vector, whose only member is uint32.
        let wire = [
            1, 0, 0, 0, 5, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0,
        ];
        let map = decode(&wire, TypedVariantMap::decode_prefix);
        assert_eq!(map.encode(Limits::default()).unwrap(), (wire.to_vec(), 2));
        assert_eq!(
            map.encode(Limits {
                max_items: 1,
                ..Limits::default()
            }),
            Err(VariantError::Limit)
        );
        assert_eq!(
            map.encode(Limits {
                max_depth: 2,
                ..Limits::default()
            }),
            Err(VariantError::Limit)
        );
    }
    #[test]
    fn unsupported_buffers_conflicting_children_and_invalid_strings() {
        let untyped_raw = [
            0, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, 1, 0, 0, 33, 1, 0, 0, 0, 0,
        ];
        assert_eq!(
            VariantMap::decode_prefix(&untyped_raw, &mut DecodeBudget::new(Limits::default())),
            Err(VariantError::UnsupportedBranch)
        );
        let conflict = [8, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0];
        assert_eq!(
            TypedVariant::decode_prefix(&conflict, &mut DecodeBudget::new(Limits::default())),
            Err(VariantError::InvalidMetadata)
        );
        let invalid_string = [5, 0, 0, 0, 2, 0, 0, 33, 1, 0, 0, 0, b'x', b'y'];
        assert_eq!(
            TypedVariant::decode_prefix(&invalid_string, &mut DecodeBudget::new(Limits::default())),
            Err(VariantError::InvalidMetadata)
        );
    }
}
