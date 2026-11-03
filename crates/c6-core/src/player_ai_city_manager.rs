//! Current Linux XP2 AI.CityManager and CityBuild saved wire state.
//!
//! This is bounded transport only. Native player/city lookups, Initialize,
//! signals, archives, analyzer reconstruction, policy and gameplay/RNG effects
//! are not reproduced. Hashes retain their saved bits and order; fixed points
//! retain signed raw storage. Unknown gameplay state is not excluded or masked.
use crate::unit::Limits;

pub const VERSION: u32 = 2;
pub const CITY_BUILD_VERSION: u32 = 17;
pub const RECYCLED_VERSION: u32 = 0x10001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    BlockSize(u32),
    UnsupportedEvaluationLayerCount(i32),
    UnsupportedNeighborCount(u32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.CityManager wire: {self:?}")
    }
}
impl std::error::Error for Error {}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
struct Writer {
    data: Vec<u8>,
    bytes: usize,
    items: usize,
    limits: Limits,
    measuring: bool,
}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let b = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(b.try_into().unwrap())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn reserve<T>(&mut self, n: usize, min: usize) -> Result<Vec<T>, Error> {
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let end = self
            .pos
            .checked_add(n.checked_mul(min).ok_or(Error::ResourceLimit)?)
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        n.checked_mul(std::mem::size_of::<T>())
            .filter(|x| *x <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        Ok(v)
    }
    fn sequence<T: Wire>(&mut self, n: usize) -> Result<Vec<T>, Error> {
        let mut v = self.reserve::<T>(n, T::MIN)?;
        for _ in 0..n {
            v.push(T::read(self)?);
        }
        Ok(v)
    }
}
impl Writer {
    fn put(&mut self, data: &[u8]) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_add(data.len())
            .ok_or(Error::ResourceLimit)?;
        if self.bytes > self.limits.max_bytes || self.bytes > isize::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        if !self.measuring {
            self.data.extend_from_slice(data);
        }
        Ok(())
    }
    fn sequence<T: Wire>(&mut self, v: &[T]) -> Result<(), Error> {
        self.items = self
            .items
            .checked_add(v.len())
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        for x in v {
            x.write(self)?;
        }
        Ok(())
    }
}
macro_rules! scalar {
    ($t:ty, $n:expr) => {
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
scalar!(i16, 2);
/// Saved native bool storage. Noncanonical bytes occur in original PlotEvaluation
/// output; transport preserves them without inventing a gameplay truth value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
        r.sequence(n)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        u32::try_from(self.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        w.sequence(self)
    }
}
impl<T: Wire + Copy + Default, const N: usize> Wire for [T; N] {
    const MIN: usize = T::MIN * N;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let mut v = [T::default(); N];
        for x in &mut v {
            *x = T::read(r)?;
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        for x in self {
            x.write(w)?;
        }
        Ok(())
    }
}
macro_rules! record {
    ($name:ident, $version:expr, {$($field:ident: $ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq)]
        pub struct $name { $(pub $field: $ty),* }
        impl Wire for $name {
            const MIN: usize = if $version == 0 { 0 } else { 4 } $(+ <$ty as Wire>::MIN)*;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                if $version != 0 { r.version($version)?; }
                Ok(Self { $($field: <$ty as Wire>::read(r)?),* })
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                if $version != 0 { ($version as u32).write(w)?; }
                $(self.$field.write(w)?;)* Ok(())
            }
        }
    };
}
// CityID and ComponentID both save owner/type as signed shorts and ID as u32.
record!(ComponentId, 0, { owner: i16, component_type: i16, id: u32 });
record!(HashValue, 0, { hash: u32, value: i32 });
record!(ControlHandle, 1, { player_id: i32, id: u32 });
record!(PlotReservation, 1, { controller: u32, value: i32, tag: u32 });
record!(TradeConnection, 1, { city: ComponentId, rail_connection: BoolByte, rail_value: i32 });
record!(TradeCity, 3, { city: ComponentId, current_routes: i32, current_units: i32,
    connected_cities: Vec<TradeConnection>, known_failed_connections: Vec<ComponentId> });
record!(UnitEfficiency, 4, {
    unit_hash: u32, max_damage: i32, is_obsolete: BoolByte, is_ranged: BoolByte,
    is_naval: BoolByte, has_zone_of_control: BoolByte, civ_unique: BoolByte,
    exclude_from_efficiency_calculation: BoolByte, can_build: BoolByte,
    speed: i32, range: i32, main_combat_strength: i32, defense: i32, num_in_use: i32,
    efficiency_against_all_enemies: i32, efficiency_against_everyone: i32,
    efficiency_against_player: Vec<i32>, raw_efficiency: i32,
    maintenance_resource_hash: u32, num_resource_needed: i32
});
/// Native WriteArray saves a count (exactly six in the current writer).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Neighbors(pub [i16; 6]);
impl Wire for Neighbors {
    const MIN: usize = 16;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)?;
        if n != 6 {
            return Err(Error::UnsupportedNeighborCount(n));
        }
        // Fixed wire array entries still consume the shared item budget.
        r.items = r.items.checked_add(6).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(Self(<[i16; 6]>::read(r)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        6u32.write(w)?;
        w.sequence(&self.0)
    }
}
record!(PlotEvaluation, 1, {
    index: i32, plot: [i32; 2], evaluate_wonders: BoolByte, city: ComponentId,
    resource_hash: u32, feature_hash: u32, location: [i32; 2], neighbors: Neighbors,
    has_improvement: BoolByte, has_district: BoolByte, has_pillaged_improvement: BoolByte,
    feature_removable: BoolByte, can_build_improvement: BoolByte, can_build_district: BoolByte,
    can_place_building: BoolByte, can_purchase: BoolByte, neighbor_can_have_district: BoolByte,
    revealed: BoolByte, owned: BoolByte, worked: BoolByte
});
record!(EvaluationLayer, 0, { buildings: Vec<HashValue>, districts: Vec<HashValue>, improvements: Vec<HashValue> });
/// Count is signed in the original writer; negative forms are unsupported.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvaluationLayers(pub Vec<EvaluationLayer>);
impl Wire for EvaluationLayers {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = i32::read(r)?;
        if n < 0 {
            return Err(Error::UnsupportedEvaluationLayerCount(n));
        }
        Ok(Self(r.sequence(n as usize)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        i32::try_from(self.0.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        w.sequence(&self.0)
    }
}
record!(CityBuild, CITY_BUILD_VERSION, {
    city: ComponentId, city_id: u32, player_owner: i32, city_ai_id: u32,
    current_yields: Vec<HashValue>, current_base_yields: Vec<HashValue>, current_gpps: Vec<HashValue>,
    best_improvement_hash: u32, city_threat: i32, under_attack: BoolByte,
    desired_defenders: i32, defenders_needed: i32,
    reservations: Vec<PlotReservation>, event_trees: Vec<u32>,
    managed_working_contract: ControlHandle, managed_contracts: Vec<ControlHandle>,
    done_off_turn: BoolByte, best_plot_to_clear: [i32; 2], best_plot_to_buy: [i32; 2],
    food_loss_amounts: Vec<i32>, food_loss_turns: Vec<i32>, food_loss_populations: Vec<i32>,
    city_unit_efficiencies: Vec<UnitEfficiency>, city_trade: TradeCity, yield_preferences: Vec<i32>,
    plot_evaluations: Vec<PlotEvaluation>, evaluation_layers: EvaluationLayers,
    unit_evaluations: Vec<HashValue>, cached_unit_evaluations: Vec<HashValue>, project_evaluations: Vec<HashValue>,
    power_surplus: i32, default_build_present_value: i32, unimproved_resources: Vec<HashValue>
});
record!(CityBuildEntry, 0, { index: i32, city_build: CityBuild });
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecycledCityBuilds {
    pub capacity: u32,
    pub entries: Vec<CityBuildEntry>,
    pub current_id_bits: u32,
    pub first_free: u32,
    pub last_free: u32,
    pub free_indices: Vec<u32>,
}
impl Wire for RecycledCityBuilds {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(RECYCLED_VERSION)?;
        let block = u32::read(r)?;
        if block != 16 {
            return Err(Error::BlockSize(block));
        }
        let capacity = u32::read(r)?;
        let entries = Vec::<CityBuildEntry>::read(r)?;
        let current_id_bits = u32::read(r)?;
        let free_count = u32::read(r)? as usize;
        let first_free = u32::read(r)?;
        let last_free = u32::read(r)?;
        let free_indices = r.sequence(free_count)?;
        Ok(Self {
            capacity,
            entries,
            current_id_bits,
            first_free,
            last_free,
            free_indices,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        RECYCLED_VERSION.write(w)?;
        16u32.write(w)?;
        self.capacity.write(w)?;
        self.entries.write(w)?;
        self.current_id_bits.write(w)?;
        u32::try_from(self.free_indices.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        self.first_free.write(w)?;
        self.last_free.write(w)?;
        w.sequence(&self.free_indices)
    }
}
record!(CityManager, VERSION, { player_id: i32, city_ais: RecycledCityBuilds });
impl CityManager {
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
        // Measure every nested byte/item before reserving or producing output.
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: true,
        };
        self.write(&mut w)?;
        w.data
            .try_reserve_exact(w.bytes)
            .map_err(|_| Error::ResourceLimit)?;
        let items = w.items;
        w.bytes = 0;
        w.items = 0;
        w.measuring = false;
        self.write(&mut w)?;
        Ok((w.data, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn specimen() -> CityManager {
        let hash_values = vec![
            HashValue {
                hash: u32::MAX,
                value: i32::MIN,
            },
            HashValue {
                hash: u32::MAX,
                value: -256,
            },
        ];
        let build = CityBuild {
            city: ComponentId {
                owner: -1,
                component_type: i16::MIN,
                id: u32::MAX,
            },
            player_owner: i32::MIN,
            current_yields: hash_values.clone(),
            managed_contracts: vec![ControlHandle {
                player_id: -1,
                id: u32::MAX,
            }],
            city_unit_efficiencies: vec![UnitEfficiency {
                can_build: BoolByte(1),
                efficiency_against_player: vec![i32::MIN, i32::MAX],
                ..Default::default()
            }],
            city_trade: TradeCity {
                connected_cities: vec![TradeConnection {
                    rail_connection: BoolByte(1),
                    rail_value: -1,
                    ..Default::default()
                }],
                known_failed_connections: vec![ComponentId::default()],
                ..Default::default()
            },
            plot_evaluations: vec![PlotEvaluation {
                owned: BoolByte(197),
                neighbors: Neighbors([i16::MIN, -1, 0, 1, 2, i16::MAX]),
                ..Default::default()
            }],
            evaluation_layers: EvaluationLayers(vec![EvaluationLayer {
                buildings: hash_values.clone(),
                districts: hash_values.clone(),
                improvements: hash_values,
            }]),
            reservations: vec![PlotReservation {
                controller: u32::MAX,
                value: i32::MIN,
                tag: 1,
            }],
            food_loss_turns: vec![i32::MIN],
            ..Default::default()
        };
        CityManager {
            player_id: -1,
            city_ais: RecycledCityBuilds {
                capacity: 16,
                entries: vec![CityBuildEntry {
                    index: -1,
                    city_build: build,
                }],
                free_indices: vec![8, 2, 9],
                current_id_bits: u32::MAX,
                first_free: 8,
                last_free: 9,
            },
        }
    }
    #[test]
    fn nonempty_nested_roundtrip_and_shared_limits() {
        let v = specimen();
        let broad = Limits {
            max_bytes: 10000,
            max_items: 10000,
        };
        let (wire, items) = v.encode_with_items(broad).unwrap();
        let exact = Limits {
            max_bytes: wire.len(),
            max_items: items,
        };
        assert_eq!(CityManager::decode(&wire, exact).unwrap(), v);
        for n in 0..wire.len() {
            assert!(
                CityManager::decode(&wire[..n], exact).is_err(),
                "prefix {n}"
            );
        }
        for short in [
            Limits {
                max_bytes: wire.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(CityManager::decode(&wire, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut suffix = wire.clone();
        suffix.extend_from_slice(&[77; 100]);
        assert_eq!(CityManager::decode(&suffix, exact), Err(Error::Trailing));
        assert_eq!(
            CityManager::decode_prefix_with_items(&suffix, exact).unwrap(),
            (v.clone(), wire.len(), items)
        );
        assert_eq!(
            CityManager::decode_prefix(&suffix, exact).unwrap(),
            (v, wire.len())
        );
    }
    #[test]
    fn hostile_counts_and_versions_refused() {
        let limits = Limits {
            max_bytes: 10000,
            max_items: 10000,
        };
        let wire = specimen().encode(limits).unwrap();
        for offset in [0, 8, 28] {
            let mut bad = wire.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(matches!(
                CityManager::decode(&bad, limits),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = wire.clone();
        bad[12..16].copy_from_slice(&8u32.to_le_bytes());
        assert_eq!(CityManager::decode(&bad, limits), Err(Error::BlockSize(8)));
        let mut bad = wire;
        bad[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(CityManager::decode(&bad, limits), Err(Error::ResourceLimit));
        let empty = CityManager::default();
        let (wire, items) = empty.encode_with_items(limits).unwrap();
        assert_eq!((wire.len(), items), (40, 0));
        assert!(CityManager::decode(
            &wire,
            Limits {
                max_bytes: 40,
                max_items: 0
            }
        )
        .is_ok());
    }
    #[test]
    fn nested_versions_signed_counts_and_raw_bool_storage() {
        let limits = Limits {
            max_bytes: 100,
            max_items: 100,
        };
        for expected in [1, 3, 4, 17] {
            let bytes = (expected + 1u32).to_le_bytes();
            let mut r = Reader {
                data: &bytes,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(
                r.version(expected),
                Err(Error::Version {
                    expected,
                    actual: expected + 1
                })
            );
        }
        let bytes = (-1i32).to_le_bytes();
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits,
        };
        assert_eq!(
            EvaluationLayers::read(&mut r),
            Err(Error::UnsupportedEvaluationLayerCount(-1))
        );
        let bytes = 7u32.to_le_bytes();
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits,
        };
        assert_eq!(
            Neighbors::read(&mut r),
            Err(Error::UnsupportedNeighborCount(7))
        );
        for byte in 0..=255 {
            let bytes = [byte];
            let mut r = Reader {
                data: &bytes,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(BoolByte::read(&mut r), Ok(BoolByte(byte)));
        }
        // Saved identity uses signed shorts, never a host pointer or padding.
        let bytes = [255, 255, 0, 128, 255, 255, 255, 255];
        let mut r = Reader {
            data: &bytes,
            pos: 0,
            items: 0,
            limits,
        };
        assert_eq!(
            ComponentId::read(&mut r).unwrap(),
            ComponentId {
                owner: -1,
                component_type: i16::MIN,
                id: u32::MAX
            }
        );
    }
}
