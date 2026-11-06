//! Culture.DoCulture in native root order. Current positive city tourism,
//! contacted-opponent tourism and anarchy-government replacement remain
//! explicit continuations; missing inputs are queried lazily.
use super::*;
impl<E: Effects> Context<'_, E> {
    fn turn_major(&self, owner: i32) -> Result<bool, Error<E::Error>> {
        self.effects
            .runtime()
            .civilization_levels
            .iter()
            .find(|r| r.0 == owner)
            .map(|r| r.1 == 0x253718b0)
            .ok_or(Error::Missing("current civilization level"))
    }
    fn turn_age(&mut self, owner: i32, golden: bool) -> Result<bool, Error<E::Error>> {
        match self.query(Query::PlayerAge { owner, golden })? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Missing("noncanonical current age flag")),
        }
    }
    fn culture_turn_culture(&self, owner: i32) -> Result<&Culture, Error<E::Error>> {
        self.state
            .research_components(owner)
            .map(|r| r.1)
            .map_err(Error::State)
    }
    fn culture_turn_culture_mut(&mut self, owner: i32) -> Result<&mut Culture, Error<E::Error>> {
        self.state
            .research_components_mut(owner)
            .map(|r| r.1)
            .map_err(Error::State)
    }
    fn turn_majors(&self) -> Result<Vec<i32>, Error<E::Error>> {
        self.effects
            .runtime()
            .alive_majors
            .clone()
            .ok_or(Error::Missing("current GetAliveMajors order"))
    }
    fn current_tourists_from(&self, owner: i32, other: i32) -> Result<i32, Error<E::Error>> {
        self.culture_turn_culture(owner)?
            .tourists_from
            .get(other as u32 as usize)
            .copied()
            .ok_or(Error::Missing("current tourists-from cell"))
    }
    fn current_tourists_to(&self, owner: i32) -> Result<i32, Error<E::Error>> {
        let mut total = 0_i32;
        for other in self.turn_majors()? {
            if other != owner
                && self
                    .state
                    .game_relations()
                    .map_err(Error::State)?
                    .has_met(owner, other)
                    .map_err(Error::Relations)?
            {
                total = total.wrapping_add(self.current_tourists_from(owner, other)?);
            }
        }
        Ok(total)
    }
    fn record_tourism(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        let cities = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("current Culture player"))?
            .cities
            .records
            .entries
            .iter()
            .map(|c| c.value.header.id)
            .collect::<Vec<_>>();
        let definitions = &self.definitions.city_tourism;
        for city in cities {
            definitions.record(self, owner, city)?;
        }
        match self.query(Query::NationalParkCount)? {
            0 => Ok(()),
            _ => Err(Error::Missing(
                "current NationalParks.RecordTourismFromParks",
            )),
        }
    }
    fn apply_tourism(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        // GetReligionTypeCreated's unused result is omitted: its only edit is
        // archive transport bookkeeping (the shared religion lookup closure).
        for other in self.turn_majors()? {
            // Player.Diplomacy.HasMet rejects self before GameRelations.HasMet.
            if other != owner
                && self
                    .state
                    .game_relations()
                    .map_err(Error::State)?
                    .has_met(owner, other)
                    .map_err(Error::Relations)?
            {
                return Err(Error::Missing(
                    "current contacted-player tourism application",
                ));
            }
        }
        Ok(())
    }
    fn distribute_tourists(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        if !self
            .culture_turn_culture(owner)?
            .tourism_locations
            .is_empty()
        {
            return Err(Error::Missing("current tourism-location distribution"));
        }
        // Native still fills and sums its temporary player counts before the
        // zero total-location-weight guard; no sort/allocation/RNG follows it.
        let mut total = 0_i32;
        for other in self.turn_majors()? {
            if other != owner && self.state.player_alive(other).map_err(Error::State)? {
                total = total.wrapping_add(self.current_tourists_from(owner, other)?);
            }
        }
        let _ = total;
        Ok(())
    }
    fn check_dominance(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        let _tourists = self.current_tourists_to(owner)?;
        let team = self.state.player_team(owner).map_err(Error::State)?;
        let mut warning_excluded = false;
        for other in self.turn_majors()? {
            let other_team = self.state.player_team(other).map_err(Error::State)?;
            if other_team != team {
                if self
                    .state
                    .game_relations()
                    .map_err(Error::State)?
                    .has_met(owner, other)
                    .map_err(Error::Relations)?
                {
                    return Err(Error::Missing(
                        "current contacted-player cultural dominance",
                    ));
                }
                // This mandatory -1 makes GetTurnsUntilVictory return -1,
                // excluding its presentation warning under either victory gate.
                warning_excluded = true;
            }
            *self
                .culture_turn_culture_mut(owner)?
                .dominant_over
                .get_mut(other as u32 as usize)
                .ok_or(Error::Missing("current dominant-over cell"))? = false;
            *self
                .culture_turn_culture_mut(owner)?
                .turns_until_dominant
                .get_mut(other as u32 as usize)
                .ok_or(Error::Missing("current turns-until-dominant cell"))? = -1;
        }
        if !warning_excluded {
            return Err(Error::Missing(
                "current cultural-victory warning predicates",
            ));
        }
        Ok(())
    }
    fn obsolete_policies(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        let _dark = self.turn_age(owner, false)?;
        let _golden = self.turn_age(owner, true)?;
        let mut index = 0;
        while index
            < self
                .culture_turn_culture(owner)?
                .active_policies
                .entries
                .len()
        {
            let hash = self.culture_turn_culture(owner)?.active_policies.entries[index].policy_hash;
            if let Some(policy) = self.definitions.policies.find_index(hash) {
                let culture = self.culture_turn_culture(owner)?;
                let active = culture
                    .policy_is_in_place
                    .pairs
                    .iter()
                    .rfind(|p| p.definition_hash == hash)
                    .ok_or(Error::Missing("current policy active flag"))?
                    .value;
                let definitions = self.definitions;
                if definitions
                    .policy_upkeep
                    .obsolete(hash, active, |replacement| {
                        match self.query(Query::PolicyUnlocked {
                            owner,
                            hash: replacement,
                        })? {
                            0 => Ok(false),
                            1 => Ok(true),
                            _ => Err(Error::Missing("noncanonical current policy unlock")),
                        }
                    })?
                {
                    // SetPolicyActive(false) and the slot store precede the
                    // distinct PolicyObsoleted signal. No policy is selected,
                    // enacted, or marked as an external policy-change command.
                    self.culture_turn_culture_mut(owner)?
                        .policy_is_in_place
                        .pairs
                        .iter_mut()
                        .rfind(|p| p.definition_hash == hash)
                        .ok_or(Error::Missing("current policy active flag"))?
                        .value = false;
                    self.culture_turn_culture_mut(owner)?
                        .active_policies
                        .entries[index]
                        .policy_hash = u32::MAX;
                    self.emit(Effect::PolicyObsoleted {
                        owner,
                        policy: policy as i32,
                        hash,
                    })?;
                    self.emit(Effect::Report(boost::Report {
                        event: 0xd127ff37,
                        specialization: None,
                        player: owner,
                        index: Some(policy as i32),
                        old_progress: None,
                        source: None,
                    }))?;
                }
                // Native still tests the ORIGINAL card's XP1 row after its
                // obsolete removal and every synchronous consumer completed.
                self.definitions.policy_upkeep.check_age(hash)?;
            }
            index += 1;
        }
        Ok(())
    }
    /// Full native DoCulture root ordering; an error retains its completed
    /// canonical prefix and is not a successful culture activation.
    pub fn do_culture(&mut self, owner: i32) -> Result<(), Error<E::Error>> {
        self.check()?;
        if self.turn_major(owner)? {
            let previous = self.culture_turn_culture(owner)?.staycationers;
            self.culture_turn_culture_mut(owner)?
                .last_turn_staycationers = previous;
        }
        self.culture_turn_culture_mut(owner)?
            .civic_completed_this_turn_hash = u32::MAX;
        self.culture_turn_culture_mut(owner)?
            .government_change_considered = false;
        let anarchy = self.culture_turn_culture(owner)?.anarchy_end_turn;
        if anarchy >= 0 && self.state.retained().core.current_turn >= anarchy {
            return Err(Error::Missing(
                "current EndAnarchy and government replacement",
            ));
        }
        let culture = self.culture_yield(owner)?;
        let amount = culture.wrapping_add(self.culture_turn_culture(owner)?.cultural_overflow_bits);
        self.culture_turn_culture_mut(owner)?.cultural_overflow_bits = 0;
        let civic = research_turn::current(self, owner, Kind::Civic)?;
        if civic != -1 {
            boost::change_research_progress(self, owner, Kind::Civic, civic, amount)
                .map_err(|e| Error::Kernel(Box::new(e)))?;
        } else {
            self.culture_turn_culture_mut(owner)?
                .civic_completed_this_turn = true;
            self.culture_turn_culture_mut(owner)?.cultural_overflow_bits = amount;
        }
        if self.turn_major(owner)? {
            self.record_tourism(owner)?;
            self.apply_tourism(owner)?;
            self.distribute_tourists(owner)?;
            self.check_dominance(owner)?;
            let tourists = self.current_tourists_to(owner)?;
            self.culture_turn_culture_mut(owner)?.last_turn_tourists_to = tourists;
        }
        if !self.culture_turn_culture(owner)?.civic_completed_this_turn {
            if self.turn_age(owner, false)?
                && self.query(Query::CurrentEraStartTurn)?
                    == self.state.retained().core.current_turn
            {
                self.culture_turn_culture_mut(owner)?
                    .civic_completed_this_turn = true;
            }
            if !self.culture_turn_culture(owner)?.civic_completed_this_turn
                && self.turn_age(owner, true)?
                && self.query(Query::CurrentEraStartTurn)?
                    == self.state.retained().core.current_turn
            {
                self.culture_turn_culture_mut(owner)?
                    .civic_completed_this_turn = true;
            }
        }
        self.obsolete_policies(owner)?;
        self.emit(Effect::GovernmentNotifications(owner))?;
        self.emit(Effect::ChooseCivic(owner))?;
        self.emit(Effect::Report(boost::Report {
            event: 0x80f1366d,
            specialization: None,
            player: owner,
            index: None,
            old_progress: None,
            source: None,
        }))
    }
}
