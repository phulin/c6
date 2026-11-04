//! Actual current pairing maps from the admitted initial memory snapshot.
use super::{Error, LoadedState, ObservedSnapshotWitness};
impl LoadedState {
    /// Current observed logical state, distinct from an attributed wire owner
    /// frame. Admission requires the same initial snapshot as other live fields.
    pub fn initialize_current_object_pairing(
        &mut self,
        witness: &ObservedSnapshotWitness,
        manager: crate::object_pairing::Manager,
    ) -> Result<(), Error> {
        if self.object_pairing.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        self.advance_admission_generation()?;
        self.object_pairing = Some(manager);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_maps_require_same_initial_witness_and_bind_once() {
        let mut state = super::super::tests::state();
        let witness = ObservedSnapshotWitness {
            source_role: crate::core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: [1; 32],
            snapshot_bytes: 1,
            capture_runtime_sha256: [2; 32],
            capture_facts_sha256: [3; 32],
            capture_configuration_sha256: [4; 32],
        };
        let empty = || {
            crate::object_pairing::Manager::from_current_maps(
                vec![],
                vec![],
                crate::unit::Limits::default(),
            )
            .unwrap()
        };
        assert!(matches!(
            state.initialize_current_object_pairing(&witness, empty()),
            Err(Error::Binding)
        ));
        state.observed_snapshot = Some(witness.clone());
        state
            .initialize_current_object_pairing(&witness, empty())
            .unwrap();
        assert_eq!(
            state
                .object_pairing()
                .unwrap()
                .get_parent(crate::object_pairing::NULL_COMPONENT),
            crate::object_pairing::NULL_COMPONENT
        );
        assert!(matches!(
            state.initialize_current_object_pairing(&witness, empty()),
            Err(Error::AlreadyBound)
        ));
        assert!(state.object_pairing_source().is_none());
    }
}
