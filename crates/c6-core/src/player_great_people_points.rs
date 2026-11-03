//! Current Linux XP2 Player.GreatPeoplePoints saved-state wire transport.
//! Does not execute Great Person recruitment, policy decisions, native initialization or remapping.
use crate::unit::Limits;
pub const VERSION: u32 = 13;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.GreatPeoplePoints wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Ordered definition-hash/signed-value pairs, preserving duplicates and raw Q8 where specified.
pub type HashValues = Vec<(u32, i32)>;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YieldChangePerEarnedGreatPersonClassOnResource {
    pub yield_hash: u32,
    pub great_person_class_hash: u32,
    pub resource_hash: u32,
    pub amount: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatPersonEarnedBoost {
    pub great_person_class_hash: u32,
    pub tech_boost: bool,
    pub other_players: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatPeoplePoints {
    pub player: i32,
    /// Great Person class definition hashes with signed raw Q8 values.
    pub points_total: HashValues,
    pub points_earned_this_turn: HashValues,
    pub bonus_points_per_turn: HashValues,
    pub free_great_person_points: i32,
    pub points_refund_percent: i32,
    pub no_patronage_faith: bool,
    pub no_patronage_gold: bool,
    /// Yield definition hashes with signed integer discounts.
    pub patronage_discount_percent: HashValues,
    /// Great Person class definition hashes with signed integer percentages.
    pub bonus_points_per_turn_percent: HashValues,
    /// Yield definition hashes with signed raw Q8 modifiers.
    pub yield_modifier_per_earned_great_person: HashValues,
    pub yield_change_per_earned_great_person_class:
        Vec<YieldChangePerEarnedGreatPersonClassOnResource>,
    pub great_person_boost_list: Vec<GreatPersonEarnedBoost>,
    /// Ordered definition hashes, not raw enum ordinals.
    pub great_person_class_no_points: Vec<u32>,
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn check(&self, n: usize) -> Result<usize, Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        Ok(end)
    }
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.check(n)?;
        let b = &self.data[self.pos..end];
        self.pos = end;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn int(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn count(&mut self, width: usize) -> Result<usize, Error> {
        let n = self.u32()? as usize;
        self.charge(n)?;
        self.check(n.checked_mul(width).ok_or(Error::ResourceLimit)?)?;
        Ok(n)
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(Error::InvalidBool(b)),
        }
    }
    fn version(&mut self) -> Result<(), Error> {
        let actual = self.u32()?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        Ok(())
    }
    fn sequence<T>(
        &mut self,
        width: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let n = self.count(width)?;
        let mut out = Vec::new();
        out.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn table(&mut self) -> Result<HashValues, Error> {
        self.sequence(8, |r| Ok((r.u32()?, r.int()?)))
    }
}
struct Writer {
    out: Vec<u8>,
    bytes: usize,
    items: usize,
    limits: Limits,
    emit: bool,
}
impl Writer {
    fn put(&mut self, b: &[u8]) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_add(b.len())
            .ok_or(Error::ResourceLimit)?;
        if self.bytes > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if self.emit {
            self.out.extend_from_slice(b)
        }
        Ok(())
    }
    fn word(&mut self, x: u32) -> Result<(), Error> {
        self.put(&x.to_le_bytes())
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn count(&mut self, n: usize) -> Result<(), Error> {
        self.charge(n)?;
        self.word(u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
    fn table(&mut self, v: &HashValues) -> Result<(), Error> {
        self.count(v.len())?;
        for (h, x) in v {
            self.word(*h)?;
            self.word(*x as u32)?;
        }
        Ok(())
    }
}
impl GreatPeoplePoints {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(data, limits)?;
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
        r.version()?;
        let v = Self {
            player: r.int()?,
            points_total: r.table()?,
            points_earned_this_turn: r.table()?,
            bonus_points_per_turn: r.table()?,
            free_great_person_points: r.int()?,
            points_refund_percent: r.int()?,
            no_patronage_faith: r.boolean()?,
            no_patronage_gold: r.boolean()?,
            patronage_discount_percent: r.table()?,
            bonus_points_per_turn_percent: r.table()?,
            yield_modifier_per_earned_great_person: r.table()?,
            yield_change_per_earned_great_person_class: r.sequence(20, |r| {
                r.version()?;
                Ok(YieldChangePerEarnedGreatPersonClassOnResource {
                    yield_hash: r.u32()?,
                    great_person_class_hash: r.u32()?,
                    resource_hash: r.u32()?,
                    amount: r.int()?,
                })
            })?,
            great_person_boost_list: r.sequence(10, |r| {
                r.version()?;
                Ok(GreatPersonEarnedBoost {
                    great_person_class_hash: r.u32()?,
                    tech_boost: r.boolean()?,
                    other_players: r.boolean()?,
                })
            })?,
            great_person_class_no_points: r.sequence(4, |r| r.u32())?,
        };
        Ok((v, r.pos, r.items))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.word(VERSION)?;
        w.word(self.player as u32)?;
        for t in [
            &self.points_total,
            &self.points_earned_this_turn,
            &self.bonus_points_per_turn,
        ] {
            w.table(t)?;
        }
        w.word(self.free_great_person_points as u32)?;
        w.word(self.points_refund_percent as u32)?;
        w.put(&[
            u8::from(self.no_patronage_faith),
            u8::from(self.no_patronage_gold),
        ])?;
        for t in [
            &self.patronage_discount_percent,
            &self.bonus_points_per_turn_percent,
            &self.yield_modifier_per_earned_great_person,
        ] {
            w.table(t)?;
        }
        w.count(self.yield_change_per_earned_great_person_class.len())?;
        for v in &self.yield_change_per_earned_great_person_class {
            w.word(VERSION)?;
            w.word(v.yield_hash)?;
            w.word(v.great_person_class_hash)?;
            w.word(v.resource_hash)?;
            w.word(v.amount as u32)?;
        }
        w.count(self.great_person_boost_list.len())?;
        for v in &self.great_person_boost_list {
            w.word(VERSION)?;
            w.word(v.great_person_class_hash)?;
            w.put(&[u8::from(v.tech_boost), u8::from(v.other_players)])?;
        }
        w.count(self.great_person_class_no_points.len())?;
        for h in &self.great_person_class_no_points {
            w.word(*h)?;
        }
        Ok(())
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            out: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            emit: false,
        };
        self.write(&mut w)?;
        let items = w.items;
        let bytes = w.bytes;
        w.out
            .try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        w.bytes = 0;
        w.items = 0;
        w.emit = true;
        self.write(&mut w)?;
        Ok((w.out, items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> GreatPeoplePoints {
        GreatPeoplePoints {
            player: -1,
            points_total: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            points_earned_this_turn: vec![(0, -256)],
            bonus_points_per_turn: vec![(1, 255)],
            free_great_person_points: i32::MIN,
            points_refund_percent: i32::MAX,
            no_patronage_faith: true,
            no_patronage_gold: false,
            patronage_discount_percent: vec![(2, -1)],
            bonus_points_per_turn_percent: vec![(3, -99)],
            yield_modifier_per_earned_great_person: vec![(4, 511)],
            yield_change_per_earned_great_person_class: vec![
                YieldChangePerEarnedGreatPersonClassOnResource {
                    yield_hash: 0,
                    great_person_class_hash: u32::MAX,
                    resource_hash: 0x80000000,
                    amount: i32::MIN,
                },
            ],
            great_person_boost_list: vec![GreatPersonEarnedBoost {
                great_person_class_hash: 42,
                tech_boost: false,
                other_players: true,
            }],
            great_person_class_no_points: vec![u32::MAX, 0, u32::MAX],
        }
    }
    #[test]
    fn nonempty_roundtrip_and_caps() {
        let v = fixture();
        let generous = Limits {
            max_bytes: 4096,
            max_items: 4096,
        };
        let (b, items) = v.encode_with_items(generous).unwrap();
        assert_eq!(items, 12);
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(GreatPeoplePoints::decode(&b, exact).unwrap(), v);
        assert_eq!(v.encode_with_items(exact).unwrap(), (b.clone(), items));
        for end in 0..b.len() {
            assert!(GreatPeoplePoints::decode(&b[..end], exact).is_err());
        }
        for lim in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(v.encode(lim).is_err());
            assert!(GreatPeoplePoints::decode(&b, lim).is_err());
        }
        let mut tail = b.clone();
        tail.push(1);
        assert_eq!(
            GreatPeoplePoints::decode(&tail, generous),
            Err(Error::Trailing)
        );
        assert_eq!(
            GreatPeoplePoints::decode_prefix(&tail, exact).unwrap(),
            (v, b.len())
        );
    }
    #[test]
    fn malformed_versions_bools_counts() {
        let v = fixture();
        let lim = Limits {
            max_bytes: 4096,
            max_items: 4096,
        };
        let b = v.encode(lim).unwrap();
        for pos in [0, 102, 126] {
            let mut bad = b.clone();
            bad[pos..pos + 4].copy_from_slice(&12u32.to_le_bytes());
            assert!(matches!(
                GreatPeoplePoints::decode(&bad, lim),
                Err(Error::Version { .. })
            ));
        }
        for pos in [60, 61, 134, 135] {
            let mut bad = b.clone();
            bad[pos] = 2;
            assert_eq!(
                GreatPeoplePoints::decode(&bad, lim),
                Err(Error::InvalidBool(2))
            );
        }
        let mut bad = b;
        bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            GreatPeoplePoints::decode(&bad, lim),
            Err(Error::ResourceLimit)
        );
    }
}
