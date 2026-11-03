//! Exact current Linux XP2 AI.GrandStrategic saved wire and all transitive current records.
//! No native lookups, archive changes, analyzers, callbacks, decisions or RNG run.
use crate::unit::Limits;
pub const VERSION: u32 = 12;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.GrandStrategic wire: {self:?}")
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
scalar!(u8, 1);
scalar!(u32, 4);
scalar!(i32, 4);
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
record!(CityPlanner,2, {player_id:i32,expansion_score:i32,turns_since_city_founding:i32});
record!(CitySettleSiteEvaluation,1, {x:i32,y:i32,score_q8:i32});
record!(PlotEvaluation,5, {x:i32,y:i32,my_nearest_city:i32,nearest_enemy_city:i32,danger:i32,cultural_pressure:i32,is_coastal_raw:u8,has_fresh_water_raw:u8,is_valid_raw:u8,new_resources_in_inner_ring:i32,plot_score_q8:i32,inner_ring_yield:Vec<HashEntry<i32>>,full_city_yield:Vec<HashEntry<i32>>,num_terrains:Vec<HashEntry<i32>>,num_resources_inner_ring:Vec<HashEntry<i32>>,num_resources:Vec<HashEntry<i32>>,num_features:Vec<HashEntry<i32>>});
record!(RegionEvaluator,1, {player_id:i32,region_id:u32,evaluations:Vec<CitySettleSiteEvaluation>,intermediate_data:PlotEvaluation,has_positive_eval_raw:u8});
record!(VictoryStrategy,4, {player_id:i32,pursuing_strategies:Vec<HashEntry<u8>>,forbidden_strategies:Vec<HashEntry<u8>>,must_change_state:Vec<HashEntry<u8>>,meets_exclusive_condition:Vec<HashEntry<u8>>,next_turn_change_allowed:Vec<HashEntry<i32>>,yield_priorities_q8:Vec<HashEntry<i32>>,pseudo_yield_priorities_saved_as_yield_hash_q8:Vec<HashEntry<i32>>});
record!(UnitEfficiencyInfo,4, {unit_hash:u32,max_damage:i32,is_obsolete_raw:u8,is_ranged_raw:u8,is_naval_raw:u8,has_zone_of_control_raw:u8,civ_unique_raw:u8,exclude_from_efficiency_calculation_raw:u8,can_build_raw:u8,speed:i32,range:i32,main_combat_strength:i32,defense:i32,num_in_use:i32,efficiency_against_all_enemies_q8:i32,efficiency_against_everyone_q8:i32,efficiency_against_player_q8:Vec<i32>,raw_efficiency_q8:i32,maintenance_resource_hash:u32,num_resource_needed:i32});
record!(UnitProduction,1, {player_id:i32,offense_units:Vec<UnitEfficiencyInfo>,has_targets_raw:u8});
record!(GrandStrategic,12, {player_id:i32,city_planner:CityPlanner,primary_regions:Vec<u32>,expansion_regions:Vec<u32>,tertiary_regions:Vec<u32>,settle_cities:Vec<RegionEvaluator>,done_island_check_raw:u8,is_on_island_raw:u8,commemorate_era_raw:u8,victory_manager:VictoryStrategy,pseudo_yield_priority_boost_q8:Vec<HashEntry<i32>>,pseudo_yield_adjustments_q8:Vec<HashEntry<i32>>,unit_production:UnitProduction,yield_sensitivity_boost_q8:Vec<HashEntry<i32>>});
impl GrandStrategic {
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

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> GrandStrategic {
        GrandStrategic {
            player_id: i32::MIN,
            city_planner: CityPlanner {
                player_id: i32::MIN,
                expansion_score: i32::MIN,
                turns_since_city_founding: i32::MIN,
            },
            primary_regions: vec![u32::MAX],
            expansion_regions: vec![u32::MAX],
            tertiary_regions: vec![u32::MAX],
            settle_cities: vec![RegionEvaluator {
                player_id: i32::MIN,
                region_id: u32::MAX,
                evaluations: vec![CitySettleSiteEvaluation {
                    x: i32::MIN,
                    y: i32::MIN,
                    score_q8: i32::MIN,
                }],
                intermediate_data: PlotEvaluation {
                    x: i32::MIN,
                    y: i32::MIN,
                    my_nearest_city: i32::MIN,
                    nearest_enemy_city: i32::MIN,
                    danger: i32::MIN,
                    cultural_pressure: i32::MIN,
                    is_coastal_raw: 254u8,
                    has_fresh_water_raw: 254u8,
                    is_valid_raw: 254u8,
                    new_resources_in_inner_ring: i32::MIN,
                    plot_score_q8: i32::MIN,
                    inner_ring_yield: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                    full_city_yield: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                    num_terrains: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                    num_resources_inner_ring: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                    num_resources: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                    num_features: vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }],
                },
                has_positive_eval_raw: 254u8,
            }],
            done_island_check_raw: 254u8,
            is_on_island_raw: 254u8,
            commemorate_era_raw: 254u8,
            victory_manager: VictoryStrategy {
                player_id: i32::MIN,
                pursuing_strategies: vec![HashEntry {
                    hash: u32::MAX,
                    value: 254u8,
                }],
                forbidden_strategies: vec![HashEntry {
                    hash: u32::MAX,
                    value: 254u8,
                }],
                must_change_state: vec![HashEntry {
                    hash: u32::MAX,
                    value: 254u8,
                }],
                meets_exclusive_condition: vec![HashEntry {
                    hash: u32::MAX,
                    value: 254u8,
                }],
                next_turn_change_allowed: vec![HashEntry {
                    hash: u32::MAX,
                    value: i32::MIN,
                }],
                yield_priorities_q8: vec![HashEntry {
                    hash: u32::MAX,
                    value: i32::MIN,
                }],
                pseudo_yield_priorities_saved_as_yield_hash_q8: vec![HashEntry {
                    hash: u32::MAX,
                    value: i32::MIN,
                }],
            },
            pseudo_yield_priority_boost_q8: vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }],
            pseudo_yield_adjustments_q8: vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }],
            unit_production: UnitProduction {
                player_id: i32::MIN,
                offense_units: vec![UnitEfficiencyInfo {
                    unit_hash: u32::MAX,
                    max_damage: i32::MIN,
                    is_obsolete_raw: 254u8,
                    is_ranged_raw: 254u8,
                    is_naval_raw: 254u8,
                    has_zone_of_control_raw: 254u8,
                    civ_unique_raw: 254u8,
                    exclude_from_efficiency_calculation_raw: 254u8,
                    can_build_raw: 254u8,
                    speed: i32::MIN,
                    range: i32::MIN,
                    main_combat_strength: i32::MIN,
                    defense: i32::MIN,
                    num_in_use: i32::MIN,
                    efficiency_against_all_enemies_q8: i32::MIN,
                    efficiency_against_everyone_q8: i32::MIN,
                    efficiency_against_player_q8: vec![i32::MIN],
                    raw_efficiency_q8: i32::MIN,
                    maintenance_resource_hash: u32::MAX,
                    num_resource_needed: i32::MIN,
                }],
                has_targets_raw: 254u8,
            },
            yield_sensitivity_boost_q8: vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }],
        }
    }
    #[test]
    fn nested_boundaries() {
        let v = sample();
        let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(GrandStrategic::decode(&raw, cap), Ok(v.clone()));
        for n in 0..raw.len() {
            assert!(GrandStrategic::decode(&raw[..n], cap).is_err());
        }
        for short in [
            Limits {
                max_bytes: raw.len() - 1,
                ..cap
            },
            Limits {
                max_items: items - 1,
                ..cap
            },
        ] {
            assert_eq!(
                GrandStrategic::decode(&raw, short),
                Err(Error::ResourceLimit)
            );
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.push(99);
        assert_eq!(GrandStrategic::decode(&tail, cap), Err(Error::Trailing));
        assert_eq!(
            GrandStrategic::decode_prefix_with_items(&tail, cap),
            Ok((v, raw.len(), items))
        );
    }
    #[test]
    fn hostile_counts_versions() {
        let raw = sample().encode(Limits::default()).unwrap();
        for (offset, expected) in [
            (0, 12),
            (8, 2),
            (52, 1),
            (68, 1),
            (84, 5),
            (199, 4),
            (303, 1),
            (315, 4),
        ] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&99u32.to_le_bytes());
            assert_eq!(
                GrandStrategic::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected,
                    actual: 99
                })
            );
        }
        for offset in [
            24, 32, 40, 48, 64, 123, 135, 147, 159, 171, 183, 207, 216, 225, 234, 243, 255, 267,
            279, 291, 311, 362, 383,
        ] {
            let mut bad = raw.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                GrandStrategic::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
    }
    #[test]
    fn every_raw_bool_byte() {
        for byte in 0..=255u8 {
            let mut v = sample();
            v.done_island_check_raw = byte;
            v.victory_manager.pursuing_strategies[0].value = byte;
            v.settle_cities[0].intermediate_data.is_valid_raw = byte;
            v.unit_production.offense_units[0].can_build_raw = byte;
            let raw = v.encode(Limits::default()).unwrap();
            assert_eq!(GrandStrategic::decode(&raw, Limits::default()).unwrap(), v);
        }
    }
}
