//! Linux XP2 Game.DiplomaticRelations.DoTurn29826a0.
//!
//! Executes the source guards against the canonical current relation owner.
//! The supported branch has no allied alive-major pair, reached friendship timer,
//! research-alliance boost, or peaceful met ever-major pair. Positive effects
//! remain explicit continuations, never silently fulfilled by native policy.
use crate::{
    game_parameters::sql_integer,
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    unit_placement_services::Runtime,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, diplomacy_meeting::MAJOR_LEVEL,
    game_diplomatic_relations::GameDiplomaticRelations, loaded_state::LoadedState,
};

#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    Requires(Transition),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    AlliancePoints { first: i32, second: i32 },
    FriendshipTimer { first: i32, second: i32 },
    ResearchAlliance { first: i32, second: i32 },
    GrievanceDecay { first: i32, second: i32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round diplomatic relations: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    binding: WireBinding,
    speeds: LinuxDefinitionRegistry,
    durations: Vec<(i32, i32)>,
    unscaled: (i32, i32),
    research_alliance: Option<i32>,
}
// Economic.AdjustForGameSpeed(int,ScalingType)2be3a70 uses the integer
// ScaleByPercentageClamped overload, not the Q8 overload used by other upkeep.
fn scale(value: i32, percentage: i32) -> i32 {
    (i64::from(value) * i64::from(percentage) / 100).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
        as i32
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let missing = |s| Error::Missing(s);
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(missing("GlobalParameters"))?;
        let parameter = |name| {
            parameters
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(sql_integer)
                .ok_or(missing("diplomacy timer parameter"))
        };
        let friendship = parameter("DIPLOMACY_DECLARED_FRIENDSHIP_TIME_LIMIT")?;
        let alliance = parameter("DIPLOMACY_ALLIANCE_TIME_LIMIT")?;
        let speeds =
            collection(rules, "GameSpeeds", "GameSpeedType").map_err(Error::Definitions)?;
        let alliances =
            collection(rules, "Alliances", "AllianceType").map_err(Error::Definitions)?;
        let speed_table = rules
            .gameplay()
            .table("GameSpeeds")
            .ok_or(missing("GameSpeeds"))?;
        let scalings = rules
            .gameplay()
            .table("GameSpeed_Scalings")
            .ok_or(missing("GameSpeed_Scalings"))?;
        let durations_table = rules
            .gameplay()
            .table("GameSpeed_Durations")
            .ok_or(missing("GameSpeed_Durations"))?;
        let mut durations = Vec::new();
        for speed in speeds.entries() {
            let row = speed_table
                .row_by_id(speed.database_rowid)
                .ok_or(missing("game speed row"))?;
            let name = row
                .text("GameSpeedType")
                .map_err(|_| missing("GameSpeedType"))?;
            let cost = row
                .get("CostMultiplier")
                .ok()
                .and_then(sql_integer)
                .ok_or(missing("speed CostMultiplier"))?;
            let scaling = scalings.rows().find(|r| {
                r.text("GameSpeedType").ok() == Some(name)
                    && r.text("ScalingType")
                        .ok()
                        .is_some_and(|s| linux_type_hash(s.as_bytes()) == 0xafc894aa)
            });
            let adjust = |value| -> Result<i32, Error> {
                let mut result = 0;
                if let Some(row) = scaling {
                    let key = row
                        .text("GameSpeedScalingType")
                        .map_err(|_| missing("GameSpeedScalingType"))?;
                    if let Some(duration) = durations_table.rows().find(|r| {
                        r.text("GameSpeedScalingType").ok() == Some(key)
                            && r.get("NumberOfTurnsOnStandard").ok().and_then(sql_integer)
                                == Some(value)
                    }) {
                        result = duration
                            .get("NumberOfTurnsScaled")
                            .ok()
                            .and_then(sql_integer)
                            .ok_or(missing("NumberOfTurnsScaled"))?;
                    } else {
                        result = scale(
                            value,
                            row.get("DefaultCostMultiplier")
                                .ok()
                                .and_then(sql_integer)
                                .ok_or(missing("DefaultCostMultiplier"))?,
                        );
                    }
                }
                // A zero specific result falls through to speed CostMultiplier.
                Ok(if result == 0 {
                    scale(value, cost)
                } else {
                    result
                })
            };
            durations.push((adjust(friendship)?, adjust(alliance)?));
        }
        Ok(Self {
            binding: state.binding().clone(),
            speeds,
            durations,
            unscaled: (friendship, alliance),
            research_alliance: alliances.find_index(0xd22c5d65).map(|i| i as i32),
        })
    }
    fn durations(&self, speed: u32) -> (i32, i32) {
        self.speeds
            .find_index(speed)
            .map_or(self.unscaled, |i| self.durations[i])
    }
}

fn evaluate(
    relations: &GameDiplomaticRelations,
    alive: &[i32],
    ever: &[i32],
    turn: i32,
    friendship_duration: i32,
    research_alliance: Option<i32>,
) -> Result<(), Error> {
    // Native nested loops compare player IDs, rather than list positions.
    for &first in alive {
        for &second in alive {
            if first < second
                && relations
                    .is_allied(first, second)
                    .map_err(Error::Relations)?
            {
                return Err(Error::Requires(Transition::AlliancePoints {
                    first,
                    second,
                }));
            }
        }
    }
    // This guard skips the entire friendship/alliance/research block, including
    // research boost scheduling. Equality is skipped; subtraction wraps in i32.
    if turn.wrapping_sub(friendship_duration) > 0 {
        for &first in alive {
            for &second in alive {
                if !relations
                    .is_allied(first, second)
                    .map_err(Error::Relations)?
                    && !relations
                        .is_permanently_allied(first, second)
                        .map_err(Error::Relations)?
                    && relations
                        .has_declared_friendship(first, second)
                        .map_err(Error::Relations)?
                {
                    return Err(Error::Requires(Transition::FriendshipTimer {
                        first,
                        second,
                    }));
                }
                // Any allied pair was already refused before this stage. The
                // ResearchAlliance lookup is still a definition-qualified guard.
                if let Some(research) = research_alliance {
                    if relations
                        .alliance_type(first, second)
                        .map_err(Error::Relations)?
                        == research
                        && first < second
                    {
                        return Err(Error::Requires(Transition::ResearchAlliance {
                            first,
                            second,
                        }));
                    }
                }
            }
        }
    }
    for &first in ever {
        for &second in ever {
            if first < second
                && !relations
                    .is_at_war(first, second)
                    .map_err(Error::Relations)?
                && relations.has_met(first, second).map_err(Error::Relations)?
            {
                return Err(Error::Requires(Transition::GrievanceDecay {
                    first,
                    second,
                }));
            }
        }
    }
    Ok(())
}

/// Uses SAME current Player.Manager alive list, canonical diplomacy-manager
/// ever-alive list, and Game.DiplomaticRelations matrices. No cloned relation
/// world, stale initial contacts, RNG draw, or presentation callback is used.
/// Reached positive transitions refuse before their first mutation.
pub fn run(
    state: &LoadedState,
    players: &Runtime,
    definitions: &Definitions,
    game_speed: u32,
) -> Result<(), Error> {
    if players.binding != *state.binding() || definitions.binding != *state.binding() {
        return Err(Error::Missing("DiplomaticRelations current binding"));
    }
    let major = |owner| -> Result<bool, Error> {
        let level = players
            .player::<()>(owner)
            .map_err(|_| Error::Missing("current Player"))?
            .civilization_level
            .ok_or(Error::Missing("current civilization level"))?;
        Ok(level == MAJOR_LEVEL)
    };
    let (list, count) = players
        .alive_begin::<()>()
        .map_err(|_| Error::Missing("current GetAlive"))?;
    let mut alive = Vec::new();
    for i in 0..count {
        let owner = players
            .alive_at::<()>(list, i)
            .map_err(|_| Error::Missing("GetAlive order"))?;
        if major(owner)? {
            alive.push(owner);
        }
    }
    let mut ever = Vec::new();
    for handle in state
        .diplomacy_manager()
        .map_err(Error::State)?
        .ever_alive()
    {
        if major(handle.owner)? {
            ever.push(handle.owner);
        }
    }
    // Both durations are computed by the native routine even on early turns.
    let (friendship_duration, _alliance_duration) = definitions.durations(game_speed);
    evaluate(
        state.game_relations().map_err(Error::State)?,
        &alive,
        &ever,
        state.retained().core.current_turn,
        friendship_duration,
        definitions.research_alliance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::game_diplomatic_relations::BoolRelationship;
    fn relations(war: i32, met: bool, friendship: bool) -> GameDiplomaticRelations {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, 0)],
            vec![(-1, 0)],
        )
        .unwrap();
        r.initialize_current_war(vec![(war, 0)]).unwrap();
        r.initialize_current_contacts(
            2,
            vec![BoolRelationship {
                value: met,
                changed_turn: 0,
            }],
        )
        .unwrap();
        r.initialize_current_friendships(vec![BoolRelationship {
            value: friendship,
            changed_turn: 0,
        }])
        .unwrap();
        r.initialize_current_permanent_alliances(vec![BoolRelationship {
            value: false,
            changed_turn: 0,
        }])
        .unwrap();
        r
    }
    #[test]
    fn source_guards_do_not_hide_reached_relationship_effects() {
        let r = relations(-1, false, true);
        assert!(evaluate(&r, &[1, 0], &[1, 0], 30, 30, Some(0)).is_ok());
        assert!(matches!(
            evaluate(&r, &[1, 0], &[1, 0], 31, 30, Some(0)),
            Err(Error::Requires(Transition::FriendshipTimer {
                first: 1,
                second: 0
            }))
        ));
        assert!(matches!(
            evaluate(&relations(-1, true, false), &[0, 1], &[0, 1], 2, 30, None),
            Err(Error::Requires(Transition::GrievanceDecay {
                first: 0,
                second: 1
            }))
        ));
        assert!(evaluate(&relations(1, true, false), &[0, 1], &[0, 1], 2, 30, None).is_ok());
        assert!(evaluate(&relations(-1, false, false), &[0, 1], &[0, 1], 2, 30, None).is_ok());
    }
    #[test]
    fn duration_scaling_uses_integer_clamp_and_truncation() {
        assert_eq!(scale(-1, 50), 0);
        assert_eq!(scale(30, 150), 45);
        assert_eq!(scale(i32::MAX, 200), i32::MAX);
    }
}
