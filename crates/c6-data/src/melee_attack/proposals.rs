//! Current-state Unit/Unit proposal registration and synchronous ObjectCreated.
//! This is an internal generation stage, not AttackInto or damage resolution.
pub mod collection_modifiers;
use c6_core::melee_attack::proposals::{alliance, collections};
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
    Runtime(kernel::Error),
    MissingCollection(u64),
    MissingCollectionKind(u64),
    Alliance(Box<alliance::Error<Error<E>>>),
    Collection(Box<collections::Error<Error<E>>>),
}
/// Execute a source-qualified downstream modifier receiver against the SAME
/// state and canonical proposal. Collection filtering/AddItem dispatch is
/// computed here; unknown modifier profiles return errors, never no-op.
pub trait Effects {
    type Error;
    fn collection_item_changed(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        proposal: &mut kernel::Handle,
        added: bool,
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
        let kind = self
            .state
            .combat_proposals()
            .map_err(Error::Access)?
            .collections()
            .map_err(Error::Runtime)?
            .collection(receiver)
            .ok_or(Error::MissingCollection(receiver))?
            .kind
            .ok_or(Error::MissingCollectionKind(receiver))?;
        match kind {
            collections::Kind::Alliance { .. } => {
                alliance::on_combat_proposed(self, receiver, proposal)
                    .map_err(|e| Error::Alliance(Box::new(e)))
            }
            collections::Kind::Player { .. } | collections::Kind::Unit { .. } => {
                collections::add_item(self, receiver, proposal)
                    .map_err(|e| Error::Collection(Box::new(e)))
            }
        }
    }
}
impl<E: Effects> collections::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    fn collections(&self) -> Result<&collections::Runtime, Self::Error> {
        self.state
            .combat_proposals()
            .map_err(Error::Access)?
            .collections()
            .map_err(Error::Runtime)
    }
    fn collections_mut(&mut self) -> Result<&mut collections::Runtime, Self::Error> {
        self.state
            .combat_proposals_mut()
            .map_err(Error::Access)?
            .collections_mut()
            .map_err(Error::Runtime)
    }
    fn item_changed(
        &mut self,
        receiver: u64,
        proposal: &mut kernel::Handle,
        added: bool,
    ) -> Result<(), Self::Error> {
        self.effects
            .collection_item_changed(self.state, receiver, proposal, added)
            .map_err(Error::Effects)
    }
}
