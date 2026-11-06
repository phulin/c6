//! Concrete City modifier leaves reached by shared modifier lifetime removal.
//! These preserve canonical gameplay arrays; they do not choose active subjects.
use crate::city_production_turn::Error;
use c6_core::{city_wire::DynamicAdjacencyBonus, loaded_state::LoadedState, unit::ComponentId};

/// ChangeProductionModifier241fa20 uses the same binary32 percent conversion
/// as the feature-adjacent leaves. The caller has already persisted the current
/// owner's GetBuildDistrict result in the canonical effect parameter.
pub fn district_production(
    state: &mut LoadedState,
    city: ComponentId,
    district: Option<u32>,
    percent: i32,
    remove: bool,
) -> Result<(), Error> {
    let Some(district) = district else {
        return Ok(());
    };
    let queue = &mut state.city_mut(i32::from(city.owner), city.id)?.build_queue;
    change_district_multiplier(
        &mut queue.district_production_multiplier,
        district,
        percent,
        remove,
    )
}
fn change_district_multiplier(
    values: &mut [c6_core::unit::HashValue<i32>],
    district: u32,
    percent: i32,
    remove: bool,
) -> Result<(), Error> {
    let mut matching = values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.definition_hash == district);
    let at = matching.next().map(|(at, _)| at).ok_or(Error::Unsupported(
        "City effect district production array entry",
    ))?;
    if matching.next().is_some() {
        return Err(Error::Unsupported(
            "duplicate City effect district production array entry",
        ));
    }
    let value = &mut values[at].value;
    *value = value.wrapping_add(c6_core::found_city::feature_production::delta_q8(
        percent, remove,
    ));
    Ok(())
}

/// RemoveDynamicAdjacencyBonus2898910/predicate2899520 compares these seven
/// selectors only. Amount, tile count, description and improvement are ignored;
/// all matches are stably erased without a report, callback or RNG draw.
fn matches_district_adjacency(row: &DynamicAdjacencyBonus, district: u32, yield_hash: u32) -> bool {
    row.district_hash == district
        && row.terrain_hash == u32::MAX
        && row.feature_hash == u32::MAX
        && !row.river
        && row.appeal == -1
        && row.adjacent_district
        && row.yield_hash == yield_hash
}
#[allow(clippy::too_many_arguments)]
pub fn district_adjacency(
    state: &mut LoadedState,
    city: ComponentId,
    district: u32,
    yield_hash: u32,
    amount: i32,
    tiles_required: i32,
    description: &[u8],
    remove: bool,
) -> Result<(), Error> {
    let rows = &mut state
        .city_mut(i32::from(city.owner), city.id)?
        .header
        .dynamic_bonuses;
    if remove {
        rows.retain(|row| !matches_district_adjacency(row, district, yield_hash));
    } else {
        rows.push(DynamicAdjacencyBonus {
            district_hash: district,
            feature_hash: u32::MAX,
            terrain_hash: u32::MAX,
            improvement_hash: Some(u32::MAX),
            river: false,
            adjacent_district: true,
            appeal: -1,
            yield_hash,
            tiles_required,
            amount,
            description: description.to_vec(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bonus(amount: i32) -> DynamicAdjacencyBonus {
        DynamicAdjacencyBonus {
            district_hash: 7,
            feature_hash: u32::MAX,
            terrain_hash: u32::MAX,
            improvement_hash: Some(99),
            river: false,
            adjacent_district: true,
            appeal: -1,
            yield_hash: 3,
            tiles_required: amount,
            amount,
            description: vec![amount as u8],
        }
    }
    #[test]
    fn removal_ignores_nonselector_fields_and_preserves_other_bonus_order() {
        let mut other_yield = bonus(2);
        other_yield.yield_hash = 4;
        let mut other_feature = bonus(4);
        other_feature.feature_hash = 5;
        let mut rows = vec![
            bonus(1),
            other_yield.clone(),
            bonus(3),
            other_feature.clone(),
            bonus(5),
        ];
        rows.retain(|row| !matches_district_adjacency(row, 7, 3));
        assert_eq!(rows, [other_yield, other_feature]);
    }
    #[test]
    fn district_percent_removal_wraps_and_retains_unrelated_cells() {
        use c6_core::unit::HashValue;
        let mut values = [
            HashValue {
                definition_hash: 5,
                value: 123,
            },
            HashValue {
                definition_hash: 7,
                value: i32::MIN,
            },
        ];
        change_district_multiplier(&mut values, 7, 1, true).unwrap();
        assert_eq!(values[0].value, 123);
        assert_eq!(values[1].value, i32::MAX - 2);
        let before = values.clone();
        assert!(change_district_multiplier(&mut values, 8, 100, true).is_err());
        assert_eq!(values, before);
    }
    #[test]
    fn every_native_selector_is_significant() {
        let row = bonus(1);
        assert!(matches_district_adjacency(&row, 7, 3));
        for selector in 0..7 {
            let mut changed = row.clone();
            match selector {
                0 => changed.district_hash = 8,
                1 => changed.terrain_hash = 8,
                2 => changed.feature_hash = 8,
                3 => changed.river = true,
                4 => changed.appeal = 0,
                5 => changed.adjacent_district = false,
                _ => changed.yield_hash = 8,
            }
            assert!(!matches_district_adjacency(&changed, 7, 3));
        }
    }
}
