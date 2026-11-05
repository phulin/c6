//! External surprise-war order over the current Linux diplomatic owners.
//!
//! The source-ordered action keeps gameplay callbacks explicit. A callback must
//! execute its effects against these same owners and RNG or return an error;
//! there is no successful default callback or native policy implementation.
use crate::{
    loaded_actions::configuration::Configuration, unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, diplomacy_meeting::MAJOR_LEVEL,
    loaded_state::LoadedState,
};

pub const SURPRISE_WAR: i32 = 0x4396_d725;
pub const SURPRISE_ACTION: u32 = 0x1b4d_9d8f;
pub mod diplomacy;
pub mod event_branches;
pub mod plain;
pub mod requirements;
pub mod requirement_modifiers;

#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Relations(c6_core::game_diplomatic_relations::Error),
    Configuration(crate::loaded_actions::configuration::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    Missing(&'static str),
    Binding,
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux surprise war: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

pub struct Definitions {
    binding: WireBinding,
    earliest_major_turn: i32,
    action_index: Option<usize>,
    action_warmonger: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let earliest_major_turn = parameters
            .rows()
            .find(|r| r.text("Name").ok() == Some("DIPLOMACY_EARLIEST_MAJOR_DOW_TURN"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("DIPLOMACY_EARLIEST_MAJOR_DOW_TURN"))?;
        let actions = collection(rules, "DiplomaticActions", "DiplomaticActionType")
            .map_err(Error::Definitions)?;
        let action_index = actions.find_index(SURPRISE_ACTION);
        let action_warmonger = if let Some(index) = action_index {
            let table = rules
                .gameplay()
                .table("DiplomaticActions")
                .ok_or(Error::Missing("DiplomaticActions"))?;
            table
                .row_by_id(actions.entries()[index].database_rowid)
                .and_then(|r| r.get("WarmongerPercent").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or(Error::Missing("WarmongerPercent"))?
        } else {
            0
        };
        let wars = collection(rules, "Wars", "WarType").map_err(Error::Definitions)?;
        if wars.find_index(SURPRISE_WAR as u32).is_none() {
            return Err(Error::Missing("SURPRISE_WAR definition"));
        }
        let levels = collection(rules, "CivilizationLevels", "CivilizationLevelType")
            .map_err(Error::Definitions)?;
        let major = levels
            .find_index(MAJOR_LEVEL)
            .ok_or(Error::Missing("major CivilizationLevel"))?;
        let receives = rules
            .gameplay()
            .table("CivilizationLevels")
            .and_then(|t| t.row_by_id(levels.entries()[major].database_rowid))
            .and_then(|r| r.get("CanReceiveInfluence").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("major CanReceiveInfluence"))?;
        if receives != 0 {
            return Err(Error::Missing("influence-receiving major declaration"));
        }
        Ok(Self {
            binding: state.binding().clone(),
            earliest_major_turn,
            action_index,
            action_warmonger,
        })
    }
    pub fn warmonger_points(
        &self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<i32, Error<std::convert::Infallible>> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let diplomacy = &state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("declarer's Diplomacy"))?
            .diplomacy;
        // ComputeDOWWarmongerPoints2aac690: signed min; no era adjustment here.
        Ok(self
            .action_warmonger
            .min(diplomacy.war_and_alliance_modifiers[6]))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Legality {
    Legal,
    Rejected,
}

/// Source CanDeclareWarOn for distinct alive majors with no teammate expansion.
/// A prior war-change turn requires the exact game-speed-adjusted peace delay;
/// `None` is unavailable and never an invented zero-duration truce.
pub struct Inputs<'a, 'c> {
    pub definitions: &'a Definitions,
    pub configuration: &'a Configuration<'c>,
    pub minimum_peace_turns: Option<i32>,
}
pub fn can_declare_surprise_war(
    state: &LoadedState,
    issuer: i32,
    target: i32,
    inputs: &Inputs<'_, '_>,
) -> Result<Legality, Error<std::convert::Infallible>> {
    if state.binding() != &inputs.definitions.binding {
        return Err(Error::Binding);
    }
    inputs
        .configuration
        .require_binding(state.binding())
        .map_err(Error::Configuration)?;
    let relations = state.game_relations().map_err(Error::State)?;
    if relations
        .is_at_war(issuer, target)
        .map_err(Error::Relations)?
    {
        return Ok(Legality::Rejected);
    }
    let first = state.world().player(issuer).map_err(Error::World)?;
    let second = state.world().player(target).map_err(Error::World)?;
    let first_team = first.team.ok_or(Error::Missing("declarer team"))?;
    let second_team = second.team.ok_or(Error::Missing("target team"))?;
    if first_team != -1 && first_team == second_team {
        return Ok(Legality::Rejected);
    }
    if issuer == target {
        return Ok(Legality::Rejected);
    }
    for owner in [issuer, target] {
        if inputs
            .configuration
            .civilization_level_type_id(owner)
            .map_err(Error::Configuration)?
            != MAJOR_LEVEL
        {
            return Err(Error::Missing("major civilization declaration branch"));
        }
        if !state.player_alive(owner).map_err(Error::State)? {
            return Err(Error::Missing("alive major declaration branch"));
        }
    }
    let turn = state.retained().core.current_turn;
    if turn < inputs.definitions.earliest_major_turn {
        return Ok(Legality::Rejected);
    }
    // The bounded action accepts no additional member of either team. The
    // current collection is used rather than a detached diplomatic snapshot.
    for player in state.world().players() {
        let owner = player.owner();
        if owner == issuer || owner == target {
            continue;
        }
        if !player
            .is_alive
            .ok_or(Error::Missing("current alive team roster"))?
        {
            continue;
        }
        let team = player
            .team
            .ok_or(Error::Missing("current alive player's team"))?;
        if (first_team != -1 && team == first_team) || (second_team != -1 && team == second_team) {
            return Err(Error::Missing("war teammate expansion"));
        }
    }
    if relations
        .has_declared_friendship(issuer, target)
        .map_err(Error::Relations)?
        || relations
            .alliance_type(issuer, target)
            .map_err(Error::Relations)?
            != -1
        || relations
            .is_permanently_allied(issuer, target)
            .map_err(Error::Relations)?
    {
        return Ok(Legality::Rejected);
    }
    // WarTypeNeedsDenouncement2aa7200 returns false for SURPRISE_WAR.
    if relations
        .is_in_defensive_pact(issuer, target)
        .map_err(Error::Relations)?
    {
        return Ok(Legality::Rejected);
    }
    let diplomacy = &state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == issuer)
        .ok_or(Error::Missing("declarer's Diplomacy"))?
        .diplomacy;
    if let Some(index) = inputs.definitions.action_index {
        let banned = diplomacy
            .banned_action_counts
            .get(index)
            .ok_or(Error::Missing("current banned-action vector"))?
            .value;
        if banned > 0 {
            return Ok(Legality::Rejected);
        }
        // Specific bans store native definition ordinals after Read fixups,
        // which are not interchangeable with this transport's saved hashes.
        if !diplomacy.banned_action_civilizations.is_empty() {
            return Err(Error::Missing("current civilization-specific action bans"));
        }
    }
    let changed = relations
        .war_changed_turn(issuer, target)
        .map_err(Error::Relations)?;
    if changed != -1
        && changed.wrapping_add(
            inputs
                .minimum_peace_turns
                .ok_or(Error::Missing("adjusted minimum peace duration"))?,
        ) > turn
    {
        return Ok(Legality::Rejected);
    }
    Ok(Legality::Legal)
}

/// Every method denotes a reached native call, including calls whose semantic
/// effects depend on current signal receivers. Implementations must preserve
/// reentrant gameplay effects and RNG. Presentation may be emitted as reports.
pub trait Effects<P> {
    type Error;
    fn preflight(
        &self,
        state: &LoadedState,
        backend: &P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn team_declaration_log(
        &mut self,
        state: &mut LoadedState,
        issuer: i32,
        target: i32,
        before: bool,
    ) -> Result<(), Self::Error>;
    fn diplomatic_log(
        &mut self,
        state: &mut LoadedState,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn expel_from_territory(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn visibility_blocker(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        source: i32,
        recipient: i32,
    ) -> Result<(), Self::Error>;
    fn delegation_changed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn embassy_removed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn recheck_visibility(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn unassign_governors(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn war_declared(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        research: &mut crate::research::Runtime,
        visibility: &mut crate::game_update_state::deactivation::VisibilityHistory,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
    fn report_declaration(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        issuer: i32,
        target: i32,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected,
    Declared,
}
#[allow(clippy::too_many_arguments)]
pub fn execute<P, E: Effects<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    research: &mut crate::research::Runtime,
    visibility: &mut crate::game_update_state::deactivation::VisibilityHistory,
    issuer: i32,
    target: i32,
    inputs: &Inputs<'_, '_>,
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    // Convert the effect-free query's error without treating unknown as false.
    let legality =
        can_declare_surprise_war(state, issuer, target, inputs).map_err(|e| match e {
            Error::State(e) => Error::State(e),
            Error::World(e) => Error::World(e),
            Error::Relations(e) => Error::Relations(e),
            Error::Configuration(e) => Error::Configuration(e),
            Error::Definitions(e) => Error::Definitions(e),
            Error::Missing(e) => Error::Missing(e),
            Error::Binding => Error::Binding,
            Error::Effect(e) => match e {},
        })?;
    if legality == Legality::Rejected {
        return Ok(Outcome::Rejected);
    }
    effects
        .preflight(state, backend, issuer, target)
        .map_err(Error::Effect)?;
    let relations = state.game_relations().map_err(Error::State)?;
    if !relations
        .has_met(issuer, target)
        .map_err(Error::Relations)?
    {
        return Err(Error::Missing(
            "original completed contact before declaration",
        ));
    }
    let turn = state.retained().core.current_turn;
    let team_path = state.world().player(issuer).map_err(Error::World)?.team != Some(-1)
        && state.world().player(target).map_err(Error::World)?.team != Some(-1);
    if team_path {
        effects
            .team_declaration_log(state, issuer, target, true)
            .map_err(Error::Effect)?;
    }
    // This deliberately calls only the storage leaf. SetAtWar's remaining
    // source calls follow below; a failure retains this exact prefix.
    state
        .war_relations_mut()
        .map_err(Error::State)?
        .set_at_war_fields(issuer, target, SURPRISE_WAR, turn)
        .map_err(Error::Relations)?;
    effects
        .diplomatic_log(state, issuer, target)
        .map_err(Error::Effect)?;
    for (owner, other) in [(issuer, target), (target, issuer)] {
        let index = usize::try_from(other).map_err(|_| Error::Missing("diplomatic target slot"))?;
        *state
            .war_diplomacy_mut(owner)
            .map_err(Error::State)?
            .denounced
            .get_mut(index)
            .ok_or(Error::Missing("denouncement slot"))? = -1;
    }
    effects
        .expel_from_territory(state, backend, issuer, target)
        .map_err(Error::Effect)?;
    effects
        .expel_from_territory(state, backend, target, issuer)
        .map_err(Error::Effect)?;
    for (source, recipient) in [(target, issuer), (target, issuer), (issuer, target)] {
        effects
            .visibility_blocker(state, backend, source, recipient)
            .map_err(Error::Effect)?;
    }
    for (owner, other) in [(issuer, target), (target, issuer)] {
        state
            .war_diplomacy_mut(owner)
            .map_err(Error::State)?
            .has_delegation[other as usize] = false;
        effects
            .delegation_changed(state, backend, owner, other)
            .map_err(Error::Effect)?;
    }
    for (owner, other) in [(issuer, target), (target, issuer)] {
        state
            .war_diplomacy_mut(owner)
            .map_err(Error::State)?
            .has_embassy[other as usize] = false;
        effects
            .embassy_removed(state, backend, owner, other)
            .map_err(Error::Effect)?;
    }
    effects
        .recheck_visibility(state, backend, issuer, target)
        .map_err(Error::Effect)?;
    effects
        .recheck_visibility(state, backend, target, issuer)
        .map_err(Error::Effect)?;
    effects
        .unassign_governors(state, backend, issuer, target)
        .map_err(Error::Effect)?;
    // Influence.CanReceiveInfluence is false for the admitted major target;
    // city-state influence blocking belongs to a separate explicit branch.
    effects
        .war_declared(state, backend, research, visibility, issuer, target)
        .map_err(Error::Effect)?;
    effects
        .report_declaration(state, backend, issuer, target)
        .map_err(Error::Effect)?;
    if team_path {
        effects
            .team_declaration_log(state, issuer, target, false)
            .map_err(Error::Effect)?;
    }
    Ok(Outcome::Declared)
}

/// Source Player.Diplomacy.OnWarDeclared's principal scalar prefix. The
/// receiver's subsequent defensive pact/grievance/promise effects are distinct
/// ordered callbacks, not fulfilled by updating these counters.
pub fn principal_war_counters(
    state: &mut LoadedState,
    receiver: i32,
    issuer: i32,
    target: i32,
) -> Result<(), c6_core::loaded_state::Error> {
    let turn = state.retained().core.current_turn;
    if receiver == issuer {
        let diplomacy = state.war_diplomacy_mut(receiver)?;
        diplomacy.last_war_and_city_turns[0] = turn;
        diplomacy.last_war_and_city_turns[8] = diplomacy.last_war_and_city_turns[8].wrapping_add(1);
        diplomacy.last_war_and_city_turns[9] = diplomacy.last_war_and_city_turns[9].wrapping_add(1);
        diplomacy.last_war_and_city_turns[3] = turn;
    }
    if receiver == target {
        state.war_diplomacy_mut(receiver)?.last_war_and_city_turns[1] = turn;
    }
    Ok(())
}
