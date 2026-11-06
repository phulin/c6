//! Targets' PlayersMet subscriber initializes current logical knowledge IDs.
use super::Error;
use c6_core::loaded_state::LoadedState;
pub fn on_met(
    state: &mut LoadedState,
    receiver: i32,
    first: i32,
    second: i32,
) -> Result<(), Error> {
    let other = if receiver == first {
        second
    } else if receiver == second {
        first
    } else {
        return Ok(());
    };
    let mapping = state
        .world()
        .player_mapping
        .as_ref()
        .ok_or(Error::Missing("first-contact current PlayerToIndex"))?;
    let index = *mapping
        .get(usize::try_from(other).map_err(|_| Error::Missing("first-contact knowledge player"))?)
        .ok_or(Error::Missing("first-contact knowledge player"))?;
    let index = usize::try_from(index)
        .map_err(|_| Error::Missing("first-contact negative knowledge mapping"))?;
    let targets = state
        .combat_ai_targets_mut(receiver)
        .map_err(|e| Error::Requirement(format!("first-contact knowledge: {e:?}")))?;
    if targets.player_id != receiver || targets.knowledge_base.owner != receiver {
        return Err(Error::Missing("first-contact knowledge parent"));
    }
    if let Some(knowledge) = targets.knowledge_base.player_knowledge.get_mut(index) {
        knowledge.player_id = other;
        targets.knowledge_base.trade_knowledge.owner = other;
    }
    Ok(())
}
