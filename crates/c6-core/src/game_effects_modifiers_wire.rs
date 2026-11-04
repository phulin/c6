//! Reached Linux XP2 ModifierSystem5 / modifier-definition2 wire transport.
//! Preserves definition-bank order, argument byte strings, raw flags/limits,
//! saved object IDs, modifier IDs and the three distinct state-list orders.
//! Native definition construction, ID lookup/remapping, attachment/callback/RNG
//! effects and gameplay installation are not applied.

pub use crate::unit::Limits;
pub const VERSION: u8 = 5;
pub const DEFINITION_VERSION: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux ModifierSystem wire: {self:?}")
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
fn byte_version(r: &mut Reader<'_>, expected: u8) -> Result<u8, Error> {
    let actual = u8::read(r)?;
    if actual != expected {
        return Err(Error::Version {
            expected: expected.into(),
            actual: actual.into(),
        });
    }
    Ok(actual)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgumentName {
    Legacy(Vec<u8>),
    /// Any first string of length one is a signed-char version marker natively.
    Versioned {
        marker_raw: u8,
        name: Vec<u8>,
    },
}
impl ArgumentName {
    pub fn name(&self) -> &[u8] {
        match self {
            Self::Legacy(name) | Self::Versioned { name, .. } => name,
        }
    }
    /// Source sign-extends the marker char, then subtracts ASCII backtick (96).
    pub fn version_signal(&self) -> i32 {
        match self {
            Self::Legacy(_) => 0,
            Self::Versioned { marker_raw, .. } => i32::from(*marker_raw as i8) - 96,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentDefinition {
    pub name: ArgumentName,
    pub value: Vec<u8>,
    pub function: Vec<u8>,
    pub extra: Vec<u8>,
    /// Read only for a positive signed-char version signal.
    pub second_extra: Option<Vec<u8>>,
}
impl Wire for ArgumentDefinition {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let first = Vec::<u8>::read(r)?;
        let name = if first.len() == 1 {
            ArgumentName::Versioned {
                marker_raw: first[0],
                name: Vec::read(r)?,
            }
        } else {
            ArgumentName::Legacy(first)
        };
        let value = Vec::read(r)?;
        let function = Vec::read(r)?;
        let extra = Vec::read(r)?;
        let second_extra = if name.version_signal() > 0 {
            Some(Vec::read(r)?)
        } else {
            None
        };
        Ok(Self {
            name,
            value,
            function,
            extra,
            second_extra,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierDefinition {
    pub wire_version: u8,
    pub modifier_id: Vec<u8>,
    pub modifier_type_id_raw: u32,
    pub permanent_raw: u8,
    pub run_once_raw: u8,
    pub owner_requirement_set_id: Vec<u8>,
    pub subject_requirement_set_id: Vec<u8>,
    pub arguments: Vec<ArgumentDefinition>,
    pub new_only_raw: u8,
    pub owner_stack_limit: u16,
    pub subject_stack_limit: u16,
    /// Explicit native false initialization; Repeatable is serialized since3.
    pub repeatable_read_default_raw: u8,
}
impl Wire for ModifierDefinition {
    const MIN: usize = 28;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: byte_version(r, DEFINITION_VERSION)?,
            modifier_id: Vec::read(r)?,
            modifier_type_id_raw: u32::read(r)?,
            permanent_raw: u8::read(r)?,
            run_once_raw: u8::read(r)?,
            owner_requirement_set_id: Vec::read(r)?,
            subject_requirement_set_id: Vec::read(r)?,
            arguments: Vec::read(r)?,
            new_only_raw: u8::read(r)?,
            owner_stack_limit: u16::read(r)?,
            subject_stack_limit: u16::read(r)?,
            repeatable_read_default_raw: 0,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierState {
    pub owner_saved_id: i32,
    /// Ordinal into this serialized definition bank, not a definition hash.
    pub definition_bank_index: u32,
    pub subject_saved_ids: Vec<i32>,
    pub activated_raw: u8,
    pub was_activated_raw: u8,
    pub instance_id: i32,
    pub tracked_saved_ids: Vec<i32>,
    pub ignored_saved_ids: Vec<i32>,
}
impl Wire for ModifierState {
    const MIN: usize = 26;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            owner_saved_id: i32::read(r)?,
            definition_bank_index: u32::read(r)?,
            subject_saved_ids: Vec::read(r)?,
            activated_raw: u8::read(r)?,
            was_activated_raw: u8::read(r)?,
            instance_id: i32::read(r)?,
            tracked_saved_ids: Vec::read(r)?,
            ignored_saved_ids: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierSystem {
    pub wire_version: u8,
    pub next_modifier_id: i32,
    pub definitions: Vec<ModifierDefinition>,
    pub states: Vec<ModifierState>,
}
impl ModifierSystem {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    /// Leaves the parent GameEffects closing guard untouched.
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
            wire_version: byte_version(&mut r, VERSION)?,
            next_modifier_id: i32::read(&mut r)?,
            definitions: Vec::read(&mut r)?,
            states: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn words(b: &mut Vec<u8>, v: &[u32]) {
        for n in v {
            b.extend(n.to_le_bytes());
        }
    }
    fn string(b: &mut Vec<u8>, v: &[u8]) {
        words(b, &[v.len() as u32]);
        b.extend(v);
    }
    fn fixture() -> Vec<u8> {
        let mut b = vec![5];
        words(&mut b, &[999, 1]);
        b.push(2);
        string(&mut b, &[0, 255]);
        words(&mut b, &[0x81234567]);
        b.extend([255, 2]);
        string(&mut b, b"O");
        string(&mut b, b"SS");
        words(&mut b, &[3]);
        for v in [b"AB".as_slice(), b"v", b"", b"x"] {
            string(&mut b, v);
        }
        for v in [b"a".as_slice(), b"X", &[0, 255], b"f", b"e", b"S"] {
            string(&mut b, v);
        }
        for v in [[255u8].as_slice(), b"N", b"Z", b"", b""] {
            string(&mut b, v);
        }
        b.push(254);
        b.extend(u16::MAX.to_le_bytes());
        b.extend(0x1234u16.to_le_bytes());
        words(&mut b, &[2, 337, 0, 3, 337, u32::MAX, 337]);
        b.extend([255, 2]);
        words(
            &mut b,
            &[i32::MIN as u32, 2, u32::MAX, 338, 1, 339, u32::MAX, 0, 0],
        );
        b.extend([0, 255]);
        words(&mut b, &[17, 1, 5, 2, 7, 7]);
        b
    }
    #[test]
    fn literal_argument_marker_sign_flags_limits_bank_and_distinct_saved_lists() {
        let b = fixture();
        assert_eq!(b.len(), 208);
        let (v, n, i) = ModifierSystem::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (208, 34));
        assert_eq!(v.next_modifier_id, 999);
        let d = &v.definitions[0];
        assert_eq!(d.modifier_id, vec![0, 255]);
        assert_eq!(d.modifier_type_id_raw, 0x81234567);
        assert_eq!(
            (
                d.permanent_raw,
                d.run_once_raw,
                d.new_only_raw,
                d.repeatable_read_default_raw
            ),
            (255, 2, 254, 0)
        );
        assert_eq!(
            (d.owner_stack_limit, d.subject_stack_limit),
            (u16::MAX, 0x1234)
        );
        assert_eq!(
            (&d.owner_requirement_set_id, &d.subject_requirement_set_id),
            (&b"O".to_vec(), &b"SS".to_vec())
        );
        assert_eq!(d.arguments[0].name, ArgumentName::Legacy(b"AB".to_vec()));
        assert_eq!(d.arguments[0].second_extra, None);
        assert_eq!(
            d.arguments[1].name,
            ArgumentName::Versioned {
                marker_raw: b'a',
                name: b"X".to_vec()
            }
        );
        assert_eq!(d.arguments[1].name.version_signal(), 1);
        assert_eq!(d.arguments[1].value, vec![0, 255]);
        assert_eq!(
            (&d.arguments[1].function, &d.arguments[1].extra),
            (&b"f".to_vec(), &b"e".to_vec())
        );
        assert_eq!(d.arguments[1].second_extra, Some(b"S".to_vec()));
        assert_eq!(d.arguments[2].name.version_signal(), -97);
        assert_eq!(d.arguments[2].name.name(), b"N");
        assert_eq!(d.arguments[2].second_extra, None);
        assert_eq!(
            v.states[0],
            ModifierState {
                owner_saved_id: 337,
                definition_bank_index: 0,
                subject_saved_ids: vec![337, -1, 337],
                activated_raw: 255,
                was_activated_raw: 2,
                instance_id: i32::MIN,
                tracked_saved_ids: vec![-1, 338],
                ignored_saved_ids: vec![339]
            }
        );
        assert_eq!(v.states[1].owner_saved_id, -1);
        assert_eq!(v.states[1].instance_id, 17);
        assert_eq!(v.states[1].tracked_saved_ids, vec![5]);
        assert_eq!(v.states[1].ignored_saved_ids, vec![7, 7]);
        assert_eq!(
            (v.states[1].activated_raw, v.states[1].was_activated_raw),
            (0, 255)
        );
    }
    #[test]
    fn all_cuts_caps_independent_tags_nested_counts_and_tail() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                ModifierSystem::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                ModifierSystem::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..34 {
            assert_eq!(
                ModifierSystem::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for (at, expected) in [(0, 5), (9, 2)] {
            let mut bad = b.clone();
            bad[at] = 99;
            assert_eq!(
                ModifierSystem::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        for at in [5, 10, 33, 37, 116, 128, 150, 162] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                ModifierSystem::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit),
                "at {at}"
            );
        }
        let mut tail = b.clone();
        tail.push(99);
        assert_eq!(
            ModifierSystem::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            ModifierSystem::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            b.len()
        );
    }
}
