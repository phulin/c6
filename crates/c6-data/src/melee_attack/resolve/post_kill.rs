//! Current loyalty/religious tail after kill bonuses and healing. Positive
//! synchronous continuations remain required; ordinary zero branches compute
//! from the canonical current Units, Players and owning City.
use super::{
    kill_awards::{Kill, RequiresKillEffects},
    GeneratedMembers,
};
use crate::{
    unit_placement_services as services, unit_placement_state as placement,
    unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::PlotCoord};
type ProviderError<E> = <E as sight::Effects>::Error;
type City = (i32, u32);
#[derive(Debug)]
pub enum Error<E> {
    Actor(Box<super::super::Error>),
    State(c6_core::loaded_state::Error),
    Maximum(crate::formation_movement_cost::Error),
    Services(services::Error<E>),
    Binding(crate::loaded_actions::Error),
    Age(c6_core::loaded_state::game_eras::AccessError),
    Effect(E),
    Missing(&'static str),
}
#[must_use = "second proposal, post-combat yields and outer Resolve are still required"]
pub struct RequiresPostCombatProposal {
    pub members: GeneratedMembers,
    pub kill: Option<Kill>,
    /// False is the source fresh-both-alive return, before the second proposal.
    pub needs_proposal: bool,
}
pub struct Inputs<'q, 'a, 'r> {
    pub placement: &'q placement::Definitions<'a, 'r>,
    pub civilization_levels: &'q [(i32, u32)],
    /// The actual Results combat type supplied by the source-owned caller.
    pub combat_type: u32,
    /// Admitted LOYALTY_MAXIMUM, read only when ChangeLoyalty receives nonzero Q8.
    pub loyalty_maximum: Option<i32>,
}
#[derive(Clone, Copy, Debug)]
pub enum LoyaltyContinuation {
    /// GameSignals+348, after the transfer-to scalar write.
    ChangedSignal,
    /// Reporting.Event 89de2474 with the fresh current City ComponentID.
    ChangedReport,
    RecalculateConversionTrends,
}
/// These are the actual reached downstream calls, never success defaults.
pub trait Effects: placement::Effects {
    fn potential_transfer_player(
        &mut self,
        _state: &LoadedState,
        _city: City,
    ) -> Result<i32, Error<ProviderError<Self>>> {
        Err(Error::Missing(
            "City.CulturalIdentity.GetPotentialTransferPlayer",
        ))
    }
    fn loyalty_continuation(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _city: City,
        _step: LoyaltyContinuation,
    ) -> Result<(), Error<ProviderError<Self>>> {
        Err(Error::Missing("City loyalty synchronous continuation"))
    }
    /// Native visibility gating and reward report45b517d6; only nonzero loyalty
    /// reaches this continuation, after ChangeLoyalty and all its callbacks.
    fn loyalty_feedback(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _winner: UnitReference,
        _city: City,
        _cached_location: PlotCoord,
        _amount: i32,
    ) -> Result<(), Error<ProviderError<Self>>> {
        Err(Error::Missing("post-combat loyalty feedback"))
    }
    /// ApplyReligiousCombatImpact uses CURRENT religions/fallbacks of these
    /// retained objects and the captured pre-GP location. No saved hash may be
    /// passed as a native religion ordinal. This positive kernel is unbound.
    fn religious_combat_impact(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _kill: Kill,
    ) -> Result<(), Error<ProviderError<Self>>> {
        Err(Error::Missing("Player.Religion.ApplyReligiousCombatImpact"))
    }
}
fn actor<E>(state: &LoadedState, unit: UnitReference) -> Result<&c6_core::unit::Unit, Error<E>> {
    super::super::actor(state, unit).map_err(|e| Error::Actor(Box::new(e)))
}
fn dead<E>(
    state: &LoadedState,
    inputs: &Inputs<'_, '_, '_>,
    unit: UnitReference,
) -> Result<bool, Error<E>> {
    let unit = actor(state, unit)?;
    Ok(unit.i_damage
        >= inputs
            .placement
            .movement
            .maximum_damage(unit)
            .map_err(Error::Maximum)?)
}
fn loyalty_bonus(base: i32, golden: bool, additional: impl FnOnce() -> i32) -> i32 {
    if base != 0 && golden {
        base.wrapping_add(additional())
    } else {
        base
    }
}
fn capped_loyalty(value: i32, maximum: i32) -> i32 {
    let maximum = maximum.wrapping_shl(8);
    let value = if value >= maximum { maximum } else { value };
    if value <= 0 {
        0
    } else {
        value
    }
}
/// City.ChangeLoyalty28747e0: exact scalar prefix and required synchronous
/// signal/report/recalculation order. Each continuation reads the same state.
pub fn change_loyalty<E: Effects>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    city: City,
    delta_raw_q8: i32,
    maximum: Option<i32>,
) -> Result<(), Error<ProviderError<E>>> {
    if delta_raw_q8 == 0 {
        return Ok(());
    }
    let current = state
        .city(city.0, city.1)
        .map_err(Error::State)?
        .cultural_identity
        .loyalty_raw_q8;
    {
        let identity = &mut state
            .city_mut(city.0, city.1)
            .map_err(Error::State)?
            .cultural_identity;
        identity.loyalty_raw_q8 = current.wrapping_add(delta_raw_q8);
        identity.last_loyalty_change_raw_q8 = delta_raw_q8;
    }
    let maximum = maximum.ok_or(Error::Missing("admitted LOYALTY_MAXIMUM"))?;
    let value = state
        .city(city.0, city.1)
        .map_err(Error::State)?
        .cultural_identity
        .loyalty_raw_q8;
    let value = capped_loyalty(value, maximum);
    state
        .city_mut(city.0, city.1)
        .map_err(Error::State)?
        .cultural_identity
        .loyalty_raw_q8 = value;
    let transfer = if value <= 0 {
        effects.potential_transfer_player(state, city)?
    } else {
        -1
    };
    state
        .city_mut(city.0, city.1)
        .map_err(Error::State)?
        .cultural_identity
        .transfer_to = transfer;
    effects.loyalty_continuation(state, sight, city, LoyaltyContinuation::ChangedSignal)?;
    effects.loyalty_continuation(state, sight, city, LoyaltyContinuation::ChangedReport)?;
    effects.loyalty_continuation(
        state,
        sight,
        city,
        LoyaltyContinuation::RecalculateConversionTrends,
    )
}
/// Ordinary unit/unit AwardPostCombatEffects28cb08c..28cb651. A City is looked
/// up at the winner's CURRENT location after all GP/healing callbacks; feedback
/// and religious impact still use the captured pre-callback location.
pub fn apply<E: Effects>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut E,
    inputs: &Inputs<'_, '_, '_>,
    pending: RequiresKillEffects,
) -> Result<RequiresPostCombatProposal, Error<ProviderError<E>>> {
    inputs
        .placement
        .validate_binding(state)
        .map_err(Error::Binding)?;
    let RequiresKillEffects { members, kill } = pending;
    if let Some(kill) = kill {
        let current_location = actor(state, kill.winner)?.k_location;
        let plot = services::plot_index_at::<ProviderError<E>>(state, current_location)
            .map_err(Error::Services)?;
        // The dead-attacker branch checks GetPlot for null. The dead-defender
        // branch dereferences it, so an invalid winner plot cannot be admitted.
        if plot.is_none() && kill.loser != members.damage.attacker {
            return Err(Error::Missing("current surviving attacker plot"));
        }
        if let Some(plot) = plot {
            if let Some(city) =
                services::owning_city(state, effects, plot).map_err(Error::Services)?
            {
                let winner_owner = actor(state, kill.winner)?.e_owner;
                let base = if city.0 == winner_owner {
                    state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == kill.winner.player())
                        .ok_or(Error::Missing("cached winner Player"))?
                        .cultural_identity
                        .post_combat_local_loyalty
                } else if city.0 == actor(state, kill.loser)?.e_owner {
                    state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == kill.winner.player())
                        .ok_or(Error::Missing("cached winner Player"))?
                        .cultural_identity
                        .post_combat_foreign_loyalty
                } else {
                    0
                };
                // Source queries the LOSER's global GoldenAge even for base0.
                let golden = state
                    .player_has_golden_age(kill.loser.player())
                    .map_err(Error::Age)?;
                let amount = if base != 0 && golden {
                    let additional = state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == kill.winner.player())
                        .ok_or(Error::Missing("cached winner Player"))?
                        .cultural_identity
                        .additional_golden_age_post_combat_loyalty;
                    loyalty_bonus(base, true, || additional)
                } else {
                    base
                };
                if amount != 0 {
                    change_loyalty(
                        state,
                        sight,
                        effects,
                        city,
                        amount.wrapping_shl(8),
                        inputs.loyalty_maximum,
                    )?;
                    effects.loyalty_feedback(
                        state,
                        sight,
                        kill.winner,
                        city,
                        kill.location,
                        amount,
                    )?;
                }
            }
        }
    }
    // Fresh source IsDead order: member0(attacker), then member1(defender).
    let needs_proposal = dead(state, inputs, members.damage.attacker)?
        || dead(state, inputs, members.damage.defender)?;
    if needs_proposal {
        let kill = kill.ok_or(Error::Missing("fresh death without retained kill objects"))?;
        // The source reads member1/DEFENDER strength, not the cached loser bonus.
        let religious = inputs.combat_type == 0x5e2f7468
            || actor(state, members.damage.defender)?
                .k_religion
                .i_religious_strength
                > 0
            || actor(state, kill.winner)?.k_religion.i_land_victory_spread > 0;
        if religious {
            let owner = actor(state, kill.loser)?.e_owner;
            let level = inputs
                .civilization_levels
                .iter()
                .find(|row| row.0 == owner)
                .ok_or(Error::Missing("current loser civilization level"))?
                .1;
            if level == 0x253718b0 || level == 0xc2577075 {
                effects.religious_combat_impact(state, sight, kill)?;
            }
        }
    }
    Ok(RequiresPostCombatProposal {
        members,
        kill,
        needs_proposal,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn golden_bonus_is_lazy_and_wraps_before_q8_conversion() {
        assert_eq!(
            loyalty_bonus(0, true, || panic!("zero skips additional")),
            0
        );
        assert_eq!(
            loyalty_bonus(7, false, || panic!("no golden skips additional")),
            7
        );
        assert_eq!(loyalty_bonus(i32::MAX, true, || 1), i32::MIN);
    }
    #[test]
    fn loyalty_cap_preserves_native_signed_order_after_wrapping_add() {
        assert_eq!(capped_loyalty(101 * 256, 100), 100 * 256);
        assert_eq!(capped_loyalty(-1, 100), 0);
        assert_eq!(capped_loyalty(i32::MAX.wrapping_add(256), 100), 0);
        assert_eq!(capped_loyalty(5, -1), 0);
    }
}
