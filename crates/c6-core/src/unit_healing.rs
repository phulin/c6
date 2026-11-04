//! Linux XP2 per-unit end-turn healing amount calculation.
//!
//! Computes Rules::Units::Instance::HealUnit's positive ChangeDamage request
//! and applies its local-authority damage/report/clear/signal continuation to
//! a live Unit through mandatory synchronous world providers. World resolution,
//! subscriber implementations and complete turn upkeep remain separate.
//! Inputs must come from one authoritative current state and configured rules;
//! unavailable provider values must not be replaced with defaults. Native
//! differential validation of this calculation remains outstanding.

use crate::unit::{ComponentId, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingTerritory {
    Friendly,
    Neutral,
    Enemy,
    /// Any GetDiplomaticTerritory result other than 1, 2, or 3.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingDomain {
    /// Native GetDomain() == 0.
    Naval,
    /// Every other native domain value follows this branch.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerritoryHealing {
    pub friendly: i32,
    pub neutral: i32,
    pub enemy: i32,
}

impl TerritoryHealing {
    fn at(self, territory: HealingTerritory) -> i32 {
        match territory {
            HealingTerritory::Friendly => self.friendly,
            HealingTerritory::Neutral => self.neutral,
            HealingTerritory::Enemy => self.enemy,
            HealingTerritory::Other => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingRules {
    /// COMBAT_HEAL_NAVAL_{FRIENDLY,NEUTRAL,ENEMY}.
    pub naval: TerritoryHealing,
    /// COMBAT_HEAL_LAND_{FRIENDLY,NEUTRAL,ENEMY}.
    pub land: TerritoryHealing,
    /// COMBAT_HEAL_CITY_GARRISON; replaces land friendly base on city plots.
    pub city_garrison: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingResource {
    NoStrategicResource,
    Required {
        /// CanAccessResourceForUnitHealing(owner, definition resource index).
        accessible_for_healing: bool,
        /// GetResourceAmount(resource, ownership=0, final bool=true).
        resource_amount: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingImprovement {
    pub pillaged: bool,
    /// Actual improvement definition's ReligiousUnitHealRate.
    pub religious_heal_rate: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingGovernor {
    pub established: bool,
    pub extra_heal: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingCity {
    /// Religion::GetExtraReligiousHeal for the city purchasing this plot.
    pub extra_religious_heal: i32,
    /// Assigned governor of that city's owner, if any.
    pub assigned_governor: Option<HealingGovernor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingPlot {
    pub owned_by_unit_owner: bool,
    pub is_city: bool,
    pub improvement: Option<HealingImprovement>,
    pub purchased_by_city: Option<HealingCity>,
    /// Owner's Districts::GetReligionHealRate at this plot.
    pub district_religion_heal_rate: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitHealingState {
    pub eligible_to_heal: bool,
    pub delayed_death: bool,
    pub territory: HealingTerritory,
    pub no_foreign_heal: bool,
    pub no_neutral_heal: bool,
    pub resource: HealingResource,
    pub domain: HealingDomain,
    pub religious_strength: i32,
    pub embarked: bool,
    pub health: TerritoryHealing,
    pub religion_health: TerritoryHealing,
    pub plot: Option<HealingPlot>,
}

/// Requested healing, before damage clamping and synchronous native callbacks.
///
/// A positive result causes native ChangeDamage(-amount), followed by a signal
/// carrying both this requested amount and actual old_damage - new_damage.
/// Zero causes neither call. In particular, a full-health unit can still have
/// a positive request and therefore deliver callbacks. This function is pure
/// and consumes no RNG; it does not claim that those callbacks consume none.
pub fn healing_amount(unit: Option<&UnitHealingState>, rules: &HealingRules) -> i32 {
    let Some(unit) = unit else { return 0 };
    let Some(plot) = unit.plot else { return 0 };
    if !unit.eligible_to_heal || unit.delayed_death {
        return 0;
    }
    if let HealingResource::Required {
        accessible_for_healing: false,
        resource_amount,
    } = unit.resource
    {
        if resource_amount <= 0 {
            return 0;
        }
    }
    if (unit.territory == HealingTerritory::Enemy && unit.no_foreign_heal)
        || (unit.territory == HealingTerritory::Neutral && unit.no_neutral_heal)
    {
        return 0;
    }

    let amount = if unit.domain == HealingDomain::Naval {
        if unit.territory == HealingTerritory::Other {
            return 0;
        }
        rules
            .naval
            .at(unit.territory)
            .wrapping_add(unit.health.at(unit.territory))
    } else if unit.religious_strength > 0 {
        let improvement_rate = if plot.owned_by_unit_owner {
            plot.improvement
                .filter(|i| !i.pillaged)
                .map_or(0, |i| i.religious_heal_rate.max(0))
        } else {
            0
        };
        let base = plot.district_religion_heal_rate.max(improvement_rate);
        let amount = base.wrapping_add(unit.religion_health.at(unit.territory));
        if plot.owned_by_unit_owner {
            amount.wrapping_add(plot.purchased_by_city.map_or(0, |c| c.extra_religious_heal))
        } else {
            amount
        }
    } else {
        let base = match unit.territory {
            HealingTerritory::Friendly => {
                let base = if plot.is_city {
                    rules.city_garrison
                } else {
                    rules.land.friendly
                };
                base.wrapping_add(unit.health.friendly)
            }
            HealingTerritory::Enemy | HealingTerritory::Neutral if !unit.embarked => rules
                .land
                .at(unit.territory)
                .wrapping_add(unit.health.at(unit.territory)),
            _ => 0,
        };
        let governor = if plot.owned_by_unit_owner {
            plot.purchased_by_city
                .and_then(|c| c.assigned_governor)
                .filter(|g| g.established)
        } else {
            None
        };
        base.wrapping_add(governor.map_or(0, |g| g.extra_heal))
    };
    amount.max(0)
}

/// World-derived reads not stored authoritatively in the Unit record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingWorldState {
    pub territory: HealingTerritory,
    pub no_foreign_heal: bool,
    pub no_neutral_heal: bool,
    pub resource: HealingResource,
    pub domain: HealingDomain,
    pub embarked: bool,
    pub plot: Option<HealingPlot>,
}

impl UnitHealingState {
    /// Stored actor fields always come from the authoritative live Unit.
    pub fn from_unit(unit: &Unit, world: HealingWorldState) -> Self {
        Self {
            eligible_to_heal: unit.b_eligible_to_heal,
            delayed_death: unit.b_delayed_death,
            territory: world.territory,
            no_foreign_heal: world.no_foreign_heal,
            no_neutral_heal: world.no_neutral_heal,
            resource: world.resource,
            domain: world.domain,
            religious_strength: unit.k_religion.i_religious_strength,
            embarked: world.embarked,
            health: TerritoryHealing {
                friendly: unit.k_health_modifiers.i_extra_friendly_heal,
                neutral: unit.k_health_modifiers.i_extra_neutral_heal,
                enemy: unit.k_health_modifiers.i_extra_enemy_heal,
            },
            religion_health: TerritoryHealing {
                friendly: unit
                    .k_health_religion_modifiers
                    .i_extra_friendly_heal_religion,
                neutral: unit
                    .k_health_religion_modifiers
                    .i_extra_neutral_heal_religion,
                enemy: unit.k_health_religion_modifiers.i_extra_enemy_heal_religion,
            },
            plot: world.plot,
        }
    }
}

pub const DAMAGE_CHANGED: u32 = 0xf8a5ab14;
/// Logical registered hash, never the process's numeric operation ID.
pub const HEAL_OPERATION_HASH: u32 = 0x7eb896fb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageChangedReport {
    pub event_hash: u32,
    pub identity: ComponentId,
    pub damage: i32,
    pub previous_damage: i32,
    pub maximum_damage: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitHealedSignal {
    pub requested: i32,
    /// Old damage minus current damage after reporting and operation clearing.
    pub actual: i32,
}

/// Binds one live actor and its authoritative world for the entire call.
///
/// Queries are read-only projections of actual current providers. Missing
/// inputs are errors. Synchronous callbacks receive this same context and may
/// mutate the actor/world; no snapshot copyback or default no-op exists.
/// Native archive dirty bookkeeping is omitted only under the same audited
/// local-authority profile as unit_turn_upkeep.
pub trait UnitHealingContext {
    type Error;
    /// Resolve the same retained live actor. Removal by a synchronous callback
    /// must return Err on subsequent access; do not revive a detached copy.
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    fn healing_world_state(&self) -> Result<HealingWorldState, Self::Error>;
    fn healing_rules(&self) -> Result<HealingRules, Self::Error>;
    /// Pure capability check before any field change or RNG consumption.
    /// Require complete report/signal subscribers and all operation Clear
    /// continuations reachable even after synchronous subscriber mutations.
    /// Unknown required effects must reject here, preserving all state.
    fn preflight_healing_effects(&self) -> Result<(), Self::Error>;
    /// Actual COMBAT_MAX_HIT_POINTS. SetDamage clamps to this global rule,
    /// even when the actor's dynamic maximum-damage property differs.
    fn combat_max_hit_points(&self) -> Result<i32, Self::Error>;
    /// GetMaxDamage at the report point: current dynamic property 0x7d3fc711
    /// converted by the native TypedVariant integer semantics, or the actual
    /// COMBAT_MAX_HIT_POINTS only if the property is absent.
    fn maximum_damage_for_report(&self) -> Result<i32, Self::Error>;
    fn send_damage_changed(&mut self, report: DamageChangedReport) -> Result<(), Self::Error>;
    /// Exact Operations::Manager::Clear continuation. Delegate orchestration
    /// to unit_operations_clear::clear_operations with complete providers.
    /// This includes deactivation,
    /// repeated current-front removals and current formation path clearing.
    /// Clearing just the Vec or recording a transcript is insufficient.
    fn clear_current_operations(&mut self) -> Result<(), Self::Error>;
    /// Game::Signals +0x1518, dispatch(unit, requested, actual), synchronous.
    /// The actor is this context's live Unit, not a saved ComponentID lookup.
    fn dispatch_unit_healed(&mut self, signal: UnitHealedSignal) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingStage {
    ActorAccess,
    MaximumDamageQuery,
    DamageChangedReport,
    ClearOperations,
    HealedSignal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitHealingError<E> {
    InputsUnavailable(E),
    EffectsUnavailable(E),
    /// Any completed damage write/subscriber effects remain visible; actor
    /// access can fail before the first write or after synchronous removal.
    ContinuationFailed {
        stage: HealingStage,
        error: E,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitHealingOutcome {
    Skipped,
    /// Signal completed; subscribers own subsequent state.
    Signalled(UnitHealedSignal),
}

/// Actual Unit::ChangeDamage/SetDamage, shared by healing and movement damage.
/// The caller checks support for all reached reports/Clear effects before this
/// function. This leaf does not perform Manager::IsDead/Kill or a healed signal.
/// Returns damage captured before the write; callbacks may change current damage.
pub fn change_damage<C: UnitHealingContext>(
    context: &mut C,
    delta: i32,
) -> Result<i32, UnitHealingError<C::Error>> {
    let limit = context
        .combat_max_hit_points()
        .map_err(UnitHealingError::InputsUnavailable)?;
    let previous_damage = context
        .unit()
        .map_err(|error| UnitHealingError::ContinuationFailed {
            stage: HealingStage::ActorAccess,
            error,
        })?
        .i_damage;
    let candidate = previous_damage.wrapping_add(delta);
    // Utilities::Range(candidate, 0, limit) compares candidate twice. In
    // particular, a negative configured upper bound is not normalized to zero.
    let damage = if candidate < 0 {
        0
    } else {
        candidate.min(limit)
    };
    context
        .unit_mut()
        .map_err(|error| UnitHealingError::ContinuationFailed {
            stage: HealingStage::ActorAccess,
            error,
        })?
        .i_damage = damage;
    let actor = context
        .unit()
        .map_err(|error| UnitHealingError::ContinuationFailed {
            stage: HealingStage::ActorAccess,
            error,
        })?;
    if previous_damage != actor.i_damage {
        let identity = ComponentId {
            owner: actor.e_owner as i16,
            component_type: 1,
            id: actor.id,
        };
        let damage = actor.i_damage;
        let maximum_damage = context.maximum_damage_for_report().map_err(|error| {
            UnitHealingError::ContinuationFailed {
                stage: HealingStage::MaximumDamageQuery,
                error,
            }
        })?;
        context
            .send_damage_changed(DamageChangedReport {
                event_hash: DAMAGE_CHANGED,
                identity,
                damage,
                previous_damage,
                maximum_damage,
            })
            .map_err(|error| UnitHealingError::ContinuationFailed {
                stage: HealingStage::DamageChangedReport,
                error,
            })?;
    }
    // Reread after synchronous reporting: subscribers can change damage/queue.
    let actor = context
        .unit()
        .map_err(|error| UnitHealingError::ContinuationFailed {
            stage: HealingStage::ActorAccess,
            error,
        })?;
    if actor.i_damage <= 0
        && actor
            .k_operations
            .nodes
            .first()
            .is_some_and(|n| n.operation_hash == HEAL_OPERATION_HASH)
    {
        context.clear_current_operations().map_err(|error| {
            UnitHealingError::ContinuationFailed {
                stage: HealingStage::ClearOperations,
                error,
            }
        })?;
    }
    Ok(previous_damage)
}

/// Apply the healing path's real damage write and synchronous continuations.
///
/// The queue stores admitted logical operation hashes. Its live numeric-ID
/// registry mapping must have been established by the runtime binding; a raw
/// numeric ID must never be placed in OperationNode::operation_hash.
pub fn heal_unit<C: UnitHealingContext>(
    context: Option<&mut C>,
) -> Result<UnitHealingOutcome, UnitHealingError<C::Error>> {
    let Some(context) = context else {
        return Ok(UnitHealingOutcome::Skipped);
    };
    let world = context
        .healing_world_state()
        .map_err(UnitHealingError::InputsUnavailable)?;
    let rules = context
        .healing_rules()
        .map_err(UnitHealingError::InputsUnavailable)?;
    let state = UnitHealingState::from_unit(
        context
            .unit()
            .map_err(|error| UnitHealingError::ContinuationFailed {
                stage: HealingStage::ActorAccess,
                error,
            })?,
        world,
    );
    let requested = healing_amount(Some(&state), &rules);
    if requested == 0 {
        return Ok(UnitHealingOutcome::Skipped);
    }
    context
        .preflight_healing_effects()
        .map_err(UnitHealingError::EffectsUnavailable)?;
    let previous_damage = change_damage(context, requested.wrapping_neg())?;
    let signal = UnitHealedSignal {
        requested,
        actual: previous_damage.wrapping_sub(
            context
                .unit()
                .map_err(|error| UnitHealingError::ContinuationFailed {
                    stage: HealingStage::ActorAccess,
                    error,
                })?
                .i_damage,
        ),
    };
    context
        .dispatch_unit_healed(signal)
        .map_err(|error| UnitHealingError::ContinuationFailed {
            stage: HealingStage::HealedSignal,
            error,
        })?;
    Ok(UnitHealingOutcome::Signalled(signal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (UnitHealingState, HealingRules) {
        (
            UnitHealingState {
                eligible_to_heal: true,
                delayed_death: false,
                territory: HealingTerritory::Friendly,
                no_foreign_heal: false,
                no_neutral_heal: false,
                resource: HealingResource::NoStrategicResource,
                domain: HealingDomain::Other,
                religious_strength: 0,
                embarked: false,
                health: TerritoryHealing {
                    friendly: 3,
                    neutral: 5,
                    enemy: 7,
                },
                religion_health: TerritoryHealing {
                    friendly: 11,
                    neutral: 13,
                    enemy: 17,
                },
                plot: Some(HealingPlot {
                    owned_by_unit_owner: true,
                    is_city: false,
                    improvement: Some(HealingImprovement {
                        pillaged: false,
                        religious_heal_rate: 19,
                    }),
                    purchased_by_city: Some(HealingCity {
                        extra_religious_heal: 23,
                        assigned_governor: Some(HealingGovernor {
                            established: true,
                            extra_heal: 29,
                        }),
                    }),
                    district_religion_heal_rate: 31,
                }),
            },
            HealingRules {
                naval: TerritoryHealing {
                    friendly: 37,
                    neutral: 41,
                    enemy: 43,
                },
                land: TerritoryHealing {
                    friendly: 47,
                    neutral: 53,
                    enemy: 59,
                },
                city_garrison: 61,
            },
        )
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Failure {
        Unsupported,
        Callback,
    }

    struct LiveContext {
        actor: Unit,
        world: HealingWorldState,
        rules: HealingRules,
        limit: i32,
        report_maximum: Result<i32, Failure>,
        admitted: bool,
        actor_removed: bool,
        remove_after_report: bool,
        remove_after_clear: bool,
        fail_at: Option<HealingStage>,
        after_report: Option<(i32, bool)>,
        after_clear_damage: Option<i32>,
        after_signal_damage: Option<i32>,
        log: Vec<&'static str>,
        reports: Vec<DamageChangedReport>,
        signals: Vec<UnitHealedSignal>,
    }

    impl LiveContext {
        fn new(damage: i32, heal_front: bool) -> Self {
            let (state, rules) = fixture();
            let mut actor = crate::unit_turn_upkeep::tests::source_only_unit();
            actor.id = 77;
            actor.e_owner = -32769;
            actor.i_damage = damage;
            actor.b_eligible_to_heal = true;
            actor.b_delayed_death = false;
            actor.k_religion.i_religious_strength = 0;
            actor.k_health_modifiers.i_extra_friendly_heal = 3;
            actor.k_operations.nodes.clear();
            let mut context = Self {
                actor,
                rules,
                limit: 100,
                report_maximum: Ok(300),
                admitted: true,
                actor_removed: false,
                remove_after_report: false,
                remove_after_clear: false,
                world: HealingWorldState {
                    territory: state.territory,
                    no_foreign_heal: state.no_foreign_heal,
                    no_neutral_heal: state.no_neutral_heal,
                    resource: state.resource,
                    domain: state.domain,
                    embarked: state.embarked,
                    plot: state.plot,
                },
                fail_at: None,
                after_report: None,
                after_clear_damage: None,
                after_signal_damage: None,
                log: Vec::new(),
                reports: Vec::new(),
                signals: Vec::new(),
            };
            context.set_front(heal_front);
            context
        }
        fn set_front(&mut self, heal: bool) {
            self.actor.k_operations.nodes = vec![crate::unit::OperationNode {
                operation_hash: if heal {
                    HEAL_OPERATION_HASH
                } else {
                    crate::unit::SKIP_TURN_HASH
                },
                directive: 0,
                flags: 0,
                push_turn: 0,
                previous_activity: 0,
                previous_automation: 0,
                parameters: crate::unit::OperationParameters::None,
            }]
            .into();
        }
        fn callback_result(&self, stage: HealingStage) -> Result<(), Failure> {
            if self.fail_at == Some(stage) {
                Err(Failure::Callback)
            } else {
                Ok(())
            }
        }
    }

    impl UnitHealingContext for LiveContext {
        type Error = Failure;
        fn unit(&self) -> Result<&Unit, Failure> {
            if self.actor_removed {
                Err(Failure::Unsupported)
            } else {
                Ok(&self.actor)
            }
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Failure> {
            if self.actor_removed {
                Err(Failure::Unsupported)
            } else {
                Ok(&mut self.actor)
            }
        }
        fn healing_world_state(&self) -> Result<HealingWorldState, Failure> {
            Ok(self.world)
        }
        fn healing_rules(&self) -> Result<HealingRules, Failure> {
            Ok(self.rules)
        }
        fn preflight_healing_effects(&self) -> Result<(), Failure> {
            if self.admitted {
                Ok(())
            } else {
                Err(Failure::Unsupported)
            }
        }
        fn combat_max_hit_points(&self) -> Result<i32, Failure> {
            Ok(self.limit)
        }
        fn maximum_damage_for_report(&self) -> Result<i32, Failure> {
            self.report_maximum
        }
        fn send_damage_changed(&mut self, report: DamageChangedReport) -> Result<(), Failure> {
            assert_eq!(self.actor.i_damage, report.damage);
            self.log.push("damage_changed");
            self.reports.push(report);
            if let Some((damage, heal)) = self.after_report {
                self.actor.i_damage = damage;
                self.set_front(heal);
            }
            self.actor_removed = self.remove_after_report;
            self.callback_result(HealingStage::DamageChangedReport)
        }
        fn clear_current_operations(&mut self) -> Result<(), Failure> {
            assert!(self.actor.i_damage <= 0);
            assert_eq!(
                self.actor.k_operations.nodes[0].operation_hash,
                HEAL_OPERATION_HASH
            );
            // This synthetic fixture has a source-qualified absent formation
            // and no operation subscribers. Production must supply full Clear.
            self.log.push("clear_operations");
            self.actor.k_operations.nodes.clear();
            if let Some(damage) = self.after_clear_damage {
                self.actor.i_damage = damage;
            }
            self.actor_removed = self.remove_after_clear;
            self.callback_result(HealingStage::ClearOperations)
        }
        fn dispatch_unit_healed(&mut self, signal: UnitHealedSignal) -> Result<(), Failure> {
            self.log.push("healed");
            self.signals.push(signal);
            if let Some(damage) = self.after_signal_damage {
                self.actor.i_damage = damage;
            }
            self.callback_result(HealingStage::HealedSignal)
        }
    }

    #[test]
    fn transition_changes_real_damage_and_reports_dynamic_maximum() {
        let mut c = LiveContext::new(100, true);
        let signal = UnitHealedSignal {
            requested: 79,
            actual: 79,
        };
        assert_eq!(
            heal_unit(Some(&mut c)),
            Ok(UnitHealingOutcome::Signalled(signal))
        );
        assert_eq!(c.actor.i_damage, 21);
        assert_eq!(c.log, ["damage_changed", "healed"]);
        assert_eq!(
            c.reports,
            [DamageChangedReport {
                event_hash: DAMAGE_CHANGED,
                identity: ComponentId {
                    owner: 32767,
                    component_type: 1,
                    id: 77
                },
                damage: 21,
                previous_damage: 100,
                maximum_damage: 300,
            }]
        );
        assert_eq!(c.actor.k_operations.nodes.len(), 1);
    }

    #[test]
    fn full_healing_clears_current_heal_before_signal_even_when_unchanged() {
        for damage in [30, 0] {
            let mut c = LiveContext::new(damage, true);
            let signal = UnitHealedSignal {
                requested: 79,
                actual: damage,
            };
            assert_eq!(
                heal_unit(Some(&mut c)),
                Ok(UnitHealingOutcome::Signalled(signal))
            );
            assert_eq!(c.actor.i_damage, 0);
            assert!(c.actor.k_operations.nodes.is_empty());
            assert_eq!(c.signals, [signal]);
            assert_eq!(c.reports.len(), usize::from(damage != 0));
            assert_eq!(&c.log[c.log.len() - 2..], ["clear_operations", "healed"]);
        }
        let mut c = LiveContext::new(0, false);
        heal_unit(Some(&mut c)).unwrap();
        assert_eq!(c.log, ["healed"]);
        assert_eq!(c.actor.k_operations.nodes.len(), 1);
    }

    #[test]
    fn synchronous_callbacks_control_current_queue_damage_and_actual_amount() {
        let mut c = LiveContext::new(100, false);
        c.after_report = Some((0, true));
        c.after_clear_damage = Some(7);
        c.after_signal_damage = Some(90);
        let signal = UnitHealedSignal {
            requested: 79,
            actual: 93,
        };
        assert_eq!(
            heal_unit(Some(&mut c)),
            Ok(UnitHealingOutcome::Signalled(signal))
        );
        assert_eq!(c.log, ["damage_changed", "clear_operations", "healed"]);
        assert_eq!(c.actor.i_damage, 90);
        assert_eq!(c.signals, [signal]);
        // A report can prevent the clear and can make actual healing negative.
        let mut c = LiveContext::new(30, true);
        c.after_report = Some((40, false));
        heal_unit(Some(&mut c)).unwrap();
        assert_eq!(c.log, ["damage_changed", "healed"]);
        assert_eq!(c.signals[0].actual, -10);
        assert_eq!(c.actor.i_damage, 40);
    }

    #[test]
    fn preflight_and_skipped_healing_preserve_all_actor_state() {
        let mut c = LiveContext::new(30, true);
        c.admitted = false;
        let original = c.actor.clone();
        assert_eq!(
            heal_unit(Some(&mut c)),
            Err(UnitHealingError::EffectsUnavailable(Failure::Unsupported))
        );
        assert_eq!(c.actor, original);
        assert!(c.log.is_empty());
        c.actor.b_eligible_to_heal = false;
        let original = c.actor.clone();
        assert_eq!(heal_unit(Some(&mut c)), Ok(UnitHealingOutcome::Skipped));
        assert_eq!(c.actor, original);
        assert!(c.log.is_empty());
        assert_eq!(
            heal_unit::<LiveContext>(None),
            Ok(UnitHealingOutcome::Skipped)
        );
    }

    #[test]
    fn continuation_failures_retain_writes_and_completed_effects() {
        for stage in [
            HealingStage::DamageChangedReport,
            HealingStage::ClearOperations,
            HealingStage::HealedSignal,
        ] {
            let mut c = LiveContext::new(30, true);
            c.fail_at = Some(stage);
            assert_eq!(
                heal_unit(Some(&mut c)),
                Err(UnitHealingError::ContinuationFailed {
                    stage,
                    error: Failure::Callback
                })
            );
            assert_eq!(c.actor.i_damage, 0);
            assert_eq!(
                c.signals.len(),
                usize::from(stage == HealingStage::HealedSignal)
            );
            assert_eq!(
                c.actor.k_operations.nodes.is_empty(),
                stage != HealingStage::DamageChangedReport
            );
        }
        let mut c = LiveContext::new(30, true);
        c.report_maximum = Err(Failure::Unsupported);
        assert_eq!(
            heal_unit(Some(&mut c)),
            Err(UnitHealingError::ContinuationFailed {
                stage: HealingStage::MaximumDamageQuery,
                error: Failure::Unsupported,
            })
        );
        assert_eq!(c.actor.i_damage, 0);
        assert!(c.log.is_empty());
    }

    #[test]
    fn actor_removal_by_callback_is_a_fallible_reread_without_copyback() {
        for after_report in [true, false] {
            let mut c = LiveContext::new(30, true);
            c.remove_after_report = after_report;
            c.remove_after_clear = !after_report;
            assert_eq!(
                heal_unit(Some(&mut c)),
                Err(UnitHealingError::ContinuationFailed {
                    stage: HealingStage::ActorAccess,
                    error: Failure::Unsupported,
                })
            );
            assert!(c.actor_removed);
            assert_eq!(c.actor.i_damage, 0);
            assert!(c.signals.is_empty());
            assert_eq!(c.actor.k_operations.nodes.is_empty(), !after_report);
        }
    }

    #[test]
    fn setter_uses_global_clamp_and_wrapping_change_damage() {
        let mut c = LiveContext::new(i32::MIN, false);
        heal_unit(Some(&mut c)).unwrap();
        assert_eq!(c.actor.i_damage, 100);
        assert_eq!(c.reports[0].maximum_damage, 300);
        assert_eq!(c.signals[0].actual, i32::MIN.wrapping_sub(100));
        let mut c = LiveContext::new(100, false);
        c.limit = -4;
        heal_unit(Some(&mut c)).unwrap();
        assert_eq!(c.actor.i_damage, -4);
        assert_eq!(c.signals[0].actual, 104);
    }

    #[test]
    fn naval_never_adds_city_governor_or_religion() {
        let (mut u, r) = fixture();
        u.domain = HealingDomain::Naval;
        u.religious_strength = 100;
        for (territory, expected) in [
            (HealingTerritory::Friendly, 40),
            (HealingTerritory::Neutral, 46),
            (HealingTerritory::Enemy, 50),
            (HealingTerritory::Other, 0),
        ] {
            u.territory = territory;
            assert_eq!(healing_amount(Some(&u), &r), expected);
        }
    }

    #[test]
    fn land_city_embark_and_governor_branches() {
        let (mut u, r) = fixture();
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.embarked = true;
        u.plot.as_mut().unwrap().is_city = true;
        assert_eq!(healing_amount(Some(&u), &r), 93);
        u.territory = HealingTerritory::Enemy;
        assert_eq!(healing_amount(Some(&u), &r), 29);
        u.plot.as_mut().unwrap().owned_by_unit_owner = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.embarked = false;
        assert_eq!(healing_amount(Some(&u), &r), 66);
    }

    #[test]
    fn religion_max_not_sum_and_independent_owned_city_bonus() {
        let (mut u, r) = fixture();
        u.religious_strength = 1;
        assert_eq!(healing_amount(Some(&u), &r), 65);
        u.plot.as_mut().unwrap().district_religion_heal_rate = -5;
        assert_eq!(healing_amount(Some(&u), &r), 53);
        u.plot
            .as_mut()
            .unwrap()
            .improvement
            .as_mut()
            .unwrap()
            .pillaged = true;
        assert_eq!(healing_amount(Some(&u), &r), 34);
        u.plot.as_mut().unwrap().owned_by_unit_owner = false;
        assert_eq!(healing_amount(Some(&u), &r), 11);
        u.territory = HealingTerritory::Other;
        assert_eq!(healing_amount(Some(&u), &r), 0);
    }

    #[test]
    fn gates_and_strategic_resource_fallback() {
        let (mut u, r) = fixture();
        assert_eq!(healing_amount(None, &r), 0);
        u.resource = HealingResource::Required {
            accessible_for_healing: false,
            resource_amount: 0,
        };
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.resource = HealingResource::Required {
            accessible_for_healing: true,
            resource_amount: -1,
        };
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.resource = HealingResource::Required {
            accessible_for_healing: false,
            resource_amount: 1,
        };
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.no_foreign_heal = true;
        u.territory = HealingTerritory::Enemy;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Neutral;
        u.no_neutral_heal = true;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Friendly;
        u.delayed_death = true;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.delayed_death = false;
        u.eligible_to_heal = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.eligible_to_heal = true;
        u.plot = None;
        assert_eq!(healing_amount(Some(&u), &r), 0);
    }

    #[test]
    fn religious_territories_and_governor_establishment() {
        let (mut u, r) = fixture();
        u.religious_strength = 1;
        for (territory, expected) in [
            (HealingTerritory::Friendly, 65),
            (HealingTerritory::Neutral, 67),
            (HealingTerritory::Enemy, 71),
            (HealingTerritory::Other, 54),
        ] {
            u.territory = territory;
            assert_eq!(healing_amount(Some(&u), &r), expected);
        }
        u.religious_strength = 0;
        // Unknown territory still reaches the owned-land governor branch.
        assert_eq!(healing_amount(Some(&u), &r), 29);
        u.plot
            .as_mut()
            .unwrap()
            .purchased_by_city
            .as_mut()
            .unwrap()
            .assigned_governor
            .as_mut()
            .unwrap()
            .established = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Neutral;
        assert_eq!(healing_amount(Some(&u), &r), 58);
        u.plot.as_mut().unwrap().purchased_by_city = None;
        assert_eq!(healing_amount(Some(&u), &r), 58);
    }

    #[test]
    fn wrapping_arithmetic_precedes_positive_request_gate() {
        let (mut u, mut r) = fixture();
        u.domain = HealingDomain::Naval;
        r.naval.friendly = i32::MAX;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        r.naval.friendly = i32::MIN;
        u.health.friendly = -1;
        assert_eq!(healing_amount(Some(&u), &r), i32::MAX);
    }
}
