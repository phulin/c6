//! Checked canonical Player religion, great-person and influence mutations.
use super::{Error, LoadedState, MissingInput};
pub struct SocialMut<'a> {
    pub religion: &'a mut crate::player_religion::Religion,
    pub great_people: &'a mut crate::player_great_people_points::GreatPeoplePoints,
    pub influence: &'a mut crate::player_influence::Influence,
}
impl LoadedState {
    pub fn social_components_mut(&mut self, owner: i32) -> Result<SocialMut<'_>, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let player = &mut self.retained.players[index];
        Ok(SocialMut {
            religion: &mut player.religion,
            great_people: &mut player.great_people_points,
            influence: &mut player.influence,
        })
    }
}
