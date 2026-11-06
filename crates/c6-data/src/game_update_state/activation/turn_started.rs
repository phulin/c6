//! Ordered first-activation PlayerTurnStarted signal on persistent stock state.
//! Inputs describe initialized listeners, never native predicate answers. The
//! same script, requirement and modifier owners are borrowed throughout.
use super::{
    relationship_requirements as relationship, requirement_changes as changes,
    victory_requirements as victory,
};
use crate::loaded_actions::{
    stock_notifications::{self as stock, scoring},
    stock_script,
};
use c6_core::{loaded_state::LoadedState, melee_attack::proposals::subject_sets};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockHeadlessExternalPlayerPolicies,
}
#[derive(Clone, Copy, Debug)]
pub enum Listener {
    LuaPlayerTurnStarted,
    Gossip {
        pending_count: Option<usize>,
    },
    Emergency {
        signals_count: Option<usize>,
    },
    /// Const gameplay reads published only as telemetry; no callback/RNG leaf.
    AnalysisTelemetry,
    Diplomacy {
        owner: i32,
    },
    Relationship {
        id: u64,
        receiver: relationship::Receiver,
    },
    Victory {
        id: u64,
        receiver: victory::Receiver,
    },
    Unsupported {
        identity: u64,
    },
}
pub struct VictoryInputs<'a> {
    pub definitions: &'a victory::Definitions,
    pub current: &'a victory::Inputs,
    pub scoring: &'a scoring::Runtime,
    pub scoring_definitions: &'a scoring::Definitions,
}
pub struct Inputs<'a> {
    pub scope: Scope,
    pub listeners: &'a [Listener],
    pub relationship_definitions: &'a relationship::Definitions,
    pub alive_majors: &'a [i32],
    pub victory: Option<VictoryInputs<'a>>,
}
#[derive(Debug)]
pub enum Continuation {
    Predicate(u64),
    Modifier {
        listener: changes::SetListener,
        set: u64,
        old: i32,
        new: i32,
    },
    DestroyRequirement(u64),
    DestroySet(u64),
    TradeRelations(&'static str),
}
#[derive(Debug)]
pub enum Cause {
    Script(stock_script::Error),
    GossipPending(Option<usize>),
    EmergencyPending(Option<usize>),
    Diplomacy(c6_core::game_update::Error),
    Relationship(relationship::DispatchError<Continuation>),
    Victory(victory::Error),
    Change(changes::Error<Continuation>),
    MissingRequirement(u64),
    MissingVictoryInputs,
    Unsupported(u64),
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Listener { index: usize, cause: Box<Cause> },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "turn-start activation: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Progress {
    pub completed: usize,
    pub skipped_unsubscribed: usize,
}
struct StockServices<'s, 'a, P> {
    backend: &'s mut stock::Runtime<'a, P>,
}
impl<P> changes::Services for StockServices<'_, '_, P> {
    type Error = Continuation;
    fn test_requirement(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        id: u64,
    ) -> Result<i32, Continuation> {
        Err(Continuation::Predicate(id))
    }
    fn modifier_changed(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        listener: changes::SetListener,
        set: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Continuation> {
        self.backend
            .reached
            .push("turn-start reached requirement-set continuation");
        Err(Continuation::Modifier {
            listener,
            set,
            old,
            new,
        })
    }
    fn trade_relations_changed(
        &mut self,
        state: &mut LoadedState,
        sets: &mut subject_sets::Runtime,
        row: &mut changes::OwnerModifier,
    ) -> Result<bool, Continuation> {
        self.backend
            .with_growth_requirement_effects(state, |_, state, canonical| {
                let result = canonical.with_opinion_graph(|graph, canonical| {
                    graph.owner_changed_current(state, sets, canonical, row.receiver)
                });
                // This row is comparison metadata; canonical graph controls own the
                // transition, including writes made before an unsupported callback.
                if let Some(modifier) = canonical
                    .modifiers()
                    .iter()
                    .find(|item| item.label == row.receiver)
                {
                    row.enabled = modifier.control.enabled;
                }
                if let Some(metadata) = canonical
                    .opinion_graph()
                    .and_then(|graph| graph.rows().iter().find(|item| item.label == row.receiver))
                {
                    row.has_activated = metadata.has_activated;
                    row.current_update_instance = Some(metadata.effect.current_update_instance);
                }
                result
            })
            .map_err(Continuation::TradeRelations)
    }
    fn destroy_requirement(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        id: u64,
    ) -> Result<(), Continuation> {
        Err(Continuation::DestroyRequirement(id))
    }
    fn destroy_set(
        &mut self,
        _: &mut LoadedState,
        _: &mut subject_sets::Runtime,
        id: u64,
    ) -> Result<(), Continuation> {
        Err(Continuation::DestroySet(id))
    }
}
/// A preceding callback may remove a later requirement's final shared owner.
/// Known released records are skipped; an absent initial record is an error.
fn removed(runtime: &changes::Runtime, id: u64) -> Result<bool, Cause> {
    if runtime.state(id).is_some() {
        Ok(false)
    } else if runtime.sets().requirement_state(id).is_some() {
        Err(Cause::MissingRequirement(id))
    } else if runtime.sets().requirement_context(id).is_some() {
        Ok(true)
    } else {
        Err(Cause::MissingRequirement(id))
    }
}
/// Execute initialized listeners in their supplied live order. A failure keeps
/// the completed gameplay prefix and identifies the actual stopping listener.
/// Positive pending queues and unknown synchronous consumers are not no-ops.
pub fn dispatch<P>(
    state: &mut LoadedState,
    backend: &mut stock::Runtime<'_, P>,
    requirements: &mut changes::Runtime,
    inputs: Inputs<'_>,
    owner: i32,
) -> Result<Progress, Error> {
    if requirements.binding() != state.binding() || !backend.script.script().matches(state) {
        return Err(Error::Binding);
    }
    let mut progress = Progress::default();
    for (index, listener) in inputs.listeners.iter().copied().enumerate() {
        let result = (|| -> Result<bool, Cause> {
            match listener {
                Listener::LuaPlayerTurnStarted => backend
                    .script
                    .on_player_turn_started(state, owner)
                    .map_err(Cause::Script)?,
                Listener::Gossip {
                    pending_count: Some(0),
                } => {}
                Listener::Gossip { pending_count } => {
                    return Err(Cause::GossipPending(pending_count))
                }
                Listener::Emergency {
                    signals_count: Some(0),
                } => {}
                Listener::Emergency { signals_count } => {
                    return Err(Cause::EmergencyPending(signals_count))
                }
                Listener::AnalysisTelemetry => {}
                Listener::Diplomacy { owner: receiver } => {
                    super::diplomacy_turn_started(state, receiver, owner)
                        .map_err(Cause::Diplomacy)?
                }
                Listener::Relationship { id, receiver } => {
                    if removed(requirements, id)? {
                        return Ok(false);
                    }
                    relationship::dispatch(
                        state,
                        inputs.relationship_definitions,
                        inputs.alive_majors,
                        requirements,
                        &mut StockServices { backend },
                        relationship::Invocation {
                            receiver: id,
                            descriptor: receiver,
                            reported: owner,
                        },
                    )
                    .map_err(Cause::Relationship)?;
                }
                Listener::Victory { id, receiver } => {
                    if removed(requirements, id)? {
                        return Ok(false);
                    }
                    if requirements
                        .state(id)
                        .ok_or(Cause::MissingRequirement(id))?
                        .active
                    {
                        let v = inputs.victory.as_ref().ok_or(Cause::MissingVictoryInputs)?;
                        let tested = victory::test(
                            state,
                            v.definitions,
                            v.current,
                            v.scoring,
                            v.scoring_definitions,
                            receiver,
                        )
                        .map_err(Cause::Victory)?;
                        requirements
                            .set_state(state, &mut StockServices { backend }, id, tested, true)
                            .map_err(Cause::Change)?;
                    }
                }
                Listener::Unsupported { identity } => return Err(Cause::Unsupported(identity)),
            }
            Ok(true)
        })();
        match result {
            Ok(true) => {
                progress.completed += 1;
                backend
                    .reached
                    .push("turn-start ordered subscriber complete");
            }
            Ok(false) => progress.skipped_unsubscribed += 1,
            Err(cause) => {
                return Err(Error::Listener {
                    index,
                    cause: Box::new(cause),
                })
            }
        }
    }
    Ok(progress)
}
