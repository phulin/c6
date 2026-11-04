//! Current district cost recomputation after research or city founding.
//! Empty rosters and non-specialty-only rosters have zero specialty average;
//! positive specialty discount/count branches remain explicit.
use crate::{
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
    research_acquisition::units::{self, Economic},
};
#[derive(Debug)]
pub enum Error {
    Binding,
    State(c6_core::loaded_state::Error),
    Rules(crate::Error),
    Definitions(unit_initialization_definitions::Error),
    Missing(&'static str),
    NonemptyDistricts,
    Progression(i32),
    Research(units::Error<std::convert::Infallible>),
}
#[derive(Clone, Copy)]
struct Definition {
    hash: u32,
    cost: i32,
    model: u32,
    parameter: i32,
    specialty: bool,
}
pub struct Definitions {
    binding: WireBinding,
    items: Vec<Definition>,
    economic: Economic,
    eras: crate::registry::LinuxDefinitionRegistry,
    starts: crate::registry::LinuxDefinitionRegistry,
    rates: Vec<i32>,
    technologies: crate::registry::LinuxDefinitionRegistry,
    civics: crate::registry::LinuxDefinitionRegistry,
}
fn integer(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    i32::try_from(row.integer(column).map_err(Error::Rules)?).map_err(|_| Error::Missing(column))
}
impl Definitions {
    pub fn from_rules(
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
        economic: Economic,
    ) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Definitions)?;
        let districts =
            collection(rules, "Districts", "DistrictType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Districts")
            .ok_or(Error::Missing("Districts"))?;
        let mut items = Vec::new();
        for entry in districts.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("district row"))?;
            let cost = integer(row, "Cost")?;
            let parameter = integer(row, "CostProgressionParam1")?;
            let model = match row.get("CostProgressionModel").map_err(Error::Rules)? {
                SqlValue::Text(name) => crate::registry::linux_type_hash(name),
                SqlValue::Null => 0,
                _ => return Err(Error::Missing("district progression model")),
            };
            items.push(Definition {
                hash: crate::registry::linux_type_hash(&entry.type_name),
                cost,
                model,
                parameter,
                specialty: integer(row, "RequiresPopulation")? != 0,
            });
        }
        let eras = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let starts = collection(rules, "StartEras", "EraType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("StartEras")
            .ok_or(Error::Missing("StartEras"))?;
        let rates = starts
            .entries()
            .iter()
            .map(|entry| {
                integer(
                    table
                        .row_by_id(entry.database_rowid)
                        .ok_or(Error::Missing("starting era row"))?,
                    "DistrictProductionRate",
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: bound.destination_binding().clone(),
            items,
            economic,
            eras,
            starts,
            rates,
            technologies: collection(rules, "Technologies", "TechnologyType")
                .map_err(Error::Definitions)?,
            civics: collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?,
        })
    }
    /// Existing native research-percentage arithmetic over the SAME current
    /// acquired flags and actual graph iteration/availability. Definition counts
    /// are checked against the complete current graphs, not guessed from SQL.
    pub fn completion_percent(
        &self,
        state: &LoadedState,
        owner: i32,
        civic_graph: &crate::research::CivicGraph,
    ) -> Result<i32, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let technology_graph = state.game_techs().map_err(Error::State)?;
        if technology_graph.nodes().len() != self.technologies.count()
            || civic_graph.nodes.len() != self.civics.count()
        {
            return Err(Error::Missing("current complete research graph counts"));
        }
        let (techs, culture) = state.research_components(owner).map_err(Error::State)?;
        let active = |iteration: &[i32], node: &dyn Fn(i32) -> Option<bool>| {
            let mut out = Vec::new();
            for &index in iteration {
                if node(index).ok_or(Error::Missing("research iteration node"))? {
                    out.push(index);
                }
            }
            Ok::<_, Error>(out)
        };
        let technology_iteration = active(technology_graph.iteration(), &|i| {
            technology_graph.node(i).map(|n| n.active)
        })?;
        let civic_iteration = active(&civic_graph.iteration, &|i| {
            civic_graph.node(i).map(|n| n.active)
        })?;
        let hashes = |definitions: &crate::registry::LinuxDefinitionRegistry| {
            (0..definitions.count())
                .map(|i| definitions.type_hash(i).expect("registry index"))
                .collect::<Vec<_>>()
        };
        let technology = units::percent_complete(
            &techs.has_tech,
            &hashes(&self.technologies),
            &technology_iteration,
            self.technologies.count() as u32,
        )
        .map_err(Error::Research)?;
        let civic = units::percent_complete(
            &culture.has_civic,
            &hashes(&self.civics),
            &civic_iteration,
            self.civics.count() as u32,
        )
        .map_err(Error::Research)?;
        Ok(technology.max(civic))
    }
    pub fn recompute_empty(
        &self,
        state: &mut LoadedState,
        owner: i32,
        percent: i32,
        start_era: Option<u32>,
    ) -> Result<(), Error> {
        self.recompute(state, owner, percent, start_era, false)
    }
    /// Stats.GetAverageNumConstructableDistricts2b82530 sums only specialty
    /// districts (Economic.IsSpecialty = RequiresPopulation). With no such
    /// current records its numerator is0 for every eligibility subset, so the
    /// supported progression models use the same zero-average/no-discount
    /// arithmetic. Actual ordinary records remain in the canonical roster.
    pub fn recompute_no_specialty(
        &self,
        state: &mut LoadedState,
        owner: i32,
        percent: i32,
        start_era: Option<u32>,
    ) -> Result<(), Error> {
        self.recompute(state, owner, percent, start_era, true)
    }
    fn recompute(
        &self,
        state: &mut LoadedState,
        owner: i32,
        percent: i32,
        start_era: Option<u32>,
        allow_non_specialty: bool,
    ) -> Result<(), Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("district owner"))?;
        if !player.districts.records.entries.is_empty() {
            if !allow_non_specialty {
                return Err(Error::NonemptyDistricts);
            }
            for record in &player.districts.records.entries {
                let definition = self
                    .items
                    .iter()
                    .find(|d| d.hash == record.value.district_hash)
                    .ok_or(Error::Missing("current district definition"))?;
                if definition.specialty {
                    return Err(Error::NonemptyDistricts);
                }
            }
        }
        let current_era = self
            .eras
            .find_index(player.eras.era_hash)
            .map_or(-1, |i| i as i32);
        let rate = if current_era == -1 {
            None
        } else {
            let hash = start_era.ok_or(Error::Missing("current configuration start era"))?;
            let index = if hash <= 0x7ff || hash == u32::MAX {
                hash
            } else {
                self.eras.find_index(hash).map_or(u32::MAX, |i| i as u32)
            };
            Some(
                self.starts
                    .find_index(index)
                    .and_then(|i| self.rates.get(i))
                    .copied()
                    .unwrap_or(1),
            )
        };
        // No synchronous callback occurs within this native loop. Validate the
        // actual destination cells before reserving/writing the current owner.
        let mut updates = Vec::new();
        for item in &self.items {
            let slot = player
                .districts
                .current_district_cost
                .iter()
                .rposition(|p| p.definition_hash == item.hash)
                .ok_or(Error::Missing("current district cost cell"))?;
            let cost = c6_core::research_turn::empty_district_cost(
                item.cost,
                item.model,
                item.parameter,
                percent,
                self.economic.game_cost_escalation,
                self.economic.speed_cost_multiplier,
                rate,
            )
            .map_err(Error::Progression)?;
            updates.push((slot, cost));
        }
        let districts = state.player_districts_mut(owner).map_err(Error::State)?;
        for (slot, cost) in updates {
            districts.current_district_cost[slot].value = cost;
        }
        Ok(())
    }
}
