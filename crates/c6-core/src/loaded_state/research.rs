//! Revision-tracked canonical Player.Techs and Player.Culture access.
use super::*;
impl LoadedState {
    pub fn research_components(
        &self,
        owner: i32,
    ) -> Result<(&crate::player_techs::Techs, &crate::player_culture::Culture), Error> {
        self.world.player(owner).map_err(Error::World)?;
        let player = self
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        Ok((&player.techs, &player.culture))
    }
    /// Reserve the revision only after validating the owner. These are the
    /// canonical components, shared by current queries and subsequent effects.
    pub fn research_components_mut(
        &mut self,
        owner: i32,
    ) -> Result<
        (
            &mut crate::player_techs::Techs,
            &mut crate::player_culture::Culture,
        ),
        Error,
    > {
        self.world.player(owner).map_err(Error::World)?;
        let index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.revision = next;
        let player = &mut self.retained.players[index];
        Ok((&mut player.techs, &mut player.culture))
    }
}

#[derive(Debug)]
pub enum WriteError {
    State(super::Error),
    World(crate::world::MissingInput),
    MissingFlag {
        player: i32,
        hash: u32,
    },
    MissingIndexedFlag {
        player: i32,
        kind: crate::research_acquisition::Kind,
        index: i32,
    },
}
impl LoadedState {
    /// Both projections represent the same native acquired byte. Validate both
    /// locations and reserve one revision before either projection changes.
    pub fn set_research_acquired(
        &mut self,
        owner: i32,
        kind: crate::research_acquisition::Kind,
        index: i32,
        hash: u32,
        value: bool,
    ) -> Result<(), WriteError> {
        use crate::research_acquisition::Kind;
        let player = self.world.player(owner).map_err(WriteError::World)?;
        let indexed = match kind {
            Kind::Technology => &player.technologies,
            Kind::Civic => &player.civics,
        };
        if indexed
            .as_ref()
            .and_then(|v| v.get(index as u32 as usize))
            .is_none()
        {
            return Err(WriteError::MissingIndexedFlag {
                player: owner,
                kind,
                index,
            });
        }
        let (techs, culture) = self.research_components(owner).map_err(WriteError::State)?;
        let pairs = match kind {
            Kind::Technology => &techs.has_tech.pairs,
            Kind::Civic => &culture.has_civic.pairs,
        };
        let slot = pairs
            .iter()
            .rposition(|pair| pair.definition_hash == hash)
            .ok_or(WriteError::MissingFlag {
                player: owner,
                hash,
            })?;
        let retained_index = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .expect("checked retained research owner");
        let next = self
            .revision
            .checked_add(1)
            .ok_or(WriteError::State(super::Error::ResourceLimit))?;
        self.revision = next;
        let player = self
            .world
            .player_mut(owner)
            .expect("checked World research owner");
        let indexed = match kind {
            Kind::Technology => &mut player.technologies,
            Kind::Civic => &mut player.civics,
        };
        indexed.as_mut().expect("checked acquired array")[index as usize] = value;
        let player = &mut self.retained.players[retained_index];
        match kind {
            Kind::Technology => player.techs.has_tech.pairs[slot].value = value,
            Kind::Civic => player.culture.has_civic.pairs[slot].value = value,
        };
        Ok(())
    }
}

impl LoadedState {
    /// Install a complete current graph owner, not a serialized approximation.
    /// Existing graph/current availability conflicts reject without mutation.
    /// Source authentication is the importing adapter's responsibility.
    pub fn install_current_game_techs(
        &mut self,
        binding: &WireBinding,
        graph: crate::game_techs::Graph,
        availability: Vec<bool>,
    ) -> Result<(), Error> {
        if binding != &self.binding
            || self.game_techs.is_some()
            || self
                .world
                .technology_availability
                .as_ref()
                .is_some_and(|old| old != &availability)
        {
            return Err(Error::Binding);
        }
        let next = self
            .admission_generation
            .checked_add(1)
            .ok_or(Error::ResourceLimit)?;
        self.world.technology_availability = Some(availability);
        self.game_techs = Some(graph);
        self.admission_generation = next;
        Ok(())
    }
}

impl LoadedState {
    /// Admit explicitly supplied current research relation fields. Other
    /// diplomacy fields remain unavailable in the supplied owner.
    pub fn install_current_research_relations(
        &mut self,
        binding: &WireBinding,
        relations: crate::game_diplomatic_relations::GameDiplomaticRelations,
    ) -> Result<(), Error> {
        if binding != &self.binding || self.game_relations.is_some() {
            return Err(Error::Binding);
        }
        self.advance_admission_generation()?;
        self.game_relations = Some(relations);
        Ok(())
    }
    /// Install complete indexed aliases of current acquired flags. No absent
    /// flag receives a reset/default value, and existing aliases must agree.
    pub fn install_current_research_flags(
        &mut self,
        binding: &WireBinding,
        flags: Vec<(i32, Vec<bool>, Vec<bool>)>,
    ) -> Result<(), Error> {
        if binding != &self.binding {
            return Err(Error::Binding);
        }
        let mut seen = std::collections::BTreeSet::new();
        for (owner, techs, civics) in &flags {
            if !seen.insert(*owner) {
                return Err(Error::Binding);
            }
            let player = self.world.player(*owner).map_err(Error::World)?;
            if player.technologies.as_ref().is_some_and(|old| old != techs)
                || player.civics.as_ref().is_some_and(|old| old != civics)
            {
                return Err(Error::Binding);
            }
        }
        self.advance_admission_generation()?;
        for (owner, techs, civics) in flags {
            let player = self.world.player_mut(owner).map_err(Error::World)?;
            player.technologies = Some(techs);
            player.civics = Some(civics);
        }
        Ok(())
    }
}

impl LoadedState {
    /// Initial current Game.Culture active-node projection. Acquisition changes
    /// local acquired bytes, not this global node availability mask.
    pub fn install_current_civic_availability(
        &mut self,
        binding: &WireBinding,
        availability: Vec<bool>,
    ) -> Result<(), Error> {
        if binding != &self.binding
            || self
                .world
                .civic_availability
                .as_ref()
                .is_some_and(|old| old != &availability)
        {
            return Err(Error::Binding);
        }
        self.advance_admission_generation()?;
        self.world.civic_availability = Some(availability);
        Ok(())
    }
}
