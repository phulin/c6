//! Linux XP2 GameEffects4 wire transport for the reached saved profile.
//! ThroughRequirements stops before ModifierSystem; Effects composes the reached
//! ModifierSystem5 and closing guard. Only thirteen GUID domains are supported.
//! Saved IDs/reference fields are not native allocation, reuse or installation.
//! Improvement2 requires current-plot owner lookup. ObjectPool allocator counters,
//! runtime owners, callbacks/RNG effects and PostLoad remain unresolved here.

pub use crate::unit::{ComponentId, Limits};
pub const VERSION: u8 = 4;
pub const REQUIREMENTS_VERSION: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ModifierSystem(crate::game_effects_modifiers_wire::Error),
    UnsupportedGuid([u8; 16]),
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux GameEffects wire: {self:?}")
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
impl Wire for i16 {
    const MIN: usize = 2;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self::from_le_bytes(r.raw(2)?.try_into().unwrap()))
    }
}
impl Wire for ComponentId {
    const MIN: usize = 8;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            owner: i16::read(r)?,
            component_type: i16::read(r)?,
            id: u32::read(r)?,
        })
    }
}
fn byte_version(r: &mut Reader<'_>, expected: u8) -> Result<u8, Error> {
    let actual = u8::read(r)?;
    if actual != expected {
        return Err(Error::Version {
            expected: expected.into(),
            actual: actual.into(),
        });
    }
    Ok(actual)
}
macro_rules! references {
    ($( $name:ident { $( $field:ident : $ty:ty ),* } => ($guid:ident,$version:literal); )*) => {
        #[derive(Clone,Debug,Eq,PartialEq)]
        pub enum LogicalReference { $( $name { wire_version:u32, $( $field:$ty ),* }, )* }
        impl LogicalReference {
            fn read(guid:[u8;16],r:&mut Reader<'_>)->Result<Self,Error> {
                match guid {
                    $( $guid => Ok(Self::$name {wire_version:r.version($version)?, $( $field:<$ty>::read(r)? ),*}), )*
                    _=>Err(Error::UnsupportedGuid(guid)),
                }
            }
        }
    };
}
pub const GUID_ALLIANCE: [u8; 16] = [
    171, 39, 18, 105, 86, 124, 217, 75, 177, 10, 114, 157, 43, 117, 102, 157,
];
pub const GUID_BELIEF: [u8; 16] = [
    78, 87, 131, 9, 181, 195, 69, 67, 191, 90, 159, 116, 132, 91, 102, 151,
];
pub const GUID_CITY: [u8; 16] = [
    132, 151, 118, 143, 209, 58, 95, 77, 165, 86, 199, 33, 239, 151, 23, 96,
];
pub const GUID_CONGRESS: [u8; 16] = [
    222, 152, 199, 76, 201, 37, 234, 65, 138, 173, 170, 95, 194, 95, 220, 247,
];
pub const GUID_DISTRICT: [u8; 16] = [
    17, 203, 71, 142, 79, 109, 20, 68, 171, 194, 70, 111, 49, 29, 211, 194,
];
pub const GUID_EMERGENCY: [u8; 16] = [
    27, 0, 50, 168, 17, 114, 24, 69, 137, 147, 189, 216, 160, 180, 213, 69,
];
pub const GUID_GAME: [u8; 16] = [
    226, 91, 217, 199, 226, 72, 58, 68, 135, 73, 109, 145, 249, 41, 206, 237,
];
pub const GUID_GOVERNOR: [u8; 16] = [
    7, 200, 70, 25, 248, 4, 154, 72, 137, 230, 138, 126, 27, 155, 51, 35,
];
pub const GUID_IMPROVEMENT: [u8; 16] = [
    58, 94, 172, 31, 120, 215, 26, 69, 167, 162, 17, 234, 12, 110, 14, 25,
];
pub const GUID_PLAYER: [u8; 16] = [
    54, 188, 96, 107, 174, 38, 58, 66, 188, 64, 34, 138, 140, 123, 13, 77,
];
pub const GUID_TEAM: [u8; 16] = [
    105, 49, 85, 206, 188, 26, 147, 65, 151, 18, 0, 225, 121, 84, 141, 234,
];
pub const GUID_TRADEROUTE: [u8; 16] = [
    132, 109, 16, 71, 46, 245, 28, 77, 168, 232, 185, 31, 78, 167, 248, 214,
];
pub const GUID_UNIT: [u8; 16] = [
    163, 32, 96, 160, 55, 228, 53, 71, 147, 17, 67, 182, 36, 51, 173, 103,
];
references! {
    Alliance { first_player_ordinal:i32,second_player_ordinal:i32,alliance_type_hash:u32 } => (GUID_ALLIANCE,1);
    Belief { belief_type_hash:u32,player_ordinal:i32,religion_type_hash:u32 } => (GUID_BELIEF,1);
    City { player_ordinal:i32,city_id:u32,subtype_raw:i32,sub_id_raw:i32 } => (GUID_CITY,2);
    Congress {  } => (GUID_CONGRESS,1);
    District { player_ordinal:i32,district_id:u32,subtype_raw:i32,sub_id_raw:i32 } => (GUID_DISTRICT,2);
    Emergency { player_ordinal:i32,emergency_type_hash:u32,identifier_raw:i32 } => (GUID_EMERGENCY,1);
    Game { subtype_raw:i32,sub_id_raw:i32 } => (GUID_GAME,2);
    Governor { player_ordinal:i32,governor_id:u32,subtype_raw:i32,sub_id_raw:i32 } => (GUID_GOVERNOR,1);
    Improvement { plot_index:i32,improvement_type_hash:u32,city:ComponentId } => (GUID_IMPROVEMENT,2);
    Player { player_ordinal:i32,subtype_raw:i32,sub_id_raw:i32 } => (GUID_PLAYER,2);
    Team { team_ordinal:i32 } => (GUID_TEAM,1);
    TradeRoute { origin_city:ComponentId,destination_city:ComponentId } => (GUID_TRADEROUTE,1);
    Unit { player_ordinal:i32,unit_id:u32,subtype_raw:i32,sub_id_raw:i32 } => (GUID_UNIT,2);
}
/// Improvement2 has no serialized owner field. Its native owner must be read
/// from the current map plot before installing any gameplay reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImprovementOwnerInput {
    CurrentPlotOwnerRequired,
}
impl LogicalReference {
    pub fn improvement_owner_input(&self) -> Option<ImprovementOwnerInput> {
        matches!(self, Self::Improvement { .. })
            .then_some(ImprovementOwnerInput::CurrentPlotOwnerRequired)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModifierObject {
    Null {
        presence_raw: u8,
    },
    Present {
        presence_raw: u8,
        wrapper_version: u8,
        type_guid: [u8; 16],
        reference: LogicalReference,
    },
}
impl Wire for ModifierObject {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let presence_raw = u8::read(r)?;
        if presence_raw == 0 {
            return Ok(Self::Null { presence_raw });
        }
        let wrapper_version = byte_version(r, 1)?;
        let type_guid = r.raw(16)?.try_into().unwrap();
        let reference = LogicalReference::read(type_guid, r)?;
        Ok(Self::Present {
            presence_raw,
            wrapper_version,
            type_guid,
            reference,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectPoolEntry {
    /// Native Read sets its allocation counter to this value before MakeObject;
    /// existing-object reuse can produce another live ID. No remap is applied.
    pub saved_instance_id: i32,
    pub object: ModifierObject,
}
impl Wire for ObjectPoolEntry {
    const MIN: usize = 5;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            saved_instance_id: i32::read(r)?,
            object: ModifierObject::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementRecord {
    pub requirement_id: Vec<u8>,
    pub context_saved_id: i32,
    pub subject_saved_id: i32,
    pub state_raw: u8,
}
impl Wire for RequirementRecord {
    const MIN: usize = 13;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            requirement_id: Vec::read(r)?,
            context_saved_id: i32::read(r)?,
            subject_saved_id: i32::read(r)?,
            state_raw: u8::read(r)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementSystem {
    pub wire_version: u8,
    pub records: Vec<RequirementRecord>,
}
impl Wire for RequirementSystem {
    const MIN: usize = 5;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            wire_version: byte_version(r, REQUIREMENTS_VERSION)?,
            records: Vec::read(r)?,
        })
    }
}
/// Wire projection only; this is not a complete GameEffects component or loader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThroughRequirements {
    /// The native reader consumes this string without comparing its contents.
    pub opening_guard: Vec<u8>,
    pub wire_version: u8,
    pub objects: Vec<ObjectPoolEntry>,
    pub requirements: RequirementSystem,
}
impl ThroughRequirements {
    /// Leaves ModifierSystem and the closing guard untouched.
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
            opening_guard: Vec::read(&mut r)?,
            wire_version: byte_version(&mut r, VERSION)?,
            objects: Vec::read(&mut r)?,
            requirements: RequirementSystem::read(&mut r)?,
        };
        Ok((v, r.pos, r.items))
    }
}
/// Complete reached serialized component, with native installation still absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Effects {
    pub prefix: ThroughRequirements,
    pub modifiers: crate::game_effects_modifiers_wire::ModifierSystem,
    /// Native Read consumes this independently without comparing guard strings.
    pub closing_guard: Vec<u8>,
}
impl Effects {
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
        let (prefix, n, i) = ThroughRequirements::decode_prefix_with_items(bytes, limits)?;
        let remaining = Limits {
            max_bytes: limits
                .max_bytes
                .checked_sub(n)
                .ok_or(Error::ResourceLimit)?,
            max_items: limits
                .max_items
                .checked_sub(i)
                .ok_or(Error::ResourceLimit)?,
        };
        let (modifiers, m, j) =
            crate::game_effects_modifiers_wire::ModifierSystem::decode_prefix_with_items(
                bytes.get(n..).ok_or(Error::Truncated)?,
                remaining,
            )
            .map_err(Error::ModifierSystem)?;
        let mut r = Reader {
            data: bytes,
            pos: n.checked_add(m).ok_or(Error::ResourceLimit)?,
            items: i.checked_add(j).ok_or(Error::ResourceLimit)?,
            limits,
        };
        let closing_guard = Vec::read(&mut r)?;
        Ok((
            Self {
                prefix,
                modifiers,
                closing_guard,
            },
            r.pos,
            r.items,
        ))
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
    fn guid(b: &mut Vec<u8>, hex: &str) {
        for pair in hex.as_bytes().as_chunks::<2>().0 {
            b.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
        }
    }
    fn fixture() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[2]);
        b.extend([0, 255, 4]);
        words(&mut b, &[6, 337]);
        b.extend([255, 1]);
        guid(&mut b, "36bc606bae263a42bc40228a8c7b0d4d");
        words(&mut b, &[2, 63, (-7i32) as u32, 9, 337]);
        b.extend([1, 1]);
        guid(&mut b, "3a5eac1f78d71a45a7a211ea0c6e0e19");
        words(&mut b, &[2, (-99i32) as u32, 0xfedcba98]);
        b.extend((-1i16).to_le_bytes());
        b.extend((-7i16).to_le_bytes());
        words(&mut b, &[0x87654321, u32::MAX]);
        b.push(0);
        words(&mut b, &[i32::MIN as u32]);
        b.extend([1, 1]);
        guid(&mut b, "693155cebc1a9341971200e179548dea");
        words(&mut b, &[1, 63, 338]);
        b.extend([1, 1]);
        guid(&mut b, "8497768fd13a5f4da556c721ef971760");
        words(&mut b, &[2, u32::MAX, u32::MAX, 2, 3, 339]);
        b.extend([1, 1]);
        guid(&mut b, "846d10472ef51c4da8e8b91f4ea7f8d6");
        words(&mut b, &[1]);
        b.extend(15i16.to_le_bytes());
        b.extend(3i16.to_le_bytes());
        words(&mut b, &[0xabcdef01]);
        b.extend((-17i16).to_le_bytes());
        b.extend((-18i16).to_le_bytes());
        words(&mut b, &[0x888999ab]);
        b.push(3);
        words(&mut b, &[2, 3]);
        b.extend([0, 255, 128]);
        words(&mut b, &[337, u32::MAX]);
        b.push(255);
        words(&mut b, &[0, i32::MIN as u32, 337]);
        b.push(2);
        b
    }
    #[test]
    fn literal_logical_ids_typed_references_nulls_order_and_requirement_bytes() {
        let b = fixture();
        assert_eq!(b.len(), 244);
        let (v, n, i) =
            ThroughRequirements::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (244, 13));
        assert_eq!(v.opening_guard, vec![0, 255]);
        assert_eq!(v.objects.len(), 6);
        assert_eq!(
            v.objects[0].saved_instance_id,
            v.objects[1].saved_instance_id
        );
        assert!(matches!(
            &v.objects[0].object,
            ModifierObject::Present {
                presence_raw: 255,
                reference: LogicalReference::Player {
                    wire_version: 2,
                    player_ordinal: 63,
                    subtype_raw: -7,
                    sub_id_raw: 9
                },
                ..
            }
        ));
        let ModifierObject::Present { reference, .. } = &v.objects[1].object else {
            panic!()
        };
        assert_eq!(
            reference,
            &LogicalReference::Improvement {
                wire_version: 2,
                plot_index: -99,
                improvement_type_hash: 0xfedcba98,
                city: ComponentId {
                    owner: -1,
                    component_type: -7,
                    id: 0x87654321
                }
            }
        );
        assert_eq!(
            reference.improvement_owner_input(),
            Some(ImprovementOwnerInput::CurrentPlotOwnerRequired)
        );
        assert_eq!(
            v.objects[2],
            ObjectPoolEntry {
                saved_instance_id: -1,
                object: ModifierObject::Null { presence_raw: 0 }
            }
        );
        assert_eq!(v.objects[3].saved_instance_id, i32::MIN);
        assert!(matches!(
            &v.objects[3].object,
            ModifierObject::Present {
                reference: LogicalReference::Team {
                    wire_version: 1,
                    team_ordinal: 63
                },
                ..
            }
        ));
        assert!(matches!(
            &v.objects[4].object,
            ModifierObject::Present {
                reference: LogicalReference::City {
                    wire_version: 2,
                    player_ordinal: -1,
                    city_id: u32::MAX,
                    subtype_raw: 2,
                    sub_id_raw: 3
                },
                ..
            }
        ));
        let ModifierObject::Present {
            reference:
                LogicalReference::TradeRoute {
                    origin_city,
                    destination_city,
                    ..
                },
            ..
        } = &v.objects[5].object
        else {
            panic!()
        };
        assert_eq!(
            *origin_city,
            ComponentId {
                owner: 15,
                component_type: 3,
                id: 0xabcdef01
            }
        );
        assert_eq!(
            *destination_city,
            ComponentId {
                owner: -17,
                component_type: -18,
                id: 0x888999ab
            }
        );
        assert_eq!(
            v.requirements.records,
            vec![
                RequirementRecord {
                    requirement_id: vec![0, 255, 128],
                    context_saved_id: 337,
                    subject_saved_id: -1,
                    state_raw: 255
                },
                RequirementRecord {
                    requirement_id: vec![],
                    context_saved_id: i32::MIN,
                    subject_saved_id: 337,
                    state_raw: 2
                }
            ]
        );
    }
    #[test]
    fn cuts_limits_independent_tags_unknown_guid_and_exact_prefix() {
        let b = fixture();
        for cut in 0..b.len() {
            assert!(
                ThroughRequirements::decode_prefix_with_items(&b[..cut], Limits::default())
                    .is_err(),
                "cut {cut}"
            );
        }
        for cap in 0..b.len() {
            assert_eq!(
                ThroughRequirements::decode_prefix_with_items(
                    &b,
                    Limits {
                        max_bytes: cap,
                        ..Limits::default()
                    }
                ),
                Err(Error::ResourceLimit)
            );
        }
        for cap in 0..13 {
            assert_eq!(
                ThroughRequirements::decode_prefix_with_items(
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
            (6, 4),
            (16, 1),
            (54, 1),
            (101, 1),
            (131, 1),
            (173, 1),
            (210, 3),
        ] {
            let mut bad = b.clone();
            bad[at] = 99;
            assert_eq!(
                ThroughRequirements::decode_prefix_with_items(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                }),
                "at {at}"
            );
        }
        for (at, expected) in [(33, 2), (71, 2), (118, 1), (148, 2), (190, 1)] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                ThroughRequirements::decode_prefix_with_items(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                }),
                "at {at}"
            );
        }
        let mut bad = b.clone();
        bad[17..33].fill(0);
        assert_eq!(
            ThroughRequirements::decode_prefix_with_items(&bad, Limits::default()),
            Err(Error::UnsupportedGuid([0; 16]))
        );
        for at in [0, 7, 211, 215] {
            let mut bad = b.clone();
            bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                ThroughRequirements::decode_prefix_with_items(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
        let mut tail = b.clone();
        tail.extend([99, 98, 97]);
        assert_eq!(
            ThroughRequirements::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            b.len()
        );
    }
    #[test]
    fn full_component_composes_shared_budget_and_independent_guard_strings() {
        let mut b = Vec::new();
        words(&mut b, &[1]);
        b.extend([b'O', 4]);
        words(&mut b, &[0]);
        b.push(3);
        words(&mut b, &[0]);
        b.push(5);
        words(&mut b, &[u32::MAX, 0, 0, 2]);
        b.extend([0, 255]);
        assert_eq!(b.len(), 34);
        let (v, n, i) = Effects::decode_prefix_with_items(&b, Limits::default()).unwrap();
        assert_eq!((n, i), (34, 3));
        assert_eq!(v.prefix.opening_guard, b"O");
        assert_eq!(v.closing_guard, vec![0, 255]);
        assert_eq!(v.modifiers.next_modifier_id, -1);
        for cut in 0..b.len() {
            assert!(Effects::decode(&b[..cut], Limits::default()).is_err());
        }
        for cap in 0..b.len() {
            assert!(Effects::decode(
                &b,
                Limits {
                    max_bytes: cap,
                    ..Limits::default()
                }
            )
            .is_err());
        }
        for cap in 0..3 {
            assert!(Effects::decode(
                &b,
                Limits {
                    max_items: cap,
                    ..Limits::default()
                }
            )
            .is_err());
        }
        assert!(Effects::decode(
            &b,
            Limits {
                max_bytes: 34,
                max_items: 3
            }
        )
        .is_ok());
        let mut tail = b.clone();
        tail.push(9);
        assert_eq!(
            Effects::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        assert_eq!(
            Effects::decode_prefix_with_items(&tail, Limits::default())
                .unwrap()
                .1,
            b.len()
        );
    }
}
