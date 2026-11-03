//! Current Linux XP2 Player.Influence saved-state wire transport.
//! Saved wire only; envoy decisions remain external. No native Read initialization, remapping or gameplay.
use crate::unit::Limits;
pub const VERSION: u32 = 22;
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
        write!(f, "Linux Player.Influence wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Ordered yield definition hash and signed raw Q8 pairs, preserving duplicates.
pub type HashValues = Vec<(u32, i32)>;
/// Current saved record. Points and yield values retain signed raw Q8 bits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Influence {
    pub player: i32,
    pub points_earned: i32,
    pub base_points_per_turn: i32,
    pub not_earning_points: bool,
    pub points_threshold: i32,
    pub tokens_per_threshold: i32,
    pub tokens_to_give: i32,
    pub duplicate_first_tokens: i32,
    pub tokens_on_first_to_meet: i32,
    pub tokens_on_trade_route_started: i32,
    pub tokens_received: Vec<i32>,
    pub virtual_tokens_received: Vec<i32>,
    pub disallow_tokens_from: Vec<bool>,
    pub yield_per_given_token: HashValues,
    pub suzerain: i32,
    pub suzerain_changed_count: i32,
    pub suzerain_unique_bonus_disabled_count: i32,
    pub levy_turn_counter: i32,
    pub yield_modifier_per_tributary: HashValues,
    pub yield_per_tributary: HashValues,
    pub open_borders_count: Vec<i32>,
    pub levy_discount_percent_by_player: Vec<i32>,
    pub copy_luxuries_count: Vec<i32>,
    pub copy_strategics_count: Vec<i32>,
    pub duplicate_tokens_when_rival_government: i32,
    pub duplicate_tokens_when_trade_route_to: i32,
    pub duplicate_tokens_when_same_religion: i32,
    pub suzerain_visibility_counter: i32,
    pub levy_discount_percent: i32,
    pub tokens_received_modifier: Vec<i32>,
    pub max_tokens_received: Vec<i32>,
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
impl Influence {
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
            points_earned: r.int()?,
            base_points_per_turn: r.int()?,
            not_earning_points: r.boolean()?,
            points_threshold: r.int()?,
            tokens_per_threshold: r.int()?,
            tokens_to_give: r.int()?,
            duplicate_first_tokens: r.int()?,
            tokens_on_first_to_meet: r.int()?,
            tokens_on_trade_route_started: r.int()?,
            tokens_received: r.sequence(4, |r| r.int())?,
            virtual_tokens_received: r.sequence(4, |r| r.int())?,
            disallow_tokens_from: r.sequence(1, |r| r.boolean())?,
            yield_per_given_token: r.table()?,
            suzerain: r.int()?,
            suzerain_changed_count: r.int()?,
            suzerain_unique_bonus_disabled_count: r.int()?,
            levy_turn_counter: r.int()?,
            yield_modifier_per_tributary: r.table()?,
            yield_per_tributary: r.table()?,
            open_borders_count: r.sequence(4, |r| r.int())?,
            levy_discount_percent_by_player: r.sequence(4, |r| r.int())?,
            copy_luxuries_count: r.sequence(4, |r| r.int())?,
            copy_strategics_count: r.sequence(4, |r| r.int())?,
            duplicate_tokens_when_rival_government: r.int()?,
            duplicate_tokens_when_trade_route_to: r.int()?,
            duplicate_tokens_when_same_religion: r.int()?,
            suzerain_visibility_counter: r.int()?,
            levy_discount_percent: r.int()?,
            tokens_received_modifier: r.sequence(4, |r| r.int())?,
            max_tokens_received: r.sequence(4, |r| r.int())?,
        };
        Ok((v, r.pos, r.items))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.word(VERSION)?;
        w.word(self.player as u32)?;
        w.word(self.points_earned as u32)?;
        w.word(self.base_points_per_turn as u32)?;
        w.put(&[u8::from(self.not_earning_points)])?;
        w.word(self.points_threshold as u32)?;
        w.word(self.tokens_per_threshold as u32)?;
        w.word(self.tokens_to_give as u32)?;
        w.word(self.duplicate_first_tokens as u32)?;
        w.word(self.tokens_on_first_to_meet as u32)?;
        w.word(self.tokens_on_trade_route_started as u32)?;
        w.count(self.tokens_received.len())?;
        for x in &self.tokens_received {
            w.word(*x as u32)?;
        }
        w.count(self.virtual_tokens_received.len())?;
        for x in &self.virtual_tokens_received {
            w.word(*x as u32)?;
        }
        w.count(self.disallow_tokens_from.len())?;
        for x in &self.disallow_tokens_from {
            w.put(&[u8::from(*x)])?;
        }
        w.table(&self.yield_per_given_token)?;
        w.word(self.suzerain as u32)?;
        w.word(self.suzerain_changed_count as u32)?;
        w.word(self.suzerain_unique_bonus_disabled_count as u32)?;
        w.word(self.levy_turn_counter as u32)?;
        w.table(&self.yield_modifier_per_tributary)?;
        w.table(&self.yield_per_tributary)?;
        w.count(self.open_borders_count.len())?;
        for x in &self.open_borders_count {
            w.word(*x as u32)?;
        }
        w.count(self.levy_discount_percent_by_player.len())?;
        for x in &self.levy_discount_percent_by_player {
            w.word(*x as u32)?;
        }
        w.count(self.copy_luxuries_count.len())?;
        for x in &self.copy_luxuries_count {
            w.word(*x as u32)?;
        }
        w.count(self.copy_strategics_count.len())?;
        for x in &self.copy_strategics_count {
            w.word(*x as u32)?;
        }
        w.word(self.duplicate_tokens_when_rival_government as u32)?;
        w.word(self.duplicate_tokens_when_trade_route_to as u32)?;
        w.word(self.duplicate_tokens_when_same_religion as u32)?;
        w.word(self.suzerain_visibility_counter as u32)?;
        w.word(self.levy_discount_percent as u32)?;
        w.count(self.tokens_received_modifier.len())?;
        for x in &self.tokens_received_modifier {
            w.word(*x as u32)?;
        }
        w.count(self.max_tokens_received.len())?;
        for x in &self.max_tokens_received {
            w.word(*x as u32)?;
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
    fn fixture() -> Influence {
        Influence {
            player: i32::MAX,
            points_earned: i32::MIN,
            base_points_per_turn: i32::MAX,
            not_earning_points: true,
            points_threshold: i32::MAX,
            tokens_per_threshold: i32::MIN,
            tokens_to_give: i32::MAX,
            duplicate_first_tokens: i32::MIN,
            tokens_on_first_to_meet: i32::MAX,
            tokens_on_trade_route_started: i32::MIN,
            tokens_received: vec![i32::MIN, -1, i32::MAX],
            virtual_tokens_received: vec![i32::MIN, -1, i32::MAX],
            disallow_tokens_from: vec![false, true, false],
            yield_per_given_token: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            suzerain: i32::MAX,
            suzerain_changed_count: i32::MIN,
            suzerain_unique_bonus_disabled_count: i32::MAX,
            levy_turn_counter: i32::MIN,
            yield_modifier_per_tributary: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            yield_per_tributary: vec![(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)],
            open_borders_count: vec![i32::MIN, -1, i32::MAX],
            levy_discount_percent_by_player: vec![i32::MIN, -1, i32::MAX],
            copy_luxuries_count: vec![i32::MIN, -1, i32::MAX],
            copy_strategics_count: vec![i32::MIN, -1, i32::MAX],
            duplicate_tokens_when_rival_government: i32::MAX,
            duplicate_tokens_when_trade_route_to: i32::MIN,
            duplicate_tokens_when_same_religion: i32::MAX,
            suzerain_visibility_counter: i32::MIN,
            levy_discount_percent: i32::MAX,
            tokens_received_modifier: vec![i32::MIN, -1, i32::MAX],
            max_tokens_received: vec![i32::MIN, -1, i32::MAX],
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
        assert_eq!(items, 33);
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Influence::decode(&b, exact).unwrap(), v);
        assert_eq!(v.encode_with_items(exact).unwrap(), (b.clone(), items));
        for end in 0..b.len() {
            assert!(Influence::decode(&b[..end], exact).is_err());
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
            assert!(Influence::decode(&b, lim).is_err());
        }
        let mut tail = b;
        tail.push(0);
        assert_eq!(Influence::decode(&tail, generous), Err(Error::Trailing));
        assert_eq!(
            Influence::decode_prefix(&tail, exact).unwrap(),
            (v, tail.len() - 1)
        );
    }
    #[test]
    fn malformed_versions_bools_counts() {
        let lim = Limits {
            max_bytes: 4096,
            max_items: 4096,
        };
        let b = fixture().encode(lim).unwrap();
        let mut bad = b.clone();
        bad[..4].copy_from_slice(&21u32.to_le_bytes());
        assert!(matches!(
            Influence::decode(&bad, lim),
            Err(Error::Version { .. })
        ));
        for pos in [16, 77, 78, 79] {
            let mut bad = b.clone();
            bad[pos] = 2;
            assert_eq!(Influence::decode(&bad, lim), Err(Error::InvalidBool(2)));
        }
        let mut bad = b;
        bad[41..45].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Influence::decode(&bad, lim), Err(Error::ResourceLimit));
    }
}
