//! Source-ordered stock leaves reached by the Linux WarDeclared signal.
//!
//! These operations do not dispatch the signal or fulfill DiplomaticState's
//! state-change callback. Callers retain that separately ordered boundary.
pub mod backstab;

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
    aid_grievances: i32,
    dow_boosts: Vec<(i32, usize, bool)>,
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
        let aid_grievances = table
            .rows()
            .find(|r| {
                r.text("Name").ok()
                    == Some("WORLD_CONGRESS_REQUEST_FOR_MILITARY_AID_GRIEVANCES_MIN")
            })
            .and_then(|r| r.get("Value").ok())
            .and_then(sql_integer)
            .ok_or(Error::Missing("military aid grievance threshold"))?;
        let mut dow_boosts = Vec::new();
        for (index, row) in rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("Boosts"))?
            .rows()
            .enumerate()
        {
            let receive = match row
                .text("BoostClass")
                .map_err(|_| Error::Missing("BoostClass"))?
            {
                "BOOST_TRIGGER_RECEIVE_DOW" => true,
                "BOOST_TRIGGER_DOW_CASUS_BELLI" => false,
                _ => continue,
            };
            let row_id = row
                .get("BoostID")
                .ok()
                .and_then(sql_integer)
                .ok_or(Error::Missing("BoostID"))?;
            dow_boosts.push((row_id, index, receive));
        }
        Ok(Self {
            aid_grievances,
            dow_boosts,
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

/// MilitaryAid2931630's source guard, evaluated at its actual ring position.
/// A true result requires StartEmergency, including its synchronous effects.
pub fn military_aid_start_reached(
    state: &LoadedState,
    defs: &Definitions,
    issuer: i32,
    target: i32,
    issuer_is_major: bool,
    target_is_major: bool,
) -> Result<bool, Error> {
    if *state.binding() != defs.binding {
        return Err(Error::Binding);
    }
    if !issuer_is_major || !target_is_major {
        return Ok(false);
    }
    let score = state
        .game_relations()
        .map_err(Error::State)?
        .grievance_score(target, issuer)
        .map_err(|_| Error::Missing("current grievance score"))?;
    Ok(score >= defs.aid_grievances)
}

#[derive(Debug)]
pub enum BoostError<E> {
    Branch(Error),
    Research(c6_core::relic_creation::boost::Error<crate::research::Error<E>>),
}

/// Boost.Manager.OnDOW2844690 for the two actual stock callback classes.
/// The positive receive-DOW branch invokes the complete canonical boost manager
/// and its mandatory research, signal and notification effects for the victim.
#[allow(clippy::too_many_arguments)]
pub fn on_dow_boost<E: crate::research::Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    research: &crate::research::Definitions,
    effects: &mut E,
    boost_row_id: i32,
    issuer: i32,
    target: i32,
    issuer_is_major: bool,
    war_type: i32,
) -> Result<(), BoostError<E::Error>> {
    if *state.binding() != defs.binding {
        return Err(BoostError::Branch(Error::Binding));
    }
    let (_, index, receive) = defs
        .dow_boosts
        .iter()
        .find(|(row, _, _)| *row == boost_row_id)
        .ok_or(BoostError::Branch(Error::Missing("current DOW boost row")))?;
    let recipient = if *receive {
        if !issuer_is_major {
            return Ok(());
        }
        target
    } else {
        if matches!(
            war_type as u32,
            0xfb189687 | 0x01f4de45 | 0x1d3fe969 | 0x4396d725 | 0x73b8c966
        ) {
            return Ok(());
        }
        issuer
    };
    c6_core::relic_creation::boost::trigger_boost(
        &mut crate::research::Context {
            state,
            definitions: research,
            effects,
        },
        Some(recipient),
        *index,
    )
    .map_err(BoostError::Research)
}

/// The already game-speed-adjusted thresholds used by
/// Player.Diplomacy.GetAllianceLevelWithPlayer2aab250. Callers obtain both from
/// their current Economic.AdjustForGameSpeed context; raw SQL values alone do
/// not establish these current values.
pub struct AllianceThresholds {
    pub level_two: i32,
    pub level_three: i32,
}

/// The actual Backstab target requirement's PlayerAllianceLevelWithPlayer.Test
/// checks the declarer's alliance level with each emergency player-data entry.
/// For the captured stock callback the data is exactly the declaration victim
/// and the requirement Level is2. This returns the semantic predicate, not the
/// native RequirementResult status (whose fulfilled value is0).
pub fn backstab_alliance_requirement(
    state: &LoadedState,
    issuer: i32,
    emergency_players: &[i32],
    minimum_level: i32,
    thresholds: &AllianceThresholds,
) -> Result<bool, Error> {
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == issuer)
        .ok_or(Error::Missing("backstab target Diplomacy"))?;
    for other in emergency_players {
        // GetAlliancePointsWithPlayer2aab2d0 explicitly returns0 for an
        // out-of-range unsigned index, including native PlayerTypes(-1).
        let points = usize::try_from(*other)
            .ok()
            .and_then(|index| player.diplomacy.alliance_points_per_player.get(index))
            .copied()
            .unwrap_or(0);
        let level = i32::from(points >= 0)
            + i32::from(points >= thresholds.level_two)
            + i32::from(points >= thresholds.level_three);
        if level >= minimum_level {
            return Ok(true);
        }
    }
    Ok(false)
}
