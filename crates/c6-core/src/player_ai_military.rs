//! Exact bounded Linux XP2 AI.Military saved wire. No native loading or policy execution.
use crate::unit::Limits;
pub const VERSION: u32 = 14;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingContext,
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    BlockSize(u32),
    FixedCount(u32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Military wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Native Military initialization determines this unsaved branch per player.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Context {
    pub operation_type_arrays_present: bool,
}
struct Reader<'a> {
    context: Option<Context>,
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
struct Writer {
    data: Vec<u8>,
    bytes: usize,
    items: usize,
    limits: Limits,
    measuring: bool,
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b.try_into().unwrap())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn reserve<T>(&mut self, n: usize, min: usize) -> Result<Vec<T>, Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let end = self
            .pos
            .checked_add(n.checked_mul(min).ok_or(Error::ResourceLimit)?)
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        n.checked_mul(std::mem::size_of::<T>())
            .filter(|x| *x <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        Ok(v)
    }
    fn sequence<T: Wire>(&mut self, n: usize) -> Result<Vec<T>, Error> {
        let mut v = self.reserve::<T>(n, T::MIN)?;
        for _ in 0..n {
            v.push(T::read(self)?);
        }
        Ok(v)
    }
}
impl Writer {
    fn put(&mut self, data: &[u8]) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_add(data.len())
            .ok_or(Error::ResourceLimit)?;
        if self.bytes > self.limits.max_bytes || self.bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        if !self.measuring {
            self.data.extend_from_slice(data);
        }
        Ok(())
    }
    fn sequence<T: Wire>(&mut self, v: &[T]) -> Result<(), Error> {
        self.items = self
            .items
            .checked_add(v.len())
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        for x in v {
            x.write(self)?;
        }
        Ok(())
    }
}
macro_rules! scalar {
    ($t:ty, $n:expr) => {
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
scalar!(i16, 2);
scalar!(u8, 1);
/// Saved native bool storage. Noncanonical bytes occur in original PlotEvaluation
/// output; transport preserves them without inventing a gameplay truth value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
        r.sequence(n)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        w.sequence(self)
    }
}
impl<T: Wire + Copy + Default, const N: usize> Wire for [T; N] {
    const MIN: usize = T::MIN * N;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let mut v = [T::default(); N];
        for x in &mut v {
            *x = T::read(r)?;
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        for x in self {
            x.write(w)?;
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, {$($field:ident: $ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = if $version == 0 { 0 } else { 4 } $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version != 0 { r.version($version)?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version != 0 { ($version as u32).write(w)?; }
                $(self.$field.write(w)?;)* Ok(())
            }
        }
    };
}

record!(ComponentId,0,{owner:i16,component_type:i16,id:u32});
record!(HashValue,0,{hash:u32,value:i32});
pub type PlotCoord = [i32; 2];
include!("player_ai_military_operations.rs");
record!(DefenseAssignment,1,{unit:u32,location:PlotCoord});
record!(MilitaryDefense,1,{player_id:i32,defense_type:i32,subject:u32,desired_units:i32,current_units:i32,assignments:Vec<DefenseAssignment>});
record!(OperationEntry,0,{index:i32,operation:Operation});
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecycledOperations {
    pub capacity: u32,
    pub entries: Vec<OperationEntry>,
    pub current_id_bits: u32,
    pub first_free: u32,
    pub last_free: u32,
    pub free_indices: Vec<u32>,
}
impl Wire for RecycledOperations {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(0x10001)?;
        let block = u32::read(r)?;
        if block != 16 {
            return Err(Error::BlockSize(block));
        }
        let capacity = u32::read(r)?;
        let entries = Vec::read(r)?;
        let current_id_bits = u32::read(r)?;
        let count = u32::read(r)? as usize;
        let first_free = u32::read(r)?;
        let last_free = u32::read(r)?;
        let free_indices = r.sequence(count)?;
        Ok(Self {
            capacity,
            entries,
            current_id_bits,
            first_free,
            last_free,
            free_indices,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        0x10001u32.write(w)?;
        16u32.write(w)?;
        self.capacity.write(w)?;
        self.entries.write(w)?;
        self.current_id_bits.write(w)?;
        u32::try_from(self.free_indices.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        self.first_free.write(w)?;
        self.last_free.write(w)?;
        w.sequence(&self.free_indices)
    }
}
/// The current WriteArray has exactly 64 entries, including its saved count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionalStrength(pub [i32; 64]);
impl Default for RegionalStrength {
    fn default() -> Self {
        Self([0; 64])
    }
}
impl Wire for RegionalStrength {
    const MIN: usize = 260;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)?;
        if n != 64 {
            return Err(Error::FixedCount(n));
        }
        let mut out = [0; 64];
        r.items = r.items.checked_add(64).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        for x in &mut out {
            *x = i32::read(r)?
        }
        Ok(Self(out))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        64u32.write(w)?;
        w.sequence(&self.0)
    }
}
record!(OperationTypes,0,{allowed:Vec<HashValue>,per_war:Vec<HashValue>,current:Vec<HashValue>});
impl Wire for Option<OperationTypes> {
    const MIN: usize = 0;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        if r.context
            .ok_or(Error::MissingContext)?
            .operation_type_arrays_present
        {
            Ok(Some(OperationTypes::read(r)?))
        } else {
            Ok(None)
        }
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if let Some(v) = self {
            v.write(w)?
        }
        Ok(())
    }
}
record!(Military,VERSION,{
 player_id:i32,regional_combat_strength:i32,regional_enemy_strength:i32,regional_other_strength:i32,
 on_war_footing:BoolByte,turns_cities_taken:Vec<i32>,desired_explorers:i32,desired_naval_explorers:i32,current_explorers:i32,current_naval_explorers:i32,
 allow_unit_construction:BoolByte,most_desired_technology_hash:u32,prepare_for_war_with:Vec<BoolByte>,player_rivals:Vec<BoolByte>,
 operation_types:Option<OperationTypes>,
 active_operations:RecycledOperations,regional_military_strength:RegionalStrength,modified_unit_preferences:Vec<HashValue>,pseudo_yield_preferences:Vec<HashValue>,
 civ_defense:Vec<MilitaryDefense>,levy_military_cost:Vec<i32>,levy_military_value:Vec<i32>
});
impl Military {
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
        Self::decode_prefix_with_items_context(data, limits, None)
    }
    /// Context is explicit: the branch has no saved discriminator.
    pub fn decode_prefix_with_items_in_context(
        data: &[u8],
        limits: Limits,
        context: Context,
    ) -> Result<(Self, usize, usize), Error> {
        Self::decode_prefix_with_items_context(data, limits, Some(context))
    }
    fn decode_prefix_with_items_context(
        data: &[u8],
        limits: Limits,
        context: Option<Context>,
    ) -> Result<(Self, usize, usize), Error> {
        if context.is_none() {
            return Err(Error::MissingContext);
        }
        let mut r = Reader {
            context,
            data,
            pos: 0,
            items: 0,
            limits,
        };
        let v = Self::read(&mut r)?;
        Ok((v, r.pos, r.items))
    }
    pub fn decode_in_context(data: &[u8], limits: Limits, context: Context) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items_in_context(data, limits, context)?;
        if n != data.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix_in_context(
        data: &[u8],
        limits: Limits,
        context: Context,
    ) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items_in_context(data, limits, context).map(|(v, n, _)| (v, n))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        // Measure every nested byte/item before reserving or producing output.
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: true,
        };
        self.write(&mut w)?;
        w.data
            .try_reserve_exact(w.bytes)
            .map_err(|_| Error::ResourceLimit)?;
        let items = w.items;
        w.bytes = 0;
        w.items = 0;
        w.measuring = false;
        self.write(&mut w)?;
        Ok((w.data, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            max_bytes: 100_000,
            max_items: 10_000,
        }
    }
    #[test]
    fn player_ai_military_context_and_every_small_carrier_cut() {
        for present in [false, true] {
            let v = Military {
                player_id: -1,
                operation_types: present.then(OperationTypes::default),
                ..Military::default()
            };
            let context = Context {
                operation_type_arrays_present: present,
            };
            let (raw, items) = v.encode_with_items(limits()).unwrap();
            let exact = Limits {
                max_bytes: raw.len(),
                max_items: items,
            };
            assert_eq!(Military::decode(&raw, exact), Err(Error::MissingContext));
            assert_eq!(
                Military::decode_in_context(&raw, exact, context).unwrap(),
                v
            );
            for end in 0..raw.len() {
                assert!(Military::decode_in_context(&raw[..end], exact, context).is_err());
            }
            for short in [
                Limits {
                    max_bytes: raw.len() - 1,
                    ..exact
                },
                Limits {
                    max_items: items - 1,
                    ..exact
                },
            ] {
                assert_eq!(
                    Military::decode_in_context(&raw, short, context),
                    Err(Error::ResourceLimit)
                );
                assert_eq!(v.encode(short), Err(Error::ResourceLimit));
            }
            let mut trailing = raw.clone();
            trailing.push(99);
            assert_eq!(
                Military::decode_in_context(&trailing, exact, context),
                Err(Error::Trailing)
            );
            assert_eq!(
                Military::decode_prefix_with_items_in_context(&trailing, exact, context).unwrap(),
                (v, raw.len(), items)
            );
        }
    }
    #[test]
    fn player_ai_military_raw_bool_hostile_counts_and_versions() {
        let context = Context {
            operation_type_arrays_present: true,
        };
        for b in 0..=u8::MAX {
            let v = Military {
                on_war_footing: BoolByte(b),
                allow_unit_construction: BoolByte(b),
                prepare_for_war_with: vec![BoolByte(b)],
                player_rivals: vec![BoolByte(b)],
                operation_types: Some(OperationTypes::default()),
                ..Military::default()
            };
            let raw = v.encode(limits()).unwrap();
            assert_eq!(
                Military::decode_in_context(&raw, limits(), context).unwrap(),
                v
            );
        }
        let raw = Military {
            operation_types: Some(OperationTypes::default()),
            ..Military::default()
        }
        .encode(limits())
        .unwrap();
        for version in [0, 1, 13, 15, u32::MAX] {
            let mut bad = raw.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                Military::decode_in_context(&bad, limits(), context),
                Err(Error::Version {
                    expected: VERSION,
                    actual: version
                })
            );
        }
        let mut bad = raw;
        bad[21..25].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Military::decode_in_context(&bad, limits(), context),
            Err(Error::ResourceLimit)
        );
    }
}
