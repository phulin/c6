//! Current Alert Update/CheckComplete joins; shared native enemy scan is reused.
use super::*;
struct Current<'a, E, M> {
    state: &'a mut LoadedState,
    actor: (i32, u32),
    data: &'a mut Processing,
    effects: &'a mut E,
    movement: &'a M,
}
impl<E: Effects, M: Movement<E>> c6_core::game_update::automation::alert::Context
    for Current<'_, E, M>
{
    type Error = Error<EffectError<E>, M::Error>;
    fn maximum(&mut self) -> Result<i32, Self::Error> {
        Ok(self.movement.fortify_turn_max())
    }
    fn fortifyable(&mut self) -> Result<bool, Self::Error> {
        self.movement
            .is_fortifyable(self.state, self.actor)
            .map_err(Error::Movement)
    }
    fn turns(&mut self) -> Result<i32, Self::Error> {
        Ok(unit(self.state, self.actor)?.i_fortify_turns)
    }
    fn set_turns(&mut self, turns: i32) -> Result<(), Self::Error> {
        self.state
            .unit_mut(self.actor.0, self.actor.1)
            .map_err(Error::State)?
            .i_fortify_turns = turns;
        Ok(())
    }
    fn fortify_activity(&mut self) -> Result<(), Self::Error> {
        loaded_upkeep::set_unit_activity(
            self.state,
            self.actor.0,
            self.actor.1,
            self.effects,
            0xd0aed661u32 as i32,
        )
        .map_err(Error::Upkeep)
    }
    fn clear_delete(&mut self) {
        self.data.delete = false;
    }
}
pub(super) fn update<E: Effects, M: Movement<E>>(
    state: &mut LoadedState,
    actor: (i32, u32),
    data: &mut Processing,
    effects: &mut E,
    movement: &M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    c6_core::game_update::automation::alert::update(&mut Current {
        state,
        actor,
        data,
        effects,
        movement,
    })
}
pub(super) fn check_complete<E: Effects, M: Movement<E>>(
    state: &mut LoadedState,
    actor: (i32, u32),
    data: &mut Processing,
    runtime: &Runtime<'_>,
    effects: &mut E,
    movement: &M,
) -> Result<(), Error<EffectError<E>, M::Error>> {
    if movement
        .alert_enemy_visible(state, runtime.visibility, actor)
        .map_err(Error::Movement)?
    {
        data.delete = true;
        loaded_upkeep::set_unit_activity(state, actor.0, actor.1, effects, 0x6b25012e)
            .map_err(Error::Upkeep)?;
    } else {
        data.delete = false;
        data.stop = true;
    }
    Ok(())
}
pub(super) struct Visibility<'a>(pub &'a unit_sight_state::Runtime);
impl loaded_upkeep::AlertVisibility for Visibility<'_> {
    type Error = &'static str;
    fn shared_layers(
        &self,
        _: &LoadedState,
        owner: i32,
    ) -> Result<&c6_core::visibility_change::SharedLayers, Self::Error> {
        self.0.shared(owner)
    }
    fn grid_width(&self, _: &LoadedState, owner: i32) -> Result<u32, Self::Error> {
        self.0
            .shared(owner)?
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .map(|l| l.grid_width)
            .ok_or("Alert normal visibility width")
    }
    fn hidden_layer0(&self, _: &LoadedState) -> Result<u32, Self::Error> {
        self.0
            .hidden_layer_hashes
            .first()
            .copied()
            .ok_or("Alert hidden visibility layer0")
    }
}
