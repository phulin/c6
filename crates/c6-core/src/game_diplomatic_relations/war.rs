//! Storage leaves of Linux DiplomaticRelations.SetAtWar2984860.
//! Contact, logging, denouncements, expulsion and visibility are ordered by the
//! action caller. These setters alone never represent a completed declaration.
use super::*;

impl GameDiplomaticRelations {
    pub fn initialize_current_war_aggressors(
        &mut self,
        cells: Vec<(i32, i32)>,
    ) -> Result<(), Error> {
        if cells.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        let (values, changed_turns) = cells.into_iter().unzip();
        let rows = Relationships {
            values,
            changed_turns: Some(changed_turns),
        };
        if self
            .current_war_aggressors
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_1b8))
            .is_some_and(|old| old != &rows)
        {
            return Err(Error::AlreadyInitialized);
        }
        self.current_war_aggressors = Some(rows);
        Ok(())
    }
    pub fn initialize_current_defensive_pacts(
        &mut self,
        cells: Vec<BoolRelationship>,
    ) -> Result<(), Error> {
        if cells.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        if self
            .current_defensive_pacts
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_140))
            .is_some_and(|old| old != &cells)
        {
            return Err(Error::AlreadyInitialized);
        }
        self.current_defensive_pacts = Some(cells);
        Ok(())
    }
    pub fn is_in_defensive_pact(&self, first: i32, second: i32) -> Result<bool, Error> {
        if first == second {
            return Ok(false);
        }
        let cells = self
            .current_defensive_pacts
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_140))
            .ok_or(Error::MissingDefensivePacts)?;
        cells
            .get(self.index(first, second)?)
            .map(|c| c.value)
            .ok_or(Error::Index)
    }
    pub fn war_aggressor(&self, first: i32, second: i32) -> Result<i32, Error> {
        if first == second {
            return Ok(-1);
        }
        self.value(
            self.current_war_aggressors
                .as_ref()
                .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_1b8))
                .ok_or(Error::MissingWarAggressors)?,
            first,
            second,
        )
    }
    /// Mutate both current payloads in source order, including changed turns.
    /// Missing storage is checked before either write. This is also the peace
    /// storage operation; executing peace gameplay requires its own caller.
    pub fn set_at_war_fields(
        &mut self,
        first: i32,
        second: i32,
        kind: i32,
        turn: i32,
    ) -> Result<(), Error> {
        if first == second {
            return Err(Error::Index);
        }
        let index = self.index(first, second)?;
        let war = self.war.as_ref().ok_or(Error::MissingWar)?;
        if war.values.get(index).is_none()
            || war
                .changed_turns
                .as_ref()
                .and_then(|t| t.get(index))
                .is_none()
        {
            return Err(Error::MissingChangedTurns);
        }
        let aggressor = self
            .current_war_aggressors
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_1b8))
            .ok_or(Error::MissingWarAggressors)?;
        if aggressor.values.get(index).is_none()
            || aggressor
                .changed_turns
                .as_ref()
                .and_then(|t| t.get(index))
                .is_none()
        {
            return Err(Error::MissingChangedTurns);
        }
        let war = self.war.as_mut().expect("checked war");
        war.values[index] = kind;
        war.changed_turns.as_mut().expect("checked war turns")[index] = turn;
        let aggressor = match &mut self.current_war_aggressors {
            Some(rows) => rows,
            None => {
                &mut self
                    .read_prefix
                    .as_mut()
                    .expect("checked aggressor prefix")
                    .at_1b8
            }
        };
        aggressor
            .changed_turns
            .as_mut()
            .expect("checked aggressor turns")[index] = turn;
        aggressor.values[index] = if kind == -1 { -1 } else { first };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn relations() -> GameDiplomaticRelations {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            3,
            vec![(-1, -1); 3],
            vec![(-1, -1); 3],
        )
        .unwrap();
        r.initialize_current_war(vec![(-1, -1); 3]).unwrap();
        r
    }
    #[test]
    fn missing_aggressor_storage_preserves_war_and_changed_turn() {
        let mut r = relations();
        assert_eq!(
            r.set_at_war_fields(0, 2, 7, 12),
            Err(Error::MissingWarAggressors)
        );
        assert_eq!(r.war_type(0, 2), Ok(-1));
        assert_eq!(r.war_changed_turn(0, 2), Ok(-1));
    }
    #[test]
    fn symmetric_war_retains_directional_aggressor_and_source_turn() {
        let mut r = relations();
        r.initialize_current_war_aggressors(vec![(-1, -1); 3])
            .unwrap();
        r.set_at_war_fields(2, 0, 7, i32::MAX).unwrap();
        assert_eq!(r.war_type(0, 2), Ok(7));
        assert_eq!(r.war_aggressor(0, 2), Ok(2));
        assert_eq!(r.war_changed_turn(0, 2), Ok(i32::MAX));
        assert_eq!(r.war_type(0, 1), Ok(-1));
        r.set_at_war_fields(0, 2, -1, i32::MIN).unwrap();
        assert_eq!(r.war_aggressor(2, 0), Ok(-1));
        assert_eq!(r.war_changed_turn(0, 2), Ok(i32::MIN));
    }
}
