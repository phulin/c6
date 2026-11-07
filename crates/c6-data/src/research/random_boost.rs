//! GrantRandomBoostGoodyHut shares the canonical research and GameSync owners.
use super::*;

impl<E: Effects> Context<'_, E> {
    fn random_boost_flag(
        &self,
        player: i32,
        kind: Kind,
        index: i32,
        boosted: bool,
    ) -> Result<bool, Error<E::Error>> {
        // Native HasTech/HasCivic and HasBoostTriggered return false for an
        // unavailable global node, before accessing the player's flag array.
        if !self.globally_available(kind, index)? {
            return Ok(false);
        }
        let definitions = match kind {
            Kind::Technology => &self.definitions.techs,
            Kind::Civic => &self.definitions.civics,
        };
        let hash = definitions
            .get(index as usize)
            .ok_or(Error::Missing("random boost definition"))?
            .hash;
        let (techs, culture) = self
            .state
            .research_components(player)
            .map_err(Error::State)?;
        let flags = match (kind, boosted) {
            (Kind::Technology, false) => &techs.has_tech,
            (Kind::Technology, true) => &techs.boost_triggered,
            (Kind::Civic, false) => &culture.has_civic,
            (Kind::Civic, true) => &culture.boost_triggered,
        };
        flags
            .pairs
            .iter()
            .rfind(|p| p.definition_hash == hash)
            .map(|p| p.value)
            .ok_or(Error::Missing("random boost current flag"))
    }

    /// Native random village award: oldest eligible era, native era-reference
    /// order, then uniform draws without replacement. Never selects a queue.
    /// Earlier awards/RNG remain committed if a synchronous callback fails.
    pub fn grant_random_boost_goody_hut(
        &mut self,
        player: i32,
        kind: Kind,
        count: i32,
        source: i32,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        use boost::Error::Context as Failed;
        self.check().map_err(Failed)?;
        let iteration = match kind {
            Kind::Technology => self
                .state
                .game_techs()
                .map_err(Error::State)
                .map_err(Failed)?
                .iteration()
                .to_vec(),
            Kind::Civic => self
                .effects
                .runtime()
                .civic_graph
                .as_ref()
                .ok_or(Error::Missing("current Game.Culture graph"))
                .map_err(Failed)?
                .iteration
                .clone(),
        };
        let mut oldest = None;
        for index in iteration {
            let costs = &self.definitions.costs[usize::from(kind == Kind::Civic)];
            let Some(cost) = costs.get(index as usize) else {
                continue;
            };
            if self
                .random_boost_flag(player, kind, index, false)
                .map_err(Failed)?
                || self
                    .random_boost_flag(player, kind, index, true)
                    .map_err(Failed)?
            {
                continue;
            }
            let era = cost
                .era
                .ok_or(Error::Missing("random boost Era reference"))
                .map_err(Failed)? as usize;
            let chronology = *self
                .definitions
                .chronology
                .get(era)
                .ok_or(Error::Missing("random boost Era chronology"))
                .map_err(Failed)?;
            if oldest.is_none_or(|v| chronology < v) && self.has_random_boost(kind, index) {
                oldest = Some(chronology);
            }
        }
        let mut candidates = Vec::new();
        if let Some(oldest) = oldest {
            // Era reference SQL orders by research rowid; the canonical
            // definition collection has that same order. Equal chronology
            // eras are visited separately in era collection order.
            for (era, chronology) in self.definitions.chronology.iter().enumerate() {
                if *chronology != oldest {
                    continue;
                }
                for (index, cost) in self.definitions.costs[usize::from(kind == Kind::Civic)]
                    .iter()
                    .enumerate()
                {
                    let index = index as i32;
                    if cost.era == Some(era as i32)
                        && self.has_random_boost(kind, index)
                        && self.globally_available(kind, index).map_err(Failed)?
                        && !self
                            .random_boost_flag(player, kind, index, false)
                            .map_err(Failed)?
                        && !self
                            .random_boost_flag(player, kind, index, true)
                            .map_err(Failed)?
                    {
                        candidates.push(index);
                    }
                }
            }
        }
        for _ in 0..count {
            if candidates.is_empty() {
                break;
            }
            let roll = self
                .state
                .game_sync_rand(candidates.len() as u16)
                .map_err(Error::Random)
                .map_err(Failed)?;
            let index = candidates.remove(usize::from(roll));
            if !self
                .random_boost_flag(player, kind, index, true)
                .map_err(Failed)?
            {
                self.trigger_boost(player, kind, index, source)?;
            }
        }
        Ok(())
    }
    fn has_random_boost(&self, kind: Kind, index: i32) -> bool {
        self.definitions.boost.boosts.iter().any(|definition| {
            match kind {
                Kind::Technology => &definition.technology,
                Kind::Civic => &definition.civic,
            }
            .as_ref()
            .is_some_and(|r| r.index == index)
        })
    }
}
