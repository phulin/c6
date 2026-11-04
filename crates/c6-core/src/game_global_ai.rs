//! Current Linux XP2 GlobalAi v4 wire transport, including concrete nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 4;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnsupportedLegacyWrite,
    ConditionalBody,
    ArrayLength { expected: u32, actual: u32 },
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux GlobalAi wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    allocated: usize,
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
        self.allocated = self
            .allocated
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        Ok(())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
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

impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.raw(1)?[0])
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&[*self])
    }
}
impl Wire for i16 {
    const MIN: usize = 2;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(2)?.try_into().unwrap()))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.raw(&self.to_le_bytes())
    }
}
impl Wire for [i32; 11] {
    const MIN: usize = 48;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let actual = u32::read(r)?;
        if actual != 11 {
            return Err(Error::ArrayLength {
                expected: 11,
                actual,
            });
        }
        r.charge(11)?;
        let mut values = [0; 11];
        for value in &mut values {
            *value = i32::read(r)?;
        }
        Ok(values)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.charge(11)?;
        11u32.write(w)?;
        for value in self {
            value.write(w)?;
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version > 0 { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version > 0 { r.version($version)?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version > 0 { ($version as u32).write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}
// All ordinal/hash/raw-Q8 words retain their wire representation. Bool carriers
// are raw bytes: the native binding transfers a byte and branches on nonzero.
record!(CityId, 0, { owner:i16, component_type:i16, id:u32 });
record!(PlotCoord, 0, { x:i32, y:i32 });
record!(ControlHandle, 1, { player_ordinal:i32, id:u32 });
record!(HashQ8, 0, { definition_hash:u32, raw_q8:i32 });
record!(MayhemTracker, 1, { native_f32_bits:u32 });
record!(ControlFactory, 1, { next_id:u32 });
/// Independently tagged saved analysis2/3. value_34 defaults to0 below3;
/// the counted11-element raw-Q8 array is present at2, not an inferred default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EspionageCityAnalysis {
    pub wire_version: u32,
    pub city: CityId,
    pub act_values_raw_q8: [i32; 11],
    pub value_34: i32,
}
impl Wire for EspionageCityAnalysis {
    const MIN: usize = 60;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 2 | 3) {
            return Err(Error::Version {
                expected: 3,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            city: CityId::read(r)?,
            act_values_raw_q8: <[i32; 11]>::read(r)?,
            value_34: if wire_version == 3 { i32::read(r)? } else { 0 },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 3 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        3u32.write(w)?;
        self.city.write(w)?;
        self.act_values_raw_q8.write(w)?;
        self.value_34.write(w)
    }
}
/// Saved Espionage2/4 wire/default projection. Absent vectors at2 have a
/// native self-default, so None preserves unknown prior state. Native Read's
/// subsequent resize(values_1a8,11,0), signals/archive/player binding are
/// unapplied; resize can preserve prior values and is not eleven known zeros.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Espionage {
    pub wire_version: u32,
    pub player_ordinal: i32,
    pub control_priority: i32,
    pub city_analyses: Vec<EspionageCityAnalysis>,
    pub flag_2a0: u8,
    pub cities: Option<Vec<CityId>>,
    pub values_158: Option<Vec<i32>>,
    pub values_180: Option<Vec<i32>>,
    pub values_1a8: Option<Vec<i32>>,
}
impl Wire for Espionage {
    const MIN: usize = 17;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 2 | 4) {
            return Err(Error::Version {
                expected: 4,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            player_ordinal: i32::read(r)?,
            control_priority: i32::read(r)?,
            city_analyses: Vec::<EspionageCityAnalysis>::read(r)?,
            flag_2a0: u8::read(r)?,
            cities: if wire_version == 4 {
                Some(Vec::<CityId>::read(r)?)
            } else {
                None
            },
            values_158: if wire_version == 4 {
                Some(Vec::<i32>::read(r)?)
            } else {
                None
            },
            values_180: if wire_version == 4 {
                Some(Vec::<i32>::read(r)?)
            } else {
                None
            },
            values_1a8: if wire_version == 4 {
                Some(Vec::<i32>::read(r)?)
            } else {
                None
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 4 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let cities = self.cities.as_ref().ok_or(Error::ConditionalBody)?;
        let values_158 = self.values_158.as_ref().ok_or(Error::ConditionalBody)?;
        let values_180 = self.values_180.as_ref().ok_or(Error::ConditionalBody)?;
        let values_1a8 = self.values_1a8.as_ref().ok_or(Error::ConditionalBody)?;
        4u32.write(w)?;
        self.player_ordinal.write(w)?;
        self.control_priority.write(w)?;
        self.city_analyses.write(w)?;
        self.flag_2a0.write(w)?;
        cities.write(w)?;
        values_158.write(w)?;
        values_180.write(w)?;
        values_1a8.write(w)
    }
}
record!(EspionageManager, 1, { entries:Vec<Espionage> });
record!(BeliefAnalysis, 1, {
    player_ordinal:i32, belief_hash:u32, belief_class_ordinal:i32, flag_10:u8, value_raw_q8:i32,
});
record!(HashBeliefAnalysis, 0, { definition_hash:u32, analysis:BeliefAnalysis });
record!(CityReligionAnalysis, 4, {
    city:CityId, position:PlotCoord, position_10:PlotCoord,
    value_18_raw_q8:i32, value_24_raw_q8:i32, value_1c_raw_q8:i32,
    flag_20:u8, values_28:Vec<u32>,
});
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Religious {
    pub wire_version: u32,
    pub initialized_raw: u8,
    pub body: Option<ReligiousBody>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReligiousBody {
    pub player_ordinal: i32,
    pub religion_hash: u32,
    pub flag_118: u8,
    pub beliefs: Vec<HashBeliefAnalysis>,
    pub handles: Vec<ControlHandle>,
    pub handle: ControlHandle,
    pub flag_130: u8,
    pub cities: Vec<CityReligionAnalysis>,
    /// Serialized pairs only. Empty at5 means no wire entries; native Read
    /// resets actual Yield definition indices to0 without revealing their count.
    pub yields: Vec<HashQ8>,
}
impl ReligiousBody {
    fn read_in_version(r: &mut Reader<'_>, version: u32) -> Result<Self, Error> {
        Ok(Self {
            player_ordinal: <i32 as Wire>::read(r)?,
            religion_hash: <u32 as Wire>::read(r)?,
            flag_118: <u8 as Wire>::read(r)?,
            beliefs: <Vec<HashBeliefAnalysis> as Wire>::read(r)?,
            handles: <Vec<ControlHandle> as Wire>::read(r)?,
            handle: <ControlHandle as Wire>::read(r)?,
            flag_130: <u8 as Wire>::read(r)?,
            cities: <Vec<CityReligionAnalysis> as Wire>::read(r)?,
            yields: if version == 6 {
                Vec::<HashQ8>::read(r)?
            } else {
                Vec::new()
            },
        })
    }
}
impl Wire for ReligiousBody {
    const MIN: usize = 34;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Self::read_in_version(r, 6)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.player_ordinal.write(w)?;
        self.religion_hash.write(w)?;
        self.flag_118.write(w)?;
        self.beliefs.write(w)?;
        self.handles.write(w)?;
        self.handle.write(w)?;
        self.flag_130.write(w)?;
        self.cities.write(w)?;
        self.yields.write(w)?;
        Ok(())
    }
}
impl Religious {
    /// True only for a present5 body: native Read resets every actual Yield
    /// definition index to raw-Q8 zero. Its definition count remains unknown.
    pub fn omitted_yields_reset_to_zero(&self) -> bool {
        self.wire_version == 5 && self.body.is_some()
    }
}
impl Wire for Religious {
    const MIN: usize = 5;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 5 | 6) {
            return Err(Error::Version {
                expected: 6,
                actual: wire_version,
            });
        }
        let initialized_raw = u8::read(r)?;
        let body = if initialized_raw != 0 {
            Some(ReligiousBody::read_in_version(r, wire_version)?)
        } else {
            None
        };
        Ok(Self {
            wire_version,
            initialized_raw,
            body,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 6 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        if (self.initialized_raw != 0) != self.body.is_some() {
            return Err(Error::ConditionalBody);
        }
        6u32.write(w)?;
        self.initialized_raw.write(w)?;
        if let Some(body) = &self.body {
            body.write(w)?;
        }
        Ok(())
    }
}
record!(ReligiousManager, 2, { entries:Vec<Religious> });
record!(GlobalAi, VERSION, {
    value_f8:u32, mayhem:MayhemTracker, control_factory:ControlFactory,
    espionage:EspionageManager, religious:ReligiousManager,
    value_128_raw_q8:i32, values_140:Vec<i32>, yields_a8:Vec<HashQ8>, yields_d0:Vec<HashQ8>,
});
impl GlobalAi {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            allocated: 0,
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
    fn specimen(flag: u8) -> GlobalAi {
        GlobalAi {
            value_f8: 0xf1234567,
            mayhem: MayhemTracker {
                native_f32_bits: 0x7fa12345,
            },
            control_factory: ControlFactory { next_id: u32::MAX },
            espionage: EspionageManager {
                entries: vec![Espionage {
                    wire_version: 4,
                    player_ordinal: -1,
                    control_priority: i32::MIN,
                    city_analyses: vec![EspionageCityAnalysis {
                        wire_version: 3,
                        city: CityId {
                            owner: -1,
                            component_type: i16::MIN,
                            id: u32::MAX,
                        },
                        act_values_raw_q8: [i32::MIN, 0, i32::MAX, -1, 1, -2, 2, -3, 3, -4, 4],
                        value_34: -9,
                    }],
                    flag_2a0: flag,
                    cities: Some(vec![CityId {
                        owner: i16::MAX,
                        component_type: -3,
                        id: 1,
                    }]),
                    values_158: Some(vec![-1]),
                    values_180: Some(vec![i32::MAX]),
                    values_1a8: Some(vec![i32::MIN]),
                }],
            },
            religious: ReligiousManager {
                entries: vec![Religious {
                    wire_version: 6,
                    initialized_raw: flag,
                    body: if flag == 0 {
                        None
                    } else {
                        Some(ReligiousBody {
                            player_ordinal: 63,
                            religion_hash: u32::MAX,
                            flag_118: flag,
                            beliefs: vec![HashBeliefAnalysis {
                                definition_hash: 0x80000000,
                                analysis: BeliefAnalysis {
                                    player_ordinal: -2,
                                    belief_hash: 0xfedcba98,
                                    belief_class_ordinal: i32::MIN,
                                    flag_10: flag,
                                    value_raw_q8: i32::MAX,
                                },
                            }],
                            handles: vec![ControlHandle {
                                player_ordinal: -1,
                                id: u32::MAX,
                            }],
                            handle: ControlHandle {
                                player_ordinal: 62,
                                id: 0,
                            },
                            flag_130: flag,
                            cities: vec![CityReligionAnalysis {
                                city: CityId {
                                    owner: -1,
                                    component_type: 2,
                                    id: u32::MAX,
                                },
                                position: PlotCoord {
                                    x: i32::MIN,
                                    y: i32::MAX,
                                },
                                position_10: PlotCoord { x: -1, y: -2 },
                                value_18_raw_q8: -3,
                                value_24_raw_q8: -4,
                                value_1c_raw_q8: -5,
                                flag_20: flag,
                                values_28: vec![0, u32::MAX],
                            }],
                            yields: vec![HashQ8 {
                                definition_hash: 7,
                                raw_q8: i32::MIN,
                            }],
                        })
                    },
                }],
            },
            value_128_raw_q8: i32::MIN,
            values_140: vec![-1, 0, i32::MAX],
            yields_a8: vec![
                HashQ8 {
                    definition_hash: u32::MAX,
                    raw_q8: -1,
                },
                HashQ8 {
                    definition_hash: u32::MAX,
                    raw_q8: i32::MIN,
                },
            ],
            yields_d0: vec![],
        }
    }
    #[test]
    fn religious5_zero_reset_is_not_an_empty_effective_yield_array() {
        let mut b = 5u32.to_le_bytes().to_vec();
        b.push(0xff);
        for word in [u32::MAX, 0x81234567] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        b.push(0xfe);
        for word in [0u32, 0, 1, i32::MIN as u32, u32::MAX] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        b.push(0x80);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0xaa);
        let decode = |data: &[u8], max_bytes| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                allocated: 0,
                limits: Limits {
                    max_bytes,
                    max_items: 0,
                },
            };
            Religious::read(&mut r).map(|v| (v, r.pos, r.items))
        };
        let (v, n, i) = decode(&b, 39).unwrap();
        assert_eq!((v.wire_version, v.initialized_raw, n, i), (5, 255, 39, 0));
        assert!(v.omitted_yields_reset_to_zero());
        let body = v.body.as_ref().unwrap();
        assert_eq!(
            (
                body.player_ordinal,
                body.religion_hash,
                body.flag_118,
                body.flag_130
            ),
            (-1, 0x81234567, 0xfe, 0x80)
        );
        assert!(body.yields.is_empty());
        assert_eq!(
            (body.handle.player_ordinal, body.handle.id),
            (i32::MIN, u32::MAX)
        );
        for cut in 0..39 {
            assert!(decode(&b[..cut], 39).is_err());
        }
        assert!(decode(&b, 38).is_err());
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        let mut current = b[..39].to_vec();
        current[..4].copy_from_slice(&6u32.to_le_bytes());
        assert!(decode(&current, 43).is_err());
        current.extend_from_slice(&0u32.to_le_bytes());
        let v = decode(&current, 43).unwrap().0;
        assert!(!v.omitted_yields_reset_to_zero());
        v.write(&mut w).unwrap();
        assert_eq!(w.data, current);
        let absent = [5, 0, 0, 0, 0, 0xee];
        let (v, n, i) = decode(&absent, 5).unwrap();
        assert_eq!((n, i), (5, 0));
        assert!(v.body.is_none());
        assert!(!v.omitted_yields_reset_to_zero());
        b[..4].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(
            decode(&b, 39),
            Err(Error::Version {
                expected: 6,
                actual: 4
            })
        );
    }
    #[test]
    fn espionage2_analysis2_omissions_preserve_unknown_prior_and_cursor() {
        let mut b = Vec::new();
        for word in [
            2u32,
            u32::MAX,
            i32::MIN as u32,
            1,
            2,
            0x0002ffff,
            0x81234567,
            11,
        ] {
            b.extend_from_slice(&word.to_le_bytes());
        }
        for n in 0..11i32 {
            b.extend_from_slice(&(n.wrapping_mul(i32::MIN + 1)).to_le_bytes());
        }
        b.extend_from_slice(&[0xfe, 0xaa]);
        let decode = |data: &[u8], max_bytes, max_items| {
            let mut r = Reader {
                data,
                pos: 0,
                items: 0,
                allocated: 0,
                limits: Limits {
                    max_bytes,
                    max_items,
                },
            };
            Espionage::read(&mut r).map(|v| (v, r.pos, r.items))
        };
        let (v, n, i) = decode(&b, 77, 12).unwrap();
        assert_eq!((v.wire_version, n, i), (2, 77, 12));
        assert_eq!(
            (v.player_ordinal, v.control_priority, v.flag_2a0),
            (-1, i32::MIN, 0xfe)
        );
        assert!(
            v.cities.is_none()
                && v.values_158.is_none()
                && v.values_180.is_none()
                && v.values_1a8.is_none()
        );
        let a = &v.city_analyses[0];
        assert_eq!(
            (
                a.wire_version,
                a.value_34,
                a.city.owner,
                a.city.component_type,
                a.city.id
            ),
            (2, 0, -1, 2, 0x81234567)
        );
        assert_eq!(a.act_values_raw_q8[10], 10);
        for cut in 0..77 {
            assert!(decode(&b[..cut], 77, 12).is_err());
        }
        assert!(decode(&b, 76, 12).is_err());
        assert!(decode(&b, 77, 11).is_err());
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits: limits(),
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        assert_eq!(a.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        assert!(w.data.is_empty());
        let mut current = specimen(255).espionage.entries.remove(0);
        current.values_1a8 = None;
        assert_eq!(current.write(&mut w), Err(Error::ConditionalBody));
        assert!(w.data.is_empty());
        b[16..20].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            decode(&b, 77, 12),
            Err(Error::Version {
                expected: 3,
                actual: 1
            })
        );
        b[..4].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            decode(&b, 77, 12),
            Err(Error::Version {
                expected: 4,
                actual: 3
            })
        );
    }
    #[test]
    fn exhaustive_raw_bool_carriers_and_small_truncations() {
        for flag in 0..=255 {
            let v = specimen(flag);
            let (raw, items) = v.encode_with_items(limits()).unwrap();
            let exact = Limits {
                max_bytes: raw.len(),
                max_items: items,
            };
            assert_eq!(GlobalAi::decode(&raw, exact).unwrap(), v);
            for end in 0..raw.len() {
                assert!(GlobalAi::decode(&raw[..end], exact).is_err());
            }
            let short = Limits {
                max_items: items - 1,
                ..exact
            };
            assert_eq!(GlobalAi::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
            let short = Limits {
                max_bytes: raw.len() - 1,
                ..exact
            };
            assert!(GlobalAi::decode(&raw, short).is_err());
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn native_float_bits_and_exact_empty_wire() {
        for bits in [
            0,
            1,
            0x80000000,
            0x7f800000,
            0xff800000,
            0x7fa12345,
            u32::MAX,
        ] {
            let words = [4, 7, 1, bits, 1, 9, 1, 0, 2, 0, 0x80000000, 0, 0, 0];
            let raw: Vec<_> = words.iter().flat_map(|x: &u32| x.to_le_bytes()).collect();
            let exact = Limits {
                max_bytes: raw.len(),
                max_items: 0,
            };
            let v = GlobalAi::decode(&raw, exact).unwrap();
            assert_eq!(v.mayhem.native_f32_bits, bits);
            assert_eq!(v.encode_with_items(exact).unwrap(), (raw, 0));
        }
    }
    #[test]
    fn hostile_counts_versions_and_inconsistent_condition() {
        let v = specimen(255);
        let raw = v.encode(limits()).unwrap();
        let mut bad = raw.clone();
        bad[..4].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            GlobalAi::decode(&bad, limits()),
            Err(Error::Version {
                expected: 4,
                actual: 3
            })
        );
        for offset in [8, 16, 24, 32, 48] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert!(matches!(
                GlobalAi::decode(&bad, limits()),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = raw.clone();
        bad[60..64].copy_from_slice(&10u32.to_le_bytes());
        assert_eq!(
            GlobalAi::decode(&bad, limits()),
            Err(Error::ArrayLength {
                expected: 11,
                actual: 10
            })
        );
        let mut bad = raw;
        bad[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(GlobalAi::decode(&bad, limits()), Err(Error::ResourceLimit));
        let mut v = v;
        v.religious.entries[0].initialized_raw = 0;
        assert_eq!(v.encode(limits()), Err(Error::ConditionalBody));
    }
}
