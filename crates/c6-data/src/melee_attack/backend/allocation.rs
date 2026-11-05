//! Shared non-temporary ObjectPool/ModifierSystem allocation metadata.
//! Logical handles stay in the existing EffectRuntime maps. Native heap/pool
//! storage is not cloned; the actual input counters and live relative order are.
use super::{ContextObject, EffectRuntime, Object, QueryError};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};

#[derive(Debug)]
pub(super) struct Pool {
    next_id: i32,
    next_order: usize,
}
pub(super) struct State {
    /// Constructor provenance only; objects remain in EffectRuntime.objects.
    pub(super) constructed_policies: Vec<u64>,
    pool: Option<Pool>,
    modifier_next: Option<i32>,
    modifier_ids: Vec<i32>,
    initial_units: Vec<(i32, Option<Vec<u32>>)>,
    initial_cities: Vec<(i32, Vec<u32>)>,
    initial_districts: Vec<ComponentId>,
    pub(super) absent_primary_districts: Vec<ComponentId>,
    absent_unit_wrappers: Vec<(UnitReference, [i32; 2])>,
    /// Imported strong owners outside the represented modifier graph.
    unit_subtype_external_owners: Vec<(u64, usize)>,
}
impl State {
    pub(super) fn from_initial(state: &LoadedState) -> Self {
        Self {
            pool: None,
            constructed_policies: Vec::new(),
            absent_unit_wrappers: Vec::new(),
            unit_subtype_external_owners: Vec::new(),
            modifier_next: None,
            modifier_ids: Vec::new(),
            initial_units: state
                .world()
                .players()
                .iter()
                .map(|p| {
                    (
                        p.owner(),
                        p.unit_records()
                            .ok()
                            .map(|rows| rows.iter().map(|r| r.value.id).collect()),
                    )
                })
                .collect(),
            absent_primary_districts: Vec::new(),
            initial_districts: state
                .retained()
                .players
                .iter()
                .flat_map(|p| {
                    p.districts.records.entries.iter().map(|r| ComponentId {
                        owner: p.header.owner as i16,
                        component_type: 3,
                        id: r.value.id,
                    })
                })
                .collect(),
            initial_cities: state
                .retained()
                .players
                .iter()
                .map(|p| {
                    (
                        p.header.owner,
                        p.cities
                            .records
                            .entries
                            .iter()
                            .map(|r| r.value.header.id)
                            .collect(),
                    )
                })
                .collect(),
        }
    }
    pub(super) fn has_initial_district(&self, district: ComponentId) -> bool {
        self.initial_districts.contains(&district)
    }
    pub(super) fn has_initial_city(&self, city: ComponentId) -> Result<bool, QueryError> {
        self.initial_cities
            .iter()
            .find(|r| r.0 == i32::from(city.owner))
            .map(|r| r.1.contains(&city.id))
            .ok_or(QueryError::Missing("initial City wrapper cache domain"))
    }
    fn has_initial_unit(&self, unit: UnitReference) -> Result<bool, QueryError> {
        self.initial_units
            .iter()
            .find(|r| r.0 == unit.player())
            .and_then(|r| r.1.as_ref())
            .map(|r| r.contains(&unit.id))
            .ok_or(QueryError::Missing("initial Unit wrapper cache domain"))
    }
}
impl EffectRuntime {
    /// Observed ObjectPool counter and normal object-list extent. Live pool
    /// Trim compaction cannot change relative order; new logical ordinals append
    /// after all initial entries, independently of physical allocation/layout.
    pub fn initialize_object_pool(
        &mut self,
        state: &LoadedState,
        next_instance_id: i32,
        source_pool_slots: usize,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if let Some(pool) = &self.allocations.pool {
            if pool.next_id == next_instance_id && pool.next_order == source_pool_slots {
                return Ok(());
            }
            return Err(QueryError::Unsupported(
                "ObjectPool counters already initialized/advanced",
            ));
        }
        self.allocations.pool = Some(Pool {
            next_id: next_instance_id,
            next_order: source_pool_slots,
        });
        Ok(())
    }
    pub fn initialize_modifier_system(
        &mut self,
        state: &LoadedState,
        next_instance_id: i32,
        existing_ids: &[i32],
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if self
            .allocations
            .modifier_next
            .is_some_and(|id| id != next_instance_id)
        {
            return Err(QueryError::Unsupported(
                "ModifierSystem counter already initialized/advanced",
            ));
        }
        for (i, id) in existing_ids.iter().enumerate() {
            if existing_ids[..i].contains(id) {
                return Err(QueryError::DuplicateModifierInstance(*id));
            }
        }
        self.allocations.modifier_next = Some(next_instance_id);
        for id in existing_ids {
            if !self.allocations.modifier_ids.contains(id) {
                self.allocations.modifier_ids.push(*id)
            }
        }
        Ok(())
    }
    pub fn next_object_instance_id(&self) -> Result<i32, QueryError> {
        self.allocations
            .pool
            .as_ref()
            .map(|p| p.next_id)
            .ok_or(QueryError::Missing("current ObjectPool counter"))
    }
    pub fn next_modifier_instance_id(&self) -> Result<i32, QueryError> {
        self.allocations
            .modifier_next
            .ok_or(QueryError::Missing("current ModifierSystem counter"))
    }
    /// Current registered Unit modifiers, including newly attached instances.
    /// This borrows the live registry, not allocation collision history.
    /// Combat collection modifiers retain their existing proposal owner.
    pub fn unit_modifiers(&self) -> Result<&[c6_core::unit_kill::objects::Modifier], QueryError> {
        self.unit_modifiers
            .as_deref()
            .ok_or(QueryError::Missing("current Unit modifier cleanup state"))
    }
    /// Post-increment immediately before modifier registration.
    /// City and Unit factories call this one owner, never separate counters.
    pub fn allocate_modifier_id(&mut self, state: &LoadedState) -> Result<i32, QueryError> {
        self.current_owner(state)?;
        let id = self.next_modifier_instance_id()?;
        if self.allocations.modifier_ids.contains(&id) {
            return Err(QueryError::DuplicateModifierInstance(id));
        }
        self.allocations
            .modifier_ids
            .try_reserve(1)
            .map_err(|_| QueryError::Missing("modifier ID allocation"))?;
        self.allocations.modifier_next = Some(id.wrapping_add(1));
        self.allocations.modifier_ids.push(id);
        Ok(id)
    }
    pub(super) fn append_normal_object(
        &mut self,
        state: &LoadedState,
        mut object: Object,
    ) -> Result<u64, QueryError> {
        self.current_owner(state)?;
        let pool = self
            .allocations
            .pool
            .as_ref()
            .ok_or(QueryError::Missing("current ObjectPool counter"))?;
        let id = pool.next_id;
        let order = pool.next_order;
        let next_order = order
            .checked_add(1)
            .ok_or(QueryError::Missing("ObjectPool order limit"))?;
        if self.objects.iter().any(|r| r.1.instance_id == id) {
            return Err(QueryError::DuplicateObjectInstance(id));
        }
        // Namespace disjoint from existing effect/modifier/requirement handles.
        let handle = (1u64 << 60) | u64::from(id as u32);
        object.instance_id = id;
        if let Some(life) = &mut object.unit_lifecycle {
            life.pool_ordinal = order
        }
        let unit_subtype = matches!(object.kind, ContextObject::Unit(_))
            && matches!(object.unit_sort_suffix, Some([1, _]));
        self.register_current_object(state, handle, object)?;
        if unit_subtype {
            // This constructor has no unrepresented caller/cache owner.
            self.allocations
                .unit_subtype_external_owners
                .push((handle, 0));
        }
        let pool = self.allocations.pool.as_mut().unwrap();
        // Append registers the object first, then increments the shared source
        // instance counter. Native addresses/weak_ptr capacity are not inputs.
        pool.next_id = id.wrapping_add(1);
        pool.next_order = next_order;
        Ok(handle)
    }
    /// Supply a directly observed empty cache entry for an existing Unit.
    /// Missing cache input remains unknown. This does not create a wrapper or
    /// advance the pool; the ordinary factory does that at its source callsite.
    pub fn initialize_unit_wrapper_cache_miss(
        &mut self,
        state: &LoadedState,
        unit: UnitReference,
        suffix: [i32; 2],
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if !matches!(suffix[0], 0 | 1) || (suffix[0] == 0 && suffix[1] != 0) {
            return Err(QueryError::Unsupported("Unit wrapper subtype"));
        }
        crate::melee_attack::actor(state, unit)
            .map_err(|_| QueryError::Missing("current Unit wrapper target"))?;
        if self
            .objects
            .iter()
            .any(|r| r.1.kind == ContextObject::Unit(unit) && r.1.unit_sort_suffix == Some(suffix))
        {
            return Err(QueryError::Unsupported(
                "observed empty Unit wrapper cache is occupied",
            ));
        }
        let key = (unit, suffix);
        if !self.allocations.absent_unit_wrappers.contains(&key) {
            self.allocations.absent_unit_wrappers.push(key);
        }
        Ok(())
    }
    /// Ordinary Unit/ability wrapper sharing. Existing wrappers keep identity;
    /// new canonical Units have constructor-established empty wrapper caches.
    /// An unimported BEFORE cache is missing input, never assumed empty.
    pub fn make_current_unit_object(
        &mut self,
        state: &LoadedState,
        unit: UnitReference,
        suffix: [i32; 2],
    ) -> Result<u64, QueryError> {
        self.current_owner(state)?;
        if !matches!(suffix[0], 0 | 1) || (suffix[0] == 0 && suffix[1] != 0) {
            return Err(QueryError::Unsupported("Unit wrapper subtype"));
        }
        crate::melee_attack::actor(state, unit)
            .map_err(|_| QueryError::Missing("current Unit wrapper target"))?;
        if let Some(row) = self
            .objects
            .iter()
            .find(|r| r.1.kind == ContextObject::Unit(unit) && r.1.unit_sort_suffix == Some(suffix))
        {
            return Ok(row.0);
        }
        if self.allocations.has_initial_unit(unit)?
            && !self
                .allocations
                .absent_unit_wrappers
                .contains(&(unit, suffix))
        {
            return Err(QueryError::Missing("current BEFORE Unit wrapper cache"));
        }
        let life = c6_core::unit_kill::objects::Object {
            pool_ordinal: 0, // Assigned from the SAME source-backed order on append.
            owned_modifiers: Vec::new(),
            affected: c6_core::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(
                Vec::new(),
            )
            .expect("empty constructor affected list"),
            destroyed: c6_core::unit_moved_signal::ListenerRing::from_ordered(&[])
                .expect("empty constructor destruction signal"),
        };
        self.append_normal_object(
            state,
            Object {
                kind: ContextObject::Unit(unit),
                instance_id: 0,
                unit_sort_suffix: Some(suffix),
                unit_lifecycle: Some(life),
            },
        )
    }
    /// Import the actual strong count for an existing ability/promotion wrapper.
    /// The count includes Unit graph context/tracked/applied references and
    /// combat collection modifier contexts. Combat subjects are proposal objects.
    /// Any remaining strong owners stay explicit and prevent logical retirement;
    /// the native ObjectPool and its type lookup hold weak pointers only.
    pub fn initialize_unit_subtype_wrapper_reference_count(
        &mut self,
        state: &LoadedState,
        handle: u64,
        strong_count: usize,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        let object = self.object(handle)?;
        ensure_unit_subtype_object(object)?;
        let modifiers = self.unit_modifiers.as_ref().ok_or(QueryError::Missing(
            "Unit subtype wrapper modifier reference graph",
        ))?;
        let represented = modifiers
            .iter()
            .map(|m| {
                usize::from(m.context == handle)
                    + m.tracked.iter().filter(|t| t.object == handle).count()
                    + m.applied.iter().filter(|&&h| h == handle).count()
            })
            .sum::<usize>()
            + combat_context_references(handle, combat_modifiers(state)?);
        let external = strong_count
            .checked_sub(represented)
            .ok_or(QueryError::Unsupported(
                "Unit subtype wrapper strong reference undercount",
            ))?;
        if let Some((_, old)) = self
            .allocations
            .unit_subtype_external_owners
            .iter()
            .find(|row| row.0 == handle)
        {
            return if *old == external {
                Ok(())
            } else {
                Err(QueryError::Unsupported(
                    "Unit subtype wrapper ownership already initialized",
                ))
            };
        }
        self.allocations
            .unit_subtype_external_owners
            .push((handle, external));
        Ok(())
    }
    /// Release a subtype wrapper after the last represented strong owner has
    /// been destroyed. Normal Unit wrappers have a separate Unit-owned cache.
    /// Missing ownership input or surviving callbacks cannot imply expiry.
    pub fn release_unit_subtype_wrapper(
        &mut self,
        state: &LoadedState,
        handle: u64,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        let modifiers = self.unit_modifiers.as_ref().ok_or(QueryError::Missing(
            "Unit subtype wrapper modifier reference graph",
        ))?;
        retire_unit_subtype_wrapper(
            handle,
            &mut self.objects,
            &mut self.allocations.unit_subtype_external_owners,
            &mut self.allocations.absent_unit_wrappers,
            modifiers,
            combat_modifiers(state)?,
        )
    }

    /// Unit.~Instance first assigns its normal modifier-object shared pointer
    /// empty. The supported stock path has already delivered UnitRemoved and
    /// cleared every logical owned/affected/tracked reference. Do not erase a
    /// live graph or silently substitute empty cleanup metadata.
    pub fn release_destroyed_unit_wrapper(
        &mut self,
        state: &LoadedState,
        unit: UnitReference,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        let matching = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, r)| r.1.kind == ContextObject::Unit(unit))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            if self
                .allocations
                .absent_unit_wrappers
                .contains(&(unit, [0, 0]))
            {
                return Ok(());
            }
            return Err(QueryError::Missing("current Unit normal-wrapper cache"));
        }
        if matching.len() != 1 || self.objects[matching[0]].1.unit_sort_suffix != Some([0, 0]) {
            return Err(QueryError::Unsupported(
                "Unit destructor additional ability-wrapper references",
            ));
        }
        let at = matching[0];
        let (handle, object) = &self.objects[at];
        let modifiers = self
            .unit_modifiers
            .as_ref()
            .ok_or(QueryError::Missing("Unit destructor modifier graph"))?;
        ensure_releasable_unit_object(*handle, object, modifiers)?;
        // Stock destruction callbacks capture raw receiver/member pointers.
        // Signal destruction clears these subscriptions without dispatch.
        self.objects[at]
            .1
            .unit_lifecycle
            .as_mut()
            .expect("checked")
            .destroyed
            .clear_subscriptions()
            .map_err(|_| {
                QueryError::Unsupported("Unit destructor held destruction subscriptions")
            })?;
        self.objects.remove(at);
        if !self
            .allocations
            .absent_unit_wrappers
            .contains(&(unit, [0, 0]))
        {
            self.allocations.absent_unit_wrappers.push((unit, [0, 0]));
        }
        // Neither source instance counter rewinds when an object is released.
        Ok(())
    }
}

fn retire_unit_subtype_wrapper(
    handle: u64,
    objects: &mut Vec<(u64, Object)>,
    external_owners: &mut Vec<(u64, usize)>,
    absent_wrappers: &mut Vec<(UnitReference, [i32; 2])>,
    modifiers: &[c6_core::unit_kill::objects::Modifier],
    combat: &[CombatModifier],
) -> Result<(), QueryError> {
    let object = &objects
        .iter()
        .find(|row| row.0 == handle)
        .ok_or(QueryError::MissingObject(handle))?
        .1;
    ensure_unit_subtype_object(object)?;
    match external_owners
        .iter()
        .find(|row| row.0 == handle)
        .map(|row| row.1)
    {
        Some(0) => (),
        Some(_) => {
            return Err(QueryError::Unsupported(
                "Unit subtype wrapper surviving external strong references",
            ))
        }
        None => return Err(QueryError::Missing("Unit subtype wrapper strong ownership")),
    }
    ensure_releasable_unit_object(handle, object, modifiers)?;
    ensure_no_combat_context_references(handle, combat)?;
    if object
        .unit_lifecycle
        .as_ref()
        .expect("checked")
        .destroyed
        .ordered_subscribers()
        .next()
        .is_some()
    {
        return Err(QueryError::Unsupported(
            "Unit subtype wrapper surviving callbacks",
        ));
    }
    let ContextObject::Unit(unit) = object.kind else {
        unreachable!()
    };
    let suffix = object.unit_sort_suffix.expect("checked");
    objects.retain(|row| row.0 != handle);
    external_owners.retain(|row| row.0 != handle);
    if !absent_wrappers.contains(&(unit, suffix)) {
        absent_wrappers.push((unit, suffix));
    }
    // Expiry does not rewind instance IDs or compact logical pool ordinals.
    Ok(())
}

type CombatModifier = c6_core::melee_attack::proposals::collection_modifiers::Modifier;
fn combat_modifiers(state: &LoadedState) -> Result<&[CombatModifier], QueryError> {
    Ok(state
        .combat_proposals()
        .map_err(QueryError::Access)?
        .collection_modifiers()
        .map_err(QueryError::Proposal)?
        .modifiers())
}
fn combat_context_references(handle: u64, modifiers: &[CombatModifier]) -> usize {
    modifiers.iter().filter(|m| m.context == handle).count()
}
fn ensure_no_combat_context_references(
    handle: u64,
    modifiers: &[CombatModifier],
) -> Result<(), QueryError> {
    if combat_context_references(handle, modifiers) != 0 {
        return Err(QueryError::Unsupported(
            "Unit subtype wrapper surviving combat modifier context",
        ));
    }
    Ok(())
}

fn ensure_unit_subtype_object(object: &Object) -> Result<(), QueryError> {
    if !matches!(object.kind, ContextObject::Unit(_))
        || !matches!(object.unit_sort_suffix, Some([1, _]))
    {
        return Err(QueryError::Unsupported(
            "Unit subtype wrapper ownership target",
        ));
    }
    Ok(())
}

fn ensure_releasable_unit_object(
    handle: u64,
    object: &Object,
    modifiers: &[c6_core::unit_kill::objects::Modifier],
) -> Result<(), QueryError> {
    let life = object
        .unit_lifecycle
        .as_ref()
        .ok_or(QueryError::Missing("Unit destructor object lifecycle"))?;
    if !life.owned_modifiers.is_empty() || !life.affected.entries().is_empty() {
        return Err(QueryError::Unsupported(
            "Unit destructor before owned/affected cleanup",
        ));
    }
    if modifiers.iter().any(|m| {
        m.context == handle
            || m.applied.contains(&handle)
            || m.tracked.iter().any(|t| t.object == handle)
    }) {
        return Err(QueryError::Unsupported(
            "Unit destructor surviving modifier references",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod destruction_tests {
    use super::*;
    use c6_core::{
        unit_kill::objects as k,
        unit_moved_signal::{modifiers as m, ListenerRing},
    };
    fn object() -> Object {
        Object {
            kind: ContextObject::Unit(UnitReference::from_component_id(
                c6_core::unit::ComponentId {
                    owner: 0,
                    component_type: 1,
                    id: 7,
                },
            )),
            instance_id: 11,
            unit_sort_suffix: Some([0, 0]),
            unit_lifecycle: Some(k::Object {
                pool_ordinal: 0,
                owned_modifiers: vec![],
                affected: m::AffectedModifiers::from_sorted(vec![]).unwrap(),
                destroyed: ListenerRing::from_ordered(&[]).unwrap(),
            }),
        }
    }
    #[test]
    fn final_owner_retirement_waits_for_graph_and_callbacks_without_changing_identity() {
        use c6_core::unit_kill::objects::Callback;
        let mut subtype = object();
        subtype.unit_sort_suffix = Some([1, 123]);
        let mut objects = vec![(1, subtype)];
        let mut owners = vec![(1, 1)];
        let mut absent = vec![];
        assert!(
            retire_unit_subtype_wrapper(1, &mut objects, &mut owners, &mut absent, &[], &[])
                .is_err()
        );
        assert_eq!(objects[0].0, 1);
        assert_eq!(objects[0].1.instance_id, 11);
        owners[0].1 = 0;
        objects[0]
            .1
            .unit_lifecycle
            .as_mut()
            .unwrap()
            .owned_modifiers
            .push(7);
        assert!(
            retire_unit_subtype_wrapper(1, &mut objects, &mut owners, &mut absent, &[], &[])
                .is_err()
        );
        let life = objects[0].1.unit_lifecycle.as_mut().unwrap();
        life.owned_modifiers.clear();
        life.destroyed
            .subscribe(Callback::OwnerDestroyed(7))
            .unwrap();
        assert!(
            retire_unit_subtype_wrapper(1, &mut objects, &mut owners, &mut absent, &[], &[])
                .is_err()
        );
        assert_eq!(objects[0].1.instance_id, 11);
        objects[0]
            .1
            .unit_lifecycle
            .as_mut()
            .unwrap()
            .destroyed
            .clear_subscriptions()
            .unwrap();
        retire_unit_subtype_wrapper(1, &mut objects, &mut owners, &mut absent, &[], &[]).unwrap();
        assert!(objects.is_empty());
        assert!(owners.is_empty());
        assert_eq!(absent.len(), 1);
    }
    #[test]
    fn surviving_combat_context_blocks_subtype_retirement() {
        use c6_core::melee_attack::proposals::collection_modifiers::EffectFamily;
        let modifier = CombatModifier {
            effect_family: EffectFamily::PlayerStrength,
            receiver: 2,
            context: 1,
            control: m::Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            instance: 4,
            definition: vec![],
            repeatable: false,
            stack_limit: 0,
            subject_requirements: vec![],
            effect: 5,
            tracked: vec![],
            applied: vec![],
        };
        assert_eq!(
            combat_context_references(1, std::slice::from_ref(&modifier)),
            1
        );
        assert!(ensure_no_combat_context_references(1, std::slice::from_ref(&modifier)).is_err());
        assert!(ensure_no_combat_context_references(7, &[modifier]).is_ok());
        assert!(ensure_no_combat_context_references(1, &[]).is_ok());
    }
    #[test]
    fn owned_modifiers_must_be_really_destroyed_before_cache_release() {
        let mut object = object();
        assert!(ensure_releasable_unit_object(1, &object, &[]).is_ok());
        object
            .unit_lifecycle
            .as_mut()
            .unwrap()
            .owned_modifiers
            .push(19);
        assert!(ensure_releasable_unit_object(1, &object, &[]).is_err());
        assert_eq!(
            object.unit_lifecycle.as_ref().unwrap().owned_modifiers,
            [19]
        );
    }
    #[test]
    fn a_tracked_reference_blocks_release_even_if_affected_list_is_empty() {
        let object = object();
        let modifier = k::Modifier {
            receiver: 2,
            context: 3,
            instance: 4,
            control: m::Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            permanent: false,
            definition: vec![],
            stack_limit: 0,
            effect: 5,
            tracked: vec![k::Tracked {
                object: 1,
                state: m::TrackedSubject {
                    requirement_set: None,
                    applied: 1,
                    removed: false,
                },
                destruction_subscription: None,
            }],
            applied: vec![],
        };
        assert!(ensure_releasable_unit_object(1, &object, &[modifier]).is_err());
    }
}
