//! Linux visibility-count transitions over typed map visibility records.
//!
//! Own layers are save-backed; shared layers are separate live state and must
//! be exported explicitly. Resource discovery, plot contents and synchronous
//! propagation remain required effect providers, including their RNG/callbacks.
//! This module does not establish complete MoveTo or turn support.
use crate::map::{Visibility, VisibilityLayer};
use crate::unit::ComponentId;
use crate::unit_sight::VisibilityChange;

pub const BASE_LAYER_HASH: u32 = 0x1135_1026;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Group {
    Own,
    Shared,
}

/// The shared array is runtime state, absent from Visibility's save grammar.
/// Never initialize it by copying own counts: sharing can change every cell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedLayers(pub Vec<VisibilityLayer>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Propagation {
    pub player: i32,
    pub plot: u32,
    pub revealed_state: i32,
    pub mask: u8,
    pub first_reveal: bool,
    pub origin: i32,
    pub sight_range: i32,
    pub source: ComponentId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewContents {
    pub player: i32,
    pub plot: u32,
    pub origin: i32,
    pub first_reveal: bool,
    pub revealed_state: i32,
    pub source: ComponentId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateError {
    MissingBaseLayer,
    PlotOutOfRange,
    NoncanonicalRevealed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeError<E> {
    Context(E),
    State(StateError),
}

/// All providers are mandatory. Preflight must reject unavailable live state,
/// effect handlers or RNG before any mutation. Callers own transaction rollback
/// if an effect itself fails after preflight; failures are never parity passes.
pub trait Context {
    type Error;
    fn is_valid(&self, player: i32) -> Result<bool, Self::Error>;
    fn preflight(&self, change: VisibilityChange) -> Result<(), Self::Error>;
    fn visibility(&self, player: i32) -> Result<&Visibility, Self::Error>;
    fn visibility_mut(&mut self, player: i32) -> Result<&mut Visibility, Self::Error>;
    fn shared_layers(&self, player: i32) -> Result<&SharedLayers, Self::Error>;
    fn shared_layers_mut(&mut self, player: i32) -> Result<&mut SharedLayers, Self::Error>;
    /// Actual manager runtime enum-to-layer-ID mapping, not a saved hash cast.
    fn hidden_layer_hash(&self, hidden_type: i32) -> Result<u32, Self::Error>;
    fn messages_disabled(&mut self) -> Result<bool, Self::Error>;
    fn local_observer(&mut self) -> Result<i32, Self::Error>;
    /// Resource::Builder::UpdateRevealedAt, including all reachable effects.
    fn update_resource_revealed(&mut self, player: i32, plot: u32) -> Result<(), Self::Error>;
    /// Full synchronous PropagateVisibilityChanges mask-selected dispatch.
    fn propagate(&mut self, change: Propagation) -> Result<(), Self::Error>;
    /// Full ViewPlotContents: plot/unit contact, continent/wonder discovery,
    /// rewards, signals and exploration, including reachable gameplay RNG.
    fn view_plot_contents(&mut self, change: ViewContents) -> Result<(), Self::Error>;
}

fn entry(layers: &[VisibilityLayer], plot: u32) -> Result<i16, StateError> {
    layers
        .iter()
        .find(|l| l.type_hash == BASE_LAYER_HASH)
        .ok_or(StateError::MissingBaseLayer)?
        .values
        .get(plot as usize)
        .copied()
        .ok_or(StateError::PlotOutOfRange)
}

/// Native base count wraps at 16 bits, then the manager clamps negative results.
pub fn change_base_count(
    layers: &mut [VisibilityLayer],
    plot: u32,
    delta: i32,
) -> Result<i16, StateError> {
    let value = layers
        .iter_mut()
        .find(|l| l.type_hash == BASE_LAYER_HASH)
        .ok_or(StateError::MissingBaseLayer)?
        .values
        .get_mut(plot as usize)
        .ok_or(StateError::PlotOutOfRange)?;
    *value = value.wrapping_add(delta as i16).max(0);
    Ok(*value)
}

/// Hidden counts first sign-extend the truncated delta, add in i32 and clamp
/// before narrowing. Overflow can therefore leave a negative stored i16 while
/// reporting the *wide* sum's positive visibility. Missing layers are native
/// null entries and cause no mutation; they are never allocated here.
pub fn change_hidden_count(
    layers: &mut [VisibilityLayer],
    plot: u32,
    layer_hash: u32,
    delta: i32,
) -> Result<bool, StateError> {
    if delta == 0 {
        return Ok(false);
    }
    let Some(layer) = layers.iter_mut().find(|l| l.type_hash == layer_hash) else {
        return Ok(false);
    };
    let value = layer
        .values
        .get_mut(plot as usize)
        .ok_or(StateError::PlotOutOfRange)?;
    let before = *value > 0;
    let sum = i32::from(*value) + i32::from(delta as i16);
    *value = sum.max(0) as i16;
    Ok(before != (sum > 0))
}

fn revealed(v: &Visibility, plot: u32) -> Result<bool, StateError> {
    match v.revealed.get(plot as usize) {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        Some(_) => Err(StateError::NoncanonicalRevealed),
        None => Err(StateError::PlotOutOfRange),
    }
}

fn reveal(v: &mut Visibility, plot: u32) -> Result<(), StateError> {
    if !revealed(v, plot)? {
        v.revealed[plot as usize] = 1;
        v.cached_old = true;
    }
    Ok(())
}

fn state<C: Context>(c: &C, player: i32, plot: u32) -> Result<i32, ChangeError<C::Error>> {
    use ChangeError::{Context, State};
    if entry(&c.shared_layers(player).map_err(Context)?.0, plot).map_err(State)? > 0 {
        Ok(2)
    } else {
        Ok(i32::from(
            revealed(c.visibility(player).map_err(Context)?, plot).map_err(State)?,
        ))
    }
}

fn propagate<C: Context>(c: &mut C, event: Propagation) -> Result<(), ChangeError<C::Error>> {
    use ChangeError::Context;
    if !c.messages_disabled().map_err(Context)?
        && c.local_observer().map_err(Context)? == event.player
    {
        c.propagate(event).map_err(Context)?;
    }
    Ok(())
}

/// Manager::ChangeVisibilityCount. Return is the last transitioning recipient's
/// result in native outgoing-vector order, not a maximum across recipients.
/// Source_flag (the first native bool) is unused by the stock Linux method.
pub fn change_visibility_count<C: Context>(
    c: &mut C,
    change: VisibilityChange,
) -> Result<i32, ChangeError<C::Error>> {
    use ChangeError::{Context, State};
    if !c.is_valid(change.owner).map_err(Context)? {
        return Ok(0);
    }
    c.preflight(change).map_err(Context)?;
    // Retain native vector order and ignore PlayerTypeCount.count, just as the
    // original loop does. Blockers test entry presence, not its stored count.
    let recipients = c
        .visibility(change.owner)
        .map_err(Context)?
        .outgoing_visibility
        .clone();
    let mut result = 0;
    let mut origin = change.origin_index;
    for recipient in recipients {
        let player = recipient.player_id;
        let blocked = c.is_valid(player).map_err(Context)?
            && player != change.owner
            && c.visibility(change.owner)
                .map_err(Context)?
                .outgoing_visibility_blockers
                .iter()
                .any(|b| b.player_id == player);
        if blocked {
            continue;
        }
        if player == change.owner {
            change_base_count(
                &mut c.visibility_mut(player).map_err(Context)?.layers,
                change.plot_index,
                change.delta,
            )
            .map_err(State)?;
        }
        let before = entry(
            &c.shared_layers(player).map_err(Context)?.0,
            change.plot_index,
        )
        .map_err(State)?;
        let after = change_base_count(
            &mut c.shared_layers_mut(player).map_err(Context)?.0,
            change.plot_index,
            change.delta,
        )
        .map_err(State)?;
        let hidden_type = if change.near_origin {
            0
        } else {
            change.hidden_type
        };
        let mut hidden_changed = false;
        if hidden_type != -1 && change.delta != 0 {
            let hash = c.hidden_layer_hash(hidden_type).map_err(Context)?;
            if player == change.owner {
                hidden_changed = change_hidden_count(
                    &mut c.visibility_mut(player).map_err(Context)?.layers,
                    change.plot_index,
                    hash,
                    change.delta,
                )
                .map_err(State)?;
            }
            hidden_changed |= change_hidden_count(
                &mut c.shared_layers_mut(player).map_err(Context)?.0,
                change.plot_index,
                hash,
                change.delta,
            )
            .map_err(State)?;
        }
        let was_revealed =
            revealed(c.visibility(player).map_err(Context)?, change.plot_index).map_err(State)?;
        if origin < 0 {
            origin = change.plot_index as i32;
        }
        let event = Propagation {
            player,
            plot: change.plot_index,
            revealed_state: 0,
            mask: 0xff,
            first_reveal: !was_revealed,
            origin,
            sight_range: change.sight_range,
            source: change.source,
        };
        if (before > 0) != (after > 0) {
            if after > 0 {
                // Both native group setters share the same revealed array.
                reveal(
                    c.visibility_mut(player).map_err(Context)?,
                    change.plot_index,
                )
                .map_err(State)?;
                c.update_resource_revealed(player, change.plot_index)
                    .map_err(Context)?;
                propagate(
                    c,
                    Propagation {
                        revealed_state: 2,
                        ..event
                    },
                )?;
                result = 2;
                c.view_plot_contents(ViewContents {
                    player,
                    plot: change.plot_index,
                    origin,
                    first_reveal: !was_revealed,
                    revealed_state: 2,
                    source: change.source,
                })
                .map_err(Context)?;
            } else {
                result = 1;
                let revealed_state = state(c, player, change.plot_index)?;
                propagate(
                    c,
                    Propagation {
                        revealed_state,
                        first_reveal: false,
                        ..event
                    },
                )?;
            }
        } else {
            if hidden_changed {
                let revealed_state = state(c, player, change.plot_index)?;
                propagate(
                    c,
                    Propagation {
                        revealed_state,
                        mask: 0x80,
                        ..event
                    },
                )?;
            }
            if after == 0 && change.delta == 0 && !was_revealed {
                reveal(
                    c.visibility_mut(player).map_err(Context)?,
                    change.plot_index,
                )
                .map_err(State)?;
                let revealed_state = state(c, player, change.plot_index)?;
                propagate(
                    c,
                    Propagation {
                        revealed_state,
                        first_reveal: true,
                        ..event
                    },
                )?;
                c.view_plot_contents(ViewContents {
                    player,
                    plot: change.plot_index,
                    origin: -1,
                    first_reveal: true,
                    revealed_state,
                    source: change.source,
                })
                .map_err(Context)?;
                result = 1;
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::OutgoingVisibility;

    fn layer(hash: u32, value: i16) -> VisibilityLayer {
        VisibilityLayer {
            type_hash: hash,
            index: 0,
            count: 1,
            grid_width: 1,
            values: vec![value],
        }
    }

    struct Fixture {
        own: Vec<Visibility>,
        shared: Vec<SharedLayers>,
        events: Vec<String>,
        observer: i32,
        available: bool,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                own: (0..3)
                    .map(|player_id| Visibility {
                        player_id,
                        cached_num_revealed_hexes: 0,
                        cached_old: false,
                        revealed: vec![0],
                        layers: vec![layer(BASE_LAYER_HASH, 0), layer(123, 0)],
                        outgoing_visibility: vec![],
                        outgoing_visibility_blockers: vec![],
                    })
                    .collect(),
                shared: (0..3)
                    .map(|_| SharedLayers(vec![layer(BASE_LAYER_HASH, 0), layer(123, 0)]))
                    .collect(),
                events: vec![],
                observer: 0,
                available: true,
            }
        }
        fn recipients(&mut self, players: &[i32]) {
            self.own[0].outgoing_visibility = players
                .iter()
                .map(|&player_id| OutgoingVisibility {
                    player_id,
                    count: 0,
                })
                .collect();
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn is_valid(&self, player: i32) -> Result<bool, Self::Error> {
            Ok((0..3).contains(&player))
        }
        fn preflight(&self, _: VisibilityChange) -> Result<(), Self::Error> {
            if self.available {
                Ok(())
            } else {
                Err("effects unavailable")
            }
        }
        fn visibility(&self, p: i32) -> Result<&Visibility, Self::Error> {
            self.own.get(p as usize).ok_or("player")
        }
        fn visibility_mut(&mut self, p: i32) -> Result<&mut Visibility, Self::Error> {
            self.own.get_mut(p as usize).ok_or("player")
        }
        fn shared_layers(&self, p: i32) -> Result<&SharedLayers, Self::Error> {
            self.shared.get(p as usize).ok_or("shared")
        }
        fn shared_layers_mut(&mut self, p: i32) -> Result<&mut SharedLayers, Self::Error> {
            self.shared.get_mut(p as usize).ok_or("shared")
        }
        fn hidden_layer_hash(&self, hidden: i32) -> Result<u32, Self::Error> {
            if hidden == 0 {
                Ok(123)
            } else {
                Err("hidden definition")
            }
        }
        fn messages_disabled(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn local_observer(&mut self) -> Result<i32, Self::Error> {
            Ok(self.observer)
        }
        fn update_resource_revealed(&mut self, p: i32, _: u32) -> Result<(), Self::Error> {
            self.events.push(format!("resource {p}"));
            Ok(())
        }
        fn propagate(&mut self, e: Propagation) -> Result<(), Self::Error> {
            self.events.push(format!(
                "propagate {} {} {} {}",
                e.player, e.revealed_state, e.mask, e.first_reveal
            ));
            Ok(())
        }
        fn view_plot_contents(&mut self, e: ViewContents) -> Result<(), Self::Error> {
            self.events.push(format!(
                "view {} {} {} {}",
                e.player, e.origin, e.revealed_state, e.first_reveal
            ));
            Ok(())
        }
    }
    fn request(delta: i32) -> VisibilityChange {
        VisibilityChange {
            owner: 0,
            plot_index: 0,
            delta,
            hidden_type: -1,
            source_flag: true,
            near_origin: false,
            origin_index: -1,
            sight_range: 2,
            source: ComponentId {
                owner: 0,
                component_type: 0,
                id: 7,
            },
        }
    }

    #[test]
    fn discovery_shares_counts_and_orders_effects() {
        let mut f = Fixture::new();
        f.recipients(&[1, 0, 2]);
        // Presence blocks recipient 2 despite zero count; self is never blocked.
        f.own[0].outgoing_visibility_blockers = vec![
            OutgoingVisibility {
                player_id: 2,
                count: 0,
            },
            OutgoingVisibility {
                player_id: 0,
                count: 1,
            },
        ];
        assert_eq!(change_visibility_count(&mut f, request(1)), Ok(2));
        assert_eq!(f.own[0].layers[0].values, [1]);
        assert_eq!(f.own[1].layers[0].values, [0]);
        assert_eq!(f.shared[1].0[0].values, [1]);
        assert_eq!(f.shared[2].0[0].values, [0]);
        assert_eq!(f.own[0].revealed, [1]);
        assert!(f.own[0].cached_old);
        assert_eq!(f.own[0].cached_num_revealed_hexes, 0);
        assert_eq!(
            f.events,
            [
                "resource 1",
                "view 1 0 2 true",
                "resource 0",
                "propagate 0 2 255 true",
                "view 0 0 2 true"
            ]
        );
    }

    #[test]
    fn removal_keeps_reveal_and_does_not_view_or_update_resources() {
        let mut f = Fixture::new();
        f.recipients(&[0]);
        f.own[0].revealed[0] = 1;
        f.own[0].layers[0].values[0] = 1;
        f.shared[0].0[0].values[0] = 1;
        assert_eq!(change_visibility_count(&mut f, request(-1)), Ok(1));
        assert_eq!(f.own[0].revealed, [1]);
        assert_eq!(f.events, ["propagate 0 1 255 false"]);
    }

    #[test]
    fn zero_delta_reveals_without_resource_update() {
        let mut f = Fixture::new();
        f.recipients(&[0]);
        assert_eq!(change_visibility_count(&mut f, request(0)), Ok(1));
        assert_eq!(f.shared[0].0[0].values, [0]);
        assert_eq!(f.own[0].revealed, [1]);
        assert_eq!(f.events, ["propagate 0 1 255 true", "view 0 -1 1 true"]);
    }

    #[test]
    fn later_recipient_overwrites_discovery_result_with_loss() {
        let mut f = Fixture::new();
        f.recipients(&[0, 1]);
        // Overflow is a loss of visibility after signed16 wrapping/clamping.
        f.shared[1].0[0].values[0] = i16::MAX;
        assert_eq!(change_visibility_count(&mut f, request(1)), Ok(1));
        assert_eq!(f.shared[1].0[0].values[0], 0);
        assert_eq!(f.own[0].revealed[0], 1);
    }

    #[test]
    fn near_origin_selects_hidden_zero_and_propagates_hidden_only_changes() {
        let mut f = Fixture::new();
        f.recipients(&[0]);
        f.own[0].revealed[0] = 1;
        f.shared[0].0[0].values[0] = 1;
        let mut r = request(1);
        r.near_origin = true;
        assert_eq!(change_visibility_count(&mut f, r), Ok(0));
        assert_eq!(f.own[0].layers[1].values, [1]);
        assert_eq!(f.shared[0].0[1].values, [1]);
        assert_eq!(f.events, ["propagate 0 2 128 false"]);
    }

    #[test]
    fn hidden_overflow_uses_wide_sum_for_transition() {
        let mut layers = vec![layer(123, i16::MAX)];
        assert_eq!(change_hidden_count(&mut layers, 0, 123, 1), Ok(false));
        assert_eq!(layers[0].values, [i16::MIN]);
        assert_eq!(change_hidden_count(&mut layers, 0, 123, 1), Ok(false));
        assert_eq!(layers[0].values, [0]);
        assert_eq!(change_hidden_count(&mut layers, 0, 999, 1), Ok(false));
    }

    #[test]
    fn unavailable_effects_fail_before_mutating() {
        let mut f = Fixture::new();
        f.recipients(&[0]);
        f.available = false;
        let own = f.own.clone();
        let shared = f.shared.clone();
        assert_eq!(
            change_visibility_count(&mut f, request(1)),
            Err(ChangeError::Context("effects unavailable"))
        );
        assert_eq!(f.own, own);
        assert_eq!(f.shared, shared);
        assert!(f.events.is_empty());
    }
}
