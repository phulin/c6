//! Current symmetric grievance amounts and holder identities on the same owner.
use super::*;
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
