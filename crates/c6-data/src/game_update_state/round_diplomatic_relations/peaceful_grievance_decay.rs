//! Qualified zero-amount/absent-holder arm of ComputeGrievanceChangePerTurn.
//! Occupation still contributes with zero grievances, so it is checked rather
//! than inferred absent. Positive amounts and assigned holders remain explicit.
use super::{Error, LoadedState, Transition};

fn zero_change(
    amount: i32,
    holder: i32,
    disabled: bool,
    occupation: impl FnOnce() -> Result<bool, Error>,
) -> Result<Option<i32>, Error> {
    if amount != 0 || holder != -1 {
        return Ok(None);
    }
    if disabled || !occupation()? {
        Ok(Some(0))
    } else {
        Ok(None)
    }
}

pub(super) fn change(state: &LoadedState, first: i32, second: i32) -> Result<i32, Error> {
    let ((amount, _), (holder, _)) = state
        .game_relations()
        .map_err(Error::State)?
        .grievance_pair(first, second)
        .map_err(Error::Relations)?;
    // Compute obtains both current Players before querying the disabled flag.
    let player = |owner| {
        state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("grievance current Player"))
    };
    let second_player = player(second)?;
    let first_player = player(first)?;
    let disabled = first_player.diplomacy.grievance_decay_disabled;
    let occupies = |p: &c6_core::loaded_components::RetainedPlayer, other: i32| {
        // IsOccupyingOriginalCapital scans this player's current cities first;
        // IsOccupyingOriginalCity reads the fixed bool only if capital is false.
        p.cities.records.entries.iter().any(|entry| {
            entry.value.header.original_capital && entry.value.header.original_owner == other
        }) || usize::try_from(other)
            .ok()
            .and_then(|i| p.diplomacy.occupies_original_city.get(i))
            .copied()
            .unwrap_or(false)
    };
    zero_change(amount, holder, disabled, || {
        let first_occupies = occupies(first_player, second);
        let second_occupies = occupies(second_player, first);
        Ok(first_occupies || second_occupies)
    })?
    .ok_or(Error::Requires(Transition::GrievanceDecay {
        first,
        second,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_amount_still_checks_occupation_and_disabled_short_circuits() {
        assert_eq!(zero_change(0, -1, false, || Ok(false)).unwrap(), Some(0));
        assert_eq!(zero_change(0, -1, false, || Ok(true)).unwrap(), None);
        assert!(zero_change(0, -1, false, || Err(Error::Missing("occupation"))).is_err());
        assert_eq!(
            zero_change(0, -1, true, || panic!("disabled skips occupation")).unwrap(),
            Some(0)
        );
        assert_eq!(
            zero_change(0, 0, false, || panic!(
                "assigned holder needs other computation"
            ))
            .unwrap(),
            None
        );
        assert_eq!(
            zero_change(3, -1, false, || panic!(
                "raw amount must not hide behind absent holder"
            ))
            .unwrap(),
            None
        );
    }
}
