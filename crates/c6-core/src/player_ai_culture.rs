//! Exact current Linux XP2 AI.CultureAi saved wire, distinct from Player.Culture.
//! No native lookups, archive changes, analyzers, callbacks, decisions or RNG run.
use crate::unit::Limits;
pub const VERSION: u32 = 15;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.CultureAi wire: {self:?}")
    }
}
impl std::error::Error for Error {}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let raw = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(raw.try_into().unwrap())
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
    data: Option<Vec<u8>>,
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn put(&mut self, raw: &[u8]) -> Result<(), Error> {
        self.pos = self
            .pos
            .checked_add(raw.len())
            .ok_or(Error::ResourceLimit)?;
        if self.pos > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if let Some(out) = &mut self.data {
            out.extend_from_slice(raw)
        }
        Ok(())
    }
}
macro_rules! scalar {
    ($t:ty,$n:expr) => {
        impl Wire for $t {
            const MIN: usize = $n;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_le_bytes(r.take()?))
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
        }
    };
}
scalar!(u32, 4);
scalar!(i32, 4);
/// Saved native bool object byte; no normalization or truth-value claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoolByte(pub u8);
impl Wire for BoolByte {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self(r.take::<1>()?[0]))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[self.0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.items = r.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        let end = n
            .checked_mul(T::MIN)
            .and_then(|n| r.pos.checked_add(n))
            .ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items || end > r.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > r.data.len() {
            return Err(Error::Truncated);
        }
        n.checked_mul(std::mem::size_of::<T>())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut v = Self::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let n = u32::try_from(self.len()).map_err(|_| Error::ResourceLimit)?;
        w.items = w
            .items
            .checked_add(self.len())
            .ok_or(Error::ResourceLimit)?;
        if w.items > w.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        n.write(w)?;
        for v in self {
            v.write(w)?
        }
        Ok(())
    }
}
/// Definition hash and value, preserving saved order, duplicates and sentinels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashEntry<T> {
    pub hash: u32,
    pub value: T,
}
impl<T: Wire> Wire for HashEntry<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.hash.write(w)?;
        self.value.write(w)
    }
}
macro_rules! record {
 ($name:ident,$version:expr,{$($field:ident:$ty:ty),* $(,)?})=>{
 #[derive(Clone,Debug,Eq,PartialEq)] pub struct $name {$(pub $field:$ty),*}
 impl Wire for $name {const MIN:usize=4$(+<$ty as Wire>::MIN)*;
 fn read(r:&mut Reader<'_>)->Result<Self,Error>{r.version($version)?;Ok(Self{$($field:<$ty as Wire>::read(r)?),*})}
 fn write(&self,w:&mut Writer)->Result<(),Error>{($version as u32).write(w)?;$(self.$field.write(w)?;)*Ok(())}}
 };
}
record!(CivicAnalysis,2,{player_id:i32,civic_hash:u32,next_civic_needed_hash:u32,is_owned:BoolByte,is_favored:BoolByte,has_boost:BoolByte,is_repeatable:BoolByte,solo_value_q8:i32,full_value_q8:i32,turns_to_unlock:i32});
record!(PolicyAnalysis,4,{player_id:i32,policy_hash:u32,slot_type_hash:u32,dark_age_policy:BoolByte,golden_age_policy:BoolByte,min_era_chronology:i32,max_era_chronology:i32,value_q8:i32});
record!(ResolutionAnalysis,2,{player_id:i32,resolution_hash:u32,value_q8:i32,option_one_value_q8:i32,option_two_value_q8:i32,best_option:i32});
// PlotCoord is two signed 32-bit coordinates, without a version word.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotCoord {
    pub x: i32,
    pub y: i32,
}
impl Wire for PlotCoord {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            x: i32::read(r)?,
            y: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.x.write(w)?;
        self.y.write(w)
    }
}
record!(RockBandAnalysis,1,{player_id:i32,base_value_q8:i32,location:PlotCoord,district_hash:u32,improvement_hash:u32,national_park:BoolByte});

record!(ControlHandle,1,{player_id:i32,id:u32});
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveBoost {
    pub behavior_tree_control_id: u32,
    pub boost_uniqueness_hash: i32,
}
impl Wire for ActiveBoost {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            behavior_tree_control_id: u32::read(r)?,
            boost_uniqueness_hash: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.behavior_tree_control_id.write(w)?;
        self.boost_uniqueness_hash.write(w)
    }
}
record!(BoostManager,1,{player_id:i32,allowed_active_boosts:u32,active_boosts:Vec<ActiveBoost>});
record!(GreatWorks,3,{player_id:i32,great_work_values_q8:Vec<i32>,working_great_work_values_q8:Vec<i32>});
record!(CultureAi,VERSION,{
 player_id:i32,managed_contracts:Vec<ControlHandle>,boost_manager:BoostManager,done_off_turn:BoolByte,
 congress_pseudo_yield_preferences:Vec<HashEntry<i32>>,pseudo_yield_preferences:Vec<HashEntry<i32>>,best_slot_policy_values:Vec<HashEntry<i32>>,
 civic_analyzers:Vec<HashEntry<CivicAnalysis>>,policy_analyzers:Vec<HashEntry<PolicyAnalysis>>,resolution_analyzers:Vec<HashEntry<ResolutionAnalysis>>,
 govt_values:Vec<HashEntry<i32>>,rockband_analyzers:Vec<RockBandAnalysis>,great_work_manager:GreatWorks,national_park_spots:Vec<i32>,num_naturalists:i32,must_change_policies:BoolByte
});
macro_rules! bounded_codec {
    ($name:ident) => {
        impl $name {
            pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
                let (v, n) = Self::decode_prefix(data, limits)?;
                if n != data.len() {
                    return Err(Error::Trailing);
                }
                Ok(v)
            }
            pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                Self::decode_prefix_with_items(data, limits).map(|(v, n, _)| (v, n))
            }
            pub fn decode_prefix_with_items(
                data: &[u8],
                limits: Limits,
            ) -> Result<(Self, usize, usize), Error> {
                let mut r = Reader {
                    data,
                    pos: 0,
                    items: 0,
                    limits,
                };
                let v = Self::read(&mut r)?;
                Ok((v, r.pos, r.items))
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                self.encode_with_items(limits).map(|(v, _)| v)
            }
            pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
                let mut w = Writer {
                    data: None,
                    pos: 0,
                    items: 0,
                    limits,
                };
                self.write(&mut w)?;
                let mut out = Vec::new();
                out.try_reserve_exact(w.pos)
                    .map_err(|_| Error::ResourceLimit)?;
                let items = w.items;
                w = Writer {
                    data: Some(out),
                    pos: 0,
                    items: 0,
                    limits,
                };
                self.write(&mut w)?;
                Ok((w.data.unwrap(), items))
            }
        }
    };
}
bounded_codec!(CultureAi);
bounded_codec!(BoostManager);
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> CultureAi {
        CultureAi {
            player_id: -1,
            managed_contracts: vec![ControlHandle {
                player_id: 3,
                id: u32::MAX,
            }],
            boost_manager: BoostManager {
                player_id: -1,
                allowed_active_boosts: 2,
                active_boosts: vec![ActiveBoost {
                    behavior_tree_control_id: u32::MAX,
                    boost_uniqueness_hash: i32::MIN,
                }],
            },
            done_off_turn: BoolByte(1),
            congress_pseudo_yield_preferences: vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }],
            pseudo_yield_preferences: vec![],
            best_slot_policy_values: vec![],
            civic_analyzers: vec![HashEntry {
                hash: 8,
                value: CivicAnalysis {
                    player_id: -1,
                    civic_hash: 8,
                    next_civic_needed_hash: u32::MAX,
                    is_owned: BoolByte(1),
                    is_favored: BoolByte(0),
                    has_boost: BoolByte(1),
                    is_repeatable: BoolByte(0),
                    solo_value_q8: i32::MIN,
                    full_value_q8: i32::MAX,
                    turns_to_unlock: -9,
                },
            }],
            policy_analyzers: vec![HashEntry {
                hash: 7,
                value: PolicyAnalysis {
                    player_id: -1,
                    policy_hash: 7,
                    slot_type_hash: 6,
                    dark_age_policy: BoolByte(0),
                    golden_age_policy: BoolByte(1),
                    min_era_chronology: -1,
                    max_era_chronology: i32::MAX,
                    value_q8: -256,
                },
            }],
            resolution_analyzers: vec![HashEntry {
                hash: 5,
                value: ResolutionAnalysis {
                    player_id: -1,
                    resolution_hash: 5,
                    value_q8: -256,
                    option_one_value_q8: 1,
                    option_two_value_q8: 2,
                    best_option: -1,
                },
            }],
            govt_values: vec![],
            rockband_analyzers: vec![RockBandAnalysis {
                player_id: 4,
                base_value_q8: -1,
                location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MAX,
                },
                district_hash: u32::MAX,
                improvement_hash: 3,
                national_park: BoolByte(1),
            }],
            great_work_manager: GreatWorks {
                player_id: -1,
                great_work_values_q8: vec![i32::MIN],
                working_great_work_values_q8: vec![i32::MAX],
            },
            national_park_spots: vec![-1, 8],
            num_naturalists: -8,
            must_change_policies: BoolByte(1),
        }
    }
    #[test]
    fn nested_boundaries() {
        let v = sample();
        let raw = v.encode(Limits::default()).unwrap();
        let (_, items) = v.encode_with_items(Limits::default()).unwrap();
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(CultureAi::decode(&raw, cap), Ok(v.clone()));
        for n in 0..raw.len() {
            assert!(CultureAi::decode(&raw[..n], cap).is_err())
        }
        for short in [
            Limits {
                max_bytes: raw.len() - 1,
                ..cap
            },
            Limits {
                max_items: items - 1,
                ..cap
            },
        ] {
            assert_eq!(CultureAi::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.push(7);
        assert_eq!(CultureAi::decode(&tail, cap), Err(Error::Trailing));
        assert_eq!(
            CultureAi::decode_prefix_with_items(&tail, cap),
            Ok((v, raw.len(), items))
        );
    }
    #[test]
    fn hostile_counts_versions_bools() {
        let raw = sample().encode(Limits::default()).unwrap();
        for (offset, expected) in [(0, 15), (12, 1), (24, 1)] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                CultureAi::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        let mut bad = raw.clone();
        bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            CultureAi::decode(&bad, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let mut bad = raw;
        bad[48] = 2;
        assert_eq!(
            CultureAi::decode(&bad, Limits::default()),
            Ok({
                let mut v = sample();
                v.done_off_turn = BoolByte(2);
                v
            })
        );
    }
    #[test]
    fn shared_boost_manager_boundary() {
        // Direct target-schema carrier: v1, signed owner, allowed, count,
        // then two ordered raw control-ID/signed uniqueness pairs.
        let raw: Vec<u8> = [1u32, 3, 2, 2, u32::MAX, 0x8000_0000, 7, (-9i32) as u32]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        let expected = BoostManager {
            player_id: 3,
            allowed_active_boosts: 2,
            active_boosts: vec![
                ActiveBoost {
                    behavior_tree_control_id: u32::MAX,
                    boost_uniqueness_hash: i32::MIN,
                },
                ActiveBoost {
                    behavior_tree_control_id: 7,
                    boost_uniqueness_hash: -9,
                },
            ],
        };
        let exact = Limits {
            max_bytes: 32,
            max_items: 2,
        };
        assert_eq!(BoostManager::decode(&raw, exact), Ok(expected.clone()));
        assert_eq!(expected.encode_with_items(exact), Ok((raw.clone(), 2)));
        for n in 0..raw.len() {
            assert!(BoostManager::decode(&raw[..n], exact).is_err());
        }
        for short in [
            Limits {
                max_bytes: 31,
                ..exact
            },
            Limits {
                max_items: 1,
                ..exact
            },
        ] {
            assert_eq!(BoostManager::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(expected.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.extend_from_slice(&[99; 64]);
        assert_eq!(BoostManager::decode(&tail, exact), Err(Error::Trailing));
        assert_eq!(
            BoostManager::decode_prefix(&tail, exact),
            Ok((expected.clone(), 32))
        );
        assert_eq!(
            BoostManager::decode_prefix_with_items(&tail, exact),
            Ok((expected, 32, 2))
        );
        let empty = BoostManager {
            player_id: -1,
            allowed_active_boosts: 0,
            active_boosts: vec![],
        };
        let empty_cap = Limits {
            max_bytes: 16,
            max_items: 0,
        };
        let bytes = empty.encode(empty_cap).unwrap();
        assert_eq!(BoostManager::decode(&bytes, empty_cap), Ok(empty));
        let mut hostile = bytes;
        hostile[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            BoostManager::decode(&hostile, empty_cap),
            Err(Error::ResourceLimit)
        );
    }
}
