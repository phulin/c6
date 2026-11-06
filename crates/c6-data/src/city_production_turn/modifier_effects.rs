//! Canonical city mutations reached by building-attached modifier effects.
//! Modifier/requirement lifetime belongs to the shared GameEffects owner; these
//! kernels do not create a second listener registry or choose subjects.
use super::{buildings::index, Error};
use c6_core::loaded_state::LoadedState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    /// City.ChangeYieldChange2897720: ef1e2348, after the indexed value write.
    YieldChange { yield_hash: u32 },
    /// CulturalIdentity.ChangeIdentityPerTurnFromModifiers28763c0: 312df0f6.
    IdentityChange { loyalty_level_hash: u32 },
}
pub trait Effects {
    /// Current Player.Districts.GetBuildDistrict2ae2660 authority. Native -1
    /// returns -1 before array access; all other cells need the current map.
    fn city_build_district(
        &mut self,
        _state: &LoadedState,
        _owner: i32,
        district: Option<u32>,
    ) -> Result<Option<u32>, Error> {
        match district {
            None => Ok(None),
            Some(_) => Err(Error::Unsupported(
                "current City effect district replacement map",
            )),
        }
    }

    fn report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        report: Report,
    ) -> Result<(), Error>;
}
/// AdjustCityYieldChange.Apply2348f30/Remove2348fc0 visits yield parameters in
/// stored order and skips a yield when its parallel amount vector is shorter.
/// The subject/parameter resolver supplies definition hashes from current Rules.
pub fn adjust_yields(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    yields: &[u32],
    amounts: &[i32],
    remove: bool,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    for (&yield_hash, &amount) in yields.iter().zip(amounts) {
        let value = &mut state.city_mut(owner, city)?.header.yield_changes;
        let i = index(value, yield_hash)?;
        value[i].value = value[i].value.wrapping_add(if remove {
            amount.wrapping_neg()
        } else {
            amount
        });
        effects.report(state, owner, city, Report::YieldChange { yield_hash })?;
    }
    Ok(())
}
/// AdjustCityIdentityPerTurn.Apply23a31c0/Remove23a3200 converts the supplied
/// whole-unit Amount to Q8, then changes the actual canonical modifier total.
pub fn adjust_identity_per_turn(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    amount: i32,
    remove: bool,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let value = &mut state.city_mut(owner, city)?.cultural_identity;
    let delta = if remove {
        amount.wrapping_neg()
    } else {
        amount
    }
    .wrapping_shl(8);
    value.identity_per_turn_from_modifiers_raw_q8 = value
        .identity_per_turn_from_modifiers_raw_q8
        .wrapping_add(delta);
    let loyalty_level_hash = value.loyalty_level_hash;
    effects.report(
        state,
        owner,
        city,
        Report::IdentityChange { loyalty_level_hash },
    )
}

/// CityHasFullLoyalty.Test23b0bc0 for an actual ICityInterface subject. The
/// caller supplies current GlobalParameters.LOYALTY_MAXIMUM, or its actual
/// absence. Result0 is this concrete test's satisfied state;1 is failure.
pub fn test_full_loyalty(city: Option<&c6_core::city::City>, maximum: Option<i32>) -> i32 {
    test_loyalty(city.map(|c| c.cultural_identity.loyalty_raw_q8), maximum)
}
fn test_loyalty(loyalty: Option<i32>, maximum: Option<i32>) -> i32 {
    match (loyalty, maximum) {
        (Some(value), Some(maximum)) if value >= maximum.wrapping_shl(8) => 0,
        _ => 1,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_loyalty_preserves_q8_threshold_and_missing_subject_parameters() {
        assert_eq!(test_loyalty(Some(25_599), Some(100)), 1);
        assert_eq!(test_loyalty(Some(25_600), Some(100)), 0);
        assert_eq!(test_loyalty(Some(25_601), Some(100)), 0);
        assert_eq!(test_loyalty(None, Some(100)), 1);
        assert_eq!(test_loyalty(Some(25_600), None), 1);
    }
}
