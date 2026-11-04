//! Player.Techs.CalculateScienceYield ordinary current-world path.
//! Numeric constituent queries preserve source order and only request positive
//! coefficient continuations. Text labels do not replace gameplay queries.
use super::*;
fn tuple_value<E>(
    lookup: &LinuxDefinitionRegistry,
    pairs: &[(u32, i32)],
    index: usize,
    name: &'static str,
) -> Result<i32, Error<E>> {
    pairs
        .iter()
        .rfind(|pair| lookup.find_index(pair.0) == Some(index))
        .map(|pair| pair.1)
        .ok_or(Error::Missing(name))
}
fn hash_value<E>(
    lookup: &LinuxDefinitionRegistry,
    pairs: &[c6_core::unit::HashValue<i32>],
    index: usize,
    name: &'static str,
) -> Result<i32, Error<E>> {
    pairs
        .iter()
        .rfind(|pair| lookup.find_index(pair.definition_hash) == Some(index))
        .map(|pair| pair.value)
        .ok_or(Error::Missing(name))
}
fn product(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
impl<E: Effects> Context<'_, E> {
    fn player(
        &self,
        owner: i32,
    ) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E::Error>> {
        self.state.world().player(owner).map_err(Error::World)?;
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("current research Player"))
    }
    pub(super) fn has_current_tech(
        &self,
        player: i32,
        index: i32,
    ) -> Result<bool, Error<E::Error>> {
        let Some(definition) = self.definitions.techs.get(index as u32 as usize) else {
            return Ok(false);
        };
        if !self.globally_available(Kind::Technology, index)? {
            return Ok(false);
        }
        self.state
            .research_components(player)
            .map_err(Error::State)?
            .0
            .has_tech
            .pairs
            .iter()
            .rfind(|pair| pair.definition_hash == definition.hash)
            .map(|pair| pair.value)
            .ok_or(Error::Missing("current acquired Tech flag"))
    }
    fn science_alliance_modifier(&self, owner: i32) -> Result<i32, Error<E::Error>> {
        let ids = self
            .effects
            .runtime()
            .alive_major_ids
            .as_ref()
            .ok_or(Error::Missing("current GetAliveMajorIDs order"))?;
        let mut ally = None;
        for &other in ids {
            if other == owner {
                continue;
            }
            if self
                .state
                .game_relations()
                .map_err(Error::State)?
                .alliance_type(owner, other)
                .map_err(Error::Relations)?
                == self.definitions.research_alliance
            {
                ally = Some(other);
                break;
            }
        }
        let Some(other) = ally else { return Ok(0) };
        let coefficient = self.player(owner)?.diplomacy.alliance_and_favor_modifiers[0];
        if coefficient == 0 {
            return Ok(0);
        }
        if !self
            .state
            .world()
            .player_is_valid(other)
            .map_err(Error::World)?
        {
            return Ok(0);
        }
        let own = research_turn::current(self, owner, Kind::Technology)?;
        if own != -1 && self.has_current_tech(other, own)? {
            return Ok(coefficient.wrapping_shl(8));
        }
        let other_current = research_turn::current(self, other, Kind::Technology)?;
        if other_current == research_turn::current(self, owner, Kind::Technology)? {
            return Ok(coefficient.wrapping_shl(8));
        }
        if research_turn::current(self, other, Kind::Technology)? != -1
            && self.has_current_tech(
                owner,
                research_turn::current(self, other, Kind::Technology)?,
            )?
        {
            return Ok(coefficient.wrapping_shl(8));
        }
        Ok(0)
    }
    fn diplomacy_yield(
        &self,
        owner: i32,
        index: usize,
        embassy: bool,
    ) -> Result<i32, Error<E::Error>> {
        let diplomacy = &self.player(owner)?.diplomacy;
        let values = if embassy {
            &diplomacy.yield_per_embassy
        } else {
            &diplomacy.yield_per_delegation
        };
        let coefficient = hash_value(
            &self.definitions.yields,
            values,
            index,
            "current delegation/embassy yield vector",
        )?;
        if coefficient <= 0 {
            return Ok(0);
        }
        let majors = self
            .effects
            .runtime()
            .alive_majors
            .as_ref()
            .ok_or(Error::Missing("current GetAliveMajors order"))?;
        let mut count = 0_i32;
        for &other in majors {
            if other == owner {
                continue;
            }
            let diplomacy = &self.player(other)?.diplomacy;
            let flags = if embassy {
                &diplomacy.has_embassy
            } else {
                &diplomacy.has_delegation
            };
            if *flags
                .get(owner as u32 as usize)
                .ok_or(Error::Missing("diplomatic recipient index"))?
            {
                count = count.wrapping_add(1);
            }
        }
        Ok(count.wrapping_mul(coefficient).wrapping_shl(8))
    }
    pub fn science_yield(&mut self, owner: i32) -> Result<i32, Error<E::Error>> {
        self.check()?;
        if self
            .state
            .research_components(owner)
            .map_err(Error::State)?
            .1
            .anarchy_end_turn
            >= 0
        {
            return Ok(0);
        }
        let index = self
            .definitions
            .yields
            .find_index(linux_type_hash(b"YIELD_SCIENCE"))
            .ok_or(Error::Missing("YIELD_SCIENCE definition"))?;
        let city_count = self.player(owner)?.cities.records.entries.len();
        let mut base = 0_i32;
        if city_count != 0 {
            let order = self
                .effects
                .runtime()
                .city_order
                .iter()
                .find(|row| row.0 == owner)
                .ok_or(Error::Missing("current research City iteration order"))?
                .1
                .clone();
            if order.len() != city_count {
                return Err(Error::Missing("current research City iteration extent"));
            }
            for city in order {
                if i32::from(city.owner) != owner
                    || !self
                        .player(owner)?
                        .cities
                        .records
                        .entries
                        .iter()
                        .any(|entry| entry.value.header.id == city.id)
                {
                    return Err(Error::Missing("current research City identity"));
                }
                base = base.wrapping_add(self.query(Query::CityYield {
                    city,
                    yield_index: index as i32,
                })?);
            }
        }
        let belief_count = self.player(owner)?.religion.belief_yields.len();
        for record in 0..belief_count {
            let belief = self.player(owner)?.religion.belief_yields[record].clone();
            if belief.yield_ordinal != index as i32 {
                continue;
            }
            let items = if belief.belief_yield_ordinal as u32 > 5 {
                0
            } else {
                self.query(Query::BeliefItemCount {
                    player: owner,
                    record,
                })?
            };
            // Native constructs count*amount as a whole integer before Q8 division.
            if belief.per_x_items <= 0 {
                continue;
            }
            let numerator = items.wrapping_mul(belief.amount).wrapping_shl(8);
            let denominator = belief.per_x_items.wrapping_shl(8);
            if denominator == 0 {
                return Err(Error::Missing("native belief yield division by zero"));
            }
            let amount = ((i64::from(numerator) << 8) / i64::from(denominator)) as i32;
            base = base.wrapping_add(amount);
        }
        let given = tuple_value(
            &self.definitions.yields,
            &self.player(owner)?.influence.yield_per_given_token,
            index,
            "current Influence given-token yield vector",
        )?;
        if given > 0 {
            let count = self.query(Query::CountTokensGiven(owner))?;
            if count > 0 {
                base = base.wrapping_add(product(given, count.wrapping_shl(8)));
            }
        }
        let tributary = tuple_value(
            &self.definitions.yields,
            &self.player(owner)?.influence.yield_per_tributary,
            index,
            "current Influence tributary yield vector",
        )?;
        if tributary > 0 {
            let count = self.query(Query::CountTributaries(owner))?;
            if count > 0 {
                base = base.wrapping_add(product(tributary, count.wrapping_shl(8)));
            }
        }
        base = base.wrapping_add(self.diplomacy_yield(owner, index, false)?);
        base = base.wrapping_add(self.diplomacy_yield(owner, index, true)?);
        base = base.wrapping_add(
            self.state
                .research_components(owner)
                .map_err(Error::State)?
                .0
                .science_rate_change_q8,
        );
        let mut modifier = 0_i32;
        let tributary_modifier = tuple_value(
            &self.definitions.yields,
            &self.player(owner)?.influence.yield_modifier_per_tributary,
            index,
            "current Influence tributary modifier vector",
        )?;
        if tributary_modifier > 0 {
            let count = self.query(Query::CountTributaries(owner))?;
            if count > 0 {
                modifier =
                    modifier.wrapping_add(product(tributary_modifier, count.wrapping_shl(8)));
            }
        }
        let great_people = tuple_value(
            &self.definitions.yields,
            &self
                .player(owner)?
                .great_people_points
                .yield_modifier_per_earned_great_person,
            index,
            "current GreatPeople yield modifier vector",
        )?;
        if great_people > 0 {
            let count = self.query(Query::EarnedGreatPeople(owner))?;
            modifier = modifier.wrapping_add(product(great_people, count.wrapping_shl(8)));
        }
        modifier = modifier.wrapping_add(self.science_alliance_modifier(owner)?);
        Ok(research_turn::yield_attribute_value(base, modifier))
    }
}
