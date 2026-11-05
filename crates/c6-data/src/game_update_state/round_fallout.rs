//! Linux XP2 Fallout.Manager::DoTurn29a8670: plot decay, then reactor aging.
//! Positive plot decay retains its write and pending signal/report continuation;
//! it cannot be treated as completed without its reached consumers.
use c6_core::{
    core_player::initialization::WireBinding, game_fallout_wire::Instance,
    loaded_state::LoadedState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reactor {
    pub plot_index: i32,
    pub owner: i32,
    pub age: i32,
    pub accident_turn: i32,
}
pub struct Initial {
    pub binding: WireBinding,
    pub plots: Vec<Instance>,
    /// Omitted from the older wire carrier: import the actual live list.
    pub reactors: Vec<Reactor>,
    pub damage_override: i32,
}
pub struct Runtime {
    binding: WireBinding,
    /// Single mutable plot owner in native plot-index order.
    pub plots: Vec<Instance>,
    /// Single mutable reactor owner, also queried by RandomEvents.
    pub reactors: Vec<Reactor>,
    pub damage_override: i32,
    pub pending: Option<Changed>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Changed {
    pub plot_index: usize,
    pub turns_remaining: i32,
    pub prevent_work_raw: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Missing(&'static str),
    Pending(Changed),
}
impl Runtime {
    pub fn from_initial(state: &LoadedState, input: Initial) -> Result<Self, Error> {
        if input.binding != *state.binding() {
            return Err(Error::Missing("Fallout initial binding"));
        }
        if input.plots.len() != state.world().map().plots.len() {
            return Err(Error::Missing("Fallout complete plot order"));
        }
        if input.plots.iter().any(|p| p.wire_version != 2) {
            return Err(Error::Missing("Fallout Instance version"));
        }
        Ok(Self {
            binding: input.binding,
            plots: input.plots,
            reactors: input.reactors,
            damage_override: input.damage_override,
            pending: None,
        })
    }
    /// Read-only query projection; never retain this as another mutable manager.
    pub fn turns(&self) -> Vec<i32> {
        self.plots.iter().map(|p| p.turns_remaining).collect()
    }
    pub fn reactor_count(&self) -> usize {
        self.reactors.len()
    }
}

/// The positive-cell continuation must dispatch Game.Signals+1158, report
/// 8e5e1e95, query the CURRENT observer's revealed state, then report8cbda9f3.
/// Those subscribers may affect gameplay and are explicitly unsupported here.
/// A pending decrement is retained, so retrying cannot decrement twice.
pub fn run<P>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
) -> Result<(), Error> {
    if runtime.binding != *state.binding() || runtime.plots.len() != state.world().map().plots.len()
    {
        return Err(Error::Missing("Fallout current binding/plot order"));
    }
    if let Some(changed) = runtime.pending {
        return Err(Error::Pending(changed));
    }
    // UpdateFallout29a8690 tests >0 before decrementing, and performs no
    // callbacks at all for zero or negative turns. Native order is plot index.
    for (plot_index, plot) in runtime.plots.iter_mut().enumerate() {
        if plot.turns_remaining > 0 {
            plot.turns_remaining = plot.turns_remaining.wrapping_sub(1);
            let changed = Changed {
                plot_index,
                turns_remaining: plot.turns_remaining,
                prevent_work_raw: plot.prevent_work_raw,
            };
            runtime.pending = Some(changed);
            return Err(Error::Pending(changed));
        }
    }
    // UpdateNuclearReactorStatus29a8830 increments EVERY age(+8), including
    // negative ages, without inspecting the city, climate, or accident turn.
    for reactor in &mut runtime.reactors {
        reactor.age = reactor.age.wrapping_add(1);
    }
    backend.reached.push("Game.Fallout.DoTurn");
    Ok(())
}
