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
    UnsupportedLegacyWrite,
    MissingRetainedCurrentBaseYields,
    MissingRetainedMaintenanceResourceHash,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitEfficiency {
    pub wire_version: u32,
    pub unit_hash: u32,
    pub max_damage: i32,
    pub is_obsolete: BoolByte,
    pub is_ranged: BoolByte,
    pub is_naval: BoolByte,
    pub has_zone_of_control: BoolByte,
    pub civ_unique: BoolByte,
    pub exclude_from_efficiency_calculation: BoolByte,
    pub can_build: BoolByte,
    pub speed: i32,
    pub range: i32,
    pub main_combat_strength: i32,
    pub defense: i32,
    pub num_in_use: i32,
    pub efficiency_against_all_enemies: i32,
    pub efficiency_against_everyone: i32,
    pub efficiency_against_player: Vec<i32>,
    pub raw_efficiency: i32,
    /// None at version 3: no saved hash; native Read assigns index-1.
    pub maintenance_resource_hash: Option<u32>,
    pub num_resource_needed: i32,
}
impl Default for UnitEfficiency {
    fn default() -> Self {
        Self {
            wire_version: 4,
            unit_hash: Default::default(),
            max_damage: Default::default(),
            is_obsolete: Default::default(),
            is_ranged: Default::default(),
            is_naval: Default::default(),
            has_zone_of_control: Default::default(),
            civ_unique: Default::default(),
            exclude_from_efficiency_calculation: Default::default(),
            can_build: Default::default(),
            speed: Default::default(),
            range: Default::default(),
            main_combat_strength: Default::default(),
            defense: Default::default(),
            num_in_use: Default::default(),
            efficiency_against_all_enemies: Default::default(),
            efficiency_against_everyone: Default::default(),
            efficiency_against_player: Default::default(),
            raw_efficiency: Default::default(),
            maintenance_resource_hash: Some(0),
            num_resource_needed: Default::default(),
        }
    }
}
impl UnitEfficiency {
    /// Native collection-index default, kept distinct from absent hash bits.
    pub fn maintenance_resource_default_index(&self) -> Option<i32> {
        (self.wire_version == 3).then_some(-1)
    }
}
impl Wire for UnitEfficiency {
    const MIN: usize = 4
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <i32 as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if wire_version != 3 && wire_version != 4 {
            return Err(Error::Version {
                expected: 4,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            unit_hash: <u32 as Wire>::read(r)?,
            max_damage: <i32 as Wire>::read(r)?,
            is_obsolete: <BoolByte as Wire>::read(r)?,
            is_ranged: <BoolByte as Wire>::read(r)?,
            is_naval: <BoolByte as Wire>::read(r)?,
            has_zone_of_control: <BoolByte as Wire>::read(r)?,
            civ_unique: <BoolByte as Wire>::read(r)?,
            exclude_from_efficiency_calculation: <BoolByte as Wire>::read(r)?,
            can_build: <BoolByte as Wire>::read(r)?,
            speed: <i32 as Wire>::read(r)?,
            range: <i32 as Wire>::read(r)?,
            main_combat_strength: <i32 as Wire>::read(r)?,
            defense: <i32 as Wire>::read(r)?,
            num_in_use: <i32 as Wire>::read(r)?,
            efficiency_against_all_enemies: <i32 as Wire>::read(r)?,
            efficiency_against_everyone: <i32 as Wire>::read(r)?,
            efficiency_against_player: <Vec<i32> as Wire>::read(r)?,
            raw_efficiency: <i32 as Wire>::read(r)?,
            maintenance_resource_hash: if wire_version == 3 {
                None
            } else {
                Some(<u32 as Wire>::read(r)?)
            },
            num_resource_needed: if wire_version == 3 {
                0
            } else {
                <i32 as Wire>::read(r)?
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 4 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let resource = self
            .maintenance_resource_hash
            .ok_or(Error::MissingRetainedMaintenanceResourceHash)?;
        4u32.write(w)?;
        self.unit_hash.write(w)?;
        self.max_damage.write(w)?;
        self.is_obsolete.write(w)?;
        self.is_ranged.write(w)?;
        self.is_naval.write(w)?;
        self.has_zone_of_control.write(w)?;
        self.civ_unique.write(w)?;
        self.exclude_from_efficiency_calculation.write(w)?;
        self.can_build.write(w)?;
        self.speed.write(w)?;
        self.range.write(w)?;
        self.main_combat_strength.write(w)?;
        self.defense.write(w)?;
        self.num_in_use.write(w)?;
        self.efficiency_against_all_enemies.write(w)?;
        self.efficiency_against_everyone.write(w)?;
        self.efficiency_against_player.write(w)?;
        self.raw_efficiency.write(w)?;
        resource.write(w)?;
        self.num_resource_needed.write(w)?;
        Ok(())
    }
}

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
/// Saved CityBuild profile only; native Initialize/analyzers are unapplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CityBuild {
    pub wire_version: u32,
    pub city: ComponentId,
    pub city_id: u32,
    pub player_owner: i32,
    pub city_ai_id: u32,
    pub current_yields: Vec<HashValue>,
    /// None at version 15: prior IndexList after native Initialize remains unknown.
    pub current_base_yields: Option<Vec<HashValue>>,
    pub current_gpps: Vec<HashValue>,
    pub best_improvement_hash: u32,
    pub city_threat: i32,
    pub under_attack: BoolByte,
    pub desired_defenders: i32,
    pub defenders_needed: i32,
    pub reservations: Vec<PlotReservation>,
    pub event_trees: Vec<u32>,
    pub managed_working_contract: ControlHandle,
    pub managed_contracts: Vec<ControlHandle>,
    pub done_off_turn: BoolByte,
    pub best_plot_to_clear: [i32; 2],
    pub best_plot_to_buy: [i32; 2],
    pub food_loss_amounts: Vec<i32>,
    pub food_loss_turns: Vec<i32>,
    pub food_loss_populations: Vec<i32>,
    pub city_unit_efficiencies: Vec<UnitEfficiency>,
    pub city_trade: TradeCity,
    pub yield_preferences: Vec<i32>,
    pub plot_evaluations: Vec<PlotEvaluation>,
    pub evaluation_layers: EvaluationLayers,
    pub unit_evaluations: Vec<HashValue>,
    /// No wire pairs at version 15; native defaults apply at actual Unit indexes.
    pub cached_unit_evaluations: Vec<HashValue>,
    pub project_evaluations: Vec<HashValue>,
    pub power_surplus: i32,
    pub default_build_present_value: i32,
    pub unimproved_resources: Vec<HashValue>,
}
// Preserve the existing synthetic current-format transport Default.
impl Default for CityBuild {
    fn default() -> Self {
        Self {
            wire_version: CITY_BUILD_VERSION,
            city: Default::default(),
            city_id: Default::default(),
            player_owner: Default::default(),
            city_ai_id: Default::default(),
            current_yields: Default::default(),
            current_base_yields: Some(Vec::new()),
            current_gpps: Default::default(),
            best_improvement_hash: Default::default(),
            city_threat: Default::default(),
            under_attack: Default::default(),
            desired_defenders: Default::default(),
            defenders_needed: Default::default(),
            reservations: Default::default(),
            event_trees: Default::default(),
            managed_working_contract: Default::default(),
            managed_contracts: Default::default(),
            done_off_turn: Default::default(),
            best_plot_to_clear: Default::default(),
            best_plot_to_buy: Default::default(),
            food_loss_amounts: Default::default(),
            food_loss_turns: Default::default(),
            food_loss_populations: Default::default(),
            city_unit_efficiencies: Default::default(),
            city_trade: Default::default(),
            yield_preferences: Default::default(),
            plot_evaluations: Default::default(),
            evaluation_layers: Default::default(),
            unit_evaluations: Default::default(),
            cached_unit_evaluations: Default::default(),
            project_evaluations: Default::default(),
            power_surplus: Default::default(),
            default_build_present_value: Default::default(),
            unimproved_resources: Default::default(),
        }
    }
}
impl CityBuild {
    /// Unapplied zero default at actual Unit collection indexes, without
    /// inferring dense index order or an empty effective native array.
    pub fn cached_unit_evaluations_default_q8(&self) -> Option<i32> {
        (self.wire_version == 15).then_some(0)
    }
}
impl Wire for CityBuild {
    const MIN: usize = 4
        + <ComponentId as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <Vec<HashValue> as Wire>::MIN
        + <Vec<HashValue> as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<PlotReservation> as Wire>::MIN
        + <Vec<u32> as Wire>::MIN
        + <ControlHandle as Wire>::MIN
        + <Vec<ControlHandle> as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <[i32; 2] as Wire>::MIN
        + <[i32; 2] as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<UnitEfficiency> as Wire>::MIN
        + <TradeCity as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<PlotEvaluation> as Wire>::MIN
        + <EvaluationLayers as Wire>::MIN
        + <Vec<HashValue> as Wire>::MIN
        + <Vec<HashValue> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<HashValue> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if wire_version != 15 && wire_version != CITY_BUILD_VERSION {
            return Err(Error::Version {
                expected: CITY_BUILD_VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            city: <ComponentId as Wire>::read(r)?,
            city_id: <u32 as Wire>::read(r)?,
            player_owner: <i32 as Wire>::read(r)?,
            city_ai_id: <u32 as Wire>::read(r)?,
            current_yields: <Vec<HashValue> as Wire>::read(r)?,
            current_base_yields: if wire_version == 15 {
                None
            } else {
                Some(<Vec<HashValue> as Wire>::read(r)?)
            },
            current_gpps: <Vec<HashValue> as Wire>::read(r)?,
            best_improvement_hash: <u32 as Wire>::read(r)?,
            city_threat: <i32 as Wire>::read(r)?,
            under_attack: <BoolByte as Wire>::read(r)?,
            desired_defenders: <i32 as Wire>::read(r)?,
            defenders_needed: <i32 as Wire>::read(r)?,
            reservations: <Vec<PlotReservation> as Wire>::read(r)?,
            event_trees: <Vec<u32> as Wire>::read(r)?,
            managed_working_contract: <ControlHandle as Wire>::read(r)?,
            managed_contracts: <Vec<ControlHandle> as Wire>::read(r)?,
            done_off_turn: <BoolByte as Wire>::read(r)?,
            best_plot_to_clear: <[i32; 2] as Wire>::read(r)?,
            best_plot_to_buy: <[i32; 2] as Wire>::read(r)?,
            food_loss_amounts: <Vec<i32> as Wire>::read(r)?,
            food_loss_turns: <Vec<i32> as Wire>::read(r)?,
            food_loss_populations: <Vec<i32> as Wire>::read(r)?,
            city_unit_efficiencies: <Vec<UnitEfficiency> as Wire>::read(r)?,
            city_trade: <TradeCity as Wire>::read(r)?,
            yield_preferences: <Vec<i32> as Wire>::read(r)?,
            plot_evaluations: <Vec<PlotEvaluation> as Wire>::read(r)?,
            evaluation_layers: <EvaluationLayers as Wire>::read(r)?,
            unit_evaluations: <Vec<HashValue> as Wire>::read(r)?,
            cached_unit_evaluations: if wire_version == 15 {
                Vec::new()
            } else {
                <Vec<HashValue> as Wire>::read(r)?
            },
            project_evaluations: <Vec<HashValue> as Wire>::read(r)?,
            power_surplus: <i32 as Wire>::read(r)?,
            default_build_present_value: <i32 as Wire>::read(r)?,
            unimproved_resources: <Vec<HashValue> as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != CITY_BUILD_VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let base = self
            .current_base_yields
            .as_ref()
            .ok_or(Error::MissingRetainedCurrentBaseYields)?;
        CITY_BUILD_VERSION.write(w)?;
        self.city.write(w)?;
        self.city_id.write(w)?;
        self.player_owner.write(w)?;
        self.city_ai_id.write(w)?;
        self.current_yields.write(w)?;
        base.write(w)?;
        self.current_gpps.write(w)?;
        self.best_improvement_hash.write(w)?;
        self.city_threat.write(w)?;
        self.under_attack.write(w)?;
        self.desired_defenders.write(w)?;
        self.defenders_needed.write(w)?;
        self.reservations.write(w)?;
        self.event_trees.write(w)?;
        self.managed_working_contract.write(w)?;
        self.managed_contracts.write(w)?;
        self.done_off_turn.write(w)?;
        self.best_plot_to_clear.write(w)?;
        self.best_plot_to_buy.write(w)?;
        self.food_loss_amounts.write(w)?;
        self.food_loss_turns.write(w)?;
        self.food_loss_populations.write(w)?;
        self.city_unit_efficiencies.write(w)?;
        self.city_trade.write(w)?;
        self.yield_preferences.write(w)?;
        self.plot_evaluations.write(w)?;
        self.evaluation_layers.write(w)?;
        self.unit_evaluations.write(w)?;
        self.cached_unit_evaluations.write(w)?;
        self.project_evaluations.write(w)?;
        self.power_surplus.write(w)?;
        self.default_build_present_value.write(w)?;
        self.unimproved_resources.write(w)?;
        Ok(())
    }
}

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
    #[test]
    fn city_build15_keeps_unknown_base_and_unapplied_cache_default() {
        let mut expected = specimen();
        let build = &mut expected.city_ais.entries[0].city_build;
        build.current_base_yields = Some(vec![HashValue {
            hash: 0xfedcba98,
            value: -257,
        }]);
        build.cached_unit_evaluations = vec![
            HashValue {
                hash: 0x80000001,
                value: i32::MIN,
            },
            HashValue {
                hash: 0x80000001,
                value: -1,
            },
        ];
        let (current, items) = expected.encode_with_items(Limits::default()).unwrap();
        let build = &expected.city_ais.entries[0].city_build;
        let base_start = 28 + 24 + 4 + 8 * build.current_yields.len();
        let base_end = base_start + 4 + 8 * build.current_base_yields.as_ref().unwrap().len();
        let child_end = current.len() - (16 + 4 * expected.city_ais.free_indices.len());
        let cache_end = child_end - 16;
        let cache_start = cache_end - (4 + 8 * build.cached_unit_evaluations.len());
        let mut bytes = current[..base_start].to_vec();
        bytes.extend_from_slice(&current[base_end..cache_start]);
        bytes.extend_from_slice(&current[cache_end..]);
        bytes[28..32].copy_from_slice(&15u32.to_le_bytes());
        let build = &mut expected.city_ais.entries[0].city_build;
        build.wire_version = 15;
        build.current_base_yields = None;
        build.cached_unit_evaluations.clear();
        assert_eq!(build.cached_unit_evaluations_default_q8(), Some(0));
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items - 3,
        };
        assert_eq!(
            CityManager::decode_prefix_with_items(&bytes, exact).unwrap(),
            (expected.clone(), bytes.len(), items - 3)
        );
        assert_eq!(expected.encode(exact), Err(Error::UnsupportedLegacyWrite));
        for n in 0..bytes.len() {
            assert!(
                CityManager::decode(&bytes[..n], exact).is_err(),
                "prefix {n}"
            );
        }
        assert_eq!(
            CityManager::decode(
                &bytes,
                Limits {
                    max_items: items - 4,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            CityManager::decode(
                &bytes,
                Limits {
                    max_bytes: bytes.len() - 1,
                    ..exact
                }
            ),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn city_build15_minimum_row_and_current_missing_field_refusal() {
        let value = CityBuild::default();
        let limits = Limits::default();
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: false,
        };
        value.write(&mut w).unwrap();
        let end = w.data.len();
        // Minimal current base/cache tables each occupy one count word.
        let mut bytes = w.data[..28].to_vec();
        bytes.extend_from_slice(&w.data[32..end - 20]);
        bytes.extend_from_slice(&w.data[end - 16..]);
        bytes[..4].copy_from_slice(&15u32.to_le_bytes());
        assert_eq!(bytes.len(), CityBuild::MIN);
        let mut wrapped = 1u32.to_le_bytes().to_vec();
        wrapped.extend_from_slice(&0i32.to_le_bytes());
        wrapped.extend_from_slice(&bytes);
        let mut r = Reader {
            data: &wrapped,
            pos: 0,
            items: 0,
            limits: Limits {
                max_bytes: wrapped.len(),
                max_items: 1,
            },
        };
        let entries = Vec::<CityBuildEntry>::read(&mut r).unwrap();
        assert_eq!(r.pos, wrapped.len());
        assert_eq!(entries[0].city_build.current_base_yields, None);
        let invalid = CityBuild {
            current_base_yields: None,
            ..CityBuild::default()
        };
        assert_eq!(
            invalid.write(&mut w),
            Err(Error::MissingRetainedCurrentBaseYields)
        );
    }
    #[test]
    fn unit_efficiency3_keeps_absent_hash_distinct_from_native_index() {
        let mut value = UnitEfficiency {
            unit_hash: u32::MAX,
            maintenance_resource_hash: Some(0xfedcba98),
            num_resource_needed: -8,
            civ_unique: BoolByte(197),
            raw_efficiency: i32::MIN,
            efficiency_against_player: vec![i32::MIN, -257, i32::MAX],
            ..Default::default()
        };
        let limits = Limits::default();
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: false,
        };
        value.write(&mut w).unwrap();
        w.data.truncate(w.data.len() - 8);
        w.data[..4].copy_from_slice(&3u32.to_le_bytes());
        value.wire_version = 3;
        value.maintenance_resource_hash = None;
        value.num_resource_needed = 0;
        let exact = Limits {
            max_bytes: w.data.len(),
            max_items: 3,
        };
        let mut r = Reader {
            data: &w.data,
            pos: 0,
            items: 0,
            limits: exact,
        };
        assert_eq!(UnitEfficiency::read(&mut r).unwrap(), value);
        assert_eq!((r.pos, r.items), (w.data.len(), 3));
        assert_eq!(value.maintenance_resource_default_index(), Some(-1));
        let mut out = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: false,
        };
        assert_eq!(value.write(&mut out), Err(Error::UnsupportedLegacyWrite));
        for n in 0..w.data.len() {
            assert!(UnitEfficiency::read(&mut Reader {
                data: &w.data[..n],
                pos: 0,
                items: 0,
                limits: exact
            })
            .is_err());
        }
        assert_eq!(
            UnitEfficiency::read(&mut Reader {
                data: &w.data,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_items: 2,
                    ..exact
                }
            }),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            UnitEfficiency::read(&mut Reader {
                data: &w.data,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: w.data.len() - 1,
                    ..exact
                }
            }),
            Err(Error::ResourceLimit)
        );
        w.data[..4].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            UnitEfficiency::read(&mut Reader {
                data: &w.data,
                pos: 0,
                items: 0,
                limits: exact
            }),
            Err(Error::Version {
                expected: 4,
                actual: 2
            })
        );
        let missing = UnitEfficiency {
            maintenance_resource_hash: None,
            ..Default::default()
        };
        assert_eq!(
            missing.write(&mut out),
            Err(Error::MissingRetainedMaintenanceResourceHash)
        );
        let mut minimal = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits,
            measuring: false,
        };
        UnitEfficiency::default().write(&mut minimal).unwrap();
        minimal.data.truncate(minimal.data.len() - 8);
        minimal.data[..4].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(minimal.data.len(), UnitEfficiency::MIN);
        let mut wrapped = 1u32.to_le_bytes().to_vec();
        wrapped.extend_from_slice(&minimal.data);
        assert_eq!(
            Vec::<UnitEfficiency>::read(&mut Reader {
                data: &wrapped,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: wrapped.len(),
                    max_items: 1
                }
            })
            .unwrap()
            .len(),
            1
        );
    }
}
