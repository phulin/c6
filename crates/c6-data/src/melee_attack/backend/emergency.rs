//! Emergency wrappers share the normal ObjectPool (IsTemporary23eb110 is false).
//! Their strong local owner is released after the target requirement and pending
//! Emergency list are destroyed. No alternate object-ID allocator is used here.
use super::{ContextObject, EffectRuntime, Object, QueryError};
use c6_core::loaded_state::LoadedState;

impl EffectRuntime {
    pub(crate) fn make_emergency_object(
        &mut self,
        state: &LoadedState,
        target: i32,
        emergency_type: i32,
        turn: i32,
    ) -> Result<u64, QueryError> {
        self.append_normal_object(state, Object {
            kind: ContextObject::Emergency { target, emergency_type, turn },
            instance_id: 0,
            unit_sort_suffix: None,
            unit_lifecycle: Some(c6_core::unit_kill::objects::Object {
                pool_ordinal: 0,
                owned_modifiers: Vec::new(),
                affected: c6_core::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(Vec::new())
                    .expect("empty Emergency constructor affected list"),
                destroyed: c6_core::unit_moved_signal::ListenerRing::from_ordered(&[])
                    .expect("empty Emergency constructor destroyed signal"),
            }),
        })
    }
    pub(crate) fn release_emergency_object(&mut self, handle: u64) -> Result<(), QueryError> {
        let object = self.object(handle)?;
        if !matches!(object.kind, ContextObject::Emergency { .. }) {
            return Err(QueryError::Unsupported("Emergency wrapper release class"));
        }
        let life = object.unit_lifecycle.as_ref()
            .ok_or(QueryError::Missing("Emergency wrapper lifecycle"))?;
        if !life.owned_modifiers.is_empty() {
            return Err(QueryError::Unsupported("Emergency wrapper modifier destruction"));
        }
        self.objects.retain(|(id, _)| *id != handle);
        Ok(())
    }
}
