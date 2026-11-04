//! Linux XP2 AI.Targets version3/4 saved wire and bounded knowledge records.
//! No native lookups, archive changes, analyzers, callbacks, decisions or RNG run.
use crate::unit::Limits;
pub const VERSION: u32 = 4;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidShape,
    UnsupportedLegacyWrite,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Targets wire: {self:?}")
    }
}
impl std::error::Error for Error {}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let raw = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(raw.try_into().unwrap())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
}
struct Writer {
    data: Option<Vec<u8>>,
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn put(&mut self, raw: &[u8]) -> Result<(), Error> {
        self.pos = self
            .pos
            .checked_add(raw.len())
            .ok_or(Error::ResourceLimit)?;
        if self.pos > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if let Some(out) = &mut self.data {
            out.extend_from_slice(raw)
        }
        Ok(())
    }
}
macro_rules! scalar {
    ($t:ty,$n:expr) => {
        impl Wire for $t {
            const MIN: usize = $n;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_le_bytes(r.take()?))
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
        }
    };
}
scalar!(u32, 4);
scalar!(i32, 4);
/// Saved native bool object byte; no normalization or truth-value claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoolByte(pub u8);
impl Wire for BoolByte {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self(r.take::<1>()?[0]))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[self.0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.items = r.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        let end = n
            .checked_mul(T::MIN)
            .and_then(|n| r.pos.checked_add(n))
            .ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items || end > r.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > r.data.len() {
            return Err(Error::Truncated);
        }
        n.checked_mul(std::mem::size_of::<T>())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut v = Self::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let n = u32::try_from(self.len()).map_err(|_| Error::ResourceLimit)?;
        w.items = w
            .items
            .checked_add(self.len())
            .ok_or(Error::ResourceLimit)?;
        if w.items > w.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        n.write(w)?;
        for v in self {
            v.write(w)?
        }
        Ok(())
    }
}
/// Definition hash and value, preserving saved order, duplicates and sentinels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashEntry<T> {
    pub hash: u32,
    pub value: T,
}
impl<T: Wire> Wire for HashEntry<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.hash.write(w)?;
        self.value.write(w)
    }
}
macro_rules! record {
 ($name:ident,$version:expr,{$($field:ident:$ty:ty),* $(,)?})=>{
 #[derive(Clone,Debug,Eq,PartialEq)] pub struct $name {$(pub $field:$ty),*}
 impl Wire for $name {const MIN:usize=4$(+<$ty as Wire>::MIN)*;
 fn read(r:&mut Reader<'_>)->Result<Self,Error>{r.version($version)?;Ok(Self{$($field:<$ty as Wire>::read(r)?),*})}
 fn write(&self,w:&mut Writer)->Result<(),Error>{($version as u32).write(w)?;$(self.$field.write(w)?;)*Ok(())}}
 };
}
scalar!(i16, 2);
/// Unsigned map key and typed value, retaining exact serialized traversal order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapEntry<T> {
    pub key: u32,
    pub value: T,
}
impl<T: Wire> Wire for MapEntry<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            key: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.key.write(w)?;
        self.value.write(w)
    }
}
/// Raw presence byte and its conditionally serialized value. Nonzero bytes are preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conditional<T> {
    pub present: BoolByte,
    pub value: Option<T>,
}
impl<T: Wire> Wire for Conditional<T> {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let present = BoolByte::read(r)?;
        let value = if present.0 != 0 {
            Some(T::read(r)?)
        } else {
            None
        };
        Ok(Self { present, value })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if (self.present.0 != 0) != self.value.is_some() {
            return Err(Error::InvalidShape);
        }
        self.present.write(w)?;
        if let Some(v) = &self.value {
            v.write(w)?
        }
        Ok(())
    }
}
/// Current CivKnowledge writes exactly 64 counted player army records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Armies(pub [ArmyKnowledge; 64]);
impl Wire for Armies {
    const MIN: usize = 4 + 64 * 5;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)?;
        if n != 64 {
            return Err(Error::InvalidShape);
        }
        r.items = r.items.checked_add(64).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let mut v = Vec::new();
        v.try_reserve_exact(64).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..64 {
            v.push(ArmyKnowledge::read(r)?)
        }
        Ok(Self(v.try_into().map_err(|_| Error::InvalidShape)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.items = w.items.checked_add(64).ok_or(Error::ResourceLimit)?;
        if w.items > w.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        64u32.write(w)?;
        for v in &self.0 {
            v.write(w)?
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlotCoord {
    pub x: i32,
    pub y: i32,
}
impl Wire for PlotCoord {
    const MIN: usize = <i32 as Wire>::MIN + <i32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            x: <i32 as Wire>::read(r)?,
            y: <i32 as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.x.write(w)?;
        self.y.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentId {
    pub owner: i16,
    pub component_type: i16,
    pub id: u32,
}
impl Wire for ComponentId {
    const MIN: usize = <i16 as Wire>::MIN + <i16 as Wire>::MIN + <u32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            owner: <i16 as Wire>::read(r)?,
            component_type: <i16 as Wire>::read(r)?,
            id: <u32 as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.owner.write(w)?;
        self.component_type.write(w)?;
        self.id.write(w)?;
        Ok(())
    }
}
record!(UnitInfo,2,{unit_id:u32,known_damage:i32,military_formation:i32,location:PlotCoord,area_id:u32,region_id:u32,turns_since_observation:u32});
record!(DistrictDefenses,2,{district_id:ComponentId,district_hash:u32,location:PlotCoord,garrison_defense_q8:i32,outer_defense_q8:i32,attack_strength_q8:i32});
record!(CityKnowledge,7,{owner:i32,id:u32,turns_since_observation:i32,population:i32,location:PlotCoord,region:u32,is_original_capital:BoolByte,is_capital:BoolByte,is_occupied:BoolByte,religion_hash:u32,current_loyalty_q8:i32,loyalty_change_per_turn_q8:i32,next_civ_identity:i32,city_defenses:Vec<DistrictDefenses>,expected_yield:Vec<HashEntry<i32>>,has_buildings:Vec<HashEntry<BoolByte>>,buildings_in_progress:Vec<HashEntry<BoolByte>>,city_districts:Vec<HashEntry<i32>>,great_works:Vec<i32>,owned_resources:Vec<HashEntry<i32>>});
record!(ArmyKnowledge,4,{army_size:Conditional<Vec<HashEntry<Vec<UnitInfo>>>>});
record!(BarbarianKnowledge,1,{camp_location:PlotCoord,turns_since_observation:i32,guard_hash:u32});
record!(PlayerKnowledge,1,{player_id:i32,victory_progress_q8:i32});
record!(WarKnowledge,1,{opponent:i32,goal_score_q8:i32,current_tally_q8:i32,is_active:BoolByte});
record!(TradeKnowledge,1,{owner:i32,from_city:ComponentId,to_city:ComponentId,origin_yield_values_q8:Vec<i32>,destination_yield_values_q8:Vec<i32>,best_yield_destination:i32,best_yield_origin:i32,total_value_q8:i32});
record!(CivKnowledge,7,{owner:i32,known_cities:Vec<CityKnowledge>,barbarians:Vec<BarbarianKnowledge>,player_knowledge:Vec<PlayerKnowledge>,total_hostile_armies_of_type:Vec<HashEntry<i32>>,war_knowledge:Vec<WarKnowledge>,player_armies:Armies,trade_knowledge:TradeKnowledge,yield_average_q8:Vec<HashEntry<i32>>,yield_error_q8:Vec<HashEntry<i32>>});
record!(ExplorationInfo,1,{num_cells:u32,cells_visible:u32,cells_revealed:u32});
record!(AiTarget,3,{target_type:i32,location:PlotCoord,target_id:ComponentId,target_range:i32,target_priority:i32,refresh_location:BoolByte,refresh_priority:BoolByte,is_currently_visible:BoolByte});
record!(TargetList,1,{targets:Vec<AiTarget>});
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Targets {
    pub wire_version: u32,
    pub player_id: i32,
    pub desired_targets: Vec<HashEntry<BoolByte>>,
    pub new_war_started: BoolByte,
    pub knowledge_base: CivKnowledge,
    pub operation_start_points: Vec<PlotCoord>,
    pub release_turn: i32,
    pub map_area_visibility: Vec<MapEntry<ExplorationInfo>>,
    pub map_visibility: Vec<MapEntry<ExplorationInfo>>,
    pub targets: TargetList,
}
impl Wire for Targets {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <Vec<HashEntry<BoolByte>> as Wire>::MIN
        + <CivKnowledge as Wire>::MIN
        + <Vec<PlotCoord> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<MapEntry<ExplorationInfo>> as Wire>::MIN
        + <Vec<MapEntry<ExplorationInfo>> as Wire>::MIN
        + <TargetList as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 3 | 4) {
            return Err(Error::Version {
                expected: 4,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            player_id: <i32 as Wire>::read(r)?,
            desired_targets: <Vec<HashEntry<BoolByte>> as Wire>::read(r)?,
            new_war_started: if wire_version == 4 {
                <BoolByte as Wire>::read(r)?
            } else {
                BoolByte(0)
            },
            knowledge_base: <CivKnowledge as Wire>::read(r)?,
            operation_start_points: <Vec<PlotCoord> as Wire>::read(r)?,
            release_turn: <i32 as Wire>::read(r)?,
            map_area_visibility: <Vec<MapEntry<ExplorationInfo>> as Wire>::read(r)?,
            map_visibility: <Vec<MapEntry<ExplorationInfo>> as Wire>::read(r)?,
            targets: <TargetList as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 4 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        4u32.write(w)?;
        self.player_id.write(w)?;
        self.desired_targets.write(w)?;
        self.new_war_started.write(w)?;
        self.knowledge_base.write(w)?;
        self.operation_start_points.write(w)?;
        self.release_turn.write(w)?;
        self.map_area_visibility.write(w)?;
        self.map_visibility.write(w)?;
        self.targets.write(w)?;
        Ok(())
    }
}
macro_rules! bounded_codec {
    ($name:ident) => {
        impl $name {
            pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
                let (v, n) = Self::decode_prefix(data, limits)?;
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
                let v = Self::read(&mut r)?;
                Ok((v, r.pos, r.items))
            }
            pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
                self.encode_with_items(limits).map(|(v, _)| v)
            }
            pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
                let mut w = Writer {
                    data: None,
                    pos: 0,
                    items: 0,
                    limits,
                };
                self.write(&mut w)?;
                let mut out = Vec::new();
                out.try_reserve_exact(w.pos)
                    .map_err(|_| Error::ResourceLimit)?;
                let items = w.items;
                w = Writer {
                    data: Some(out),
                    pos: 0,
                    items: 0,
                    limits,
                };
                self.write(&mut w)?;
                Ok((w.data.unwrap(), items))
            }
        }
    };
}
bounded_codec!(PlotCoord);
bounded_codec!(ComponentId);
bounded_codec!(UnitInfo);
bounded_codec!(DistrictDefenses);
bounded_codec!(CityKnowledge);
bounded_codec!(ArmyKnowledge);
bounded_codec!(BarbarianKnowledge);
bounded_codec!(PlayerKnowledge);
bounded_codec!(WarKnowledge);
bounded_codec!(TradeKnowledge);
bounded_codec!(CivKnowledge);
bounded_codec!(ExplorationInfo);
bounded_codec!(AiTarget);
bounded_codec!(TargetList);
bounded_codec!(Targets);
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_targets3_default_and_independent_knowledge_tag() {
        fn words(raw: &mut Vec<u8>, xs: &[u32]) {
            for v in xs {
                raw.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut raw = Vec::new();
        words(&mut raw, &[3, u32::MAX, 1, u32::MAX]);
        raw.push(197);
        words(&mut raw, &[7, u32::MAX, 0, 0, 0, 0, 0, 64]);
        for _ in 0..64 {
            words(&mut raw, &[4]);
            raw.push(0);
        }
        // TradeKnowledge1, two raw ComponentIDs, empty yield vectors.
        words(
            &mut raw,
            &[
                1,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                0,
                0,
                0,
                0,
                0,
            ],
        );
        words(&mut raw, &[0, 0, 0, u32::MAX, 0, 0, 1, 0]);
        assert_eq!(raw.len(), 445);
        let cap = Limits {
            max_bytes: 445,
            max_items: 65,
        };
        let (v, n, i) = Targets::decode_prefix_with_items(&raw, cap).unwrap();
        assert_eq!((n, i), (445, 65));
        assert_eq!(v.wire_version, 3);
        assert_eq!(v.new_war_started, BoolByte(0));
        assert_eq!(
            (v.desired_targets[0].hash, v.desired_targets[0].value),
            (u32::MAX, BoolByte(197))
        );
        assert_eq!(v.release_turn, -1);
        assert_eq!(v.encode(cap), Err(Error::UnsupportedLegacyWrite));
        for n in 0..raw.len() {
            assert!(Targets::decode(&raw[..n], cap).is_err());
        }
        for short in [
            Limits {
                max_bytes: 444,
                ..cap
            },
            Limits {
                max_items: 64,
                ..cap
            },
        ] {
            assert_eq!(Targets::decode(&raw, short), Err(Error::ResourceLimit));
        }
        let mut bad = raw.clone();
        bad[17..21].copy_from_slice(&6u32.to_le_bytes());
        assert_eq!(
            Targets::decode(&bad, cap),
            Err(Error::Version {
                expected: 7,
                actual: 6
            })
        );
    }
    #[test]
    fn targets_raw_carriers() {
        for byte in 0..=255 {
            let v = AiTarget {
                target_type: i32::MIN,
                location: PlotCoord {
                    x: i32::MAX,
                    y: i32::MIN,
                },
                target_id: ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MAX,
                    id: u32::MAX,
                },
                target_range: i32::MIN,
                target_priority: i32::MAX,
                refresh_location: BoolByte(byte),
                refresh_priority: BoolByte(byte),
                is_currently_visible: BoolByte(byte),
            };
            let raw = v.encode(Limits::default()).unwrap();
            let limits = Limits {
                max_bytes: raw.len(),
                max_items: 0,
            };
            assert_eq!(AiTarget::decode(&raw, limits), Ok(v.clone()));
            for end in 0..raw.len() {
                assert!(AiTarget::decode(&raw[..end], limits).is_err())
            }
            assert_eq!(
                v.encode(Limits {
                    max_bytes: raw.len() - 1,
                    ..limits
                }),
                Err(Error::ResourceLimit)
            );
        }
    }
    #[test]
    fn targets_hostile() {
        let mut r = Reader {
            data: &[255; 8],
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: 4096,
                max_items: 100,
            },
        };
        assert_eq!(
            Vec::<CityKnowledge>::read(&mut r),
            Err(Error::ResourceLimit)
        );
        for version in [0, 1, 2, 5, u32::MAX] {
            assert!(matches!(
                Targets::decode(&version.to_le_bytes(), Limits::default()),
                Err(Error::Version { .. })
            ));
        }
        let v = ArmyKnowledge {
            army_size: Conditional {
                present: BoolByte(0),
                value: Some(vec![]),
            },
        };
        assert_eq!(v.encode(Limits::default()), Err(Error::InvalidShape));
        for byte in [1, 2, 127, 255] {
            let v = ArmyKnowledge {
                army_size: Conditional {
                    present: BoolByte(byte),
                    value: Some(vec![]),
                },
            };
            let raw = v.encode(Limits::default()).unwrap();
            assert_eq!(ArmyKnowledge::decode(&raw, Limits::default()), Ok(v));
        }
    }
    fn carrier<T: Wire + Clone + Eq + std::fmt::Debug>(v: T) {
        let limits = Limits::default();
        let mut w = Writer {
            data: None,
            pos: 0,
            items: 0,
            limits,
        };
        v.write(&mut w).unwrap();
        let exact = Limits {
            max_bytes: w.pos,
            max_items: w.items,
        };
        let mut w = Writer {
            data: Some(Vec::new()),
            pos: 0,
            items: 0,
            limits: exact,
        };
        v.write(&mut w).unwrap();
        let raw = w.data.unwrap();
        let mut r = Reader {
            data: &raw,
            pos: 0,
            items: 0,
            limits: exact,
        };
        assert_eq!(T::read(&mut r), Ok(v.clone()));
        assert_eq!((r.pos, r.items), (raw.len(), w.items));
        for cut in 0..raw.len() {
            if raw.len() > 4096 && cut > 64 && cut + 64 < raw.len() {
                continue;
            }
            let mut r = Reader {
                data: &raw[..cut],
                pos: 0,
                items: 0,
                limits: exact,
            };
            assert!(T::read(&mut r).is_err());
        }
        for limits in [
            Limits {
                max_bytes: raw.len() - 1,
                ..exact
            },
            Limits {
                max_items: exact.max_items.saturating_sub(1),
                ..exact
            },
        ] {
            if limits.max_items == exact.max_items && limits.max_bytes == exact.max_bytes {
                continue;
            }
            let mut r = Reader {
                data: &raw,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(T::read(&mut r), Err(Error::ResourceLimit));
            let mut w = Writer {
                data: None,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(v.write(&mut w), Err(Error::ResourceLimit));
        }
    }
    #[test]
    fn targets_all_transitive_carriers() {
        carrier(PlotCoord {
            x: i32::MIN,
            y: i32::MIN,
        });
        carrier(ComponentId {
            owner: i16::MIN,
            component_type: i16::MIN,
            id: u32::MAX,
        });
        carrier(UnitInfo {
            unit_id: u32::MAX,
            known_damage: i32::MIN,
            military_formation: i32::MIN,
            location: PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            },
            area_id: u32::MAX,
            region_id: u32::MAX,
            turns_since_observation: u32::MAX,
        });
        carrier(DistrictDefenses {
            district_id: ComponentId {
                owner: i16::MIN,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            district_hash: u32::MAX,
            location: PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            },
            garrison_defense_q8: i32::MIN,
            outer_defense_q8: i32::MIN,
            attack_strength_q8: i32::MIN,
        });
        carrier(CityKnowledge {
            owner: i32::MIN,
            id: u32::MAX,
            turns_since_observation: i32::MIN,
            population: i32::MIN,
            location: PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            },
            region: u32::MAX,
            is_original_capital: BoolByte(254),
            is_capital: BoolByte(254),
            is_occupied: BoolByte(254),
            religion_hash: u32::MAX,
            current_loyalty_q8: i32::MIN,
            loyalty_change_per_turn_q8: i32::MIN,
            next_civ_identity: i32::MIN,
            city_defenses: vec![DistrictDefenses {
                district_id: ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                },
                district_hash: u32::MAX,
                location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MIN,
                },
                garrison_defense_q8: i32::MIN,
                outer_defense_q8: i32::MIN,
                attack_strength_q8: i32::MIN,
            }],
            expected_yield: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
            has_buildings: vec![HashEntry {
                hash: 0x87654321,
                value: BoolByte(254),
            }],
            buildings_in_progress: vec![HashEntry {
                hash: 0x87654321,
                value: BoolByte(254),
            }],
            city_districts: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
            great_works: vec![i32::MIN],
            owned_resources: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
        });
        carrier(ArmyKnowledge {
            army_size: Conditional {
                present: BoolByte(127),
                value: Some(vec![HashEntry {
                    hash: 0x87654321,
                    value: vec![UnitInfo {
                        unit_id: u32::MAX,
                        known_damage: i32::MIN,
                        military_formation: i32::MIN,
                        location: PlotCoord {
                            x: i32::MIN,
                            y: i32::MIN,
                        },
                        area_id: u32::MAX,
                        region_id: u32::MAX,
                        turns_since_observation: u32::MAX,
                    }],
                }]),
            },
        });
        carrier(BarbarianKnowledge {
            camp_location: PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            },
            turns_since_observation: i32::MIN,
            guard_hash: u32::MAX,
        });
        carrier(PlayerKnowledge {
            player_id: i32::MIN,
            victory_progress_q8: i32::MIN,
        });
        carrier(WarKnowledge {
            opponent: i32::MIN,
            goal_score_q8: i32::MIN,
            current_tally_q8: i32::MIN,
            is_active: BoolByte(254),
        });
        carrier(TradeKnowledge {
            owner: i32::MIN,
            from_city: ComponentId {
                owner: i16::MIN,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            to_city: ComponentId {
                owner: i16::MIN,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            origin_yield_values_q8: vec![i32::MIN],
            destination_yield_values_q8: vec![i32::MIN],
            best_yield_destination: i32::MIN,
            best_yield_origin: i32::MIN,
            total_value_q8: i32::MIN,
        });
        carrier(CivKnowledge {
            owner: i32::MIN,
            known_cities: vec![CityKnowledge {
                owner: i32::MIN,
                id: u32::MAX,
                turns_since_observation: i32::MIN,
                population: i32::MIN,
                location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MIN,
                },
                region: u32::MAX,
                is_original_capital: BoolByte(254),
                is_capital: BoolByte(254),
                is_occupied: BoolByte(254),
                religion_hash: u32::MAX,
                current_loyalty_q8: i32::MIN,
                loyalty_change_per_turn_q8: i32::MIN,
                next_civ_identity: i32::MIN,
                city_defenses: vec![DistrictDefenses {
                    district_id: ComponentId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: u32::MAX,
                    },
                    district_hash: u32::MAX,
                    location: PlotCoord {
                        x: i32::MIN,
                        y: i32::MIN,
                    },
                    garrison_defense_q8: i32::MIN,
                    outer_defense_q8: i32::MIN,
                    attack_strength_q8: i32::MIN,
                }],
                expected_yield: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
                has_buildings: vec![HashEntry {
                    hash: 0x87654321,
                    value: BoolByte(254),
                }],
                buildings_in_progress: vec![HashEntry {
                    hash: 0x87654321,
                    value: BoolByte(254),
                }],
                city_districts: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
                great_works: vec![i32::MIN],
                owned_resources: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
            }],
            barbarians: vec![BarbarianKnowledge {
                camp_location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MIN,
                },
                turns_since_observation: i32::MIN,
                guard_hash: u32::MAX,
            }],
            player_knowledge: vec![PlayerKnowledge {
                player_id: i32::MIN,
                victory_progress_q8: i32::MIN,
            }],
            total_hostile_armies_of_type: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
            war_knowledge: vec![WarKnowledge {
                opponent: i32::MIN,
                goal_score_q8: i32::MIN,
                current_tally_q8: i32::MIN,
                is_active: BoolByte(254),
            }],
            player_armies: Armies(std::array::from_fn(|_| ArmyKnowledge {
                army_size: Conditional {
                    present: BoolByte(127),
                    value: Some(vec![HashEntry {
                        hash: 0x87654321,
                        value: vec![UnitInfo {
                            unit_id: u32::MAX,
                            known_damage: i32::MIN,
                            military_formation: i32::MIN,
                            location: PlotCoord {
                                x: i32::MIN,
                                y: i32::MIN,
                            },
                            area_id: u32::MAX,
                            region_id: u32::MAX,
                            turns_since_observation: u32::MAX,
                        }],
                    }]),
                },
            })),
            trade_knowledge: TradeKnowledge {
                owner: i32::MIN,
                from_city: ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                },
                to_city: ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                },
                origin_yield_values_q8: vec![i32::MIN],
                destination_yield_values_q8: vec![i32::MIN],
                best_yield_destination: i32::MIN,
                best_yield_origin: i32::MIN,
                total_value_q8: i32::MIN,
            },
            yield_average_q8: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
            yield_error_q8: vec![HashEntry {
                hash: 0x87654321,
                value: i32::MIN,
            }],
        });
        carrier(ExplorationInfo {
            num_cells: u32::MAX,
            cells_visible: u32::MAX,
            cells_revealed: u32::MAX,
        });
        carrier(AiTarget {
            target_type: i32::MIN,
            location: PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            },
            target_id: ComponentId {
                owner: i16::MIN,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            target_range: i32::MIN,
            target_priority: i32::MIN,
            refresh_location: BoolByte(254),
            refresh_priority: BoolByte(254),
            is_currently_visible: BoolByte(254),
        });
        carrier(TargetList {
            targets: vec![AiTarget {
                target_type: i32::MIN,
                location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MIN,
                },
                target_id: ComponentId {
                    owner: i16::MIN,
                    component_type: i16::MIN,
                    id: u32::MAX,
                },
                target_range: i32::MIN,
                target_priority: i32::MIN,
                refresh_location: BoolByte(254),
                refresh_priority: BoolByte(254),
                is_currently_visible: BoolByte(254),
            }],
        });
        carrier(Targets {
            wire_version: 4,
            player_id: i32::MIN,
            desired_targets: vec![HashEntry {
                hash: 0x87654321,
                value: BoolByte(254),
            }],
            new_war_started: BoolByte(254),
            knowledge_base: CivKnowledge {
                owner: i32::MIN,
                known_cities: vec![CityKnowledge {
                    owner: i32::MIN,
                    id: u32::MAX,
                    turns_since_observation: i32::MIN,
                    population: i32::MIN,
                    location: PlotCoord {
                        x: i32::MIN,
                        y: i32::MIN,
                    },
                    region: u32::MAX,
                    is_original_capital: BoolByte(254),
                    is_capital: BoolByte(254),
                    is_occupied: BoolByte(254),
                    religion_hash: u32::MAX,
                    current_loyalty_q8: i32::MIN,
                    loyalty_change_per_turn_q8: i32::MIN,
                    next_civ_identity: i32::MIN,
                    city_defenses: vec![DistrictDefenses {
                        district_id: ComponentId {
                            owner: i16::MIN,
                            component_type: i16::MIN,
                            id: u32::MAX,
                        },
                        district_hash: u32::MAX,
                        location: PlotCoord {
                            x: i32::MIN,
                            y: i32::MIN,
                        },
                        garrison_defense_q8: i32::MIN,
                        outer_defense_q8: i32::MIN,
                        attack_strength_q8: i32::MIN,
                    }],
                    expected_yield: vec![HashEntry {
                        hash: 0x87654321,
                        value: i32::MIN,
                    }],
                    has_buildings: vec![HashEntry {
                        hash: 0x87654321,
                        value: BoolByte(254),
                    }],
                    buildings_in_progress: vec![HashEntry {
                        hash: 0x87654321,
                        value: BoolByte(254),
                    }],
                    city_districts: vec![HashEntry {
                        hash: 0x87654321,
                        value: i32::MIN,
                    }],
                    great_works: vec![i32::MIN],
                    owned_resources: vec![HashEntry {
                        hash: 0x87654321,
                        value: i32::MIN,
                    }],
                }],
                barbarians: vec![BarbarianKnowledge {
                    camp_location: PlotCoord {
                        x: i32::MIN,
                        y: i32::MIN,
                    },
                    turns_since_observation: i32::MIN,
                    guard_hash: u32::MAX,
                }],
                player_knowledge: vec![PlayerKnowledge {
                    player_id: i32::MIN,
                    victory_progress_q8: i32::MIN,
                }],
                total_hostile_armies_of_type: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
                war_knowledge: vec![WarKnowledge {
                    opponent: i32::MIN,
                    goal_score_q8: i32::MIN,
                    current_tally_q8: i32::MIN,
                    is_active: BoolByte(254),
                }],
                player_armies: Armies(std::array::from_fn(|_| ArmyKnowledge {
                    army_size: Conditional {
                        present: BoolByte(127),
                        value: Some(vec![HashEntry {
                            hash: 0x87654321,
                            value: vec![UnitInfo {
                                unit_id: u32::MAX,
                                known_damage: i32::MIN,
                                military_formation: i32::MIN,
                                location: PlotCoord {
                                    x: i32::MIN,
                                    y: i32::MIN,
                                },
                                area_id: u32::MAX,
                                region_id: u32::MAX,
                                turns_since_observation: u32::MAX,
                            }],
                        }]),
                    },
                })),
                trade_knowledge: TradeKnowledge {
                    owner: i32::MIN,
                    from_city: ComponentId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: u32::MAX,
                    },
                    to_city: ComponentId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: u32::MAX,
                    },
                    origin_yield_values_q8: vec![i32::MIN],
                    destination_yield_values_q8: vec![i32::MIN],
                    best_yield_destination: i32::MIN,
                    best_yield_origin: i32::MIN,
                    total_value_q8: i32::MIN,
                },
                yield_average_q8: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
                yield_error_q8: vec![HashEntry {
                    hash: 0x87654321,
                    value: i32::MIN,
                }],
            },
            operation_start_points: vec![PlotCoord {
                x: i32::MIN,
                y: i32::MIN,
            }],
            release_turn: i32::MIN,
            map_area_visibility: vec![MapEntry {
                key: 0x87654321,
                value: ExplorationInfo {
                    num_cells: u32::MAX,
                    cells_visible: u32::MAX,
                    cells_revealed: u32::MAX,
                },
            }],
            map_visibility: vec![MapEntry {
                key: 0x87654321,
                value: ExplorationInfo {
                    num_cells: u32::MAX,
                    cells_visible: u32::MAX,
                    cells_revealed: u32::MAX,
                },
            }],
            targets: TargetList {
                targets: vec![AiTarget {
                    target_type: i32::MIN,
                    location: PlotCoord {
                        x: i32::MIN,
                        y: i32::MIN,
                    },
                    target_id: ComponentId {
                        owner: i16::MIN,
                        component_type: i16::MIN,
                        id: u32::MAX,
                    },
                    target_range: i32::MIN,
                    target_priority: i32::MIN,
                    refresh_location: BoolByte(254),
                    refresh_priority: BoolByte(254),
                    is_currently_visible: BoolByte(254),
                }],
            },
        });
    }
}
