//! Exact current Linux XP2 AI.Economic saved wire, including the separate AI.Trade.Player v3.
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
        write!(f, "Linux AI.Economic wire: {self:?}")
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
record!(ControlHandle,1,{player_id:i32,id:u32});
record!(TradePlayer,3,{player_id:i32,control_id:u32,best_tech_driven_road_hash:u32,turns_since_railroad_tree:i32});
record!(Economic,4,{player_id:i32,current_balance_q8:i32,city_income_q8:i32,religious_income_q8:i32,diplomatic_income_q8:i32,percentage_bonuses_income_q8:i32,city_expenses_q8:i32,turns_with_negative_income:i32,total_saving_weight:i32,saving_weights:Vec<i32>,savings_q8:Vec<i32>,yield_priority_boost_q8:Vec<HashEntry<i32>>,managed_contracts:Vec<ControlHandle>,trade_manager:TradePlayer});
impl Economic {
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
    fn sample() -> Economic {
        Economic {
            player_id: -1,
            current_balance_q8: i32::MIN,
            city_income_q8: i32::MAX,
            religious_income_q8: -1,
            diplomatic_income_q8: -256,
            percentage_bonuses_income_q8: 7,
            city_expenses_q8: 9,
            turns_with_negative_income: -1,
            total_saving_weight: i32::MIN,
            saving_weights: vec![-1, i32::MAX],
            savings_q8: vec![i32::MIN],
            yield_priority_boost_q8: vec![
                HashEntry {
                    hash: u32::MAX,
                    value: -1,
                },
                HashEntry {
                    hash: u32::MAX,
                    value: i32::MIN,
                },
            ],
            managed_contracts: vec![ControlHandle {
                player_id: -1,
                id: u32::MAX,
            }],
            trade_manager: TradePlayer {
                player_id: -1,
                control_id: u32::MAX,
                best_tech_driven_road_hash: u32::MAX,
                turns_since_railroad_tree: i32::MIN,
            },
        }
    }
    #[test]
    fn nested_boundaries() {
        let v = sample();
        let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
        assert_eq!(items, 6);
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(Economic::decode(&raw, cap), Ok(v.clone()));
        for n in 0..raw.len() {
            assert!(Economic::decode(&raw[..n], cap).is_err());
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
            assert_eq!(Economic::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.push(7);
        assert_eq!(Economic::decode(&tail, cap), Err(Error::Trailing));
        assert_eq!(
            Economic::decode_prefix_with_items(&tail, cap),
            Ok((v, raw.len(), items))
        );
    }
    #[test]
    fn hostile_counts_versions() {
        let raw = sample().encode(Limits::default()).unwrap();
        for (offset, expected) in [(0, 4), (84, 1), (96, 3)] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Economic::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        for offset in [40, 52, 60, 80] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Economic::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
    }
}
