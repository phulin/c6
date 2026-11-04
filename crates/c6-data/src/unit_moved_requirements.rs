//! Source-bound UnitEmbarked and CityHasGarrisonUnit UnitMoved callbacks.
//! Receiver identities/fields are inputs; Test answers are computed live.
pub use crate::plot_adjacent_owner_requirement::{DefinitionFlags, SimpleState};
use crate::{
    formation_rules, loaded_actions::Error as RulesError, registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority, unit_initialization_definitions::collection,
    unit_movement_state, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{LoadedState, MissingInput},
    map::UnitReference,
    simple_requirement::{self, Requirement},
    unit::{ComponentId, PlotCoord, Unit},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Object {
    Unit(ComponentId),
    City(ComponentId),
    NoInterface,
    Unavailable(u64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedUnit {
    None,
    Unit(ComponentId),
    Unavailable(u64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    UnitEmbarked,
    CityHasGarrisonUnit {
        military_formation: i32,
        cached_unit: CachedUnit,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    pub simple: SimpleState,
    pub subject: Object,
    pub kind: Kind,
}
impl Receiver {
    pub fn from_initial(
        active: bool,
        state: i32,
        flags: Option<DefinitionFlags>,
        subject: Object,
        kind: Kind,
    ) -> Self {
        let simple = match flags {
            Some(f) => SimpleState::Defined(Requirement {
                active,
                state,
                inverse: f.inverse,
                persistent: f.persistent,
                triggered: f.triggered,
            }),
            None => SimpleState::DefinitionUnavailable { active, state },
        };
        Self {
            simple,
            subject,
            kind,
        }
    }
    pub fn active(&self) -> bool {
        match self.simple {
            SimpleState::Defined(r) => r.active,
            SimpleState::DefinitionUnavailable { active, .. } => active,
        }
    }
    pub fn state(&self) -> i32 {
        match self.simple {
            SimpleState::Defined(r) => r.state,
            SimpleState::DefinitionUnavailable { state, .. } => state,
        }
    }
}
#[derive(Debug)]
pub enum QueryError {
    Binding,
    DuplicateReceiver(u64),
    MissingReceiver(u64),
    MissingObject(u64),
    MissingDefinition(u64),
    MissingDefinitions,
    WrongKind,
    NullCityInterface,
    NullCity,
    MissingCachedUnit(u64),
    Defender(crate::combat_defender::Error),
    Rules(&'static str),
    Combat(RulesError),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnitMoved requirement: {self:?}")
    }
}
impl std::error::Error for QueryError {}
pub struct Runtime {
    binding: WireBinding,
    receivers: BTreeMap<u64, Receiver>,
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        receivers: Vec<(u64, Receiver)>,
    ) -> Result<Self, QueryError> {
        if state.binding() != binding {
            return Err(QueryError::Binding);
        }
        let mut owned = BTreeMap::new();
        for (id, r) in receivers {
            if owned.insert(id, r).is_some() {
                return Err(QueryError::DuplicateReceiver(id));
            }
        }
        Ok(Self {
            binding: binding.clone(),
            receivers: owned,
        })
    }
    pub fn check(&self, state: &LoadedState) -> Result<(), QueryError> {
        if &self.binding == state.binding() {
            Ok(())
        } else {
            Err(QueryError::Binding)
        }
    }
    pub fn receiver(&self, id: u64) -> Result<&Receiver, QueryError> {
        self.receivers
            .get(&id)
            .ok_or(QueryError::MissingReceiver(id))
    }
    pub fn receiver_mut(&mut self, id: u64) -> Result<&mut Receiver, QueryError> {
        self.receivers
            .get_mut(&id)
            .ok_or(QueryError::MissingReceiver(id))
    }
    pub fn state(&self, id: u64) -> Result<i32, QueryError> {
        Ok(self.receiver(id)?.state())
    }
}

/// Configuration-admitted rules, not captured CanAttack/IsDefensible answers.
pub struct Definitions {
    binding: WireBinding,
    combat: formation_rules::Definitions,
    districts: LinuxDefinitionRegistry,
    district_properties: Vec<(bool, i32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, QueryError> {
        let combat = formation_rules::Definitions::from_rules(state, rules, bound)
            .map_err(QueryError::Combat)?;
        let districts = collection(rules, "Districts", "DistrictType")
            .map_err(|_| QueryError::Rules("Districts"))?;
        let table = rules
            .gameplay()
            .table("Districts")
            .ok_or(QueryError::Rules("Districts"))?;
        let district_properties = districts
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or(QueryError::Rules("District row"))?;
                let internal = match row
                    .integer("InternalOnly")
                    .map_err(|_| QueryError::Rules("InternalOnly"))?
                {
                    0 => false,
                    1 => true,
                    _ => return Err(QueryError::Rules("InternalOnly bool")),
                };
                let hp = match row
                    .get("HitPoints")
                    .map_err(|_| QueryError::Rules("HitPoints"))?
                {
                    crate::SqlValue::Null => 0,
                    crate::SqlValue::Integer(n) => {
                        i32::try_from(*n).map_err(|_| QueryError::Rules("HitPoints i32"))?
                    }
                    _ => return Err(QueryError::Rules("HitPoints integer")),
                };
                Ok((internal, hp))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            combat,
            districts,
            district_properties,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), QueryError> {
        if &self.binding == state.binding() {
            Ok(())
        } else {
            Err(QueryError::Binding)
        }
    }
    fn district(
        &self,
        d: &c6_core::player_districts::District,
    ) -> Result<(bool, bool), QueryError> {
        let Some(index) = self.districts.find_index(d.district_hash) else {
            return Ok((false, false));
        };
        let (internal, hp) = self
            .district_properties
            .get(index)
            .copied()
            .ok_or(QueryError::Rules("district index"))?;
        let defensible = d.district_hash == c6_core::city_district::CITY_CENTER_HASH
            || (hp > 0 && d.is_complete && !d.pillaged);
        Ok((defensible, internal))
    }
}
pub trait Context {
    type Error;
    fn state(&self) -> &LoadedState;
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
    fn definitions(&self) -> Result<&Definitions, QueryError>;
    fn changed(&mut self, receiver: u64, old: i32, new: i32) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Query(QueryError),
    Consumer {
        receiver: u64,
        old: i32,
        new: i32,
        error: E,
    },
}
fn unit(state: &LoadedState, id: ComponentId) -> Result<Option<&Unit>, QueryError> {
    Ok(state
        .world()
        .unit(UnitReference::from_component_id(id))
        .map_err(QueryError::World)?
        .map(|u| u.record))
}
fn city(state: &LoadedState, id: ComponentId) -> Result<Option<&c6_core::city::City>, QueryError> {
    match state.city(i32::from(id.owner), id.id) {
        Ok(c) => Ok(Some(c)),
        Err(c6_core::loaded_state::Error::Missing(MissingInput::City { .. })) => Ok(None),
        Err(e) => Err(QueryError::State(e)),
    }
}
fn subject_unit(subject: Object) -> Result<Option<ComponentId>, QueryError> {
    match subject {
        Object::Unit(id) => Ok(Some(id)),
        Object::City(_) | Object::NoInterface => Ok(None),
        Object::Unavailable(id) => Err(QueryError::MissingObject(id)),
    }
}
fn subject_city(subject: Object) -> Result<Option<ComponentId>, QueryError> {
    match subject {
        Object::City(id) => Ok(Some(id)),
        Object::Unit(_) | Object::NoInterface => Ok(None),
        Object::Unavailable(id) => Err(QueryError::MissingObject(id)),
    }
}
fn matches_unit_subject(subject: Object, current: ComponentId) -> Result<bool, QueryError> {
    Ok(subject_unit(subject)? == Some(current))
}
/// UnitEmbarked.Test23c9de0. Null interface differs from a resolved interface
/// whose current unit is absent; HOLD suppresses the saved embarked flag.
fn test_embarked(state: &LoadedState, r: &Receiver) -> Result<i32, QueryError> {
    let Some(id) = subject_unit(r.subject)? else {
        return Ok(3);
    };
    Ok(match unit(state, id)? {
        Some(u) if unit_movement_state::unit_is_embarked(u) => 0,
        _ => 1,
    })
}
fn cached(r: &Receiver) -> Result<(i32, CachedUnit), QueryError> {
    match r.kind {
        Kind::CityHasGarrisonUnit {
            military_formation,
            cached_unit,
        } => Ok((military_formation, cached_unit)),
        _ => Err(QueryError::WrongKind),
    }
}
fn test_garrison(state: &LoadedState, defs: &Definitions, r: &Receiver) -> Result<i32, QueryError> {
    let Some(id) = subject_city(r.subject)? else {
        return Ok(1);
    };
    let Some(c) = city(state, id)? else {
        return Ok(1);
    };
    let owner = i32::from(id.owner);
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(QueryError::NullCity)?;
    let (formation, cached) = cached(r)?;
    for district_id in &c.districts.district_ids {
        if *district_id == u32::MAX {
            continue;
        }
        let Some(d) = player
            .districts
            .records
            .entries
            .iter()
            .find(|e| e.index == (*district_id & 0xffff) as i32)
            .map(|e| &e.value)
        else {
            continue;
        };
        let (defensible, internal) = defs.district(d)?;
        if !defensible {
            continue;
        }
        let candidate = match cached {
            CachedUnit::Unit(id) => {
                if internal {
                    continue;
                }
                let u = unit(state, id)?.ok_or(QueryError::MissingCachedUnit(u64::from(id.id)))?;
                if u.k_location
                    != (PlotCoord {
                        x: d.location.0,
                        y: d.location.1,
                    })
                {
                    continue;
                }
                Some(u)
            }
            CachedUnit::Unavailable(id) => return Err(QueryError::MissingCachedUnit(id)),
            CachedUnit::None => {
                if !d.is_complete || internal {
                    continue;
                }
                let at = PlotCoord {
                    x: d.location.0,
                    y: d.location.1,
                };
                // Preserve BOTH native calls. HasGarrisonedUnit first selects
                // and rechecks IsCombat; only its true result invokes the
                // independent GetGarrisonedUnit selection and IsCombat again.
                if crate::combat_defender::garrison_lookup(state, &defs.combat, at)
                    .map_err(QueryError::Defender)?
                    .is_none()
                {
                    continue;
                }
                crate::combat_defender::garrison_lookup(state, &defs.combat, at)
                    .map_err(QueryError::Defender)?
            }
        };
        if let Some(u) = candidate {
            if formation == -1 || u.e_military_formation == formation {
                return Ok(0);
            }
        }
    }
    Ok(1)
}
pub fn test<C: Context>(owner: &C, receiver: u64) -> Result<i32, QueryError> {
    owner.runtime().check(owner.state())?;
    let r = owner.runtime().receiver(receiver)?;
    match r.kind {
        Kind::UnitEmbarked => test_embarked(owner.state(), r),
        Kind::CityHasGarrisonUnit { .. } => {
            let defs = owner.definitions()?;
            defs.check(owner.state())?;
            test_garrison(owner.state(), defs, r)
        }
    }
}
struct Live<'a, C> {
    owner: &'a mut C,
    receiver: u64,
}
impl<C: Context> simple_requirement::Context for Live<'_, C> {
    type Error = Error<C::Error>;
    fn active(&mut self) -> Result<bool, Self::Error> {
        Ok(self
            .owner
            .runtime()
            .receiver(self.receiver)
            .map_err(Error::Query)?
            .active())
    }
    fn requirement(&mut self) -> Result<&mut Requirement, Self::Error> {
        match &mut self
            .owner
            .runtime_mut()
            .receiver_mut(self.receiver)
            .map_err(Error::Query)?
            .simple
        {
            SimpleState::Defined(r) => Ok(r),
            _ => Err(Error::Query(QueryError::MissingDefinition(self.receiver))),
        }
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Self::Error> {
        self.owner
            .changed(self.receiver, old, new)
            .map_err(|error| Error::Consumer {
                receiver: self.receiver,
                old,
                new,
                error,
            })
    }
}
impl<C: Context> simple_requirement::TestContext for Live<'_, C> {
    fn test(&mut self) -> Result<i32, Self::Error> {
        test(self.owner, self.receiver).map_err(Error::Query)
    }
}
/// Dispatch filtering precedes Invalidate's active guard, exactly as the native
/// callbacks. City cached-unit mutation occurs even for an inactive requirement.
pub fn on_unit_moved<C: Context>(
    owner: &mut C,
    receiver: u64,
    current: ComponentId,
    _old: PlotCoord,
    new: PlotCoord,
) -> Result<(), Error<C::Error>> {
    owner.runtime().check(owner.state()).map_err(Error::Query)?;
    let r = owner.runtime().receiver(receiver).map_err(Error::Query)?;
    let kind = r.kind;
    let subject = r.subject;
    match kind {
        Kind::UnitEmbarked => {
            if !matches_unit_subject(subject, current).map_err(Error::Query)? {
                return Ok(());
            }
        }
        Kind::CityHasGarrisonUnit {
            military_formation, ..
        } => {
            let subject = subject_city(subject)
                .map_err(Error::Query)?
                .ok_or(Error::Query(QueryError::NullCityInterface))?;
            if subject.owner != current.owner {
                return Ok(());
            }
            let u = unit(owner.state(), current)
                .map_err(Error::Query)?
                .ok_or(Error::Query(QueryError::MissingObject(u64::from(
                    current.id,
                ))))?;
            let defs = owner.definitions().map_err(Error::Query)?;
            defs.check(owner.state()).map_err(Error::Query)?;
            if !defs
                .combat
                .can_attack(owner.state(), u)
                .map_err(|e| Error::Query(QueryError::Combat(e)))?
            {
                return Ok(());
            }
            let c = city(owner.state(), subject)
                .map_err(Error::Query)?
                .ok_or(Error::Query(QueryError::NullCity))?;
            let at = PlotCoord {
                x: c.header.location_x,
                y: c.header.location_y,
            };
            let cached_unit = if at == new {
                CachedUnit::Unit(current)
            } else {
                CachedUnit::None
            };
            owner
                .runtime_mut()
                .receiver_mut(receiver)
                .map_err(Error::Query)?
                .kind = Kind::CityHasGarrisonUnit {
                military_formation,
                cached_unit,
            };
        }
    }
    simple_requirement::invalidate(&mut Live { owner, receiver })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embarked_filter_compares_complete_subject_identity_before_test() {
        let current = ComponentId {
            owner: 7,
            component_type: 8,
            id: 0x12345678,
        };
        assert!(matches_unit_subject(Object::Unit(current), current).unwrap());
        for other in [
            ComponentId {
                owner: 6,
                ..current
            },
            ComponentId {
                component_type: 9,
                ..current
            },
            ComponentId {
                id: current.id ^ 0x10000,
                ..current
            },
        ] {
            assert!(!matches_unit_subject(Object::Unit(other), current).unwrap());
        }
        assert!(!matches_unit_subject(Object::City(current), current).unwrap());
        assert!(!matches_unit_subject(Object::NoInterface, current).unwrap());
        assert!(matches!(
            matches_unit_subject(Object::Unavailable(41), current),
            Err(QueryError::MissingObject(41))
        ));
    }
}
