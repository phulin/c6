//! Source-ordered human-sender delegation command through an external AI response.
//!
//! This executes SetDelegation, not the broader UI offer/availability query.
//! The admitted endpoints are met, peaceful, alive majors. Human recipients,
//! uninitialized response handlers and arbitrary diplomacy remain unsupported.
use super::configuration::Configuration;
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash,
    unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{ActionStamp, LoadedState},
};

pub const ACTION_TYPE: u32 = 0xc08d_3473;
pub const DEFINITION_NAME: &[u8] = b"DIPLOACTION_DIPLOMATIC_DELEGATION";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Response {
    Accept,
    Refuse,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingDecision {
    pub sender: i32,
    pub recipient: i32,
    pub choices: [Response; 2],
}
/// A retained command, not a second copy of any diplomacy component.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Request {
    pub decision: PendingDecision,
    stamp: ActionStamp,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Accepted { gold_cost: i32 },
    Refused,
    InsufficientGold,
}
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Policy(c6_core::loaded_state::diplomatic_delegation::Error),
    Configuration(super::configuration::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Rules(crate::Error),
    Missing(&'static str),
    Binding,
    Rejected,
    Conflict,
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux delegation command: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

pub struct Definitions {
    binding: WireBinding,
    cost: i32,
    divisor: i32,
    speed_multipliers: Vec<(u32, i32)>,
}
impl Definitions {
    pub fn from_rules<E>(state: &LoadedState, rules: &Rules) -> Result<Self, Error<E>> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Missing("Linux XP2 delegation rules"));
        }
        let actions = collection(rules, "DiplomaticActions", "DiplomaticActionType")
            .map_err(|_| Error::Missing("DiplomaticActions registry"))?;
        let action = actions
            .find(linux_type_hash(DEFINITION_NAME))
            .ok_or(Error::Missing("delegation action definition"))?;
        let row = rules
            .gameplay()
            .table("DiplomaticActions")
            .and_then(|t| t.row_by_id(action.database_rowid))
            .ok_or(Error::Missing("delegation definition row"))?;
        let cost = row
            .get("Cost")
            .ok()
            .and_then(sql_integer)
            .ok_or(Error::Missing("delegation Cost"))?;
        let divisor = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?
            .rows()
            .find(|r| r.text("Name").ok() == Some("PURCHASE_DIVISOR"))
            .and_then(|r| r.get("Value").ok())
            .and_then(sql_integer)
            .ok_or(Error::Missing("PURCHASE_DIVISOR"))?;
        if divisor == 0 {
            return Err(Error::Missing("nonzero PURCHASE_DIVISOR"));
        }
        let speeds = collection(rules, "GameSpeeds", "GameSpeedType")
            .map_err(|_| Error::Missing("GameSpeeds registry"))?;
        let mut speed_multipliers = Vec::new();
        for speed in speeds.entries() {
            let value = rules
                .gameplay()
                .table("GameSpeeds")
                .and_then(|t| t.row_by_id(speed.database_rowid))
                .and_then(|r| r.get("CostMultiplier").ok())
                .and_then(sql_integer)
                .ok_or(Error::Missing("speed CostMultiplier"))?;
            speed_multipliers.push((linux_type_hash(&speed.type_name), value));
        }
        Ok(Self {
            binding: state.binding().clone(),
            cost,
            divisor,
            speed_multipliers,
        })
    }
    pub fn cost<E>(
        &self,
        state: &LoadedState,
        config: &Configuration<'_>,
    ) -> Result<i32, Error<E>> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        config
            .require_binding(state.binding())
            .map_err(Error::Configuration)?;
        let speed = config
            .game_fields()
            .game_speed_type
            .ok_or(Error::Missing("current game speed"))?;
        let multiplier = self
            .speed_multipliers
            .iter()
            .find(|v| v.0 == speed)
            .map_or(100, |v| v.1);
        Ok(action_cost(self.cost, multiplier, self.divisor))
    }
}
fn action_cost(base: i32, multiplier: i32, divisor: i32) -> i32 {
    let scaled = c6_core::research_acquisition::units::adjust_for_game_speed(base, multiplier);
    scaled.wrapping_sub(scaled.wrapping_rem(divisor))
}
/// Each method is a reached callback, never an optional success stub. Implement
/// against the same current modifiers, reports and RNG as the enclosing Session.
pub trait Effects<P> {
    type Error;
    fn preflight(
        &self,
        state: &LoadedState,
        backend: &P,
        sender: i32,
        recipient: i32,
    ) -> Result<(), Self::Error>;
    fn gold_changed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn delegation_established(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        sender: i32,
        recipient: i32,
    ) -> Result<(), Self::Error>;
    fn recheck_visibility(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        sender: i32,
        recipient: i32,
    ) -> Result<(), Self::Error>;
}
/// Qualify the supported command at SetDelegation::Process's response-policy
/// call. In particular no delegation flag, gold, response or native AI choice
/// is manufactured before the recipient supplies its decision.
pub(crate) fn request<E>(
    state: &LoadedState,
    sender: i32,
    recipient: i32,
    config: &Configuration<'_>,
) -> Result<Request, Error<E>> {
    config
        .require_binding(state.binding())
        .map_err(Error::Configuration)?;
    if config
        .is_any_multiplayer(state.binding())
        .map_err(Error::Configuration)?
    {
        return Err(Error::Missing("single-player delegation command"));
    }
    if sender == recipient {
        return Err(Error::Rejected);
    }
    for owner in [sender, recipient] {
        if config
            .civilization_level_type_id(owner)
            .map_err(Error::Configuration)?
            != c6_core::diplomacy_meeting::MAJOR_LEVEL
            || !state.player_alive(owner).map_err(Error::State)?
        {
            return Err(Error::Missing("alive major delegation endpoints"));
        }
    }
    if state.player_slot_status(sender).map_err(Error::State)? != 3
        || state.player_slot_status(recipient).map_err(Error::State)? != 1
    {
        return Err(Error::Missing(
            "human sender and AI recipient delegation branch",
        ));
    }
    let relations = state.game_relations().map_err(Error::State)?;
    if !relations
        .has_met(sender, recipient)
        .map_err(Error::Relations)?
        || relations
            .is_at_war(sender, recipient)
            .map_err(Error::Relations)?
    {
        return Err(Error::Rejected);
    }
    state
        .validate_delegation_response_policy(recipient, sender, linux_type_hash(DEFINITION_NAME))
        .map_err(Error::Policy)?;
    Ok(Request {
        decision: PendingDecision {
            sender,
            recipient,
            choices: [Response::Accept, Response::Refuse],
        },
        stamp: state.action_stamp(),
    })
}
fn player<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current delegation player"))
}
fn change_gold<P, E: Effects<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    effects: &mut E,
    owner: i32,
    delta: i32,
) -> Result<(), Error<E::Error>> {
    let before = player(state, owner)?.treasury.gold_balance_q8;
    let after = before.saturating_add(delta);
    if after != before {
        state
            .treasury_mut(owner)
            .map_err(Error::State)?
            .gold_balance_q8 = after;
        effects
            .gold_changed(state, backend, owner)
            .map_err(Error::Effect)?;
    }
    Ok(())
}
pub(crate) fn respond<P, E: Effects<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    request: Request,
    response: Response,
    definitions: &Definitions,
    config: &Configuration<'_>,
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    if request.stamp != state.action_stamp() {
        return Err(Error::Conflict);
    }
    let PendingDecision {
        sender, recipient, ..
    } = request.decision;
    let cost = definitions.cost(state, config)?;
    effects
        .preflight(state, backend, sender, recipient)
        .map_err(Error::Effect)?;
    // ProcessAction's initialized branch clears this AFTER evaluating policy,
    // for both acceptance and refusal. No policy selector is executed here.
    state
        .finish_delegation_response_policy(recipient, sender, linux_type_hash(DEFINITION_NAME))
        .map_err(Error::Policy)?;
    if response == Response::Refuse {
        return Ok(Outcome::Refused);
    }
    let cost_q8 = cost.wrapping_shl(8);
    if player(state, sender)?.treasury.gold_balance_q8 < cost_q8 {
        return Ok(Outcome::InsufficientGold);
    }
    change_gold(
        state,
        backend,
        effects,
        sender,
        cost.wrapping_neg().wrapping_shl(8),
    )?;
    let slot =
        usize::try_from(recipient).map_err(|_| Error::Missing("delegation recipient slot"))?;
    *state
        .war_diplomacy_mut(sender)
        .map_err(Error::State)?
        .has_delegation
        .get_mut(slot)
        .ok_or(Error::Missing("delegation recipient slot"))? = true;
    // The source setter emits even when the flag was already true.
    effects
        .delegation_established(state, backend, sender, recipient)
        .map_err(Error::Effect)?;
    effects
        .recheck_visibility(state, backend, sender, recipient)
        .map_err(Error::Effect)?;
    change_gold(state, backend, effects, recipient, cost_q8)?;
    // The sender is human: ProcessActionResult's AI-only tail is not reached.
    Ok(Outcome::Accepted { gold_cost: cost })
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_point_cost_scales_before_divisor_rounding() {
        let cost = super::action_cost;
        assert_eq!(cost(25, 100, 5), 25);
        assert_eq!(cost(25, 50, 5), 10);
        assert_eq!(cost(25, 150, 5), 35);
        assert_eq!(cost(25, 200, 5), 50);
        // Q8 constructors wrap BEFORE the economic multiply; native AsInt
        // arithmetic-shifts negative fractions, signed remainder follows it.
        assert_eq!(cost(0x0100_0001, 100, 5), 0);
        assert_eq!(cost(-25, 50, 5), -10);
    }
}
