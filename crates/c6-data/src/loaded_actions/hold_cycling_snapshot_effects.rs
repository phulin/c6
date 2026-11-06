//! Source-bounded omission of discarded stock readiness output.
//!
//! This is not a CanCyclePast answer or action-legality provider. The Unit cache
//! snapshot discards these query results at the headless boundary. We may omit
//! its holding scans only when their complete reached gameplay/RNG effect
//! frontier is closed. Positive builder/religious/custom branches still fail.
use crate::loaded_actions::{transition::ScriptState, upgrade_actions};
use crate::{
    registry::{linux_type_hash as hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions as defs, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    script_event_registry::{GroupEntry, ScriptEventRegistry},
    unit::Unit,
};

type Result<T> = std::result::Result<T, &'static str>;

/// The cached builtin event has no reached listeners. Stock headless service
/// calls can add other event names, but never register new listeners. Replacing
/// the ScriptSystem or installing callbacks invalidates this qualification.
pub struct CachedEmptyOperationTests {
    binding: WireBinding,
}
impl CachedEmptyOperationTests {
    pub fn from_current(state: &LoadedState, script: &ScriptState) -> Result<Self> {
        if !script.matches(state) {
            return Err("snapshot ScriptSystem identity");
        }
        check_cached_empty(script.registry())?;
        Ok(Self {
            binding: state.binding().clone(),
        })
    }
}
fn check_cached_empty(registry: &ScriptEventRegistry) -> Result<()> {
    let event = registry
        .cached_event("CanStartOperation")
        .ok_or("snapshot CanStartOperation must already be cached")?;
    if event.test_all != event.methods[5] {
        return Err("snapshot CanStartOperation builtin binding");
    }
    let shared = registry
        .shared_group(event.shared_group)
        .ok_or("snapshot CanStartOperation shared group")?;
    for entry in &shared.entries_in_native_order {
        if let GroupEntry::Table(id) = entry {
            let local = registry
                .local_group(*id)
                .ok_or("snapshot CanStartOperation local group")?;
            if local.objlen != 0 {
                return Err("snapshot CanStartOperation callbacks require mutable execution");
            }
        }
    }
    Ok(())
}

pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    techs: LinuxDefinitionRegistry,
    civics: LinuxDefinitionRegistry,
    abilities: LinuxDefinitionRegistry,
    airlift_building: Option<u32>,
    water_terrains: Vec<u32>,
    warrior_properties: Option<c6_core::unit::variants::TypedVariantMap>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self> {
        if state.binding() != bound.destination_binding() {
            return Err("snapshot effects rules binding");
        }
        defs::Definitions::validate_resources(rules, bound)
            .map_err(|_| "snapshot effects resources")?;
        let units = defs::collection(rules, "Units", "UnitType").map_err(|_| "snapshot Units")?;
        let table = rules.gameplay().table("Units").ok_or("snapshot Units")?;
        let warrior = units
            .find(hash(b"UNIT_WARRIOR"))
            .ok_or("snapshot Warrior definition")?;
        let row = table
            .row_by_id(warrior.database_rowid)
            .ok_or("snapshot Warrior row")?;
        // These are the actual early subtype gates, not type-name inference.
        for key in [
            "Spy",
            "MakeTradeRoute",
            "FoundCity",
            "FoundReligion",
            "EvangelizeBelief",
            "LaunchInquisition",
            "ExtractsArtifacts",
            "WMDCapable",
        ] {
            if row.get(key).map_err(|_| "snapshot Warrior flag")? != &SqlValue::Integer(0) {
                return Err("snapshot positive specialized Unit definition");
            }
        }
        for key in ["RangedCombat", "Bombard"] {
            if row.get(key).map_err(|_| "snapshot Warrior strength")? != &SqlValue::Integer(0) {
                return Err("snapshot positive ranged Unit definition");
            }
        }
        let xp2 = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or("snapshot Units_XP2")?;
        if xp2
            .rows()
            .filter(|r| r.text("UnitType").ok() == Some("UNIT_WARRIOR"))
            .any(|r| r.get("TourismBombPossible").ok() != Some(&SqlValue::Integer(0)))
        {
            return Err("snapshot positive TourismBomb definition");
        }
        let routes = rules
            .gameplay()
            .table("Route_ValidBuildUnits")
            .ok_or("snapshot Route_ValidBuildUnits")?;
        if routes
            .rows()
            .any(|r| r.text("UnitType").ok() == Some("UNIT_WARRIOR"))
        {
            return Err("snapshot positive route builder");
        }
        // RegisterCommands/RegisterOperations are the stock XP2 lifecycle.
        // A new holding definition has an unreviewed effect frontier.
        for (table, key, allowed) in [
            ("UnitCommands", "CommandType", COMMANDS),
            ("UnitOperations", "OperationType", OPERATIONS),
        ] {
            for row in rules
                .gameplay()
                .table(table)
                .ok_or("snapshot holding definitions")?
                .rows()
            {
                match row
                    .get("HoldCycling")
                    .map_err(|_| "snapshot HoldCycling flag")?
                {
                    SqlValue::Integer(0) => (),
                    SqlValue::Integer(1) => {
                        if !allowed.contains(&row.text(key).map_err(|_| "snapshot holding type")?) {
                            return Err("snapshot custom holding subtype");
                        }
                    }
                    _ => return Err("snapshot HoldCycling boolean"),
                }
            }
        }
        let buildings = defs::collection(rules, "Buildings", "BuildingType")
            .map_err(|_| "snapshot Buildings")?;
        let bt = rules
            .gameplay()
            .table("Buildings")
            .ok_or("snapshot Buildings")?;
        let airlift_building = buildings
            .entries()
            .iter()
            .find(|e| {
                bt.row_by_id(e.database_rowid)
                    .and_then(|r| r.text("PrereqTech").ok())
                    == Some("TECH_ADVANCED_FLIGHT")
            })
            .map(|e| hash(&e.type_name));
        let terrains = rules
            .gameplay()
            .table("Terrains")
            .ok_or("snapshot Terrains")?;
        let water_terrains = terrains
            .rows()
            .map(|r| {
                let water = match r.get("Water").map_err(|_| "snapshot Terrain Water")? {
                    SqlValue::Integer(0) => false,
                    SqlValue::Integer(1) => true,
                    _ => return Err("snapshot Terrain Water boolean"),
                };
                let name = r.text("TerrainType").map_err(|_| "snapshot TerrainType")?;
                Ok(water.then(|| hash(name.as_bytes())))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(Self {
            binding: state.binding().clone(),
            units,
            water_terrains,
            warrior_properties: None,
            techs: defs::collection(rules, "Technologies", "TechnologyType")
                .map_err(|_| "snapshot Technologies")?,
            civics: defs::collection(rules, "Civics", "CivicType")
                .map_err(|_| "snapshot Civics")?,
            abilities: defs::collection(rules, "UnitAbilities", "UnitAbilityType")
                .map_err(|_| "snapshot UnitAbilities")?,
            airlift_building,
        })
    }
    /// Complete current stock Definition.IProperties map. No action in this
    /// profile changes definitions; the source teleport predicate reads this
    /// map, so SQL absence alone is insufficient.
    pub fn bind_current_unit_properties(
        &mut self,
        runtime: &crate::unit_creation::Runtime,
    ) -> Result<()> {
        if runtime.binding != self.binding || self.warrior_properties.is_some() {
            return Err("snapshot current definition property binding");
        }
        let index = self
            .units
            .find_index(hash(b"UNIT_WARRIOR"))
            .ok_or("snapshot Warrior index")?;
        let properties = runtime
            .definition_properties
            .as_ref()
            .and_then(|maps| maps.iter().find(|p| p.native_unit_index == index))
            .ok_or("snapshot current Warrior definition properties")?;
        self.warrior_properties = Some(properties.values.clone());
        Ok(())
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
}

#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub upgrade: &'a upgrade_actions::Definitions,
    pub build_maps: &'a crate::unit_creation::signals::BuildMaps,
    pub empty_operation_tests: &'a CachedEmptyOperationTests,
}
#[derive(Debug)]
pub enum CheckError {
    Unsupported(&'static str),
    Upgrade(upgrade_actions::Error<()>),
}
impl From<&'static str> for CheckError {
    fn from(value: &'static str) -> Self {
        Self::Unsupported(value)
    }
}
impl Inputs<'_> {
    /// Validate the current actor at every discarded snapshot. No supplied
    /// CanStart answers, policy choices, gameplay stores or RNG are involved.
    pub fn check(self, state: &LoadedState, unit: &Unit) -> std::result::Result<(), CheckError> {
        let d = self.definitions;
        if state.binding() != &d.binding || self.empty_operation_tests.binding != d.binding {
            return Err("snapshot effects current binding".into());
        }
        let properties = d
            .warrior_properties
            .as_ref()
            .ok_or("snapshot current Warrior definition properties")?;
        if properties.entries.iter().any(|e| e.key == 0x3f9140c1) {
            return Err("snapshot teleport-city definition property continuation".into());
        }
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(|_| "snapshot effects Unit runtime")?;
        if runtime.domain != 2
            || d.units
                .at(runtime.definition_index as u32)
                .map(|e| e.type_name.as_slice())
                != Some(b"UNIT_WARRIOR".as_slice())
            || unit.e_type_index != hash(b"UNIT_WARRIOR")
        {
            return Err("snapshot effects supports current land Warrior only".into());
        }
        if unit.i_build_charges > 0
            || unit.i_park_charges > 0
            || unit.k_great_person.e_individual != u32::MAX
            || unit.k_great_person.e_class != u32::MAX
            || unit.k_religion.i_spread_charges > 0
            || unit.k_religion.i_religious_heal_charges > 0
            || unit.k_religion.b_converts_barbarians
            || unit.k_movement_modifiers.i_jump_range > 0
            || unit.k_movement_modifiers.b_can_paradrop
            || unit.k_combat_modifers.b_conduct_raid
            || unit.k_combat_modifers.i_ranged_strength_modifier > 0
            || unit.k_combat_modifers.i_bombard_strength_modifier > 0
            || queried_ability_present(&d.abilities, &runtime.abilities)
        {
            return Err("snapshot positive specialized Warrior effects".into());
        }
        if !unit.k_operations.nodes.is_empty() {
            return Err("snapshot effects current operation interruption".into());
        }
        let map = state.world().map();
        let (plot_index, plot) = c6_core::movement_world::plot_at(map, unit.k_location)
            .map_err(|_| "snapshot effects current plot")?;
        if d.water_terrains.contains(&plot.terrain_hash)
            || crate::movement_placement_state::plot_owner(plot)
                .map_err(|_| "snapshot effects plot owner")?
                != unit.e_owner
        {
            return Err("snapshot effects requires own land plot".into());
        }
        // XP2 CondemnHeretic/PlunderTradeRoute first scan current cell owners.
        // With no foreign identity, no religion/diplomacy/output branch runs.
        for layer in &map.units.layers {
            for cell in layer
                .cells
                .iter()
                .filter(|c| c.plot_index as usize == plot_index)
            {
                if cell
                    .units
                    .iter()
                    .any(|r| i32::from(r.component_id().owner) != unit.e_owner)
                {
                    return Err("snapshot foreign current-cell command continuation".into());
                }
            }
        }
        if upgrade_actions::target::<()>(
            state,
            self.upgrade,
            self.build_maps,
            (unit.e_owner, unit.id),
        )
        .map_err(CheckError::Upgrade)?
        .is_some()
        {
            return Err("snapshot positive Upgrade continuation".into());
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == unit.e_owner)
            .ok_or("snapshot effects Player")?;
        let world_player = state
            .world()
            .player(unit.e_owner)
            .map_err(|_| "snapshot effects World Player")?;
        // Corps/Army reject before parameter/candidate scans when an unlock is
        // missing. GetCorpsArmyUnlock*, HasCivic and HasTech are scalar reads.
        for (civic, tech) in [
            (
                player.units.land_corps_unlock_civic_hash,
                player.units.land_corps_unlock_tech_hash,
            ),
            (
                player.units.land_army_unlock_civic_hash,
                player.units.land_army_unlock_tech_hash,
            ),
        ] {
            let missing = |registry: &LinuxDefinitionRegistry,
                           hash: u32,
                           mask: &Option<Vec<bool>>|
             -> Result<bool> {
                let Some(index) = registry.find_index(hash) else {
                    return Ok(false);
                };
                Ok(!*mask
                    .as_ref()
                    .and_then(|m| m.get(index))
                    .ok_or("snapshot military unlock mask")?)
            };
            if !missing(&d.civics, civic, &world_player.civics)?
                && !missing(&d.techs, tech, &world_player.technologies)?
            {
                return Err("snapshot unlocked military formation continuation".into());
            }
        }
        // Airlift's radius-one callback only reads District/City buildings or
        // improvement properties and writes its caller's stack boolean. Exclude
        // positive/custom property branches, retaining no temporary UI output.
        if let Some(building) = d.airlift_building {
            for p in &state.retained().players {
                if p.cities.records.entries.iter().any(|c| {
                    c.value
                        .buildings
                        .building_locations
                        .iter()
                        .any(|b| b.definition_hash == building && b.value >= 0)
                }) {
                    return Err("snapshot positive Airlift building continuation".into());
                }
            }
        }
        for (index, p) in map.plots.iter().enumerate() {
            let coordinate = c6_core::unit::PlotCoord {
                x: (index % map.header.width as usize) as i32,
                y: (index / map.header.width as usize) as i32,
            };
            if (coordinate == unit.k_location
                || c6_core::movement_world::adjacent(map, unit.k_location, coordinate)
                    .map_err(|_| "snapshot Airlift adjacency")?)
                && p.improvement_hash != u32::MAX
            {
                return Err("snapshot Airlift improvement property continuation".into());
            }
        }
        Ok(())
    }
}

// HasAbility (0x2c293b0) resolves the queried enum/hash through the actual
// ability collection, then checks that native-index count >0. Unrelated live
// abilities never enter these two rejected subtype prefixes.
fn queried_ability_present(registry: &LinuxDefinitionRegistry, counts: &[i32]) -> bool {
    [0x14a05859, hash(b"ABILITY_SPREAD_DISSENT")]
        .into_iter()
        .any(|key| {
            registry
                .find_index(key)
                .and_then(|index| counts.get(index))
                .is_some_and(|&count| count > 0)
        })
}

const COMMANDS: &[&str] = &[
    "UNITCOMMAND_UPGRADE",
    "UNITCOMMAND_ENTER_FORMATION",
    "UNITCOMMAND_EXIT_FORMATION",
    "UNITCOMMAND_ACTIVATE_GREAT_PERSON",
    "UNITCOMMAND_DISTRICT_PRODUCTION",
    "UNITCOMMAND_FORM_CORPS",
    "UNITCOMMAND_FORM_ARMY",
    "UNITCOMMAND_PLUNDER_TRADE_ROUTE",
    "UNITCOMMAND_CONDEMN_HERETIC",
    "UNITCOMMAND_WONDER_PRODUCTION",
    "UNITCOMMAND_HARVEST_WONDER",
    "UNITCOMMAND_AIRLIFT",
    "UNITCOMMAND_PARADROP",
    "UNITCOMMAND_PROJECT_PRODUCTION",
    "UNITCOMMAND_PRIORITY_TARGET",
    "UNITCOMMAND_SPREAD_DISSENT",
    "UNITCOMMAND_MOVE_JUMP",
    "UNITCOMMAND_BUILDING_PRODUCTION",
];
const OPERATIONS: &[&str] = &[
    "UNITOPERATION_AIR_ATTACK",
    "UNITOPERATION_BUILD_IMPROVEMENT",
    "UNITOPERATION_BUILD_ROUTE",
    "UNITOPERATION_CLEAR_CONTAMINATION",
    "UNITOPERATION_CONVERT_BARBARIANS",
    "UNITOPERATION_DEPLOY",
    "UNITOPERATION_DESIGNATE_PARK",
    "UNITOPERATION_EVANGELIZE_BELIEF",
    "UNITOPERATION_EXCAVATE",
    "UNITOPERATION_FOUND_CITY",
    "UNITOPERATION_FOUND_RELIGION",
    "UNITOPERATION_HARVEST_RESOURCE",
    "UNITOPERATION_LAUNCH_INQUISITION",
    "UNITOPERATION_MAKE_TRADE_ROUTE",
    "UNITOPERATION_WMD_STRIKE",
    "UNITOPERATION_COASTAL_RAID",
    "UNITOPERATION_PILLAGE",
    "UNITOPERATION_PILLAGE_ROUTE",
    "UNITOPERATION_PLANT_FOREST",
    "UNITOPERATION_RANGE_ATTACK",
    "UNITOPERATION_REBASE",
    "UNITOPERATION_REMOVE_FEATURE",
    "UNITOPERATION_REMOVE_HERESY",
    "UNITOPERATION_REMOVE_IMPROVEMENT",
    "UNITOPERATION_REPAIR",
    "UNITOPERATION_REPAIR_ROUTE",
    "UNITOPERATION_SPREAD_RELIGION",
    "UNITOPERATION_SPY_COUNTERSPY",
    "UNITOPERATION_SPY_DISRUPT_ROCKETRY",
    "UNITOPERATION_SPY_GAIN_SOURCES",
    "UNITOPERATION_SPY_GREAT_WORK_HEIST",
    "UNITOPERATION_SPY_LISTENING_POST",
    "UNITOPERATION_SPY_RECRUIT_PARTISANS",
    "UNITOPERATION_SPY_SABOTAGE_PRODUCTION",
    "UNITOPERATION_SPY_SIPHON_FUNDS",
    "UNITOPERATION_SPY_STEAL_TECH_BOOST",
    "UNITOPERATION_SPY_TRAVEL_NEW_CITY",
    "UNITOPERATION_TELEPORT_TO",
    "UNITOPERATION_TELEPORT_TO_CITY",
    "UNITOPERATION_UPGRADE",
    "UNITOPERATION_RELIGIOUS_HEAL",
    "UNITOPERATION_SPY_FABRICATE_SCANDAL",
    "UNITOPERATION_SPY_FOMENT_UNREST",
    "UNITOPERATION_SPY_NEUTRALIZE_GOVERNOR",
    "UNITOPERATION_BUILD_IMPROVEMENT_ADJACENT",
    "UNITOPERATION_TOURISM_BOMB",
    "UNITOPERATION_SPY_BREACH_DAM",
];

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::script_event_registry::{
        AppendOrder, EventIdentity, KnownNameEntry, LocalGroup, RegistrySeed, ScriptObjectId,
        SharedGroup,
    };
    use std::collections::BTreeMap;

    fn registry(local_length: usize, cached: bool) -> ScriptEventRegistry {
        let shared = ScriptObjectId(1);
        let local = ScriptObjectId(2);
        let methods = std::array::from_fn(|i| ScriptObjectId(10 + i as u64));
        let event = EventIdentity {
            event: ScriptObjectId(3),
            shared_group: shared,
            local_group: local,
            test_all: methods[5],
            methods,
        };
        ScriptEventRegistry::from_seed_with_cache(
            RegistrySeed {
                names: BTreeMap::from([(
                    "CanStartOperation".into(),
                    KnownNameEntry::Shared(shared),
                )]),
                shared_groups: BTreeMap::from([(
                    shared,
                    SharedGroup {
                        entries_in_native_order: vec![GroupEntry::Table(local)],
                        append_order: AppendOrder::DenseNumericArray,
                    },
                )]),
                local_groups: BTreeMap::from([(
                    local,
                    LocalGroup {
                        objlen: local_length,
                        numeric_slots: BTreeMap::new(),
                    },
                )]),
            },
            if cached {
                BTreeMap::from([("CanStartOperation".into(), event)])
            } else {
                BTreeMap::new()
            },
        )
        .unwrap()
    }
    #[test]
    fn subtype_ability_guards_read_only_their_actual_native_indices() {
        use crate::{registry::DefinitionEntry, RulesRowId};
        let registry = LinuxDefinitionRegistry::from_native_order(vec![
            DefinitionEntry {
                database_rowid: RulesRowId(1),
                type_name: b"UNRELATED_ABILITY".to_vec(),
            },
            DefinitionEntry {
                database_rowid: RulesRowId(2),
                type_name: b"ABILITY_SPREAD_DISSENT".to_vec(),
            },
        ]);
        assert!(!queried_ability_present(&registry, &[1, 0]));
        assert!(queried_ability_present(&registry, &[0, 1]));
        assert!(!queried_ability_present(&registry, &[1, -1]));
        assert!(!queried_ability_present(&registry, &[1]));
    }
    #[test]
    fn only_existing_empty_builtin_test_all_can_be_omitted_without_registry_changes() {
        let empty = registry(0, true);
        let before = empty.clone();
        assert_eq!(check_cached_empty(&empty), Ok(()));
        assert_eq!(empty, before);
        // A cache miss would create registry identities. An advertised slot
        // must remain in the mutable script executor even when its value is nil.
        assert!(check_cached_empty(&registry(0, false)).is_err());
        assert!(check_cached_empty(&registry(1, true)).is_err());
    }
}
