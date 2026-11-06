//! Static district adjacency rows over the current canonical map and districts.
//! Planning map changes, normalized AI yields, and dynamic City bonuses are
//! separate native paths. Unsupported spatial shapes remain explicit.
use super::super::yield_definitions::Adjacency;
use super::{Definitions, Error};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Row, Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
    movement_world::DefinitionLookup,
    yields::{PrerequisiteKind, PrerequisiteState},
};

struct Rule {
    validity: Adjacency,
    yield_hash: u32,
    terrain: Option<u32>,
    feature: Option<u32>,
    improvement: Option<u32>,
    district: Option<u32>,
    other_district: bool,
    excluded_traits: Vec<u32>,
}
pub(super) struct RulesCache {
    rules: Vec<Rule>,
    civilization_traits: Vec<(u32, u32)>,
    leader_traits: Vec<(u32, u32)>,
    player_traits: Vec<(i32, Vec<u32>)>,
}
fn hash(row: Row<'_>, column: &'static str) -> Result<u32, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Text(v) => Ok(linux_type_hash(v)),
        SqlValue::Null => Ok(u32::MAX),
        _ => Err(Error::Unsupported(column)),
    }
}
fn integer(row: Row<'_>, column: &'static str) -> Result<i32, Error> {
    crate::game_parameters::sql_integer(row.get(column).map_err(Error::Rules)?)
        .ok_or(Error::Unsupported(column))
}
fn reference(
    row: Row<'_>,
    column: &'static str,
    registry: &LinuxDefinitionRegistry,
) -> Result<Option<u32>, Error> {
    let hash = hash(row, column)?;
    // Native definition references resolve to null when the named row is absent.
    Ok(registry.find(hash).map(|_| hash))
}
impl RulesCache {
    pub(super) fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let registry =
            collection(rules, "Adjacency_YieldChanges", "ID").map_err(Error::Definitions)?;
        let yields = collection(rules, "Yields", "YieldType").map_err(Error::Definitions)?;
        let terrain = collection(rules, "Terrains", "TerrainType").map_err(Error::Definitions)?;
        let features = collection(rules, "Features", "FeatureType").map_err(Error::Definitions)?;
        let improvements =
            collection(rules, "Improvements", "ImprovementType").map_err(Error::Definitions)?;
        let districts =
            collection(rules, "Districts", "DistrictType").map_err(Error::Definitions)?;
        let civics = collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?;
        let techs =
            collection(rules, "Technologies", "TechnologyType").map_err(Error::Definitions)?;
        let table = |name| rules.gameplay().table(name).ok_or(Error::Unsupported(name));
        let exclusions = table("ExcludedAdjacencies")?;
        let mut cached = Vec::with_capacity(registry.count());
        for entry in registry.entries() {
            let row = table("Adjacency_YieldChanges")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("district adjacency row"))?;
            let source = linux_type_hash(&entry.type_name);
            let excluded_traits = exclusions
                .rows()
                .filter_map(|r| match hash(r, "YieldChangeId") {
                    Ok(h) if h == source => Some(hash(r, "TraitType")),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let native_index =
                |registry: &LinuxDefinitionRegistry, column| -> Result<Option<i32>, Error> {
                    Ok(registry.find_index(hash(row, column)?).map(|i| i as i32))
                };
            let no_resource_class = match row.get("AdjacentResourceClass").map_err(Error::Rules)? {
                SqlValue::Null => true,
                SqlValue::Text(v) => v.as_slice() == b"NO_RESOURCECLASS",
                _ => false,
            };
            let supported = [
                "AdjacentSeaResource",
                "AdjacentRiver",
                "AdjacentWonder",
                "AdjacentNaturalWonder",
                "AdjacentResource",
            ]
            .into_iter()
            .try_fold(true, |ok, name| {
                Ok::<_, Error>(ok && integer(row, name)? == 0)
            })? && no_resource_class;
            let yield_hash = reference(row, "YieldType", &yields)?
                .ok_or(Error::Unsupported("district adjacency Yield reference"))?;
            cached.push(Rule {
                validity: Adjacency {
                    yield_index: native_index(&yields, "YieldType")?.unwrap_or(-1),
                    change: integer(row, "YieldChange")?,
                    tiles_required: integer(row, "TilesRequired")?,
                    improvement: None,
                    self_yield: integer(row, "Self")? != 0,
                    supported,
                    excluded: !excluded_traits.is_empty(),
                    prerequisites: [
                        native_index(&civics, "PrereqCivic")?,
                        native_index(&civics, "ObsoleteCivic")?,
                        native_index(&techs, "PrereqTech")?,
                        native_index(&techs, "ObsoleteTech")?,
                    ],
                },
                yield_hash,
                terrain: reference(row, "AdjacentTerrain", &terrain)?,
                feature: reference(row, "AdjacentFeature", &features)?,
                improvement: reference(row, "AdjacentImprovement", &improvements)?,
                district: reference(row, "AdjacentDistrict", &districts)?,
                other_district: integer(row, "OtherDistrictAdjacent")? != 0,
                excluded_traits,
            });
        }
        let traits = |name, column| -> Result<Vec<(u32, u32)>, Error> {
            table(name)?
                .rows()
                .map(|r| Ok((hash(r, column)?, hash(r, "TraitType")?)))
                .collect()
        };
        Ok(Self {
            rules: cached,
            civilization_traits: traits("CivilizationTraits", "CivilizationType")?,
            leader_traits: traits("LeaderTraits", "LeaderType")?,
            player_traits: Vec::new(),
        })
    }
    pub(super) fn bind_configuration(&mut self, players: &[(i32, u32, u32)]) {
        self.player_traits = players
            .iter()
            .map(|&(owner, civ, leader)| {
                let traits = self
                    .civilization_traits
                    .iter()
                    .filter(|(hash, _)| *hash == civ)
                    .chain(
                        self.leader_traits
                            .iter()
                            .filter(|(hash, _)| *hash == leader),
                    )
                    .map(|(_, t)| *t)
                    .collect();
                (owner, traits)
            })
            .collect();
    }
    pub(super) fn evaluate(
        &self,
        index: u32,
        state: &LoadedState,
        definitions: &Definitions,
        owner: i32,
        location: (i32, i32),
        requested: u32,
    ) -> Result<i32, Error> {
        let rule = self
            .rules
            .get(index as usize)
            .ok_or(Error::Unsupported("district adjacency rule index"))?;
        let world = state.world();
        if !rule
            .validity
            .valid(|technology, index| {
                let kind = if technology {
                    PrerequisiteKind::Technology
                } else {
                    PrerequisiteKind::Civic
                };
                let mask = world.acquired_mask(owner, kind)?;
                Ok::<_, c6_core::yields::MissingInput>(
                    index >= 0
                        && (index as usize) < mask.len()
                        && world.globally_available(kind, index)?
                        && mask[index as usize],
                )
            })
            .map_err(|_| Error::Unsupported("district adjacency prerequisite state"))?
        {
            return Ok(0);
        }
        // IsBonusValid checks trait exclusions before the requested yield gate.
        if !rule.excluded_traits.is_empty() {
            let traits = &self
                .player_traits
                .iter()
                .find(|(p, _)| *p == owner)
                .ok_or(Error::Unsupported(
                    "district adjacency player civilization/leader traits",
                ))?
                .1;
            if rule.excluded_traits.iter().any(|t| traits.contains(t)) {
                return Ok(0);
            }
        }
        if rule.yield_hash != requested {
            return Ok(0);
        }
        if !rule.validity.supported {
            return Err(Error::Unsupported("district adjacency spatial rule"));
        }
        let mut count = i32::from(rule.validity.self_yield);
        for direction in 0..6 {
            let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
                state, location, direction,
            ) else {
                continue;
            };
            let plot = world
                .map()
                .plot_at(x, y)
                .ok_or(Error::Unsupported("district adjacency neighbor"))?;
            let district = if rule.other_district || rule.district.is_some() {
                current_district(state, definitions, plot)?
            } else {
                None
            };
            count = count.wrapping_add(rule.count_neighbor(owner, plot, district));
        }
        super::super::tiles::adjacency_amount(
            rule.validity.change,
            rule.validity.tiles_required,
            count,
        )
        .ok_or(Error::Unsupported("district adjacency fixed point divisor"))
    }
}
#[derive(Clone, Copy)]
pub(super) struct NeighborDistrict {
    pub(super) hash: u32,
    pub(super) owner: i32,
    pub(super) internal_only: bool,
    pub(super) complete: bool,
    pub(super) pillaged: bool,
}
pub(super) fn current_district(
    state: &LoadedState,
    definitions: &Definitions,
    plot: &Plot,
) -> Result<Option<NeighborDistrict>, Error> {
    let Some(attributes) = &plot.city_attributes else {
        return Ok(None);
    };
    let owner = i32::from(attributes.player);
    if owner == -1 || attributes.district == u32::MAX {
        return Ok(None);
    }
    if !state
        .world()
        .player_is_valid(owner)
        .map_err(|_| Error::Unsupported("adjacent district player"))?
    {
        return Ok(None);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("adjacent district owner"))?;
    let Some(district) = player
        .districts
        .records
        .entries
        .iter()
        .find(|e| e.index as u32 == attributes.district & 0xffff)
        .map(|e| &e.value)
    else {
        return Ok(None);
    };
    let internal_only = definitions.district(district)?.internal_only;
    Ok(Some(NeighborDistrict {
        hash: district.district_hash,
        owner,
        internal_only,
        complete: district.is_complete,
        pillaged: district.pillaged,
    }))
}
impl Rule {
    fn count_neighbor(&self, owner: i32, plot: &Plot, district: Option<NeighborDistrict>) -> i32 {
        let mut count = 0i32;
        if self.other_district
            && district
                .is_some_and(|d| d.owner == owner && !d.internal_only && d.complete && !d.pillaged)
        {
            count += 1;
        }
        if self.feature == Some(plot.feature_hash) {
            count += 1;
        }
        if self.improvement == Some(plot.improvement_hash)
            && !plot.flags.get(PlotFlag::ImprovementPillaged)
        {
            count += 1;
        }
        if self.terrain == Some(plot.terrain_hash) {
            count += 1;
        }
        if self.district.is_some_and(|hash| {
            district.is_some_and(|d| d.hash == hash && d.complete && !d.pillaged)
        }) {
            count += 1;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule() -> Rule {
        Rule {
            validity: Adjacency {
                yield_index: 0,
                change: 1,
                tiles_required: 2,
                improvement: None,
                self_yield: false,
                supported: true,
                excluded: false,
                prerequisites: [None; 4],
            },
            yield_hash: 1,
            terrain: None,
            feature: None,
            improvement: None,
            district: None,
            other_district: true,
            excluded_traits: Vec::new(),
        }
    }
    fn plot() -> Plot {
        Plot {
            area_id: 0,
            landmass_id: 0,
            region_id: 0,
            terrain_hash: 2,
            feature_hash: 3,
            feature_layout: 0,
            continent_hash: 0,
            unit_count: 0,
            resource_hash: u32::MAX,
            resource_count: 0,
            improvement_hash: 4,
            improvement_owner: -1,
            route_type: -1,
            route_era: -1,
            appeal: 0,
            river_east_flow: 0,
            river_south_east_flow: 0,
            river_south_west_flow: 0,
            river_crossing_count: 0,
            river_crossing_mask: 0,
            cliff_crossing_mask: 0,
            flags: Default::default(),
            properties: None,
            city_attributes: None,
        }
    }
    #[test]
    fn other_district_requires_own_complete_unpillaged_noninternal() {
        let rule = rule();
        let plot = plot();
        let district = NeighborDistrict {
            hash: 7,
            owner: 0,
            internal_only: false,
            complete: true,
            pillaged: false,
        };
        assert_eq!(rule.count_neighbor(0, &plot, Some(district)), 1);
        for rejected in [
            NeighborDistrict {
                owner: 1,
                ..district
            },
            NeighborDistrict {
                complete: false,
                ..district
            },
            NeighborDistrict {
                pillaged: true,
                ..district
            },
            NeighborDistrict {
                internal_only: true,
                ..district
            },
        ] {
            assert_eq!(rule.count_neighbor(0, &plot, Some(rejected)), 0);
        }
        assert_eq!(rule.count_neighbor(0, &plot, None), 0);
    }
    #[test]
    fn named_district_has_no_owner_or_internal_filter_but_checks_completion() {
        let mut rule = rule();
        rule.other_district = false;
        rule.district = Some(7);
        let district = NeighborDistrict {
            hash: 7,
            owner: 1,
            internal_only: true,
            complete: true,
            pillaged: false,
        };
        assert_eq!(rule.count_neighbor(0, &plot(), Some(district)), 1);
        assert_eq!(
            rule.count_neighbor(
                0,
                &plot(),
                Some(NeighborDistrict {
                    complete: false,
                    ..district
                })
            ),
            0
        );
        assert_eq!(
            rule.count_neighbor(
                0,
                &plot(),
                Some(NeighborDistrict {
                    pillaged: true,
                    ..district
                })
            ),
            0
        );
    }
    #[test]
    fn independent_neighbor_shapes_add_and_fractional_rows_round_separately() {
        let mut rule = rule();
        rule.other_district = false;
        rule.feature = Some(3);
        rule.terrain = Some(2);
        rule.improvement = Some(4);
        let mut plot = plot();
        assert_eq!(rule.count_neighbor(0, &plot, None), 3);
        plot.flags.set(PlotFlag::ImprovementPillaged, true);
        assert_eq!(rule.count_neighbor(0, &plot, None), 2);
        let amount = super::super::super::tiles::adjacency_amount;
        assert_eq!(amount(1, 2, 1), Some(0));
        assert_eq!(amount(1, 2, 3), Some(1));
        assert_eq!(amount(-1, 2, 1), Some(-1));
        assert_eq!(
            amount(1, 2, 1)
                .unwrap()
                .wrapping_add(amount(1, 2, 1).unwrap()),
            0
        );
    }
    #[test]
    fn trait_configuration_uses_both_current_identity_collections() {
        let mut cache = RulesCache {
            rules: vec![],
            civilization_traits: vec![(5, 11), (6, 12)],
            leader_traits: vec![(8, 13)],
            player_traits: vec![],
        };
        cache.bind_configuration(&[(0, 5, 8), (1, 6, 9)]);
        assert_eq!(cache.player_traits, vec![(0, vec![11, 13]), (1, vec![12])]);
    }
}
