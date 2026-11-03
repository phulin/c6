//! Linux XP2 movement queries over typed wire state and native-order rules.
//!
//! This imports the bounded, verified movement-relevant fields, not a complete
//! live world. Missing player/district/cache inputs remain errors. The caller
//! must gate the module, rules and configuration identities before import.
//! Static contracts: `docs/linux-movement-world.md`. Native acceptance pending.
use crate::map::{CityAttributes, Map, Plot, PlotFlag};
use crate::movement::{EdgeFacts, FormationFacts, PlotFacts, RouteFacts};
use crate::unit::{Formation, HashValue, PlotCoord, Unit};

/// Adapter for the actual native collection order (never SQLite row IDs).
/// `find_index` has the original Find semantics: a small value is a direct
/// index, otherwise a hash; equal hashes select the last inserted definition.
/// `c6_data::registry::LinuxDefinitionRegistry` supplies that lookup contract.
pub trait DefinitionLookup {
    fn count(&self) -> usize;
    fn find_index(&self, index_or_hash: u32) -> Option<usize>;
    fn type_hash(&self, index: usize) -> Option<u32>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    DefinitionProperties,
    DefinitionIndex,
    UnitArrayStorage,
    InvalidMapDimensions,
    PlotOutsideMap,
    PlayerUnits { player: i32 },
    PlotCityAttributes,
    DistrictLookup { player: i8, district: u32 },
    CityDomainValidity { plot: usize, domain: i32 },
    PortalLookup,
}
impl std::fmt::Display for MissingInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing or inconsistent Linux movement input: {self:?}")
    }
}
impl std::error::Error for MissingInput {}

/// Borrowed properties in native collection order. These are observed native
/// getter values, not default values or SQL REAL values rounded by this module.
pub struct DefinitionTable<'a, T> {
    pub lookup: &'a dyn DefinitionLookup,
    properties: &'a [T],
}
impl<'a, T> DefinitionTable<'a, T> {
    pub fn new(
        lookup: &'a dyn DefinitionLookup,
        properties: &'a [T],
    ) -> Result<Self, MissingInput> {
        if lookup.count() != properties.len() || lookup.count() > i32::MAX as usize {
            return Err(MissingInput::DefinitionProperties);
        }
        Ok(Self { lookup, properties })
    }
    pub fn at(&self, index: i32) -> Option<&T> {
        self.properties.get(index as u32 as usize)
    }
    /// Native Plot stores terrain/feature/improvement indexes in signed shorts.
    pub fn plot_index(&self, wire_hash: u32) -> Result<i32, MissingInput> {
        match self.lookup.find_index(wire_hash) {
            Some(index) if index < self.properties.len() => Ok(i32::from(index as i16)),
            Some(_) => Err(MissingInput::DefinitionIndex),
            None => Ok(-1),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainProperties {
    pub movement_cost: i32,
    pub is_water: bool,
}
pub struct MovementDefinitions<'a> {
    pub terrains: DefinitionTable<'a, TerrainProperties>,
    pub features: DefinitionTable<'a, i32>,
    pub improvements: DefinitionTable<'a, i32>,
    /// Original binary32 MovementCost getter bits, indexed by signed route byte.
    pub routes: DefinitionTable<'a, u32>,
    /// Original IsFreeEmbark getter, indexed by runtime district type.
    pub districts: DefinitionTable<'a, bool>,
}

/// Reproduce current Unit reader's Reset(false) then sequential hash overlay.
/// Unknown hashes consume the pair without writing; duplicates overwrite. The
/// wire pair count is deliberately unrelated to the reconstructed vector length.
pub fn remap_unit_bools(
    pairs: &[HashValue<bool>],
    lookup: &dyn DefinitionLookup,
) -> Result<Vec<bool>, MissingInput> {
    if lookup.count() > i32::MAX as usize {
        return Err(MissingInput::DefinitionIndex);
    }
    let mut values = vec![false; lookup.count()];
    for pair in pairs {
        if let Some(index) = lookup.find_index(pair.definition_hash) {
            *values.get_mut(index).ok_or(MissingInput::DefinitionIndex)? = pair.value;
        }
    }
    Ok(values)
}

/// Imported once per Unit/rules change. No world or wire Unit clone is required.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitMovement {
    pub invalid_terrain: Vec<bool>,
    pub ignore_terrain: Vec<bool>,
    pub ignore_feature: Vec<bool>,
    pub ignore_rivers: bool,
    pub ignore_shores: bool,
    pub escort_mobility: bool,
}

/// Stored pointer identity and state at an actual native recycled-container slot.
#[derive(Clone, Copy, Debug)]
pub struct UnitMovementSlot<'a> {
    pub resolved_id: u32,
    pub state: &'a UnitMovement,
}
#[derive(Clone, Copy, Debug)]
pub struct UnitMovementMatch<'a> {
    pub requested_id: u32,
    pub resolved_id: u32,
    pub state: &'a UnitMovement,
}
/// Original Units::Find rejects UINT_MAX, then uses only the low sixteen slot
/// bits. Slots are in block*8+element order; unallocated/null slots are None.
/// Retain both IDs: native lookup does not reject a generation mismatch.
pub fn find_unit_slot<'a>(
    slots: &[Option<UnitMovementSlot<'a>>],
    requested_id: u32,
) -> Option<UnitMovementMatch<'a>> {
    if requested_id == u32::MAX {
        return None;
    }
    slots
        .get((requested_id & 0xffff) as usize)?
        .as_ref()
        .map(|slot| UnitMovementMatch {
            requested_id,
            resolved_id: slot.resolved_id,
            state: slot.state,
        })
}
impl UnitMovement {
    pub fn from_wire(
        unit: &Unit,
        terrain: &dyn DefinitionLookup,
        feature: &dyn DefinitionLookup,
    ) -> Result<Self, MissingInput> {
        Ok(Self {
            invalid_terrain: remap_unit_bools(&unit.ab_invalid_terrain_types, terrain)?,
            ignore_terrain: remap_unit_bools(&unit.ab_ignore_terrain_types, terrain)?,
            ignore_feature: remap_unit_bools(&unit.ab_ignore_feature_types, feature)?,
            ignore_rivers: unit.k_movement_modifiers.b_ignore_rivers,
            ignore_shores: unit.k_movement_modifiers.b_ignore_shores,
            escort_mobility: unit.k_movement_modifiers.b_escort_mobility,
        })
    }
    pub fn ignores_terrain(&self, index: i32) -> Result<bool, MissingInput> {
        if self.invalid_terrain.len() > i32::MAX as usize {
            return Err(MissingInput::UnitArrayStorage);
        }
        // The original bounds against INVALID terrain but reads IGNORE terrain.
        if index == -1 || index >= self.invalid_terrain.len() as i32 {
            return Ok(false);
        }
        self.ignore_terrain
            .get(index as usize)
            .copied()
            .ok_or(MissingInput::UnitArrayStorage)
    }
    pub fn ignores_feature(&self, index: i32) -> Result<bool, MissingInput> {
        if self.ignore_feature.len() > i32::MAX as usize {
            return Err(MissingInput::UnitArrayStorage);
        }
        if index == -1 || index >= self.ignore_feature.len() as i32 {
            return Ok(false);
        }
        self.ignore_feature
            .get(index as usize)
            .copied()
            .ok_or(MissingInput::UnitArrayStorage)
    }
}

/// Lookup receives the owning Player and the full native member ID unchanged.
/// Its implementation must use native recycled-slot semantics (`find_unit_slot`),
/// rather than reject mismatching generation bits. `None`
/// means a verified absent member, not an unavailable Player/Units registry.
/// Keep the supplied lookup stable during this read-only query.
pub fn resolve_formation<'a>(
    formation: &Formation,
    terrain: i32,
    feature: i32,
    mut find: impl FnMut(i32, u32) -> Result<Option<&'a UnitMovement>, MissingInput>,
) -> Result<FormationFacts, MissingInput> {
    let mut aggregate = |predicate: fn(&UnitMovement) -> Result<bool, MissingInput>| {
        let mut all = true;
        for &id in &formation.unit_ids {
            if let Some(unit) = find(formation.owner, id)? {
                if predicate(unit)? {
                    if unit.escort_mobility {
                        return Ok(true);
                    }
                } else {
                    all = false;
                }
            }
        }
        Ok(all)
    };
    // Index-dependent queries use the same independent aggregation/early exit.
    let ignore_rivers = aggregate(|u| Ok(u.ignore_rivers))?;
    let ignore_shores = aggregate(|u| Ok(u.ignore_shores))?;
    let mut typed = |index: i32, terrain_query: bool| {
        let mut all = true;
        for &id in &formation.unit_ids {
            if let Some(unit) = find(formation.owner, id)? {
                let ignores = if terrain_query {
                    unit.ignores_terrain(index)?
                } else {
                    unit.ignores_feature(index)?
                };
                if ignores && unit.escort_mobility {
                    return Ok(true);
                }
                all &= ignores;
            }
        }
        Ok(all)
    };
    Ok(FormationFacts {
        ignore_terrain: typed(terrain, true)?,
        ignore_feature: typed(feature, false)?,
        ignore_rivers,
        ignore_shores,
        can_embark: formation.can_embark,
    })
}

/// The exact result of the owning Player's Districts::Find(instance ID).
/// An absent district is distinct from unavailable runtime state (an error).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictInstance {
    pub type_index: i32,
    pub complete: bool,
}
pub trait RuntimePlotInputs {
    fn district(&self, city: &CityAttributes) -> Result<Option<DistrictInstance>, MissingInput>;
    fn domain_valid(&self, plot: usize, domain: i32) -> Result<bool, MissingInput>;
}

/// Explicitly unavailable runtime state. Useful for resolving ordinary land
/// without silently treating city/district state as absent.
pub struct NoRuntimePlotInputs;
impl RuntimePlotInputs for NoRuntimePlotInputs {
    fn district(&self, city: &CityAttributes) -> Result<Option<DistrictInstance>, MissingInput> {
        Err(MissingInput::DistrictLookup {
            player: city.player,
            district: city.district,
        })
    }
    fn domain_valid(&self, plot: usize, domain: i32) -> Result<bool, MissingInput> {
        Err(MissingInput::CityDomainValidity { plot, domain })
    }
}

pub fn resolve_plot(
    plot: &Plot,
    plot_index: usize,
    domain: i32,
    definitions: &MovementDefinitions<'_>,
    runtime: &impl RuntimePlotInputs,
) -> Result<PlotFacts, MissingInput> {
    if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(MissingInput::PlotCityAttributes);
    }
    let terrain = definitions.terrains.plot_index(plot.terrain_hash)?;
    let feature = definitions.features.plot_index(plot.feature_hash)?;
    let improvement = definitions.improvements.plot_index(plot.improvement_hash)?;
    let terrain_definition = definitions.terrains.at(terrain);
    // Actual target's built-in water indexes are tested before its definition.
    let is_water = terrain == 15 || terrain == 16 || terrain_definition.is_some_and(|t| t.is_water);
    let district = plot
        .city_attributes
        .as_ref()
        .map(|c| runtime.district(c))
        .transpose()?
        .flatten();
    let district_type = district.map_or(-1, |d| d.type_index);
    let is_city = if district_type == -1 {
        false
    } else {
        definitions
            .districts
            .lookup
            .type_hash(district_type as u32 as usize)
            .ok_or(MissingInput::DefinitionIndex)?
            == 0x8a32_007d
    };
    let district_free_embark = if district_type == -1 {
        None
    } else {
        Some(
            *definitions
                .districts
                .at(district_type)
                .ok_or(MissingInput::DefinitionProperties)?,
        )
    };
    let city_domain_valid = if is_water && plot.city_attributes.is_some() {
        runtime.domain_valid(plot_index, domain)?
    } else {
        // This predicate is unreachable in the shore rules in this case.
        false
    };
    Ok(PlotFacts {
        is_city,
        is_water,
        has_city_attributes: plot.city_attributes.is_some(),
        city_domain_valid,
        terrain_cost: terrain_definition.map(|t| t.movement_cost),
        feature_change: definitions.features.at(feature).copied(),
        improvement_change: definitions.improvements.at(improvement).copied(),
        route: (plot.route_type != -1).then(|| RouteFacts {
            pillaged: plot.flags.get(PlotFlag::RoutePillaged),
            bridged: plot.flags.get(PlotFlag::RouteBridgey),
            movement_cost_float_bits: definitions.routes.at(i32::from(plot.route_type)).copied(),
        }),
        district_free_embark,
        district_complete: district.is_some_and(|d| d.complete),
    })
}

/// Borrow plots directly in native y*width+x order; no world scans or clones.
pub fn plot_at(map: &Map, location: PlotCoord) -> Result<(usize, &Plot), MissingInput> {
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(MissingInput::InvalidMapDimensions);
    }
    if location.x < 0
        || location.y < 0
        || location.x >= map.header.width
        || location.y >= map.header.height
    {
        return Err(MissingInput::PlotOutsideMap);
    }
    let index = (location.y as usize)
        .checked_mul(map.header.width as usize)
        .and_then(|i| i.checked_add(location.x as usize))
        .ok_or(MissingInput::PlotOutsideMap)?;
    Ok((
        index,
        map.plots.get(index).ok_or(MissingInput::PlotOutsideMap)?,
    ))
}

fn row_half(y: i32) -> i32 {
    if y >= 0 {
        y >> 1
    } else {
        let minus_one = y.wrapping_sub(1);
        minus_one.wrapping_add(((minus_one as u32) >> 31) as i32) >> 1
    }
}
pub fn coord_range(value: i32, dimension: i32, wrap: bool) -> i32 {
    if !wrap {
        value
    } else if value < 0 {
        // Native adds dimension even when the negative remainder is zero.
        (value % dimension).wrapping_add(dimension)
    } else if value >= dimension {
        value % dimension
    } else {
        value
    }
}

/// Exact original adjacency loop for in-map coordinates and positive dimensions.
/// Vertical wrapping is used here; GetDirectionToPlot separately wraps only X.
pub fn adjacent(map: &Map, from: PlotCoord, to: PlotCoord) -> Result<bool, MissingInput> {
    plot_at(map, from)?;
    plot_at(map, to)?;
    const X: [i32; 6] = [0, 1, 1, 0, -1, -1];
    const Y: [i32; 6] = [1, 0, -1, -1, 0, 1];
    let hex_x = from.x.wrapping_sub(row_half(from.y));
    for direction in 0..6 {
        let y = from.y.wrapping_add(Y[direction]);
        let x = hex_x.wrapping_add(X[direction]).wrapping_add(row_half(y));
        let x = coord_range(x, map.header.width, map.header.wrap_x);
        let y = coord_range(y, map.header.height, map.header.wrap_y);
        if x >= 0
            && y >= 0
            && x < map.header.width
            && y < map.header.height
            && x == to.x
            && y == to.y
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Native directional sector, including nonadjacent destinations and X wrapping.
/// `None` is NO_DIRECTION; it is not proof that the plots are adjacent.
pub fn direction_to(map: &Map, from: PlotCoord, to: PlotCoord) -> Result<Option<u8>, MissingInput> {
    plot_at(map, from)?;
    plot_at(map, to)?;
    let mut dx =
        to.x.wrapping_sub(row_half(to.y))
            .wrapping_sub(from.x.wrapping_sub(row_half(from.y)));
    if map.header.wrap_x {
        let half = ((map.header.width as u32) >> 1) as i32;
        if dx > half {
            dx = dx.wrapping_sub(map.header.width);
        } else if dx < half.wrapping_neg() {
            dx = dx.wrapping_add(map.header.width);
        }
    }
    let dy = to.y.wrapping_sub(from.y);
    Ok(if dy > 0 {
        Some(if dx < 0 { 5 } else { 0 })
    } else if dy < 0 {
        Some(if dx > 0 { 2 } else { 3 })
    } else if dx > 0 {
        Some(1)
    } else if dx < 0 {
        Some(4)
    } else {
        None
    })
}

/// Resolve directional river and adjacency from the typed Map. Nonempty portal
/// tables still require native sparse-table reconstruction: this fails explicitly
/// for nonadjacent movement rather than treating serialized entrances as sources.
pub fn resolve_edge(
    map: &Map,
    from: PlotCoord,
    to: PlotCoord,
    from_facts: PlotFacts,
    to_facts: PlotFacts,
) -> Result<EdgeFacts, MissingInput> {
    let (_, source) = plot_at(map, from)?;
    plot_at(map, to)?;
    let adjacent = adjacent(map, from, to)?;
    let river_crossing =
        if from_facts.is_water || to_facts.is_water || from_facts.is_city || to_facts.is_city {
            false
        } else {
            direction_to(map, from, to)?.is_some_and(|d| source.river_crossing_mask & (1 << d) != 0)
        };
    let source_portal_has_exit_at_destination = if adjacent {
        false // The actual portal phase never queries a portal on adjacent edges.
    } else {
        let mut route = None;
        for component in &map.components {
            if let crate::map::Component::Route(value) = component {
                if route.is_some() {
                    return Err(MissingInput::PortalLookup);
                }
                route = Some(value);
            }
        }
        match route {
            Some(value) if value.portals.is_empty() => false,
            _ => return Err(MissingInput::PortalLookup),
        }
    };
    Ok(EdgeFacts {
        river_crossing,
        adjacent,
        source_portal_has_exit_at_destination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Lookup(Vec<u32>);
    impl DefinitionLookup for Lookup {
        fn count(&self) -> usize {
            self.0.len()
        }
        fn find_index(&self, key: u32) -> Option<usize> {
            if (key as usize) < self.count() {
                Some(key as usize)
            } else {
                self.0.iter().rposition(|&h| h == key)
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            self.0.get(index).copied()
        }
    }
    #[test]
    fn hash_overlay_uses_native_order_defaults_and_last_duplicate() {
        let registry = Lookup(vec![90, 70, 90]);
        let pairs = [
            HashValue {
                definition_hash: 70,
                value: true,
            },
            HashValue {
                definition_hash: 90,
                value: true,
            },
            HashValue {
                definition_hash: 90,
                value: false,
            },
            HashValue {
                definition_hash: 999,
                value: true,
            },
            HashValue {
                definition_hash: 0,
                value: true,
            },
        ];
        assert_eq!(
            remap_unit_bools(&pairs, &registry).unwrap(),
            [true, true, false]
        );
        assert_eq!(remap_unit_bools(&[], &registry).unwrap(), [false; 3]);
    }
    #[test]
    fn terrain_getter_bounds_against_invalid_vector() {
        let unit = UnitMovement {
            invalid_terrain: vec![true],
            ignore_terrain: vec![false, true],
            ignore_feature: vec![true],
            ignore_rivers: false,
            ignore_shores: false,
            escort_mobility: false,
        };
        assert!(!unit.ignores_terrain(0).unwrap());
        assert!(!unit.ignores_terrain(1).unwrap());
        assert!(unit.ignores_feature(0).unwrap());
        assert!(!unit.ignores_feature(-1).unwrap());
        assert_eq!(
            unit.ignores_terrain(-2),
            Err(MissingInput::UnitArrayStorage)
        );
        let mut inconsistent = unit;
        inconsistent.invalid_terrain.push(false);
        inconsistent.ignore_terrain.pop();
        assert_eq!(
            inconsistent.ignores_terrain(1),
            Err(MissingInput::UnitArrayStorage)
        );
        let slots = [
            None,
            Some(UnitMovementSlot {
                resolved_id: 0x1234_0001,
                state: &inconsistent,
            }),
        ];
        let found = find_unit_slot(&slots, 0xabcd_0001).unwrap();
        assert_eq!(found.requested_id, 0xabcd_0001);
        assert_eq!(found.resolved_id, 0x1234_0001);
        assert!(std::ptr::eq(found.state, &inconsistent));
        assert!(find_unit_slot(&slots, u32::MAX).is_none());
        assert!(find_unit_slot(&slots, 2).is_none());
    }
    #[test]
    fn plot_index_uses_native_short_storage_and_does_not_lookup_hash_as_rowid() {
        let registry = Lookup(vec![700, 500]);
        let table = DefinitionTable::new(&registry, &[9, 4]).unwrap();
        assert_eq!(table.plot_index(500).unwrap(), 1);
        assert_eq!(table.plot_index(0).unwrap(), 0);
        assert_eq!(table.plot_index(u32::MAX).unwrap(), -1);
        assert_eq!(table.at(-1), None);
        assert!(matches!(
            DefinitionTable::new(&registry, &[1]),
            Err(MissingInput::DefinitionProperties)
        ));
    }
    #[test]
    fn native_negative_row_and_range_quirks_are_preserved() {
        assert_eq!(row_half(-1), -1);
        assert_eq!(row_half(-2), -1);
        assert_eq!(row_half(i32::MIN), 0x3fff_ffff);
        assert_eq!(coord_range(-10, 10, true), 10);
        assert_eq!(coord_range(-1, 10, true), 9);
    }
    #[test]
    fn formation_uses_owner_lookup_skips_absent_members_and_escort_is_per_predicate() {
        let mut wire = vec![0u8; 52];
        wire[..4].copy_from_slice(&1i32.to_le_bytes());
        wire[16..20].copy_from_slice(&7i32.to_le_bytes());
        wire[29] = 1;
        wire[48..52].copy_from_slice(&3u32.to_le_bytes());
        for id in [100u32, 101, 102] {
            wire.extend(id.to_le_bytes());
        }
        let formation = Formation::decode(&wire, crate::unit::Limits::default()).unwrap();
        let ordinary = UnitMovement {
            invalid_terrain: vec![false],
            ignore_terrain: vec![false],
            ignore_feature: vec![true],
            ignore_rivers: false,
            ignore_shores: false,
            escort_mobility: false,
        };
        let escort = UnitMovement {
            invalid_terrain: vec![false],
            ignore_terrain: vec![true],
            ignore_feature: vec![false],
            ignore_rivers: true,
            ignore_shores: false,
            escort_mobility: true,
        };
        let facts = resolve_formation(&formation, 0, 0, |owner, id| {
            assert_eq!(owner, 7);
            Ok(match id {
                100 => None,
                101 => Some(&ordinary),
                102 => Some(&escort),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert!(facts.ignore_terrain);
        assert!(!facts.ignore_feature);
        assert!(facts.ignore_rivers);
        assert!(!facts.ignore_shores);
        assert!(facts.can_embark);
        assert_eq!(
            resolve_formation(&formation, 0, 0, |_, _| Err(MissingInput::PlayerUnits {
                player: 7
            })),
            Err(MissingInput::PlayerUnits { player: 7 })
        );
    }

    fn empty<T>() -> crate::map::Recycled<T> {
        crate::map::Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn map_fixture() -> Map {
        let mut plot = Plot::decode(&[0; 55], 6, crate::map::Limits::default()).unwrap();
        plot.feature_hash = u32::MAX;
        plot.improvement_hash = u32::MAX;
        plot.route_type = -1;
        Map {
            header: crate::map::Header {
                width: 5,
                height: 5,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: true,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 25,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: empty(),
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots: vec![plot; 25],
            units: crate::map::UnitContainer {
                width: 5,
                height: 5,
                layers: vec![],
            },
            components: vec![crate::map::Component::Route(crate::map::RouteComponent {
                width: 5,
                height: 5,
                portals: vec![],
            })],
        }
    }
    #[test]
    fn typed_map_resolves_six_directions_wrap_and_river_guards() {
        let mut map = map_fixture();
        let registry = Lookup(vec![100]);
        let terrain = [TerrainProperties {
            movement_cost: 2,
            is_water: false,
        }];
        let route_bits = [0.5f32.to_bits()];
        let defs = MovementDefinitions {
            terrains: DefinitionTable::new(&registry, &terrain).unwrap(),
            features: DefinitionTable::new(&registry, &[1]).unwrap(),
            improvements: DefinitionTable::new(&registry, &[3]).unwrap(),
            routes: DefinitionTable::new(&registry, &route_bits).unwrap(),
            districts: DefinitionTable::new(&registry, &[false]).unwrap(),
        };
        let from = PlotCoord { x: 2, y: 2 };
        let neighbors = [(2, 3), (3, 2), (2, 1), (1, 1), (1, 2), (1, 3)];
        for (d, (x, y)) in neighbors.into_iter().enumerate() {
            let to = PlotCoord { x, y };
            assert!(adjacent(&map, from, to).unwrap());
            assert_eq!(direction_to(&map, from, to).unwrap(), Some(d as u8));
        }
        assert!(adjacent(&map, PlotCoord { x: 0, y: 2 }, PlotCoord { x: 4, y: 2 }).unwrap());
        assert_eq!(
            direction_to(&map, PlotCoord { x: 0, y: 2 }, PlotCoord { x: 4, y: 2 }).unwrap(),
            Some(4)
        );
        map.plots[12].river_crossing_mask = 1 << 1;
        let from_facts = resolve_plot(&map.plots[12], 12, 2, &defs, &NoRuntimePlotInputs).unwrap();
        let to_facts = resolve_plot(&map.plots[13], 13, 2, &defs, &NoRuntimePlotInputs).unwrap();
        assert_eq!(from_facts.terrain_cost, Some(2));
        assert_eq!(from_facts.feature_change, None);
        assert!(
            resolve_edge(&map, from, PlotCoord { x: 3, y: 2 }, from_facts, to_facts)
                .unwrap()
                .river_crossing
        );
        let mut city = to_facts;
        city.is_city = true;
        assert!(
            !resolve_edge(&map, from, PlotCoord { x: 3, y: 2 }, from_facts, city)
                .unwrap()
                .river_crossing
        );
        assert!(
            !resolve_edge(&map, from, PlotCoord { x: 4, y: 4 }, from_facts, to_facts)
                .unwrap()
                .source_portal_has_exit_at_destination
        );
        map.components.clear();
        assert_eq!(
            resolve_edge(&map, from, PlotCoord { x: 4, y: 4 }, from_facts, to_facts),
            Err(MissingInput::PortalLookup)
        );
    }
}
