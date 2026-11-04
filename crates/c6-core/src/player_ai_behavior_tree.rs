//! Current Linux XP2 BehaviorTree.Manager saved wire state, including Blackboard
//! and all 73 built-in NodeData dispatch identities. No policy is executed.
//! Native lookup, callback subscription, archive and load reconstruction effects
//! remain outside this bounded transport. No unknown gameplay state is excluded.
use crate::unit::Limits;
pub const VERSION: u32 = 1;
pub const RECYCLED_VERSION: u32 = 0x10001;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    BlockSize(u32),
    UnsupportedDataTypeCount(u32),
    UnsupportedNodeType(u32),
    NodeFieldShape,
    PresenceShape,
    UnsupportedLegacyWrite,
    MissingRetainedDefinitionHash(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux BehaviorTree wire: {self:?}")
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
scalar!(u8, 1);
/// Raw native bool storage, preserved without logical normalization.
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

record!(ComponentId,0,{owner:i16,component_type:i16,id:u32});
record!(PlotCoord,0,{x:i32,y:i32});
record!(ControlHandle,1,{player_id:i32,id:u32});
/// Narrow native string bytes: count and exact storage, without UTF-8 conversion.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedString(pub Vec<u8>);
impl Wire for SavedString {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self(Vec::<u8>::read(r)?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)
    }
}
/// Conditional native object reference. A nonzero raw presence byte requires a
/// saved identifier; loading this identifier does not resolve a live pointer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Presence<T> {
    pub present: BoolByte,
    pub value: Option<T>,
}
impl<T: Wire> Wire for Presence<T> {
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
            return Err(Error::PresenceShape);
        }
        self.present.write(w)?;
        if let Some(v) = &self.value {
            v.write(w)?;
        }
        Ok(())
    }
}
/// Saved analysis trigger, with definition hashes distinct from native collection indexes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameModifierAnalysis {
    pub wire_version: u32,
    pub belief_hash: u32,
    pub building_hash: u32,
    pub civic_hash: u32,
    pub district_hash: u32,
    /// None at12: native Read defaults the corresponding definition index to-1.
    pub feature_hash: Option<u32>,
    pub handicap_hash: u32,
    pub improvement_hash: u32,
    /// None at12: native Read defaults the corresponding definition index to-1.
    pub minor_civ_type_hash: Option<u32>,
    pub technology_hash: u32,
    pub unit1_hash: u32,
    pub unit2_hash: u32,
    pub yield_hash: u32,
    pub resource_hash: u32,
    pub great_work_object_hash: u32,
    pub policy_hash: u32,
    /// None at12: native Read defaults the corresponding definition index to-1.
    pub governor_hash: Option<u32>,
    pub government_hash: u32,
    pub government_slot_hash: u32,
    pub project_hash: u32,
    pub great_person_class_hash: u32,
    /// None at12: native Read defaults the corresponding definition index to-1.
    pub unit_operation_hash: Option<u32>,
    /// None at12: native Read defaults the corresponding definition index to-1.
    pub unit_promotion_class_hash: Option<u32>,
    pub opponent: i32,
    pub religion: i32,
    pub count: i32,
    pub city_id: ComponentId,
    pub start_turn: i32,
    pub time_horizon: i32,
    pub discount_rate_bits: u32,
    pub expected_bonus_value: i32,
    pub expected_turns_to_meet: i32,
    pub analysis_depth: i32,
    pub unit_list: Vec<u32>,
    pub best_improvement: BoolByte,
    pub collection_found_item: BoolByte,
    pub collection_iterator_index: i32,
    pub city: Presence<ComponentId>,
    pub district: Presence<ComponentId>,
    pub player: Presence<i32>,
    pub plot: Presence<i32>,
    pub unit: Presence<ComponentId>,
}
impl Default for GameModifierAnalysis {
    fn default() -> Self {
        Self {
            wire_version: 17,
            belief_hash: Default::default(),
            building_hash: Default::default(),
            civic_hash: Default::default(),
            district_hash: Default::default(),
            feature_hash: Some(0),
            handicap_hash: Default::default(),
            improvement_hash: Default::default(),
            minor_civ_type_hash: Some(0),
            technology_hash: Default::default(),
            unit1_hash: Default::default(),
            unit2_hash: Default::default(),
            yield_hash: Default::default(),
            resource_hash: Default::default(),
            great_work_object_hash: Default::default(),
            policy_hash: Default::default(),
            governor_hash: Some(0),
            government_hash: Default::default(),
            government_slot_hash: Default::default(),
            project_hash: Default::default(),
            great_person_class_hash: Default::default(),
            unit_operation_hash: Some(0),
            unit_promotion_class_hash: Some(0),
            opponent: Default::default(),
            religion: Default::default(),
            count: Default::default(),
            city_id: Default::default(),
            start_turn: Default::default(),
            time_horizon: Default::default(),
            discount_rate_bits: Default::default(),
            expected_bonus_value: Default::default(),
            expected_turns_to_meet: Default::default(),
            analysis_depth: Default::default(),
            unit_list: Default::default(),
            best_improvement: Default::default(),
            collection_found_item: Default::default(),
            collection_iterator_index: Default::default(),
            city: Default::default(),
            district: Default::default(),
            player: Default::default(),
            plot: Default::default(),
            unit: Default::default(),
        }
    }
}
impl GameModifierAnalysis {
    /// Applies only to the five absent definition fields at version12.
    /// This is a native index default, not serialized hash bits or an executed remap.
    pub fn omitted_definition_default_index(&self) -> Option<i32> {
        (self.wire_version == 12).then_some(-1)
    }
}
impl Wire for GameModifierAnalysis {
    const MIN: usize = 4
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <ComponentId as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<u32> as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Presence<ComponentId> as Wire>::MIN
        + <Presence<ComponentId> as Wire>::MIN
        + <Presence<i32> as Wire>::MIN
        + <Presence<i32> as Wire>::MIN
        + <Presence<ComponentId> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 12 | 17) {
            return Err(Error::Version {
                expected: 17,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            belief_hash: <u32 as Wire>::read(r)?,
            building_hash: <u32 as Wire>::read(r)?,
            civic_hash: <u32 as Wire>::read(r)?,
            district_hash: <u32 as Wire>::read(r)?,
            feature_hash: if wire_version == 17 {
                Some(<u32 as Wire>::read(r)?)
            } else {
                None
            },
            handicap_hash: <u32 as Wire>::read(r)?,
            improvement_hash: <u32 as Wire>::read(r)?,
            minor_civ_type_hash: if wire_version == 17 {
                Some(<u32 as Wire>::read(r)?)
            } else {
                None
            },
            technology_hash: <u32 as Wire>::read(r)?,
            unit1_hash: <u32 as Wire>::read(r)?,
            unit2_hash: <u32 as Wire>::read(r)?,
            yield_hash: <u32 as Wire>::read(r)?,
            resource_hash: <u32 as Wire>::read(r)?,
            great_work_object_hash: <u32 as Wire>::read(r)?,
            policy_hash: <u32 as Wire>::read(r)?,
            governor_hash: if wire_version == 17 {
                Some(<u32 as Wire>::read(r)?)
            } else {
                None
            },
            government_hash: <u32 as Wire>::read(r)?,
            government_slot_hash: <u32 as Wire>::read(r)?,
            project_hash: <u32 as Wire>::read(r)?,
            great_person_class_hash: <u32 as Wire>::read(r)?,
            unit_operation_hash: if wire_version == 17 {
                Some(<u32 as Wire>::read(r)?)
            } else {
                None
            },
            unit_promotion_class_hash: if wire_version == 17 {
                Some(<u32 as Wire>::read(r)?)
            } else {
                None
            },
            opponent: <i32 as Wire>::read(r)?,
            religion: <i32 as Wire>::read(r)?,
            count: <i32 as Wire>::read(r)?,
            city_id: <ComponentId as Wire>::read(r)?,
            start_turn: <i32 as Wire>::read(r)?,
            time_horizon: <i32 as Wire>::read(r)?,
            discount_rate_bits: <u32 as Wire>::read(r)?,
            expected_bonus_value: <i32 as Wire>::read(r)?,
            expected_turns_to_meet: <i32 as Wire>::read(r)?,
            analysis_depth: <i32 as Wire>::read(r)?,
            unit_list: <Vec<u32> as Wire>::read(r)?,
            best_improvement: <BoolByte as Wire>::read(r)?,
            collection_found_item: <BoolByte as Wire>::read(r)?,
            collection_iterator_index: <i32 as Wire>::read(r)?,
            city: <Presence<ComponentId> as Wire>::read(r)?,
            district: <Presence<ComponentId> as Wire>::read(r)?,
            player: <Presence<i32> as Wire>::read(r)?,
            plot: <Presence<i32> as Wire>::read(r)?,
            unit: <Presence<ComponentId> as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 17 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let feature_hash = self
            .feature_hash
            .ok_or(Error::MissingRetainedDefinitionHash("feature_hash"))?;
        let minor_civ_type_hash = self
            .minor_civ_type_hash
            .ok_or(Error::MissingRetainedDefinitionHash("minor_civ_type_hash"))?;
        let governor_hash = self
            .governor_hash
            .ok_or(Error::MissingRetainedDefinitionHash("governor_hash"))?;
        let unit_operation_hash = self
            .unit_operation_hash
            .ok_or(Error::MissingRetainedDefinitionHash("unit_operation_hash"))?;
        let unit_promotion_class_hash =
            self.unit_promotion_class_hash
                .ok_or(Error::MissingRetainedDefinitionHash(
                    "unit_promotion_class_hash",
                ))?;
        17u32.write(w)?;
        self.belief_hash.write(w)?;
        self.building_hash.write(w)?;
        self.civic_hash.write(w)?;
        self.district_hash.write(w)?;
        feature_hash.write(w)?;
        self.handicap_hash.write(w)?;
        self.improvement_hash.write(w)?;
        minor_civ_type_hash.write(w)?;
        self.technology_hash.write(w)?;
        self.unit1_hash.write(w)?;
        self.unit2_hash.write(w)?;
        self.yield_hash.write(w)?;
        self.resource_hash.write(w)?;
        self.great_work_object_hash.write(w)?;
        self.policy_hash.write(w)?;
        governor_hash.write(w)?;
        self.government_hash.write(w)?;
        self.government_slot_hash.write(w)?;
        self.project_hash.write(w)?;
        self.great_person_class_hash.write(w)?;
        unit_operation_hash.write(w)?;
        unit_promotion_class_hash.write(w)?;
        self.opponent.write(w)?;
        self.religion.write(w)?;
        self.count.write(w)?;
        self.city_id.write(w)?;
        self.start_turn.write(w)?;
        self.time_horizon.write(w)?;
        self.discount_rate_bits.write(w)?;
        self.expected_bonus_value.write(w)?;
        self.expected_turns_to_meet.write(w)?;
        self.analysis_depth.write(w)?;
        self.unit_list.write(w)?;
        self.best_improvement.write(w)?;
        self.collection_found_item.write(w)?;
        self.collection_iterator_index.write(w)?;
        self.city.write(w)?;
        self.district.write(w)?;
        self.player.write(w)?;
        self.plot.write(w)?;
        self.unit.write(w)?;
        Ok(())
    }
}
/// Entry order and duplicate labels remain intact.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DataEntry {
    pub label_hash: u32,
    pub indices: [i32; 12],
}
impl Wire for DataEntry {
    const MIN: usize = 60;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        let label_hash = u32::read(r)?;
        let n = u32::read(r)?;
        if n != 12 {
            return Err(Error::UnsupportedDataTypeCount(n));
        }
        r.items = r.items.checked_add(12).ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(Self {
            label_hash,
            indices: <[i32; 12]>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        1u32.write(w)?;
        self.label_hash.write(w)?;
        12u32.write(w)?;
        w.sequence(&self.indices)
    }
}
record!(StringTable,1,{entries:Vec<DataEntry>});
record!(BehaviorTreePriority,0,{type_hash_or_id:u32,weight:i32,priority:i32});
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlackboardData<T> {
    pub value: T,
    pub next_unused_index: i32,
}
impl<T: Wire> Wire for BlackboardData<T> {
    const MIN: usize = 8 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            value: T::read(r)?,
            next_unused_index: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        1u32.write(w)?;
        self.value.write(w)?;
        self.next_unused_index.write(w)
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlackboardVector<T> {
    pub data: Vec<BlackboardData<T>>,
    pub first_unused_index: i32,
}
impl<T: Wire> Wire for BlackboardVector<T> {
    const MIN: usize = 12;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        Ok(Self {
            data: Vec::<BlackboardData<T>>::read(r)?,
            first_unused_index: i32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        1u32.write(w)?;
        self.data.write(w)?;
        self.first_unused_index.write(w)
    }
}
record!(Blackboard,3,{
 string_table:StringTable,units:BlackboardVector<Vec<BehaviorTreePriority>>,techs:BlackboardVector<Vec<BehaviorTreePriority>>,
 buildings:BlackboardVector<Vec<BehaviorTreePriority>>,ints:BlackboardVector<i32>,bools:BlackboardVector<BoolByte>,
 plots:BlackboardVector<PlotCoord>,strings:BlackboardVector<SavedString>,improvements:BlackboardVector<Vec<BehaviorTreePriority>>,
 ids:BlackboardVector<Vec<BehaviorTreePriority>>,fixed_points_q8:BlackboardVector<i32>,districts:BlackboardVector<Vec<BehaviorTreePriority>>,
 civics:BlackboardVector<Vec<BehaviorTreePriority>>,job_index:i32
});
record!(SharedData,0,{data_type:i32,index:u32});
/// Each value has a statically pinned field binding for its built-in node hash.
/// Shared values retain references; Blackboard owns the actual saved values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeValue {
    Shared(SharedData),
    Word(u32),
    Byte(BoolByte),
    Plot(PlotCoord),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind {
    Shared,
    Word,
    Byte,
    Plot,
}
/// Current NodeData dispatch contracts, in InitializeBehaviorTreeNodeIndices order.
/// The C++ binding text distinguishes enum/hash, scalar and SharedData types even
/// where their transport widths coincide. Unknown hashes are unframed and rejected.
pub const NODE_SCHEMAS:&[(u32,&str,&[FieldKind],&[&str])]=&[
(0x6edc3ca3,"Concurrent",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x642f418e,"Priority",&[],&[]),
(0x540f4a82,"Sequence",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool> const&)"]),
(0x33507bd3,"Add Goal Tech",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte,FieldKind::Word,FieldKind::Word,FieldKind::Word,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::TechTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::TechTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::CivicTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::CivicTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","void Serialization::EnumToStream<GameCore::TechTypes>(Serialization::OStream&, GameCore::TechTypes const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","void Serialization::EnumToStream<GameCore::CivicTypes>(Serialization::OStream&, GameCore::CivicTypes const&)","void Serialization::EnumToStream<GameCore::PlayerTypes>(Serialization::OStream&, GameCore::PlayerTypes const&)"]),
(0xb0d545b6,"Build Unit",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word,FieldKind::Byte,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <unsigned int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<unsigned int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","void Serialization::EnumToStream<GameCore::PlayerTypes>(Serialization::OStream&, GameCore::PlayerTypes const&)"]),
(0x797851c6,"Find Available Unit",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <bool>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)"]),
(0x316d2a0a,"Notify Owner",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)"]),
(0xe51c9860,"Barbarian Spawn Change",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0x56bf9e75,"Run Behavior Tree",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Byte,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x296535ee,"Operation Move",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte,FieldKind::Word,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitAiType> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::UnitAiType> const&)"]),
(0x8659a091,"Decode Trigger",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <unsigned int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<unsigned int> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::TechTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::TechTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::CivicTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::CivicTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)"]),
(0x7ba68056,"Choose City",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0x1391b0a0,"Build Improvement",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","void Serialization::EnumToStream<GameCore::ImprovementTypes>(Serialization::OStream&, GameCore::ImprovementTypes const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0xc5ee92f4,"Build Military Improvement",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Improvement> const&)"]),
(0xeb71123e,"Build Production",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x746d959b,"Move Unit",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)"]),
(0xcf0cb52f,"Operation Attack",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0xcfb7446a,"Not",&[],&[]),
(0x59392bc0,"Purchase Plot",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Byte,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::operator<<(Serialization::OStream&, GameCore::PlayerTypes const&)"]),
(0x966ee597,"Recruit Units",&[FieldKind::Shared,FieldKind::Word,FieldKind::Word,FieldKind::Byte,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Unit> const&)"]),
(0x1707f724,"Operation Attack Units",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0x2c3fcfae,"Choose Improvement",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","void Serialization::EnumToStream<GameCore::UnitTypes>(Serialization::OStream&, GameCore::UnitTypes const&)"]),
(0x2d90c51e,"Barbarian Raid Check",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0x662b9193,"In Difficulty Range",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> const&)","void Serialization::WriteIndexToHash<GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> >(Serialization::OStream&, unsigned int, GameCore::Definition::DefinitionCollectionWithTypeIndex<GameCore::Definition::Difficulty> const&)"]),
(0xd6c09a03,"Settle",&[FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x3029df07,"Check Plot",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::ImprovementTypes>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <bool>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::UnitTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","void Serialization::EnumToStream<GameCore::UnitTypes>(Serialization::OStream&, GameCore::UnitTypes const&)","void Serialization::EnumToStream<GameCore::FeatureTypes>(Serialization::OStream&, GameCore::FeatureTypes const&)"]),
(0xde83cc71,"Clear Plot",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0xcaefe870,"Barbarian Recruit",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0x1f4f7262,"Interval",&[FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0xf326fede,"Operation Defend Units",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0xd5ecce03,"Turn Limiter",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0xa8120f1b,"Operation Change Target",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <FixedPointT<8u> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<FixedPointT<8u> const> const&)","Serialization::OStream& Serialization::operator<< <FixedPointT<8u> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<FixedPointT<8u> const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0xce7fa0df,"Operation Is Ready",&[],&[]),
(0x5d8e0ce0,"Contract Manager Decorator",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x2e6d1c36,"Operation Air Assault",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x41a9698a,"Operation Attack City",&[],&[]),
(0x18c44457,"Make Formation",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)"]),
(0xbd56a18f,"Protect Unit",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)","Serialization::OStream& Serialization::operator<< <FixedPointT<8u> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<FixedPointT<8u> const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0xd663b617,"City Attack",&[FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::operator<<(Serialization::OStream&, GameCore::UnitTypes const&)"]),
(0xc95d7a87,"Naval Choose Target",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::operator<<(Serialization::OStream&, GameCore::DomainTypes const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0x89017f32,"Naval Patrol",&[],&[]),
(0x10e94f6b,"Naval Pillage",&[],&[]),
(0xb99b00b3,"Reserve Plot",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0xe5ecbdf8,"Operation Pillage",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0x64e81b91,"Operation Siege City",&[],&[]),
(0x27831ec6,"Garrison City",&[],&[]),
(0xf5e54411,"Is City Threatened",&[],&[]),
(0xf6646bd9,"Can Alpha City",&[],&[]),
(0x1d780b47,"Railroad Target",&[FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x45dfb3d9,"Is At War",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <bool>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x8339fe50,"Find Unit Targets",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::DistrictTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > >(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<GameCore::BuildingTypes>, Types::LinearAlloc::Default<1u, 0u> > > const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x8286e7de,"Has Valid Target",&[],&[]),
(0x64ff9b77,"Upgrade Units",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0x6beaa2f4,"Has Great Person",&[FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)","void Serialization::EnumToStream<GameCore::GreatPersonClassTypes>(Serialization::OStream&, GameCore::GreatPersonClassTypes const&)"]),
(0xf6138737,"Use Great Person",&[FieldKind::Word],&["void Serialization::EnumToStream<GameCore::GreatPersonClassTypes>(Serialization::OStream&, GameCore::GreatPersonClassTypes const&)"]),
(0x54d32214,"Cancel Contract",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)"]),
(0x250328f3,"Lock Units",&[FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0x886aa38a,"Operation Diplomacy",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)"]),
(0x2a823400,"SPY: Choose Mission",&[FieldKind::Shared,FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)"]),
(0x17dd9a6d,"SPY: Do Operation",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0x52143d21,"SPY: Travel",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x359c381b,"ARCHAEOLOGY: Dig",&[FieldKind::Shared,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <GameCore::PlotCoord const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<GameCore::PlotCoord const> const&)","Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x33831d28,"ARCHAEOLOGY: Donate",&[FieldKind::Word],&["Serialization::OStream& Serialization::OStream::operator<< <unsigned int>(unsigned int const&)"]),
(0x98a31c9,"Operation Naval Escort",&[FieldKind::Shared,FieldKind::Word,FieldKind::Word],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)","Serialization::OStream& Serialization::OStream::operator<< <int>(int const&)"]),
(0x4d39e120,"Has Player Ability",&[FieldKind::Shared,FieldKind::Shared,FieldKind::Shared,FieldKind::Byte],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)","Serialization::OStream& Serialization::OStream::operator<< <bool>(bool const&)"]),
(0xae4e2af2,"Has Inquisition",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int const> const&)"]),
(0xf1112ac8,"Launch Inquisition",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<Types::Vector<GameCore::AI::BehaviorTree::BehaviorTreePriority<unsigned int>, Types::LinearAlloc::Default<1u, 0u> > const> const&)"]),
(0x740165cf,"Will use nukes",&[],&[]),
(0x9d4a6546,"Operation Launch nuke",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <bool const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<bool const> const&)"]),
(0xdfe80f7e,"Is Near Fallout",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <int>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<int> const&)"]),
(0xc12f1e25,"LUA Script Condition",&[FieldKind::Shared],&["Serialization::OStream& Serialization::operator<< <String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const>(Serialization::OStream&, GameCore::AI::BehaviorTree::SharedData<String::BasicT<Platform::StaticHeapAllocator<5, 0>, (String::Encoding)0> const> const&)"]),
(0x654ba399,"Operation Military Formation",&[],&[]),
(0xb1bc26a7,"Use Scripted Command",&[],&[]),
 ];
fn node_schema(hash: u32) -> Result<&'static [FieldKind], Error> {
    NODE_SCHEMAS
        .iter()
        .find(|x| x.0 == hash)
        .map(|x| x.2)
        .ok_or(Error::UnsupportedNodeType(hash))
}
fn node28_omissions(hash: u32) -> &'static [usize] {
    match hash {
        0x296535ee => &[6], // Operation Move
        0xf326fede => &[1], // Operation Defend Units
        0xa8120f1b => &[4], // Operation Change Target
        0xe5ecbdf8 => &[2], // Operation Pillage
        0x9d4a6546 => &[0], // Operation Launch nuke
        _ => &[],
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeDefinition {
    pub shape: i32,
    pub node_hash: u32,
    pub node_data_version: u32,
    /// Serialized values only; version28 omits some current-schema slots.
    /// Native NodeData defaults/reconstruction are not executed.
    pub fields: Vec<NodeValue>,
    pub num_shared_data: u32,
}
impl Default for NodeDefinition {
    fn default() -> Self {
        Self {
            shape: 0,
            node_hash: 0,
            node_data_version: 33,
            fields: Vec::new(),
            num_shared_data: 0,
        }
    }
}
impl NodeDefinition {
    pub fn omitted_current_field_indices(&self) -> &'static [usize] {
        if self.node_data_version == 28 {
            node28_omissions(self.node_hash)
        } else {
            &[]
        }
    }
}
impl Wire for NodeDefinition {
    const MIN: usize = 20;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(3)?;
        let shape = i32::read(r)?;
        let node_hash = u32::read(r)?;
        let schema = node_schema(node_hash)?;
        let node_data_version = u32::read(r)?;
        if !matches!(node_data_version, 28 | 33) {
            return Err(Error::Version {
                expected: 33,
                actual: node_data_version,
            });
        }
        let omissions = if node_data_version == 28 {
            node28_omissions(node_hash)
        } else {
            &[]
        };
        // Dispatch fields are fixed schema members, not counted dynamic items.
        let mut fields = Vec::new();
        fields
            .try_reserve_exact(schema.len())
            .map_err(|_| Error::ResourceLimit)?;
        for (index, k) in schema.iter().enumerate() {
            if omissions.contains(&index) {
                continue;
            }
            fields.push(match k {
                FieldKind::Shared => NodeValue::Shared(SharedData::read(r)?),
                FieldKind::Word => NodeValue::Word(u32::read(r)?),
                FieldKind::Byte => NodeValue::Byte(BoolByte::read(r)?),
                FieldKind::Plot => NodeValue::Plot(PlotCoord::read(r)?),
            });
        }
        Ok(Self {
            shape,
            node_hash,
            node_data_version,
            fields,
            num_shared_data: u32::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.node_data_version != 33 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let schema = node_schema(self.node_hash)?;
        if schema.len() != self.fields.len() {
            return Err(Error::NodeFieldShape);
        }
        3u32.write(w)?;
        self.shape.write(w)?;
        self.node_hash.write(w)?;
        33u32.write(w)?;
        for (k, v) in schema.iter().zip(&self.fields) {
            match (k, v) {
                (FieldKind::Shared, NodeValue::Shared(v)) => v.write(w)?,
                (FieldKind::Word, NodeValue::Word(v)) => v.write(w)?,
                (FieldKind::Byte, NodeValue::Byte(v)) => v.write(w)?,
                (FieldKind::Plot, NodeValue::Plot(v)) => v.write(w)?,
                _ => return Err(Error::NodeFieldShape),
            }
        }
        self.num_shared_data.write(w)
    }
}
record!(DecisionState,1,{first_child:u32,jump:u32,last_child:u32,parent:u32,start_at:u32});
record!(BehaviorTree,5,{
 num_nodes:u32,player_id:i32,control_id:u32,is_active:BoolByte,is_temporary:BoolByte,mark_for_destroy:BoolByte,
 allow_recruiting:BoolByte,actions_taken:BoolByte,name:SavedString,owner_name:SavedString,request_priority:i32,
 recruit_priority:i32,tree_id:u32,operation_id:u32,present_value_q8:i32,trigger:GameModifierAnalysis,
 action_states:Vec<i32>,decision_states:Vec<DecisionState>,definitions:Vec<NodeDefinition>,managed_contracts:Vec<ControlHandle>
});
record!(BehaviorTreeEntry,0,{index:i32,tree:BehaviorTree});
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecycledBehaviorTrees {
    pub capacity: u32,
    pub entries: Vec<BehaviorTreeEntry>,
    pub current_id_bits: u32,
    pub first_free: u32,
    pub last_free: u32,
    pub free_indices: Vec<u32>,
}
impl Wire for RecycledBehaviorTrees {
    const MIN: usize = 32;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(RECYCLED_VERSION)?;
        let block = u32::read(r)?;
        if block != 16 {
            return Err(Error::BlockSize(block));
        }
        let capacity = u32::read(r)?;
        let entries = Vec::<BehaviorTreeEntry>::read(r)?;
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
record!(BehaviorTreeManager,VERSION,{player_id:i32,blackboard:Blackboard,behavior_trees:RecycledBehaviorTrees});
impl BehaviorTreeManager {
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
    fn specimen() -> BehaviorTreeManager {
        let definitions = NODE_SCHEMAS
            .iter()
            .map(|(hash, _, schema, _)| NodeDefinition {
                shape: -7,
                node_hash: *hash,
                node_data_version: 33,
                num_shared_data: u32::MAX,
                fields: schema
                    .iter()
                    .map(|k| match k {
                        FieldKind::Shared => NodeValue::Shared(SharedData {
                            data_type: i32::MIN,
                            index: u32::MAX,
                        }),
                        FieldKind::Word => NodeValue::Word(0xdeadbeef),
                        FieldKind::Byte => NodeValue::Byte(BoolByte(0xfe)),
                        FieldKind::Plot => NodeValue::Plot(PlotCoord {
                            x: i32::MIN,
                            y: i32::MAX,
                        }),
                    })
                    .collect(),
            })
            .collect();
        let mut value = BehaviorTreeManager {
            player_id: -1,
            ..BehaviorTreeManager::default()
        };
        value.blackboard.string_table.entries.push(DataEntry {
            label_hash: u32::MAX,
            indices: [-1; 12],
        });
        value.blackboard.strings.data.push(BlackboardData {
            value: SavedString(vec![0, 255, 128]),
            next_unused_index: -2,
        });
        value.behavior_trees.entries.push(BehaviorTreeEntry {
            index: -17,
            tree: BehaviorTree {
                definitions,
                ..BehaviorTree::default()
            },
        });
        value.behavior_trees.free_indices = vec![u32::MAX, 0, 17];
        value
    }
    fn limits() -> Limits {
        Limits {
            max_bytes: 1 << 20,
            max_items: 1 << 16,
        }
    }
    #[test]
    fn node_data28_serialized_omissions_and_trailer_alignment() {
        fn word(raw: &mut Vec<u8>, v: u32) {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        for (hash, _, schema, _) in NODE_SCHEMAS {
            let omitted = match hash {
                0x296535ee => Some(6),
                0xf326fede => Some(1),
                0xa8120f1b => Some(4),
                0xe5ecbdf8 => Some(2),
                0x9d4a6546 => Some(0),
                _ => None,
            };
            let mut raw = Vec::new();
            for v in [3u32, i32::MIN as u32, *hash, 28] {
                word(&mut raw, v);
            }
            for (i, k) in schema.iter().enumerate() {
                if omitted == Some(i) {
                    continue;
                }
                match k {
                    FieldKind::Shared => {
                        word(&mut raw, i32::MIN as u32 + i as u32);
                        word(&mut raw, 0x80000000 + i as u32);
                    }
                    FieldKind::Word => word(&mut raw, 0xdead0000 + i as u32),
                    FieldKind::Byte => raw.push(197 ^ i as u8),
                    FieldKind::Plot => {
                        word(&mut raw, i32::MIN as u32 + i as u32);
                        word(&mut raw, i32::MAX as u32 - i as u32);
                    }
                }
            }
            word(&mut raw, 0x12345678);
            let used = raw.len();
            word(&mut raw, 0x87654321);
            let cap = Limits {
                max_bytes: used,
                max_items: 0,
            };
            let mut r = Reader {
                data: &raw,
                pos: 0,
                items: 0,
                limits: cap,
            };
            let v = NodeDefinition::read(&mut r).unwrap();
            assert_eq!((r.pos, r.items), (used, 0));
            assert_eq!(v.node_data_version, 28);
            assert_eq!(v.num_shared_data, 0x12345678);
            assert_eq!(
                v.fields.len(),
                schema.len() - usize::from(omitted.is_some())
            );
            assert_eq!(v.omitted_current_field_indices(), omitted.as_slice());
            let mut w = Writer {
                data: Vec::new(),
                bytes: 0,
                items: 0,
                limits: cap,
                measuring: true,
            };
            assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
            if omitted.is_some() {
                for n in 0..used {
                    let mut r = Reader {
                        data: &raw[..n],
                        pos: 0,
                        items: 0,
                        limits: cap,
                    };
                    assert!(NodeDefinition::read(&mut r).is_err());
                }
            }
            let mut r = Reader {
                data: &raw,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: used - 1,
                    ..cap
                },
            };
            assert_eq!(NodeDefinition::read(&mut r), Err(Error::ResourceLimit));
        }
        assert_eq!(NodeDefinition::default().node_data_version, 33);
    }
    #[test]
    fn legacy_analysis12_absent_hashes_are_not_index_bits() {
        fn words(raw: &mut Vec<u8>, xs: &[u32]) {
            for v in xs {
                raw.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut raw = Vec::new();
        words(&mut raw, &[12]);
        for i in 0..17 {
            words(&mut raw, &[0x80000000 + i]);
        }
        words(
            &mut raw,
            &[
                u32::MAX,
                i32::MIN as u32,
                i32::MAX as u32,
                0x0002ffff,
                u32::MAX,
                i32::MIN as u32,
                i32::MAX as u32,
                0x7fc00001,
                (-257i32) as u32,
                u32::MAX,
                i32::MIN as u32,
                2,
                0x80000001,
                u32::MAX,
            ],
        );
        raw.extend_from_slice(&[197, 2]);
        words(&mut raw, &[u32::MAX]);
        raw.push(197);
        words(&mut raw, &[0x0002ffff, u32::MAX]);
        raw.push(0);
        raw.push(2);
        words(&mut raw, &[i32::MIN as u32]);
        raw.extend_from_slice(&[0, 0]);
        assert_eq!(raw.len(), 151);
        let cap = Limits {
            max_bytes: 151,
            max_items: 2,
        };
        let mut r = Reader {
            data: &raw,
            pos: 0,
            items: 0,
            limits: cap,
        };
        let v = GameModifierAnalysis::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (151, 2));
        assert_eq!(v.wire_version, 12);
        assert_eq!(
            [
                v.feature_hash,
                v.minor_civ_type_hash,
                v.governor_hash,
                v.unit_operation_hash,
                v.unit_promotion_class_hash
            ],
            [None; 5]
        );
        assert_eq!(v.omitted_definition_default_index(), Some(-1));
        assert_eq!(v.belief_hash, 0x80000000);
        assert_eq!(v.great_person_class_hash, 0x80000010);
        assert_eq!(v.discount_rate_bits, 0x7fc00001);
        assert_eq!(v.unit_list, [0x80000001, u32::MAX]);
        assert_eq!(v.city.present, BoolByte(197));
        assert_eq!(v.city.value.as_ref().unwrap().id, u32::MAX);
        assert_eq!(v.player.value, Some(i32::MIN));
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits: cap,
            measuring: true,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        for n in 0..raw.len() {
            let mut r = Reader {
                data: &raw[..n],
                pos: 0,
                items: 0,
                limits: cap,
            };
            assert!(GameModifierAnalysis::read(&mut r).is_err());
        }
        for limits in [
            Limits {
                max_bytes: 150,
                ..cap
            },
            Limits {
                max_items: 1,
                ..cap
            },
        ] {
            let mut r = Reader {
                data: &raw,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(
                GameModifierAnalysis::read(&mut r),
                Err(Error::ResourceLimit)
            );
        }
        let mut current = GameModifierAnalysis::default();
        assert_eq!(current.wire_version, 17);
        assert_eq!(current.omitted_definition_default_index(), None);
        current.feature_hash = None;
        assert_eq!(
            current.write(&mut w),
            Err(Error::MissingRetainedDefinitionHash("feature_hash"))
        );
    }
    #[test]
    fn all_current_dispatches_and_boundaries() {
        let value = specimen();
        let (wire, items) = value.encode_with_items(limits()).unwrap();
        let exact = Limits {
            max_bytes: wire.len(),
            max_items: items,
        };
        assert_eq!(
            BehaviorTreeManager::decode_prefix_with_items(&wire, exact).unwrap(),
            (value.clone(), wire.len(), items)
        );
        for n in 0..wire.len() {
            assert!(
                BehaviorTreeManager::decode(&wire[..n], exact).is_err(),
                "cut {n}"
            );
        }
        for cap in [
            Limits {
                max_bytes: wire.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(value.encode(cap), Err(Error::ResourceLimit));
            assert_eq!(
                BehaviorTreeManager::decode(&wire, cap),
                Err(Error::ResourceLimit)
            );
        }
        let mut suffix = wire.clone();
        suffix.push(1);
        assert_eq!(
            BehaviorTreeManager::decode(&suffix, exact),
            Err(Error::Trailing)
        );
        assert_eq!(
            BehaviorTreeManager::decode_prefix(&suffix, exact).unwrap(),
            (value, wire.len())
        );
    }
    #[test]
    fn raw_bool_string_float_and_presence() {
        for byte in 0..=255 {
            let mut v = BehaviorTreeManager::default();
            v.blackboard.bools.data.push(BlackboardData {
                value: BoolByte(byte),
                next_unused_index: -1,
            });
            v.behavior_trees.entries.push(BehaviorTreeEntry {
                index: 0,
                tree: BehaviorTree {
                    is_active: BoolByte(byte),
                    trigger: GameModifierAnalysis {
                        discount_rate_bits: 0x7fa00001,
                        city: Presence {
                            present: BoolByte(byte),
                            value: if byte == 0 {
                                None
                            } else {
                                Some(ComponentId {
                                    owner: -1,
                                    component_type: -2,
                                    id: u32::MAX,
                                })
                            },
                        },
                        ..GameModifierAnalysis::default()
                    },
                    ..BehaviorTree::default()
                },
            });
            let wire = v.encode(limits()).unwrap();
            assert_eq!(BehaviorTreeManager::decode(&wire, limits()).unwrap(), v);
        }
        for mask in 0u8..32 {
            let component = ComponentId {
                owner: i16::MIN,
                component_type: i16::MAX,
                id: u32::MAX,
            };
            let reference = |bit: u32, value: ComponentId| Presence {
                present: BoolByte(if mask & (1u8 << bit) == 0u8 { 0 } else { 0x80 }),
                value: if mask & (1u8 << bit) == 0u8 {
                    None
                } else {
                    Some(value)
                },
            };
            let trigger = GameModifierAnalysis {
                city: reference(0, component.clone()),
                district: reference(1, component.clone()),
                unit: reference(4, component),
                player: Presence {
                    present: BoolByte(mask & 4),
                    value: if mask & 4 == 0 { None } else { Some(i32::MIN) },
                },
                plot: Presence {
                    present: BoolByte(mask & 8),
                    value: if mask & 8 == 0 { None } else { Some(i32::MAX) },
                },
                ..GameModifierAnalysis::default()
            };
            let mut v = BehaviorTreeManager::default();
            v.behavior_trees.entries.push(BehaviorTreeEntry {
                index: 0,
                tree: BehaviorTree {
                    trigger,
                    ..BehaviorTree::default()
                },
            });
            let wire = v.encode(limits()).unwrap();
            assert_eq!(BehaviorTreeManager::decode(&wire, limits()).unwrap(), v);
        }
    }
    #[test]
    fn unsupported_unframed_nodes_and_shapes() {
        let mut v = specimen();
        v.behavior_trees.entries[0].tree.definitions[0].node_hash = 0;
        assert_eq!(v.encode(limits()), Err(Error::UnsupportedNodeType(0)));
        v.behavior_trees.entries[0].tree.definitions[0].node_hash = NODE_SCHEMAS[0].0;
        v.behavior_trees.entries[0].tree.definitions[0]
            .fields
            .clear();
        assert_eq!(v.encode(limits()), Err(Error::NodeFieldShape));
        let mut v = BehaviorTreeManager::default();
        v.behavior_trees.entries.push(BehaviorTreeEntry {
            index: 0,
            tree: BehaviorTree::default(),
        });
        v.behavior_trees.entries[0].tree.trigger.player = Presence {
            present: BoolByte(2),
            value: None,
        };
        assert_eq!(v.encode(limits()), Err(Error::PresenceShape));
    }
    #[test]
    fn hostile_counts_and_versions() {
        let mut raw = BehaviorTreeManager::default().encode(limits()).unwrap();
        raw[..4].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            BehaviorTreeManager::decode(&raw, limits()),
            Err(Error::Version {
                expected: 1,
                actual: 2
            })
        );
        raw[..4].copy_from_slice(&1u32.to_le_bytes());
        raw[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            BehaviorTreeManager::decode(&raw, limits()),
            Err(Error::ResourceLimit)
        );
    }
}
