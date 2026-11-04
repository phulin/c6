//! Computed stock command-manager guards and Formation legal choices.
//! Current state/resource/registry admission is external to these calculations.
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions as defs,
    unit_operation_manager::PhaseDefinitions,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    messages::{HandlerId, OperationHash},
    skip_turn::placement,
    unit::{ComponentId, Formation, Unit},
    unit_operation_interruption::ObservedRegistry,
};

#[derive(Debug)]
pub enum Error {
    Definitions(defs::Error),
    Rules(crate::Error),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Phase(crate::unit_operation_manager::Error),
    Placement(placement::Error),
    Registry(c6_core::unit_operation_interruption::Error),
    Unsupported(&'static str),
}
#[derive(Clone, Copy)]
struct UnitDefinition {
    trader: bool,
    spy: bool,
    class: i32,
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    properties: Vec<UnitDefinition>,
    max_hit_points: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Unsupported("command rules/state binding"));
        }
        defs::Definitions::validate_resources(rules, bound).map_err(Error::Definitions)?;
        let units = defs::collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("Units"))?;
        let properties = units
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("Unit row"))?;
                let flag = |key| match row.get(key).map_err(Error::Rules)? {
                    SqlValue::Integer(0) => Ok(false),
                    SqlValue::Integer(1) => Ok(true),
                    _ => Err(Error::Unsupported(key)),
                };
                let hash = match row.get("FormationClass").map_err(Error::Rules)? {
                    SqlValue::Text(s) => linux_type_hash(s),
                    SqlValue::Null => 0,
                    _ => return Err(Error::Unsupported("FormationClass")),
                };
                Ok(UnitDefinition {
                    trader: flag("MakeTradeRoute")?,
                    spy: flag("Spy")?,
                    class: formation_class(hash),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?;
        let mut values = parameters
            .rows()
            .filter(|r| r.text("Name").is_ok_and(|s| s == "COMBAT_MAX_HIT_POINTS"));
        let row = values
            .next()
            .ok_or(Error::Unsupported("COMBAT_MAX_HIT_POINTS"))?;
        if values.next().is_some() {
            return Err(Error::Unsupported("duplicate COMBAT_MAX_HIT_POINTS"));
        }
        let max_hit_points = match row.get("Value").map_err(Error::Rules)? {
            SqlValue::Text(v) => std::str::from_utf8(v).ok().and_then(|v| v.parse().ok()),
            SqlValue::Integer(v) => i32::try_from(*v).ok(),
            _ => None,
        }
        .ok_or(Error::Unsupported("COMBAT_MAX_HIT_POINTS scalar"))?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            properties,
            max_hit_points,
        })
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("current command state binding"));
        }
        Ok(())
    }
    fn definition(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<Option<UnitDefinition>, Error> {
        let index = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index;
        if index == -1 {
            return Ok(None);
        }
        self.units
            .at(index as u32)
            .ok_or(Error::Unsupported("current Unit definition index"))?;
        self.properties
            .get(index as usize)
            .copied()
            .map(Some)
            .ok_or(Error::Unsupported("current Unit properties"))
    }
    fn class(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Error> {
        Ok(self.definition(state, unit)?.map_or(-1, |d| d.class))
    }
    fn max_damage(&self, unit: &Unit) -> Result<i32, Error> {
        max_damage_from_properties(&unit.k_properties, self.max_hit_points)
    }
    fn dead(&self, unit: &Unit) -> Result<bool, Error> {
        Ok(unit.i_damage >= self.max_damage(unit)?)
    }

    /// Stock Commands.Manager.CanStart(false) common guards, in source order.
    /// The existing observed interruption provider computes only closed leaves;
    /// unknown actual queue identities/virtual bindings fail at that branch.
    pub fn common_guards(
        &self,
        state: &LoadedState,
        unit: &Unit,
        phases: &PhaseDefinitions,
        operations: Option<&ObservedRegistry<'_>>,
    ) -> Result<bool, Error> {
        self.check(state)?;
        if !phases
            .from_processor(&state.retained().core.phase_processor, unit.e_owner)
            .map_err(Error::Phase)?
        {
            return Ok(false);
        }
        state.world().player(unit.e_owner).map_err(Error::World)?;
        if !state.turn_active(unit.e_owner).map_err(Error::State)? {
            return Ok(false);
        }
        if state
            .player_slot_status(unit.e_owner)
            .map_err(Error::State)?
            == 3
            && state.pause_storage().map_err(Error::State)? != 0
        {
            return Ok(false);
        }
        if let Some(front) = unit.k_operations.nodes.first() {
            let operations = operations.ok_or(Error::Unsupported(
                "current operation interruption registry",
            ))?;
            let id = operations
                .saved_hash_runtime_id(OperationHash(front.operation_hash))
                .map_err(Error::Registry)?;
            if id != HandlerId(-1) {
                if let Some(handler) = operations.handler(id).map_err(Error::Registry)? {
                    if !handler.can_be_interrupted() {
                        return Ok(false);
                    }
                }
            }
        }
        if self.dead(unit)? {
            return Ok(false);
        }
        Ok(!unit.b_delayed_death)
    }

    /// Complete stock Formation.CanAddUnit(pointer) calculation. No ownership,
    /// domain, movement or count policy is added beyond the original checks.
    pub fn formation_can_add(
        &self,
        state: &LoadedState,
        formation: &Formation,
        candidate: Option<&Unit>,
    ) -> Result<bool, Error> {
        self.check(state)?;
        let Some(candidate) = candidate else {
            return Ok(false);
        };
        let Some(&first) = formation.unit_ids.first() else {
            return Ok(true);
        };
        let first = find_unit(state, formation.owner, first)?;
        if state
            .unit_runtime(candidate.e_owner, candidate.id)
            .map_err(Error::State)?
            .domain
            == 1
            || candidate.b_delayed_death
            || self.dead(candidate)?
        {
            return Ok(false);
        }
        let first = first.ok_or(Error::Unsupported("nonempty Formation first Unit null"))?;
        if candidate.k_location != first.k_location {
            return Ok(false);
        }
        let definition = self
            .definition(state, candidate)?
            .ok_or(Error::Unsupported("CanAddUnit null candidate definition"))?;
        if definition.trader || definition.spy {
            return Ok(false);
        }
        let first_class = self.class(state, first)?;
        let candidate_class = self.class(state, candidate)?;
        if formation.unit_ids.len() == 1
            && matches!((first_class, candidate_class), (1, 2) | (2, 1))
        {
            return Ok(false);
        }
        let members = match formation_for(state, candidate)? {
            None => Vec::new(),
            Some(group) => {
                let mut members = Vec::new();
                for &id in &group.unit_ids {
                    if let Some(unit) = find_unit(state, group.owner, id)? {
                        members.push(unit)
                    }
                }
                members
            }
        };
        membership_permits(
            &formation.unit_ids,
            candidate.id,
            &members,
            |id| {
                let member = find_unit(state, formation.owner, id)?
                    .ok_or(Error::Unsupported("CanAddUnit current member null"))?;
                self.class(state, member)
            },
            |member| self.class(state, member),
        )
    }

    /// HoldCycling passes a NONNULL results map. EnterFormation therefore
    /// computes candidates and returns !empty directly, without GetParameters
    /// or the no-output CanStart branch. Logical output order/duplicates remain.
    pub fn enter_formation_candidates(
        &self,
        state: &LoadedState,
        unit: &Unit,
        tags: Option<&Tags>,
    ) -> Result<Vec<ComponentId>, Error> {
        self.check(state)?;
        if state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .domain
            == 1
        {
            return Ok(Vec::new());
        }
        let definition = self
            .definition(state, unit)?
            .ok_or(Error::Unsupported("EnterFormation null Unit definition"))?;
        if definition.trader || definition.spy {
            return Ok(Vec::new());
        }
        let index = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index;
        let name = &self
            .units
            .at(index as u32)
            .ok_or(Error::Unsupported("Unit definition"))?
            .type_name;
        let tags = tags.ok_or(Error::Unsupported("current Tag repository"))?;
        if tags.has(linux_type_hash(name), b"CLASS_RELIGIOUS_ALL") {
            return Ok(Vec::new());
        }
        let layer = placement::unit_layer(state.world(), unit).map_err(Error::Placement)?;
        let at =
            placement::units_at(state.world(), layer, unit.k_location).map_err(Error::Placement)?;
        let Some(formation) = formation_for(state, unit)? else {
            return Ok(Vec::new());
        };
        let mut candidates = Vec::new();
        for reference in at {
            let id = reference.component_id();
            if id
                == (ComponentId {
                    owner: unit.e_owner as i16,
                    component_type: 1,
                    id: unit.id,
                })
                || i32::from(id.owner) != unit.e_owner
            {
                continue;
            }
            let candidate = find_unit(state, formation.owner, id.id)?;
            if self.formation_can_add(state, formation, candidate)? {
                candidates.push(id)
            }
        }
        Ok(candidates)
    }
}
/// Complete current tag-name vector, sorted hash→type-index lookup and
/// type-index→tag-index memberships. Duplicate hashes select the first lookup.
/// Tag lookup
/// IDs are vector positions, NOT hashes. Names retain native lexical order;
/// duplicate names use the first matching position. This is initial state, not
/// a captured HasTag answer. A missing repository must not become an empty one.
pub struct Tags {
    names: Vec<Vec<u8>>,
    type_lookup: Vec<(u32, u32)>,
    memberships: Vec<(u32, u32)>,
}
impl Tags {
    pub fn from_current(
        names: Vec<Vec<u8>>,
        type_lookup: Vec<(u32, u32)>,
        memberships: Vec<(u32, u32)>,
    ) -> Result<Self, Error> {
        if names.len() > u32::MAX as usize
            || names.iter().any(|n| n.contains(&0))
            || names.windows(2).any(|p| p[0] > p[1])
            || type_lookup.windows(2).any(|p| p[0].0 > p[1].0)
            || memberships.windows(2).any(|p| p[0].0 > p[1].0)
            || memberships
                .iter()
                .any(|(_, id)| *id as usize >= names.len())
        {
            return Err(Error::Unsupported("current Tag repository shape"));
        }
        Ok(Self {
            names,
            type_lookup,
            memberships,
        })
    }
    pub fn has(&self, type_hash: u32, name: &[u8]) -> bool {
        let tag = self.names.partition_point(|s| s.as_slice() < name);
        if !self.names.get(tag).is_some_and(|s| s == name) {
            return false;
        }
        let index = self
            .type_lookup
            .partition_point(|&(hash, _)| hash < type_hash);
        let Some(&(hash, type_index)) = self.type_lookup.get(index) else {
            return false;
        };
        hash == type_hash && self.memberships.contains(&(type_index, tag as u32))
    }
}
fn max_damage_from_properties(
    properties: &c6_core::unit::variants::TypedVariantMap,
    fallback: i32,
) -> Result<i32, Error> {
    let mut values = properties.entries.iter().filter(|e| e.key == 0x7d3fc711);
    let Some(entry) = values.next() else {
        return Ok(fallback);
    };
    if values.next().is_some() {
        return Err(Error::Unsupported("duplicate current MaxDamage property"));
    }
    let value = &entry.value;
    if value.type_bits != 2
        || value.modifier_bits != 0
        || value.value.count_flags != 0
        || value.value.element_size != 0
    {
        return Err(Error::Unsupported("MaxDamage integer conversion"));
    }
    match value.value.payload {
        c6_core::unit::variants::VariantPayload::Inline32(v) => Ok(v as i32),
        _ => Err(Error::Unsupported("MaxDamage payload")),
    }
}
fn membership_permits<M: Copy, E>(
    destination: &[u32],
    candidate_id: u32,
    members: &[M],
    mut destination_class: impl FnMut(u32) -> Result<i32, E>,
    mut candidate_class: impl FnMut(M) -> Result<i32, E>,
) -> Result<bool, E> {
    for &id in destination {
        if id == candidate_id {
            return Ok(false);
        }
        let class = destination_class(id)?;
        for &member in members {
            if classes_conflict(class, candidate_class(member)?) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn classes_conflict(a: i32, b: i32) -> bool {
    a == b || a == 4 || b == 4
}
fn formation_class(hash: u32) -> i32 {
    match hash {
        0x9ad485f8 => 0,
        0xf1c32241 => 1,
        0xc2510aa7 => 2,
        0xfeeb5b2f => 3,
        0x31b700cb => 4,
        _ => -1,
    }
}
fn find_unit(state: &LoadedState, owner: i32, id: u32) -> Result<Option<&Unit>, Error> {
    Ok(state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .unit(id)
        .map_err(Error::World)?
        .map(|v| v.record))
}
fn formation_for<'a>(state: &'a LoadedState, unit: &Unit) -> Result<Option<&'a Formation>, Error> {
    let runtime = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?;
    let Some(reference) = runtime.formation else {
        return Ok(None);
    };
    state
        .formation_runtime(reference.owner, reference.requested_id)
        .map_err(Error::State)?;
    let formation = state
        .world()
        .player(reference.owner)
        .map_err(Error::World)?
        .formation(reference.requested_id)
        .map_err(Error::World)?
        .ok_or(Error::Unsupported("current Formation null"))?;
    if formation.resolved_id != reference.resolved_id {
        return Err(Error::Unsupported("current Formation identity"));
    }
    Ok(Some(formation.record))
}

/// Original stock dispatch identity after actual registration. Other subtypes
/// retain their key and fail only when full CanStart dispatch reaches them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandCode {
    EnterFormation,
    ExitFormation,
    HarvestWonder,
    Other,
}
/// This entry models the original scalar BaseHandler.GetHash getter only.
/// A custom/effectful virtual type getter must not be represented by this type;
/// use the general Queries effect interface until that override is implemented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegisteredCommand {
    pub type_hash: u32,
    pub code: CommandCode,
}
/// Current ordered registrations produced by original Register semantics.
/// Fresh hashes append; a duplicate replaces its existing slot and keeps its
/// numeric ID. Construction verifies this unique-slot invariant, not runtime
/// observation. RegisterCommands order is distinct from SQL definition order.
pub struct RegisteredCommands {
    slots: Vec<RegisteredCommand>,
}
impl RegisteredCommands {
    pub fn scan_budget(&self) -> usize {
        self.slots.len() + 1
    }
    pub fn from_current_slots(slots: Vec<RegisteredCommand>) -> Result<Self, Error> {
        if slots.len() > 0x800 {
            return Err(Error::Unsupported("command numeric slot domain"));
        }
        for (i, slot) in slots.iter().enumerate() {
            if slots[..i].iter().any(|s| s.type_hash == slot.type_hash) {
                return Err(Error::Unsupported("duplicate command slot hash"));
            }
            let expected = match slot.code {
                CommandCode::EnterFormation => Some(b"UNITCOMMAND_ENTER_FORMATION".as_slice()),
                CommandCode::ExitFormation => Some(b"UNITCOMMAND_EXIT_FORMATION".as_slice()),
                CommandCode::HarvestWonder => Some(b"UNITCOMMAND_HARVEST_WONDER".as_slice()),
                CommandCode::Other => None,
            };
            if expected.is_some_and(|name| linux_type_hash(name) != slot.type_hash) {
                return Err(Error::Unsupported("stock command type/code binding"));
            }
        }
        Ok(Self { slots })
    }
}
/// Stock default TypedVariantMap/TypedVariantManager plus ordinary allocation.
/// The local results contain only a ComponentID POD array (type0x401, size8),
/// never custom variants. Its creation/destruction writes only temporary data
/// and allocator state; retaining native heap layout is unnecessary here.
/// Custom managers, interposition or semantic allocation hooks are unsupported.
#[derive(Clone, Copy)]
pub enum TemporaryScope {
    StockPodResults,
}

/// Concrete command-side Queries implementation. All supported guards/subtypes
/// are computed from owned state. No CanStart return is supplied as an input.
/// Operations remain a separate existing Manager/Script provider dependency.
pub struct Computed<'a> {
    pub definitions: &'a Definitions,
    pub state: &'a LoadedState,
    pub unit: &'a Unit,
    pub phases: &'a PhaseDefinitions,
    pub operations: Option<&'a ObservedRegistry<'a>>,
    pub registrations: &'a RegisteredCommands,
    pub tags: Option<&'a Tags>,
    pub temporary_scope: TemporaryScope,
}
impl super::Queries for Computed<'_> {
    type Handler = usize;
    type Error = Error;
    fn slot(&mut self, kind: super::Kind, index: usize) -> Result<super::Slot<usize>, Error> {
        if kind != super::Kind::Command {
            return Err(Error::Unsupported(
                "operation HoldCycling full Manager/Script provider",
            ));
        }
        Ok(if index < self.registrations.slots.len() {
            super::Slot::Handler(index)
        } else {
            super::Slot::End
        })
    }
    fn type_key(&mut self, kind: super::Kind, handler: usize) -> Result<u32, Error> {
        if kind != super::Kind::Command {
            return Err(Error::Unsupported("operation type getter"));
        }
        self.registrations
            .slots
            .get(handler)
            .map(|s| s.type_hash)
            .ok_or(Error::Unsupported("command handler slot"))
    }
    fn runtime_id(&mut self, kind: super::Kind, key: u32) -> Result<i32, Error> {
        if kind != super::Kind::Command {
            return Err(Error::Unsupported("operation hash map"));
        }
        Ok(self
            .registrations
            .slots
            .iter()
            .position(|s| s.type_hash == key)
            .map_or(-1, |i| i as i32))
    }
    fn can_start_and_cleanup(&mut self, query: super::DefaultQuery) -> Result<bool, Error> {
        if query.kind != super::Kind::Command {
            return Err(Error::Unsupported("operation CanStart"));
        }
        if !self
            .definitions
            .common_guards(self.state, self.unit, self.phases, self.operations)?
        {
            return Ok(false);
        }
        let handler = self
            .registrations
            .slots
            .get(query.runtime_id as u32 as usize)
            .ok_or(Error::Unsupported(
                "command CanStart requested handler null",
            ))?;
        match handler.code {
            CommandCode::EnterFormation => Ok(!self
                .definitions
                .enter_formation_candidates(self.state, self.unit, self.tags)?
                .is_empty()),
            CommandCode::ExitFormation => super::exit_formation_current(self.state, self.unit)
                .map_err(|_| Error::Unsupported("ExitFormation current runtime resolution")),
            CommandCode::HarvestWonder => Ok(super::harvest_wonder_can_start()),
            CommandCode::Other => Err(Error::Unsupported(
                "selected command subtype CanStart effects",
            )),
        }
        // Supported output destruction is local POD buffer/tree cleanup only.
    }
}

#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub hold_definitions: &'a super::Definitions,
    pub definitions: &'a Definitions,
    pub phases: &'a PhaseDefinitions,
    pub operations: Option<&'a ObservedRegistry<'a>>,
    pub registrations: &'a RegisteredCommands,
    pub tags: Option<&'a Tags>,
    pub temporary_scope: TemporaryScope,
}
#[derive(Debug)]
pub enum CycleError {
    Reporting(crate::loaded_actions::Error),
    Command(super::Error<Error>),
    Binding,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tag_names_are_indices_not_hashes_and_duplicates_choose_first() {
        let tags = Tags::from_current(
            vec![b"A".to_vec(), b"CLASS_RELIGIOUS_ALL".to_vec()],
            vec![(3, 9), (3, 10), (u32::MAX, 12)],
            vec![(9, 0), (10, 1), (12, 1)],
        )
        .unwrap();
        assert!(!tags.has(3, b"CLASS_RELIGIOUS_ALL"));
        assert!(tags.has(3, b"A"));
        assert!(tags.has(u32::MAX, b"CLASS_RELIGIOUS_ALL"));
        assert!(!tags.has(99, b"A"));
        assert!(!tags.has(3, b"a"));
        assert!(Tags::from_current(vec![b"B".to_vec(), b"A".to_vec()], vec![], vec![]).is_err());
    }
    #[test]
    fn source_class_mapping_and_cross_group_conflicts() {
        let hashes = [0x9ad485f8, 0xf1c32241, 0xc2510aa7, 0xfeeb5b2f, 0x31b700cb];
        for (i, hash) in hashes.into_iter().enumerate() {
            assert_eq!(formation_class(hash), i as i32)
        }
        assert_eq!(formation_class(0), -1);
        // Support conflicts with every group, including unknown(-1); otherwise
        // distinct classes do not conflict. This is not a max-member policy.
        assert!(classes_conflict(4, -1));
        assert!(classes_conflict(-1, -1));
        assert!(classes_conflict(0, 0));
        assert!(!classes_conflict(0, 3));
        assert!(!classes_conflict(1, 2)); // separate singleton rule rejects this pair
    }
    #[test]
    fn current_registration_order_is_independent_of_sql_and_bindings_checked() {
        let enter = RegisteredCommand {
            type_hash: linux_type_hash(b"UNITCOMMAND_ENTER_FORMATION"),
            code: CommandCode::EnterFormation,
        };
        let exit = RegisteredCommand {
            type_hash: linux_type_hash(b"UNITCOMMAND_EXIT_FORMATION"),
            code: CommandCode::ExitFormation,
        };
        let slots = RegisteredCommands::from_current_slots(vec![exit, enter]).unwrap();
        assert_eq!(slots.slots, [exit, enter]);
        assert!(RegisteredCommands::from_current_slots(vec![enter, enter]).is_err());
        assert!(
            RegisteredCommands::from_current_slots(vec![RegisteredCommand {
                type_hash: enter.type_hash,
                code: CommandCode::ExitFormation
            }])
            .is_err()
        );
    }
}

#[cfg(test)]
mod membership_tests {
    use super::*;
    use std::cell::RefCell;
    #[test]
    fn duplicate_id_stops_before_class_query_and_empty_group_is_not_candidate_class() {
        assert!(
            !membership_permits(&[7], 7, &[9], |_| Err::<i32, _>("must not read"), |_| Ok(4))
                .unwrap()
        );
        // Native null candidate Formation produces no group entries. It does
        // not substitute the candidate as a singleton or impose a count cap.
        let mut reads = 0;
        assert!(membership_permits(
            &[1, 2, 3, 4, 5],
            7,
            &[] as &[u32],
            |_| {
                reads += 1;
                Ok::<_, ()>(0)
            },
            |_| panic!()
        )
        .unwrap());
        assert_eq!(reads, 5);
    }
    #[test]
    fn nested_native_order_and_first_conflict_or_error() {
        let calls = RefCell::new(Vec::new());
        let result = membership_permits(
            &[1, 2],
            7,
            &[10, 11],
            |id| {
                calls.borrow_mut().push(id);
                Ok::<_, ()>(id as i32)
            },
            |id| {
                calls.borrow_mut().push(id);
                Ok(if id == 10 { 2 } else { 3 })
            },
        )
        .unwrap();
        assert!(!result);
        assert_eq!(*calls.borrow(), [1, 10, 11, 2, 10]);
        let mut called = Vec::new();
        let result = membership_permits(
            &[1, 2],
            7,
            &[10, 11],
            |_| Ok(1),
            |id| {
                called.push(id);
                Err::<i32, _>("missing current class")
            },
        );
        assert_eq!(result, Err("missing current class"));
        assert_eq!(called, [10]);
    }
    #[test]
    fn dynamic_max_damage_never_defaults_unknown_property_to_parameter() {
        use c6_core::unit::variants::*;
        let mut map = TypedVariantMap::default();
        assert_eq!(max_damage_from_properties(&map, 100).unwrap(), 100);
        let entry = TypedEntry {
            key: 0x7d3fc711,
            name: None,
            value: TypedVariant {
                type_bits: 2,
                modifier_bits: 0,
                value: ArrayVariant {
                    count_flags: 0,
                    element_size: 0,
                    payload: VariantPayload::Inline32(i32::MIN as u32),
                },
            },
        };
        map.entries.push(entry.clone());
        assert_eq!(max_damage_from_properties(&map, 100).unwrap(), i32::MIN);
        map.entries[0].value.type_bits = 20;
        assert!(max_damage_from_properties(&map, 100).is_err());
        map.entries[0] = entry.clone();
        map.entries.push(entry);
        assert!(max_damage_from_properties(&map, 100).is_err());
    }
}
