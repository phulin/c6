//! Reached Linux XP2 Deal Manager2 / Deal5 / Item5 saved-state transport.
//! Supports the reached Gold1, Resources2 and Agreements2 payloads only.
//! Saved IDs, factory/type/subtype hashes, raw flags and parameter maps remain
//! distinct. Native AddDeal/AddItem, parent linking, resource/technology index
//! conversion, callbacks, RNG and gameplay installation are not applied.

use crate::unit::variants::{self, TypedVariantMap};
pub use crate::unit::Limits;
pub const VERSION: u32 = 2;
pub const GOLD: u32 = 0x05eb0df7;
pub const RESOURCES: u32 = 0xc03f9783;
pub const AGREEMENTS: u32 = 0x8bbf84cc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    UnsupportedItem(u32),
    Parameters(variants::VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Diplomacy.Deals wire: {self:?}")
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Payload {
    Gold { wire_version: u32, amount: i32 },
    Resources { wire_version: u32, amount: i32 },
    Agreements { wire_version: u32 },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub factory_type_hash: u32,
    pub wire_version: u32,
    pub saved_id: u32,
    pub saved_parent_id: u32,
    pub saved_type_hash: u32,
    pub subtype_hash: u32,
    pub enact_turn: i32,
    pub duration: i32,
    pub from_player: i32,
    pub to_player: i32,
    pub from_renewed_raw: u8,
    pub to_renewed_raw: u8,
    pub locked_raw: u8,
    pub unacceptable_raw: u8,
    pub parameters_wire_version: u32,
    pub parameters: TypedVariantMap,
    pub payload: Payload,
}
impl Wire for Item {
    const MIN: usize = 56;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let factory_type_hash = u32::read(r)?;
        if !matches!(factory_type_hash, GOLD | RESOURCES | AGREEMENTS) {
            return Err(Error::UnsupportedItem(factory_type_hash));
        }
        let wire_version = r.version(5)?;
        let saved_id = u32::read(r)?;
        let saved_parent_id = u32::read(r)?;
        let saved_type_hash = u32::read(r)?;
        let subtype_hash = u32::read(r)?;
        let enact_turn = i32::read(r)?;
        let duration = i32::read(r)?;
        let from_player = i32::read(r)?;
        let to_player = i32::read(r)?;
        let from_renewed_raw = u8::read(r)?;
        let to_renewed_raw = u8::read(r)?;
        let locked_raw = u8::read(r)?;
        let unacceptable_raw = u8::read(r)?;
        let parameters_wire_version = r.version(1)?;
        let mut budget = variants::DecodeBudget::new(variants::Limits {
            max_bytes: r.limits.max_bytes - r.pos,
            max_items: r.limits.max_items - r.items,
            ..variants::Limits::default()
        });
        let (parameters, used) = TypedVariantMap::decode_prefix(&r.data[r.pos..], &mut budget)
            .map_err(Error::Parameters)?;
        r.pos += used;
        r.items += budget.items_used();
        let payload = match factory_type_hash {
            GOLD => Payload::Gold {
                wire_version: r.version(1)?,
                amount: i32::read(r)?,
            },
            RESOURCES => Payload::Resources {
                wire_version: r.version(2)?,
                amount: i32::read(r)?,
            },
            AGREEMENTS => Payload::Agreements {
                wire_version: r.version(2)?,
            },
            _ => unreachable!(),
        };
        Ok(Self {
            factory_type_hash,
            wire_version,
            saved_id,
            saved_parent_id,
            saved_type_hash,
            subtype_hash,
            enact_turn,
            duration,
            from_player,
            to_player,
            from_renewed_raw,
            to_renewed_raw,
            locked_raw,
            unacceptable_raw,
            parameters_wire_version,
            parameters,
            payload,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deal {
    pub wire_version: u32,
    pub saved_id: u32,
    pub player1: i32,
    pub player2: i32,
    pub enacted_raw: u8,
    pub demand_raw: u8,
    pub gift_raw: u8,
    pub items: Vec<Item>,
}
impl Wire for Deal {
    const MIN: usize = 23;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(5)?,
            saved_id: u32::read(r)?,
            player1: i32::read(r)?,
            player2: i32::read(r)?,
            enacted_raw: u8::read(r)?,
            demand_raw: u8::read(r)?,
            gift_raw: u8::read(r)?,
            items: Vec::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub next_enacted_deal_id: u32,
    pub deals: Vec<Deal>,
}
impl Manager {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if n != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
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
            next_enacted_deal_id: u32::read(&mut r)?,
            deals: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = &[
        2, 0, 0, 0, 152, 186, 220, 254, 1, 0, 0, 0, 5, 0, 0, 0, 255, 255, 255, 255, 253, 255, 255,
        255, 62, 0, 0, 0, 255, 2, 128, 3, 0, 0, 0, 247, 13, 235, 5, 5, 0, 0, 0, 77, 0, 0, 0, 254,
        255, 255, 255, 120, 86, 52, 18, 21, 180, 127, 96, 156, 255, 255, 255, 255, 255, 255, 127,
        255, 255, 255, 255, 63, 0, 0, 0, 255, 2, 128, 7, 1, 0, 0, 0, 1, 0, 0, 128, 1, 0, 0, 0, 7,
        0, 0, 0, 2, 0, 110, 109, 5, 0, 0, 0, 3, 0, 0, 33, 1, 0, 0, 0, 120, 121, 0, 1, 0, 0, 0, 249,
        255, 255, 255, 131, 151, 63, 192, 5, 0, 0, 0, 77, 0, 0, 0, 254, 255, 255, 255, 120, 86, 52,
        18, 21, 180, 127, 96, 156, 255, 255, 255, 255, 255, 255, 127, 255, 255, 255, 255, 63, 0, 0,
        0, 255, 2, 128, 7, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 249, 255, 255, 255, 204, 132, 191,
        139, 5, 0, 0, 0, 77, 0, 0, 0, 254, 255, 255, 255, 120, 86, 52, 18, 21, 180, 127, 96, 156,
        255, 255, 255, 255, 255, 255, 127, 255, 255, 255, 255, 63, 0, 0, 0, 255, 2, 128, 7, 1, 0,
        0, 0, 0, 0, 0, 0, 2, 0, 0, 0,
    ];
    #[test]
    fn saved_identity_flags_factory_and_payloads_remain_distinct() {
        let limits = Limits {
            max_bytes: FIXTURE.len(),
            max_items: 10,
        };
        let (m, n, i) = Manager::decode_prefix_with_items(FIXTURE, limits).unwrap();
        assert_eq!((n, i), (FIXTURE.len(), 10));
        assert_eq!(m.next_enacted_deal_id, 0xfedcba98);
        let d = &m.deals[0];
        assert_eq!((d.saved_id, d.player1, d.player2), (u32::MAX, -3, 62));
        assert_eq!((d.enacted_raw, d.demand_raw, d.gift_raw), (255, 2, 128));
        let item = &d.items[0];
        assert_eq!(
            (
                item.factory_type_hash,
                item.saved_type_hash,
                item.subtype_hash
            ),
            (GOLD, 0x12345678, 0x607fb415)
        );
        assert_eq!((item.saved_id, item.saved_parent_id), (77, u32::MAX - 1));
        assert_eq!(
            (
                item.enact_turn,
                item.duration,
                item.from_player,
                item.to_player
            ),
            (-100, i32::MAX, -1, 63)
        );
        assert_eq!(
            (
                item.from_renewed_raw,
                item.to_renewed_raw,
                item.locked_raw,
                item.unacceptable_raw
            ),
            (255, 2, 128, 7)
        );
        assert_eq!(
            item.parameters.entries[0].name.as_deref(),
            Some(b"nm".as_slice())
        );
        assert_eq!(
            item.payload,
            Payload::Gold {
                wire_version: 1,
                amount: -7
            }
        );
        assert_eq!(
            d.items[1].payload,
            Payload::Resources {
                wire_version: 2,
                amount: -7
            }
        );
        assert_eq!(d.items[2].payload, Payload::Agreements { wire_version: 2 });
        assert!(d.items[1].parameters.entries.is_empty());
        let mut tail = FIXTURE.to_vec();
        tail.extend([0x55, 0xaa]);
        assert_eq!(
            Manager::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Manager::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            FIXTURE.len()
        );
    }
    #[test]
    fn independent_tags_all_truncations_budgets_and_hostile_counts_reject() {
        for cut in 0..FIXTURE.len() {
            assert!(
                Manager::decode(&FIXTURE[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
            assert!(
                Manager::decode(
                    FIXTURE,
                    Limits {
                        max_bytes: cut,
                        max_items: 10
                    }
                )
                .is_err(),
                "byte cap {cut}"
            );
        }
        for max_items in 0..10 {
            assert!(Manager::decode(
                FIXTURE,
                Limits {
                    max_bytes: FIXTURE.len(),
                    max_items
                }
            )
            .is_err());
        }
        for (at, expected) in TAGS {
            let mut bad = FIXTURE.to_vec();
            bad[*at..*at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: *expected,
                    actual: 99
                })
            );
        }
        for at in COUNTS {
            let mut bad = FIXTURE.to_vec();
            bad[*at..*at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(Manager::decode(&bad, Limits::default()).is_err());
        }
        let mut bad = FIXTURE.to_vec();
        bad[35..39].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, Limits::default()),
            Err(Error::UnsupportedItem(0))
        );
        // A recognized but unreached kind must never silently skip its payload.
        bad[35..39].copy_from_slice(&0x89431d84u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, Limits::default()),
            Err(Error::UnsupportedItem(0x89431d84))
        );
    }
    const TAGS: &[(usize, u32)] = &[
        (0, 2),
        (12, 5),
        (39, 5),
        (79, 1),
        (114, 1),
        (126, 5),
        (166, 1),
        (174, 2),
        (186, 5),
        (226, 1),
        (234, 2),
    ];
    const COUNTS: &[usize] = &[8, 31, 170, 230];
}
