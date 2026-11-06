//! Storage leaves of Linux DiplomaticRelations.SetAtWar2984860.
//! Contact, logging, denouncements, expulsion and visibility are ordered by the
//! action caller. These setters alone never represent a completed declaration.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrievanceLogItem {
    pub holder: i32,
    pub source: i32,
    pub turn: i32,
    pub amount: i32,
    pub reason: Vec<u8>,
    pub other_reason: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Consequences {
    tension: Relationships,
    log: Vec<GrievanceLogItem>,
}

impl GameDiplomaticRelations {
    pub fn initialize_current_war_consequences(
        &mut self,
        tension: Vec<(i32, i32)>,
        log: Vec<GrievanceLogItem>,
    ) -> Result<(), Error> {
        if tension.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        if self.current_war_consequences.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        let (values, changed_turns) = tension.into_iter().unzip();
        self.current_war_consequences = Some(Consequences {
            tension: Relationships {
                values,
                changed_turns: Some(changed_turns),
            },
            log,
        });
        Ok(())
    }
    pub fn tension(&self) -> Result<&Relationships, Error> {
        self.current_war_consequences
            .as_ref()
            .map(|c| &c.tension)
            .ok_or(Error::DefinitionInputs)
    }
    pub fn grievance_log(&self) -> Result<&[GrievanceLogItem], Error> {
        self.current_war_consequences
            .as_ref()
            .map(|c| c.log.as_slice())
            .ok_or(Error::DefinitionInputs)
    }
    pub fn grievance_matrices(&self) -> Result<(&Relationships, &Relationships), Error> {
        self.current_grievances
            .as_ref()
            .map(|(a, h)| (a, h))
            .ok_or(Error::DefinitionInputs)
    }
    /// Source ChangeGrievanceAndTension29850e0: directed score followed by
    /// absolute tension. Holder companion turns are deliberately untouched.
    pub fn change_grievance_and_tension(
        &mut self,
        holder: i32,
        source: i32,
        delta: i32,
        turn: i32,
    ) -> Result<(), Error> {
        let index = self.index(holder, source)?;
        let (amounts, holders) = self
            .current_grievances
            .as_ref()
            .ok_or(Error::DefinitionInputs)?;
        let tension = self.tension()?;
        for cells in [amounts, holders, tension] {
            if cells.values.get(index).is_none()
                || cells
                    .changed_turns
                    .as_ref()
                    .and_then(|t| t.get(index))
                    .is_none()
            {
                return Err(Error::InvalidStorage);
            }
        }
        let (amounts, holders) = self
            .current_grievances
            .as_mut()
            .expect("checked grievances");
        amounts
            .changed_turns
            .as_mut()
            .expect("checked amount turns")[index] = turn;
        let current_holder = holders.values[index];
        let opposite = if current_holder == holder || current_holder == -1 {
            if current_holder == -1 {
                holders.values[index] = holder;
            }
            amounts.values[index] = amounts.values[index].wrapping_add(delta);
            source
        } else {
            amounts.values[index] = amounts.values[index].wrapping_sub(delta);
            holder
        };
        if amounts.values[index] < 0 {
            holders.values[index] = opposite;
            amounts.values[index] = amounts.values[index].wrapping_abs();
        }
        let tension = &mut self
            .current_war_consequences
            .as_mut()
            .expect("checked tension")
            .tension;
        let contribution = delta.wrapping_abs();
        if contribution > 0 {
            tension
                .changed_turns
                .as_mut()
                .expect("checked tension turns")[index] = turn;
        }
        tension.values[index] = tension.values[index].wrapping_add(contribution).max(0);
        Ok(())
    }
    /// The reporting event between score mutation and this append belongs to
    /// the callback caller. The native bool argument does not alter push_back.
    pub fn append_grievance_log(&mut self, item: GrievanceLogItem) -> Result<(), Error> {
        self.current_war_consequences
            .as_mut()
            .ok_or(Error::DefinitionInputs)?
            .log
            .push(item);
        Ok(())
    }
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
    pub fn war_aggressors(&self) -> Result<&Relationships, Error> {
        self.current_war_aggressors
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_1b8))
            .ok_or(Error::MissingWarAggressors)
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
    #[test]
    fn grievance_reversal_preserves_holder_turn_and_accumulates_tension() {
        let mut r = relations();
        r.initialize_current_grievances(vec![(0, -1); 3], vec![(-1, -1); 3])
            .unwrap();
        r.initialize_current_war_consequences(vec![(0, -1); 3], vec![])
            .unwrap();
        r.change_grievance_and_tension(1, 0, 150, 4).unwrap();
        assert_eq!(r.grievance_score(1, 0), Ok(150));
        r.change_grievance_and_tension(0, 1, 200, 5).unwrap();
        assert_eq!(r.grievance_score(0, 1), Ok(50));
        let (amount, holder) = r.grievance_matrices().unwrap();
        assert_eq!(amount.changed_turns().unwrap()[2], 5);
        assert_eq!(holder.changed_turns().unwrap()[2], -1);
        assert_eq!(r.tension().unwrap().values()[2], 350);
        assert_eq!(r.tension().unwrap().changed_turns().unwrap()[2], 5);
    }
}
