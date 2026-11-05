//! Reached Improvement MakeObject/ClearModifiers on the shared ObjectPool.
//! Positive modifier removal remains explicit; an empty temporary strong owner
//! still consumes the normal instance ID on a cache miss.
use super::{ContextObject, EffectRuntime, Object, QueryError};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};

impl EffectRuntime {
    /// `complete_domain` qualifies the actual initial Improvement object domain,
    /// including empty caches. Unknown initial cache membership is never a miss.
    pub fn clear_improvement_modifiers(
        &mut self,
        state: &LoadedState,
        plot: u32,
        improvement: u32,
        city: ComponentId,
        owner: i32,
        complete_domain: bool,
    ) -> Result<(), QueryError> {
        self.current_owner(state)?;
        if !complete_domain {
            return Err(QueryError::Missing("current Improvement wrapper domain"));
        }
        let kind = ContextObject::Improvement {
            plot,
            improvement,
            city,
            owner,
        };
        if let Some((_, object)) = self.objects.iter().find(|(_, object)| object.kind == kind) {
            let life = object
                .unit_lifecycle
                .as_ref()
                .ok_or(QueryError::Missing("Improvement owned modifier list"))?;
            if !life.owned_modifiers.is_empty() {
                return Err(QueryError::Unsupported(
                    "positive Improvement ClearModifiers",
                ));
            }
            return Ok(());
        }
        let life = c6_core::unit_kill::objects::Object {
            pool_ordinal: 0,
            owned_modifiers: Vec::new(),
            affected: c6_core::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(
                Vec::new(),
            )
            .expect("empty constructor affected list"),
            destroyed: c6_core::unit_moved_signal::ListenerRing::from_ordered(&[])
                .expect("empty constructor destruction signal"),
        };
        let handle = self.append_normal_object(
            state,
            Object {
                kind,
                instance_id: 0,
                unit_sort_suffix: None,
                unit_lifecycle: Some(life),
            },
        )?;
        // Share's normal list and type index are weak owners. After ClearModifiers
        // sees an empty list, the sole local shared_ptr dies at 23e1a27. Keep the
        // consumed allocation counter/order, discard only the expired wrapper.
        self.objects.retain(|(id, _)| *id != handle);
        Ok(())
    }
}
