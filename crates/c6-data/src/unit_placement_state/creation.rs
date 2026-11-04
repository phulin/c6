//! Fresh Unit Placement.Set flags1 over the same canonical state and sight.
use super::*;
use c6_core::unit_placement_transition::normal_move::{set_initial, InitialContext};
#[allow(clippy::too_many_arguments)]
pub fn set<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions<'_, '_>,
    queries: &Queries<'_>,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    request: Request,
    tags: &crate::loaded_actions::reporting::hold_cycling::commands::Tags,
) -> Result<i32, Error<EffectsError<E>>> {
    definitions.validate_binding(state).map_err(Error::Query)?;
    let mut context = Context {
        sight: sight::Context {
            state,
            runtime,
            effects,
            sight_definitions: definitions.sight,
            actor: request.actor,
        },
        definitions,
        queries,
        creation_tags: Some(tags),
    };
    set_initial(&mut context, request.destination, request.validation_flags)
        .map_err(|error| Error::Transition(Box::new(error)))
}
impl<E: Effects> InitialContext for Context<'_, '_, '_, E> {
    fn definition_team_visibility(&mut self) -> Result<bool, Error<EffectsError<E>>> {
        let actor = self.actor()?;
        let index = self
            .sight
            .state
            .unit_runtime(actor.e_owner, actor.id)
            .map_err(Error::State)?
            .definition_index;
        self.definitions
            .placement
            .unit_team_visibility(
                usize::try_from(index).map_err(|_| Error::Missing("Unit definition index"))?,
            )
            .ok_or(Error::Missing("Unit TeamVisibility"))
    }
    fn local_observer(&mut self) -> Result<i32, Error<EffectsError<E>>> {
        self.sight
            .runtime
            .local_observer
            .ok_or(Error::Missing("local observer"))
    }
    fn same_team(&mut self, observer: i32, owner: i32) -> Result<bool, Error<EffectsError<E>>> {
        let a = self
            .sight
            .state
            .player_team(observer)
            .map_err(Error::State)?;
        let b = self.sight.state.player_team(owner).map_err(Error::State)?;
        Ok(a == b)
    }
}
