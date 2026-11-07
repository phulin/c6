//! Retained activation inputs for the bounded initialized Session composition.
//!
//! These are simulator-owned semantic inputs, never native pointers or decoded
//! readsets. Preparing them does not execute a callback or choose a policy.
use crate::{
    game_update_state::activation as a,
    loaded_actions::{session::stock_turn_services as turn, stock_notifications as stock},
    resource_binding::ResourceAuthority,
    Rules,
};
use c6_core::loaded_state::LoadedState;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
pub const SCHEMA: &str = "c6-initialized-turn-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Callback {
    City,
    BehaviorTree,
    FreeCities,
    Barbarian,
    Planning,
    Diplomatic,
    Religious,
    Targets,
    GrandStrategic,
    Governors,
    Research,
    Economic,
    Espionage,
    Homeland,
    Culture,
    Military,
    Tactical,
}
impl Callback {
    fn human(self) -> Result<stock::human_activation::Kind> {
        use stock::human_activation::Kind as K;
        Ok(match self {
            Self::City => K::City,
            Self::BehaviorTree => K::BehaviorTree,
            Self::Planning => K::Planning,
            Self::Diplomatic => K::Diplomatic,
            Self::Religious => K::Religious,
            Self::Targets => K::Targets,
            Self::GrandStrategic => K::GrandStrategic,
            Self::Governors => K::Governors,
            Self::Research => K::Research,
            Self::Economic => K::Economic,
            Self::Espionage => K::Espionage,
            Self::Homeland => K::Homeland,
            Self::Culture => K::Culture,
            Self::Military => K::Military,
            Self::Tactical => K::Tactical,
            Self::FreeCities | Self::Barbarian => {
                return Err("NPC callback in human registration".into())
            }
        })
    }
    fn npc(self) -> stock::npc_leaves::Kind {
        use stock::npc_leaves::Kind as K;
        match self {
            Self::City => K::City,
            Self::BehaviorTree => K::BehaviorTree,
            Self::FreeCities => K::FreeCities,
            Self::Barbarian => K::Barbarian,
            Self::Planning => K::Planning,
            Self::Diplomatic => K::Diplomatic,
            Self::Religious => K::Religious,
            Self::Targets => K::Targets,
            Self::GrandStrategic => K::GrandStrategic,
            Self::Governors => K::Governors,
            Self::Research => K::Research,
            Self::Economic => K::Economic,
            Self::Espionage => K::Espionage,
            Self::Homeland => K::Homeland,
            Self::Culture => K::Culture,
            Self::Military => K::Military,
            Self::Tactical => K::Tactical,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scout {
    pub uses_q8: Vec<i32>,
    pub fallback_q8: Option<i32>,
    pub primary_regions: Vec<u32>,
    pub secondary_regions: Vec<u32>,
    pub continent_discovery_bonus: i32,
    pub natural_wonder_discovery_bonus: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Human {
    pub owner: i32,
    pub off_turn: Vec<Callback>,
    pub priorities: Vec<(Callback, i32)>,
    pub pending_pantheon: Option<bool>,
    pub religious_contract_count: Option<usize>,
    pub initialized: bool,
    pub scout: Scout,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub owner: i32,
    pub off_turn: Vec<Callback>,
    pub priorities: Vec<(Callback, i32)>,
    pub pending_pantheon: Option<bool>,
    pub religious_contract_count: Option<usize>,
    pub great_work_request_count: Option<usize>,
    pub city_priorities: Option<Vec<(u32, i32)>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inactive {
    pub owner: i32,
    pub skip_off_turn: bool,
    pub off_turn: Vec<Callback>,
    pub culture_analyzed: Option<bool>,
    pub unit_production_calculated: Option<bool>,
    pub unit_efficiencies_count: Option<usize>,
    pub diplomatic_done: Option<bool>,
    pub military_analyzed: Option<bool>,
    pub economic_analyzed: Option<bool>,
    pub city_research_ready: Option<bool>,
    pub readiness_only: Vec<(Callback, bool)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Victory {
    pub alive_majors: Vec<i32>,
    pub team_players: Vec<(i32, Option<Vec<Option<i32>>>)>,
    pub civilization_levels: Vec<(i32, u32)>,
    pub game_start_turn: Option<i32>,
    pub turn_limit_type: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scoring {
    pub caches: Vec<(i32, i32, i32)>,
    pub civilization_levels: Vec<(i32, u32)>,
    pub great_people_received: Option<Vec<(i32, i32)>>,
    pub current_era_scores: Option<Vec<(i32, Vec<i32>)>>,
    pub district_builds: Option<stock::scoring::DistrictBuildMaps>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deals {
    pub global: Vec<u32>,
    pub players: Vec<(i32, Vec<u32>)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub humans: Vec<Human>,
    pub npc: Vec<Npc>,
    pub inactive: Vec<Inactive>,
    pub victory: Victory,
    pub scoring: Scoring,
    pub deals: Deals,
}
struct PreparedHuman {
    input: stock::human_activation::Inputs,
    scout: a::maintenance::ScoutInputs,
    initialized: bool,
}
/// Owned values persist between advances. Mutable deals and callback flags must
/// not be reconstructed from the initial DTO after an action or turn.
pub struct Prepared {
    humans: Vec<PreparedHuman>,
    npc: Vec<stock::npc_leaves::Inputs>,
    pub inactive: RefCell<Vec<stock::inactive_control::Inputs>>,
    pub victory: a::victory_requirements::Inputs,
    pub scoring: RefCell<stock::scoring::Runtime>,
    pub deals: a::treasury_deals::CurrentDeals,
    relationships: a::relationship_requirements::Definitions,
    victory_definitions: a::victory_requirements::Definitions,
    scoring_definitions: stock::scoring::Definitions,
    treasury: a::treasury_deals::Definitions,
    social: a::social::Definitions,
    managers: a::cityless_managers::Definitions,
    maintenance: stock::maintenance::Definitions,
    notifications: crate::loaded_actions::notifications::Definitions,
    phases: a::Phases,
    forced_rebase: a::forced_rebase::Definitions,
}
impl Prepared {
    pub fn new(
        input: Input,
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
    ) -> Result<Self> {
        if input.schema != SCHEMA {
            return Err("initialized turn schema".into());
        }
        let binding = state.binding();
        let mut humans = Vec::new();
        for h in input.humans {
            if humans
                .iter()
                .any(|v: &PreparedHuman| v.input.owner == h.owner)
            {
                return Err("duplicate human activation owner".into());
            }
            let s = h.scout;
            humans.push(PreparedHuman {
                input:stock::human_activation::Inputs::from_initial(state,binding,h.owner,
                    h.off_turn.into_iter().map(Callback::human).collect::<Result<_>>()?,
                    h.priorities.into_iter().map(|(k,p)|Ok((k.human()?,p))).collect::<Result<_>>()?,
                    h.pending_pantheon,h.religious_contract_count,
                    stock::human_activation::Scope::StockHumanExternalPoliciesNoAddedPlanningConsumers).map_err(error)?,
                scout:a::maintenance::ScoutInputs {binding:binding.clone(),uses_q8:s.uses_q8,
                    fallback_q8:s.fallback_q8,primary_regions:s.primary_regions,secondary_regions:s.secondary_regions,
                    continent_discovery_bonus:s.continent_discovery_bonus,natural_wonder_discovery_bonus:s.natural_wonder_discovery_bonus},initialized:h.initialized,
            });
        }
        let mut npc = Vec::new();
        for n in input.npc {
            if npc
                .iter()
                .any(|v: &stock::npc_leaves::Inputs| v.owner == n.owner)
            {
                return Err("duplicate NPC activation owner".into());
            }
            let mut value = stock::npc_leaves::Inputs::from_initial(
                state,
                binding,
                n.owner,
                n.off_turn.into_iter().map(Callback::npc).collect(),
                n.priorities
                    .into_iter()
                    .map(|(k, p)| (k.npc(), p))
                    .collect(),
                n.pending_pantheon,
                n.religious_contract_count,
                stock::npc_leaves::Scope::StockNpcExternalPoliciesNoAddedPlanningConsumers,
            )
            .map_err(error)?
            .with_great_work_request_count(n.great_work_request_count);
            if let Some(p) = n.city_priorities {
                value = value.with_city_priorities(p).map_err(error)?;
            }
            npc.push(value);
        }
        let mut inactive = Vec::new();
        for i in input.inactive {
            if inactive
                .iter()
                .any(|v: &stock::inactive_control::Inputs| v.owner == i.owner)
            {
                return Err("duplicate inactive controller owner".into());
            }
            let mut value = stock::inactive_control::Inputs::from_initial(
                state,
                binding,
                i.owner,
                i.skip_off_turn,
                i.off_turn
                    .into_iter()
                    .map(Callback::human)
                    .collect::<Result<_>>()?,
                stock::inactive_control::Flags {
                    culture_analyzed: i.culture_analyzed,
                    unit_production_calculated: i.unit_production_calculated,
                    unit_efficiencies_count: i.unit_efficiencies_count,
                    diplomatic_done: i.diplomatic_done,
                    military_analyzed: i.military_analyzed,
                    economic_analyzed: i.economic_analyzed,
                },
                stock::inactive_control::Scope::StockNoAddedPlanningConsumers,
            )
            .map_err(error)?
            .with_city_research_ready(i.city_research_ready);
            for (kind, ready) in i.readiness_only {
                value = value
                    .with_readiness_only(kind.human()?, ready)
                    .map_err(error)?;
            }
            inactive.push(value);
        }
        let v = input.victory;
        let s = input.scoring;
        Ok(Self {
            humans,
            npc,
            inactive: RefCell::new(inactive),
            victory: a::victory_requirements::Inputs {
                binding: binding.clone(),
                alive_majors: v.alive_majors,
                team_players: v.team_players,
                civilization_levels: v.civilization_levels,
                game_start_turn: v.game_start_turn,
                turn_limit_type: v.turn_limit_type,
            },
            scoring: RefCell::new(stock::scoring::Runtime {
                binding: binding.clone(),
                caches: s
                    .caches
                    .into_iter()
                    .map(|(o, turn, score)| (o, stock::scoring::Cache { turn, score }))
                    .collect(),
                globals: stock::scoring::GlobalInputs {
                    civilization_levels: s.civilization_levels,
                    great_people_received: s.great_people_received,
                    current_era_scores: s.current_era_scores,
                    district_builds: s.district_builds,
                },
            }),
            deals: a::treasury_deals::CurrentDeals::from_initial(
                state,
                input.deals.global,
                input.deals.players,
            )
            .map_err(error)?,
            relationships: a::relationship_requirements::Definitions::from_rules(rules, resources)
                .map_err(error)?,
            victory_definitions: a::victory_requirements::Definitions::from_rules(rules, resources)
                .map_err(error)?,
            scoring_definitions: stock::scoring::Definitions::from_rules(rules, resources)
                .map_err(error)?,
            treasury: a::treasury_deals::Definitions::from_rules(state, rules).map_err(error)?,
            social: a::social::Definitions::from_rules(state, rules).map_err(error)?,
            managers: a::cityless_managers::Definitions::from_rules(state, rules).map_err(error)?,
            maintenance: stock::maintenance::Definitions::from_rules(state, rules, resources)
                .map_err(error)?,
            notifications: crate::loaded_actions::notifications::Definitions::from_rules(
                rules, resources,
            )
            .map_err(error)?,
            phases: a::Phases::from_rules(rules, resources).map_err(error)?,
            forced_rebase: a::forced_rebase::Definitions::from_rules(state, rules, resources)
                .map_err(error)?,
        })
    }
    /// Bind all retained owners in one scope; callback references cannot escape.
    /// Additional feature owners are mandatory explicit inputs, never defaults.
    pub fn with_services<'s, 'd, 'r, R>(
        &'s mut self,
        common: Common<'s, 'd>,
        current: Current<'s, 'd, 'r>,
        run: impl FnOnce(turn::StockTurnServices<'_, 'd, 'r>) -> R,
    ) -> R {
        let humans = self
            .humans
            .iter()
            .map(|h| turn::Human {
                inputs: &h.input,
                scout: &h.scout,
                initialized: h.initialized,
            })
            .collect::<Vec<_>>();
        let npc = self.npc.iter().collect::<Vec<_>>();
        // The locally assembled reference lists require a shorter borrow scope.
        run(turn::StockTurnServices {
            requirements: current.requirements,
            listeners: current.listeners,
            relationships: &self.relationships,
            victory_definitions: &self.victory_definitions,
            victory: &self.victory,
            influence: current.influence,
            humans: &humans,
            npc: &npc,
            inactive: &self.inactive,
            scoring: &self.scoring,
            scoring_definitions: &self.scoring_definitions,
            research_definitions: common.research,
            choices: common.technologies,
            culture: common.culture,
            treasury: &self.treasury,
            deals: &mut self.deals,
            social_definitions: &self.social,
            social_queries: current.social,
            notifications: &self.notifications,
            manager_definitions: &self.managers,
            manager_queries: current.manager_queries,
            managers: current.managers,
            maintenance: &self.maintenance,
            movement: common.movement,
            placement: current.placement,
            phases: &self.phases,
            forced_rebase: &self.forced_rebase,
            city: current.city,
            unit_production: current.unit_production,
            camps: current.camps,
            city_outcomes: vec![],
            city_reports: vec![],
        })
    }
}
/// Common definitions already retained by the initialized Session arena.
pub struct Common<'s, 'd> {
    pub research: &'s crate::research::Definitions,
    pub technologies: &'s crate::loaded_actions::choose_tech::Definitions,
    pub culture: &'s stock::research::CultureInputs<'s>,
    pub movement: &'s crate::unit_movement_state::Inputs<'d>,
}
/// Additional gameplay owners not yet transported by this DTO. The bounded
/// cityless binding below exposes missing city/production/camp work as errors.
/// A complete initialized turn profile must supply their dedicated bindings.
pub struct Current<'s, 'd, 'r> {
    pub requirements: &'s RefCell<a::requirement_changes::Runtime>,
    pub listeners: &'s [a::turn_started::Listener],
    pub influence: stock::turn::InfluenceInputs<'s, 'r>,
    pub social: &'s a::social::Queries<'s>,
    pub manager_queries: &'s a::cityless_managers::Queries<'s>,
    pub managers: &'s RefCell<a::cityless_managers::Runtime>,
    pub placement: &'s dyn stock::movement::PlacementQueries,
    pub city: Option<turn::City<'s>>,
    pub unit_production: Option<turn::UnitProduction<'s, 'd, 'r>>,
    pub camps: Option<turn::Camp<'s, 'd, 'r>>,
}
