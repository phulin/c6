//! FinishMoves uses existing current Formation attribute and report providers.
use super::*;
use c6_core::{movement_execution as kernel, unit_initialization::FormationReference};
struct Finish<'a, 'd, 'r, E> {
    state: &'a mut LoadedState,
    inputs: &'d Inputs<'r>,
    effects: &'a mut E,
    actor: UnitHandle,
}
impl<E: Effects> Finish<'_, '_, '_, E> {
    fn sync(&mut self, f: FormationReference) -> Result<(), Error<E::Error>> {
        let found = self
            .state
            .world()
            .player(f.owner)
            .map_err(Error::World)?
            .formation(f.requested_id)
            .map_err(Error::World)?
            .ok_or(Error::Unsupported(
                "retained FinishMoves Formation lifetime",
            ))?;
        if found.resolved_id != f.resolved_id {
            return Err(Error::Unsupported("replaced FinishMoves Formation"));
        }
        update_formation(
            self.state,
            f.owner,
            f.requested_id,
            self.inputs,
            self.effects,
        )
        .map_err(|e| Error::Formation(Box::new(e)))
    }
}
impl<E: Effects> kernel::UnitMovementContext for Finish<'_, '_, '_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor.get(self.state).map_err(Error::World)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.actor.get_mut(self.state).map_err(Error::State)
    }
    fn send_movement_report(&mut self, r: MovementReport) -> Result<(), Self::Error> {
        self.effects.report(self.state, r).map_err(Error::Provider)
    }
    fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
        if let Some(f) = kernel::FinishMovementContext::current_formation(self)? {
            self.sync(f)?
        }
        Ok(())
    }
}
impl<E: Effects> kernel::FinishMovementContext for Finish<'_, '_, '_, E> {
    type Formation = FormationReference;
    fn current_formation(&mut self) -> Result<Option<FormationReference>, Self::Error> {
        let u = self.actor.get(self.state).map_err(Error::World)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .formation)
    }
    fn sync_retained_formation(&mut self, f: FormationReference) -> Result<(), Self::Error> {
        self.sync(f)
    }
}
pub fn finish_unit_moves<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), kernel::UnitMovementError<Error<E::Error>>> {
    kernel::finish_unit_moves(&mut Finish {
        state,
        inputs,
        effects,
        actor: UnitHandle { owner, id },
    })
}
impl<E: Effects> kernel::FinishFormationContext for Members<'_, '_, '_, E> {
    fn finish_unit(&mut self, u: UnitHandle) -> Result<(), kernel::UnitMovementError<Self::Error>> {
        finish_unit_moves(self.state, u.owner, u.id, self.inputs, self.effects)
    }
}
pub fn finish_formation_moves<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<(), kernel::FormationMovementError<Error<E::Error>>> {
    kernel::finish_formation_moves(&mut Members {
        state,
        inputs,
        effects,
        owner,
        id,
    })
}
