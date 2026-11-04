//! Unit sight over the owned gameplay state and separately supplied live sharing.
//! This connects raycasting, count mutation and discovery dispatch; it is not a
//! complete MoveTo action. Runtime shared layers are never inferred from saves.
use crate::loaded_actions::city_range_attack;
use c6_core::{
    loaded_state::{self, LoadedState},
    map::Visibility,
    unit::{ComponentId, PlotCoord, SightPlotEntry, Unit},
    unit_sight::{self, SightContext, VisibilityChange},
    visibility_change::{self, Propagation, SharedLayers, ViewContents},
    visibility_contents, visibility_propagation, world,
};

/// Native-order owner records. Duplicate owners are rejected rather than merged.
pub struct SharedVisibility {
    pub owner: i32,
    pub layers: SharedLayers,
}
pub struct Runtime {
    pub shared: Vec<SharedVisibility>,
    /// Actual manager enum-index to definition-ID mapping.
    pub hidden_layer_hashes: Vec<u32>,
    pub messages_disabled: bool,
    /// Unknown is valid only while no observer-dependent effect is reached.
    pub local_observer: Option<i32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlotVisibilityChanged {
    pub plot: u32,
    pub player: i32,
    pub source: ComponentId,
    pub other_player: i32,
    pub first_reveal: bool,
}
impl Runtime {
    /// Checked actual live group1 storage; never reconstructed from group0.
    pub fn shared(&self, owner: i32) -> Result<&SharedLayers, &'static str> {
        let mut rows = self.shared.iter().filter(|r| r.owner == owner);
        let value = rows.next().ok_or("missing live shared visibility")?;
        if rows.next().is_some() {
            return Err("duplicate live shared visibility owner");
        }
        Ok(&value.layers)
    }
    fn shared_mut(&mut self, owner: i32) -> Result<&mut SharedLayers, &'static str> {
        self.shared(owner)?;
        Ok(&mut self
            .shared
            .iter_mut()
            .find(|r| r.owner == owner)
            .unwrap()
            .layers)
    }
}

#[derive(Debug)]
pub enum Error<E> {
    Loaded(loaded_state::Error),
    World(world::MissingInput),
    Query(crate::loaded_actions::Error),
    Missing(&'static str),
    Visibility(visibility_change::StateError),
    ContentsMissingPlot(u32),
    Effect(E),
    Propagation(visibility_propagation::Error<E>),
}
impl<E> From<crate::loaded_actions::Error> for Error<E> {
    fn from(error: crate::loaded_actions::Error) -> Self {
        Self::Query(error)
    }
}

/// Live effects operate on the SAME state and sharing arrays as the transition.
/// They must run full reached gameplay callbacks/RNG, including contact/rewards.
/// No default implementations silently discard discovery or propagation.
pub trait Effects {
    type Error;
    type Contents<'a>: visibility_contents::Context<Error = Self::Error>
    where
        Self: 'a;
    fn preflight_sight(
        &self,
        state: &LoadedState,
        runtime: &Runtime,
        actor: (i32, u32),
        add: bool,
    ) -> Result<(), Self::Error>;
    fn preflight_visibility(
        &self,
        state: &LoadedState,
        runtime: &Runtime,
        change: VisibilityChange,
    ) -> Result<(), Self::Error>;
    fn water(&mut self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error>;
    /// Native feature definition index (not the saved definition hash).
    fn feature(&mut self, state: &LoadedState, plot: u32) -> Result<i32, Self::Error>;
    fn revealed_feature(
        &mut self,
        state: &LoadedState,
        player: i32,
        feature: i32,
    ) -> Result<bool, Self::Error>;
    fn promotion_class(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<Option<&str>, Self::Error>;
    fn natural_wonder_experience(&mut self, state: &LoadedState) -> Result<i32, Self::Error>;
    fn experience_modifier(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
    ) -> Result<i32, Self::Error>;
    fn change_experience(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        actor: (i32, u32),
        amount: i32,
    ) -> Result<(), Self::Error>;
    fn update_resource_revealed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        player: i32,
        plot: u32,
    ) -> Result<(), Self::Error>;
    fn preflight_hidden_propagation(
        &self,
        state: &LoadedState,
        runtime: &Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error>;
    /// Actual Core block stack and qualified stock EngineUtility block methods.
    fn visibility_reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error>;
    fn unit_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        event: visibility_propagation::UnitVisibilityChanged,
    ) -> Result<(), Self::Error>;
    fn propagate(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        event: Propagation,
    ) -> Result<(), Self::Error>;
    /// Borrow the live discovery/contact/reward providers. The transition calls
    /// the concrete stock ViewPlotContents implementation itself.
    fn contents<'a>(
        &'a mut self,
        state: &'a mut LoadedState,
        runtime: &'a mut Runtime,
    ) -> Result<Self::Contents<'a>, Self::Error>;
    fn plot_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        event: PlotVisibilityChanged,
    ) -> Result<(), Self::Error>;
    fn signal_unit_plot(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Self::Error>;
}

/// Borrow the actual actor and map; callbacks may mutate either between reads.
/// Effect preflight must preserve the native actor lifetime: destroying and
/// replacing its slot while this Unit* operation is active is unsupported.
pub struct Context<'a, E> {
    pub state: &'a mut LoadedState,
    pub runtime: &'a mut Runtime,
    pub effects: &'a mut E,
    pub sight_definitions: &'a city_range_attack::Definitions,
    pub actor: (i32, u32),
}
fn mapped_visibility<'a>(
    rows: &'a [c6_core::map::PlayerVisibility],
    mapping: Option<&[i32; 65]>,
    owner: i32,
) -> Result<&'a Visibility, &'static str> {
    let index = usize::try_from(owner)
        .ok()
        .and_then(|i| mapping?.get(i))
        .and_then(|i| usize::try_from(*i).ok())
        .ok_or("visibility owner mapping")?;
    let row = rows
        .get(index)
        .filter(|r| r.player == owner && r.visibility.player_id == owner)
        .ok_or("mapped visibility object")?;
    if rows.iter().filter(|r| r.player == owner).count() != 1 {
        return Err("ambiguous visibility object");
    }
    Ok(&row.visibility)
}
impl<E: Effects> Context<'_, E> {
    fn own_visibility(&self, owner: i32) -> Result<&Visibility, Error<E::Error>> {
        let world = self.state.world();
        mapped_visibility(
            &world.map().visibility,
            world.player_mapping.as_ref(),
            owner,
        )
        .map_err(Error::Missing)
    }
    fn actor(&self) -> Result<&Unit, Error<E::Error>> {
        self.state
            .world()
            .player(self.actor.0)
            .map_err(Error::World)?
            .unit(self.actor.1)
            .map_err(Error::World)?
            .map(|v| v.record)
            .ok_or(Error::Missing("sight actor"))
    }
    /// Execute both phases against the owned Unit and concrete visibility counts.
    /// An effect error retains the native prefix; it is not a successful action.
    pub fn update_map(
        &mut self,
        origin: PlotCoord,
        add: bool,
    ) -> Result<(), unit_sight::SightError<Error<E::Error>>> {
        unit_sight::update_map(self, origin, add)
    }
}

impl<E: Effects> visibility_change::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    fn is_valid(&self, owner: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(Error::World)
    }
    fn preflight(&self, change: VisibilityChange) -> Result<(), Self::Error> {
        // Unknown observer is permitted only when the concrete reached branches
        // are independent of it. Validate this before the first count write.
        if self.runtime.local_observer.is_none() && !self.runtime.messages_disabled {
            let source = self.own_visibility(change.owner)?;
            for recipient in &source.outgoing_visibility {
                let player = recipient.player_id;
                if self
                    .state
                    .world()
                    .player_is_valid(player)
                    .map_err(Error::World)?
                    && player != change.owner
                    && source
                        .outgoing_visibility_blockers
                        .iter()
                        .any(|b| b.player_id == player)
                {
                    continue;
                }
                let own = self.own_visibility(player)?;
                let shared = self.runtime.shared(player).map_err(Error::Missing)?;
                let before = shared
                    .0
                    .iter()
                    .find(|l| l.type_hash == visibility_change::BASE_LAYER_HASH)
                    .and_then(|l| l.values.get(change.plot_index as usize))
                    .copied()
                    .ok_or(Error::Missing("shared base visibility count"))?;
                let after = visibility_change::base_count_after(before, change.delta);
                if (before > 0) != (after > 0)
                    || (after == 0
                        && change.delta == 0
                        && own.revealed.get(change.plot_index as usize) == Some(&0))
                {
                    return Err(Error::Missing("local observer"));
                }
                let hidden_type = if change.near_origin {
                    0
                } else {
                    change.hidden_type
                };
                if hidden_type != -1 && change.delta != 0 {
                    let hash = usize::try_from(hidden_type)
                        .ok()
                        .and_then(|i| self.runtime.hidden_layer_hashes.get(i))
                        .copied()
                        .ok_or(Error::Missing("hidden visibility layer mapping"))?;
                    let probe =
                        |layers: &[c6_core::map::VisibilityLayer]| -> Result<bool, Self::Error> {
                            let Some(layer) = layers.iter().find(|l| l.type_hash == hash) else {
                                return Ok(false);
                            };
                            let mut value = layer
                                .values
                                .get(change.plot_index as usize)
                                .copied()
                                .ok_or(Error::Missing("hidden visibility count"))?;
                            // Native base writes precede hidden writes, including
                            // an explicitly supplied aliased layer hash.
                            if hash == visibility_change::BASE_LAYER_HASH {
                                value = visibility_change::base_count_after(value, change.delta);
                            }
                            Ok(visibility_change::hidden_count_after(value, change.delta).1)
                        };
                    let mut changed = probe(&shared.0)?;
                    if player == change.owner {
                        changed |= probe(&own.layers)?;
                    }
                    if changed
                        && !visibility_propagation::hidden_propagation_is_empty(
                            self.state.world(),
                            change.plot_index,
                        )
                        .map_err(Error::World)?
                    {
                        return Err(Error::Missing("local observer"));
                    }
                }
            }
        }
        self.effects
            .preflight_visibility(self.state, self.runtime, change)
            .map_err(Error::Effect)
    }
    fn visibility(&self, owner: i32) -> Result<&Visibility, Self::Error> {
        self.own_visibility(owner)
    }
    fn visibility_mut(&mut self, owner: i32) -> Result<&mut Visibility, Self::Error> {
        self.own_visibility(owner)?;
        self.state
            .world_mut()
            .map_err(Error::Loaded)?
            .visibility_mut(owner)
            .ok_or(Error::Missing("own visibility"))
    }
    fn shared_layers(&self, owner: i32) -> Result<&SharedLayers, Self::Error> {
        self.runtime.shared(owner).map_err(Error::Missing)
    }
    fn shared_layers_mut(&mut self, owner: i32) -> Result<&mut SharedLayers, Self::Error> {
        self.runtime.shared(owner).map_err(Error::Missing)?;
        // A recipient-only shared change is still a gameplay state mutation;
        // invalidate prepared transactions even when no own layer is touched.
        self.state.world_mut().map_err(Error::Loaded)?;
        self.runtime.shared_mut(owner).map_err(Error::Missing)
    }
    fn hidden_layer_hash(&self, index: i32) -> Result<u32, Self::Error> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.runtime.hidden_layer_hashes.get(i))
            .copied()
            .ok_or(Error::Missing("hidden visibility layer mapping"))
    }
    fn empty_hidden_propagation(&self, plot: u32) -> Result<bool, Self::Error> {
        visibility_propagation::hidden_propagation_is_empty(self.state.world(), plot)
            .map_err(Error::World)
    }
    fn messages_disabled(&mut self) -> Result<bool, Self::Error> {
        Ok(self.runtime.messages_disabled)
    }
    fn local_observer(&mut self) -> Result<i32, Self::Error> {
        self.runtime
            .local_observer
            .ok_or(Error::Missing("local observer"))
    }
    fn update_resource_revealed(&mut self, player: i32, plot: u32) -> Result<(), Self::Error> {
        self.effects
            .update_resource_revealed(self.state, self.runtime, player, plot)
            .map_err(Error::Effect)
    }
    fn propagate(&mut self, event: Propagation) -> Result<(), Self::Error> {
        if event.mask == 0x80 {
            visibility_propagation::propagate_hidden(self, event).map_err(|error| match error {
                visibility_propagation::Error::Context(error) => error,
                visibility_propagation::Error::World(error) => Error::World(error),
                visibility_propagation::Error::UnsupportedMask(mask) => {
                    Error::Propagation(visibility_propagation::Error::UnsupportedMask(mask))
                }
                visibility_propagation::Error::MissingVisibility(player) => {
                    Error::Propagation(visibility_propagation::Error::MissingVisibility(player))
                }
                visibility_propagation::Error::NoncanonicalRevealed => {
                    Error::Propagation(visibility_propagation::Error::NoncanonicalRevealed)
                }
                visibility_propagation::Error::Allocation => {
                    Error::Propagation(visibility_propagation::Error::Allocation)
                }
            })
        } else {
            self.effects
                .propagate(self.state, self.runtime, event)
                .map_err(Error::Effect)
        }
    }
    fn view_plot_contents(&mut self, event: ViewContents) -> Result<(), Self::Error> {
        let mut contents = self
            .effects
            .contents(self.state, self.runtime)
            .map_err(Error::Effect)?;
        visibility_contents::view_plot_contents(&mut contents, event).map_err(|e| match e {
            visibility_contents::Error::Context(e) => Error::Effect(e),
            visibility_contents::Error::MissingPlot(index) => Error::ContentsMissingPlot(index),
        })
    }
}
impl<E: Effects> SightContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Map = ();
    type VisibilityManager = ();
    type Visibility = i32;
    type Player = i32;
    type Plot = u32;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.state
            .unit_mut(self.actor.0, self.actor.1)
            .map_err(Error::Loaded)
    }
    fn preflight_sight(&self, add: bool) -> Result<(), Self::Error> {
        self.effects
            .preflight_sight(self.state, self.runtime, self.actor, add)
            .map_err(Error::Effect)
    }
    fn sight_plots(
        &mut self,
        _owner: i32,
        range: i32,
        origin: PlotCoord,
        features: bool,
        terrain: bool,
    ) -> Result<Vec<SightPlotEntry>, Self::Error> {
        city_range_attack::sight_plots_with_flags(
            self.state,
            (origin.x, origin.y),
            range,
            features,
            terrain,
            self.sight_definitions,
        )
        .map_err(Error::Query)
        .map(|v| {
            v.into_iter()
                .map(|p| SightPlotEntry {
                    ui_index: p.index,
                    i_ring: p.distance,
                })
                .collect()
        })
    }
    fn globals_map(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn map_instance(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn visibility_manager(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn owner_visibility(&mut self, _: (), owner: i32) -> Result<i32, Self::Error> {
        self.own_visibility(owner)?;
        Ok(owner)
    }
    fn player(&mut self, owner: i32) -> Result<i32, Self::Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        Ok(owner)
    }
    fn domain(&mut self) -> Result<i32, Self::Error> {
        let u = self.actor()?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::Loaded)?
            .domain)
    }
    fn plot_index(&mut self, _: (), origin: PlotCoord) -> Result<i32, Self::Error> {
        c6_core::movement_world::plot_at(self.state.world().map(), origin)
            .map_err(|_| Error::Missing("sight origin plot"))
            .and_then(|(i, _)| i32::try_from(i).map_err(|_| Error::Missing("sight plot index")))
    }
    fn plot_xy(&mut self, _: (), index: i32) -> Result<PlotCoord, Self::Error> {
        let map = self.state.world().map();
        if index < 0 || map.header.width <= 0 || index as usize >= map.plots.len() {
            return Err(Error::Missing("old sight origin plot"));
        }
        Ok(PlotCoord {
            x: index % map.header.width,
            y: index / map.header.width,
        })
    }
    fn plot(&mut self, _: (), index: u32) -> Result<u32, Self::Error> {
        self.state
            .world()
            .map()
            .plots
            .get(index as usize)
            .ok_or(Error::Missing("sight plot"))?;
        Ok(index)
    }
    fn water(&mut self, plot: u32) -> Result<bool, Self::Error> {
        self.effects.water(self.state, plot).map_err(Error::Effect)
    }
    fn feature(&mut self, plot: u32) -> Result<i32, Self::Error> {
        self.effects
            .feature(self.state, plot)
            .map_err(Error::Effect)
    }
    fn revealed_feature(&mut self, player: i32, feature: i32) -> Result<bool, Self::Error> {
        self.effects
            .revealed_feature(self.state, player, feature)
            .map_err(Error::Effect)
    }
    fn is_revealed(&mut self, owner: i32, index: u32) -> Result<bool, Self::Error> {
        match self.own_visibility(owner)?.revealed.get(index as usize) {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(Error::Missing("canonical revealed byte")),
        }
    }
    fn change_visibility(&mut self, _: (), change: VisibilityChange) -> Result<i32, Self::Error> {
        visibility_change::change_visibility_count(self, change).map_err(|e| match e {
            visibility_change::ChangeError::Context(e) => e,
            visibility_change::ChangeError::State(e) => Error::Visibility(e),
        })
    }
    fn promotion_class(&mut self) -> Result<Option<&str>, Self::Error> {
        self.effects
            .promotion_class(self.state, self.actor)
            .map_err(Error::Effect)
    }
    fn natural_wonder_experience(&mut self) -> Result<i32, Self::Error> {
        self.effects
            .natural_wonder_experience(self.state)
            .map_err(Error::Effect)
    }
    fn experience_modifier(&mut self) -> Result<i32, Self::Error> {
        self.effects
            .experience_modifier(self.state, self.actor)
            .map_err(Error::Effect)
    }
    fn change_experience(&mut self, amount: i32) -> Result<(), Self::Error> {
        self.effects
            .change_experience(self.state, self.runtime, self.actor, amount)
            .map_err(Error::Effect)
    }
    fn plot_visibility_changed(
        &mut self,
        plot: u32,
        player: i32,
        source: ComponentId,
        other: i32,
        first: bool,
    ) -> Result<(), Self::Error> {
        self.effects
            .plot_visibility_changed(
                self.state,
                self.runtime,
                PlotVisibilityChanged {
                    plot,
                    player,
                    source,
                    other_player: other,
                    first_reveal: first,
                },
            )
            .map_err(Error::Effect)
    }
    fn signal_unit_plot(&mut self, plot: u32) -> Result<(), Self::Error> {
        self.effects
            .signal_unit_plot(self.state, self.runtime, self.actor, plot)
            .map_err(Error::Effect)
    }
}

impl<E: Effects> visibility_propagation::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    fn world(&self) -> &world::World {
        self.state.world()
    }
    fn preflight_hidden(&self, event: Propagation) -> Result<(), Self::Error> {
        self.effects
            .preflight_hidden_propagation(self.state, self.runtime, event)
            .map_err(Error::Effect)
    }
    fn reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error> {
        self.effects
            .visibility_reporting_blocks()
            .map_err(Error::Effect)
    }
    fn unit_visible(
        &mut self,
        player: i32,
        reference: c6_core::map::UnitReference,
    ) -> Result<bool, Self::Error> {
        let unit = self
            .state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .ok_or(Error::Missing("propagation unit lifetime"))?
            .record;
        let location = unit.k_location;
        city_range_attack::unit_visible_with_counts(
            self.state,
            unit,
            player,
            self.sight_definitions,
            |hash, normal| {
                let layers = self.runtime.shared(player).map_err(Error::Missing)?;
                let Some(layer) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(Error::Missing("shared normal visibility layer"))
                    } else {
                        Ok(0)
                    };
                };
                let index = layer
                    .grid_width
                    .wrapping_mul(location.y as u32)
                    .wrapping_add(location.x as u32) as usize;
                Ok(layer.values.get(index).copied().unwrap_or(0))
            },
            || {
                self.runtime
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(Error::Missing("hidden visibility layer0"))
            },
        )
    }
    fn unit_visibility_changed(
        &mut self,
        event: visibility_propagation::UnitVisibilityChanged,
    ) -> Result<(), Self::Error> {
        self.effects
            .unit_visibility_changed(self.state, self.runtime, event)
            .map_err(Error::Effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(owner: i32) -> c6_core::map::PlayerVisibility {
        c6_core::map::PlayerVisibility {
            player: owner,
            visibility: Visibility {
                player_id: owner,
                cached_num_revealed_hexes: 0,
                cached_old: false,
                revealed: vec![0],
                layers: vec![],
                outgoing_visibility: vec![],
                outgoing_visibility_blockers: vec![],
            },
        }
    }
    #[test]
    fn reordered_visibility_requires_actual_manager_mapping() {
        let rows = vec![row(7), row(2)];
        let mut mapping = [-1; 65];
        mapping[7] = 0;
        mapping[2] = 1;
        assert!(std::ptr::eq(
            mapped_visibility(&rows, Some(&mapping), 2).unwrap(),
            &rows[1].visibility
        ));
        mapping[2] = 0;
        assert!(mapped_visibility(&rows, Some(&mapping), 2).is_err());
        assert!(mapped_visibility(&rows, None, 2).is_err());
    }
    #[test]
    fn contradictory_or_duplicate_owner_is_not_silently_selected() {
        let mut rows = vec![row(2)];
        let mut mapping = [-1; 65];
        mapping[2] = 0;
        rows[0].visibility.player_id = 7;
        assert!(mapped_visibility(&rows, Some(&mapping), 2).is_err());
        rows[0].visibility.player_id = 2;
        rows.push(row(2));
        assert!(mapped_visibility(&rows, Some(&mapping), 2).is_err());
    }
}
