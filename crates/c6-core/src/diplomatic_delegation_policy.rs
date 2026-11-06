//! Deterministic tail of the initialized Linux delegation response policy seam.
//!
//! External policies select accept/refuse. This preserves the reached
//! ChangePartnerActionClosed(other, delegation, false) store in either case.
//! Uninitialized handlers require additional state initialization and activation
//! and remain unsupported here. This is not a complete delegation action.
use crate::player_ai_diplomatic::{BoolByte, Diplomatic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedVersion,
    InvalidPlayers,
    MissingHandler,
    UninitializedHandler,
    HandlerIdentity,
    MissingClosedRow,
    MissingDefinition,
    DuplicateDefinition,
}

/// Preserve the source-established side effect after an external response.
///
/// `definition_hash` identifies DIPLOACTION_DIPLOMATIC_DELEGATION in the actual
/// rules registry, not the SetDelegation action message type hash. The imported
/// closed row must already contain exactly one entry for that definition.
/// Validation finishes before mutation; no evaluation, callback or RNG runs.
pub fn finish_initialized_response(
    state: &mut Diplomatic,
    target: i32,
    sender: i32,
    definition_hash: u32,
) -> Result<(), Error> {
    let (row, position) = validate_position(state, target, sender, definition_hash)?;
    state.closed_partner_action[row][position].value = BoolByte(0);
    Ok(())
}

/// Read-only admission for the initialized branch, before exposing a decision.
pub fn validate_initialized_response(
    state: &Diplomatic,
    target: i32,
    sender: i32,
    definition_hash: u32,
) -> Result<(), Error> {
    validate_position(state, target, sender, definition_hash).map(|_| ())
}

fn validate_position(
    state: &Diplomatic,
    target: i32,
    sender: i32,
    definition_hash: u32,
) -> Result<(usize, usize), Error> {
    if state.wire_version != 18 {
        return Err(Error::UnsupportedVersion);
    }
    if target < 0 || sender < 0 || target == sender || state.player_id != target {
        return Err(Error::InvalidPlayers);
    }
    let index = usize::try_from(sender).map_err(|_| Error::InvalidPlayers)?;
    let handler = state
        .diplo_action_evaluator
        .handlers
        .get(index)
        .ok_or(Error::MissingHandler)?;
    if handler.owner == -1 || handler.opponent == -1 {
        return Err(Error::UninitializedHandler);
    }
    if handler.owner != target || handler.opponent != sender {
        return Err(Error::HandlerIdentity);
    }
    let row = state
        .closed_partner_action
        .get(index)
        .ok_or(Error::MissingClosedRow)?;
    let mut matches = row
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.hash == definition_hash);
    let position = matches.next().ok_or(Error::MissingDefinition)?.0;
    if matches.next().is_some() {
        return Err(Error::DuplicateDefinition);
    }
    Ok((index, position))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_ai_diplomatic::{
        ActionEvalVector, DiplomaticActionHandler, HashEntry, StateVector,
    };

    fn state() -> Diplomatic {
        Diplomatic {
            wire_version: 18,
            player_id: 1,
            min_force_to_notice_q8: 0,
            min_troubling_force_q8: 0,
            diplomatic_scores_q8: vec![],
            base_diplo_scores_q8: vec![],
            visible_scores_q8: vec![],
            diplomatic_states: StateVector {
                states: vec![],
                is_updated: BoolByte(0),
            },
            diplo_action_evaluator: ActionEvalVector {
                handlers: vec![DiplomaticActionHandler {
                    owner: 1,
                    opponent: 0,
                    desirability: Some(vec![]),
                    cost: Some(vec![]),
                    alliance_value_q8: 0,
                    war_value_q8: 0,
                    war_value_penalties_q8: 0,
                    is_initialized: BoolByte(1),
                }],
                is_updated: BoolByte(0),
            },
            num_majors_met: 0,
            player_threats_q8: vec![],
            player_major_threats: vec![],
            player_trusts_q8: vec![],
            managed_contracts: vec![],
            best_potential_friend: -1,
            partner_action_preference: vec![],
            partner_action_worth_q8: vec![],
            closed_partner_action: vec![vec![HashEntry {
                hash: 17,
                value: BoolByte(1),
            }]],
            requested_items: vec![],
            excluded_items: Some(vec![]),
            favor_traded_this_turn: 0,
            done_off_turn: BoolByte(0),
        }
    }

    #[test]
    fn preserves_all_other_typed_state_and_is_idempotent() {
        let mut value = state();
        let mut expected = value.clone();
        expected.closed_partner_action[0][0].value = BoolByte(0);
        finish_initialized_response(&mut value, 1, 0, 17).unwrap();
        assert_eq!(value, expected);
        finish_initialized_response(&mut value, 1, 0, 17).unwrap();
        assert_eq!(value, expected);
    }

    #[test]
    fn initialization_uses_bound_player_ids_instead_of_saved_bool() {
        let mut value = state();
        value.diplo_action_evaluator.handlers[0].is_initialized = BoolByte(0);
        validate_initialized_response(&value, 1, 0, 17).unwrap();
        assert_eq!(value.closed_partner_action[0][0].value, BoolByte(1));
        finish_initialized_response(&mut value, 1, 0, 17).unwrap();
        assert_eq!(value.closed_partner_action[0][0].value, BoolByte(0));
        assert_eq!(
            value.diplo_action_evaluator.handlers[0].is_initialized,
            BoolByte(0)
        );
    }

    #[test]
    fn rejects_uninitialized_and_ambiguous_imports_before_mutation() {
        let mut value = state();
        value.diplo_action_evaluator.handlers[0].owner = -1;
        let before = value.clone();
        assert_eq!(
            finish_initialized_response(&mut value, 1, 0, 17),
            Err(Error::UninitializedHandler)
        );
        assert_eq!(value, before);
        value.diplo_action_evaluator.handlers[0].owner = 1;
        value.closed_partner_action[0].push(HashEntry {
            hash: 17,
            value: BoolByte(1),
        });
        let before = value.clone();
        assert_eq!(
            finish_initialized_response(&mut value, 1, 0, 17),
            Err(Error::DuplicateDefinition)
        );
        assert_eq!(value, before);
    }
}
