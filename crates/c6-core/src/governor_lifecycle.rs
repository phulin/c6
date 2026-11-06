//! Linux Governor state transitions. Decisions and synchronous consumers belong
//! to the caller; these operations never select a governor or a city.
use crate::player_governors::{self as wire, CityId, EstablishedCity, Governor, Governors};

/// Unsaved Player.Governors maximum. Saved governor_points is not a substitute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Current {
    pub maximum_points: i32,
}

pub fn points_spent(governors: &Governors) -> i32 {
    governors
        .entries
        .iter()
        .flat_map(|s| &s.governor.promotions)
        .fold(0i32, |n, p| n.wrapping_add(i32::from(p.earned)))
}

/// ChangeGovernorPoints2b0c6e0. Positive major deltas change saved points and
/// the unsaved maximum and reset considered. Every major delta reports, even
/// zero/negative; the caller delivers that report after this ordered prefix.
pub fn change_points(governors: &mut Governors, current: &mut Current, major: bool, delta: i32) {
    if major && delta > 0 {
        governors.governor_points = governors.governor_points.wrapping_add(delta);
        if current.maximum_points < governors.governor_points {
            current.maximum_points = governors.governor_points;
        }
        governors.title_considered = false;
    }
}

pub struct Appointment<'a> {
    pub definition_hash: u32,
    pub name: &'a [u8],
    pub identity_pressure: i32,
    pub govern_city_state: bool,
    pub assign_to_major: bool,
    /// Complete promotion collection in native definition-index order.
    pub promotions: &'a [u32],
}

/// Construct and allocate the instance; earned base titles and their signals
/// are applied by the caller after insertion, in Governor promotion order.
pub fn appoint(
    governors: &mut Governors,
    owner: i32,
    definition: &Appointment<'_>,
) -> Result<Option<i32>, crate::city_initialization::Error> {
    use crate::map::{Recycled, Slot};
    let mut records = Recycled {
        capacity: governors.capacity,
        entries: std::mem::take(&mut governors.entries)
            .into_iter()
            .map(|s| Slot {
                index: s.index,
                value: s.governor,
            })
            .collect(),
        current_id_bits: governors.current_id_bits,
        free_head: governors.free_head,
        free_tail: governors.free_tail,
        free_links: std::mem::take(&mut governors.free_links),
    };
    let result = crate::city_initialization::allocate(&mut records);
    if let Ok(Some(allocation)) = result {
        let governor = Governor {
            wire_version: wire::INSTANCE_VERSION,
            id: allocation.id as i32,
            definition_hash: definition.definition_hash,
            name: definition.name.to_vec(),
            assigned_city: CityId {
                owner: -1,
                component_type: 2,
                id: u32::MAX,
            },
            owning_player: owner,
            turns_on_site: 0,
            cultural_pressure: definition.identity_pressure,
            domestic_identity_pressure: 0,
            foreign_identity_pressure: 0,
            domestic_identity_pressure_once_per_city: 0,
            foreign_identity_pressure_once_per_city: 0,
            established: false,
            neutralized_turns: 0,
            neutralized_indefinitely: 0,
            identity_per_title: 0,
            alliance_points: 0,
            ignore_identity_pressure: 0,
            feature_no_improvement_appeal_modifier: 0,
            prevent_structural_damage: 0,
            assign_to_major: definition.assign_to_major,
            resource_power_provided_change: 0,
            faith_purchase_districts: 0,
            extra_heal: 0,
            established_cities: vec![],
            promotions: definition
                .promotions
                .iter()
                .map(|&definition_hash| wire::Promotion {
                    definition_hash,
                    earned: false,
                })
                .collect(),
            govern_city_state: definition.govern_city_state,
            grievances: vec![],
        };
        let at = records
            .entries
            .partition_point(|s| s.index < allocation.slot as i32);
        records.entries.insert(
            at,
            Slot {
                index: allocation.slot as i32,
                value: governor,
            },
        );
    }
    governors.capacity = records.capacity;
    governors.entries = records
        .entries
        .into_iter()
        .map(|s| wire::Slot {
            index: s.index,
            governor: s.value,
        })
        .collect();
    governors.current_id_bits = records.current_id_bits;
    governors.free_head = records.free_head;
    governors.free_tail = records.free_tail;
    governors.free_links = records.free_links;
    result.map(|a| a.map(|a| a.id as i32))
}

pub fn same_city(a: &CityId, b: &CityId) -> bool {
    (a.owner, a.component_type, a.id) == (b.owner, b.component_type, b.id)
}
fn valid(city: &CityId) -> bool {
    city.owner != -1 && city.id != u32::MAX
}
pub fn turns_on_site(governor: &Governor, city: &CityId) -> i32 {
    if !valid(city) {
        return 0;
    }
    governor
        .established_cities
        .iter()
        .find(|r| same_city(&r.city, city))
        .map_or(0, |r| r.value)
}

/// EditTurnsOnSite2a37160. Return true precisely when this call reaches the
/// SetEstablished(true) transition. A newly inserted city map entry does not
/// test the threshold until a later call. Other cities decay and are erased
/// only on a later call that observes a nonpositive value.
pub fn edit_turns_on_site(
    governor: &mut Governor,
    city: CityId,
    delta: i32,
    threshold: i32,
) -> bool {
    let mut found = false;
    let mut establish = false;
    governor.established_cities.retain_mut(|r| {
        if same_city(&r.city, &city) && valid(&city) {
            found = true;
            if !governor.established {
                r.value = r.value.wrapping_add(delta);
                if r.value >= threshold {
                    establish = true;
                }
            }
            true
        } else if r.value > 0 {
            r.value = r.value.wrapping_sub(delta);
            true
        } else {
            false
        }
    });
    if !found {
        if let Some(row) = governor
            .established_cities
            .iter_mut()
            .find(|r| same_city(&r.city, &city))
        {
            row.value = row.value.wrapping_add(delta);
            return establish && !governor.established;
        }
        // CityID operator< compares the complete unsigned ComponentID value.
        let bits = |c: &CityId| {
            (u64::from(c.id) << 32)
                | (u64::from(c.component_type as u16) << 16)
                | u64::from(c.owner as u16)
        };
        let at = governor
            .established_cities
            .partition_point(|r| bits(&r.city) < bits(&city));
        governor
            .established_cities
            .insert(at, EstablishedCity { city, value: delta });
    }
    establish && !governor.established
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Governors {
        Governors {
            wire_version: wire::VERSION,
            governor_points: 0,
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
            neutralized_indefinitely: 0,
            title_considered: true,
            favored_governor_hash: 0,
            diplomatic_favor: 0,
        }
    }
    #[test]
    fn positive_points_preserve_maximum_and_nonpositive_only_report() {
        let mut g = empty();
        let mut c = Current { maximum_points: 4 };
        change_points(&mut g, &mut c, false, 10);
        assert_eq!(g.governor_points, 0);
        change_points(&mut g, &mut c, true, 1);
        assert_eq!(
            (g.governor_points, c.maximum_points, g.title_considered),
            (1, 4, false)
        );
        change_points(&mut g, &mut c, true, -1);
        assert_eq!(g.governor_points, 1);
        change_points(&mut g, &mut c, true, 8);
        assert_eq!(c.maximum_points, 9);
    }
    #[test]
    fn first_city_entry_defers_threshold_and_old_city_decay_preserves_order() {
        let mut g = empty();
        let promotions = [5, 6];
        let id = appoint(
            &mut g,
            0,
            &Appointment {
                definition_hash: 9,
                name: b"name",
                identity_pressure: 8,
                govern_city_state: false,
                assign_to_major: false,
                promotions: &promotions,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(id, 65536);
        assert_eq!(g.free_links, (1..8).collect::<Vec<_>>());
        let gov = &mut g.entries[0].governor;
        assert_eq!(gov.promotions.len(), 2);
        assert_eq!(points_spent(&empty()), 0);
        let city = CityId {
            owner: 0,
            component_type: 2,
            id: 65536,
        };
        assert!(!edit_turns_on_site(gov, city.clone(), 1, 1));
        assert!(edit_turns_on_site(gov, city.clone(), 1, 1));
        assert_eq!(turns_on_site(gov, &city), 2);
        gov.established = true;
        assert!(!edit_turns_on_site(gov, city.clone(), 1, 1));
        assert_eq!(turns_on_site(gov, &city), 2);
        let other = CityId {
            id: 65537,
            ..city.clone()
        };
        gov.established = false;
        assert!(!edit_turns_on_site(gov, other.clone(), 1, 5));
        assert_eq!(turns_on_site(gov, &city), 1);
        assert!(!edit_turns_on_site(gov, other.clone(), 1, 5));
        assert_eq!(turns_on_site(gov, &city), 0);
        assert!(!edit_turns_on_site(gov, other, 1, 5));
        assert_eq!(gov.established_cities.len(), 1);
    }
}
