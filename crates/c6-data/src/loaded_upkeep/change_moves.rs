//! Unit movement expenditure through the existing retained upkeep context.
use super::{
    Effects, Error, Inputs, LoadedState, Movement, MovementReport, RestoreMovementContext, Unit,
    UnitHandle,
};
use c6_core::movement_execution::{self, UnitMovementContext, UnitMovementError};

struct Change<'a, 's, 'i, 'd, E>(&'a mut Movement<'s, 'i, 'd, E>);
impl<E: Effects> UnitMovementContext for Change<'_, '_, '_, '_, E> {
    type Error = Error<E::Error>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        RestoreMovementContext::unit(self.0)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        RestoreMovementContext::unit_mut(self.0)
    }
    fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error> {
        RestoreMovementContext::send_movement_report(self.0, report)
    }
    fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
        RestoreMovementContext::sync_current_formation(self.0)
    }
}
/// Change one actual Unit's movement, preserving changed/exhausted reports and
/// the fresh current-Formation Sync performed by the original Unit method.
pub fn change_unit_moves<E: Effects>(
    state: &mut LoadedState,
    owner: i32,
    id: u32,
    inputs: &Inputs<'_>,
    effects: &mut E,
    delta_q8: i32,
) -> Result<bool, UnitMovementError<Error<E::Error>>> {
    let mut movement = Movement {
        state,
        inputs,
        effects,
        actor: UnitHandle { owner, id },
    };
    movement_execution::change_unit_moves(&mut Change(&mut movement), delta_q8)
}
