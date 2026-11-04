//! Current canonical research, civic progress and boosts over admitted rules.
//! Synchronous consumers are required effects. An error is an incomplete native
//! prefix, not a successfully executed research action or complete turn.
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use acquisition::Context as _;
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    movement_world::DefinitionLookup, player_culture::Culture, player_techs::Techs,
    relic_creation::boost, research_acquisition as acquisition, research_turn,
};
pub use research_turn::{InsertMode, Kind};
pub mod diplomacy;
pub mod districts;
pub mod units;
mod yields;
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Graph(c6_core::game_techs::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    World(c6_core::world::MissingInput),
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Costs(crate::research_unit_costs::Error),
    DistrictCosts(districts::Error),
    AcquiredWrite(c6_core::loaded_state::research::WriteError),
    Binding,
    IllegalTechnology(i32),
    IllegalCivic(i32),
    Missing(&'static str),
    Effect(E),
    Kernel(Box<boost::Error<Error<E>>>),
}
#[derive(Clone, Debug)]
pub struct CivicGraph {
    pub nodes: Vec<c6_core::game_techs::Node>,
    pub iteration: Vec<i32>,
}
impl CivicGraph {
    fn node(&self, index: i32) -> Option<&c6_core::game_techs::Node> {
        if index == -1 {
            return None;
        }
        self.nodes
            .get(index as u32 as usize)
            .filter(|n| n.native_type == index)
            .or_else(|| self.nodes.iter().find(|n| n.native_type == index))
    }
}
/// Unsaved current inputs belong to this one initialized World. Absent inputs
/// are never inferred from serialized bytes, owner numbers or queue contents.
pub struct Runtime {
    pub binding: WireBinding,
    pub civic_graph: Option<CivicGraph>,
    pub alive_majors: Option<Vec<i32>>,
    pub alive_major_ids: Option<Vec<i32>>,
    pub city_order: Vec<(i32, Vec<c6_core::unit::ComponentId>)>,
    pub game_era: Option<i32>,
    pub start_era_hash: Option<u32>,
    pub civilization_levels: Vec<(i32, u32)>,
    pub scratch_players: Vec<(i32, bool)>,
    pub manager_logging: Option<bool>,
}
#[derive(Clone, Debug)]
pub enum Effect {
    PlayerAcquired {
        player: i32,
        kind: Kind,
        index: i32,
    },
    GainAnalysis {
        player: i32,
        kind: Kind,
    },
    UpdateUnitsForTech(i32),
    DistrictCosts(i32),
    GameAcquired {
        player: i32,
        kind: Kind,
        index: i32,
    },
    Report(boost::Report),
    Notification(boost::Notification),
    ChooseTechnology(i32),
    BoostSignal {
        player: i32,
        kind: Kind,
        index: i32,
    },
    GovernmentNotifications(i32),
    BoostLog {
        player: i32,
        kind: Kind,
        index: i32,
        had_progress: bool,
        acquired: bool,
    },
    BoostTelemetry {
        player: i32,
        type_name: Vec<u8>,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum Query {
    CityYield {
        city: c6_core::unit::ComponentId,
        yield_index: i32,
    },
    BeliefItemCount {
        player: i32,
        record: usize,
    },
    CountTokensGiven(i32),
    CountTributaries(i32),
    EarnedGreatPeople(i32),
    CultureYield(i32),
    TouristsFrom(i32),
    EverAliveMajorsCount,
    ResearchAgreementTech {
        player: i32,
        other: i32,
    },
}
/// The same live effects owner services synchronous callbacks and current yield
/// calculations. Query implementations must compute from the supplied current
/// World; captured method answers are not a substitute for those computations.
pub trait Effects {
    type Error;
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
    fn effect(
        &mut self,
        state: &mut LoadedState,
        definitions: &Definitions,
        effect: Effect,
    ) -> Result<(), Self::Error>;
    fn query(
        &mut self,
        state: &mut LoadedState,
        definitions: &Definitions,
        query: Query,
    ) -> Result<i32, Self::Error>;
}
pub struct Definitions {
    binding: WireBinding,
    boost: boost::Rules,
    techs: Vec<acquisition::Definition>,
    civics: Vec<acquisition::Definition>,
    costs: [Vec<boost::CostDefinition>; 2],
    eras: LinuxDefinitionRegistry,
    yields: LinuxDefinitionRegistry,
    research_alliance: i32,
    culture_alliance: i32,
    technology_hashes: Vec<u32>,
    route_prerequisites: Vec<Option<i32>>,
    technology_modifiers: Vec<u32>,
    civic_modifiers: Vec<u32>,
    civic_resource_prerequisites: Vec<u32>,
    civic_random_agendas: Vec<u32>,
    civic_obsolete_actions: Vec<u32>,
    resource_tech_prerequisites: Vec<u32>,
    district_costs: districts::Definitions,
    diplomatic_visibility: diplomacy::Definitions,
    chronology: Vec<i32>,
    era_percent: [Option<i32>; 4],
    tourism_per_citizen: Option<i32>,
    tourism_to_move: Option<i32>,
    speed: Option<i32>,
    unit_costs: crate::research_unit_costs::Definitions,
}
fn scalar<E>(row: crate::Row<'_>, name: &'static str) -> Result<i32, Error<E>> {
    match row.get(name).map_err(Error::Rules)? {
        SqlValue::Integer(value) => i32::try_from(*value).map_err(|_| Error::Missing(name)),
        _ => Err(Error::Missing(name)),
    }
}
fn parameter(rules: &Rules, name: &str) -> Option<i32> {
    let mut rows = rules
        .gameplay()
        .table("GlobalParameters")?
        .rows()
        .filter(|row| row.text("Name").ok() == Some(name));
    let row = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    match row.get("Value").ok()? {
        SqlValue::Integer(v) => i32::try_from(*v).ok(),
        SqlValue::Text(v) => std::str::from_utf8(v).ok()?.parse().ok(),
        _ => None,
    }
}
impl Definitions {
    pub fn from_rules(
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
        game_speed: u32,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Definitions)?;
        let eras = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let yields = collection(rules, "Yields", "YieldType").map_err(Error::Definitions)?;
        let alliances =
            collection(rules, "Alliances", "AllianceType").map_err(Error::Definitions)?;
        let culture_alliance = alliances.find_index(0x40c39831).map_or(-1, |i| i as i32);
        let research_alliance = alliances.find_index(0xd22c5d65).map_or(-1, |i| i as i32);
        let era_rows = rules
            .gameplay()
            .table("Eras")
            .ok_or(Error::Missing("Eras"))?;
        let chronology = eras
            .entries()
            .iter()
            .map(|entry| {
                scalar(
                    era_rows
                        .row_by_id(entry.database_rowid)
                        .ok_or(Error::Missing("Era row"))?,
                    "ChronologyIndex",
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut all_rules = Vec::new();
        let mut all_acquired = Vec::new();
        let mut costs = Vec::new();
        for (table, key) in [("Technologies", "TechnologyType"), ("Civics", "CivicType")] {
            let registry = collection(rules, table, key).map_err(Error::Definitions)?;
            let rows = rules.gameplay().table(table).ok_or(Error::Missing(table))?;
            let mut definitions = Vec::new();
            let mut acquired = Vec::new();
            let mut definition_costs = Vec::new();
            for (index, entry) in registry.entries().iter().enumerate() {
                let row = rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("research definition row"))?;
                let repeatable = match row.get("Repeatable").map_err(Error::Rules)? {
                    SqlValue::Integer(0) => false,
                    SqlValue::Integer(1) => true,
                    _ => return Err(Error::Missing("Repeatable bool")),
                };
                let hash = linux_type_hash(&entry.type_name);
                let era = match row.get("EraType").map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    SqlValue::Text(name) => eras
                        .find_index(linux_type_hash(name))
                        .map(|index| index as i32),
                    _ => return Err(Error::Missing("EraType reference")),
                };
                definitions.push(Some(boost::ResearchDefinition {
                    index: index as i32,
                    hash,
                    type_name: entry.type_name.clone(),
                    repeatable,
                }));
                acquired.push(acquisition::Definition {
                    hash,
                    repeatable: Some(repeatable),
                });
                definition_costs.push(boost::CostDefinition {
                    cost: scalar(row, "Cost")?,
                    era,
                });
            }
            all_rules.push(definitions);
            all_acquired.push(acquired);
            costs.push(definition_costs);
        }
        let mut boost_rules = boost::Rules {
            technologies: all_rules.remove(0),
            civics: all_rules.remove(0),
            boosts: Vec::new(),
        };
        for row in rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("Boosts"))?
            .rows()
        {
            let mut refs = Vec::new();
            for (field, definitions) in [
                ("TechnologyType", &boost_rules.technologies),
                ("CivicType", &boost_rules.civics),
            ] {
                refs.push(match row.get(field).map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    SqlValue::Text(name) => definitions
                        .iter()
                        .flatten()
                        .rfind(|d| d.hash == linux_type_hash(name))
                        .map(|d| boost::Reference {
                            index: d.index,
                            type_name: d.type_name.clone(),
                        }),
                    _ => return Err(Error::Missing("boost reference")),
                });
            }
            boost_rules.boosts.push(boost::Definition {
                technology: refs.remove(0),
                civic: refs.remove(0),
                percent: scalar(row, "Boost")?,
                num_items: scalar(row, "NumItems")?,
            });
        }
        let speeds =
            collection(rules, "GameSpeeds", "GameSpeedType").map_err(Error::Definitions)?;
        let speed = if game_speed == u32::MAX {
            None
        } else {
            let entry = speeds
                .find(game_speed)
                .ok_or(Error::Missing("current GameSpeed definition"))?;
            Some(scalar(
                rules
                    .gameplay()
                    .table("GameSpeeds")
                    .and_then(|t| t.row_by_id(entry.database_rowid))
                    .ok_or(Error::Missing("GameSpeed row"))?,
                "CostMultiplier",
            )?)
        };
        let mut route_prerequisites = Vec::new();
        for row in rules
            .gameplay()
            .table("Routes_XP2")
            .ok_or(Error::Missing("Routes_XP2"))?
            .rows()
        {
            route_prerequisites.push(match row.get("PrereqTech").map_err(Error::Rules)? {
                SqlValue::Null => None,
                SqlValue::Text(name) => boost_rules
                    .technologies
                    .iter()
                    .flatten()
                    .rfind(|d| d.hash == linux_type_hash(name))
                    .map(|d| d.index),
                _ => return Err(Error::Missing("route prerequisite technology reference")),
            });
        }
        let mut technology_modifiers = Vec::new();
        for row in rules
            .gameplay()
            .table("TechnologyModifiers")
            .ok_or(Error::Missing("TechnologyModifiers"))?
            .rows()
        {
            match row.get("TechnologyType").map_err(Error::Rules)? {
                SqlValue::Text(name) => technology_modifiers.push(linux_type_hash(name)),
                _ => return Err(Error::Missing("TechnologyModifiers.TechnologyType")),
            }
        }
        let mut resource_tech_prerequisites = Vec::new();
        for row in rules
            .gameplay()
            .table("Resources")
            .ok_or(Error::Missing("Resources"))?
            .rows()
        {
            match row.get("PrereqTech").map_err(Error::Rules)? {
                SqlValue::Null => {}
                SqlValue::Text(name) => resource_tech_prerequisites.push(linux_type_hash(name)),
                _ => return Err(Error::Missing("Resource.PrereqTech")),
            }
        }
        let mut civic_columns = Vec::new();
        for (table, column) in [
            ("CivicModifiers", "CivicType"),
            ("Resources", "PrereqCivic"),
            ("RandomAgendasForCivic", "CivicType"),
            ("DiplomaticActions", "InitiatorObsoleteCivic"),
        ] {
            let mut values = Vec::new();
            for row in rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing(table))?
                .rows()
            {
                match row.get(column).map_err(Error::Rules)? {
                    SqlValue::Null => {}
                    SqlValue::Text(name) => values.push(linux_type_hash(name)),
                    _ => return Err(Error::Missing(column)),
                }
            }
            civic_columns.push(values);
        }
        let unit_costs =
            crate::research_unit_costs::Definitions::from_rules(rules, bound, game_speed)
                .map_err(Error::Costs)?;
        let district_costs =
            districts::Definitions::from_rules(rules, bound, unit_costs.economic())
                .map_err(Error::DistrictCosts)?;
        Ok(Self {
            binding: bound.destination_binding().clone(),
            route_prerequisites,
            technology_modifiers,
            civic_modifiers: civic_columns.remove(0),
            civic_resource_prerequisites: civic_columns.remove(0),
            civic_random_agendas: civic_columns.remove(0),
            civic_obsolete_actions: civic_columns.remove(0),
            resource_tech_prerequisites,
            district_costs,
            diplomatic_visibility: diplomacy::Definitions::from_rules(rules, bound)
                .map_err(|_| Error::Missing("diplomatic visibility definitions"))?,
            technology_hashes: boost_rules
                .technologies
                .iter()
                .flatten()
                .map(|d| d.hash)
                .collect(),
            boost: boost_rules,
            techs: all_acquired.remove(0),
            civics: all_acquired.remove(0),
            costs: [costs.remove(0), costs.remove(0)],
            eras,
            yields,
            research_alliance,
            culture_alliance,
            chronology,
            era_percent: [
                parameter(rules, "TECH_COST_PERCENT_CHANGE_AFTER_GAME_ERA"),
                parameter(rules, "TECH_COST_PERCENT_CHANGE_BEFORE_GAME_ERA"),
                parameter(rules, "CIVIC_COST_PERCENT_CHANGE_AFTER_GAME_ERA"),
                parameter(rules, "CIVIC_COST_PERCENT_CHANGE_BEFORE_GAME_ERA"),
            ],
            tourism_per_citizen: parameter(rules, "TOURISM_CULTURE_PER_CITIZEN"),
            tourism_to_move: parameter(rules, "TOURISM_TOURISM_TO_MOVE_CITIZEN"),
            speed,
            unit_costs,
        })
    }
    /// Install supplied/computed complete current nodes once. The current
    /// initialized graph must not be reconstructed from SQL prerequisites.
    pub fn install_current_technology_graph<E>(
        &self,
        state: &mut LoadedState,
        nodes: Vec<c6_core::game_techs::Node>,
        iteration: Vec<i32>,
    ) -> Result<(), Error<E>> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let graph = c6_core::game_techs::Graph::from_complete_current(
            nodes,
            iteration,
            c6_core::unit::Limits::default(),
        )
        .map_err(Error::Graph)?;
        let availability = (0..self.techs.len())
            .map(|index| graph.has_tech(index as i32))
            .collect();
        state
            .install_current_game_techs(&self.binding, graph, availability)
            .map_err(Error::State)
    }
    /// Every indexed acquired byte must be present in the current writer's
    /// hash pairs. Missing inputs remain errors rather than reset false flags.
    pub fn install_current_player_flags<E>(&self, state: &mut LoadedState) -> Result<(), Error<E>> {
        let mut flags = Vec::new();
        for player in &state.retained().players {
            let mut values = Vec::new();
            for (definitions, pairs) in [
                (&self.techs, &player.techs.has_tech.pairs),
                (&self.civics, &player.culture.has_civic.pairs),
            ] {
                values.push(
                    definitions
                        .iter()
                        .map(|d| {
                            pairs
                                .iter()
                                .rfind(|p| p.definition_hash == d.hash)
                                .map(|p| p.value)
                                .ok_or(Error::Missing("complete current acquired flags"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            let civics = values.pop().expect("two acquired vectors");
            let techs = values.pop().expect("two acquired vectors");
            flags.push((player.header.owner, techs, civics));
        }
        state
            .install_current_research_flags(&self.binding, flags)
            .map_err(Error::State)
    }
    /// Mirror the current initialized civic graph into World's shared
    /// availability view used by unlock/resource/yield queries.
    pub fn install_current_civic_availability<E>(
        &self,
        state: &mut LoadedState,
        graph: &CivicGraph,
    ) -> Result<(), Error<E>> {
        let availability = (0..self.civics.len())
            .map(|i| graph.node(i as i32).is_some_and(|n| n.active))
            .collect();
        state
            .install_current_civic_availability(&self.binding, availability)
            .map_err(Error::State)
    }
    /// DatabaseUtility.GetModifierDefinitions for TechnologyModifiers. A
    /// positive result requires real modifier attachment by the caller.
    pub fn technology_has_attached_modifiers(
        &self,
        index: i32,
    ) -> Result<bool, Error<std::convert::Infallible>> {
        let definition = self
            .boost
            .technologies
            .iter()
            .flatten()
            .find(|d| d.index == index)
            .ok_or(Error::IllegalTechnology(index))?;
        Ok(self.technology_modifiers.contains(&definition.hash))
    }
    pub fn technology_has_resource_unlock(
        &self,
        index: i32,
    ) -> Result<bool, Error<std::convert::Infallible>> {
        let definition = self
            .boost
            .technologies
            .iter()
            .flatten()
            .find(|d| d.index == index)
            .ok_or(Error::IllegalTechnology(index))?;
        Ok(self.resource_tech_prerequisites.contains(&definition.hash))
    }
    pub fn technology_era_hash(
        &self,
        index: i32,
    ) -> Result<Option<u32>, Error<std::convert::Infallible>> {
        let cost = self.costs[0]
            .get(index as u32 as usize)
            .ok_or(Error::IllegalTechnology(index))?;
        Ok(cost.era.and_then(|era| self.eras.type_hash(era as usize)))
    }
    pub fn civic_acquisition_filters(
        &self,
        index: i32,
    ) -> Result<(bool, bool, bool, bool), Error<std::convert::Infallible>> {
        let hash = self
            .civics
            .get(index as u32 as usize)
            .ok_or(Error::IllegalCivic(index))?
            .hash;
        Ok((
            self.civic_modifiers.contains(&hash),
            self.civic_resource_prerequisites.contains(&hash),
            self.civic_random_agendas.contains(&hash),
            self.civic_obsolete_actions.contains(&hash),
        ))
    }
    pub fn civic_era_hash(
        &self,
        index: i32,
    ) -> Result<Option<u32>, Error<std::convert::Infallible>> {
        let cost = self.costs[1]
            .get(index as u32 as usize)
            .ok_or(Error::IllegalCivic(index))?;
        Ok(cost.era.and_then(|era| self.eras.type_hash(era as usize)))
    }
    pub fn diplomatic_visibility(&self) -> &diplomacy::Definitions {
        &self.diplomatic_visibility
    }
    pub fn research_rules(&self) -> &boost::Rules {
        &self.boost
    }
    pub fn era_index(&self, hash: u32) -> i32 {
        self.eras.find_index(hash).map_or(-1, |v| v as i32)
    }
}
pub struct Context<'a, E: Effects> {
    pub state: &'a mut LoadedState,
    pub definitions: &'a Definitions,
    pub effects: &'a mut E,
}
impl<E: Effects> Context<'_, E> {
    /// Preserve the current numeric query effects of the stock PlayerTechs
    /// publication while allowing a headless adapter to retain semantic output
    /// instead of native presentation caches. This is not a generic dispatcher:
    /// its caller must qualify all actual subscribers/prepublication handlers.
    pub fn stock_technology_publication_yields(
        &mut self,
        report: &boost::Report,
    ) -> Result<Vec<i32>, Error<E::Error>> {
        self.check()?;
        if !matches!(report.event, 0x48062f97 | 0x4378a305 | 0xf621c5ac)
            || report.specialization.is_some()
            || report.old_progress.is_some()
            || report.source.is_some()
        {
            return Err(Error::Missing("unsupported stock technology publication"));
        }
        research_turn::technology_publication_yields(self, report.player)
            .map_err(|e| Error::Kernel(Box::new(e)))
    }
    /// Current stock PlayerCulture publication reads culture for the current
    /// civic's turns-left, then calculates it independently for Values.Fill.
    /// The caller qualifies its actual synchronous consumers separately.
    pub fn stock_civic_publication_yields(
        &mut self,
        report: &boost::Report,
    ) -> Result<Vec<i32>, Error<E::Error>> {
        self.check()?;
        if !matches!(report.event, 0xc5613042 | 0xac2769a3 | 0x87667464)
            || report.specialization.is_some()
            || report.old_progress.is_some()
            || report.source.is_some()
        {
            return Err(Error::Missing("unsupported stock civic publication"));
        }
        let mut values = Vec::with_capacity(2);
        if research_turn::current(self, report.player, Kind::Civic)? != -1 {
            values.push(self.culture_yield(report.player)?);
        }
        values.push(self.culture_yield(report.player)?);
        Ok(values)
    }
    /// Techs.OnPlayerTechResearched: minor catch-up progresses to one whole
    /// science below cost, rather than granting the technology. Queries use the
    /// current acquired aliases and actual alive-major vector.
    pub fn stock_player_technology_catchup(
        &mut self,
        owner: i32,
        event_player: i32,
        index: i32,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        self.stock_player_research_catchup(owner, event_player, index, Kind::Technology)
    }
    /// Culture.OnPlayerCivicCulturevated uses the same current major threshold;
    /// its SetCulturalProgress also updates lifetime culture and notices.
    pub fn stock_player_civic_catchup(
        &mut self,
        owner: i32,
        event_player: i32,
        index: i32,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        self.stock_player_research_catchup(owner, event_player, index, Kind::Civic)
    }
    fn stock_player_research_catchup(
        &mut self,
        owner: i32,
        event_player: i32,
        index: i32,
        kind: Kind,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        self.check().map_err(boost::Error::Context)?;
        let level = |owner| {
            self.effects
                .runtime()
                .civilization_levels
                .iter()
                .find(|p| p.0 == owner)
                .map(|p| p.1)
                .ok_or(Error::Missing("current civilization level"))
        };
        if level(event_player).map_err(boost::Error::Context)? != 0x253718b0 {
            return Ok(());
        }
        let hash = (match kind {
            Kind::Technology => &self.definitions.boost.technologies,
            Kind::Civic => &self.definitions.boost.civics,
        })
        .iter()
        .flatten()
        .find(|d| d.index == index)
        .ok_or(boost::Error::Definition)?
        .hash;
        let components = self
            .state
            .research_components(owner)
            .map_err(Error::State)
            .map_err(boost::Error::Context)?;
        let has = (match kind {
            Kind::Technology => &components.0.has_tech,
            Kind::Civic => &components.1.has_civic,
        })
        .pairs
        .iter()
        .rev()
        .find(|p| p.definition_hash == hash)
        .map(|p| p.value)
        .ok_or(boost::Error::MissingBool(hash))?;
        if has {
            return Ok(());
        }
        let own_level = level(owner).map_err(boost::Error::Context)?;
        if own_level == 0x253718b0 || own_level == 0xd959ec24 {
            return Ok(());
        }
        let alive = self
            .effects
            .runtime()
            .alive_majors
            .as_ref()
            .ok_or(boost::Error::Context(Error::Missing(
                "current alive major order",
            )))?;
        let threshold = research_turn::minor_technology_catchup_threshold(alive.len() as i32);
        let mut count = 0i32;
        for &other in alive {
            let components = self
                .state
                .research_components(other)
                .map_err(Error::State)
                .map_err(boost::Error::Context)?;
            if (match kind {
                Kind::Technology => &components.0.has_tech,
                Kind::Civic => &components.1.has_civic,
            })
            .pairs
            .iter()
            .rev()
            .find(|p| p.definition_hash == hash)
            .map(|p| p.value)
            .ok_or(boost::Error::MissingBool(hash))?
            {
                count = count.wrapping_add(1);
            }
        }
        if count >= threshold {
            let cost = boost::research_cost(self, kind, index, false)?;
            boost::set_research_progress(
                self,
                owner,
                kind,
                index,
                cost.wrapping_sub(1).wrapping_shl(8),
            )?;
        }
        Ok(())
    }
    /// Positive CanResearch legality over the current initialized graph. The
    /// hidden-until-prerequisite reveal gate is implied by all prerequisites
    /// being acquired (or by an empty prerequisite list). It changes no true
    /// result here; no AI recommendation or policy selection is performed.
    pub fn can_research_technology(
        &self,
        player: i32,
        index: i32,
    ) -> Result<bool, Error<E::Error>> {
        self.check()?;
        let Some(definition) = self.definitions.techs.get(index as u32 as usize) else {
            return Ok(false);
        };
        if !self.globally_available(Kind::Technology, index)? {
            return Ok(false);
        }
        if self.has_current_tech(player, index)? && definition.repeatable != Some(true) {
            return Ok(false);
        }
        let node = self
            .node(Kind::Technology, index)?
            .ok_or(Error::Missing("active technology node"))?;
        for &prerequisite in &node.prerequisites {
            if !self.has_current_tech(player, prerequisite)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Ordered legal external choices. No native AI decision fills an empty
    /// research queue. An empty result is distinct from unavailable input.
    pub fn technology_choices(&self, player: i32) -> Result<Vec<i32>, Error<E::Error>> {
        let mut choices = Vec::new();
        for index in 0..self.definitions.techs.len() {
            if self.can_research_technology(player, index as i32)? {
                choices.push(index as i32);
            }
        }
        Ok(choices)
    }
    /// External policy action. Input is a native technology collection index;
    /// invalid/ineligible choices reject before any queue or callback changes.
    pub fn request_technology(
        &mut self,
        player: i32,
        index: i32,
        mode: InsertMode,
    ) -> Result<(), Error<E::Error>> {
        if !self.can_research_technology(player, index)? {
            return Err(Error::IllegalTechnology(index));
        }
        self.choose(player, Kind::Technology, index, mode)
            .map_err(|error| Error::Kernel(Box::new(error)))
    }
    /// Current CanProgress legality for an externally supplied civic index.
    /// All prerequisites imply the hidden-civic reveal condition; no policy
    /// selection or recommendation score is consulted.
    pub fn can_progress_civic(&self, player: i32, index: i32) -> Result<bool, Error<E::Error>> {
        self.check()?;
        let Some(definition) = self.definitions.civics.get(index as u32 as usize) else {
            return Ok(false);
        };
        if !self.globally_available(Kind::Civic, index)? {
            return Ok(false);
        }
        if self.has_current_civic(player, index)? && definition.repeatable != Some(true) {
            return Ok(false);
        }
        let node = self
            .node(Kind::Civic, index)?
            .ok_or(Error::Missing("active civic node"))?;
        for &prerequisite in &node.prerequisites {
            if !self.has_current_civic(player, prerequisite)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Preserve the admitted current Game.Culture iteration order. An empty
    /// legal set differs from unavailable graph or acquired-flag inputs.
    pub fn civic_choices(&self, player: i32) -> Result<Vec<i32>, Error<E::Error>> {
        let graph = self
            .effects
            .runtime()
            .civic_graph
            .as_ref()
            .ok_or(Error::Missing("current Game.Culture graph"))?;
        let mut choices = Vec::new();
        for &index in &graph.iteration {
            if self.can_progress_civic(player, index)? {
                choices.push(index);
            }
        }
        Ok(choices)
    }
    pub fn request_civic(
        &mut self,
        player: i32,
        index: i32,
        mode: InsertMode,
    ) -> Result<(), Error<E::Error>> {
        if !self.can_progress_civic(player, index)? {
            return Err(Error::IllegalCivic(index));
        }
        self.choose(player, Kind::Civic, index, mode)
            .map_err(|error| Error::Kernel(Box::new(error)))
    }
    /// Stock AI Research.OnResearchCompleted callback in the no-contract,
    /// no-route-unlock branch. Recomputed recommendation scores are planning
    /// data only; native QueueNextTech never supplies a policy decision here.
    /// The actual subscriber identity must be checked by the invoking adapter.
    pub fn stock_research_completed_without_contracts(
        &mut self,
        player: i32,
        index: i32,
    ) -> Result<(), Error<E::Error>> {
        self.check()?;
        let owner = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == player)
            .ok_or(Error::Missing("research analysis player"))?;
        let analysis = owner
            .ai_research
            .as_ref()
            .ok_or(Error::Missing("current research analysis"))?;
        if !analysis.managed_contracts.is_empty() {
            return Err(Error::Missing("research completion AI contract consumers"));
        }
        if self
            .definitions
            .route_prerequisites
            .iter()
            .flatten()
            .any(|tech| *tech == index && index != -1)
        {
            return Err(Error::Missing(
                "research completion Economic.UnlockNewTradeRoute",
            ));
        }
        if self
            .state
            .player_slot_status(player)
            .map_err(Error::State)?
            != 3
        {
            return Err(Error::Missing("external technology choice continuation"));
        }
        Ok(())
    }
    pub fn recompute_empty_district_costs(&mut self, player: i32) -> Result<(), Error<E::Error>> {
        self.check()?;
        let percent = self
            .percentage(player, Kind::Technology)?
            .max(self.percentage(player, Kind::Civic)?);
        self.definitions
            .district_costs
            .recompute_empty(
                self.state,
                player,
                percent,
                self.effects.runtime().start_era_hash,
            )
            .map_err(Error::DistrictCosts)
    }
    fn check(&self) -> Result<(), Error<E::Error>> {
        if self.state.binding() != &self.definitions.binding
            || self.state.binding() != &self.effects.runtime().binding
        {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    fn emit(&mut self, effect: Effect) -> Result<(), Error<E::Error>> {
        self.effects
            .effect(self.state, self.definitions, effect)
            .map_err(Error::Effect)
    }
    fn query(&mut self, query: Query) -> Result<i32, Error<E::Error>> {
        self.effects
            .query(self.state, self.definitions, query)
            .map_err(Error::Effect)
    }
    fn node(
        &self,
        kind: Kind,
        index: i32,
    ) -> Result<Option<&c6_core::game_techs::Node>, Error<E::Error>> {
        match kind {
            Kind::Technology => Ok(self.state.game_techs().map_err(Error::State)?.node(index)),
            Kind::Civic => Ok(self
                .effects
                .runtime()
                .civic_graph
                .as_ref()
                .ok_or(Error::Missing("current Game.Culture graph"))?
                .node(index)),
        }
    }
    fn percentage(&self, player: i32, kind: Kind) -> Result<i32, Error<E::Error>> {
        let (definitions, iteration, flags) = match kind {
            Kind::Technology => (
                &self.definitions.techs,
                self.state.game_techs().map_err(Error::State)?.iteration(),
                &self
                    .state
                    .research_components(player)
                    .map_err(Error::State)?
                    .0
                    .has_tech,
            ),
            Kind::Civic => (
                &self.definitions.civics,
                self.effects
                    .runtime()
                    .civic_graph
                    .as_ref()
                    .ok_or(Error::Missing("current Game.Culture graph"))?
                    .iteration
                    .as_slice(),
                &self
                    .state
                    .research_components(player)
                    .map_err(Error::State)?
                    .1
                    .has_civic,
            ),
        };
        if definitions.is_empty() {
            return Ok(100);
        }
        let mut count = 0_i32;
        for index in iteration {
            if !self.globally_available(kind, *index)? {
                continue;
            }
            let definition = definitions
                .get(*index as u32 as usize)
                .ok_or(Error::Missing("research iteration index"))?;
            let acquired = flags
                .pairs
                .iter()
                .rfind(|p| p.definition_hash == definition.hash)
                .ok_or(Error::Missing("current acquired research flag"))?
                .value;
            if acquired {
                count = count.wrapping_add(1);
            }
        }
        if count as u32 > definitions.len() as u32 {
            return Ok(100);
        }
        Ok(count.wrapping_mul(100) / (definitions.len() as i32))
    }
    pub fn choose(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        mode: InsertMode,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        self.check().map_err(boost::Error::Context)?;
        research_turn::choose(self, player, kind, index, mode)
    }
    pub fn do_research(&mut self, player: i32) -> Result<(), boost::Error<Error<E::Error>>> {
        self.check().map_err(boost::Error::Context)?;
        research_turn::do_research(self, player)
    }
    pub fn change_progress(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        delta_q8: i32,
    ) -> Result<bool, boost::Error<Error<E::Error>>> {
        self.check().map_err(boost::Error::Context)?;
        boost::change_research_progress(self, player, kind, index, delta_q8)
    }
    pub fn trigger_boost(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        source: i32,
    ) -> Result<(), boost::Error<Error<E::Error>>> {
        self.check().map_err(boost::Error::Context)?;
        boost::trigger_player_boost(self, player, kind, index, source)
    }
}
impl<E: Effects> acquisition::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    fn preflight_acquisition(
        &self,
        player: i32,
        _kind: Kind,
        _index: i32,
        _value: bool,
    ) -> Result<(), Self::Error> {
        self.check()?;
        self.state
            .research_components(player)
            .map_err(Error::State)?;
        Ok(())
    }
    fn definitions(&self, kind: Kind) -> &[acquisition::Definition] {
        match kind {
            Kind::Technology => &self.definitions.techs,
            Kind::Civic => &self.definitions.civics,
        }
    }
    fn techs(&self, player: i32) -> Result<&Techs, Self::Error> {
        Ok(self
            .state
            .research_components(player)
            .map_err(Error::State)?
            .0)
    }
    fn techs_mut(&mut self, player: i32) -> Result<&mut Techs, Self::Error> {
        Ok(self
            .state
            .research_components_mut(player)
            .map_err(Error::State)?
            .0)
    }
    fn culture(&self, player: i32) -> Result<&Culture, Self::Error> {
        Ok(self
            .state
            .research_components(player)
            .map_err(Error::State)?
            .1)
    }
    fn culture_mut(&mut self, player: i32) -> Result<&mut Culture, Self::Error> {
        Ok(self
            .state
            .research_components_mut(player)
            .map_err(Error::State)?
            .1)
    }
    fn write_acquired_flag(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        hash: u32,
        value: bool,
    ) -> Result<(), Self::Error> {
        self.state
            .set_research_acquired(player, kind, index, hash, value)
            .map_err(Error::AcquiredWrite)
    }
    fn civic_acquired_count(&self, player: i32) -> Result<usize, Self::Error> {
        self.state
            .world()
            .player(player)
            .map_err(Error::World)?
            .civics
            .as_ref()
            .map(Vec::len)
            .ok_or(Error::Missing("current acquired Civic vector"))
    }
    fn globally_available(&self, kind: Kind, index: i32) -> Result<bool, Self::Error> {
        Ok(self.node(kind, index)?.is_some_and(|node| node.active))
    }
    fn signal_player_acquired(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
    ) -> Result<(), Self::Error> {
        self.emit(Effect::PlayerAcquired {
            player,
            kind,
            index,
        })
    }
    fn signal_gain_analysis(&mut self, player: i32, kind: Kind) -> Result<(), Self::Error> {
        self.emit(Effect::GainAnalysis { player, kind })
    }
    fn recompute_unit_costs(&mut self, player: i32) -> Result<(), Self::Error> {
        let completion = acquisition::units::ResearchCompletion {
            technologies: self.percentage(player, Kind::Technology)?,
            civics: self.percentage(player, Kind::Civic)?,
        };
        self.definitions
            .unit_costs
            .recompute_loaded(self.state, player, completion)
            .map_err(Error::Costs)
    }
    fn update_units_for_tech(&mut self, player: i32) -> Result<(), Self::Error> {
        self.emit(Effect::UpdateUnitsForTech(player))
    }
    fn recompute_district_costs(&mut self, player: i32) -> Result<(), Self::Error> {
        self.emit(Effect::DistrictCosts(player))
    }
    fn early_research_agreement(&mut self, player: i32, index: i32) -> Result<(), Self::Error> {
        boost::early_research_agreement(self, player, index)
            .map_err(|error| Error::Kernel(Box::new(error)))
    }
    fn alive_majors(&self) -> Result<Vec<i32>, Self::Error> {
        self.effects
            .runtime()
            .alive_majors
            .clone()
            .ok_or(Error::Missing("current GetAliveMajors order"))
    }
    fn team(&self, player: i32) -> Result<i32, Self::Error> {
        self.state.player_team(player).map_err(Error::State)
    }
    fn trigger_boost(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        source: i32,
    ) -> Result<(), Self::Error> {
        boost::trigger_player_boost(self, player, kind, index, source)
            .map_err(|error| Error::Kernel(Box::new(error)))
    }
    fn signal_game_acquired(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
    ) -> Result<(), Self::Error> {
        self.emit(Effect::GameAcquired {
            player,
            kind,
            index,
        })
    }
}
impl<E: Effects> boost::Context for Context<'_, E> {
    fn preflight_boost(&self, player: i32) -> Result<(), Self::Error> {
        self.check()?;
        self.state
            .research_components(player)
            .map_err(Error::State)?;
        Ok(())
    }
    fn boost_rules(&self) -> &boost::Rules {
        &self.definitions.boost
    }
    fn runtime_civic_count(&self, player: i32) -> Result<usize, Self::Error> {
        acquisition::Context::civic_acquired_count(self, player)
    }
    fn cost_node_raw(&mut self, kind: Kind, index: i32) -> Result<Option<i32>, Self::Error> {
        Ok(self.node(kind, index)?.map(|node| node.cost_raw))
    }
    fn cost_definition(
        &self,
        kind: Kind,
        index: i32,
    ) -> Result<Option<boost::CostDefinition>, Self::Error> {
        Ok(
            self.definitions.costs[if kind == Kind::Technology { 0 } else { 1 }]
                .get(index as u32 as usize)
                .copied(),
        )
    }
    fn cost_game_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn current_game_era(&mut self) -> Result<i32, Self::Error> {
        self.effects
            .runtime()
            .game_era
            .ok_or(Error::Missing("current Game.Eras era"))
    }
    fn era_chronology(&self, era: i32) -> Result<Option<i32>, Self::Error> {
        Ok(self
            .definitions
            .chronology
            .get(era as u32 as usize)
            .copied())
    }
    fn era_cost_percent(&mut self, kind: Kind, after: bool) -> Result<i32, Self::Error> {
        self.definitions.era_percent
            [(if kind == Kind::Technology { 0 } else { 2 }) + usize::from(!after)]
        .ok_or(Error::Missing("era research cost parameter"))
    }
    fn game_speed_cost_multiplier(&mut self) -> Result<Option<i32>, Self::Error> {
        Ok(self.definitions.speed)
    }
    fn scratch(&mut self, player: i32) -> Result<bool, Self::Error> {
        let mut rows = self
            .effects
            .runtime()
            .scratch_players
            .iter()
            .filter(|row| row.0 == player);
        let value = rows
            .next()
            .ok_or(Error::Missing("current Player.IsScratch byte"))?
            .1;
        if rows.next().is_some() {
            return Err(Error::Missing("duplicate Player.IsScratch input"));
        }
        Ok(value)
    }
    fn barbarian(&mut self, player: i32) -> Result<bool, Self::Error> {
        let mut rows = self
            .effects
            .runtime()
            .civilization_levels
            .iter()
            .filter(|row| row.0 == player);
        let value = rows
            .next()
            .ok_or(Error::Missing("current Player civilization level"))?
            .1;
        if rows.next().is_some() {
            return Err(Error::Missing("duplicate civilization level input"));
        }
        Ok(value == 0xd959ec24)
    }
    fn boost_report(&mut self, report: boost::Report) -> Result<(), Self::Error> {
        self.emit(Effect::Report(report))
    }
    fn boost_notification(&mut self, notification: boost::Notification) -> Result<(), Self::Error> {
        self.emit(Effect::Notification(notification))
    }
    fn edit_game_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn signal_boost(&mut self, player: i32, kind: Kind, index: i32) -> Result<(), Self::Error> {
        self.emit(Effect::BoostSignal {
            player,
            kind,
            index,
        })
    }
    fn tourism_culture_per_citizen(&mut self) -> Result<i32, Self::Error> {
        self.definitions
            .tourism_per_citizen
            .ok_or(Error::Missing("TOURISM_CULTURE_PER_CITIZEN"))
    }
    fn total_tourists_from(&mut self, player: i32) -> Result<i32, Self::Error> {
        self.total_tourists_from_current(player)
    }
    fn government_notifications(&mut self, player: i32) -> Result<(), Self::Error> {
        self.emit(Effect::GovernmentNotifications(player))
    }
    fn research_agreement_tech(&mut self, player: i32, other: i32) -> Result<i32, Self::Error> {
        self.state
            .game_relations()
            .map_err(Error::State)?
            .research_agreement_tech(player, other, &self.definitions.technology_hashes)
            .map_err(Error::Relations)
    }

    fn manager_logging(&mut self) -> Result<bool, Self::Error> {
        self.effects
            .runtime()
            .manager_logging
            .ok_or(Error::Missing("current BoostManager logging flag"))
    }
    fn manager_log(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        had_progress: bool,
        acquired: bool,
    ) -> Result<(), Self::Error> {
        self.emit(Effect::BoostLog {
            player,
            kind,
            index,
            had_progress,
            acquired,
        })
    }
    fn publish_boost_telemetry(
        &mut self,
        player: i32,
        type_name: &[u8],
    ) -> Result<(), Self::Error> {
        self.emit(Effect::BoostTelemetry {
            player,
            type_name: type_name.to_vec(),
        })
    }
}
impl<E: Effects> research_turn::Context for Context<'_, E> {
    fn research_yield_q8(&mut self, player: i32, kind: Kind) -> Result<i32, Self::Error> {
        match kind {
            Kind::Technology => self.science_yield(player),
            Kind::Civic => self.culture_yield(player),
        }
    }
    fn notify_choose_technology(&mut self, player: i32) -> Result<(), Self::Error> {
        self.emit(Effect::ChooseTechnology(player))
    }
}

#[cfg(test)]
mod current_civic_tests {
    use super::*;
    #[test]
    fn current_civic_node_prefers_matching_positional_entry_before_first_duplicate() {
        let graph = CivicGraph {
            nodes: vec![
                c6_core::game_techs::Node {
                    native_type: 1,
                    active: false,
                    cost_raw: 10,
                    prerequisites: vec![],
                },
                c6_core::game_techs::Node {
                    native_type: 1,
                    active: true,
                    cost_raw: 20,
                    prerequisites: vec![],
                },
            ],
            iteration: vec![1, 1],
        };
        assert_eq!(graph.node(1).unwrap().cost_raw, 20);
        assert!(graph.node(-1).is_none());
    }
}
