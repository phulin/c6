//! NearCultureBorder::Test uses retained observations and current plot owners.
use super::Error;
use c6_core::{
    loaded_state::LoadedState, player_ai_targets::ArmyKnowledge, player_diplomacy::Diplomacy,
    unit::PlotCoord,
};
#[derive(Clone, Copy)]
pub(super) struct UnitDefinition {
    pub trade: bool,
    pub spy: bool,
    pub civilian: bool,
}
pub(super) fn has_recent_incursion(d: &Diplomacy, subject: i32, since: i32) -> bool {
    d.incursions_made
        .iter()
        .find(|(p, _)| *p == subject)
        .and_then(|(_, t)| t.entries.iter().find(|(k, _)| *k == 0x626c7a34))
        .is_some_and(|(_, turns)| turns.iter().any(|&turn| turn > since))
}
pub(super) fn has_border_promise(d: &Diplomacy, subject: i32) -> bool {
    d.promises_made
        .iter()
        .find(|(p, _)| *p == subject)
        .is_some_and(|(_, t)| t.entries.iter().any(|(k, _)| *k == 0x9c55a0d8u32 as i32))
}
pub(super) fn plot_owner(state: &LoadedState, at: PlotCoord) -> Result<Option<i32>, Error> {
    let map = state.world().map();
    let x = c6_core::movement_world::coord_range(at.x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(at.y, map.header.height, map.header.wrap_y);
    if x < 0 || x >= map.header.width || y < 0 || y >= map.header.height {
        return Ok(None);
    }
    let plot = map
        .plots
        .get(y as usize * map.header.width as usize + x as usize)
        .ok_or(Error::Missing("current border neighborhood Plot"))?;
    crate::movement_placement_state::plot_owner(plot)
        .map(Some)
        .map_err(Error::Placement)
}
pub(super) fn count(
    army: &ArmyKnowledge,
    radius: u32,
    context: i32,
    subject: i32,
    mut definition: impl FnMut(u32) -> Result<UnitDefinition, Error>,
    mut owner: impl FnMut(PlotCoord) -> Result<Option<i32>, Error>,
) -> Result<i32, Error> {
    let mut domestic = 0i32;
    let mut foreign = 0i32;
    for group in army
        .army_size
        .value
        .as_ref()
        .ok_or(Error::Missing("current border army contents"))?
    {
        let def = definition(group.hash)?;
        if def.trade || def.spy || def.civilian {
            continue;
        }
        for unit in &group.value {
            if unit.turns_since_observation > 2 {
                continue;
            }
            let at = PlotCoord {
                x: unit.location.x,
                y: unit.location.y,
            };
            let mut near = false;
            c6_core::unit_placement_tails::neighborhood(at, radius, |cell| {
                // Native OR does not short-circuit traversal or plot queries.
                near |= owner(cell)? == Some(context);
                Ok::<_, Error>(())
            })?;
            if near {
                let home = owner(at)?.ok_or(Error::Missing("current border army center Plot"))?;
                if home == subject {
                    domestic = domestic.wrapping_add(1);
                } else {
                    foreign = foreign.wrapping_add(1);
                }
            }
        }
    }
    Ok(foreign.wrapping_add(domestic / 4))
}
#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::player_ai_targets::{
        BoolByte, Conditional, HashEntry, PlotCoord as KnownCoord, UnitInfo,
    };
    fn army(ages: &[u32]) -> ArmyKnowledge {
        ArmyKnowledge {
            army_size: Conditional {
                present: BoolByte(1),
                value: Some(vec![HashEntry {
                    hash: 1234,
                    value: ages
                        .iter()
                        .enumerate()
                        .map(|(i, &age)| UnitInfo {
                            unit_id: i as u32,
                            known_damage: 0,
                            military_formation: 0,
                            location: KnownCoord { x: 4, y: 4 },
                            area_id: 0,
                            region_id: 0,
                            turns_since_observation: age,
                        })
                        .collect(),
                }]),
            },
        }
    }
    fn military(_: u32) -> Result<UnitDefinition, Error> {
        Ok(UnitDefinition {
            trade: false,
            spy: false,
            civilian: false,
        })
    }
    #[test]
    fn observation_age_and_center_ownership_control_weight() {
        let a = army(&[0, 1, 2, 0, 3, u32::MAX]);
        // Four recent observations inside the subject's culture count as one.
        let at_home = |p: PlotCoord| Ok(Some(if p.x == 4 && p.y == 4 { 1 } else { 0 }));
        assert_eq!(count(&a, 1, 0, 1, military, at_home).unwrap(), 1);
        assert_eq!(count(&a, 1, 0, 1, military, |_| Ok(Some(0))).unwrap(), 4);
        assert_eq!(count(&a, 1, 0, 1, military, |_| Ok(Some(-1))).unwrap(), 0);
        assert_eq!(
            count(&army(&[0, 0, 0]), 1, 0, 1, military, at_home).unwrap(),
            0
        );
    }
    #[test]
    fn domestic_quarters_are_aggregated_across_definition_groups() {
        let mut a = army(&[0, 0]);
        let mut second = army(&[0, 0, 0]).army_size.value.unwrap().remove(0);
        second.hash = 2345;
        second.value[2].location.x = 5;
        a.army_size.value.as_mut().unwrap().push(second);
        let owner = |p: PlotCoord| Ok(Some(if p.x == 4 && p.y == 4 { 1 } else { 0 }));
        assert_eq!(count(&a, 1, 0, 1, military, owner).unwrap(), 2);
    }
    #[test]
    fn radius_and_unit_classes_gate_the_actual_neighborhood() {
        let a = army(&[0]);
        let owner = |p: PlotCoord| Ok(Some(if p.x == 6 && p.y == 4 { 0 } else { -1 }));
        assert_eq!(count(&a, 1, 0, 1, military, owner).unwrap(), 0);
        assert_eq!(count(&a, 2, 0, 1, military, owner).unwrap(), 1);
        for (trade, spy, civilian) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            assert_eq!(
                count(
                    &a,
                    2,
                    0,
                    1,
                    |_| Ok(UnitDefinition {
                        trade,
                        spy,
                        civilian
                    }),
                    |_| panic!("excluded unit queried map")
                )
                .unwrap(),
                0
            );
        }
    }
    #[test]
    fn positive_neighborhood_still_visits_every_cell() {
        let mut visits = 0;
        assert_eq!(
            count(&army(&[0]), 1, 0, 1, military, |_| {
                visits += 1;
                Ok(Some(0))
            })
            .unwrap(),
            1
        );
        assert_eq!(visits, 8); // Seven cells, then the center ownership query.
    }
}
