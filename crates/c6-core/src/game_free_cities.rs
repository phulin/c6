//! Current Linux XP2 FreeCities Manager v3 wire transport, including concrete nested records.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 3;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux FreeCities wire: {self:?}")
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

// Raw bool storage and definition hashes are preserved without lookup or normalization.
record!(HashBool,0,{definition_hash:u32,value_raw:u8});
record!(PlotCoord,0,{x:i32,y:i32});
record!(Instance,5,{
    player_ordinal:i32,city_index:i32,city_location:PlotCoord,living_raw:u8,
    turns_since_last_unit:i32,urban_defenses:i32,retaliation_target_ordinal:i32,
    retaliation_behavior_tree:u32,buildable_units:Vec<HashBool>,buildable_walls:Vec<HashBool>,
});
record!(Manager,VERSION,{
    player_ordinal:i32,control_priority_ordinal:i32,instances:Vec<Instance>,
});
impl Manager {
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
    fn sample(raw: u8) -> Manager {
        Manager {
            player_ordinal: i32::MIN,
            control_priority_ordinal: i32::MAX,
            instances: vec![Instance {
                player_ordinal: -1,
                city_index: i32::MIN,
                city_location: PlotCoord { x: i32::MAX, y: -1 },
                living_raw: raw,
                turns_since_last_unit: i32::MIN,
                urban_defenses: i32::MAX,
                retaliation_target_ordinal: -1,
                retaliation_behavior_tree: u32::MAX,
                buildable_units: vec![
                    HashBool {
                        definition_hash: u32::MAX,
                        value_raw: raw,
                    },
                    HashBool {
                        definition_hash: 0,
                        value_raw: raw,
                    },
                    HashBool {
                        definition_hash: u32::MAX,
                        value_raw: raw,
                    },
                ],
                buildable_walls: vec![HashBool {
                    definition_hash: 0x80000000,
                    value_raw: raw,
                }],
            }],
        }
    }
    #[test]
    fn all_bool_carriers_and_small_cuts() {
        for flag in 0..=255 {
            let value = sample(flag);
            let (bytes, items) = value
                .encode_with_items(Limits {
                    max_bytes: 1024,
                    max_items: 1024,
                })
                .unwrap();
            let exact = Limits {
                max_bytes: bytes.len(),
                max_items: items,
            };
            assert_eq!(
                Manager::decode_prefix_with_items(&bytes, exact).unwrap(),
                (value.clone(), bytes.len(), items)
            );
            for end in 0..bytes.len() {
                assert!(Manager::decode(&bytes[..end], exact).is_err());
            }
            for short in [
                Limits {
                    max_bytes: bytes.len() - 1,
                    ..exact
                },
                Limits {
                    max_items: items - 1,
                    ..exact
                },
            ] {
                assert_eq!(Manager::decode(&bytes, short), Err(Error::ResourceLimit));
                assert_eq!(value.encode(short), Err(Error::ResourceLimit));
            }
            let mut suffix = bytes.clone();
            suffix.push(0);
            assert_eq!(Manager::decode(&suffix, exact), Err(Error::Trailing));
            assert_eq!(
                Manager::decode_prefix(&suffix, exact).unwrap(),
                (value, bytes.len())
            );
        }
    }
    #[test]
    fn hostile_versions_and_counts() {
        let limits = Limits {
            max_bytes: 1024,
            max_items: 1024,
        };
        let bytes = sample(255).encode(limits).unwrap();
        for site in [0, 16] {
            for version in [0, 1, 2, 4, 6, u32::MAX] {
                let mut b = bytes.clone();
                b[site..site + 4].copy_from_slice(&version.to_le_bytes());
                assert!(matches!(
                    Manager::decode(&b, limits),
                    Err(Error::Version { .. })
                ));
            }
        }
        for site in [12, 53, 72] {
            let mut b = bytes.clone();
            b[site..site + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(Manager::decode(&b, limits), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn empty_manager_has_zero_items() {
        let v = Manager {
            player_ordinal: -1,
            control_priority_ordinal: i32::MIN,
            instances: vec![],
        };
        let limits = Limits {
            max_bytes: 16,
            max_items: 0,
        };
        let b = v.encode(limits).unwrap();
        assert_eq!(b.len(), 16);
        assert_eq!(Manager::decode(&b, limits).unwrap(), v);
    }
}
