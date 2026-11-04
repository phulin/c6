//! Current-state Unit/Unit proposal registration and synchronous ObjectCreated.
//! This is an internal generation stage, not AttackInto or damage resolution.
use c6_core::{
    loaded_state::{combat_proposals::AccessError, LoadedState},
    map::UnitReference,
    melee_attack::proposals::registry as kernel,
    unit::ComponentId,
};

#[derive(Debug)]
pub enum Error<E> {
    Access(AccessError),
    Actor(super::Error),
    Effects(E),
    UnsupportedDistrictReference,
}
/// Execute a source-qualified receiver against the SAME mutable state and
/// canonical proposal. The receiver's collection/requirement/effect algorithms
/// must be concrete; unknown or missing profiles return an error, never no-op.
pub trait Effects {
    type Error;
    fn object_created_receiver(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: kernel::Handle,
    ) -> Result<(), Self::Error>;
}
pub struct Context<'a, E> {
    state: &'a mut LoadedState,
    effects: &'a mut E,
}
impl<'a, E: Effects> Context<'a, E> {
    pub fn new(state: &'a mut LoadedState, effects: &'a mut E) -> Self {
        Self { state, effects }
    }
    pub fn state(&self) -> &LoadedState {
        self.state
    }
    pub fn propose_units(
        &mut self,
        attacker: UnitReference,
        defender: UnitReference,
        location: c6_core::unit::PlotCoord,
    ) -> Result<(u32, kernel::Handle), kernel::DispatchError<Error<E::Error>>> {
        // Native caller already holds these objects. Resolve the real current
        // Rust counterparts before the first registry mutation; no query result
        // or imported native strength/legality answer is substituted.
        super::actor(self.state, attacker)
            .map_err(|e| kernel::DispatchError::Context(Error::Actor(e)))?;
        super::actor(self.state, defender)
            .map_err(|e| kernel::DispatchError::Context(Error::Actor(e)))?;
        let proposal = kernel::Proposal::melee_units(attacker, defender, Some(location));
        kernel::propose(self, proposal)
    }
}
impl<E: Effects> kernel::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    fn runtime(&self) -> Result<&kernel::Runtime, Self::Error> {
        self.state.combat_proposals().map_err(Error::Access)
    }
    fn runtime_mut(&mut self) -> Result<&mut kernel::Runtime, Self::Error> {
        self.state.combat_proposals_mut().map_err(Error::Access)
    }
    fn owner(&mut self, participant: kernel::Participant) -> Result<i32, Self::Error> {
        match participant {
            kernel::Participant::Unit(id) => {
                Ok(super::actor(self.state, id).map_err(Error::Actor)?.e_owner)
            }
            kernel::Participant::District(_) => Err(Error::UnsupportedDistrictReference),
        }
    }
    fn unit_id(&mut self, id: UnitReference) -> Result<ComponentId, Self::Error> {
        let unit = super::actor(self.state, id).map_err(Error::Actor)?;
        Ok(ComponentId {
            owner: unit.e_owner as i16,
            component_type: 1,
            id: unit.id,
        })
    }
    fn callback(&mut self, receiver: u64, proposal: kernel::Handle) -> Result<(), Self::Error> {
        self.effects
            .object_created_receiver(self.state, receiver, proposal)
            .map_err(Error::Effects)
    }
}
