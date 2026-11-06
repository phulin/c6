//! Current City dynamic bonuses in GetPlayerYieldFrom (Linux XP2 2a3ebd0).
//! These rows use whole signed arithmetic, independently of static Q8 rows.
use super::{adjacency, Definitions, Error};
use c6_core::{city_wire::DynamicAdjacencyBonus, loaded_state::LoadedState, map::Plot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spatial {
    Terrain(u32),
    Feature(u32),
    Improvement(u32),
}
fn spatial(row: &DynamicAdjacencyBonus) -> Option<Spatial> {
    // A terrain/feature/improvement query runs only without any competing
    // shape. Adjacent-district, appeal and river terms are independently added.
    if row.adjacent_district || row.appeal >= 0 || row.river {
        return None;
    }
    match (
        row.terrain_hash,
        row.feature_hash,
        row.improvement_hash.filter(|&h| h != u32::MAX),
    ) {
        (terrain, u32::MAX, None) if terrain != u32::MAX => Some(Spatial::Terrain(terrain)),
        (u32::MAX, feature, None) if feature != u32::MAX => Some(Spatial::Feature(feature)),
        (u32::MAX, u32::MAX, Some(improvement)) => Some(Spatial::Improvement(improvement)),
        _ => None,
    }
}
fn district_matches(owner: i32, neighbor: adjacency::NeighborDistrict) -> bool {
    // The dynamic helper has no IsPillaged check (2a41225..2a41252).
    neighbor.owner == owner && neighbor.complete && !neighbor.internal_only
}
fn finish(row: &DynamicAdjacencyBonus, amount: i32, appeal: i32, river: bool) -> i32 {
    let mut amount = amount;
    if row.appeal >= 0 && appeal >= row.appeal {
        amount = amount.wrapping_add(row.amount);
    }
    if row.river && river {
        amount = amount.wrapping_add(row.amount);
    }
    if row.tiles_required >= 2 {
        amount / row.tiles_required
    } else {
        amount
    }
}
pub(super) fn evaluate(
    row: &DynamicAdjacencyBonus,
    state: &LoadedState,
    definitions: &Definitions,
    owner: i32,
    location: (i32, i32),
    plot: &Plot,
) -> Result<i32, Error> {
    let shape = spatial(row);
    let mut amount = 0i32;
    if shape.is_some() || row.adjacent_district {
        for direction in 0..6 {
            let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
                state, location, direction,
            ) else {
                continue;
            };
            let neighbor = state
                .world()
                .map()
                .plot_at(x, y)
                .ok_or(Error::Unsupported("dynamic district adjacency neighbor"))?;
            let matches = match shape {
                Some(Spatial::Terrain(hash)) => neighbor.terrain_hash == hash,
                Some(Spatial::Feature(hash)) => neighbor.feature_hash == hash,
                // Native GetYieldFrom(ImprovementTypes,int,Plot*) checks only
                // GetImprovementType, including pillaged improvements.
                Some(Spatial::Improvement(hash)) => neighbor.improvement_hash == hash,
                None => false,
            };
            if matches {
                amount = amount.wrapping_add(row.amount);
            }
            if row.adjacent_district {
                if let Some(mut district) =
                    adjacency::current_district(state, definitions, neighbor)?
                {
                    // District.GetOwner resolves its current plot owner.
                    district.owner = crate::movement_placement_state::plot_owner(neighbor)
                        .map_err(|_| Error::Unsupported("dynamic adjacent district owner"))?;
                    if district_matches(owner, district) {
                        amount = amount.wrapping_add(row.amount);
                    }
                }
            }
        }
    }
    Ok(finish(
        row,
        amount,
        i32::from(plot.appeal),
        plot.river_crossing_count > 0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row() -> DynamicAdjacencyBonus {
        DynamicAdjacencyBonus {
            district_hash: 1,
            terrain_hash: u32::MAX,
            feature_hash: u32::MAX,
            improvement_hash: None,
            river: false,
            adjacent_district: true,
            appeal: -1,
            yield_hash: 2,
            tiles_required: 1,
            amount: 1,
            description: vec![],
        }
    }
    #[test]
    fn japan_dynamic_districts_include_pillaged_but_exclude_foreign_incomplete_internal() {
        let district = adjacency::NeighborDistrict {
            hash: 4,
            owner: 1,
            internal_only: false,
            complete: true,
            pillaged: true,
        };
        assert!(district_matches(1, district));
        assert!(!district_matches(0, district));
        assert!(!district_matches(
            1,
            adjacency::NeighborDistrict {
                complete: false,
                ..district
            }
        ));
        assert!(!district_matches(
            1,
            adjacency::NeighborDistrict {
                internal_only: true,
                ..district
            }
        ));
    }
    #[test]
    fn exclusive_spatial_shapes_and_additive_appeal_river() {
        let mut r = row();
        r.adjacent_district = false;
        r.terrain_hash = 3;
        assert_eq!(spatial(&r), Some(Spatial::Terrain(3)));
        r.feature_hash = 4;
        assert_eq!(spatial(&r), None);
        r.terrain_hash = u32::MAX;
        assert_eq!(spatial(&r), Some(Spatial::Feature(4)));
        r.feature_hash = u32::MAX;
        r.improvement_hash = Some(5);
        assert_eq!(spatial(&r), Some(Spatial::Improvement(5)));
        r.river = true;
        r.appeal = 2;
        assert_eq!(spatial(&r), None);
        assert_eq!(finish(&r, 3, 2, true), 5);
        assert_eq!(finish(&r, 3, 1, false), 3);
    }
    #[test]
    fn signed_division_follows_wrapping_sum_per_row_and_only_divides_at_least_two() {
        let mut r = row();
        r.tiles_required = 2;
        assert_eq!(finish(&r, -1, 0, false), 0);
        assert_eq!(finish(&r, -3, 0, false), -1);
        assert_eq!(finish(&r, 3, 0, false), 1);
        r.river = true;
        r.amount = 1;
        assert_eq!(finish(&r, i32::MAX, 0, true), i32::MIN / 2);
        for divisor in [i32::MIN, -1, 0, 1] {
            r.tiles_required = divisor;
            assert_eq!(finish(&r, -3, 0, false), -3);
        }
    }
}
