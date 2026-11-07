//! Linux experience-report queries and semantic headless output.
//!
//! These bounded queries follow the pinned Linux XP2 source. They do not
//! establish complete combat or reporting parity; the caller qualifies reached
//! native consumers before replacing presentation dispatch with these events.
use crate::formation_movement_cost;
use c6_core::{loaded_state::LoadedState, map::UnitReference};

#[derive(Debug)]
pub enum Error {
    Actor(crate::melee_attack::Error),
    World(c6_core::world::MissingInput),
    Maximum(formation_movement_cost::Error),
    Unsupported(&'static str),
    Interruption(c6_core::unit_operation_interruption::Error),
    Promotion(crate::loaded_actions::promotion_actions::Error<()>),
}

/// Semantic output after the caller has executed the reached gameplay effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Gained {
        unit: UnitReference,
        amount: i32,
        promotable: bool,
    },
    Changed {
        unit: UnitReference,
    },
    /// Host UnitPromoted79 announces availability after XP changes. It is
    /// presentation output, separate from actually earning a chosen promotion.
    PromotionAvailable {
        unit: UnitReference,
    },
}

/// Reports::CanSeePlayer (290ba60), for a stable current local observer.
/// The original performs a second observer lookup after a failed owner match;
/// callers must supply the same authoritative observer for both reads.
pub fn observer_can_see_player(observer: i32, owner: i32) -> bool {
    observer == owner || observer == 1000
}

/// The Promote availability query reached by the experience-changed translator
/// (320cee0): CanStart with visible-only true, empty parameters and nonnull
/// results. Manager (2d8b7b0) therefore skips phase, turn and pause guards.
/// The caller supplies maximum-damage definitions admitted for this state's
/// target and rules binding.
///
/// A nonempty operation queue reaches IsInterruptible before the death guards;
/// that branch requires a qualified interruption provider. A movable,
/// promotable unit with no pending operations reaches promotion-definition
/// enumeration (2d8fb1f). The rules-bound variant below supplies it; this legacy
/// maximum-only provider reports that missing dependency explicitly.
pub fn promotion_available_after_change(
    state: &LoadedState,
    maximum: &formation_movement_cost::Definitions,
    unit: UnitReference,
) -> Result<bool, Error> {
    promotion_available_after_change_with_interruption(state, maximum, unit, |_| {
        Err(Error::Unsupported(
            "Promote CanStart operation queue IsInterruptible",
        ))
    })
}

/// The manager invokes this provider only for a nonempty current operation
/// queue, with the requested operation ID -1. The provider must resolve the
/// actual front runtime ID and source-qualified current handler; a wire hash
/// alone does not establish its virtual interruption behavior.
pub fn promotion_available_after_change_with_interruption(
    state: &LoadedState,
    maximum: &formation_movement_cost::Definitions,
    unit: UnitReference,
    interruptible: impl FnOnce(&c6_core::unit::Unit) -> Result<bool, Error>,
) -> Result<bool, Error> {
    promotion_available_with_choices(state, maximum, unit, interruptible, |_| {
        Err(Error::Unsupported(
            "Promote CanStart promotion enumeration and CanEarnPromotion results",
        ))
    })
}

/// The positive translator continuation uses the same current rule-driven
/// eligibility as the explicit Session promotion action. Its nonnull result
/// collection contains every earnable promotion, preserving definition order.
pub fn promotion_available_after_change_with_promotions(
    state: &LoadedState,
    maximum: &formation_movement_cost::Definitions,
    unit: UnitReference,
    definitions: &crate::loaded_actions::promotion_actions::Definitions,
    interruptible: impl FnOnce(&c6_core::unit::Unit) -> Result<bool, Error>,
) -> Result<bool, Error> {
    promotion_available_with_choices(state, maximum, unit, interruptible, |current| {
        crate::loaded_actions::promotion_actions::earnable_choices(
            state,
            definitions,
            (current.e_owner, current.id),
        )
        .map(|choices| !choices.is_empty())
        .map_err(Error::Promotion)
    })
}

fn promotion_available_with_choices(
    state: &LoadedState,
    maximum: &formation_movement_cost::Definitions,
    unit: UnitReference,
    interruptible: impl FnOnce(&c6_core::unit::Unit) -> Result<bool, Error>,
    choices: impl FnOnce(&c6_core::unit::Unit) -> Result<bool, Error>,
) -> Result<bool, Error> {
    let current = crate::melee_attack::actor(state, unit).map_err(Error::Actor)?;
    state
        .world()
        .player(current.e_owner)
        .map_err(Error::World)?;
    if !current.k_operations.nodes.is_empty() && !interruptible(current)? {
        return Ok(false);
    }
    if current.i_damage >= maximum.maximum_damage(current).map_err(Error::Maximum)? {
        return Ok(false);
    }
    if current.b_delayed_death {
        return Ok(false);
    }
    // Promote::CanStart (2d8fac0) evaluates these in this lazy order.
    if !c6_core::skip_turn::legality::can_move(current) {
        return Ok(false);
    }
    // Experience::CanPromote (2c181c0) reads the current +8 flag directly.
    if !current.k_experience.b_can_promote {
        return Ok(false);
    }
    // HasPendingOperations (2702cd0) tests this same queue's current size.
    if !current.k_operations.nodes.is_empty() {
        return Ok(false);
    }
    choices(current)
}
