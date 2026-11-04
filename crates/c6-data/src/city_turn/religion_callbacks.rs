//! Concrete first-init religion signal recipients over the current shared
//! requirement owner. Equal religion types do not suppress the signal: the
//! follower-count transition can still invalidate Player requirements.
use super::{Error as CityError, GrowthRules};
use c6_core::{
    city_religion::NO_RELIGION, game_quests::QuestData, loaded_state::LoadedState,
    melee_attack::proposals::subject_sets, simple_requirement,
};

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
struct Invalidation<'a> {
    label: u64,
    value: &'a mut simple_requirement::Requirement,
    tested: i32,
}
impl simple_requirement::Context for Invalidation<'_> {
    type Error = Error;
    fn active(&mut self) -> Result<bool, Error> {
        Ok(self.value.active)
    }
    fn requirement(&mut self) -> Result<&mut simple_requirement::Requirement, Error> {
        Ok(self.value)
    }
    fn changed(&mut self, old: i32, new: i32) -> Result<(), Error> {
        Err(Error::ChangedRequirement {
            label: self.label,
            old,
            new,
        })
    }
}
impl simple_requirement::TestContext for Invalidation<'_> {
    fn test(&mut self) -> Result<i32, Error> {
        Ok(self.tested)
    }
}
fn invalidate(runtime: &mut subject_sets::Runtime, label: u64, tested: i32) -> Result<(), Error> {
    let value = runtime
        .simple_requirement_mut(label)
        .ok_or(Error::MissingRequirement(label))?;
    simple_requirement::invalidate(&mut Invalidation {
        label,
        value,
        tested,
    })
}
/// Execute the actual source-qualified ordered recipients for the first
/// NO_RELIGION(-1)→NO_RELIGION(-1) follower update. Lists/quests are CURRENT
/// owners, including any modeled subscription changes during CityCreated.
/// Requirement changes use the same mutable backend store. A newly reached
/// state-change subscriber remains an error after its canonical state write;
/// it is never replaced by a successful no-op.
#[allow(clippy::too_many_arguments)]
pub fn city_changed(
    state: &LoadedState,
    owner: i32,
    city: u32,
    old: i32,
    new: i32,
    listeners: &[CityListener],
    current_quests: &[QuestData],
    rules: &GrowthRules,
    requirements: &mut subject_sets::Runtime,
) -> Result<(), Error> {
    if old != -1 || new != -1 {
        return Err(Error::Unsupported("positive religion-change recipients"));
    }
    let city = state.city(owner, city)?;
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
                if city.header.capital
                    && current_quests
                        .iter()
                        .any(|q| q.type_hash == quest_type && q.client_ordinal == owner)
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
                    invalidate(requirements, requirement, 1)?;
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
                invalidate(requirements, requirement, 1)?;
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_invalidation_preserves_equal_false_state_and_inactive_guard() {
        let mut value = simple_requirement::Requirement {
            active: true,
            state: 1,
            inverse: false,
            persistent: false,
            triggered: false,
        };
        simple_requirement::invalidate(&mut Invalidation {
            label: 7,
            value: &mut value,
            tested: 1,
        })
        .unwrap();
        assert_eq!(value.state, 1);
        value.active = false;
        simple_requirement::invalidate(&mut Invalidation {
            label: 7,
            value: &mut value,
            tested: 0,
        })
        .unwrap();
        assert_eq!(value.state, 1);
    }
    #[test]
    fn repeated_triggered_state_requires_real_subscriber_continuation() {
        let mut value = simple_requirement::Requirement {
            active: true,
            state: 4,
            inverse: true,
            persistent: false,
            triggered: true,
        };
        assert!(matches!(
            simple_requirement::invalidate(&mut Invalidation {
                label: 9,
                value: &mut value,
                tested: 1
            }),
            Err(Error::ChangedRequirement {
                label: 9,
                old: 4,
                new: 4
            })
        ));
        assert_eq!(value.state, 4);
    }
}
