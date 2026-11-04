//! Actual selected phase TurnMode rules for current next-player selection.
pub mod cityless_managers;
pub mod maintenance;
pub mod relationship_requirements;
pub mod requirement_changes;
pub mod social;
pub mod treasury_deals;
pub mod turn_started;
pub mod upkeep;
pub mod victory_requirements;
use crate::{registry::linux_type_hash, resource_binding::ResourceAuthority, Rules};
use c6_core::{game_update::Error, loaded_state::LoadedState};
#[derive(Debug)]
pub enum DefinitionError {
    Rules(crate::Error),
    Resources(crate::resource_binding::Error),
    Missing(&'static str),
}
pub struct Phases {
    modes: Vec<Option<u32>>,
    segments: Vec<Option<u32>>,
}
impl Phases {
    /// The authority supplies the CURRENT selected native slots, preserving
    /// null entries and order; SQL row IDs are only definition lookup keys.
    pub fn from_rules(
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        authority
            .validate_rules(rules)
            .map_err(DefinitionError::Resources)?;
        let table = rules
            .gameplay()
            .table("TurnPhases")
            .ok_or(DefinitionError::Missing("TurnPhases"))?;
        let modes = authority
            .phases()
            .slots()
            .iter()
            .map(|slot| {
                let Some(slot) = slot else { return Ok(None) };
                let row = table
                    .row_by_id(slot.row_id)
                    .ok_or(DefinitionError::Missing("selected phase rule"))?;
                Ok(Some(linux_type_hash(
                    row.text("TurnMode")
                        .map_err(DefinitionError::Rules)?
                        .as_bytes(),
                )))
            })
            .collect::<Result<Vec<_>, DefinitionError>>()?;
        let segments_table = rules
            .gameplay()
            .table("TurnSegments")
            .ok_or(DefinitionError::Missing("TurnSegments"))?;
        let segments = authority
            .phases()
            .slots()
            .iter()
            .map(|slot| {
                let Some(segment) = slot.as_ref().and_then(|s| s.active_segment) else {
                    return Ok(None);
                };
                let row = segments_table
                    .row_by_id(segment.row_id)
                    .ok_or(DefinitionError::Missing("selected active segment"))?;
                Ok(Some(linux_type_hash(
                    row.text("TurnSegmentType")
                        .map_err(DefinitionError::Rules)?
                        .as_bytes(),
                )))
            })
            .collect::<Result<Vec<_>, DefinitionError>>()?;
        Ok(Self { modes, segments })
    }
    /// GetCurrentTurnSegment2a54f60 reads the current selected phase's actual
    /// active segment. A null phase or segment returns native INVALID (-1).
    pub fn current_segment(&self, state: &LoadedState) -> Result<u32, Error> {
        let index = state.retained().core.phase_processor.current_phase;
        if index == -1 {
            return Ok(u32::MAX);
        }
        let index =
            usize::try_from(index).map_err(|_| Error::Unsupported("invalid current phase"))?;
        self.segments
            .get(index)
            .copied()
            .map(|v| v.unwrap_or(u32::MAX))
            .ok_or(Error::Unsupported(
                "current phase outside selected segment slots",
            ))
    }
    /// GetPhaseTurnMode2a55320 uses the exact sequential fallback only for an
    /// absent current definition. Invalid positive enum indices remain errors.
    pub fn mode(&self, state: &LoadedState) -> Result<u32, Error> {
        let index = state.retained().core.phase_processor.current_phase;
        if index == -1 {
            return Ok(0x5681fb60);
        }
        let index =
            usize::try_from(index).map_err(|_| Error::Unsupported("invalid current phase"))?;
        self.modes
            .get(index)
            .copied()
            .map(|value| value.unwrap_or(0x5681fb60))
            .ok_or(Error::Unsupported(
                "current phase outside selected definition slots",
            ))
    }
}

/// Current Player.Diplomacy turn-start subscriber (2aa1db0). Unrelated players
/// return before processing; the empty canonical maps execute no promise or
/// incursion gameplay. Nonempty maps need their actual expiration/effect path.
pub fn diplomacy_turn_started(
    state: &LoadedState,
    receiver: i32,
    player: i32,
) -> Result<(), Error> {
    state
        .world()
        .player(receiver)
        .map_err(|_| Error::Unsupported("turn-start diplomacy receiver"))?;
    if receiver != player {
        return Ok(());
    }
    let diplomacy = &state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == receiver)
        .ok_or(Error::Unsupported("turn-start canonical diplomacy"))?
        .diplomacy;
    if !diplomacy.promises_made.is_empty() {
        return Err(Error::Unsupported("current promise processing"));
    }
    if !diplomacy.incursions_made.is_empty() {
        return Err(Error::Unsupported("current incursion expiration"));
    }
    Ok(())
}
