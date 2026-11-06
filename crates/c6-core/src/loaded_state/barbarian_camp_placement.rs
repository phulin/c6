//! Canonical camp manager mutations and the unsaved native local-food field.
use super::{Error, LoadedState};
impl LoadedState {
    pub fn barbarian_manager_mut(&mut self) -> Result<&mut crate::game_barbarian::Manager, Error> {
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.barbarian)
    }
    /// None preserves the absence of a live capture for imported instances.
    pub fn barbarian_local_food(&self, index: usize) -> Option<i32> {
        self.barbarian_local_food
            .iter()
            .find(|&&(i, _)| i == index)
            .map(|&(_, food)| food)
    }
    pub fn set_barbarian_local_food(&mut self, index: usize, food: i32) -> Result<(), Error> {
        if index >= self.retained.barbarian.instances.len() {
            return Err(Error::ResourceLimit);
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        if let Some((_, value)) = self
            .barbarian_local_food
            .iter_mut()
            .find(|(i, _)| *i == index)
        {
            *value = food;
        } else {
            self.barbarian_local_food.push((index, food));
        }
        Ok(())
    }
}
impl LoadedState {
    /// AddImprovement (0x2b1d000) appends only a nonnegative, absent plot index.
    /// The separately retained improvement_array_size is not changed here.
    pub fn add_player_improvement(&mut self, owner: i32, plot: i32) -> Result<(), Error> {
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(super::MissingInput::Player(owner)))?;
        if plot < 0
            || self.retained.players[player]
                .improvements
                .plot_indices
                .contains(&plot)
        {
            return Ok(());
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.retained.players[player]
            .improvements
            .plot_indices
            .push(plot);
        Ok(())
    }
}

impl LoadedState {
    /// RemoveImprovement2b1d090 erases all matches in order; size metadata stays.
    pub fn remove_player_improvement(&mut self, owner: i32, plot: i32) -> Result<(), Error> {
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(super::MissingInput::Player(owner)))?;
        if plot < 0 {
            return Ok(());
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.retained.players[player]
            .improvements
            .plot_indices
            .retain(|&p| p != plot);
        Ok(())
    }
}
