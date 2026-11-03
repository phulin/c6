//! Current Linux XP2 GlobalAi v4 wire transport, including concrete nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 4;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
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
record!(EspionageCityAnalysis, 3, { city:CityId, act_values_raw_q8:[i32;11], value_34:i32 });
record!(Espionage, 4, {
    player_ordinal:i32, control_priority:i32,
    city_analyses:Vec<EspionageCityAnalysis>, flag_2a0:u8,
    cities:Vec<CityId>, values_158:Vec<i32>, values_180:Vec<i32>, values_1a8:Vec<i32>,
});
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
    pub initialized_raw: u8,
    pub body: Option<ReligiousBody>,
}
record!(ReligiousBody, 0, {
    player_ordinal:i32, religion_hash:u32, flag_118:u8,
    beliefs:Vec<HashBeliefAnalysis>, handles:Vec<ControlHandle>, handle:ControlHandle,
    flag_130:u8, cities:Vec<CityReligionAnalysis>, yields:Vec<HashQ8>,
});
impl Wire for Religious {
    const MIN: usize = 5;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(6)?;
        let initialized_raw = u8::read(r)?;
        let body = if initialized_raw != 0 {
            Some(ReligiousBody::read(r)?)
        } else {
            None
        };
        Ok(Self {
            initialized_raw,
            body,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
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
                    player_ordinal: -1,
                    control_priority: i32::MIN,
                    city_analyses: vec![EspionageCityAnalysis {
                        city: CityId {
                            owner: -1,
                            component_type: i16::MIN,
                            id: u32::MAX,
                        },
                        act_values_raw_q8: [i32::MIN, 0, i32::MAX, -1, 1, -2, 2, -3, 3, -4, 4],
                        value_34: -9,
                    }],
                    flag_2a0: flag,
                    cities: vec![CityId {
                        owner: i16::MAX,
                        component_type: -3,
                        id: 1,
                    }],
                    values_158: vec![-1],
                    values_180: vec![i32::MAX],
                    values_1a8: vec![i32::MIN],
                }],
            },
            religious: ReligiousManager {
                entries: vec![Religious {
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
        for offset in [8, 16, 24] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert!(matches!(
                GlobalAi::decode(&bad, limits()),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = raw;
        bad[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(GlobalAi::decode(&bad, limits()), Err(Error::ResourceLimit));
        let mut v = v;
        v.religious.entries[0].initialized_raw = 0;
        assert_eq!(v.encode(limits()), Err(Error::ConditionalBody));
    }
}
