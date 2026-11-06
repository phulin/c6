//! Current symmetric grievance amounts and holder identities on the same owner.
use super::*;

/// ((amount, changed turn), (holder, changed turn)).
pub type GrievancePair = ((i32, i32), (i32, i32));
impl GameDiplomaticRelations {
    pub fn initialize_current_grievances(
        &mut self,
        amounts: Vec<(i32, i32)>,
        holders: Vec<(i32, i32)>,
    ) -> Result<(), Error> {
        if amounts.len() != total(self.player_count)? || holders.len() != amounts.len() {
            return Err(Error::InvalidStorage);
        }
        if self.current_grievances.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        let matrix = |pairs: Vec<(i32, i32)>| {
            let (values, turns) = pairs.into_iter().unzip();
            Relationships {
                values,
                changed_turns: Some(turns),
            }
        };
        self.current_grievances = Some((matrix(amounts), matrix(holders)));
        Ok(())
    }
    /// Raw amount and holder, including both original companion turn words.
    /// Unlike the signed score, this does not hide an amount behind holder=-1.
    pub fn grievance_pair(&self, first: i32, second: i32) -> Result<GrievancePair, Error> {
        let index = self.index(first, second)?;
        let (amounts, holders) = self
            .current_grievances
            .as_ref()
            .ok_or(Error::DefinitionInputs)?;
        let cell = |r: &Relationships| -> Result<(i32, i32), Error> {
            Ok((
                *r.values.get(index).ok_or(Error::InvalidStorage)?,
                *r.changed_turns
                    .as_ref()
                    .and_then(|t| t.get(index))
                    .ok_or(Error::MissingChangedTurns)?,
            ))
        };
        Ok((cell(amounts)?, cell(holders)?))
    }
    /// DecayGrievanceScore2983be0 writes the two value cells directly. The
    /// caller supplies the exact ComputeGrievanceChangePerTurn result; neither
    /// companion turn, tension, log nor callback is touched by this leaf.
    pub fn decay_grievance_score(
        &mut self,
        first: i32,
        second: i32,
        change: i32,
    ) -> Result<(), Error> {
        let index = self.index(first, second)?;
        let (amounts, holders) = self
            .current_grievances
            .as_mut()
            .ok_or(Error::DefinitionInputs)?;
        if index >= amounts.values.len() || index >= holders.values.len() {
            return Err(Error::InvalidStorage);
        }
        let sum = amounts.values[index].wrapping_add(change);
        amounts.values[index] = sum.max(0);
        if sum < 1 {
            holders.values[index] = -1;
        }
        Ok(())
    }
    /// GetGrievanceScore returns zero before amount lookup for holder=-1.
    /// Otherwise its sign is positive for the first argument's holder.
    pub fn grievance_score(&self, first: i32, second: i32) -> Result<i32, Error> {
        if first == second {
            return Ok(0);
        }
        let (amounts, holders) = self
            .current_grievances
            .as_ref()
            .ok_or(Error::DefinitionInputs)?;
        let holder = self.value(holders, first, second)?;
        if holder == -1 {
            return Ok(0);
        }
        let amount = self.value(amounts, first, second)?;
        Ok(amount.wrapping_mul(if holder == first { 1 } else { -1 }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decay_writes_values_without_retimestamping_relationships() {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, 0)],
            vec![(-1, 0)],
        )
        .unwrap();
        r.initialize_current_grievances(vec![(3, 19)], vec![(0, 23)])
            .unwrap();
        r.decay_grievance_score(0, 1, -2).unwrap();
        assert_eq!(r.grievance_score(0, 1), Ok(1));
        r.decay_grievance_score(0, 1, -2).unwrap();
        let (amounts, holders) = r.grievance_matrices().unwrap();
        assert_eq!(amounts.values(), &[0]);
        assert_eq!(holders.values(), &[-1]);
        assert_eq!(amounts.changed_turns(), Some([19].as_slice()));
        assert_eq!(holders.changed_turns(), Some([23].as_slice()));
    }
    #[test]
    fn holder_sign_and_missing_owner_are_distinct() {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, 0)],
            vec![(-1, 0)],
        )
        .unwrap();
        assert_eq!(r.grievance_score(0, 0), Ok(0));
        assert!(r.grievance_score(0, 1).is_err());
        r.initialize_current_grievances(vec![(17, 3)], vec![(1, 4)])
            .unwrap();
        assert_eq!(r.grievance_score(0, 1), Ok(-17));
        assert_eq!(r.grievance_score(1, 0), Ok(17));
    }
}
