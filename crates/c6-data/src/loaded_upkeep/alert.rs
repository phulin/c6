//! Read-only stock Alert enemy scan over current typed state.
use super::*;
use c6_core::visibility_change::SharedLayers;

/// Current live group1 state, shared with visibility-count mutation providers.
/// Saved group0 layers do not implement this authority. Providers borrow their
/// owned current arrays after callbacks, without unrelated World revision gates.
pub trait AlertVisibility {
    type Error;
    fn shared_layers(&self, state: &LoadedState, owner: i32) -> Result<&SharedLayers, Self::Error>;
    fn grid_width(&self, state: &LoadedState, owner: i32) -> Result<u32, Self::Error>;
    /// Actual manager hidden enum0 -> layer hash, read only on the hidden branch.
    fn hidden_layer0(&self, state: &LoadedState) -> Result<u32, Self::Error>;
}

impl<C> AlertVisibility for RulesEffects<'_, C>
where
    C: Continuations + AlertVisibility<Error = <C as Continuations>::Error>,
{
    type Error = RulesEffectError<<C as Continuations>::Error>;
    fn shared_layers(&self, state: &LoadedState, owner: i32) -> Result<&SharedLayers, Self::Error> {
        self.continuations
            .shared_layers(state, owner)
            .map_err(RulesEffectError::Continuation)
    }
    fn hidden_layer0(&self, state: &LoadedState) -> Result<u32, Self::Error> {
        self.continuations
            .hidden_layer0(state)
            .map_err(RulesEffectError::Continuation)
    }
    fn grid_width(&self, state: &LoadedState, owner: i32) -> Result<u32, Self::Error> {
        self.continuations
            .grid_width(state, owner)
            .map_err(RulesEffectError::Continuation)
    }
}

/// Original Alert::IsEnemyVisible: copy the current sight entries, then scan
/// layer0 plot occupancy in stored order. Visibility precedes delayed-death,
/// owner and war predicates, including for friendly Units. No actions, scripts,
/// callbacks or RNG draws occur in this source-derived predicate.
pub fn alert_enemy_visible<E: AlertVisibility>(
    state: &LoadedState,
    actor: &Unit,
    definitions: &crate::formation_rules::Definitions,
    visibility: &E,
) -> Result<bool, Error<E::Error>> {
    definitions.check(state).map_err(Error::Rules)?;
    let sight = actor.k_sight_plots.clone();
    let owner = actor.e_owner;
    let present = crate::movement_placement_state::visibility_context_present(
        &state.world().map().visibility,
        state.world().player_mapping.as_ref(),
        owner,
    )
    .map_err(|_| Error::Unsupported("Alert owner visibility context"))?;
    for entry in sight {
        let index = entry.ui_index as usize;
        if state.world().map().plots.get(index).is_none() {
            continue;
        }
        for reference in state.world().plot_units(0, index).map_err(Error::World)? {
            let candidate = state.world().unit(*reference).map_err(Error::World)?;
            if !present {
                continue;
            }
            let Some(candidate) = candidate else {
                continue;
            };
            let unit = candidate.record;
            let visible = definitions.unit_visible_with_counts(
                state,
                unit,
                owner,
                |hash, normal| {
                    let shared = visibility
                        .shared_layers(state, owner)
                        .map_err(Error::Provider)?;
                    let Some(layer) = shared.0.iter().find(|layer| layer.type_hash == hash) else {
                        return if normal {
                            Err(Error::Unsupported(
                                "Alert initialized shared normal visibility layer",
                            ))
                        } else {
                            Ok(0)
                        };
                    };
                    let index = visibility
                        .grid_width(state, owner)
                        .map_err(Error::Provider)?
                        .wrapping_mul(unit.k_location.y as u32)
                        .wrapping_add(unit.k_location.x as u32)
                        as usize;
                    Ok(layer.values.get(index).copied().unwrap_or(0))
                },
                || visibility.hidden_layer0(state).map_err(Error::Provider),
            )?;
            if !visible || unit.b_delayed_death || unit.e_owner == owner {
                continue;
            }
            if state
                .game_relations()
                .map_err(Error::State)?
                .is_at_war(owner, unit.e_owner)
                .map_err(|_| Error::Unsupported("Alert current war relation"))?
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
