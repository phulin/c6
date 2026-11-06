//! Current ordinary civilian and land military Unit eligibility and external queue selection.
//! CanTrain2851040 is queried from live state; native legality answers are never
//! inputs. Specialist districts, corps/armies, religion and resource branches stay explicit.
use super::Error;
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash, unit_creation::signals::BuildMaps,
    unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    city_production::{Order, QueueEntry},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    player_districts::District,
};

struct Unit {
    hash: u32,
    index: usize,
    sea: bool,
    combat: bool,
    civilian: bool,
    major_only: bool,
    prerequisites: [Option<usize>; 2],
    obsolete: [Option<usize>; 4],
    upgrades: Vec<u32>,
    replacements: Vec<u32>,
    ordinary_land: bool,
    buildings: bool,
    hero: bool,
    religion: bool,
    religious_strength: i32,
    inquisition: bool,
    spy: bool,
    trade: bool,
    district: bool,
    strategic: bool,
    charges: i32,
    population: Option<i32>,
    population_cost: Option<i32>,
}
pub struct Definitions {
    inputs: InputIdentities,
    units: Vec<Unit>,
    center_train_contaminated: bool,
    production_yield: i32,
    center_no_military_domain: bool,
    government_bonuses: crate::registry::LinuxDefinitionRegistry,
}
/// Retained mutable replacement owner and actual Fallout current values. These
/// are queried again for every decision/front entry; no cached legality bit.
pub struct Current<'a> {
    pub build_maps: &'a BuildMaps,
    pub fallout_turns: &'a [i32],
    pub civilization_levels: &'a [(i32, u32)],
}
fn number(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    sql_integer(row.get(column).map_err(Error::Rules)?).ok_or(Error::Unsupported(column))
}
fn optional_number(row: crate::Row<'_>, column: &'static str) -> Result<Option<i32>, Error> {
    let v = row.get(column).map_err(Error::Rules)?;
    if matches!(v, SqlValue::Null) {
        Ok(None)
    } else {
        sql_integer(v).map(Some).ok_or(Error::Unsupported(column))
    }
}
fn reference(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(linux_type_hash(v))),
        _ => Err(Error::Unsupported(column)),
    }
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let units = collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let techs =
            collection(rules, "Technologies", "TechnologyType").map_err(Error::Definitions)?;
        let civics = collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?;
        let table = |n| rules.gameplay().table(n).ok_or(Error::Unsupported(n));
        let mut values = Vec::new();
        for row in table("Units")?.rows() {
            let name = row.text("UnitType").map_err(Error::Rules)?;
            let hash = linux_type_hash(name.as_bytes());
            let mut major_only = false;
            for xp in table("Units_XP2")?.rows() {
                if xp.text("UnitType").map_err(Error::Rules)? == name {
                    major_only = number(xp, "MajorCivOnly")? != 0;
                }
            }
            let mut upgrades = Vec::new();
            for r in table("UnitUpgrades")?.rows() {
                if r.text("Unit").map_err(Error::Rules)? == name {
                    upgrades.push(
                        reference(r, "UpgradeUnit")?.ok_or(Error::Unsupported("upgrade Unit"))?,
                    );
                }
            }
            let mut replacements = Vec::new();
            for r in table("UnitReplaces")?.rows() {
                if r.text("ReplacesUnitType").map_err(Error::Rules)? == name {
                    replacements.push(
                        reference(r, "CivUniqueUnitType")?
                            .ok_or(Error::Unsupported("replacement Unit"))?,
                    );
                }
            }
            let mut buildings = false;
            for r in table("Unit_BuildingPrereqs")?.rows() {
                buildings |= r.text("Unit").map_err(Error::Rules)? == name;
            }
            let mut hero = false;
            for r in table("HeroClasses")?.rows() {
                hero |= reference(r, "UnitType")? == Some(hash);
            }
            let tr = |column| -> Result<Option<usize>, Error> {
                Ok(reference(row, column)?.and_then(|h| techs.find_index(h)))
            };
            let cr = |column| -> Result<Option<usize>, Error> {
                Ok(reference(row, column)?.and_then(|h| civics.find_index(h)))
            };
            values.push(Unit {
                hash,
                civilian: row.text("FormationClass").map_err(Error::Rules)?
                    == "FORMATION_CLASS_CIVILIAN",
                index: units
                    .find_index(hash)
                    .ok_or(Error::Unsupported("Unit definition index"))?,
                sea: row.text("Domain").map_err(Error::Rules)? == "DOMAIN_SEA",
                combat: number(row, "Combat")? > 0
                    || number(row, "RangedCombat")? > 0
                    || number(row, "Bombard")? > 0,
                major_only,
                prerequisites: [tr("PrereqTech")?, cr("PrereqCivic")?],
                obsolete: [
                    tr("MandatoryObsoleteTech")?,
                    cr("MandatoryObsoleteCivic")?,
                    tr("ObsoleteTech")?,
                    cr("ObsoleteCivic")?,
                ],
                upgrades,
                replacements,
                ordinary_land: row.text("Domain").map_err(Error::Rules)? == "DOMAIN_LAND"
                    && row.text("FormationClass").map_err(Error::Rules)?
                        == "FORMATION_CLASS_LAND_COMBAT",
                buildings,
                hero,
                religion: number(row, "EnabledByReligion")? != 0,
                religious_strength: number(row, "ReligiousStrength")?,
                inquisition: number(row, "RequiresInquisition")? != 0,
                spy: number(row, "Spy")? != 0,
                trade: number(row, "MakeTradeRoute")? != 0,
                district: reference(row, "PrereqDistrict")?.is_some(),
                strategic: reference(row, "StrategicResource")?.is_some(),
                charges: number(row, "BuildCharges")?,
                population: optional_number(row, "PrereqPopulation")?,
                population_cost: optional_number(row, "PopulationCost")?,
            });
        }
        let mut parameter = None;
        for row in table("GlobalParameters")?.rows() {
            if row.text("Name").map_err(Error::Rules)? == "CITY_CENTER_TRAIN_WHEN_CONTAMINATED" {
                parameter = sql_integer(row.get("Value").map_err(Error::Rules)?);
            }
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            units: values,
            center_no_military_domain: table("Districts")?
                .rows()
                .find(|row| row.text("DistrictType").ok() == Some("DISTRICT_CITY_CENTER"))
                .ok_or(Error::Unsupported("CityCenter definition"))?
                .text("MilitaryDomain")
                .map_err(Error::Rules)?
                == "NO_DOMAIN",
            production_yield: collection(rules, "Yields", "YieldType")
                .map_err(Error::Definitions)?
                .find_index(linux_type_hash(b"YIELD_PRODUCTION"))
                .ok_or(Error::Unsupported("Production Yield definition"))?
                as i32,
            government_bonuses: collection(rules, "GovernmentBonusNames", "GovernmentBonusType")
                .map_err(Error::Definitions)?,
            center_train_contaminated: parameter
                .ok_or(Error::Unsupported("CITY_CENTER_TRAIN_WHEN_CONTAMINATED"))?
                != 0,
        })
    }
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("current Train Player"))
}
pub(super) fn acquired(
    state: &LoadedState,
    owner: i32,
    index: usize,
    civic: bool,
) -> Result<bool, Error> {
    let p = state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("current Train Player mask"))?;
    let mask = if civic { &p.civics } else { &p.technologies };
    mask.as_ref()
        .and_then(|v| v.get(index))
        .copied()
        .ok_or(Error::Unsupported("current Train prerequisite mask"))
}
fn find_district(state: &LoadedState, owner: i32, id: u32) -> Result<Option<&District>, Error> {
    if id == u32::MAX {
        return Ok(None);
    }
    Ok(player(state, owner)?
        .districts
        .records
        .entries
        .iter()
        .find(|e| e.index == (id & 0xffff) as i32)
        .map(|e| &e.value))
}
fn center_status(
    state: &LoadedState,
    owner: i32,
    city: u32,
    current: &Current<'_>,
) -> Result<(bool, bool), Error> {
    let c = state.city(owner, city)?;
    let location = (c.header.location_x, c.header.location_y);
    let mut center = None;
    for id in &c.districts.district_ids {
        if let Some(d) = find_district(state, owner, *id)? {
            if d.location == location {
                center = Some(d.district_hash);
                break;
            }
        }
    }
    let Some(hash) = center else {
        return Ok((false, false));
    };
    let mut all_pillaged = false;
    let mut contamination = false;
    for id in &c.districts.district_ids {
        if let Some(d) = find_district(state, owner, *id)? {
            if d.district_hash == hash {
                all_pillaged = true;
                if !d.pillaged {
                    all_pillaged = false;
                    break;
                }
            }
        }
    }
    for id in &c.districts.district_ids {
        if let Some(d) = find_district(state, owner, *id)? {
            if d.district_hash == hash {
                let plot = super::buildings::plot_index(state, [d.location.0, d.location.1])?;
                let turns = *current
                    .fallout_turns
                    .get(plot as usize)
                    .ok_or(Error::Unsupported("current Train Fallout cell"))?;
                if turns > 0 {
                    contamination = true;
                    break;
                }
            }
        }
    }
    Ok((all_pillaged, contamination))
}
// GetUnitProductionCost2855466..28554f6 subtracts the integer part of
// modifier * FixedPoint(cost). Keep both native low-word Q8 wrapping steps.
fn favored_cost(cost: i32, civilian: bool, production_favored: bool, modifier: i32) -> i32 {
    if cost <= 0 || civilian || !production_favored || modifier == 0 {
        return cost;
    }
    cost.wrapping_sub(
        c6_core::city_production::execution::multiply(modifier, cost.wrapping_shl(8)) >> 8,
    )
}
fn build_mapping(current: &Current<'_>, owner: i32, index: usize) -> Result<i32, Error> {
    current
        .build_maps
        .players
        .iter()
        .find(|p| p.0 == owner)
        .and_then(|p| p.1.get(index))
        .copied()
        .ok_or(Error::Unsupported("current Unit build replacement cell"))
}
fn prerequisites_met(state: &LoadedState, owner: i32, unit: &Unit) -> Result<bool, Error> {
    let mut met = true;
    for (i, reference) in unit.prerequisites.iter().enumerate() {
        if let Some(index) = reference {
            met &= acquired(state, owner, *index, i == 1)?;
        }
    }
    Ok(met)
}
fn available_upgrade(
    state: &LoadedState,
    owner: i32,
    unit: &Unit,
    defs: &Definitions,
    current: &Current<'_>,
) -> Result<bool, Error> {
    //285122d..285139b: resolve each upgrade's first trainable replacement,
    // then inspect prerequisite masks and CanEverTrain. Unreached resource
    // branches must not reject an ordinary ancient-era Scout or Warrior.
    for hash in &unit.upgrades {
        let original = defs
            .units
            .iter()
            .find(|u| u.hash == *hash)
            .ok_or(Error::Unsupported("upgrade Unit definition"))?;
        let mut candidate = original;
        for replacement in &original.replacements {
            let replacement = defs
                .units
                .iter()
                .find(|u| u.hash == *replacement)
                .ok_or(Error::Unsupported("replacement Unit definition"))?;
            if build_mapping(current, owner, replacement.index)? == replacement.index as i32 {
                //2bf46e2 chooses the first CanEverTrain replacement; a
                // missing prerequisite returns -1, rather than trying more.
                if prerequisites_met(state, owner, replacement)? {
                    candidate = replacement;
                }
                break;
            }
        }
        if !prerequisites_met(state, owner, candidate)?
            || build_mapping(current, owner, candidate.index)? != candidate.index as i32
        {
            continue;
        }
        let major = current
            .civilization_levels
            .iter()
            .find(|p| p.0 == owner)
            .ok_or(Error::Unsupported("current Train civilization level"))?
            .1
            == linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV");
        if !major {
            continue;
        }
        if candidate.strategic {
            return Err(Error::Unsupported(
                "available upgrade strategic-resource amount",
            ));
        }
        return Ok(true);
    }
    Ok(false)
}
fn required_population(prerequisite: Option<i32>, cost: Option<i32>) -> i32 {
    let mut minimum = prerequisite.unwrap_or(0);
    if let Some(cost) = cost {
        if cost >= minimum {
            minimum = cost.wrapping_add(1);
        }
    }
    minimum
}
fn civilian_center_allowed(pillaged: bool, contaminated: bool, charges: i32) -> bool {
    (charges > 0 || !contaminated) && (contaminated || !pillaged)
}
/// Native ordinary query flags: visible=false, prerequisites not ignored, no
/// diagnostic DTO. Ordinary single formations use -1/0; corps/armies remain explicit.
pub fn can_train(
    state: &LoadedState,
    owner: i32,
    city: u32,
    unit: u32,
    formation: i32,
    defs: &Definitions,
    current: &Current<'_>,
) -> Result<bool, Error> {
    can_train_inner(state, owner, city, unit, formation, defs, current, false)
}
#[allow(clippy::too_many_arguments)]
fn can_train_inner(
    state: &LoadedState,
    owner: i32,
    city: u32,
    unit: u32,
    formation: i32,
    defs: &Definitions,
    current: &Current<'_>,
    ignore_resources: bool,
) -> Result<bool, Error> {
    if state.binding().inputs != defs.inputs || current.build_maps.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let c = state.city(owner, city)?;
    if c.header.delayed_death {
        return Ok(false);
    }
    let Some(d) = defs.units.iter().find(|d| d.hash == unit) else {
        return Ok(false);
    };
    let mapped = current
        .build_maps
        .players
        .iter()
        .find(|p| p.0 == owner)
        .and_then(|p| p.1.get(d.index))
        .copied()
        .ok_or(Error::Unsupported("current Unit build replacement cell"))?;
    if mapped != d.index as i32 {
        return Ok(false);
    }
    let disabled = player(state, owner)?
        .units
        .unit_build_disabled
        .iter()
        .rev()
        .find(|v| v.definition_hash == unit)
        .ok_or(Error::Unsupported("current Unit build-disabled cell"))?
        .value;
    if disabled {
        return Ok(false);
    }
    if d.sea {
        return Err(Error::Unsupported("CanBuildUnitsOfDomain sea"));
    }
    if d.major_only {
        let level = current
            .civilization_levels
            .iter()
            .find(|p| p.0 == owner)
            .ok_or(Error::Unsupported("current Train civilization level"))?
            .1;
        if level != linux_type_hash(b"CIVILIZATION_LEVEL_FULL_CIV") {
            return Ok(false);
        }
    }
    for (i, reference) in d.prerequisites.iter().enumerate() {
        if let Some(index) = reference {
            if !acquired(state, owner, *index, i == 1)? {
                return Ok(false);
            }
        }
    }
    for (i, reference) in d.obsolete.iter().enumerate() {
        if let Some(index) = reference {
            if acquired(state, owner, *index, i % 2 == 1)? {
                return Ok(false);
            }
        }
    }
    if available_upgrade(state, owner, d, defs, current)? {
        return Ok(false);
    }
    if d.buildings {
        return Err(Error::Unsupported(
            "Unit prerequisite-building support slots",
        ));
    }
    if d.hero {
        return Err(Error::Unsupported("Hero CanTrain"));
    }
    if d.religion || d.religious_strength > 0 || d.inquisition {
        return Err(Error::Unsupported("religious Unit CanTrain"));
    }
    if d.spy {
        return Err(Error::Unsupported("spy Unit capacity"));
    }
    if d.trade {
        return Err(Error::Unsupported("trade Unit capacity"));
    }
    if d.district {
        return Err(Error::Unsupported("Unit prerequisite district"));
    }
    let (pillaged, contaminated) = center_status(state, owner, city, current)?;
    let contaminated = contaminated && !defs.center_train_contaminated;
    if d.combat {
        if !d.ordinary_land || !defs.center_no_military_domain {
            return Err(Error::Unsupported("non-land military Unit eligibility"));
        }
        //285199f..2851e1f: CITY_CENTER has NO_DOMAIN, so a city with
        // only its center takes the center fallback, as noncombat units do.
        // Other districts may supply military domains and enemy occupation.
        for id in &c.districts.district_ids {
            let district = find_district(state, owner, *id)?
                .ok_or(Error::Unsupported("current military district"))?;
            if district.district_hash != linux_type_hash(b"DISTRICT_CITY_CENTER") {
                return Err(Error::Unsupported(
                    "military Unit specialist district eligibility",
                ));
            }
        }
        if !matches!(formation, -1 | 0) {
            return Err(Error::Unsupported("military Unit corps/army eligibility"));
        }
    }
    if !civilian_center_allowed(pillaged, contaminated, d.charges) {
        return Ok(false);
    }
    if c.header.population < required_population(d.population, d.population_cost) {
        return Ok(false);
    }
    if d.strategic && !ignore_resources {
        return Err(Error::Unsupported("Unit strategic-resource availability"));
    }
    Ok(matches!(formation, -1 | 0))
}
/// External MakeRequest(UnitTypes)2822300 picks a single Unit formation0.
/// The same QueueEffects owner performs source NewFront callbacks and reports.
#[allow(clippy::too_many_arguments)]
pub fn append_unit(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    unit: u32,
    defs: &Definitions,
    current: &Current<'_>,
    effects: &mut impl super::QueueEffects,
) -> Result<(), Error> {
    if !can_train(state, owner, city, unit, 0, defs, current)? {
        return Err(Error::IllegalSelection(Order::Train {
            unit_hash: unit,
            military_formation: 0,
        }));
    }
    super::append_selected(state, owner, city, QueueEntry::train_request(unit), effects)
}
/// Shared GetUnitProductionCost28553d0 query, without progress modifiers.
pub fn production_cost(
    state: &LoadedState,
    owner: i32,
    unit_hash: u32,
    formation: i32,
    definitions: &Definitions,
) -> Result<i32, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    if !matches!(formation, -1 | 0) {
        return Err(Error::Unsupported("corps/army Unit cost"));
    }
    let definition = definitions
        .units
        .iter()
        .find(|d| d.hash == unit_hash)
        .ok_or(Error::Unsupported("current Unit definition"))?;
    if !definition.civilian && !definition.ordinary_land {
        return Err(Error::Unsupported("nonordinary Unit production cost"));
    }
    let player = player(state, owner)?;
    let cost = player
        .units
        .current_unit_cost
        .iter()
        .rev()
        .find(|v| v.definition_hash == unit_hash)
        .ok_or(Error::Unsupported("current Unit production array cell"))?
        .value;
    Ok(favored_cost(
        cost,
        definition.civilian,
        player.units.favored_production_yield == definitions.production_yield,
        player.units.favored_production_modifier_q8,
    ))
}

/// Current required cost28553d0 and UpdateUnitProgress284f8c0 factors for
/// ordinary civilian and land combat single formations. Cost is the player's canonical cached
/// integer; no native threshold or static hardcoded Unit cost is accepted.
pub fn current_build(
    state: &LoadedState,
    owner: i32,
    city: u32,
    entry: &QueueEntry,
    definitions: &Definitions,
) -> Result<super::CurrentBuild, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    let Order::Train {
        unit_hash,
        military_formation,
    } = entry.order
    else {
        return Err(Error::Unsupported("non-Train current build"));
    };
    if !matches!(military_formation, -1 | 0) {
        return Err(Error::Unsupported("corps/army Unit cost"));
    }
    let player = player(state, owner)?;
    let queue = &state.city(owner, city)?.build_queue;
    let cell = |values: &[c6_core::unit::HashValue<i32>], hash: u32| -> Result<i32, Error> {
        values
            .iter()
            .rev()
            .find(|v| v.definition_hash == hash)
            .map(|v| v.value)
            .ok_or(Error::Unsupported("current Unit production array cell"))
    };
    let cost = production_cost(state, owner, unit_hash, military_formation, definitions)?;
    let multiplier = cell(&player.units.unit_production_multiplier_q8, unit_hash)?
        .wrapping_add(cell(&queue.unit_production_multiplier, unit_hash)?);
    let bonus = |hash| -> Result<i32, Error> {
        if player.culture.anarchy_end_turn >= 0
            || definitions.government_bonuses.find_index(hash).is_none()
        {
            return Ok(0);
        }
        cell(&player.bonuses.flat_bonuses, hash)
    };
    let total = bonus(0xc1a005d2)?.wrapping_add(bonus(0x3d5ff1a2)?);
    let factor = c6_core::city_production::execution::multiply(
        multiplier,
        c6_core::city_production::execution::percent_multiplier(total),
    );
    Ok(super::CurrentBuild {
        cost,
        factors_q8: vec![factor],
    })
}

/// UpdateUnitProgress284f8c0 reserves the current strategic resource and emits
/// Climate accounting only when the selected Unit has StrategicResource.
/// Resource-free Units have no mutation in that prefix; progress factors are
/// computed separately by current_build from the same canonical state.
pub fn start_progress(
    state: &LoadedState,
    owner: i32,
    city: u32,
    entry: &QueueEntry,
    definitions: &Definitions,
) -> Result<(), Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    state.city(owner, city)?;
    let Order::Train { unit_hash, .. } = entry.order else {
        return Err(Error::Unsupported("non-Train progress start"));
    };
    let unit = definitions
        .units
        .iter()
        .find(|d| d.hash == unit_hash)
        .ok_or(Error::Unsupported("current Unit definition"))?;
    if unit.strategic {
        return Err(Error::Unsupported(
            "Unit progress strategic resource and Climate accounting",
        ));
    }
    Ok(())
}

pub trait FrontEffects {
    fn unit_front_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        request_id: u32,
        unit_index: i32,
        formation: i32,
    ) -> Result<(), Error>;
}
///285abd0: recheck eligibility with resource-ignore=true, then reserve resources
/// if needed and emit2842ce14. Resource-free current ordinary Units need no
/// reservation. Unsupported invalid-front removal is never silently accepted.
pub fn new_front(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    defs: &Definitions,
    current: &Current<'_>,
    effects: &mut impl FrontEffects,
) -> Result<(), Error> {
    let entry = state
        .city(owner, city)?
        .build_queue
        .entries
        .first()
        .cloned()
        .ok_or(Error::Unsupported("empty Unit new front"))?;
    let Order::Train {
        unit_hash,
        military_formation,
    } = entry.order
    else {
        return Err(Error::Unsupported("non-Train new front"));
    };
    let d = defs
        .units
        .iter()
        .find(|d| d.hash == unit_hash)
        .ok_or(Error::Unsupported("current Unit front definition"))?;
    if !can_train_inner(
        state,
        owner,
        city,
        unit_hash,
        military_formation,
        defs,
        current,
        true,
    )? {
        return Err(Error::Unsupported("illegal Unit front removal"));
    }
    if d.strategic {
        return Err(Error::Unsupported(
            "Unit front strategic resource reservation",
        ));
    }
    effects.unit_front_report(
        state,
        owner,
        city,
        entry.request_id,
        d.index as i32,
        military_formation,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn population_cost_uses_native_comparison_before_wrapping_increment() {
        assert_eq!(required_population(None, None), 0);
        assert_eq!(required_population(Some(3), Some(2)), 3);
        assert_eq!(required_population(Some(2), Some(2)), 3);
        assert_eq!(required_population(None, Some(-1)), 0);
        assert_eq!(
            required_population(Some(i32::MAX), Some(i32::MAX)),
            i32::MIN
        );
    }
    #[test]
    fn civilian_builder_charge_exception_preserves_pillage_contamination_order() {
        assert!(civilian_center_allowed(false, false, 0));
        assert!(!civilian_center_allowed(true, false, 3));
        assert!(!civilian_center_allowed(false, true, 0));
        assert!(civilian_center_allowed(false, true, 3));
        assert!(civilian_center_allowed(true, true, 3));
    }
    #[test]
    fn military_favored_cost_preserves_q8_fraction_and_native_overflow() {
        assert_eq!(favored_cost(30, false, true, 64), 23);
        assert_eq!(favored_cost(30, false, true, -64), 38);
        assert_eq!(favored_cost(30, true, true, 64), 30);
        assert_eq!(favored_cost(30, false, false, 64), 30);
        assert_eq!(favored_cost(-1, false, true, 64), -1);
        // FixedPoint(int) wraps before multiplication, and the final Q8
        // product wraps before AsInt. These are not a wide integer percent.
        assert_eq!(favored_cost(8_388_608, false, true, 64), 10_485_760);
        assert_eq!(favored_cost(30, false, true, 0), 30);
    }
}
