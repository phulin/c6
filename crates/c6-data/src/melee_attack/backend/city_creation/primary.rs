//! District primary contexts of Objects::District subtype1 Building owners.
//! Factory Initialize borrows the primary; subject requirements retain it.
use super::{AffectedModifiers, EffectRuntime, Error, ListenerRing, Object};
use crate::melee_attack::backend::{ContextObject, Object as ContextRecord};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};
#[derive(Default)]
pub(super) struct State {
    entries: Vec<Entry>,
}
struct Entry {
    district: ComponentId,
    handle: Option<u64>,
    retained: bool,
}
impl EffectRuntime {
    /// Bind an observed absent subtype0 District cache before any action.
    /// A missing cache for an existing District remains unknown, not empty.
    pub fn initialize_absent_district_primary(
        &mut self,
        state: &LoadedState,
        district: ComponentId,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if !state.is_pristine() || district.component_type != 3 {
            return Err(Error::Missing("pristine primary District admission"));
        }
        state
            .district(i32::from(district.owner), district.id)
            .map_err(|_| Error::Missing("initial primary District"))?;
        if self
            .allocations
            .absent_primary_districts
            .contains(&district)
        {
            return Err(Error::Missing("duplicate primary District admission"));
        }
        self.allocations.absent_primary_districts.push(district);
        Ok(())
    }
    pub(super) fn primary_district_context(
        &mut self,
        state: &LoadedState,
        owner: u64,
        retain: bool,
    ) -> Result<u64, Error> {
        let ContextObject::Building { city, district, .. } =
            self.object(owner).map_err(Error::Query)?.kind
        else {
            return Ok(owner);
        };
        self.primary_for_district(state, city, district, retain)
    }
    /// Native MakeObject(District*, subtype0, subvalue0), reached unconditionally
    /// by the DistrictDestroyed GameEffects subscriber even with logging off.
    pub fn make_district_object(
        &mut self,
        state: &LoadedState,
        district: ComponentId,
    ) -> Result<u64, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if district.component_type != 3 {
            return Err(Error::Missing("District wrapper component type"));
        }
        let record = state
            .district(i32::from(district.owner), district.id)
            .map_err(|_| Error::Missing("current District wrapper target"))?;
        let city = ComponentId {
            owner: district.owner,
            component_type: 2,
            id: record.city_id,
        };
        state
            .city(i32::from(city.owner), city.id)
            .map_err(|_| Error::Missing("current District containing City"))?;
        self.primary_for_district(state, city, district, false)
    }
    fn primary_for_district(
        &mut self,
        state: &LoadedState,
        city: ComponentId,
        district: ComponentId,
        retain: bool,
    ) -> Result<u64, Error> {
        if let Some(handle) = self
            .objects
            .iter()
            .find(|(_, object)| object.kind == (ContextObject::District { city, district }))
            .map(|row| row.0)
        {
            // Existing ownership remains in the canonical graph. A requirement
            // retaining this source-created primary outlives the factory borrow.
            if let Some(entry) = self
                .city_creation
                .as_mut()
                .ok_or(Error::Missing("primary District lifecycle"))?
                .primary
                .entries
                .iter_mut()
                .find(|entry| entry.handle == Some(handle))
            {
                entry.retained |= retain;
            }
            return Ok(handle);
        }
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("primary District lifecycle"))?;
        if !creation
            .primary
            .entries
            .iter()
            .any(|r| r.district == district)
        {
            if self.allocations.has_initial_district(district)
                && !self
                    .allocations
                    .absent_primary_districts
                    .contains(&district)
            {
                return Err(Error::Missing("current BEFORE primary District cache"));
            }
            creation.primary.entries.push(Entry {
                district,
                handle: None,
                retained: false,
            });
        }
        let at = creation
            .primary
            .entries
            .iter()
            .position(|r| r.district == district)
            .unwrap();
        if let Some(handle) = creation.primary.entries[at].handle {
            creation.primary.entries[at].retained |= retain;
            return Ok(handle);
        }
        let handle = self
            .append_normal_object(
                state,
                ContextRecord {
                    kind: ContextObject::District { city, district },
                    instance_id: 0,
                    unit_sort_suffix: None,
                    unit_lifecycle: None,
                },
            )
            .map_err(Error::Query)?;
        let creation = self.city_creation.as_mut().unwrap();
        creation.objects.push(Object {
            handle,
            destruction: Some(super::destruction::Metadata {
                owned_modifiers: Vec::new(),
            }),
            affected: AffectedModifiers::from_sorted(Vec::new())
                .expect("empty primary affected list"),
            destroyed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
        });
        creation.primary.entries[at].handle = Some(handle);
        creation.primary.entries[at].retained = retain;
        Ok(handle)
    }
    pub(super) fn release_factory_primary(&mut self, handle: u64) -> Result<(), Error> {
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("primary District lifecycle"))?;
        let Some(entry) = creation
            .primary
            .entries
            .iter_mut()
            .find(|r| r.handle == Some(handle))
        else {
            return Ok(());
        };
        if entry.retained {
            return Ok(());
        }
        if creation.modifiers.iter().any(|record| {
            record.owner == handle
                || record.applied_subjects.contains(&handle)
                || record.tracked.iter().any(|row| row.subject == handle)
        }) || self.unit_modifiers.as_ref().is_some_and(|records| {
            records.iter().any(|record| {
                record.context == handle
                    || record.applied.contains(&handle)
                    || record.tracked.iter().any(|row| row.object == handle)
            })
        }) {
            // These are actual strong owners in the same canonical graph.
            return Ok(());
        }
        let object = creation
            .objects
            .iter()
            .find(|object| object.handle == handle)
            .ok_or(Error::Missing("current primary District lifecycle"))?;
        if !object.affected.entries().is_empty()
            || object.destroyed.ordered_subscribers().next().is_some()
        {
            return Err(Error::Missing(
                "primary District surviving effect/listener ownership",
            ));
        }
        // These source-gated Initialize methods retain no primary context.
        // Release the borrowed wrapper; the shared monotonic counter stays live.
        entry.handle = None;
        creation.objects.retain(|r| r.handle != handle);
        self.objects.retain(|r| r.0 != handle);
        Ok(())
    }
}
