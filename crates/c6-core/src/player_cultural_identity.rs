//! Complete current Linux Player.CulturalIdentity saved wire only.
//! Raw Q8 values are signed backing integers. Native archive effects, initialization,
//! gameplay decisions, actions and turns are not performed.
use crate::unit::Limits;
pub const VERSION: u32 = 10;
pub const WIRE_BYTES: usize = 60;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Player.CulturalIdentity wire: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Fourteen scalar members in original writer order; `_raw` denotes Q8 backing bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CulturalIdentity {
    pub extra_capital_pressure_radius: i32,
    pub identity_per_turn_for_domestic_trade_route_origin_raw: i32,
    pub martial_law_modifier: i32,
    pub post_combat_foreign_loyalty: i32,
    pub post_combat_local_loyalty: i32,
    pub additional_golden_age_post_combat_loyalty: i32,
    pub post_pillage_foreign_loyalty: i32,
    pub post_pillage_local_loyalty: i32,
    pub get_skip_free_city_step: i32,
    pub foreign_identity_per_turn_from_nearby_great_works_raw: i32,
    pub domestic_identity_per_turn_from_nearby_great_works_raw: i32,
    pub foreign_identity_per_turn_from_nearby_wonders_raw: i32,
    pub domestic_identity_per_turn_from_nearby_wonders_raw: i32,
    pub always_loyal_coastal_home_continent_count: i32,
}
impl CulturalIdentity {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used, _) = Self::decode_prefix_with_items(data, limits)?;
        if used != data.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(data, limits).map(|(v, n, _)| (v, n))
    }
    pub fn decode_prefix_with_items(
        data: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        if limits.max_bytes < 4 {
            return Err(Error::ResourceLimit);
        }
        let actual = u32::from_le_bytes(data.get(..4).ok_or(Error::Truncated)?.try_into().unwrap());
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        if limits.max_bytes < WIRE_BYTES {
            return Err(Error::ResourceLimit);
        }
        let bytes = data.get(..WIRE_BYTES).ok_or(Error::Truncated)?;
        let mut words = bytes[4..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| i32::from_le_bytes(*b));
        Ok((
            Self {
                extra_capital_pressure_radius: words.next().unwrap(),
                identity_per_turn_for_domestic_trade_route_origin_raw: words.next().unwrap(),
                martial_law_modifier: words.next().unwrap(),
                post_combat_foreign_loyalty: words.next().unwrap(),
                post_combat_local_loyalty: words.next().unwrap(),
                additional_golden_age_post_combat_loyalty: words.next().unwrap(),
                post_pillage_foreign_loyalty: words.next().unwrap(),
                post_pillage_local_loyalty: words.next().unwrap(),
                get_skip_free_city_step: words.next().unwrap(),
                foreign_identity_per_turn_from_nearby_great_works_raw: words.next().unwrap(),
                domestic_identity_per_turn_from_nearby_great_works_raw: words.next().unwrap(),
                foreign_identity_per_turn_from_nearby_wonders_raw: words.next().unwrap(),
                domestic_identity_per_turn_from_nearby_wonders_raw: words.next().unwrap(),
                always_loyal_coastal_home_continent_count: words.next().unwrap(),
            },
            WIRE_BYTES,
            0,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        if limits.max_bytes < WIRE_BYTES {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(WIRE_BYTES)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        for value in [
            self.extra_capital_pressure_radius,
            self.identity_per_turn_for_domestic_trade_route_origin_raw,
            self.martial_law_modifier,
            self.post_combat_foreign_loyalty,
            self.post_combat_local_loyalty,
            self.additional_golden_age_post_combat_loyalty,
            self.post_pillage_foreign_loyalty,
            self.post_pillage_local_loyalty,
            self.get_skip_free_city_step,
            self.foreign_identity_per_turn_from_nearby_great_works_raw,
            self.domestic_identity_per_turn_from_nearby_great_works_raw,
            self.foreign_identity_per_turn_from_nearby_wonders_raw,
            self.domestic_identity_per_turn_from_nearby_wonders_raw,
            self.always_loyal_coastal_home_continent_count,
        ] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        Ok((out, 0))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn carrier() -> Vec<u8> {
        let mut b = VERSION.to_le_bytes().to_vec();
        for v in [
            i32::MIN,
            i32::MAX,
            -1,
            0,
            1,
            255,
            256,
            -256,
            257,
            -257,
            0x12345678,
            -123456789,
            42,
            -42,
        ] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }
    #[test]
    fn signed_raw_roundtrip() {
        let b = carrier();
        let lim = Limits {
            max_bytes: 60,
            max_items: 0,
        };
        let (v, n, i) = CulturalIdentity::decode_prefix_with_items(&b, lim).unwrap();
        assert_eq!((n, i), (60, 0));
        assert_eq!(
            v.identity_per_turn_for_domestic_trade_route_origin_raw,
            i32::MAX
        );
        assert_eq!(
            v.foreign_identity_per_turn_from_nearby_great_works_raw,
            -257
        );
        assert_eq!(v.encode_with_items(lim).unwrap(), (b.clone(), 0));
        let mut suffix = b.clone();
        suffix.push(0);
        assert_eq!(CulturalIdentity::decode(&suffix, lim), Err(Error::Trailing));
        assert_eq!(
            CulturalIdentity::decode_prefix(&suffix, lim).unwrap(),
            (v, 60)
        );
    }
    #[test]
    fn every_truncation_and_byte_cap() {
        let b = carrier();
        let lim = Limits {
            max_bytes: 60,
            max_items: 0,
        };
        let v = CulturalIdentity::decode(&b, lim).unwrap();
        for n in 0..60 {
            assert_eq!(
                CulturalIdentity::decode(&b[..n], lim),
                Err(Error::Truncated)
            );
            let short = Limits {
                max_bytes: n,
                ..lim
            };
            assert_eq!(
                CulturalIdentity::decode(&b, short),
                Err(Error::ResourceLimit)
            );
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn rejects_unknown_versions() {
        for version in [0, 1, 9, 11, u32::MAX] {
            let mut b = carrier();
            b[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                CulturalIdentity::decode(
                    &b,
                    Limits {
                        max_bytes: 60,
                        max_items: 0
                    }
                ),
                Err(Error::Version {
                    expected: 10,
                    actual: version
                })
            );
        }
    }
}
