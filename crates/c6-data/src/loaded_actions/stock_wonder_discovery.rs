//! Reached natural-wonder discovery subscribers in retained native order.
//! Current player type, era rates, feature flags, research and history owners
//! determine each branch. Unknown positive continuations remain explicit.
use super::{movement::Error, Runtime};
use crate::Rules;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};

#[derive(Clone, Copy, Debug)]
pub enum Subscriber {
    SummaryMajorsOnly,
    Gossip,
    EraScore,
    History,
    Boost { row: i64 },
}
/// Actual stock NaturalWonderRevealed routes, including both Host publishers,
/// with no added synchronous gameplay consumers. The discovery popup and the
/// UI's SkipNextAutoEndTurn flag become semantic headless output.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevealedReport {
    pub propagation: c6_core::visibility_change::Propagation,
    pub feature: i32,
}
/// Controller requests retained at the external policy boundary. An ordinary
/// explicit command can inhibit UI auto-end-turn without forcing a new player
/// decision. A positive reactivation during automation remains unsupported.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DecisionControl {
    pub skip_next_automatic_end_turn: bool,
    pub reactivation_requested_for: Option<i32>,
}
#[derive(Clone, Copy)]
enum Current {
    SummaryMajorsOnly,
    Gossip,
    EraScore,
    History,
    Boost { index: usize },
}
pub(super) struct Bindings {
    order: Vec<Current>,
    ever_alive_majors: Vec<Option<i32>>,
    pub(super) unit_wonder_empty: bool,
    pub(super) promotion_classes: Vec<(u32, Option<String>)>,
    pub(super) experience: i32,
    pub(super) change_definitions: Option<crate::melee_attack::experience::Definitions>,
    pub(super) maximum_damage: i32,
    pub(super) bonuses: crate::registry::LinuxDefinitionRegistry,
    report_scope: Option<Scope>,
    revealed_reports: Vec<RevealedReport>,
    decision_control: DecisionControl,
}
impl<P> Runtime<'_, P> {
    /// Complete actual GameSignals+678 order and its WasEverAlive-major pointer
    /// roster. These are current registrations/identities, never callback answers.
    pub fn bind_wonder_discovery(
        &mut self,
        state: &LoadedState,
        rules: &Rules,
        order: Vec<Subscriber>,
        ever_alive_majors: Vec<Option<i32>>,
        unit_wonder_empty: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("wonder discovery identity"))?;
        let boosts = rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("wonder boost definitions"))?;
        let order = order
            .into_iter()
            .map(|subscriber| {
                Ok(match subscriber {
                    Subscriber::SummaryMajorsOnly => Current::SummaryMajorsOnly,
                    Subscriber::Gossip => Current::Gossip,
                    Subscriber::EraScore => Current::EraScore,
                    Subscriber::History => Current::History,
                    Subscriber::Boost { row } => Current::Boost {
                        index: boosts
                            .rows()
                            .enumerate()
                            .find(|(_, r)| r.row_id().is_some_and(|id| id.0 == row))
                            .ok_or(Error::Missing("wonder boost row"))?
                            .0,
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let units = crate::unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        let classes = crate::unit_initialization_definitions::collection(
            rules,
            "UnitPromotionClasses",
            "PromotionClassType",
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        let unit_rows = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("sight Unit rules"))?;
        let promotion_classes = units
            .entries()
            .iter()
            .map(|entry| {
                let row = unit_rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("sight Unit row"))?;
                let class = match row
                    .get("PromotionClass")
                    .map_err(|e| Error::Requirement(format!("{e:?}")))?
                {
                    crate::SqlValue::Null => None,
                    crate::SqlValue::Text(value) => classes
                        .find_index(crate::registry::linux_type_hash(value))
                        .map(|_| String::from_utf8(value.to_vec()))
                        .transpose()
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?,
                    _ => return Err(Error::Missing("sight Unit promotion reference")),
                };
                Ok((crate::registry::linux_type_hash(&entry.type_name), class))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let parameter = |name| -> Result<i32, Error> {
            rules
                .gameplay()
                .table("GlobalParameters")
                .and_then(|t| t.rows().find(|r| r.text("Name").ok() == Some(name)))
                .and_then(|r| r.get("Value").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or(Error::Missing("sight experience parameter"))
        };
        let experience = parameter("EXPERIENCE_REVEAL_NATURAL_WONDER")?;
        let maximum_damage = parameter("COMBAT_MAX_HIT_POINTS")?;
        let bonuses = crate::unit_initialization_definitions::collection(
            rules,
            "GovernmentBonusNames",
            "GovernmentBonusType",
        )
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        let current = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("wonder sight owner"))?;
        if current.wonders.is_some() {
            return Err(Error::Missing("wonder discovery already bound"));
        }
        current.wonders = Some(Bindings {
            order,
            ever_alive_majors,
            unit_wonder_empty,
            promotion_classes,
            experience,
            change_definitions: None,
            maximum_damage,
            bonuses,
            report_scope: None,
            revealed_reports: Vec::new(),
            decision_control: DecisionControl::default(),
        });
        Ok(())
    }
    pub fn bind_wonder_discovery_reports(
        &mut self,
        state: &LoadedState,
        scope: Scope,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("wonder report identity"))?;
        let binding = self
            .resource_revelation
            .as_mut()
            .and_then(|r| r.wonders.as_mut())
            .ok_or(Error::Missing("wonder discovery binding"))?;
        if binding.report_scope.is_some() {
            return Err(Error::Missing("wonder reports already bound"));
        }
        binding.report_scope = Some(scope);
        Ok(())
    }
    pub fn wonder_revealed_reports(&self) -> &[RevealedReport] {
        self.resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .map_or(&[], |r| r.revealed_reports.as_slice())
    }
    pub fn wonder_discovery_decision_control(&self) -> DecisionControl {
        self.resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .map_or(DecisionControl::default(), |r| r.decision_control)
    }
    pub(super) fn report_wonder_revealed(
        &mut self,
        state: &LoadedState,
        propagation: c6_core::visibility_change::Propagation,
        feature: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("wonder report identity"))?;
        let processing = state.processing_automation().map_err(Error::State)?;
        let observer = self
            .movement()?
            .local_observer
            .ok_or(Error::Missing("wonder report local observer"))?;
        let binding = self
            .resource_revelation
            .as_mut()
            .and_then(|r| r.wonders.as_mut())
            .ok_or(Error::Missing("wonder discovery binding"))?;
        binding
            .report_scope
            .ok_or(Error::Missing("wonder reveal report consumers"))?;
        // HOST3214fe0 compares GetPlayerProcessingAutomation, not the acting
        // player. It requests reactivation only for the current local pass.
        // HOST320e2e0 inhibits auto-end-turn in either case. External policies
        // retain their explicit EndTurn choice; no policy is selected here.
        binding.decision_control.skip_next_automatic_end_turn = true;
        if processing == observer {
            binding.decision_control.reactivation_requested_for = Some(observer);
            return Err(Error::Missing("wonder automation player reactivation"));
        }
        binding.revealed_reports.push(RevealedReport {
            propagation,
            feature,
        });
        Ok(())
    }
    /// Bind canonical XP definitions from this state's authoritative resources.
    pub fn bind_wonder_experience(
        &mut self,
        state: &LoadedState,
        rules: &Rules,
        authority: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("wonder experience identity"))?;
        let definitions =
            crate::melee_attack::experience::Definitions::from_rules(state, rules, authority)
                .map_err(|e| Error::Requirement(format!("wonder experience definitions: {e:?}")))?;
        let binding = self
            .resource_revelation
            .as_mut()
            .and_then(|r| r.wonders.as_mut())
            .ok_or(Error::Missing("wonder discovery binding"))?;
        if binding.change_definitions.is_some() {
            return Err(Error::Missing("wonder experience already bound"));
        }
        binding.change_definitions = Some(definitions);
        Ok(())
    }
    pub(super) fn wonder_discovered(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        feature: i32,
    ) -> Result<(), Error> {
        let binding = self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing(
                "current natural wonder signal registrations",
            ))?;
        let order = binding.order.clone();
        let ever_alive = binding.ever_alive_majors.clone();
        for subscriber in order {
            match subscriber {
                Current::SummaryMajorsOnly => {
                    // GeneralPlayerDataProvider's actual include flags select
                    // majors only; NaturalWonderFound explicitly tests IsMajor.
                    let level = self
                        .movement()?
                        .civilization_levels
                        .iter()
                        .find(|r| r.0 == player)
                        .ok_or(Error::Missing("wonder current civilization level"))?
                        .1;
                    // OnFindWonder26445f0 only increments the Summary
                    // repository dataset. It has no gameplay subscribers or
                    // RNG; like UnitCreated's HistoricalSummary this output
                    // is omitted at the stock headless boundary.
                    let _major = level == 0x253718b0;
                }
                Current::Gossip => {
                    let level = self
                        .movement()?
                        .civilization_levels
                        .iter()
                        .find(|r| r.0 == player)
                        .ok_or(Error::Missing("wonder current civilization level"))?
                        .1;
                    if level == 0x253718b0 && self.current_gossip_history_empty() != Some(true) {
                        // AddGossipItem29b25d0 tests the live history size
                        // before storing an occurrence or reporting it.
                        return Err(Error::Missing("wonder gossip nonempty live history"));
                    }
                }
                Current::EraScore => {
                    // Eras::$_11 has no major guard. Read its actual current
                    // +4c score rate even for a trivial/nonmajor player.
                    let rate = state
                        .game_era_discovery_rates(player)
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        .map_or(0, |r| r[1]);
                    if rate != 0 {
                        return Err(Error::Missing("positive wonder era score continuation"));
                    }
                }
                Current::History => {
                    let features = self.movement()?.definitions.features;
                    let mut other_found = false;
                    for other in ever_alive
                        .iter()
                        .flatten()
                        .copied()
                        .filter(|p| *p != player)
                    {
                        if crate::unit_sight_state::revealed_feature::get(
                            state, features, other, feature,
                        )
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        {
                            other_found = true;
                            break;
                        }
                    }
                    // OnFindWonder constructs this actual temporary Moment
                    // before HandleMoment applies its capability/major guard.
                    let moment = Moment {
                        wire_version: MOMENT_VERSION,
                        type_hash: if other_found { 0xb0ded3d6 } else { 0xbc50184b },
                        turn: -1,
                        action_plot: PlotCoord { x: -9999, y: -9999 },
                        acting_player_ordinal: player,
                        id: -1,
                        has_ever_been_commemorated_raw: 0,
                        game_era_hash: u32::MAX,
                        extra_data: vec![MomentData {
                            kind_raw: 0x536178f1,
                            value: feature,
                        }],
                        city_name_bytes: vec![],
                        unit_citizen_name_bytes: vec![],
                        other_city_name_bytes: vec![],
                        era_score: 0,
                    };
                    let history = self
                        .unit_creation
                        .as_ref()
                        .and_then(|u| u.history.as_ref())
                        .ok_or(Error::Missing("wonder canonical history owner"))?;
                    if !history.presentation_qualified {
                        return Err(Error::Missing("wonder history report consumers"));
                    }
                    let mut effects =
                        crate::loaded_actions::government_actions::history::StockEffects {
                            recorded: history.recorded,
                            reports: Vec::new(),
                        };
                    crate::melee_attack::resolve::completion::history::handle_moment(
                        state,
                        history.definitions,
                        history.inputs,
                        moment,
                        &mut effects,
                    )
                    .map_err(|e| Error::Requirement(format!("wonder history: {e:?}")))?;
                }
                Current::Boost { index } => {
                    // Canonical TriggerBoost reads logging, then current scratch
                    // and barbarian state before any research/telemetry write.
                    self.action_research_boost(state, player, index)
                        .map_err(|e| Error::Requirement(format!("wonder boost: {e:?}")))?;
                }
            }
        }
        self.reached
            .push("stock natural wonder discovery subscribers");
        Ok(())
    }
}
