//! Current player resource allocation to city amenities.
//! Definitions are cached once; allocations remain on the canonical Resources owner.
use crate::{
    city_turn::{self, Error},
    registry::linux_type_hash,
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_capabilities::Membership,
    loaded_state::LoadedState,
    player_resources::{CityResource, ResourceEntry},
};

struct Resource {
    hash: u32,
    happiness: i32,
    peak: Option<i32>,
    luxury: bool,
    bonus: bool,
}
pub struct Definitions {
    binding: WireBinding,
    resources: Vec<Resource>,
    eras: Vec<(u32, i32)>,
    minimum: i32,
    loss: i32,
}
fn integer(row: crate::Row<'_>, name: &'static str) -> Result<i32, Error> {
    row.get(name)
        .map_err(Error::Rules)
        .and_then(|v| crate::game_parameters::sql_integer(v).ok_or(Error::Unsupported(name)))
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let era_defs = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let mut eras = Vec::new();
        for entry in era_defs.entries() {
            let row = table("Eras")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("allocation era"))?;
            eras.push((
                linux_type_hash(&entry.type_name),
                integer(row, "ChronologyIndex")?,
            ));
        }
        let mut resources = Vec::new();
        for entry in collection(rules, "Resources", "ResourceType")
            .map_err(Error::Definitions)?
            .entries()
        {
            let row = table("Resources")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("allocation resource"))?;
            let class = row.text("ResourceClassType").map_err(Error::Rules)?;
            let peak = row.text("PeakEra").map_err(Error::Rules)?;
            let peak = eras
                .iter()
                .find(|v| v.0 == linux_type_hash(peak.as_bytes()))
                .map(|v| v.1);
            resources.push(Resource {
                hash: linux_type_hash(&entry.type_name),
                happiness: integer(row, "Happiness")?,
                peak,
                luxury: class == "RESOURCECLASS_LUXURY",
                bonus: class == "RESOURCECLASS_BONUS",
            });
        }
        let parameter = |name: &str| -> Result<i32, Error> {
            let row = table("GlobalParameters")?
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .ok_or(Error::Unsupported("luxury happiness parameter"))?;
            let value = row.get("Value").map_err(Error::Rules)?;
            match value {
                SqlValue::Text(v) => std::str::from_utf8(v).ok().and_then(|v| v.parse().ok()),
                v => crate::game_parameters::sql_integer(v),
            }
            .ok_or(Error::Unsupported("luxury happiness parameter value"))
        };
        Ok(Self {
            binding: state.binding().clone(),
            resources,
            eras,
            minimum: parameter("LUXURY_HAPPINESS_MINIMUM")?,
            loss: parameter("LUXURY_HAPPINESS_LOSS_PER_ERA")?,
        })
    }
}
#[derive(Clone, Debug)]
struct City {
    id: u32,
    score: i32,
    allocated: i32,
}
#[derive(Clone, Debug)]
struct Available {
    hash: u32,
    copies: i32,
    capacity: i32,
    repeat: bool,
}
// Native ASL strict insertion-sort branch. Larger partitioned sorts are an
// explicit unsupported transition until their tie permutation is implemented.
fn sort<T>(rows: &mut [T], less: impl Fn(&T, &T) -> bool) -> Result<(), Error> {
    if rows.len() > 32 {
        return Err(Error::Unsupported("luxury allocation sort above32"));
    }
    for i in 1..rows.len() {
        let mut j = i;
        while j > 0 && less(&rows[j], &rows[j - 1]) {
            rows.swap(j, j - 1);
            j -= 1;
        }
    }
    Ok(())
}
fn count(rows: &[ResourceEntry<i32>], hash: u32) -> i32 {
    rows.iter()
        .find(|r| r.resource_hash == hash)
        .map_or(0, |r| r.value)
}
fn distribute(
    cities: &mut [City],
    available: &mut [Available],
    mut append: impl FnMut(CityResource),
) -> Result<(), Error> {
    let sort_cities = |cities: &mut [City]| {
        sort(cities, |a, b| {
            a.score.wrapping_add(a.allocated) < b.score.wrapping_add(b.allocated)
        })
    };
    sort_cities(cities)?;
    sort(available, |a, b| a.capacity > b.capacity)?;
    for resource in available {
        let mut at = 0;
        let mut n = 0i32;
        while resource.capacity > n && at < cities.len() {
            append(CityResource {
                city_id: cities[at].id,
                resource_hash: resource.hash,
            });
            cities[at].allocated = cities[at].allocated.wrapping_add(1);
            at += 1;
            n = n.wrapping_add(1);
            if resource.repeat && resource.copies > 1 && at == cities.len() {
                at = 0;
            }
        }
        sort_cities(cities)?;
    }
    Ok(())
}
/// Execute the native allocation pass. Its final reporting event is delivered
/// by the caller's same stock headless backend, after this gameplay mutation.
pub fn allocate(
    state: &mut LoadedState,
    definitions: &Definitions,
    owner: i32,
    city_definitions: &city_turn::yield_calculation::Definitions,
    capabilities: &Membership,
) -> Result<(), Error> {
    if state.binding() != &definitions.binding {
        return Err(Error::Unsupported("luxury allocation binding"));
    }
    state
        .player_resources_mut(owner)?
        .luxury_allocations
        .clear();
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("allocation player"))?;
    let mut cities = Vec::new();
    for row in &player.cities.records.entries {
        let city = &row.value;
        let id = city.header.id;
        let amenities = city_definitions.amenities.amenities(
            state,
            owner,
            id,
            capabilities,
            &city_definitions.growth,
        )?;
        let luxuries = city_definitions
            .amenities
            .from_luxuries(state, owner, id, capabilities)?;
        let needed = city_definitions
            .economy
            .amenities_needed(city.header.population, capabilities);
        cities.push(City {
            id,
            score: amenities.wrapping_sub(luxuries).wrapping_sub(needed),
            allocated: 0,
        });
    }
    let current_era = definitions
        .eras
        .iter()
        .find(|r| r.0 == player.eras.era_hash)
        .map(|r| r.1);
    let resources = &player.resources;
    let mut available = Vec::new();
    for resource in &definitions.resources {
        let amounts = resources
            .resource_ownership
            .iter()
            .find(|r| r.resource_hash == resource.hash)
            .ok_or(Error::Unsupported("allocation ownership row"))?;
        if amounts.value.len() < 2 {
            return Err(Error::Unsupported("allocation ownership categories"));
        }
        let owned = amounts.value[0]
            .wrapping_sub(count(&resources.reserved_resources, resource.hash).max(0));
        let imported = amounts.value[1];
        if (owned <= 0 && imported <= 0) || count(&resources.banned_resources, resource.hash) != 0 {
            continue;
        }
        let mut capacity = resource.happiness.max(0);
        if capacity > 0 {
            if let (Some(now), Some(peak)) = (current_era, resource.peak) {
                let delta = now.wrapping_sub(peak);
                let distance = if delta > 0 {
                    delta
                } else {
                    delta.wrapping_neg()
                };
                capacity = capacity
                    .wrapping_sub(distance.wrapping_mul(definitions.loss))
                    .max(definitions.minimum);
            }
        }
        let repeat = count(&resources.no_cap_resources, resource.hash) > 0;
        let copies = if repeat {
            owned.wrapping_add(imported)
        } else {
            1
        };
        if repeat {
            capacity = capacity.wrapping_mul(copies);
        }
        if owned > 0 {
            if resource.luxury {
                capacity =
                    capacity.wrapping_add(resources.extra_amenities_per_owned_luxury_resource);
            }
            if resource.bonus {
                capacity =
                    capacity.wrapping_add(resources.extra_amenities_per_owned_bonus_resource);
            }
        }
        if capacity > 0 {
            available.push(Available {
                hash: resource.hash,
                copies,
                capacity,
                repeat,
            });
        }
    }
    let output = &mut state.player_resources_mut(owner)?.luxury_allocations;
    distribute(&mut cities, &mut available, |row| output.push(row))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scarce_resources_follow_need_and_rebalance_between_types() {
        let mut cities = vec![
            City {
                id: 7,
                score: 0,
                allocated: 0,
            },
            City {
                id: 8,
                score: -1,
                allocated: 0,
            },
        ];
        let mut resources = vec![
            Available {
                hash: 10,
                copies: 1,
                capacity: 1,
                repeat: false,
            },
            Available {
                hash: 11,
                copies: 1,
                capacity: 1,
                repeat: false,
            },
        ];
        let mut output = Vec::new();
        distribute(&mut cities, &mut resources, |r| {
            output.push((r.city_id, r.resource_hash))
        })
        .unwrap();
        assert_eq!(output, [(8, 10), (8, 11)]); // Equal need retains the preceding native order.
    }
    #[test]
    fn capped_luxury_does_not_repeat_but_no_cap_copies_wrap() {
        let mut cities = vec![City {
            id: 7,
            score: 0,
            allocated: 0,
        }];
        let mut resources = vec![
            Available {
                hash: 10,
                copies: 1,
                capacity: 4,
                repeat: false,
            },
            Available {
                hash: 11,
                copies: 2,
                capacity: 3,
                repeat: true,
            },
        ];
        let mut output = Vec::new();
        distribute(&mut cities, &mut resources, |r| {
            output.push(r.resource_hash)
        })
        .unwrap();
        assert_eq!(output, [10, 11, 11, 11]);
    }
}
