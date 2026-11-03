//! Linux XP2 current-version DiplomacyBlackboard wire state only.
//! Raw fixed-point words, enum values, map order and duplicate keys are retained.
//! This does not execute diplomacy or reproduce native object construction/read side effects.

pub use crate::unit::{Error, Limits};

/// Ordered serialized map entries; no insertion, sorting or deduplication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongArray<T> {
    pub key: i64,
    pub values: [T; 64],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModifierEntry {
    pub key: u32,
    pub tracker: DiplomaticModifierTracker,
}
/// FixedPointT<8> fields hold their signed raw 32-bit words.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiplomaticModifierTracker {
    pub is_active: bool,
    pub current_value: i32,
    pub initial_value: i32,
    pub max_value: i32,
    pub increment: i32,
    pub increment_turns: i32,
    pub cooldown_turns: i32,
    pub accumulation_turn_count: i32,
    pub reduction: i32,
    /// Encoding-0 string bytes, without Unicode conversion or terminator.
    pub modifier_name: Vec<u8>,
    pub visibility: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiplomacyBlackboard {
    pub fixed_point_arrays: Vec<LongArray<i32>>,
    pub uint_arrays: Vec<LongArray<u32>>,
    pub bool_arrays: Vec<LongArray<bool>>,
    pub diplomatic_modifiers: Vec<Vec<ModifierEntry>>,
    pub warmonger_scratch_player: i32,
    pub warmonger_scratch_value: i32,
    pub warmonger_scaling_multiplier: i32,
    pub one_sided_trade_value: [i32; 64],
    pub broken_promises: [i32; 64],
    pub kept_promises: [i32; 64],
    pub no_promises: [i32; 64],
    pub first_impressions: [i32; 64],
    pub broken_promise_turn: [i32; 64],
    pub broken_pledges: [i32; 64],
    pub kept_pledges: [i32; 64],
    pub no_pledges: [i32; 64],
}
pub type Blackboard = DiplomacyBlackboard;

fn charge(items: &mut usize, n: usize, limits: Limits) -> Result<(), Error> {
    *items = items.checked_add(n).ok_or(Error::ResourceLimit)?;
    if *items > limits.max_items {
        return Err(Error::ResourceLimit);
    }
    Ok(())
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn bool(&mut self) -> Result<bool, Error> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(Error::NoncanonicalBool(n)),
        }
    }
    fn version(&mut self) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual != 13 {
            return Err(Error::Version {
                expected: 13,
                actual,
            });
        }
        Ok(())
    }
    fn list<T>(
        &mut self,
        min: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.u32()? as usize;
        charge(&mut self.items, n, self.limits)?;
        let bytes = n.checked_mul(min).ok_or(Error::ResourceLimit)?;
        if self.pos.checked_add(bytes).ok_or(Error::ResourceLimit)? > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if bytes > self.data.len() - self.pos {
            return Err(Error::Truncated);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn array<T: Copy + Default>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<[T; 64], Error> {
        let mut out = [T::default(); 64];
        for value in &mut out {
            *value = read(self)?;
        }
        Ok(out)
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        if self
            .data
            .len()
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(b.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(b);
        Ok(())
    }
    fn u32(&mut self, n: u32) -> Result<(), Error> {
        self.bytes(&n.to_le_bytes())
    }
    fn i32(&mut self, n: i32) -> Result<(), Error> {
        self.u32(n as u32)
    }
    fn i64(&mut self, n: i64) -> Result<(), Error> {
        self.bytes(&n.to_le_bytes())
    }
    fn bool(&mut self, n: bool) -> Result<(), Error> {
        self.bytes(&[u8::from(n)])
    }
    fn count(&mut self, n: usize) -> Result<(), Error> {
        charge(&mut self.items, n, self.limits)?;
        self.u32(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn list<T>(
        &mut self,
        values: &[T],
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.count(values.len())?;
        for value in values {
            write(self, value)?;
        }
        Ok(())
    }
    fn array<T>(
        &mut self,
        values: &[T; 64],
        mut write: impl FnMut(&mut Self, &T) -> Result<(), Error>,
    ) -> Result<(), Error> {
        for value in values {
            write(self, value)?;
        }
        Ok(())
    }
}
trait Wire: Sized {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Wire for DiplomaticModifierTracker {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version()?;
        Ok(Self {
            is_active: r.bool()?,
            current_value: r.i32()?,
            initial_value: r.i32()?,
            max_value: r.i32()?,
            increment: r.i32()?,
            increment_turns: r.i32()?,
            cooldown_turns: r.i32()?,
            accumulation_turn_count: r.i32()?,
            reduction: r.i32()?,
            modifier_name: r.list(1, |r| Ok(r.bytes(1)?[0]))?,
            visibility: r.i32()?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(13)?;
        w.bool(self.is_active)?;
        for n in [
            self.current_value,
            self.initial_value,
            self.max_value,
            self.increment,
            self.increment_turns,
            self.cooldown_turns,
            self.accumulation_turn_count,
            self.reduction,
        ] {
            w.i32(n)?;
        }
        w.count(self.modifier_name.len())?;
        w.bytes(&self.modifier_name)?;
        w.i32(self.visibility)
    }
}
impl Wire for DiplomacyBlackboard {
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version()?;
        Ok(Self {
            fixed_point_arrays: r.list(264, |r| {
                Ok(LongArray {
                    key: r.i64()?,
                    values: r.array(Reader::i32)?,
                })
            })?,
            uint_arrays: r.list(264, |r| {
                Ok(LongArray {
                    key: r.i64()?,
                    values: r.array(Reader::u32)?,
                })
            })?,
            bool_arrays: r.list(72, |r| {
                Ok(LongArray {
                    key: r.i64()?,
                    values: r.array(Reader::bool)?,
                })
            })?,
            diplomatic_modifiers: r.list(4, |r| {
                r.list(49, |r| {
                    Ok(ModifierEntry {
                        key: r.u32()?,
                        tracker: DiplomaticModifierTracker::read(r)?,
                    })
                })
            })?,
            warmonger_scratch_player: r.i32()?,
            warmonger_scratch_value: r.i32()?,
            warmonger_scaling_multiplier: r.i32()?,
            one_sided_trade_value: r.array(Reader::i32)?,
            broken_promises: r.array(Reader::i32)?,
            kept_promises: r.array(Reader::i32)?,
            no_promises: r.array(Reader::i32)?,
            first_impressions: r.array(Reader::i32)?,
            broken_promise_turn: r.array(Reader::i32)?,
            broken_pledges: r.array(Reader::i32)?,
            kept_pledges: r.array(Reader::i32)?,
            no_pledges: r.array(Reader::i32)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.i32(13)?;
        w.list(&self.fixed_point_arrays, |w, e| {
            w.i64(e.key)?;
            w.array(&e.values, |w, n| w.i32(*n))
        })?;
        w.list(&self.uint_arrays, |w, e| {
            w.i64(e.key)?;
            w.array(&e.values, |w, n| w.u32(*n))
        })?;
        w.list(&self.bool_arrays, |w, e| {
            w.i64(e.key)?;
            w.array(&e.values, |w, n| w.bool(*n))
        })?;
        w.list(&self.diplomatic_modifiers, |w, v| {
            w.list(v, |w, e| {
                w.u32(e.key)?;
                e.tracker.write(w)
            })
        })?;
        w.i32(self.warmonger_scratch_player)?;
        w.i32(self.warmonger_scratch_value)?;
        w.i32(self.warmonger_scaling_multiplier)?;
        for a in [
            &self.one_sided_trade_value,
            &self.broken_promises,
            &self.kept_promises,
            &self.no_promises,
            &self.first_impressions,
            &self.broken_promise_turn,
            &self.broken_pledges,
            &self.kept_pledges,
            &self.no_pledges,
        ] {
            w.array(a, |w, n| w.i32(*n))?;
        }
        Ok(())
    }
}
macro_rules! codec {
    ($t:ty) => {
        impl $t {
            /// Only current wire version 13. Item charges aggregate dynamic entries and raw string bytes; fixed arrays are uncharged.
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
                let value = Self::read(&mut r)?;
                Ok((value, r.pos, r.items))
            }
            pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
                let (value, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
                Ok((value, n))
            }
            pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
                let (value, n) = Self::decode_prefix(bytes, limits)?;
                if n != bytes.len() {
                    return Err(Error::TrailingBytes);
                }
                Ok(value)
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
    };
}
codec!(DiplomacyBlackboard);
codec!(DiplomaticModifierTracker);

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> DiplomacyBlackboard {
        let mut empty = vec![0; 2336];
        empty[..4].copy_from_slice(&13i32.to_le_bytes());
        let mut b = DiplomacyBlackboard::decode(&empty, Limits::default()).unwrap();
        b.fixed_point_arrays = vec![
            LongArray {
                key: i64::MIN,
                values: [i32::MIN; 64],
            },
            LongArray {
                key: i64::MIN,
                values: [i32::MAX; 64],
            },
        ];
        b.uint_arrays = vec![LongArray {
            key: i64::MAX,
            values: [u32::MAX; 64],
        }];
        b.bool_arrays = vec![LongArray {
            key: -1,
            values: [true; 64],
        }];
        let t = DiplomaticModifierTracker {
            is_active: true,
            current_value: i32::MIN,
            initial_value: 1,
            max_value: 2,
            increment: -3,
            increment_turns: 4,
            cooldown_turns: 5,
            accumulation_turn_count: -6,
            reduction: 7,
            modifier_name: vec![0xff, 0, 0x80],
            visibility: i32::MIN,
        };
        b.diplomatic_modifiers = vec![
            vec![
                ModifierEntry {
                    key: u32::MAX,
                    tracker: t.clone(),
                },
                ModifierEntry {
                    key: u32::MAX,
                    tracker: t,
                },
            ],
            vec![],
        ];
        b.warmonger_scratch_player = i32::MIN;
        b.warmonger_scratch_value = -123;
        b.warmonger_scaling_multiplier = 456;
        b.one_sided_trade_value = [1; 64];
        b.broken_promises = [2; 64];
        b.kept_promises = [3; 64];
        b.no_promises = [4; 64];
        b.first_impressions = [5; 64];
        b.broken_promise_turn = [6; 64];
        b.broken_pledges = [7; 64];
        b.kept_pledges = [8; 64];
        b.no_pledges = [9; 64];
        b
    }
    #[test]
    fn nested_roundtrip_and_aggregate_limits() {
        let b = sample();
        let (wire, items) = b.encode_with_items(Limits::default()).unwrap();
        assert_eq!(items, 14);
        let (actual, n, used) =
            DiplomacyBlackboard::decode_prefix_with_items(&wire, Limits::default()).unwrap();
        assert_eq!(actual, b);
        assert_eq!(n, wire.len());
        assert_eq!(used, items);
        let exact = Limits {
            max_bytes: wire.len(),
            max_items: items,
        };
        assert_eq!(b.encode(exact).unwrap(), wire);
        assert_eq!(DiplomacyBlackboard::decode(&wire, exact).unwrap(), b);
        for limits in [
            Limits {
                max_bytes: wire.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(b.encode(limits), Err(Error::ResourceLimit));
            assert_eq!(
                DiplomacyBlackboard::decode(&wire, limits),
                Err(Error::ResourceLimit)
            );
        }
        for end in 0..wire.len() {
            assert!(
                DiplomacyBlackboard::decode(&wire[..end], Limits::default()).is_err(),
                "prefix {end}"
            );
        }
    }
    #[test]
    fn independently_framed_tracker() {
        let mut wire = 13i32.to_le_bytes().to_vec();
        wire.push(1);
        for n in [101i32, -202, 303, -404, 505, -606, 707, -808] {
            wire.extend_from_slice(&n.to_le_bytes());
        }
        wire.extend_from_slice(&3u32.to_le_bytes());
        wire.extend_from_slice(&[0xff, 0, 0x80]);
        wire.extend_from_slice(&i32::MIN.to_le_bytes());
        let t = DiplomaticModifierTracker::decode(&wire, Limits::default()).unwrap();
        assert_eq!(
            (t.current_value, t.initial_value, t.max_value, t.increment),
            (101, -202, 303, -404)
        );
        assert_eq!(
            (
                t.increment_turns,
                t.cooldown_turns,
                t.accumulation_turn_count,
                t.reduction
            ),
            (505, -606, 707, -808)
        );
        assert_eq!(t.modifier_name, [0xff, 0, 0x80]);
        assert_eq!(t.visibility, i32::MIN);
        assert_eq!(t.encode(Limits::default()).unwrap(), wire);
        for end in 0..wire.len() {
            assert!(DiplomaticModifierTracker::decode(&wire[..end], Limits::default()).is_err());
        }
        let mut b = sample().encode(Limits::default()).unwrap();
        // Header + fixed-point map + uint map + bool count + long key.
        let first_bool = 4 + 4 + 2 * 264 + 4 + 264 + 4 + 8;
        b[first_bool] = 3;
        assert_eq!(
            DiplomacyBlackboard::decode(&b, Limits::default()),
            Err(Error::NoncanonicalBool(3))
        );
    }
    #[test]
    fn wire_order_and_refusals() {
        let b = sample();
        let mut wire = b.encode(Limits::default()).unwrap();
        assert_eq!(&wire[8..16], &i64::MIN.to_le_bytes());
        let arrays_start = wire.len() - 9 * 256;
        for i in 0..9 {
            assert_eq!(
                &wire[arrays_start + i * 256..arrays_start + i * 256 + 4],
                &((i + 1) as i32).to_le_bytes()
            );
        }
        wire.push(9);
        assert_eq!(
            DiplomacyBlackboard::decode(&wire, Limits::default()),
            Err(Error::TrailingBytes)
        );
        assert_eq!(
            DiplomacyBlackboard::decode_prefix(&wire, Limits::default())
                .unwrap()
                .1,
            wire.len() - 1
        );
        wire[0] = 12;
        assert_eq!(
            DiplomacyBlackboard::decode(&wire, Limits::default()),
            Err(Error::Version {
                expected: 13,
                actual: 12
            })
        );
        let tracker = &b.diplomatic_modifiers[0][0].tracker;
        let mut t = tracker.encode(Limits::default()).unwrap();
        assert_eq!(t.len(), 48);
        assert_eq!(&t[5..9], &i32::MIN.to_le_bytes());
        t[4] = 2;
        assert_eq!(
            DiplomaticModifierTracker::decode(&t, Limits::default()),
            Err(Error::NoncanonicalBool(2))
        );
        t[0] = 2;
        assert_eq!(
            DiplomaticModifierTracker::decode(&t, Limits::default()),
            Err(Error::Version {
                expected: 13,
                actual: 2
            })
        );
        let mut huge = vec![0; 8];
        huge[..4].copy_from_slice(&13i32.to_le_bytes());
        huge[4..].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            DiplomacyBlackboard::decode(&huge, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
