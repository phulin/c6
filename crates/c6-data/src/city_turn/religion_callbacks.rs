//! Concrete first-init religion signal recipients over the current shared
//! requirement owner. Equal religion types do not suppress the signal: the
//! follower-count transition can still invalidate Player requirements.
use super::{Error as CityError, GrowthRules};
use c6_core::{
    city_religion::NO_RELIGION, loaded_state::LoadedState, melee_attack::proposals::subject_sets,
    simple_requirement,
};

/// Current native QuestData fields queried by this listener, distinct from
/// the saved-wire definition hash and from a complete quest-completion owner.
#[derive(Clone, Debug)]
pub struct CurrentQuest {
    pub quest_type_index: u32,
    pub client: i32,
    pub religion_parameter: i32,
}
#[derive(Clone, Debug)]
pub enum CityListener {
    Gossip,
    Emergency,
    Eras,
    History,
    QuestConvertCapital {
        quest_type: u32,
    },
    Boost,
    GrandStrategic,
    PlayerFoundedOurReligion {
        requirement: u64,
        context_player: Option<i32>,
        subject_player: Option<i32>,
    },
    PlayerMajorityReligionIsOwner {
        requirement: u64,
        subject_player: Option<i32>,
    },
    Achievements,
}
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    Missing(&'static str),
    MissingRequirement(u64),
    ChangedRequirement { label: u64, old: i32, new: i32 },
    Unsupported(&'static str),
    RequirementContinuation(String),
    BoostContinuation(String),
}
impl From<c6_core::loaded_state::Error> for Error {
    fn from(e: c6_core::loaded_state::Error) -> Self {
        Self::State(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city religion signal: {self:?}")
    }
}
impl std::error::Error for Error {}

fn created_religion(state: &LoadedState, owner: i32, rules: &GrowthRules) -> Result<i32, Error> {
    let Some(row) = state
        .retained()
        .religion
        .records
        .iter()
        .find(|r| r.founder == owner)
    else {
        return Ok(-1);
    };
    if row.religion_hash == NO_RELIGION {
        return Ok(-1);
    }
    rules
        .religions
        .find_index(row.religion_hash)
        .map(|i| i as i32)
        .ok_or(Error::Missing("current founded Religion definition"))
}
fn player_exists(state: &LoadedState, owner: i32) -> bool {
    state
        .retained()
        .players
        .iter()
        .any(|p| p.header.owner == owner)
}
/// Synchronous continuation over the same canonical requirement/set store.
/// The callback runs after the SimpleRequirement write and before the next
/// signal recipient; it must dispatch actual set and modifier subscriptions.
pub trait RequirementEffects {
    fn changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut subject_sets::Runtime,
        label: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Error>;
}
struct Invalidation<'a, E> {
    label: u64,
    runtime: &'a mut subject_sets::Runtime,
    changed: E,
    tested: i32,
}
impl<E> simple_requirement::Context for Invalidation<'_, E>
where
    E: FnMut(&mut subject_sets::Runtime, u64, i32, i32) -> Result<(), Error>,
{
    type Error = Error;
    fn active(&mut self) -> Result<bool, Error> {
        Ok(self
            .runtime
            .simple_requirement(self.label)
            .ok_or(Error::MissingRequirement(self.label))?
            .active)
    }
    fn requirement(&mut self) -> Result<&mut simple_requirement::Requirement, Error> {
        self.runtime
            .simple_requirement_mut(self.label)
            .ok_or(Error::MissingRequirement(self.label))
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Error> {
        (self.changed)(self.runtime, self.label, old, new)
    }
}
impl<E> simple_requirement::TestContext for Invalidation<'_, E>
where
    E: FnMut(&mut subject_sets::Runtime, u64, i32, i32) -> Result<(), Error>,
{
    fn test(&mut self) -> Result<i32, Error> {
        Ok(self.tested)
    }
}
fn invalidate<E: RequirementEffects>(
    state: &mut LoadedState,
    runtime: &mut subject_sets::Runtime,
    effects: &mut E,
    label: u64,
    tested: i32,
) -> Result<(), Error> {
    simple_requirement::invalidate(&mut Invalidation {
        label,
        runtime,
        tested,
        changed: |runtime: &mut subject_sets::Runtime, label, old, new| {
            effects.changed(state, runtime, label, old, new)
        },
    })
}
/// Execute the actual source-qualified ordered recipients for the first
/// NO_RELIGION(-1)→NO_RELIGION(-1) follower update. Lists/quests are CURRENT
/// owners, including any modeled subscription changes during CityCreated.
/// Requirement changes dispatch synchronously over the same mutable backend
/// store. Unsupported subscribers preserve their completed mutation prefix.
#[allow(clippy::too_many_arguments)]
pub fn city_changed<E: RequirementEffects>(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    old: i32,
    new: i32,
    listeners: &[CityListener],
    current_quests: &[CurrentQuest],
    rules: &GrowthRules,
    requirements: &mut subject_sets::Runtime,
    effects: &mut E,
) -> Result<(), Error> {
    if old != -1 || new != -1 {
        return Err(Error::Unsupported("positive religion-change recipients"));
    }
    let capital = state.city(owner, city)?.header.capital;
    for listener in listeners {
        match *listener {
            // Source guards: Gossip/Emergency reject new==-1; Eras/Boost get
            // no Game.Religion record at -1. History/Achievements compare types.
            // GrandStrategic compares types after pure configuration/favored
            // religion reads, before its gameplay analysis/signal branch.
            CityListener::Gossip
            | CityListener::Emergency
            | CityListener::Eras
            | CityListener::History
            | CityListener::Boost
            | CityListener::GrandStrategic
            | CityListener::Achievements => {}
            CityListener::QuestConvertCapital { quest_type } => {
                if capital
                    && current_quests.iter().any(|q| {
                        q.quest_type_index == quest_type
                            && q.client == owner
                            && q.religion_parameter == new
                    })
                {
                    return Err(Error::Unsupported(
                        "current matching quest parameters and completion",
                    ));
                }
            }
            CityListener::PlayerFoundedOurReligion {
                requirement,
                context_player,
                subject_player,
            } => {
                let (Some(context), Some(subject)) = (context_player, subject_player) else {
                    continue;
                };
                if context != owner
                    || !player_exists(state, context)
                    || !player_exists(state, subject)
                {
                    continue;
                }
                let founded = created_religion(state, subject, rules)?;
                if founded == old || founded == new {
                    // Source237f060 returns state1 when subject founded none.
                    invalidate(state, requirements, effects, requirement, 1)?;
                }
            }
            CityListener::PlayerMajorityReligionIsOwner {
                requirement,
                subject_player,
            } => {
                if subject_player != Some(owner) || !player_exists(state, owner) {
                    continue;
                }
                let player = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("current religion Player"))?;
                if player
                    .cities
                    .records
                    .entries
                    .iter()
                    .any(|c| c.value.religion.majority_religion_hash != NO_RELIGION)
                {
                    return Err(Error::Unsupported(
                        "current positive majority religion predicate",
                    ));
                }
                // All current cities follow none, so GetReligionInMajorityOfCities
                // has no real religion and Test2387530 returns state1.
                invalidate(state, requirements, effects, requirement, 1)?;
            }
        }
    }
    Ok(())
}
/// The one current PlayerReligionChanged recipient is the stock Achievement
/// callback. With new==-1 it returns after pure mode/unlock queries, before any
/// award/check branch. Caller still qualifies its actual ordered subscriber.
pub fn player_changed_no_religion(new: i32) -> Result<(), Error> {
    if new == -1 {
        Ok(())
    } else {
        Err(Error::Unsupported(
            "positive player religion achievement recipients",
        ))
    }
}
impl From<Error> for CityError {
    fn from(error: Error) -> Self {
        CityError::ReligionCallback(error)
    }
}
/// Actual ordered PopulationChanged subscribers. The Lua recipient is executed
/// by the caller's existing semantic ScriptState, including event creation.
#[derive(Clone, Debug)]
pub enum PopulationListener {
    Lua,
    History,
    Boost {
        population_of_player: bool,
        minimum: i32,
        /// Current Boost definition collection ordinal, retained from registration.
        definition: usize,
    },
    GrandStrategic {
        owner: i32,
        settlement_decay_amount: i32,
    },
    PlayerPopulationLead {
        requirement: u64,
        context_player: i32,
        subject_player: i32,
        minimum_lead: i32,
        percent_bits: u32,
    },
    Achievements,
}
pub trait PopulationScript {
    fn trigger_population_boost(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: usize,
    ) -> Result<(), Error> {
        Err(Error::Unsupported("population boost continuation"))
    }

    fn on_city_population_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        delta: i32,
    ) -> Result<(), Error>;
}
fn population(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current population Player"))?;
    Ok(player
        .cities
        .records
        .entries
        .iter()
        .fold(0i32, |sum, entry| {
            sum.wrapping_add(entry.value.header.population)
        }))
}
/// 23c2e00 uses wrapping signed subtraction/absolute value, then f32 products.
/// Return values are the native SimpleRequirement states (0 satisfied, 1 failed).
pub fn population_lead_test(context: i32, subject: i32, minimum: i32, percent: f32) -> i32 {
    let difference = context.wrapping_sub(subject);
    let absolute = if difference < 0 {
        difference.wrapping_neg()
    } else {
        difference
    };
    if absolute < minimum {
        return 1;
    }
    let satisfied = if percent > 0.0 {
        subject as f32 >= percent * context as f32
    } else {
        context as f32 >= -(percent * subject as f32)
    };
    i32::from(!satisfied)
}
/// The signed threshold check in Linux Boost.Manager.OnCityPopulationChanged
/// (28445a0). Delta is ignored; value29 substitutes current player population.
/// The returned ordinal identifies the captured definition, not the target civic.
pub fn population_boost(
    state: &LoadedState,
    owner: i32,
    city: u32,
    listener: &PopulationListener,
) -> Result<Option<usize>, Error> {
    let PopulationListener::Boost {
        population_of_player,
        minimum,
        definition,
    } = *listener
    else {
        return Ok(None);
    };
    let amount = if population_of_player {
        population(state, owner)?
    } else {
        state.city(owner, city)?.header.population
    };
    Ok((amount >= minimum).then_some(definition))
}
/// Executes the current founding/growth population listeners against the same
/// canonical City and shared requirement store. Positive boosts dispatch through
/// the caller's canonical research owner before the next registered recipient.
/// Reached positive history/achievement paths remain explicit.
#[allow(clippy::too_many_arguments)]
pub fn population_changed<S: PopulationScript, E: RequirementEffects>(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    delta: i32,
    listeners: &[PopulationListener],
    requirements: &mut subject_sets::Runtime,
    script: &mut S,
    effects: &mut E,
) -> Result<(), Error> {
    for listener in listeners {
        let current = state.city(owner, city)?.header.population;
        match *listener {
            PopulationListener::Lua => {
                script.on_city_population_changed(state, owner, city, delta)?
            }
            PopulationListener::History => {
                let old = current.wrapping_sub(delta);
                if [25, 20, 15, 10]
                    .iter()
                    .any(|&threshold| current >= threshold && old < threshold)
                {
                    return Err(Error::Unsupported("population historic moment threshold"));
                }
            }
            PopulationListener::Boost { .. } => {
                if let Some(definition) = population_boost(state, owner, city, listener)? {
                    script.trigger_population_boost(state, owner, definition)?;
                }
            }
            PopulationListener::GrandStrategic {
                owner: subscriber,
                settlement_decay_amount,
            } => {
                if subscriber == owner {
                    let value = state
                        .player_ai_grand_strategic_mut(owner)?
                        .as_mut()
                        .ok_or(Error::Missing("current GrandStrategic city planner"))?;
                    value.city_planner.expansion_score = value
                        .city_planner
                        .expansion_score
                        .wrapping_add(settlement_decay_amount.wrapping_mul(2).wrapping_neg());
                }
            }
            PopulationListener::PlayerPopulationLead {
                requirement,
                context_player,
                subject_player,
                minimum_lead,
                percent_bits,
            } => {
                let tested = population_lead_test(
                    population(state, context_player)?,
                    population(state, subject_player)?,
                    minimum_lead,
                    f32::from_bits(percent_bits),
                );
                invalidate(state, requirements, effects, requirement, tested)?;
            }
            PopulationListener::Achievements => {
                if current >= 30 {
                    return Err(Error::Unsupported("population achievement eligibility"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn current(value: simple_requirement::Requirement) -> subject_sets::Runtime {
        let mut runtime = subject_sets::Runtime::from_initial(1, vec![], vec![]).unwrap();
        runtime
            .import_current::<()>(
                vec![subject_sets::changes::CurrentRequirement {
                    label: 7,
                    native_id: 2,
                    context: 0,
                    subject: subject_sets::Subject::ModifierObject(3),
                    definition: subject_sets::RequirementDefinition {
                        id: b"population".to_vec(),
                        predicate: subject_sets::Predicate::Unsupported(0),
                        inverse: value.inverse,
                        persistent: value.persistent,
                    },
                    value: subject_sets::InitializedRequirement::Simple(value),
                    references: 1,
                    subscribers: vec![],
                }],
                vec![],
            )
            .unwrap();
        runtime
    }
    fn value() -> simple_requirement::Requirement {
        simple_requirement::Requirement {
            active: true,
            state: 1,
            inverse: false,
            persistent: false,
            triggered: false,
        }
    }
    #[test]
    fn change_dispatch_reads_canonical_write_and_preserves_failure_prefix() {
        let mut runtime = current(value());
        let mut called = false;
        let result = simple_requirement::invalidate(&mut Invalidation {
            label: 7,
            runtime: &mut runtime,
            tested: 0,
            changed: |runtime: &mut subject_sets::Runtime, label, old, new| {
                assert_eq!((label, old, new), (7, 1, 0));
                assert_eq!(runtime.simple_requirement(label).unwrap().state, 0);
                called = true;
                Err(Error::Unsupported("actual reached subscriber"))
            },
        });
        assert!(called);
        assert!(matches!(
            result,
            Err(Error::Unsupported("actual reached subscriber"))
        ));
        assert_eq!(runtime.requirement_state(7), Some(0));
    }
    #[test]
    fn equal_and_inactive_requirements_do_not_dispatch_but_trigger_four_does() {
        let mut runtime = current(value());
        for tested in [1, 0] {
            if tested == 0 {
                runtime.simple_requirement_mut(7).unwrap().active = false;
            }
            simple_requirement::invalidate(&mut Invalidation {
                label: 7,
                runtime: &mut runtime,
                tested,
                changed: |_: &mut subject_sets::Runtime, _, _, _| panic!("unexpected change"),
            })
            .unwrap();
        }
        let r = runtime.simple_requirement_mut(7).unwrap();
        r.active = true;
        r.state = 4;
        r.inverse = true;
        r.triggered = true;
        let mut count = 0;
        simple_requirement::invalidate(&mut Invalidation {
            label: 7,
            runtime: &mut runtime,
            tested: 1,
            changed: |runtime: &mut subject_sets::Runtime, label, old, new| {
                assert_eq!((old, new), (4, 4));
                assert_eq!(runtime.requirement_state(label), Some(4));
                count += 1;
                Ok(())
            },
        })
        .unwrap();
        assert_eq!(count, 1);
    }
    #[test]
    fn population_lead_preserves_minimum_float_sign_and_wrapping_edges() {
        assert_eq!(population_lead_test(1, 0, 3, 1.1), 1);
        assert_eq!(population_lead_test(1, 1, 3, -1.1), 1);
        assert_eq!(population_lead_test(10, 13, 3, 1.1), 0);
        assert_eq!(population_lead_test(10, 7, 3, 1.1), 1);
        assert_eq!(population_lead_test(13, 10, 3, -1.1), 0);
        assert_eq!(population_lead_test(i32::MIN, 0, 0, 1.1), 1);
        assert_eq!(population_lead_test(10, 13, 0, f32::NAN), 1);
    }
}
