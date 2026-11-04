//! Once-bound current proposal ownership. No absent native registry is inferred
//! from saved state, constructor defaults or a previous completed combat.
use super::{Error, LoadedState, ObservedSnapshotWitness};
use crate::melee_attack::proposals::registry::Runtime;
#[derive(Debug)]
pub enum AccessError {
    State(Error),
    Missing,
}
impl LoadedState {
    pub fn initialize_combat_proposals(
        &mut self,
        witness: &ObservedSnapshotWitness,
        runtime: Runtime,
    ) -> Result<(), Error> {
        if self.combat_proposals.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if self.observed_snapshot.as_ref() != Some(witness) {
            return Err(Error::Binding);
        }
        self.combat_proposals = Some(runtime);
        Ok(())
    }
    pub fn combat_proposals(&self) -> Result<&Runtime, AccessError> {
        self.combat_proposals.as_ref().ok_or(AccessError::Missing)
    }
    pub fn combat_proposals_mut(&mut self) -> Result<&mut Runtime, AccessError> {
        if self.combat_proposals.is_none() {
            return Err(AccessError::Missing);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AccessError::State(Error::ResourceLimit))?;
        self.revision = next;
        Ok(self
            .combat_proposals
            .as_mut()
            .expect("checked current owner"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        map::UnitReference,
        melee_attack::proposals::registry::{self as p, Subscriber},
        unit::ComponentId,
        unit_moved_signal::ListenerRing,
    };
    struct Current {
        state: LoadedState,
    }
    impl p::Context for Current {
        type Error = &'static str;
        fn runtime(&self) -> Result<&Runtime, Self::Error> {
            self.state.combat_proposals().map_err(|_| "missing")
        }
        fn runtime_mut(&mut self) -> Result<&mut Runtime, Self::Error> {
            self.state.combat_proposals_mut().map_err(|_| "missing")
        }
        fn owner(&mut self, _: p::Participant) -> Result<i32, Self::Error> {
            unreachable!()
        }
        fn unit_id(&mut self, _: UnitReference) -> Result<ComponentId, Self::Error> {
            unreachable!()
        }
        fn callback(&mut self, _: u64, h: p::Handle) -> Result<(), Self::Error> {
            self.state
                .combat_proposals_mut()
                .unwrap()
                .proposal_mut(h)
                .unwrap()
                .attacker_strength_modifier = 9;
            Err("required effect unavailable")
        }
    }
    #[test]
    fn canonical_owner_retains_registered_object_and_effect_prefix_on_error() {
        let (mut state, _, _) = super::super::tests::current_update_player_fixture();
        // Synthetic explicit current runtime on the real LoadedState adapter;
        // this test neither invents snapshot provenance nor claims native parity.
        state.combat_proposals = Some(
            Runtime::from_initial(
                7832,
                vec![],
                Some(ListenerRing::from_ordered(&[Subscriber::Callback(1)]).unwrap()),
                Some(vec![]),
                Some(vec![]),
            )
            .unwrap(),
        );
        let unit = UnitReference::from_component_id(ComponentId {
            owner: 0,
            component_type: 1,
            id: 7,
        });
        let mut c = Current { state };
        let revision = c.state.revision;
        assert!(matches!(
            p::propose(&mut c, p::Proposal::melee_units(unit, unit, None)),
            Err(p::DispatchError::Context("required effect unavailable"))
        ));
        let owner = c.state.combat_proposals().unwrap();
        assert_eq!(owner.counter(), 7833);
        let (id, h) = owner.active()[0];
        assert_eq!(id, 7833);
        assert_eq!(owner.proposal(h).unwrap().attacker_strength_modifier, 9);
        assert!(c.state.revision > revision);
    }
    #[test]
    fn missing_owner_does_not_advance_revision() {
        let (mut state, _, _) = super::super::tests::current_update_player_fixture();
        let before = state.revision;
        assert!(matches!(
            state.combat_proposals_mut(),
            Err(AccessError::Missing)
        ));
        assert_eq!(state.revision, before);
    }
}
