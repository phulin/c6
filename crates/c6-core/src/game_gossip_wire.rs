//! Linux XP2 Gossip3 wire projection, independent payload6 and TurnVisibility3.
//! Retains constructor and saved type hashes separately, ordered occurrences,
//! raw flags, definition hashes, strings and visibility words. Native handler
//! registration, definition remaps, compact-field setters, queued-pointer
//! rebinding, events and gameplay are not applied. No native UI policy runs.

pub use crate::unit::{Limits, PlotCoord};
pub const VERSION: u32 = 3;
pub const PAYLOAD_VERSION: u32 = 6;
pub const VISIBILITY_VERSION: u32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    UnknownConstructor { type_hash: u32 },
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Gossip wire: {self:?}")
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
impl Wire for PlotCoord {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            x: i32::read(r)?,
            y: i32::read(r)?,
        })
    }
}
macro_rules! payloads {
    ($( $name:ident { $( $field:ident : $ty:ty ),* } => [ $( $hash:literal ),+ ]; )*) => {
        #[derive(Clone,Debug,Eq,PartialEq)]
        pub enum Payload { $( $name { $( $field : $ty ),* }, )* }
        impl Payload {
            fn supports_type(hash:u32)->bool { matches!(hash, $( $( $hash )|+ )|*) }
            fn read(hash:u32,r:&mut Reader<'_>)->Result<Self,Error> {
                match hash {
                    $( $( $hash )|+ => Ok(Self::$name {$( $field: <$ty>::read(r)? ),*}), )*
                    _ => Err(Error::UnknownConstructor {type_hash:hash}),
                }
            }
        }
    };
}
payloads! {
    Agenda { agenda_leader_raw:i32,other_leader_raw:i32,other_player_ordinal:i32,modifier_hash:u32 } => [2326925441,3354988722];
    Anarchy { new_government_hash:u32,old_government_hash:u32,turns:i32 } => [1878998816];
    Artifact { great_work_hash:u32 } => [2196298948,2334537708];
    CityBuilding { city_id:u32,building_hash:u32 } => [518018841,2978276111];
    City { city_id:u32 } => [607510360,843182221,1274348112,1300653782,2153904494];
    CityDistrict { city_id:u32,district_hash:u32 } => [1630197340];
    CityState { city_state_ordinal:i32,influence_added:i32 } => [84372384];
    ConquerCity { old_owner_ordinal:i32,new_city_id:u32 } => [238720930,3234026731];
    Government { government_hash:u32 } => [1027546484];
    GreatPerson { unit_hash:u32,class_hash:u32,individual_hash:u32 } => [4237839488];
    OtherPlayer { other_player_ordinal:i32 } => [62994617,235155835,526321474,563310662,944135207,1046181038,1781195325,1919056712,2021101175,2740050500,2904288945,3957268140,4013563915];
    PlayerBelief { belief_hash:u32 } => [56826145];
    PlayerCivic { civic_hash:u32 } => [558761677];
    PlayerEra { old_era_hash:u32,new_era_hash:u32 } => [3501661599];
    PlayerFeature { feature_hash:u32 } => [1987996187];
    PlayerReligion { religion_hash:u32 } => [1292432916,2003196433,3738986836];
    PlayerTech { tech_hash:u32 } => [2060601050];
    Plot { coordinate:PlotCoord } => [855647858,1467001846];
    Policy { policy_hash:u32 } => [2637133864];
    Project { project_hash:u32 } => [79918836,3872654018];
    RandomEvent { event_hash:u32 } => [3317513695];
    RockConcert { band_player_ordinal:i32,city_id:u32,band_name:Vec<u8>,result_hash:u32 } => [1690227124];
    Spy { spy_player_ordinal:i32,city_id:u32,detected_raw:u8 } => [30913075,331451630,714329741,810463566,837680396,1981938698,2125791539,3896805381,4118109735,4177905494];
    Strategy { strategy_hash:u32,started_raw:u8 } => [977617119];
    Unit { unit_hash:u32 } => [47448450,1585430139,2186896208];
    UnitImprovement { unit_hash:u32,improvement_hash:u32 } => [563179799];
    UnitImprovementBuilding { unit_hash:u32,improvement_hash:u32,building_hash:u32,district_hash:u32 } => [689589196];
    UnitPromotion { unit_hash:u32,promotion_hash:u32 } => [1071911233,3146089312];
    WMD { count:i32 } => [1420295525];
    WMDStrike { wmd_raw:i32 } => [2292620293];
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub constructor_type_hash: u32,
    pub saved_type_hash: u32,
    pub game_turn: i32,
    pub viewed_player_ordinal: i32,
    pub payload_version: u32,
    pub payload: Payload,
}
impl Wire for Occurrence {
    const MIN: usize = 24;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let constructor_type_hash = u32::read(r)?;
        if !Payload::supports_type(constructor_type_hash) {
            return Err(Error::UnknownConstructor {
                type_hash: constructor_type_hash,
            });
        }
        let saved_type_hash = u32::read(r)?;
        let game_turn = i32::read(r)?;
        let viewed_player_ordinal = i32::read(r)?;
        let payload_version = r.version(PAYLOAD_VERSION)?;
        let payload = Payload::read(constructor_type_hash, r)?;
        Ok(Self {
            constructor_type_hash,
            saved_type_hash,
            game_turn,
            viewed_player_ordinal,
            payload_version,
            payload,
        })
    }
}
/// Raw wire words before the native setter narrowing/definition validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnVisibility {
    pub wire_version: u32,
    pub turn: i32,
    pub viewing_player_ordinal: i32,
    pub viewed_player_ordinal: i32,
    pub visibility_raw: i32,
    pub source_hash: u32,
    pub source_name_index: i32,
}
impl Wire for TurnVisibility {
    const MIN: usize = 28;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: r.version(VISIBILITY_VERSION)?,
            turn: i32::read(r)?,
            viewing_player_ordinal: i32::read(r)?,
            viewed_player_ordinal: i32::read(r)?,
            visibility_raw: i32::read(r)?,
            source_hash: u32::read(r)?,
            source_name_index: i32::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: u32,
    pub gossip_items: Vec<Occurrence>,
    pub turn_visibilities: Vec<TurnVisibility>,
    /// Serialized queue records, before native matching/rebinding to item pointers.
    pub queued_gossips: Vec<Occurrence>,
}
impl Manager {
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
            gossip_items: Vec::read(&mut r)?,
            turn_visibilities: Vec::read(&mut r)?,
            queued_gossips: Vec::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn words(b: &mut Vec<u8>, v: &[u32]) {
        for n in v {
            b.extend(n.to_le_bytes());
        }
    }
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        words(
            &mut b,
            &[
                3,
                3,
                1300653782,
                0xabcdef01,
                2,
                5,
                6,
                0x10000,
                2326925441,
                3354988722,
                3,
                63,
                6,
                17,
                18,
                u32::MAX,
                0xfedcba98,
                30913075,
                30913075,
                4,
                2,
                6,
                7,
                0x20000,
            ],
        );
        b.push(255);
        words(
            &mut b,
            &[
                2,
                3,
                70000,
                300,
                u32::MAX,
                260,
                0x81234567,
                u32::MAX,
                3,
                70001,
                301,
                62,
                261,
                0xfedcba98,
                77,
                3,
                1690227124,
                1690227124,
                5,
                3,
                6,
                4,
                0x30000,
                3,
            ],
        );
        b.extend([0, 255, 128]);
        words(
            &mut b,
            &[0x87654321, 977617119, 977617119, 6, 4, 6, 0xabcdef01],
        );
        b.push(2);
        words(
            &mut b,
            &[
                689589196, 689589196, 7, 5, 6, 0x81234567, 0x82345678, 0x83456789, 0x8456789a,
            ],
        );
        b
    }
    #[test]
    fn typed_literal_preserves_hashes_order_flags_strings_and_queue_records() {
        let b = fixture();
        assert_eq!(b.len(), 261);
        let (v, n, i) = Manager::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (261, 11));
        assert_eq!(v.gossip_items[0].constructor_type_hash, 1300653782);
        assert_eq!(v.gossip_items[0].saved_type_hash, 0xabcdef01);
        assert_eq!(
            v.gossip_items[0].payload,
            Payload::City { city_id: 0x10000 }
        );
        assert_eq!(
            v.gossip_items[1].payload,
            Payload::Agenda {
                agenda_leader_raw: 17,
                other_leader_raw: 18,
                other_player_ordinal: -1,
                modifier_hash: 0xfedcba98
            }
        );
        assert_eq!(
            v.gossip_items[2].payload,
            Payload::Spy {
                spy_player_ordinal: 7,
                city_id: 0x20000,
                detected_raw: 255
            }
        );
        assert_eq!(
            v.turn_visibilities[0],
            TurnVisibility {
                wire_version: 3,
                turn: 70000,
                viewing_player_ordinal: 300,
                viewed_player_ordinal: -1,
                visibility_raw: 260,
                source_hash: 0x81234567,
                source_name_index: -1
            }
        );
        assert_eq!(v.turn_visibilities[1].source_name_index, 77);
        assert_eq!(
            v.queued_gossips[0].payload,
            Payload::RockConcert {
                band_player_ordinal: 4,
                city_id: 0x30000,
                band_name: vec![0, 255, 128],
                result_hash: 0x87654321
            }
        );
        assert_eq!(
            v.queued_gossips[1].payload,
            Payload::Strategy {
                strategy_hash: 0xabcdef01,
                started_raw: 2
            }
        );
        assert_eq!(
            v.queued_gossips[2].payload,
            Payload::UnitImprovementBuilding {
                unit_hash: 0x81234567,
                improvement_hash: 0x82345678,
                building_hash: 0x83456789,
                district_hash: 0x8456789a
            }
        );
    }
    #[test]
    fn all_cuts_limits_independent_tags_unknown_dispatch_and_trailing() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                Manager::decode(&b[..cut], Limits::default()).is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                Manager::decode(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..11 {
            assert_eq!(
                Manager::decode(
                    &b,
                    Limits {
                        max_items: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for (at, expected) in [
            (0, 3),
            (24, 6),
            (48, 6),
            (84, 6),
            (101, 3),
            (129, 3),
            (177, 6),
            (216, 6),
            (241, 6),
        ] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                }),
                "at {at}"
            );
        }
        let mut bad = b.clone();
        bad[8..12].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            Manager::decode(&bad, Limits::default()),
            Err(Error::UnknownConstructor { type_hash: 0 })
        );
        for at in [4, 97, 157, 189] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Manager::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit),
                "at {at}"
            );
        }
        let mut tail = b.clone();
        tail.push(0);
        assert_eq!(
            Manager::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Manager::decode_prefix(&tail, Limits::default()).unwrap().1,
            b.len()
        );
    }
}
