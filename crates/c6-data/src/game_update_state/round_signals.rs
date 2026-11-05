//! Exact source-ordered stock GameSignals turn subscribers (+708/+750).
//! Requirement invalidations borrow the ongoing turn's SAME live store/backend.
use super::activation::{cityless_managers, requirement_changes};
use c6_core::{
    core_player::initialization::WireBinding, game_update::Error, loaded_state::LoadedState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmergencyScore {
    ReligiousGoal,
    Favor,
    AtWar,
    BadCo2,
    MilitaryInEnemyTerritory,
    ReligionInEnemyTerritory,
    District,
    Co2,
    Building,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummaryProvider {
    General,
    Expansion1,
    Expansion2,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Callback {
    SummaryStatistics,
    Summary(SummaryProvider),
    Script,
    Emergency(EmergencyScore),
    Invalidate(u64),
    DiplomacyEnd,
    Achievements,
}
/// Source-qualified stock subscriptions only; added/custom callbacks require
/// actual implementations and cannot be admitted through a presentation enum.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNoAddedConsumers,
}
pub struct Bindings {
    binding: WireBinding,
    begin: Vec<Callback>,
    end: Vec<Callback>,
}
impl Bindings {
    pub fn from_initial(
        state: &LoadedState,
        begin: Vec<Callback>,
        end: Vec<Callback>,
        _: Scope,
    ) -> Self {
        Self {
            binding: state.binding().clone(),
            begin,
            end,
        }
    }
    pub fn callbacks(&self, begin: bool) -> &[Callback] {
        if begin {
            &self.begin
        } else {
            &self.end
        }
    }
}
/// Concrete services execute current predicates/modifiers and the persistent
/// script registry. There are no successful default continuation methods.
pub trait Effects: requirement_changes::Services {
    fn script_turn(
        &mut self,
        state: &LoadedState,
        event: &str,
        arguments: &[i32],
    ) -> Result<(), Error>;
}
fn emergency_empty(
    state: &LoadedState,
    managers: &cityless_managers::Runtime,
    kind: EmergencyScore,
) -> Result<(), Error> {
    // CheckNewReligiousGoal2931930 only applies to matching existing instances.
    // Score callbacks read players/definitions/counts and then either iterate
    // the same instances themselves or call ChangePlayerScore2931ee0. Neither
    // those read-only preambles nor the empty instance loops draw RNG. There
    // is no generation/timer/state write before the instance-dependent body.
    if !managers.congress(state)?.emergencies.is_empty() {
        return Err(Error::Unsupported(match kind {
            EmergencyScore::ReligiousGoal => "turn Emergency religious-goal continuation",
            _ => "turn Emergency scoring continuation",
        }));
    }
    Ok(())
}
fn presentation(callback: Callback) -> bool {
    // GatherPerTurnPlayerStatistics2a23f30 writes only summary datasets.
    // Exact provider dispatch is admitted separately: General2643710/2643840
    // (including CollectCityTurnData), XP1 literal RET2dcb9f0, XP22dd1800.
    // They read gameplay getters and write Repository datapoints/string pools;
    // there is no gameplay mutation, dispatch, or reachable RNG in this scope.
    // Achievements.OnGameTurnStarted28fba50 similarly only tests read-only
    // gameplay predicates and sends UnlockAchievement28f1f60 to the platform
    // achievement interface; headless omits that platform presentation output.
    matches!(
        callback,
        Callback::SummaryStatistics | Callback::Summary(_) | Callback::Achievements
    )
}

/// Called at the parent's actual synchronous signal stage. Any failure leaves
/// completed earlier callbacks and current requirement/modifier writes intact.
/// `turn` is the source signal argument, not a snapshot substituted for it.
pub fn run<E: Effects>(
    state: &mut LoadedState,
    bindings: &Bindings,
    begin: bool,
    turn: i32,
    managers: &cityless_managers::Runtime,
    requirements: &mut requirement_changes::Runtime,
    effects: &mut E,
) -> Result<(), Error>
where
    E::Error: std::fmt::Debug,
{
    if bindings.binding != *state.binding() || requirements.binding() != state.binding() {
        return Err(Error::Unsupported("turn signal current binding"));
    }
    for &callback in bindings.callbacks(begin) {
        match callback {
            Callback::Script => effects.script_turn(
                state,
                if begin {
                    "OnGameTurnStarted"
                } else {
                    "OnGameTurnEnded"
                },
                &[turn],
            )?,
            Callback::Emergency(kind) => emergency_empty(state, managers, kind)?,
            Callback::Invalidate(receiver) => requirements
                .invalidate(state, effects, receiver)
                .map_err(|_| Error::Unsupported("turn signal requirement invalidation"))?,
            Callback::DiplomacyEnd => {
                // OnGameTurnEnded2914bd0 returns before logging/CloseAllOpenSessions
                // exactly when the SAME current native session list is empty.
                if !state
                    .diplomacy_manager()
                    .map_err(|_| Error::Unsupported("turn diplomacy owner"))?
                    .open_sessions()
                    .is_empty()
                {
                    return Err(Error::Unsupported(
                        "turn diplomacy close-open-sessions continuation",
                    ));
                }
            }
            other if presentation(other) => {}
            _ => return Err(Error::Unsupported("turn signal callback")),
        }
    }
    Ok(())
}
