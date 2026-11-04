//! Linux XP2 DiplomaticRelations14 serialized projection only.
//! Relationship occurrence order, raw bool bytes, definition hashes and signed
//! values/turns are retained. This does not install native FAutoVariable state,
//! remap definitions, apply PostLoad or implement diplomacy gameplay.
//! GrievanceLog (since15) is absent at14 and its prior runtime value is unknown.
//! Only the independently evidenced reached version14 is supported.

pub use crate::unit::Limits;
pub const VERSION: u32 = 14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.DiplomaticRelations wire: {self:?}")
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
    fn raw(&mut self, size: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(size).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn version(&mut self, expected: u32) -> Result<u32, Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(actual)
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
}
impl Wire for u32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(4)?.try_into().unwrap()))
    }
}
impl Wire for i32 {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(u32::read(r)? as i32)
    }
}
impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.raw(1)?[0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let count = u32::read(r)? as usize;
        r.items = r.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let minimum = count.checked_mul(T::MIN).ok_or(Error::ResourceLimit)?;
        if minimum > r.limits.max_bytes - r.pos {
            return Err(Error::ResourceLimit);
        }
        if minimum > r.data.len() - r.pos {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(T::read(r)?);
        }
        Ok(values)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relationship<T> {
    pub value: T,
    pub changed_turn: i32,
}
impl<T: Wire> Wire for Relationship<T> {
    const MIN: usize = T::MIN + 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            value: T::read(r)?,
            changed_turn: i32::read(r)?,
        })
    }
}
/// The native version gate retains the previous value without reading bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedPriorState {
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiplomaticRelations {
    pub wire_version: u32,
    pub max_num_players: i32,
    pub has_met: Vec<Relationship<u8>>,
    pub has_declared_friendship: Vec<Relationship<u8>>,
    pub is_permanently_allied: Vec<Relationship<u8>>,
    pub is_in_defensive_pact: Vec<Relationship<u8>>,
    pub research_agreement_tech_hashes: Vec<Relationship<u32>>,
    pub at_war_type_hashes: Vec<Relationship<u32>>,
    pub war_declarer_ordinals: Vec<Relationship<i32>>,
    pub joint_war_target_ordinals: Vec<Relationship<i32>>,
    pub allies_share_visibility_raw: u8,
    pub alliance_type_hashes: Vec<Relationship<u32>>,
    pub shares_suzerain: Vec<Relationship<u8>>,
    pub alliance_amount_hashes: Vec<Relationship<u32>>,
    pub num_grievances: Vec<Relationship<i32>>,
    pub grievance_holder_ordinals: Vec<Relationship<i32>>,
    pub tension_score: Vec<Relationship<i32>>,
    pub grievance_log: OmittedPriorState,
}
impl DiplomaticRelations {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((v, n))
    }
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
        let v = Self {
            wire_version: r.version(VERSION)?,
            max_num_players: i32::read(&mut r)?,
            has_met: Vec::read(&mut r)?,
            has_declared_friendship: Vec::read(&mut r)?,
            is_permanently_allied: Vec::read(&mut r)?,
            is_in_defensive_pact: Vec::read(&mut r)?,
            research_agreement_tech_hashes: Vec::read(&mut r)?,
            at_war_type_hashes: Vec::read(&mut r)?,
            war_declarer_ordinals: Vec::read(&mut r)?,
            joint_war_target_ordinals: Vec::read(&mut r)?,
            allies_share_visibility_raw: u8::read(&mut r)?,
            alliance_type_hashes: Vec::read(&mut r)?,
            shares_suzerain: Vec::read(&mut r)?,
            alliance_amount_hashes: Vec::read(&mut r)?,
            num_grievances: Vec::read(&mut r)?,
            grievance_holder_ordinals: Vec::read(&mut r)?,
            tension_score: Vec::read(&mut r)?,
            grievance_log: OmittedPriorState::Unknown,
        };
        Ok((v, r.pos, r.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        fn words(b: &mut Vec<u8>, v: &[u32]) {
            for n in v {
                b.extend(n.to_le_bytes());
            }
        }
        words(&mut b, &[14, 64, 2]);
        b.push(255);
        words(&mut b, &[u32::MAX]);
        b.push(2);
        words(&mut b, &[7]);
        for (flag, turn) in [(3, 11), (4, 12), (5, 13)] {
            words(&mut b, &[1]);
            b.push(flag);
            words(&mut b, &[turn]);
        }
        words(
            &mut b,
            &[
                1,
                0x81234567,
                21,
                1,
                0xfedcba98,
                22,
                1,
                u32::MAX,
                23,
                1,
                63,
                24,
            ],
        );
        b.push(254);
        words(&mut b, &[1, 0x87654321, 31, 1]);
        b.push(6);
        words(&mut b, &[32]);
        words(
            &mut b,
            &[
                1,
                0xabcdef01,
                33,
                1,
                (-99i32) as u32,
                34,
                1,
                62,
                35,
                1,
                (-77i32) as u32,
                36,
            ],
        );
        b
    }
    #[test]
    fn literal_order_hashes_signed_values_and_omitted_prior_log() {
        let b = fixture();
        assert_eq!(b.len(), 167);
        let (v, n, i) =
            DiplomaticRelations::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (167, 15));
        assert_eq!(v.max_num_players, 64);
        assert_eq!(
            v.has_met,
            vec![
                Relationship {
                    value: 255,
                    changed_turn: -1
                },
                Relationship {
                    value: 2,
                    changed_turn: 7
                }
            ]
        );
        assert_eq!(v.has_declared_friendship[0].value, 3);
        assert_eq!(v.is_permanently_allied[0].value, 4);
        assert_eq!(v.is_in_defensive_pact[0].value, 5);
        assert_eq!(v.research_agreement_tech_hashes[0].value, 0x81234567);
        assert_eq!(v.at_war_type_hashes[0].value, 0xfedcba98);
        assert_eq!(v.war_declarer_ordinals[0].value, -1);
        assert_eq!(v.joint_war_target_ordinals[0].value, 63);
        assert_eq!(v.allies_share_visibility_raw, 254);
        assert_eq!(v.alliance_type_hashes[0].value, 0x87654321);
        assert_eq!(v.shares_suzerain[0].value, 6);
        assert_eq!(v.alliance_amount_hashes[0].value, 0xabcdef01);
        assert_eq!(v.num_grievances[0].value, -99);
        assert_eq!(v.grievance_holder_ordinals[0].value, 62);
        assert_eq!(
            v.tension_score[0],
            Relationship {
                value: -77,
                changed_turn: 36
            }
        );
        assert_eq!(v.grievance_log, OmittedPriorState::Unknown);
    }
    #[test]
    fn every_cut_budget_version_and_prefix_boundary() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                DiplomaticRelations::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                DiplomaticRelations::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..15 {
            assert_eq!(
                DiplomaticRelations::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for version in [0u32, 1, 8, 13, 15, u32::MAX] {
            let mut bad = b.clone();
            bad[..4].copy_from_slice(&version.to_le_bytes());
            assert_eq!(
                DiplomaticRelations::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 14,
                    actual: version
                })
            );
        }
        let mut hostile = b.clone();
        hostile[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            DiplomaticRelations::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let mut tail = b.clone();
        tail.extend([15, 0, 0, 0]);
        assert_eq!(
            DiplomaticRelations::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            DiplomaticRelations::decode_prefix(&tail, Limits::default())
                .unwrap()
                .1,
            b.len()
        );
    }
}
