//! City branches of Player.Resources activation over current canonical fields.
use super::missing;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{game_update::Error, loaded_state::LoadedState};

pub(super) struct Definitions {
    resources: LinuxDefinitionRegistry,
    visibility: crate::city_turn::resources::Definitions,
    accumulating: Vec<bool>,
    rates: Vec<(i32, i32)>,
    compatible_improvements: Vec<Vec<u32>>,
    buildings: LinuxDefinitionRegistry,
    maintenance: Vec<usize>,
    power: crate::city_power::Definitions,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &[(u32, bool)],
    ) -> Result<Self, Error> {
        let registry = collection(rules, "Resources", "ResourceType")
            .map_err(|_| missing("resource city definitions"))?;
        let accumulating = registry
            .entries()
            .iter()
            .map(|r| {
                resources
                    .iter()
                    .find(|row| row.0 == linux_type_hash(&r.type_name))
                    .map(|row| row.1)
                    .ok_or(missing("resource city accumulation definition"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut rates = Vec::new();
        let mut compatible_improvements = Vec::new();
        let consumption = rules
            .gameplay()
            .table("Resource_Consumption")
            .ok_or(missing("resource extraction rates"))?;
        let improvements = rules
            .gameplay()
            .table("Improvement_ValidResources")
            .ok_or(missing("resource extraction improvements"))?;
        for resource in registry.entries() {
            let row = consumption.rows().find(|row| {
                row.text("ResourceType").ok().map(str::as_bytes)
                    == Some(resource.type_name.as_slice())
            });
            let rate = |column| -> Result<i32, Error> {
                row.map(|row| {
                    row.get(column)
                        .ok()
                        .and_then(crate::game_parameters::sql_integer)
                        .ok_or(missing("resource extraction rate"))
                })
                .unwrap_or(Ok(0))
            };
            rates.push((rate("BaseExtractionRate")?, rate("ImprovedExtractionRate")?));
            compatible_improvements.push(
                improvements
                    .rows()
                    .filter(|row| {
                        row.text("ResourceType").ok().map(str::as_bytes)
                            == Some(resource.type_name.as_slice())
                    })
                    .map(|row| {
                        row.text("ImprovementType")
                            .map(|name| linux_type_hash(name.as_bytes()))
                            .map_err(|_| missing("resource extraction improvement reference"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        let buildings = collection(rules, "Buildings", "BuildingType")
            .map_err(|_| missing("resource maintenance buildings"))?;
        let mut maintenance = Vec::new();
        for row in rules
            .gameplay()
            .table("Building_ResourceCosts")
            .ok_or(missing("Building_ResourceCosts"))?
            .rows()
        {
            if let Some(index) = row
                .text("BuildingType")
                .ok()
                .and_then(|name| buildings.find_index(linux_type_hash(name.as_bytes())))
            {
                maintenance.push(index);
            }
        }
        Ok(Self {
            resources: registry,
            accumulating,
            rates,
            compatible_improvements,
            buildings,
            maintenance,
            visibility: crate::city_turn::resources::Definitions::from_rules(rules)
                .map_err(|_| missing("resource visibility definitions"))?,
            power: crate::city_power::Definitions::from_current_rules(state, rules)
                .map_err(|_| missing("resource power definitions"))?,
        })
    }
}

/// IncrementAccumulatedResources2b6e310 visits City order, then ascending map
/// indices returned by GetPurchasedPlots2a42110. No-work/unrevealed and
/// non-accumulating resources return before extraction or resource mutation.
pub(super) fn accumulate(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    effects: &mut impl super::Effects,
) -> Result<Option<i32>, Error> {
    use c6_core::map::PlotFlag;
    let cities = super::player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .map(|e| e.value.header.id)
        .collect::<Vec<_>>();
    let mut last = None;
    for city_id in cities {
        let plots = state
            .world()
            .map()
            .plots
            .iter()
            .enumerate()
            .filter(|(_, plot)| {
                plot.city_attributes
                    .as_ref()
                    .is_some_and(|a| i32::from(a.player) == owner && a.owning_city == city_id)
            })
            .map(|(index, _)| index as u32)
            .collect::<Vec<_>>();
        for plot_id in plots {
            let plot = state
                .world()
                .map()
                .plot_by_index(plot_id)
                .ok_or(missing("purchased resource plot"))?;
            if plot.resource_hash == u32::MAX {
                continue;
            }
            let index = defs
                .resources
                .find_index(plot.resource_hash)
                .ok_or(missing("purchased plot resource definition"))?;
            let visible = defs
                .visibility
                .visible_resource(state, owner, index as i32)
                .map_err(|_| missing("current purchased resource visibility"))?;
            if visible < 0
                || !*defs
                    .accumulating
                    .get(visible as usize)
                    .ok_or(missing("current resource consumption definition"))?
            {
                continue;
            }
            let hash = linux_type_hash(&defs.resources.entries()[index].type_name);
            let (base, improved_rate) = defs.rates[index];
            let mut amount = base;
            let improved = !plot.flags.get(PlotFlag::ImprovementPillaged)
                && defs.compatible_improvements[index].contains(&plot.improvement_hash);
            let district = plot
                .city_attributes
                .as_ref()
                .filter(|a| a.district != u32::MAX)
                .and_then(|a| {
                    super::player(state, owner)
                        .ok()?
                        .districts
                        .records
                        .entries
                        .iter()
                        .find(|e| e.value.id == a.district)
                        .map(|e| &e.value)
                });
            // Native accumulation's district test checks pillage only. It is
            // distinct from GetResourceExtractableAt's completed-district gate.
            if improved || district.is_some_and(|district| !district.pillaged) {
                let extra = super::player(state, owner)?
                    .resources
                    .extra_accumulation_modifier
                    .iter()
                    .rfind(|row| defs.resources.find_index(row.resource_hash) == Some(index))
                    .ok_or(missing("current resource accumulation modifier"))?
                    .value
                    .max(0);
                amount = amount.wrapping_add(improved_rate).wrapping_add(extra);
                let city = state
                    .city(owner, city_id)
                    .map_err(|_| missing("resource extracting City"))?;
                if city.resources.extra_accumulation_modifier > 0 {
                    amount = amount.wrapping_add(city.resources.extra_accumulation_modifier);
                }
                let specific = city
                    .resources
                    .extra_accumulation_specific_resource_modifier
                    .iter()
                    .rfind(|row| defs.resources.find_index(row.definition_hash) == Some(index))
                    .ok_or(missing("current city resource accumulation modifier"))?
                    .value;
                if specific > 0 {
                    amount = amount.wrapping_add(specific);
                }
                if city
                    .resources
                    .extra_accumulation_for_strategic_diversity_modifier
                    > 0
                {
                    return Err(missing(
                        "positive resource strategic diversity extraction modifier",
                    ));
                }
            }
            let terrain = state
                .city(owner, city_id)
                .map_err(|_| missing("resource extracting City"))?
                .resources
                .extra_accumulation_terrain_modifier
                .iter()
                .rfind(|row| row.definition_hash == plot.terrain_hash)
                .ok_or(missing("current terrain resource extraction modifier"))?
                .value;
            if terrain > 0 {
                return Err(missing("positive terrain resource extraction modifier"));
            }
            if amount != 0 {
                effects.change_owned_resource(state, owner, hash, amount)?;
                let total = state
                    .player_resources_mut(owner)
                    .map_err(|_| missing("accumulated resource owner"))?
                    .accumulated_resource_per_turn_q8
                    .iter_mut()
                    .rfind(|row| defs.resources.find_index(row.resource_hash) == Some(index))
                    .ok_or(missing("accumulated resource per turn cell"))?;
                total.value = total.value.wrapping_add(amount.wrapping_shl(8));
                last = Some(index as i32);
            }
        }
    }
    Ok(last)
}

fn reset_temporary_power(power: &mut c6_core::city_services::Power) {
    // ResetTemporaryPower28bf570 resets scalar amounts, retaining source IDs
    // and vector order. Generated contributions reset both Q8 fields.
    power.temporary_power_raw_q8 = 0;
    for source in &mut power.temporary_power_sources {
        source.amount_raw_q8 = 0;
    }
    for source in &mut power.generated_power_sources {
        source.amount_raw_q8 = 0;
        source.source_consumed_raw_q8 = 0;
    }
}

/// PayResourceMaintenanceCosts2b6eee0 precedes the all-city temporary reset
/// and lazy IsFullyPowered check in SupplementCityPowerWithResources2b6f150.
pub(super) fn maintain(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
) -> Result<(), Error> {
    let ids = super::player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .map(|city| city.value.header.id)
        .collect::<Vec<_>>();
    for &city in &ids {
        let current = state
            .city(owner, city)
            .map_err(|_| missing("resource maintenance City"))?;
        for &index in &defs.maintenance {
            let location = current
                .buildings
                .building_locations
                .iter()
                .rfind(|row| defs.buildings.find_index(row.definition_hash) == Some(index))
                .ok_or(missing("resource maintenance building presence"))?
                .value;
            if location < 0 {
                continue;
            }
            let pillaged = current
                .buildings
                .pillaged
                .iter()
                .rfind(|row| defs.buildings.find_index(row.definition_hash) == Some(index))
                .ok_or(missing("resource maintenance building pillage"))?
                .value;
            if pillaged {
                continue;
            }
            return Err(missing("owned building resource maintenance consumption"));
        }
    }
    let mut shortage = false;
    for city in ids {
        reset_temporary_power(
            &mut state
                .city_mut(owner, city)
                .map_err(|_| missing("resource temporary power City"))?
                .power,
        );
        let projected = defs
            .power
            .project_fields(
                &state
                    .city(owner, city)
                    .map_err(|_| missing("resource powered City"))?
                    .power,
            )
            .map_err(|_| missing("current resource power fields"))?;
        let fully = defs
            .power
            .is_fully_powered(&projected, || {
                Err(crate::city_power::Error::Missing(
                    "resource power current project",
                ))
            })
            .map_err(|_| missing("current resource power project"))?;
        shortage |= !fully;
    }
    if shortage {
        return Err(missing("positive city resource power supplementation"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::city_services::{
        CityId, GeneratedPowerSource, PlotCoord, Power, TemporaryPowerSource,
    };
    #[test]
    fn temporary_rollover_retains_source_identity_and_resets_both_generated_amounts() {
        let city = || CityId {
            owner: 3,
            component_type: 2,
            id: 65541,
        };
        let mut power = Power {
            wire_version: 10,
            free_power_raw_q8: 512,
            temporary_power_raw_q8: 768,
            required_power_total_raw_q8: 256,
            required_power_from_modifiers_raw_q8: 0,
            ever_generated_power: true,
            turn_last_notified_need_power: 8,
            free_power_sources: vec![],
            temporary_power_sources: vec![TemporaryPowerSource {
                amount_raw_q8: 768,
                source_resource_hash: 13,
                from_city: city(),
            }],
            required_power_sources: vec![],
            generated_power_sources: vec![GeneratedPowerSource {
                amount_raw_q8: 1024,
                source_resource_hash: 17,
                source_consumed_raw_q8: 256,
                to_city: city(),
                location: PlotCoord { x: 4, y: 9 },
            }],
            modified_free_power: 2,
        };
        reset_temporary_power(&mut power);
        assert_eq!(power.temporary_power_raw_q8, 0);
        assert_eq!(power.temporary_power_sources.len(), 1);
        assert_eq!(power.temporary_power_sources[0].amount_raw_q8, 0);
        assert_eq!(power.temporary_power_sources[0].from_city.id, 65541);
        assert_eq!(power.generated_power_sources.len(), 1);
        assert_eq!(power.generated_power_sources[0].amount_raw_q8, 0);
        assert_eq!(power.generated_power_sources[0].source_consumed_raw_q8, 0);
        assert_eq!(power.generated_power_sources[0].source_resource_hash, 17);
        assert_eq!(power.free_power_raw_q8, 512);
        assert_eq!(power.required_power_total_raw_q8, 256);
    }
}
