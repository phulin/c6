//! Current city Power activation with admitted native-order rules.
//! Wire hashes are explicitly resolved to runtime indices. Projection of saved
//! fields alone does not establish a final native initialized/current city.
pub mod loaded_gossip;

use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{collection, Definitions as ResourceDefinitions},
    Rules, SqlValue,
};
use c6_core::{
    city_power_activation::{self as core, Source, State},
    city_services::Power,
    movement_world::DefinitionLookup,
};
#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    Resources(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
    Index,
    Allocation(std::collections::TryReserveError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city power: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy)]
pub struct BuildingPower {
    base_index: Option<i32>,
    required: i32,
}
pub struct Definitions {
    binding: Option<c6_core::core_player::initialization::WireBinding>,
    buildings: LinuxDefinitionRegistry,
    projects: LinuxDefinitionRegistry,
    xp_projects: LinuxDefinitionRegistry,
    building_power: Vec<BuildingPower>,
    project_power: Vec<i32>,
    project_fully_powered: Vec<Option<bool>>,
    gossip: LinuxDefinitionRegistry,
    gossip_properties: Vec<c6_core::city_power_activation::gossip_delivery::Definition>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules, authority: &ResourceAuthority<'_>) -> Result<Self, Error> {
        ResourceDefinitions::validate_resources(rules, authority).map_err(Error::Resources)?;
        let buildings = collection(rules, "Buildings", "BuildingType").map_err(Error::Resources)?;
        let projects = collection(rules, "Projects", "ProjectType").map_err(Error::Resources)?;
        let xp_buildings =
            collection(rules, "Buildings_XP2", "BuildingType").map_err(Error::Resources)?;
        let xp_projects =
            collection(rules, "Projects_XP2", "ProjectType").map_err(Error::Resources)?;
        let integer = |table: &'static str, rowid, column: &'static str| -> Result<i32, Error> {
            let row = rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing(table))?
                .row_by_id(rowid)
                .ok_or(Error::Missing("definition row"))?;
            match row.get(column).map_err(Error::Rules)? {
                SqlValue::Null => Ok(0),
                SqlValue::Integer(n) => Ok(*n as i32),
                _ => Err(Error::Missing("noninteger power requirement")),
            }
        };
        let building_power = xp_buildings
            .entries()
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                Ok(BuildingPower {
                    base_index: buildings
                        .find_index(xp_buildings.type_hash(index).ok_or(Error::Index)?)
                        .map(|index| index as i32),
                    required: integer("Buildings_XP2", entry.database_rowid, "RequiredPower")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let project_power = xp_projects
            .entries()
            .iter()
            .map(|entry| {
                integer(
                    "Projects_XP2",
                    entry.database_rowid,
                    "RequiredPowerWhileActive",
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let project_fully_powered = xp_projects
            .entries()
            .iter()
            .map(|entry| {
                let row = rules
                    .gameplay()
                    .table("Projects_XP2")
                    .ok_or(Error::Missing("Projects_XP2"))?
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Project_XP2 row"))?;
                match row.get("FullyPoweredWhileActive").map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    SqlValue::Integer(value) => Ok(Some(*value != 0)),
                    _ => Err(Error::Missing("FullyPoweredWhileActive boolean")),
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let gossip = collection(rules, "Gossips", "GossipType").map_err(Error::Resources)?;
        let gossip_properties = gossip
            .entries()
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                Ok(
                    c6_core::city_power_activation::gossip_delivery::Definition {
                        type_hash: gossip.type_hash(index).ok_or(Error::Index)?,
                        visibility_level: integer(
                            "Gossips",
                            entry.database_rowid,
                            "VisibilityLevel",
                        )?,
                        filter: integer("Gossips", entry.database_rowid, "Filter")? != 0,
                    },
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: Some(authority.destination_binding().clone()),
            buildings,
            projects,
            xp_projects,
            building_power,
            project_power,
            project_fully_powered,
            gossip,
            gossip_properties,
        })
    }
    pub fn gossip_definition(
        &self,
        key: u32,
    ) -> Option<c6_core::city_power_activation::gossip_delivery::Definition> {
        self.gossip
            .find_index(key)
            .and_then(|index| self.gossip_properties.get(index))
            .copied()
    }

    /// Exact lazy IsFullyPowered. The current project is not queried unless
    /// missing raw Q8 power is positive; it is an index, not a saved hash.
    pub fn is_fully_powered(
        &self,
        power: &State,
        current_project: impl FnOnce() -> Result<i32, Error>,
    ) -> Result<bool, Error> {
        if power.missing_power_raw_q8() <= 0 {
            return Ok(true);
        }
        let index = current_project()?;
        if index == -1 {
            return Ok(false);
        }
        let project = self
            .projects
            .find_index(index as u32)
            .and_then(|index| self.projects.type_hash(index))
            .and_then(|hash| self.xp_projects.find_index(hash));
        Ok(project
            .and_then(|index| self.project_fully_powered.get(index))
            .copied()
            .flatten()
            .unwrap_or(false))
    }

    /// Copies exactly the represented scalar values and performs the source
    /// enum lookup for each saved source key, preserving order and duplicates.
    /// The caller supplies authority for all native Read/current-state effects.
    pub fn project_fields(&self, power: &Power) -> Result<State, Error> {
        let mut required_sources = Vec::new();
        required_sources
            .try_reserve(power.required_power_sources.len())
            .map_err(Error::Allocation)?;
        for source in &power.required_power_sources {
            required_sources.push(Source {
                amount_raw_q8: source.amount_raw_q8,
                building: self
                    .buildings
                    .find_index(source.building_source_hash)
                    .map_or(-1, |i| i as i32),
                project: self
                    .projects
                    .find_index(source.project_source_hash)
                    .map_or(-1, |i| i as i32),
            });
        }
        Ok(State {
            free_raw_q8: power.free_power_raw_q8,
            temporary_raw_q8: power.temporary_power_raw_q8,
            required_raw_q8: power.required_power_total_raw_q8,
            modifier_required_raw_q8: power.required_power_from_modifiers_raw_q8,
            modified_free_percent: power.modified_free_power,
            ever_generated: power.ever_generated_power,
            required_sources,
        })
    }
}

/// Actual current logical fields/continuations. The owning caller must keep
/// these in its city, not a disposable snapshot when executing gameplay.
/// Signal/report callbacks may mutate them; the core re-reads after delivery.
pub trait Live {
    fn power(&mut self) -> Result<&mut State, Error>;
    fn owner_is_major(&mut self) -> Result<bool, Error>;
    fn building_locations(&mut self) -> Result<&[i16], Error>;
    fn pillaged_buildings(&mut self) -> Result<&[bool], Error>;
    fn current_project(&mut self) -> Result<i32, Error>;
    fn game_present(&mut self) -> Result<bool, Error>;
    fn generated_power_signal(&mut self, first: bool) -> Result<(), Error>;
    fn shortage_changed_report(&mut self) -> Result<(), Error>;
}
struct Context<'a, L> {
    definitions: &'a Definitions,
    live: &'a mut L,
}
impl<L: Live> core::Context for Context<'_, L> {
    type Error = Error;
    type Building = BuildingPower;
    type Project = usize;
    fn power(&mut self) -> Result<&mut State, Error> {
        self.live.power()
    }
    fn allocation_error(&mut self, error: std::collections::TryReserveError) -> Error {
        Error::Allocation(error)
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        self.live.owner_is_major()
    }
    fn buildings(&mut self) -> Result<&[BuildingPower], Error> {
        Ok(&self.definitions.building_power)
    }
    fn required_building_power(&mut self, b: BuildingPower) -> Result<i32, Error> {
        Ok(b.required)
    }
    fn base_building(&mut self, b: BuildingPower) -> Result<Option<i32>, Error> {
        Ok(b.base_index)
    }
    fn has_building(&mut self, index: i32) -> Result<bool, Error> {
        if index == -1 {
            return Ok(false);
        }
        let index = usize::try_from(index).map_err(|_| Error::Index)?;
        Ok(*self
            .live
            .building_locations()?
            .get(index)
            .ok_or(Error::Index)?
            >= 0)
    }
    fn building_pillaged(&mut self, index: i32) -> Result<bool, Error> {
        let values = self.live.pillaged_buildings()?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| values.get(index))
            .copied()
            .unwrap_or(false))
    }
    fn current_project(&mut self) -> Result<i32, Error> {
        self.live.current_project()
    }
    fn project_definition(&mut self, index: i32) -> Result<Option<usize>, Error> {
        Ok(self
            .definitions
            .projects
            .find_index(index as u32)
            .and_then(|index| self.definitions.projects.type_hash(index))
            .and_then(|hash| self.definitions.xp_projects.find_index(hash)))
    }
    fn required_project_power(&mut self, index: usize) -> Result<i32, Error> {
        self.definitions
            .project_power
            .get(index)
            .copied()
            .ok_or(Error::Index)
    }
    fn game_present(&mut self) -> Result<bool, Error> {
        self.live.game_present()
    }
    fn generated_power_signal(&mut self, first: bool) -> Result<(), Error> {
        self.live.generated_power_signal(first)
    }
    fn power_shortage_changed_report(&mut self) -> Result<(), Error> {
        self.live.shortage_changed_report()
    }
}
pub fn recalculate<L: Live>(definitions: &Definitions, live: &mut L) -> Result<(), Error> {
    core::recalculate(&mut Context { definitions, live })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registry::DefinitionEntry, RulesRowId};
    fn registry(names: &[&str]) -> LinuxDefinitionRegistry {
        LinuxDefinitionRegistry::from_native_order(
            names
                .iter()
                .enumerate()
                .map(|(i, name)| DefinitionEntry {
                    database_rowid: RulesRowId(100 + i as i64),
                    type_name: name.as_bytes().to_vec(),
                })
                .collect(),
        )
    }
    fn definitions() -> Definitions {
        Definitions {
            binding: None,
            buildings: registry(&["BUILDING_FIRST", "BUILDING_SECOND"]),
            projects: registry(&["PROJECT_TEST"]),
            xp_projects: registry(&["PROJECT_TEST"]),
            building_power: vec![BuildingPower {
                base_index: Some(1),
                required: 3,
            }],
            project_power: vec![2],
            project_fully_powered: vec![Some(true)],
            gossip: registry(&["GOSSIP_SYNTHETIC"]),
            gossip_properties: vec![
                c6_core::city_power_activation::gossip_delivery::Definition {
                    type_hash: crate::registry::linux_type_hash(b"GOSSIP_SYNTHETIC"),
                    visibility_level: 2,
                    filter: false,
                },
            ],
        }
    }
    #[test]
    fn gossip_lookup_uses_native_index_or_hash_and_explicit_miss() {
        let defs = definitions();
        let hash = crate::registry::linux_type_hash(b"GOSSIP_SYNTHETIC");
        let by_index = defs.gossip_definition(0).unwrap();
        assert_eq!(defs.gossip_definition(hash), Some(by_index));
        assert_eq!(by_index.type_hash, hash);
        assert_eq!(by_index.visibility_level, 2);
        assert!(!by_index.filter);
        assert_eq!(defs.gossip_definition(u32::MAX), None);
    }

    #[test]
    fn saved_hashes_resolve_to_native_indices_not_row_ids() {
        let defs = definitions();
        let power = Power {
            wire_version: 10,
            free_power_raw_q8: 9,
            temporary_power_raw_q8: 4,
            required_power_total_raw_q8: 3,
            required_power_from_modifiers_raw_q8: 2,
            ever_generated_power: false,
            turn_last_notified_need_power: 123,
            modified_free_power: 10,
            free_power_sources: vec![],
            temporary_power_sources: vec![],
            generated_power_sources: vec![],
            required_power_sources: vec![
                c6_core::city_services::RequiredPowerSource {
                    amount_raw_q8: 11,
                    building_source_hash: defs.buildings.type_hash(1).unwrap(),
                    project_source_hash: u32::MAX,
                },
                c6_core::city_services::RequiredPowerSource {
                    amount_raw_q8: 12,
                    building_source_hash: 0,
                    project_source_hash: 0,
                },
            ],
        };
        let state = defs.project_fields(&power).unwrap();
        assert_eq!(
            state.required_sources,
            vec![
                Source {
                    amount_raw_q8: 11,
                    building: 1,
                    project: -1
                },
                Source {
                    amount_raw_q8: 12,
                    building: 0,
                    project: 0
                }
            ]
        );
        assert_eq!(power.turn_last_notified_need_power, 123);
        assert_eq!(state.modified_free_percent, 10);
    }
    #[test]
    fn fully_powered_short_circuits_and_resolves_optional_active_project() {
        let mut defs = definitions();
        let mut state = city().state;
        assert!(defs
            .is_fully_powered(&state, || panic!("project queried without shortage"))
            .unwrap());
        state.required_raw_q8 = 999;
        assert!(defs.is_fully_powered(&state, || Ok(0)).unwrap());
        assert!(!defs.is_fully_powered(&state, || Ok(-1)).unwrap());
        defs.project_fully_powered[0] = None;
        assert!(!defs.is_fully_powered(&state, || Ok(0)).unwrap());
        assert!(matches!(
            defs.is_fully_powered(&state, || Err(Error::Missing("project"))),
            Err(Error::Missing("project"))
        ));
    }

    struct City {
        state: State,
        events: Vec<&'static str>,
        unavailable_pillaged: bool,
    }
    impl Live for City {
        fn power(&mut self) -> Result<&mut State, Error> {
            Ok(&mut self.state)
        }
        fn owner_is_major(&mut self) -> Result<bool, Error> {
            Ok(true)
        }
        fn building_locations(&mut self) -> Result<&[i16], Error> {
            Ok(&[-1, 4])
        }
        fn pillaged_buildings(&mut self) -> Result<&[bool], Error> {
            if self.unavailable_pillaged {
                Err(Error::Missing("pillaged"))
            } else {
                Ok(&[false, false])
            }
        }
        fn current_project(&mut self) -> Result<i32, Error> {
            Ok(0)
        }
        fn game_present(&mut self) -> Result<bool, Error> {
            Ok(true)
        }
        fn generated_power_signal(&mut self, first: bool) -> Result<(), Error> {
            assert!(first);
            self.events.push("signal");
            self.state.temporary_raw_q8 = 9999;
            Ok(())
        }
        fn shortage_changed_report(&mut self) -> Result<(), Error> {
            self.events.push("report");
            Ok(())
        }
    }
    fn city() -> City {
        City {
            state: State {
                free_raw_q8: 256,
                temporary_raw_q8: 0,
                required_raw_q8: 100,
                modifier_required_raw_q8: 128,
                modified_free_percent: 0,
                ever_generated: false,
                required_sources: vec![],
            },
            events: vec![],
            unavailable_pillaged: false,
        }
    }
    #[test]
    fn rules_backed_context_computes_requirements_before_real_signal_reread() {
        let mut city = city();
        recalculate(&definitions(), &mut city).unwrap();
        assert_eq!(city.state.required_raw_q8, 1408);
        assert_eq!(
            city.state.required_sources[0],
            Source {
                amount_raw_q8: 768,
                building: 1,
                project: -1
            }
        );
        assert_eq!(
            city.state.required_sources[1],
            Source {
                amount_raw_q8: 512,
                building: -1,
                project: 0
            }
        );
        assert!(city.state.ever_generated);
        assert_eq!(city.events, vec!["signal"]);
    }
    #[test]
    fn unavailable_pillaged_state_is_not_false() {
        let mut city = city();
        city.unavailable_pillaged = true;
        assert!(matches!(
            recalculate(&definitions(), &mut city),
            Err(Error::Missing("pillaged"))
        ));
        assert_eq!(city.state.required_raw_q8, 0);
        assert!(city.events.is_empty());
    }
}
