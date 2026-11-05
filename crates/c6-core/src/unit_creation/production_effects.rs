//! Targets.UnitCreated -> ArmyKnowledge.AddUnit (Linux XP2 21ab510/1ffed10).
use crate::player_ai_targets::UnitInfo;

/// Refresh the first matching ID in place, or append in observation order.
/// No planner decision, signal or random draw occurs in this source leaf.
pub fn observe_unit(bucket: &mut Vec<UnitInfo>, mut current: UnitInfo) {
    current.turns_since_observation = 0;
    if let Some(previous) = bucket.iter_mut().find(|v| v.unit_id == current.unit_id) {
        *previous = current;
    } else {
        bucket.push(current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_ai_targets::PlotCoord;
    fn unit(id: u32, damage: i32) -> UnitInfo {
        UnitInfo {
            unit_id: id,
            known_damage: damage,
            military_formation: 0,
            location: PlotCoord { x: 3, y: 4 },
            area_id: 7,
            region_id: 9,
            turns_since_observation: 12,
        }
    }
    #[test]
    fn update_first_duplicate_without_reordering_then_append() {
        let mut bucket = vec![unit(2, 1), unit(1, 2), unit(2, 3)];
        observe_unit(&mut bucket, unit(2, 10));
        assert_eq!(
            bucket
                .iter()
                .map(|u| (u.unit_id, u.known_damage))
                .collect::<Vec<_>>(),
            vec![(2, 10), (1, 2), (2, 3)]
        );
        assert_eq!(bucket[0].turns_since_observation, 0);
        observe_unit(&mut bucket, unit(4, 8));
        assert_eq!(bucket.last().unwrap().unit_id, 4);
        assert_eq!(bucket.last().unwrap().turns_since_observation, 0);
    }
}
