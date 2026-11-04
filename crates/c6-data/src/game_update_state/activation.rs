//! Actual selected phase TurnMode rules for current next-player selection.
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
        Ok(Self { modes })
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
