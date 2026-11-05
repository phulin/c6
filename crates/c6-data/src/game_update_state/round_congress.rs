//! Congress.DoTurn2c4a600: favor accrual, elapsed counters and phase gates.
pub mod favor;
use super::activation::cityless_managers;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    game_capabilities::Membership,
    game_update::Error,
    loaded_state::{game_update::round, LoadedState},
};

pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    initial_era: i32,
    segments: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("Congress GlobalParameters"))?;
        let initial_era = parameters
            .rows()
            .find(|r| r.text("Name").ok() == Some("WORLD_CONGRESS_INITIAL_ERA"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Unsupported("WORLD_CONGRESS_INITIAL_ERA"))?;
        let segments = collection(rules, "TurnSegments", "TurnSegmentType")
            .map_err(|_| Error::Unsupported("Congress TurnSegments"))?;
        Ok(Self {
            binding: state.binding().clone(),
            initial_era,
            segments,
        })
    }
}
/// Stored at Manager+c0 and +108, distinct from the shared last-special-session
/// field used by the activation callback. Both wrap once after AwardFavor.
pub struct Current {
    pub normal_session_elapsed: i32,
    pub established_turns: i32,
}
pub trait Effects {
    fn favor_per_turn(&mut self, state: &LoadedState, owner: i32) -> Result<i32, Error>;
    fn favor_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        amount: i32,
        reason: i32,
    ) -> Result<(), Error>;
}

/// ChangeDiplomaticFavor2a667f0 clamps the current balance after wrapping add,
/// increments lifetime only for positive changes, and ALWAYS sends its report.
pub fn change_favor(
    state: &mut LoadedState,
    owner: i32,
    delta: i32,
    effects: &mut impl Effects,
) -> Result<(), Error> {
    let favor = &mut state
        .activation_managers_mut(owner)
        .map_err(|_| Error::Unsupported("current Congress player"))?
        .congress
        .favor;
    favor.player = favor.player.wrapping_add(delta).max(0);
    if delta > 0 {
        favor.lifetime = favor.lifetime.wrapping_add(delta);
    }
    effects.favor_changed(state, owner, delta, -1)
}
fn disabled(state: &mut LoadedState, definitions: &Definitions, hash: u32) -> Result<(), Error> {
    let index = definitions
        .segments
        .find_index(hash)
        .map(|v| v as i32)
        .unwrap_or(-1);
    round::set_segment_disabled(state, hash, index, true)
}
pub fn run(
    state: &mut LoadedState,
    definitions: &Definitions,
    managers: &mut cityless_managers::Runtime,
    capabilities: &Membership,
    alive_major_ids: &[i32],
    effects: &mut impl Effects,
) -> Result<(), Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Unsupported("Congress round binding"));
    }
    if capabilities.has(0x7237dec9) {
        let era = state
            .game_eras()
            .map_err(|_| Error::Unsupported("current Congress era"))?
            .current_era()
            .ok_or(Error::Unsupported("current Congress era index"))?;
        for &owner in alive_major_ids {
            let favor = effects.favor_per_turn(state, owner)?;
            change_favor(state, owner, favor, effects)?;
        }
        let current = managers
            .congress_round
            .as_mut()
            .ok_or(Error::Unsupported("current Congress elapsed counters"))?;
        current.normal_session_elapsed = current.normal_session_elapsed.wrapping_add(1);
        current.established_turns = current.established_turns.wrapping_add(1);
        if era >= definitions.initial_era {
            return Err(Error::Unsupported(
                "Congress current discussions/session continuation",
            ));
        }
    }
    // Disabled capability and the pre-initial-era branch both disable all
    // three Congress segments, in this exact source order.
    for hash in [0xb52a667b, 0x2c2337c1, 0xd74f83e5] {
        disabled(state, definitions, hash)?;
    }
    Ok(())
}
