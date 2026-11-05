//! Source ordinary Farm/Mine legality for owned resource/feature-free first placement.
//! Other improvement and replacement branches remain explicit unsupported inputs.
use super::{Definitions, Error, Kind};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    game_national_parks::ParkData,
    loaded_state::LoadedState,
    map::{Component, PlotFlag, UnitReference},
    unit::Unit,
};
use std::convert::Infallible;

pub(super) struct Static {
    terrains: Vec<u32>,
    hills: Vec<u32>,
}
impl Static {
    pub(super) fn from_rules(rules: &Rules, kind: Kind) -> Result<Self, Error<crate::Error>> {
        let name = kind.type_name();
        let missing = Error::Missing("Farm flat legality rules");
        let row = rules
            .gameplay()
            .table("Improvements")
            .ok_or(missing)?
            .rows()
            .find(|r| r.text("ImprovementType").ok() == Some(name))
            .ok_or(Error::Missing("Farm flat definition"))?;
        for (key, expected) in [
            ("SameAdjacentValid", 1),
            ("RequiresRiver", 0),
            ("EnforceTerrain", 0),
            ("BuildInLine", 0),
            ("CanBuildOutsideTerritory", 0),
            ("BuildOnFrontier", 0),
            ("Coast", 0),
            ("OnePerCity", 0),
            ("ValidAdjacentTerrainAmount", 0),
            ("AdjacentSeaResource", 0),
            ("RequiresAdjacentBonusOrLuxury", 0),
            ("NoAdjacentSpecialtyDistrict", 0),
            ("RequiresAdjacentLuxury", 0),
            ("AdjacentToLand", 0),
            ("OnlyOpenBorders", 0),
        ] {
            if row
                .integer(key)
                .map_err(|error| Error::Effect { stage: key, error })?
                != expected
            {
                return Err(Error::Unsupported("Farm flat definition branch"));
            }
        }
        if row.text("Domain").ok() != Some("DOMAIN_LAND")
            || !matches!(row.get("MinimumAppeal"), Ok(SqlValue::Null))
        {
            return Err(Error::Unsupported("Farm flat domain/appeal"));
        }
        for table in [
            "Improvements_XP2",
            "Improvement_ValidAdjacentTerrains",
            "Improvement_ValidAdjacentResources",
            "Improvement_InvalidAdjacentFeatures",
        ] {
            if rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing(table))?
                .rows()
                .any(|r| r.text("ImprovementType").ok() == Some(name))
            {
                return Err(Error::Unsupported("Farm flat extension/adjacency rule"));
            }
        }
        let terrains = rules
            .gameplay()
            .table("Improvement_ValidTerrains")
            .ok_or(Error::Missing("Farm ValidTerrains"))?
            .rows()
            .filter(|r| {
                r.text("ImprovementType").ok() == Some(name)
                    && matches!(r.get("PrereqTech"), Ok(SqlValue::Null))
                    && matches!(r.get("PrereqCivic"), Ok(SqlValue::Null))
            })
            .map(|r| {
                r.text("TerrainType")
                    .map(|s| linux_type_hash(s.as_bytes()))
                    .map_err(|error| Error::Effect {
                        stage: "Farm TerrainType",
                        error,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if terrains.is_empty() { return Err(Error::Missing("ordinary improvement terrain references")); }
        let hills = rules.gameplay().table("Terrains").ok_or(Error::Missing("Terrain definitions"))?.rows()
            .filter(|r|r.integer("Hills").ok()==Some(1))
            .map(|r|r.text("TerrainType").map(|v|linux_type_hash(v.as_bytes())).map_err(|error|Error::Effect{stage:"TerrainType",error}))
            .collect::<Result<Vec<_>,_>>()?;
        Ok(Self { terrains, hills })
    }
}
/// Actual current climate/park owners borrowed across requests. These contain
/// state, never native CanStart results or projected legal-action answers.
pub struct Inputs<'a> {
    pub fallout_turns: &'a [i32],
    pub drought_ids: &'a [i32],
    pub national_parks: &'a [ParkData],
}
/// Handler-level CanStart. Manager legality and ScriptTestAll remain mandatory
/// in the queued request pipeline. `unit` is its current script-updated actor.
pub fn can_start(
    state: &LoadedState,
    defs: &Definitions,
    inputs: &Inputs<'_>,
    actor: UnitReference,
    unit: &Unit,
    improvement_hash: u32,
    visible_only: bool,
) -> Result<bool, Error<Infallible>> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("Farm legality binding"));
    }
    if improvement_hash != defs.improvement_hash || unit.e_type_index != defs.builder_hash {
        return Err(Error::Unsupported("ordinary Builder Farm legality"));
    }
    if (unit.e_owner, unit.id) != (actor.player(), actor.id) {
        return Err(Error::Unsupported("Farm legality actor identity"));
    }
    if unit.i_build_charges <= 0 || !c6_core::skip_turn::legality::can_move(unit) {
        return Ok(false);
    }
    let (index, plot) = c6_core::movement_world::plot_at(state.world().map(), unit.k_location)
        .map_err(|_| Error::Missing("Farm legality plot"))?;
    let fallout = inputs
        .fallout_turns
        .get(index)
        .ok_or(Error::Missing("Farm current Fallout"))?;
    if *fallout > 0 {
        return Err(Error::Unsupported("Farm fallout work permission"));
    }
    // Farm has no XP2 BuildOnAdjacentPlot row. Native visible-only returns here.
    if visible_only {
        return Ok(true);
    }
    let player = state.retained().players.iter().find(|p|p.header.owner==unit.e_owner)
        .ok_or(Error::Missing("improvement prerequisite player"))?;
    use c6_core::yields::{PrerequisiteKind, PrerequisiteState};
    for (reference, flags, kind) in [(defs.prerequisite_technology, &player.techs.has_tech.pairs, PrerequisiteKind::Technology),
        (defs.prerequisite_civic, &player.culture.has_civic.pairs, PrerequisiteKind::Civic)] {
        if let Some((reference,index)) = reference {
            if !state.world().globally_available(kind,index).map_err(|_|Error::Missing("current improvement prerequisite availability"))? {return Ok(false);}
            if !flags.iter().rfind(|r|r.definition_hash==reference)
                .ok_or(Error::Missing("current improvement prerequisite flag"))?.value { return Ok(false); }
        }
    }
    // ClimateData stores active drought object IDs, not one ID per plot.
    // With no objects GetActiveDroughtTypeAtPlot returns NO_DROUGHT before
    // any plot-membership lookup, as in current tile-yield calculation.
    if !inputs.drought_ids.is_empty() {
        return Err(Error::Unsupported("Farm active drought objects"));
    }
    if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Missing("Farm current CityAttributes"));
    }
    let Some(attributes) = &plot.city_attributes else {
        return Ok(false);
    };
    if i32::from(attributes.player) != unit.e_owner {
        return Ok(false);
    }
    // The supported ordinary plot has neither a district nor a wonder. A
    // present identifier requires its own live object lookup, not a saved flag.
    if attributes.district != u32::MAX || attributes.wonder_building_hash != u32::MAX {
        return Err(Error::Unsupported("Farm district/wonder plot"));
    }
    if inputs
        .national_parks
        .iter()
        .any(|p| p.plot_indices.contains(&(index as i32)))
    {
        return Ok(false);
    }
    let terrain = state
        .world()
        .map()
        .components
        .iter()
        .find_map(|c| match c {
            Component::Terrain(t) => Some(t),
            _ => None,
        })
        .ok_or(Error::Missing("Farm current Terrain"))?;
    if terrain
        .coastal_lowlands
        .iter()
        .any(|p| p.plot_index == index as i32 && p.flooded)
    {
        return Ok(false);
    }
    if plot.flags.get(PlotFlag::Impassable) {
        return Ok(false);
    }
    if defs.legality.hills.contains(&plot.terrain_hash) != (defs.kind == Kind::Mine)
        || plot.feature_hash != u32::MAX
        || plot.resource_hash != u32::MAX
        || plot.improvement_hash != u32::MAX
    {
        return Err(Error::Unsupported(
            "Farm flat empty first-placement profile",
        ));
    }
    Ok(defs.legality.terrains.contains(&plot.terrain_hash))
}
