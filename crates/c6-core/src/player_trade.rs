//! Linux XP2 Player.Trade v22/v35 saved-state wire transport.
//! Does not execute trade routes, policy decisions, native initialization or remapping.
use crate::unit::Limits;
pub const VERSION: u32 = 35;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.Trade wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Ordered definition-hash/raw signed Q8 pairs; duplicates and all hash patterns survive.
pub type YieldTable = Vec<(u32, i32)>;
/// Ordered improvement/terrain definition hashes with nested yield hash/Q8 tables.
pub type NestedYieldTable = Vec<(u32, YieldTable)>;
/// Tables absent from the version 22 wire whose native Read writes defaults at
/// actual definition indexes. An empty serialized-pair carrier is not an empty
/// effective native array; registry cardinalities and indexes remain separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedDefinitionTable {
    CityStateYield,
    CulturalDominanceYield,
    OriginSuzerainYield,
    DestinationSuzerainYield,
    PathTileYield,
    FollowerYield,
    InternationalYieldModifier,
    InternationalYieldPerImprovementAtOrigin,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub wire_version: u32,
    pub player: i32,
    pub outgoing_route_capacity: i32,
    pub notified_outgoing_route_capacity: i32,
    pub extra_route_range: i32,
    pub improved_route_level: i32,
    pub immediate_trading_post_count: i32,
    pub gain_tiles_en_route_radius: i32,
    pub anyone_plunder_favor: i32,
    pub anyone_trade_to_favor: i32,
    pub extra_land_range_refuel: i32,
    pub extra_water_range_refuel: i32,
    pub free_tech_boost_first_trading_post_each_civ: i32,
    pub free_civic_boost_first_trading_post_each_civ: i32,
    pub trade_route_capacity_increase_on_meeting: i32,
    pub forbid_land_route_domestic: i32,
    pub forbid_land_route_international: i32,
    pub civ_received_civic_boost_first_trading_post: Vec<bool>,
    pub civ_received_tech_boost_first_trading_post: Vec<bool>,
    pub flat_yield_bonus: YieldTable,
    pub flat_yield_bonus_for_domestic: YieldTable,
    pub flat_yield_bonus_for_city_state: YieldTable,
    pub flat_yield_bonus_for_international: YieldTable,
    pub flat_yield_bonus_for_intercontinental_domestic: YieldTable,
    pub flat_yield_bonus_for_intercontinental_international: YieldTable,
    pub flat_yield_bonus_for_cultural_dominance: YieldTable,
    pub yield_bonus_for_districts_city_state: YieldTable,
    pub origin_yield_for_ally_route: YieldTable,
    pub destination_yield_for_ally_route: YieldTable,
    pub origin_yield_for_suzerain_route: YieldTable,
    pub destination_yield_for_suzerain_route: YieldTable,
    pub flat_yield_bonus_to_others: YieldTable,
    pub techs_civics_per_yield: i32,
    pub per_specialty_district_yield_bonus_for_domestic: YieldTable,
    pub yield_bonus_per_trading_post_in_foreign_city: YieldTable,
    pub yield_bonus_per_trading_post_in_own_city: YieldTable,
    pub yield_bonus_per_path_tile: YieldTable,
    pub yield_bonus_per_follower: YieldTable,
    pub international_yield_modifier: YieldTable,
    pub origin_yield_modifier: YieldTable,
    pub destination_yield_modifier: YieldTable,
    pub flat_yield_bonus_per_player: [YieldTable; 64],
    pub flat_origin_yield_per_improvement_at_destination: NestedYieldTable,
    pub flat_destination_yield_per_improvement_at_destination: NestedYieldTable,
    pub flat_international_yield_per_improvement_at_origin: NestedYieldTable,
    pub per_specialty_district_yield_bonus_for_international: YieldTable,
    pub flat_origin_yield_per_terrain_for_domestic: NestedYieldTable,
    pub flat_destination_yield_per_terrain_for_domestic: NestedYieldTable,
    pub flat_origin_yield_per_terrain_for_international: NestedYieldTable,
    pub flat_destination_yield_per_terrain_for_international: NestedYieldTable,
    pub international_majors_trade_disabled_count: i32,
    pub international_minors_trade_disabled_count: i32,
    pub domestic_trade_disabled_count: i32,
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
    fn bools(&mut self) -> Result<Vec<bool>, Error> {
        let n = self.count(1)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            let b = self.take(1)?[0];
            v.push(match b {
                0 => false,
                1 => true,
                _ => return Err(Error::InvalidBool(b)),
            });
        }
        Ok(v)
    }
    fn table(&mut self) -> Result<YieldTable, Error> {
        let n = self.count(8)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push((self.u32()?, self.int()?));
        }
        Ok(v)
    }
    fn nested(&mut self) -> Result<NestedYieldTable, Error> {
        let n = self.count(8)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push((self.u32()?, self.table()?));
        }
        Ok(v)
    }
    fn players(&mut self) -> Result<[YieldTable; 64], Error> {
        self.charge(64)?;
        self.check(64 * 4)?;
        let mut a = std::array::from_fn(|_| Vec::new());
        for x in &mut a {
            *x = self.table()?;
        }
        Ok(a)
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
    fn bools(&mut self, v: &[bool]) -> Result<(), Error> {
        self.count(v.len())?;
        for x in v {
            self.put(&[u8::from(*x)])?;
        }
        Ok(())
    }
    fn table(&mut self, v: &YieldTable) -> Result<(), Error> {
        self.count(v.len())?;
        for (h, q) in v {
            self.word(*h)?;
            self.word(*q as u32)?;
        }
        Ok(())
    }
    fn nested(&mut self, v: &NestedYieldTable) -> Result<(), Error> {
        self.count(v.len())?;
        for (h, t) in v {
            self.word(*h)?;
            self.table(t)?;
        }
        Ok(())
    }
    fn players(&mut self, v: &[YieldTable; 64]) -> Result<(), Error> {
        self.charge(64)?;
        for t in v {
            self.table(t)?;
        }
        Ok(())
    }
}
impl Trade {
    /// True only when both receipt vectors were serialized. Version 22 instead
    /// derives 64 false entries from the unconditional native Initialize/Reset;
    /// those fixed derived entries consume no wire collection-item charge.
    pub fn boost_receipts_wire_present(&self) -> bool {
        self.wire_version == VERSION
    }

    /// Unapplied native definition-indexed default, in signed raw Q8 units.
    /// The improvement case applies to each actual Improvement's Yield list.
    /// No registry order or dense native index is inferred from wire hashes.
    pub fn omitted_definition_table_default_q8(
        &self,
        table: OmittedDefinitionTable,
    ) -> Option<i32> {
        if self.wire_version != 22 {
            return None;
        }
        Some(
            if table == OmittedDefinitionTable::InternationalYieldModifier {
                256
            } else {
                0
            },
        )
    }

    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(data, limits)?;
        if n != data.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(data, limits)?;
        Ok((v, n))
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
        let actual = r.u32()?;
        if actual != 22 && actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let v = Self {
            wire_version: actual,
            player: r.int()?,
            outgoing_route_capacity: r.int()?,
            notified_outgoing_route_capacity: r.int()?,
            extra_route_range: r.int()?,
            improved_route_level: r.int()?,
            immediate_trading_post_count: r.int()?,
            gain_tiles_en_route_radius: r.int()?,
            anyone_plunder_favor: r.int()?,
            anyone_trade_to_favor: r.int()?,
            extra_land_range_refuel: r.int()?,
            extra_water_range_refuel: r.int()?,
            free_tech_boost_first_trading_post_each_civ: if actual == 22 { 0 } else { r.int()? },
            free_civic_boost_first_trading_post_each_civ: if actual == 22 { 0 } else { r.int()? },
            trade_route_capacity_increase_on_meeting: if actual == 22 { 0 } else { r.int()? },
            forbid_land_route_domestic: if actual == 22 { 0 } else { r.int()? },
            forbid_land_route_international: if actual == 22 { 0 } else { r.int()? },
            civ_received_civic_boost_first_trading_post: if actual == 22 {
                vec![false; 64]
            } else {
                r.bools()?
            },
            civ_received_tech_boost_first_trading_post: if actual == 22 {
                vec![false; 64]
            } else {
                r.bools()?
            },
            flat_yield_bonus: r.table()?,
            flat_yield_bonus_for_domestic: r.table()?,
            flat_yield_bonus_for_city_state: if actual == 22 { Vec::new() } else { r.table()? },
            flat_yield_bonus_for_international: r.table()?,
            flat_yield_bonus_for_intercontinental_domestic: r.table()?,
            flat_yield_bonus_for_intercontinental_international: r.table()?,
            flat_yield_bonus_for_cultural_dominance: if actual == 22 {
                Vec::new()
            } else {
                r.table()?
            },
            yield_bonus_for_districts_city_state: r.table()?,
            origin_yield_for_ally_route: r.table()?,
            destination_yield_for_ally_route: r.table()?,
            origin_yield_for_suzerain_route: if actual == 22 { Vec::new() } else { r.table()? },
            destination_yield_for_suzerain_route: if actual == 22 {
                Vec::new()
            } else {
                r.table()?
            },
            flat_yield_bonus_to_others: r.table()?,
            techs_civics_per_yield: r.int()?,
            per_specialty_district_yield_bonus_for_domestic: r.table()?,
            yield_bonus_per_trading_post_in_foreign_city: r.table()?,
            yield_bonus_per_trading_post_in_own_city: r.table()?,
            yield_bonus_per_path_tile: if actual == 22 { Vec::new() } else { r.table()? },
            yield_bonus_per_follower: if actual == 22 { Vec::new() } else { r.table()? },
            international_yield_modifier: if actual == 22 { Vec::new() } else { r.table()? },
            origin_yield_modifier: r.table()?,
            destination_yield_modifier: r.table()?,
            flat_yield_bonus_per_player: r.players()?,
            flat_origin_yield_per_improvement_at_destination: r.nested()?,
            flat_destination_yield_per_improvement_at_destination: r.nested()?,
            flat_international_yield_per_improvement_at_origin: if actual == 22 {
                Vec::new()
            } else {
                r.nested()?
            },
            per_specialty_district_yield_bonus_for_international: r.table()?,
            flat_origin_yield_per_terrain_for_domestic: r.nested()?,
            flat_destination_yield_per_terrain_for_domestic: r.nested()?,
            flat_origin_yield_per_terrain_for_international: r.nested()?,
            flat_destination_yield_per_terrain_for_international: r.nested()?,
            international_majors_trade_disabled_count: if actual == 22 { 0 } else { r.int()? },
            international_minors_trade_disabled_count: if actual == 22 { 0 } else { r.int()? },
            domestic_trade_disabled_count: if actual == 22 { 0 } else { r.int()? },
        };
        Ok((v, r.pos, r.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        // Measure before reserving. Wire bytes and typed-storage element limits are
        // independent: dynamic storage is bounded by max_items, not max_bytes.
        let mut w = Writer {
            out: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            emit: false,
        };
        self.write(&mut w)?;
        let bytes = w.bytes;
        let items = w.items;
        w.out
            .try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        w.bytes = 0;
        w.items = 0;
        w.emit = true;
        self.write(&mut w)?;
        Ok((w.out, items))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        w.word(VERSION)?;
        w.word(self.player as u32)?;
        w.word(self.outgoing_route_capacity as u32)?;
        w.word(self.notified_outgoing_route_capacity as u32)?;
        w.word(self.extra_route_range as u32)?;
        w.word(self.improved_route_level as u32)?;
        w.word(self.immediate_trading_post_count as u32)?;
        w.word(self.gain_tiles_en_route_radius as u32)?;
        w.word(self.anyone_plunder_favor as u32)?;
        w.word(self.anyone_trade_to_favor as u32)?;
        w.word(self.extra_land_range_refuel as u32)?;
        w.word(self.extra_water_range_refuel as u32)?;
        w.word(self.free_tech_boost_first_trading_post_each_civ as u32)?;
        w.word(self.free_civic_boost_first_trading_post_each_civ as u32)?;
        w.word(self.trade_route_capacity_increase_on_meeting as u32)?;
        w.word(self.forbid_land_route_domestic as u32)?;
        w.word(self.forbid_land_route_international as u32)?;
        w.bools(&self.civ_received_civic_boost_first_trading_post)?;
        w.bools(&self.civ_received_tech_boost_first_trading_post)?;
        w.table(&self.flat_yield_bonus)?;
        w.table(&self.flat_yield_bonus_for_domestic)?;
        w.table(&self.flat_yield_bonus_for_city_state)?;
        w.table(&self.flat_yield_bonus_for_international)?;
        w.table(&self.flat_yield_bonus_for_intercontinental_domestic)?;
        w.table(&self.flat_yield_bonus_for_intercontinental_international)?;
        w.table(&self.flat_yield_bonus_for_cultural_dominance)?;
        w.table(&self.yield_bonus_for_districts_city_state)?;
        w.table(&self.origin_yield_for_ally_route)?;
        w.table(&self.destination_yield_for_ally_route)?;
        w.table(&self.origin_yield_for_suzerain_route)?;
        w.table(&self.destination_yield_for_suzerain_route)?;
        w.table(&self.flat_yield_bonus_to_others)?;
        w.word(self.techs_civics_per_yield as u32)?;
        w.table(&self.per_specialty_district_yield_bonus_for_domestic)?;
        w.table(&self.yield_bonus_per_trading_post_in_foreign_city)?;
        w.table(&self.yield_bonus_per_trading_post_in_own_city)?;
        w.table(&self.yield_bonus_per_path_tile)?;
        w.table(&self.yield_bonus_per_follower)?;
        w.table(&self.international_yield_modifier)?;
        w.table(&self.origin_yield_modifier)?;
        w.table(&self.destination_yield_modifier)?;
        w.players(&self.flat_yield_bonus_per_player)?;
        w.nested(&self.flat_origin_yield_per_improvement_at_destination)?;
        w.nested(&self.flat_destination_yield_per_improvement_at_destination)?;
        w.nested(&self.flat_international_yield_per_improvement_at_origin)?;
        w.table(&self.per_specialty_district_yield_bonus_for_international)?;
        w.nested(&self.flat_origin_yield_per_terrain_for_domestic)?;
        w.nested(&self.flat_destination_yield_per_terrain_for_domestic)?;
        w.nested(&self.flat_origin_yield_per_terrain_for_international)?;
        w.nested(&self.flat_destination_yield_per_terrain_for_international)?;
        w.word(self.international_majors_trade_disabled_count as u32)?;
        w.word(self.international_minors_trade_disabled_count as u32)?;
        w.word(self.domestic_trade_disabled_count as u32)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Trade {
        Trade {
            wire_version: VERSION,
            player: 0i32.wrapping_mul(-257),
            outgoing_route_capacity: 1i32.wrapping_mul(-257),
            notified_outgoing_route_capacity: 2i32.wrapping_mul(-257),
            extra_route_range: 3i32.wrapping_mul(-257),
            improved_route_level: 4i32.wrapping_mul(-257),
            immediate_trading_post_count: 5i32.wrapping_mul(-257),
            gain_tiles_en_route_radius: 6i32.wrapping_mul(-257),
            anyone_plunder_favor: 7i32.wrapping_mul(-257),
            anyone_trade_to_favor: 8i32.wrapping_mul(-257),
            extra_land_range_refuel: 9i32.wrapping_mul(-257),
            extra_water_range_refuel: 10i32.wrapping_mul(-257),
            free_tech_boost_first_trading_post_each_civ: 11i32.wrapping_mul(-257),
            free_civic_boost_first_trading_post_each_civ: 12i32.wrapping_mul(-257),
            trade_route_capacity_increase_on_meeting: 13i32.wrapping_mul(-257),
            forbid_land_route_domestic: 14i32.wrapping_mul(-257),
            forbid_land_route_international: 15i32.wrapping_mul(-257),
            civ_received_civic_boost_first_trading_post: vec![true, false, true],
            civ_received_tech_boost_first_trading_post: vec![true, false, true],
            flat_yield_bonus: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_domestic: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_city_state: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_international: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_for_intercontinental_domestic: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_yield_bonus_for_intercontinental_international: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_yield_bonus_for_cultural_dominance: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_for_districts_city_state: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_for_ally_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_for_ally_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_for_suzerain_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_for_suzerain_route: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_to_others: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            techs_civics_per_yield: 31i32.wrapping_mul(-257),
            per_specialty_district_yield_bonus_for_domestic: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            yield_bonus_per_trading_post_in_foreign_city: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            yield_bonus_per_trading_post_in_own_city: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_per_path_tile: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            yield_bonus_per_follower: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            international_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            origin_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            destination_yield_modifier: vec![(u32::MAX, i32::MIN), (7, 257), (7, -1)],
            flat_yield_bonus_per_player: std::array::from_fn(|i| vec![(i as u32, i as i32 - 32)]),
            flat_origin_yield_per_improvement_at_destination: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_improvement_at_destination: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_international_yield_per_improvement_at_origin: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            per_specialty_district_yield_bonus_for_international: vec![
                (u32::MAX, i32::MIN),
                (7, 257),
                (7, -1),
            ],
            flat_origin_yield_per_terrain_for_domestic: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_terrain_for_domestic: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_origin_yield_per_terrain_for_international: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            flat_destination_yield_per_terrain_for_international: vec![
                (0x81234567, vec![(9, -257), (9, 513)]),
                (0x81234567, vec![]),
            ],
            international_majors_trade_disabled_count: 49i32.wrapping_mul(-257),
            international_minors_trade_disabled_count: 50i32.wrapping_mul(-257),
            domestic_trade_disabled_count: 51i32.wrapping_mul(-257),
        }
    }
    #[test]
    fn nonempty_exact_and_short_budgets() {
        let v = sample();
        let generous = Limits {
            max_bytes: 100000,
            max_items: 100000,
        };
        let (b, items) = v.encode_with_items(generous).unwrap();
        let exact = Limits {
            max_bytes: b.len(),
            max_items: items,
        };
        assert_eq!(Trade::decode(&b, exact).unwrap(), v);
        assert_eq!(
            Trade::decode_prefix_with_items(&b, exact).unwrap(),
            (v.clone(), b.len(), items)
        );
        for end in 0..b.len() {
            assert!(Trade::decode(&b[..end], exact).is_err(), "prefix {end}");
        }
        for short in [
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(Trade::decode(&b, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn versions_bools_trailing_and_hostile_counts() {
        let v = sample();
        let lim = Limits {
            max_bytes: 100000,
            max_items: 100000,
        };
        let b = v.encode(lim).unwrap();
        for version in [0, 34, 36, u32::MAX] {
            let mut x = b.clone();
            x[..4].copy_from_slice(&version.to_le_bytes());
            assert!(matches!(Trade::decode(&x, lim), Err(Error::Version { .. })));
        }
        let mut x = b.clone();
        x.push(1);
        assert_eq!(Trade::decode(&x, lim), Err(Error::Trailing));
        assert_eq!(Trade::decode_prefix(&x, lim).unwrap(), (v, b.len()));
        let mut x = b.clone();
        x[72] = 2;
        assert_eq!(Trade::decode(&x, lim), Err(Error::InvalidBool(2)));
        let mut x = b;
        x[68..72].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Trade::decode(&x, lim), Err(Error::ResourceLimit));
    }
    #[test]
    fn legacy22_omissions_preserve_present_pairs_and_reset_defaults() {
        // Source version 22 grammar: header+10 scalars,9 Yield tables, scalar,
        //5 Yield tables,64 uncounted player tables,2 Improvement tables,
        //international-specialty Yield table and4 Terrain tables.
        let mut b = Vec::new();
        for x in [22u32, u32::MAX, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        for x in [
            2u32,
            0xfedcba98,
            i32::MIN as u32,
            0xfedcba98,
            (-257i32) as u32,
        ] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        for _ in 0..8 {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        b.extend_from_slice(&(-2i32).to_le_bytes());
        for _ in 0..(5 + 64) {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        for x in [1u32, u32::MAX, 1, 0x80000001, (-256i32) as u32] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        for _ in 0..(1 + 1 + 4) {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        let exact = Limits {
            max_bytes: b.len(),
            max_items: 68,
        };
        let (v, n, items) = Trade::decode_prefix_with_items(&b, exact).unwrap();
        assert_eq!((n, items), (b.len(), 68));
        assert_eq!(v.wire_version, 22);
        assert_eq!(
            (
                v.player,
                v.outgoing_route_capacity,
                v.extra_water_range_refuel,
                v.anyone_trade_to_favor
            ),
            (-1, 1, 10, 8)
        );
        assert_eq!(v.techs_civics_per_yield, -2);
        assert_eq!(
            v.flat_yield_bonus,
            vec![(0xfedcba98, i32::MIN), (0xfedcba98, -257)]
        );
        assert_eq!(
            v.flat_origin_yield_per_improvement_at_destination,
            vec![(u32::MAX, vec![(0x80000001, -256)])]
        );
        assert_eq!(v.free_tech_boost_first_trading_post_each_civ, 0);
        assert_eq!(v.free_civic_boost_first_trading_post_each_civ, 0);
        assert_eq!(v.trade_route_capacity_increase_on_meeting, 0);
        assert_eq!(
            (
                v.forbid_land_route_domestic,
                v.forbid_land_route_international
            ),
            (0, 0)
        );
        assert_eq!(
            (
                v.international_majors_trade_disabled_count,
                v.international_minors_trade_disabled_count,
                v.domestic_trade_disabled_count
            ),
            (0, 0, 0)
        );
        assert_eq!(
            v.civ_received_civic_boost_first_trading_post,
            vec![false; 64]
        );
        assert_eq!(
            v.civ_received_tech_boost_first_trading_post,
            vec![false; 64]
        );
        assert!(!v.boost_receipts_wire_present());
        assert!(v.international_yield_modifier.is_empty()); // no wire pairs
        assert_eq!(
            v.omitted_definition_table_default_q8(
                OmittedDefinitionTable::InternationalYieldModifier
            ),
            Some(256)
        );
        assert_eq!(
            v.omitted_definition_table_default_q8(
                OmittedDefinitionTable::InternationalYieldPerImprovementAtOrigin
            ),
            Some(0)
        );
        assert_eq!(
            sample().omitted_definition_table_default_q8(OmittedDefinitionTable::CityStateYield),
            None
        );
        assert_eq!(v.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..b.len() {
            assert!(Trade::decode(&b[..n], exact).is_err(), "prefix {n}");
        }
        assert_eq!(
            Trade::decode(
                &b,
                Limits {
                    max_items: 67,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            Trade::decode(
                &b,
                Limits {
                    max_bytes: b.len() - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        let mut tail = b.clone();
        tail.push(1);
        assert_eq!(
            Trade::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        b[..4].copy_from_slice(&21u32.to_le_bytes());
        assert_eq!(
            Trade::decode(&b, exact),
            Err(Error::Version {
                expected: 35,
                actual: 21
            })
        );
    }
}
