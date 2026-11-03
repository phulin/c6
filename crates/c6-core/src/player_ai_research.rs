//! Exact current Linux XP2 AI.Research saved wire.
//! No native lookups, archive changes, analyzers, callbacks, decisions or RNG run.
use crate::unit::Limits;
pub const VERSION: u32 = 4;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Research wire: {self:?}")
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
pub use crate::player_ai_culture::BoostManager;
impl From<crate::player_ai_culture::Error> for Error {
    fn from(e: crate::player_ai_culture::Error) -> Self {
        use crate::player_ai_culture::Error as E;
        match e {
            E::Truncated => Self::Truncated,
            E::Trailing => Self::Trailing,
            E::ResourceLimit => Self::ResourceLimit,
            E::Version { expected, actual } => Self::Version { expected, actual },
        }
    }
}
impl Wire for BoostManager {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let limits = Limits {
            max_bytes: r
                .limits
                .max_bytes
                .checked_sub(r.pos)
                .ok_or(Error::ResourceLimit)?,
            max_items: r
                .limits
                .max_items
                .checked_sub(r.items)
                .ok_or(Error::ResourceLimit)?,
        };
        let (v, n, items) = Self::decode_prefix_with_items(&r.data[r.pos..], limits)?;
        r.pos = r.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        r.items = r.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let limits = Limits {
            max_bytes: w
                .limits
                .max_bytes
                .checked_sub(w.pos)
                .ok_or(Error::ResourceLimit)?,
            max_items: w
                .limits
                .max_items
                .checked_sub(w.items)
                .ok_or(Error::ResourceLimit)?,
        };
        let (bytes, items) = self.encode_with_items(limits)?;
        w.items = w.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        w.put(&bytes)
    }
}
record!(ControlHandle,1,{player_id:i32,id:u32});
// Unsaved m_units/m_buildings are not part of this record.
record!(TechTreeAnalysis,3,{tech_hash:u32,next_tech_hash:u32,player_id:i32,turns_to_research:i32,value_q8:i32,full_value_q8:i32,has_tech:BoolByte,is_repeatable:BoolByte,has_boost:BoolByte,is_leaf:BoolByte});
record!(Research,VERSION,{player_id:i32,research_target_hash:u32,turns_to_research_unit:Vec<HashEntry<i32>>,favored_techs:Vec<HashEntry<BoolByte>>,tree_analysis:Vec<HashEntry<TechTreeAnalysis>>,done_off_turn:BoolByte,managed_contracts:Vec<ControlHandle>,boost_manager:BoostManager,turns_remaining_on_current_research:i32});
impl Research {
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
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Research {
        Research {
            player_id: -1,
            research_target_hash: u32::MAX,
            turns_to_research_unit: vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }],
            favored_techs: vec![HashEntry {
                hash: 0,
                value: BoolByte(254),
            }],
            tree_analysis: vec![HashEntry {
                hash: 17,
                value: TechTreeAnalysis {
                    tech_hash: 17,
                    next_tech_hash: u32::MAX,
                    player_id: 4,
                    turns_to_research: -2,
                    value_q8: i32::MIN,
                    full_value_q8: i32::MAX,
                    has_tech: BoolByte(2),
                    is_repeatable: BoolByte(255),
                    has_boost: BoolByte(127),
                    is_leaf: BoolByte(128),
                },
            }],
            done_off_turn: BoolByte(249),
            managed_contracts: vec![ControlHandle {
                player_id: -1,
                id: u32::MAX,
            }],
            boost_manager: BoostManager {
                player_id: -1,
                allowed_active_boosts: 0,
                active_boosts: vec![crate::player_ai_culture::ActiveBoost {
                    behavior_tree_control_id: u32::MAX,
                    boost_uniqueness_hash: i32::MIN,
                }],
            },
            turns_remaining_on_current_research: i32::MIN,
        }
    }
    #[test]
    fn research_raw_and_limits() {
        let v = sample();
        let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
        let limits = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(Research::decode(&raw, limits), Ok(v.clone()));
        for end in 0..raw.len() {
            assert!(Research::decode(&raw[..end], limits).is_err());
        }
        for l in [
            Limits {
                max_bytes: raw.len() - 1,
                ..limits
            },
            Limits {
                max_items: items - 1,
                ..limits
            },
        ] {
            assert_eq!(Research::decode(&raw, l), Err(Error::ResourceLimit));
            assert_eq!(v.encode(l), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.extend_from_slice(&[0; 20]);
        assert_eq!(Research::decode(&tail, limits), Err(Error::Trailing));
        assert_eq!(
            Research::decode_prefix_with_items(&tail, limits),
            Ok((v, raw.len(), items))
        );
    }
    #[test]
    fn research_hostile() {
        let raw = sample().encode(Limits::default()).unwrap();
        for at in [0, 41, 78, 90] {
            let mut bad = raw.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Research::decode(
                &bad,
                Limits {
                    max_bytes: 4096,
                    max_items: 100
                }
            )
            .is_err());
        }
        for at in [12, 24, 33, 74, 102] {
            let mut bad = raw.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Research::decode(
                &bad,
                Limits {
                    max_bytes: 4096,
                    max_items: 100
                }
            )
            .is_err());
        }
    }
}
