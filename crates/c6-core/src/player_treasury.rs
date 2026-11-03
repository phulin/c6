//! Current Linux XP2 Player.Treasury v10 wire transport.
//! No live Reset, definition remapping, callbacks, gameplay, actions or turns.
use crate::unit::Limits;

pub const VERSION: u32 = 10;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Treasury wire: {self:?}")
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
        // Check each allocation's addressable layout; wire bytes are a separate budget.
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        if bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn version(&mut self) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
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
macro_rules! record {
    ($name:ident, $version:expr, { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = (if $version { 4 } else { 0 }) $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version { r.version()?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version { VERSION.write(w)?; }
                $(self.$field.write(w)?;)*
                Ok(())
            }
        }
    };
}
use crate::unit::HashValue;
impl Wire for HashValue<i32> {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            definition_hash: u32::read(r)?,
            value: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.definition_hash.write(w)?;
        self.value.write(w)
    }
}
record!(DiplomaticDeal,false,{ first:u32, second:u32 });
record!(Treasury,true,{
    gold_balance_q8:i32,
    gold_rate_change_q8:i32,
    cached_net_income_excluding_trade_q8:i32,
    bankruptcy_amenities:i32,
    maintenance_discount_per_unit:i32,
    unit_upgrade_discount_percent:i32,
    maintenance_discount_per_wmd_modifier:i32,
    levied_unit_upgrade_discount_percent:i32,
    cached_net_income_q8:i32,
    gold_interest_percent_q8:i32,
    gold_dispersal_change_q8:Vec<HashValue<i32>>,
    gold_diplomatic_deals:Vec<DiplomaticDeal>,
});
impl Treasury {
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
    fn sample() -> Treasury {
        Treasury {
            gold_balance_q8: i32::MIN,
            gold_rate_change_q8: -1,
            cached_net_income_excluding_trade_q8: 256,
            bankruptcy_amenities: -3,
            maintenance_discount_per_unit: 4,
            unit_upgrade_discount_percent: 5,
            maintenance_discount_per_wmd_modifier: 6,
            levied_unit_upgrade_discount_percent: 7,
            cached_net_income_q8: 8,
            gold_interest_percent_q8: i32::MAX,
            gold_dispersal_change_q8: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: -99,
                },
                HashValue {
                    definition_hash: 1,
                    value: 256,
                },
            ],
            gold_diplomatic_deals: vec![
                DiplomaticDeal {
                    first: u32::MAX,
                    second: 0,
                },
                DiplomaticDeal {
                    first: 1,
                    second: 2,
                },
            ],
        }
    }
    #[test]
    fn nonempty_exact_caps_and_truncations() {
        let value = sample();
        let limits = Limits {
            max_bytes: 1000,
            max_items: 100,
        };
        let (bytes, items) = value.encode_with_items(limits).unwrap();
        assert_eq!((bytes.len(), items), (84, 4));
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(Treasury::decode(&bytes, exact).unwrap(), value);
        assert_eq!(
            value.encode_with_items(exact).unwrap(),
            (bytes.clone(), items)
        );
        for n in 0..bytes.len() {
            assert!(Treasury::decode(&bytes[..n], limits).is_err());
        }
        for low in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(Treasury::decode(&bytes, low), Err(Error::ResourceLimit));
            assert_eq!(value.encode(low), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn framing_version_and_unsigned_pairs() {
        let value = sample();
        let limits = Limits {
            max_bytes: 1000,
            max_items: 100,
        };
        let mut bytes = value.encode(limits).unwrap();
        assert_eq!(&bytes[0..8], &[10, 0, 0, 0, 0, 0, 0, 128]);
        assert_eq!(&bytes[68..76], &[255, 255, 255, 255, 0, 0, 0, 0]);
        bytes.push(2);
        assert_eq!(Treasury::decode(&bytes, limits), Err(Error::Trailing));
        assert_eq!(
            Treasury::decode_prefix_with_items(&bytes, limits).unwrap(),
            (value, 84, 4)
        );
        bytes[0] = 9;
        assert_eq!(
            Treasury::decode(&bytes, limits),
            Err(Error::Version {
                expected: 10,
                actual: 9
            })
        );
    }
    #[test]
    fn hostile_count_rejected_before_allocation() {
        let limits = Limits {
            max_bytes: 1000,
            max_items: 100,
        };
        let mut bytes = sample().encode(limits).unwrap();
        bytes[44..48].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Treasury::decode(&bytes, limits), Err(Error::ResourceLimit));
    }
}
