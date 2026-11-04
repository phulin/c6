//! Loaded PlayerTurnComplete prefix over the canonical imported processor state.
//! This does not execute Game.Update or accept a complete EndTurn action.
use super::LoadedState;
use crate::{
    city_production::BuildQueue,
    turn_completion::{self as core, Error, LogBranch, World},
};

/// Synchronous effects remain mandatory and operate on the SAME current state.
pub trait Effects {
    fn log(&mut self, state: &mut LoadedState, branch: LogBranch, player: i32)
        -> Result<(), Error>;
    fn report(&mut self, state: &mut LoadedState, event: u32, player: i32) -> Result<(), Error>;
    /// Exact EditBuildQueue archive/dirty effects, before its mutable exposure.
    fn edit_build_queue(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), Error>;
}

/// Complete Game.Update, when an implementation exists. This has no default:
/// skipped upkeep or a player choice cannot be represented by successful return.
pub trait GameUpdate: Effects {
    fn game_update(&mut self, state: &mut LoadedState) -> Result<(), Error>;
}

/// One exclusive logical Game and Player Manager; no native pointers or copied
/// completion flags. Controls must be supplied; wire membership is insufficient.
pub struct Context<'a, E> {
    pub state: &'a mut LoadedState,
    pub effects: &'a mut E,
}

fn missing(_: impl std::fmt::Debug) -> Error {
    Error::Unsupported("loaded turn-completion current state unavailable")
}

impl<E: Effects> Context<'_, E> {
    /// Only the message prefix. Its RequiresGameUpdate result is a mandatory
    /// INTERNAL continuation, never a yielded player decision or accepted turn.
    pub fn execute(&mut self, player: i32, reason: u32) -> Result<core::Prefix<i32>, Error> {
        core::execute_prefix(self, player, reason)
    }
    fn cities(&self, owner: i32) -> Result<&crate::core_player::Cities, Error> {
        self.state.world.player(owner).map_err(missing)?;
        let cities = &self
            .state
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("loaded turn-completion city owner"))?
            .cities;
        // The wire preserves native live-chain traversal order, not sorted IDs.
        // Reject malformed aliases instead of inventing an iteration identity.
        for (n, entry) in cities.records.entries.iter().enumerate() {
            if entry.index < 0
                || entry.index as u32 >= cities.records.capacity
                || (n > 0 && cities.records.entries[n - 1].index >= entry.index)
            {
                return Err(Error::Unsupported(
                    "loaded turn-completion city slot metadata",
                ));
            }
        }
        Ok(cities)
    }
    fn cursor(&self, owner: i32, from: u32) -> Result<(i32, u32), Error> {
        let length = self.cities(owner)?.records.entries.len();
        let length = u32::try_from(length).map_err(|_| Error::Unsupported("city cursor size"))?;
        Ok((owner, from.min(length)))
    }
}

impl<E: Effects> World for Context<'_, E> {
    type Player = i32;
    type Game = ();
    type PlayerManager = ();
    type Cities = i32;
    type City = (i32, u32);
    type Cursor = (i32, u32);
    fn resolve_player(&mut self, owner: i32) -> Result<i32, Error> {
        self.state.world.player(owner).map_err(missing)?;
        Ok(owner)
    }
    fn game_instance(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn player_manager(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn player_id(&mut self, owner: i32) -> Result<usize, Error> {
        usize::try_from(owner)
            .ok()
            .filter(|&i| i < 64)
            .ok_or(Error::Unsupported("completion player index"))
    }
    fn is_human(&mut self, owner: i32) -> Result<bool, Error> {
        Ok(self.state.player_slot_status(owner).map_err(missing)? == 3)
    }
    fn is_alive(&mut self, owner: i32) -> Result<bool, Error> {
        self.state.player_alive(owner).map_err(missing)
    }
    fn is_turn_active(&mut self, owner: i32) -> Result<bool, Error> {
        self.state.turn_active(owner).map_err(missing)
    }
    fn is_paused(&mut self, _: ()) -> Result<bool, Error> {
        Ok(self.state.pause_storage().map_err(missing)? != 0)
    }
    fn completion_flag(&mut self, id: usize) -> Result<bool, Error> {
        self.state
            .retained
            .core
            .player_processor
            .player_complete
            .get(id)
            .copied()
            .ok_or(Error::Unsupported("completion player index"))
    }
    fn set_completion_flag(&mut self, id: usize, complete: bool) -> Result<(), Error> {
        if id >= 64 {
            return Err(Error::Unsupported("completion player index"));
        }
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(Error::Unsupported("loaded revision exhausted"))?;
        self.state.retained.core.player_processor.player_complete[id] = complete;
        self.state.revision = revision;
        Ok(())
    }
    fn is_valid_slot(&mut self, _: (), slot: i32) -> Result<bool, Error> {
        self.state.world.player_is_valid(slot).map_err(missing)
    }
    fn log(&mut self, branch: LogBranch, owner: i32) -> Result<(), Error> {
        self.effects.log(self.state, branch, owner)
    }
    fn report(&mut self, event: u32, owner: i32) -> Result<(), Error> {
        self.effects.report(self.state, event, owner)
    }
    fn completion_cities(&mut self, owner: i32) -> Result<i32, Error> {
        self.cities(owner)?;
        Ok(owner)
    }
    fn cities_player(&mut self, owner: i32) -> Result<i32, Error> {
        Ok(owner)
    }
    fn cities_begin(&mut self, owner: i32) -> Result<Self::Cursor, Error> {
        self.cursor(owner, 0)
    }
    fn cities_end(&mut self, owner: i32) -> Result<Self::Cursor, Error> {
        self.cursor(owner, u32::MAX)
    }
    fn cursor_ne(&mut self, a: Self::Cursor, b: Self::Cursor) -> Result<bool, Error> {
        Ok(a != b)
    }
    fn find_cursor_city(&mut self, owner: i32, cursor: Self::Cursor) -> Result<Self::City, Error> {
        if owner != cursor.0 {
            return Err(Error::Unsupported("city cursor receiver"));
        }
        let id = self
            .cities(owner)?
            .records
            .entries
            .get(cursor.1 as usize)
            .ok_or(Error::Unsupported("city cursor invalidated"))?
            .value
            .header
            .id;
        self.state.city(owner, id).map_err(missing)?;
        Ok((owner, id))
    }
    fn next_city(&mut self, cursor: Self::Cursor) -> Result<Self::Cursor, Error> {
        self.cursor(
            cursor.0,
            cursor
                .1
                .checked_add(1)
                .ok_or(Error::Unsupported("city cursor overflow"))?,
        )
    }
    fn build_queue(&mut self, city: Self::City) -> Result<&BuildQueue, Error> {
        Ok(&self
            .state
            .city(city.0, city.1)
            .map_err(missing)?
            .build_queue)
    }
    fn edit_build_queue(&mut self, city: Self::City) -> Result<&mut BuildQueue, Error> {
        self.effects.edit_build_queue(self.state, city.0, city.1)?;
        self.state.city(city.0, city.1).map_err(missing)?;
        let revision = self
            .state
            .revision
            .checked_add(1)
            .ok_or(Error::Unsupported("loaded revision exhausted"))?;
        let player = self
            .state
            .retained
            .players
            .iter_mut()
            .find(|p| p.header.owner == city.0)
            .ok_or(Error::Unsupported("city owner"))?;
        let entry = player
            .cities
            .records
            .entries
            .iter_mut()
            .find(|e| e.value.header.owner == city.0 && e.value.header.id == city.1)
            .ok_or(Error::Unsupported("city identity"))?;
        self.state.revision = revision;
        Ok(&mut entry.value.build_queue)
    }
}

impl<E: GameUpdate> core::GameUpdate for Context<'_, E> {
    fn game_update(&mut self, _: ()) -> Result<(), Error> {
        self.effects.game_update(self.state)
    }
}
