//! Map.DoTurn over current canonical components. Eruption/flood expiry leaves
//! publish through the same caller-owned backend; they do not erase history.
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    map::{Component, FeatureComponent},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Route,
    Feature,
    River,
    Terrain,
    Territories,
    TerritoryBuilder,
}
impl Kind {
    fn of(value: &Component) -> Self {
        match value {
            Component::Route(_) => Self::Route,
            Component::Feature(_) => Self::Feature,
            Component::River(_) => Self::River,
            Component::Terrain(_) => Self::Terrain,
            Component::Territories { .. } => Self::Territories,
            Component::TerritoryBuilder => Self::TerritoryBuilder,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNoAddedComponentsOrConsumers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expired {
    Volcano { plot: i32 },
    VolcanoAffected { plot: i32 },
    NaturalWonder { plot: i32 },
    Flood { plot: i32 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Binding,
    Components,
    Failed,
    Requires(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Map.DoTurn: {self:?}")
    }
}
impl std::error::Error for Error {}
pub trait Effects {
    /// Original synchronous reporting boundary. Current visibility getters,
    /// gameplay-capable consumers and their RNG must execute or fail explicitly.
    fn expired(&mut self, state: &mut LoadedState, event: Expired) -> Result<(), Error>;
}
impl<P> Effects for crate::loaded_actions::stock_notifications::Runtime<'_, P> {
    fn expired(&mut self, state: &mut LoadedState, event: Expired) -> Result<(), Error> {
        self.check(state).map_err(|_| Error::Binding)?;
        match event {
            Expired::Flood { plot } => self
                .river_flood_expired(state, plot)
                .map_err(|_| Error::Requires("current flood expiry report consumers")),
            _ => Err(Error::Requires("stock eruption expiry report consumers")),
        }
    }
}
pub struct Runtime {
    instance: ActionStamp,
    bindings: Vec<Kind>,
    /// Sole live owner of the unsaved Feature susceptibility vector. It is
    /// recomputed from canonical current volcano metadata at the native point.
    susceptible: Vec<i32>,
    failed: bool,
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        bindings: Vec<Kind>,
        susceptible: Vec<i32>,
        _: Scope,
    ) -> Result<Self, Error> {
        if state
            .world()
            .map()
            .components
            .iter()
            .map(Kind::of)
            .ne(bindings.iter().copied())
        {
            return Err(Error::Components);
        }
        Ok(Self {
            instance: state.action_stamp(),
            bindings,
            susceptible,
            failed: false,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Binding);
        }
        if self.failed {
            return Err(Error::Failed);
        }
        Ok(())
    }
    /// Read current storage on every call, including after a component callback.
    pub fn component_count(&self, state: &LoadedState) -> Result<u32, Error> {
        self.check(state)?;
        u32::try_from(state.world().map().components.len()).map_err(|_| Error::Components)
    }
    /// Partial derived-state writes remain inspectable after an error.
    pub fn susceptible_plots(&self, state: &LoadedState) -> Result<&[i32], Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Binding);
        }
        Ok(&self.susceptible)
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub fn component_turn(
        &mut self,
        state: &mut LoadedState,
        index: u32,
        effects: &mut impl Effects,
    ) -> Result<(), Error> {
        self.check(state)?;
        let result = self.component_inner(state, index, effects);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn component_inner(
        &mut self,
        state: &mut LoadedState,
        index: u32,
        effects: &mut impl Effects,
    ) -> Result<(), Error> {
        let kind = state
            .world()
            .map()
            .components
            .get(index as usize)
            .map(Kind::of)
            .ok_or(Error::Components)?;
        if self.bindings.get(index as usize) != Some(&kind) {
            return Err(Error::Components);
        }
        let turn = state.retained().core.current_turn;
        match kind {
            Kind::Feature => {
                // Native UpdateVolcanoes caches its vector end. Only expiry
                // publications occur here; no eruption or RNG draw is created.
                let count = feature(state, index)?.volcanoes.len();
                for i in 0..count {
                    let volcano = feature(state, index)?
                        .volcanoes
                        .get(i)
                        .ok_or(Error::Components)?;
                    if expires(volcano.eruptions.last().map(|e| (e.turn, e.duration)), turn) {
                        let plot = volcano.plot;
                        let affected = volcano.affected_plots.clone();
                        effects.expired(state, Expired::Volcano { plot })?;
                        for plot in affected {
                            effects.expired(state, Expired::VolcanoAffected { plot })?;
                        }
                    }
                }
                let count = feature(state, index)?.natural_wonder_volcanoes.len();
                for i in 0..count {
                    let volcano = feature(state, index)?
                        .natural_wonder_volcanoes
                        .get(i)
                        .ok_or(Error::Components)?;
                    if expires(volcano.eruptions.last().map(|e| (e.turn, e.duration)), turn) {
                        if let Some(&plot) = volcano.plots.first() {
                            effects.expired(state, Expired::NaturalWonder { plot })?;
                        }
                    }
                }
                // CalculateSusceptibleTiles clears before appending each
                // active ordinary volcano's exact affected-plot order.
                self.susceptible.clear();
                append_normal_susceptible(feature(state, index)?, &mut self.susceptible);
                if !feature(state, index)?.natural_wonder_volcanoes.is_empty() {
                    return Err(Error::Requires(
                        "natural wonder affected-plot susceptibility",
                    ));
                }
            }
            Kind::River => {
                let count = river(state, index)?.rivers.len();
                for i in 0..count {
                    let river = river(state, index)?
                        .rivers
                        .get(i)
                        .ok_or(Error::Components)?;
                    if expires(river.floods.last().map(|f| (f.turn, f.duration)), turn) {
                        let plots = river.floodplains.clone();
                        for plot in plots {
                            effects.expired(state, Expired::Flood { plot })?;
                        }
                    }
                }
            }
            // These concrete stock vtables share the source literal RET body.
            Kind::Route | Kind::Terrain | Kind::Territories | Kind::TerritoryBuilder => {}
        }
        Ok(())
    }
}
fn feature(state: &LoadedState, index: u32) -> Result<&FeatureComponent, Error> {
    match state.world().map().components.get(index as usize) {
        Some(Component::Feature(f)) => Ok(f.as_ref()),
        _ => Err(Error::Components),
    }
}
fn river(state: &LoadedState, index: u32) -> Result<&c6_core::map::RiverComponent, Error> {
    match state.world().map().components.get(index as usize) {
        Some(Component::River(r)) => Ok(r.as_ref()),
        _ => Err(Error::Components),
    }
}
fn expires(last: Option<(i32, i32)>, turn: i32) -> bool {
    last.is_some_and(|(start, duration)| start.wrapping_add(duration) == turn)
}
fn append_normal_susceptible(feature: &FeatureComponent, output: &mut Vec<i32>) {
    for volcano in &feature.volcanoes {
        if volcano.turn_active > 0 {
            output.extend_from_slice(&volcano.affected_plots);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiry_requires_exact_turn_and_wraps() {
        assert!(!expires(None, 1));
        assert!(!expires(Some((3, 2)), 6));
        assert!(expires(Some((3, 2)), 5));
        assert!(expires(Some((i32::MAX, 1)), i32::MIN));
    }
    #[test]
    fn susceptibility_uses_current_active_flags_and_preserves_duplicates() {
        use c6_core::map::{SparsePlotData, Volcano};
        let volcano = |turn_active, affected_plots| Volcano {
            name_hash: u32::MAX,
            plot: 0,
            discovering_civilization_hash: u32::MAX,
            turn_active,
            custom_name: vec![],
            eruptions: vec![],
            affected_plots,
            periods_raw: vec![],
            chances_raw: vec![],
        };
        let feature = FeatureComponent {
            extra_yields: SparsePlotData {
                default: vec![],
                indices: vec![],
                values: vec![],
            },
            volcanoes: vec![
                volcano(-1, vec![9]),
                volcano(1, vec![3, 2, 3]),
                volcano(0, vec![8]),
                volcano(4, vec![2]),
            ],
            natural_wonder_volcanoes: vec![],
            plot_volcanoes: SparsePlotData {
                default: vec![],
                indices: vec![],
                values: vec![],
            },
            ice: vec![],
        };
        let mut result = vec![];
        append_normal_susceptible(&feature, &mut result);
        assert_eq!(result, [3, 2, 3, 2]);
    }
}
