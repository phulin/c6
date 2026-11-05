//! Source-ordered stock leaves reached by the Linux WarDeclared signal.
//!
//! These operations do not dispatch the signal or fulfill DiplomaticState's
//! state-change callback. Callers retain that separately ordered boundary.
use crate::{game_parameters::sql_integer, Rules};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Debug)]
pub enum Error {
    Binding,
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    Access(c6_core::loaded_state::game_statistics::AccessError),
}

pub struct Definitions {
    binding: WireBinding,
    declaration: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let table = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut rows = table
            .rows()
            .filter(|r| r.text("Name").ok() == Some("MAYHEM_DECLARATION_OF_WAR"));
        let declaration = rows
            .next()
            .and_then(|r| r.get("Value").ok())
            .and_then(sql_integer)
            .ok_or(Error::Missing("MAYHEM_DECLARATION_OF_WAR"))?;
        if rows.next().is_some() {
            return Err(Error::Missing("unique MAYHEM_DECLARATION_OF_WAR"));
        }
        Ok(Self {
            binding: state.binding().clone(),
            declaration,
        })
    }
}

/// MayhemTracker.LogWarDeclaration20f51d0, admitted non-barbarian surprise war.
/// The integer conversion, doubling and accumulator addition are binary32.
pub fn global_mayhem(state: &mut LoadedState, defs: &Definitions) -> Result<(), Error> {
    if *state.binding() != defs.binding {
        return Err(Error::Binding);
    }
    let contribution = defs.declaration as f32;
    state
        .add_global_mayhem(contribution + contribution)
        .map_err(Error::Access)?;
    Ok(())
}

/// Targets.WarStarted21ab5a0 → CivKnowledge.StartWar2001ed0 →
/// WarKnowledge.Initialize1fff580. The native war type argument is unused.
pub fn targets_war_started(
    state: &mut LoadedState,
    receiver: i32,
    issuer: i32,
    target: i32,
) -> Result<(), Error> {
    let opponent = if receiver == issuer {
        target
    } else if receiver == target {
        issuer
    } else {
        return Ok(());
    };
    if opponent == -1 {
        return Ok(());
    }
    let current = state
        .combat_ai_targets_mut(receiver)
        .map_err(Error::Access)?;
    if current.player_id != receiver || current.knowledge_base.owner != receiver {
        return Err(Error::Binding);
    }
    let war = current
        .knowledge_base
        .war_knowledge
        .get_mut(usize::try_from(opponent).map_err(|_| Error::Missing("WarKnowledge index"))?)
        .ok_or(Error::Missing("WarKnowledge slot"))?;
    war.opponent = opponent;
    war.goal_score_q8 = 0;
    war.current_tally_q8 = 0;
    war.is_active.0 = 1;
    current.new_war_started.0 = 1;
    Ok(())
}

/// EndNonwarOperations2148600 iterates current active operations. An empty
/// current collection reaches no cancellation or callback. Other branches must
/// supply actual operation termination, never treat them as completed here.
pub fn end_nonwar_operations_empty(state: &mut LoadedState, receiver: i32) -> Result<(), Error> {
    let military = state
        .player_ai_military_mut(receiver)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current Military"))?;
    if !military.active_operations.entries.is_empty() {
        return Err(Error::Missing("EndNonwarOperations cancellation"));
    }
    Ok(())
}

/// PrepareForWarWith214c710(false), after EndNonwarOperations and the preceding
/// diplomatic state trigger. The true-only PrepareForWar signal is not reached.
pub fn clear_prepare_for_war(
    state: &mut LoadedState,
    receiver: i32,
    opponent: i32,
) -> Result<(), Error> {
    let military = state
        .player_ai_military_mut(receiver)
        .map_err(Error::State)?
        .as_mut()
        .ok_or(Error::Missing("current Military"))?;
    military
        .prepare_for_war_with
        .get_mut(usize::try_from(opponent).map_err(|_| Error::Missing("prepare-for-war index"))?)
        .ok_or(Error::Missing("prepare-for-war slot"))?
        .0 = 0;
    Ok(())
}
