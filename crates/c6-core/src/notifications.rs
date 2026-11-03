//! Pinned Linux XP2 PlayerNotifications v1 / Instance v5 transport.
//! Localized arguments are explicitly unsupported. This is not gameplay state or
//! notification selection/activation: native event subscribers remain unmodeled.
use crate::unit::variants::{self, DecodeBudget, TypedVariantVectorMap};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_records: usize,
    /// Aggregate string bytes, variant entries/name/payload bytes, and records.
    pub max_items: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_records: 100_000,
            max_items: 1_000_000,
            max_depth: 64,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    TrailingBytes,
    ResourceLimit,
    UnsupportedVersion(u32),
    InvalidBoolean(u8),
    InvalidLocaleHeader,
    UnsupportedLocaleArguments,
    Variant(variants::VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<variants::VariantError> for Error {
    fn from(e: variants::VariantError) -> Self {
        Self::Variant(e)
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComponentId {
    pub owner: i16,
    pub component_type: i16,
    pub id: u32,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalizedString {
    /// Base string bytes, without UTF-8 conversion or an added terminator.
    pub base: Vec<u8>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Notification {
    /// Factory discriminator and Instance type are independently preserved.
    pub factory_type: u32,
    pub id: ComponentId,
    pub type_hash: u32,
    pub severity: i32,
    pub summary: LocalizedString,
    pub message: LocalizedString,
    pub location: [i32; 2],
    pub component: ComponentId,
    // Offset names deliberately retain unclassified semantics.
    pub native_188: i32,
    pub native_18c: i32,
    pub is_dismissed: bool,
    pub native_199: bool,
    pub native_19a: bool,
    pub native_190: i32,
    pub native_19b: bool,
    pub native_19c: bool,
    pub parameters: TypedVariantVectorMap,
    pub native_19d: bool,
    pub has_been_activated: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlayerNotifications {
    pub owner: i32,
    pub next_id: u32,
    pub records: Vec<Notification>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
    variants: DecodeBudget,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        let b = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(Error::InvalidBoolean(b)),
        }
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let v = self.u32()?;
        if v == expected {
            Ok(())
        } else {
            Err(Error::UnsupportedVersion(v))
        }
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self
            .items
            .checked_add(self.variants.items_used())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_items
        {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn component(&mut self) -> Result<ComponentId, Error> {
        let b = self.take(8)?;
        Ok(ComponentId {
            owner: i16::from_le_bytes(b[..2].try_into().unwrap()),
            component_type: i16::from_le_bytes(b[2..4].try_into().unwrap()),
            id: u32::from_le_bytes(b[4..].try_into().unwrap()),
        })
    }
    fn string(&mut self) -> Result<LocalizedString, Error> {
        if self.u32()? != u32::MAX || self.take(1)?[0] != 1 {
            return Err(Error::InvalidLocaleHeader);
        }
        let n = self.u32()? as usize;
        self.charge(n)?;
        let base = self.take(n)?.to_vec();
        if self.take(1)?[0] != 0 || self.take(1)?[0] != 0 {
            return Err(Error::UnsupportedLocaleArguments);
        }
        Ok(LocalizedString { base })
    }
    fn record(&mut self) -> Result<Notification, Error> {
        let factory_type = self.u32()?;
        self.version(5)?;
        let id = self.component()?;
        let type_hash = self.u32()?;
        let severity = self.u32()? as i32;
        let summary = self.string()?;
        let message = self.string()?;
        let location = [self.u32()? as i32, self.u32()? as i32];
        let component = self.component()?;
        let native_188 = self.u32()? as i32;
        let native_18c = self.u32()? as i32;
        let is_dismissed = self.boolean()?;
        let native_199 = self.boolean()?;
        let native_19a = self.boolean()?;
        let native_190 = self.u32()? as i32;
        let native_19b = self.boolean()?;
        let native_19c = self.boolean()?;
        let (parameters, n) =
            TypedVariantVectorMap::decode_prefix(&self.bytes[self.pos..], &mut self.variants)?;
        self.take(n)?;
        self.charge(0)?;
        let native_19d = self.boolean()?;
        let has_been_activated = self.boolean()?;
        Ok(Notification {
            factory_type,
            id,
            type_hash,
            severity,
            summary,
            message,
            location,
            component,
            native_188,
            native_18c,
            is_dismissed,
            native_199,
            native_19a,
            native_190,
            native_19b,
            native_19c,
            parameters,
            native_19d,
            has_been_activated,
        })
    }
}
fn variant_limits(l: Limits) -> variants::Limits {
    variants::Limits {
        max_bytes: l.max_bytes,
        max_items: l.max_items,
        max_depth: l.max_depth,
    }
}
impl PlayerNotifications {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        Self::decode_with_boundaries(bytes, limits).map(|x| x.0)
    }
    /// Returns exact record byte ranges in the supplied owner section.
    pub fn decode_with_boundaries(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, Vec<std::ops::Range<usize>>), Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
            variants: DecodeBudget::new(variant_limits(limits)),
        };
        r.version(1)?;
        let owner = r.u32()? as i32;
        let next_id = r.u32()?;
        let count = r.u32()? as usize;
        if count > limits.max_records {
            return Err(Error::ResourceLimit);
        }
        r.charge(count)?;
        // Every supported record has at least 85 bytes; check before allocation.
        if count > bytes.len().saturating_sub(r.pos) / 85 {
            return Err(Error::Truncated);
        }
        let mut records = Vec::new();
        let mut boundaries = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        boundaries
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            let start = r.pos;
            records.push(r.record()?);
            boundaries.push(start..r.pos);
        }
        if r.pos != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        Ok((
            Self {
                owner,
                next_id,
                records,
            },
            boundaries,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        if self.records.len() > limits.max_records || self.records.len() > i32::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        let mut w = Writer {
            bytes: Vec::new(),
            limits,
        };
        w.word(1)?;
        w.word(self.owner as u32)?;
        w.word(self.next_id)?;
        w.word(self.records.len() as u32)?;
        for r in &self.records {
            w.word(r.factory_type)?;
            w.word(5)?;
            w.component(r.id)?;
            w.word(r.type_hash)?;
            w.word(r.severity as u32)?;
            w.string(&r.summary)?;
            w.string(&r.message)?;
            for x in r.location {
                w.word(x as u32)?;
            }
            w.component(r.component)?;
            w.word(r.native_188 as u32)?;
            w.word(r.native_18c as u32)?;
            w.put(&[r.is_dismissed as u8, r.native_199 as u8, r.native_19a as u8])?;
            w.word(r.native_190 as u32)?;
            w.put(&[r.native_19b as u8, r.native_19c as u8])?;
            let (parameters, _) = r.parameters.encode(variant_limits(limits))?;
            w.put(&parameters)?;
            w.put(&[r.native_19d as u8, r.has_been_activated as u8])?;
        }
        // Reuse aggregate validation; no partially encoded output escapes.
        Self::decode(&w.bytes, limits)?;
        Ok(w.bytes)
    }
}
struct Writer {
    bytes: Vec<u8>,
    limits: Limits,
}
impl Writer {
    fn put(&mut self, b: &[u8]) -> Result<(), Error> {
        if self
            .bytes
            .len()
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        self.bytes
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    fn word(&mut self, v: u32) -> Result<(), Error> {
        self.put(&v.to_le_bytes())
    }
    fn component(&mut self, v: ComponentId) -> Result<(), Error> {
        self.put(&v.owner.to_le_bytes())?;
        self.put(&v.component_type.to_le_bytes())?;
        self.word(v.id)
    }
    fn string(&mut self, s: &LocalizedString) -> Result<(), Error> {
        let n = u32::try_from(s.base.len()).map_err(|_| Error::ResourceLimit)?;
        self.word(u32::MAX)?;
        self.put(&[1])?;
        self.word(n)?;
        self.put(&s.base)?;
        self.put(&[0, 0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PlayerNotifications {
        let r = Notification {
            factory_type: 123,
            type_hash: 456,
            severity: i32::MIN,
            id: ComponentId {
                owner: -1,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            summary: LocalizedString { base: vec![0, 255] },
            native_188: i32::MAX,
            is_dismissed: true,
            ..Default::default()
        };
        PlayerNotifications {
            owner: -1,
            next_id: u32::MAX,
            records: vec![r.clone(), r],
        }
    }
    #[test]
    fn preserves_order_duplicates_raw_strings_and_discriminators() {
        let v = fixture();
        let b = v.encode(Limits::default()).unwrap();
        assert_eq!(
            PlayerNotifications::decode(&b, Limits::default()).unwrap(),
            v
        );
        for end in 0..b.len() {
            assert!(PlayerNotifications::decode(&b[..end], Limits::default()).is_err());
        }
    }
    #[test]
    fn rejects_locale_branches_and_aggregate_budget() {
        let v = fixture();
        let mut b = v.encode(Limits::default()).unwrap();
        // Owner header16 + factory/version/ID/type/severity24 + locale header9 + base2.
        b[51] = 1;
        assert_eq!(
            PlayerNotifications::decode(&b, Limits::default()),
            Err(Error::UnsupportedLocaleArguments)
        );
        let l = Limits {
            max_items: 5,
            ..Limits::default()
        }; // 2 records plus 4 base bytes.
        assert_eq!(v.encode(l), Err(Error::ResourceLimit));
        let b = v.encode(Limits::default()).unwrap();
        assert_eq!(
            PlayerNotifications::decode(&b, l),
            Err(Error::ResourceLimit)
        );
    }
}
