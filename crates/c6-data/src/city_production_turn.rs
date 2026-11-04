//! Canonical Linux City BuildQueue activation and progress application.
//!
//! Policies choose queue entries. Current yield/cost/legality queries and reached
//! creation/report subscribers use the same caller-owned effects service as the
//! rest of the turn. An effects error can follow a real mutation prefix; callers
//! must retain that incomplete transition instead of treating it as a completed
//! turn. No native production policy or host random stream is substituted.
pub mod buildings;

use c6_core::{
    city_production::{execution, Order, QueueEntry},
    loaded_state::LoadedState,
    unit::HashValue,
};

#[derive(Debug)]
pub enum Error {
    Binding,
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    District(c6_core::loaded_state::district_initialization::DistrictError),
    State(c6_core::loaded_state::Error),
    MissingProgress(Order),
    Unsupported(&'static str),
}
impl From<c6_core::loaded_state::Error> for Error {
    fn from(value: c6_core::loaded_state::Error) -> Self {
        Self::State(value)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City production transition: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub struct CurrentBuild {
    /// Current GetCurrentBuildProgressRequired, in whole production units.
    pub cost: i32,
    /// Update*Progress factors in native multiplication order. This includes
    /// dynamic player bonuses; it is not a cache inferred from saved fields.
    pub factors_q8: Vec<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// No build selected. The turn's generic production was banked in overflow.
    NeedsProduction,
    Progressed,
    Completed {
        needs_production: bool,
    },
}

/// All methods are mandatory: production cannot silently omit legality,
/// strategic-resource consumption, completion effects or synchronous reports.
pub trait Effects {
    fn calculate_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
        production: Option<Order>,
    ) -> Result<i32, Error>;
    /// Resolve current legality, cost and ordered multipliers. An unsupported
    /// upgrade/replacement branch returns an error before progress is written.
    fn current_build(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<CurrentBuild, Error>;
    /// Update*Progress's first-positive-progress resource reservation/consumption
    /// and climate effects. Called before the progress addition, in source order.
    fn start_progress(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<(), Error>;
    fn progress_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<(), Error>;
    /// CheckIf*Finished after threshold equality. Must execute the actual
    /// creation/repair, queue, completion grants, signals and RNG against state.
    /// Return false for native creation failure; never create a default object.
    fn finish(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        entry: &QueueEntry,
    ) -> Result<bool, Error>;
    /// Native DoNewFrontCheck after successful completion; recheck the next
    /// externally chosen entry without selecting native AI production.
    fn new_front(&mut self, state: &mut LoadedState, owner: i32, city: u32) -> Result<(), Error>;
}

fn progress(
    state: &LoadedState,
    owner: i32,
    city: u32,
    order: Order,
) -> Result<&HashValue<i32>, Error> {
    let queue = &state.city(owner, city)?.build_queue;
    let (values, hash) = match order {
        Order::Train { unit_hash, .. } => (&queue.unit_progress, unit_hash),
        Order::Construct { building_hash } => (&queue.building_progress, building_hash),
        Order::Zone { district_hash } => (&queue.district_progress, district_hash),
        Order::Advance { project_hash } => (&queue.project_progress, project_hash),
        Order::Other(_) => return Err(Error::Unsupported("production order")),
    };
    let mut values = values.iter().filter(|p| p.definition_hash == hash);
    let value = values.next().ok_or(Error::MissingProgress(order))?;
    if values.next().is_some() {
        return Err(Error::Unsupported("ambiguous progress definition"));
    }
    Ok(value)
}
fn set_progress(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    order: Order,
    value: i32,
) -> Result<(), Error> {
    progress(state, owner, city, order)?;
    let queue = &mut state.city_mut(owner, city)?.build_queue;
    let (values, hash) = match order {
        Order::Train { unit_hash, .. } => (&mut queue.unit_progress, unit_hash),
        Order::Construct { building_hash } => (&mut queue.building_progress, building_hash),
        Order::Zone { district_hash } => (&mut queue.district_progress, district_hash),
        Order::Advance { project_hash } => (&mut queue.project_progress, project_hash),
        Order::Other(_) => return Err(Error::Unsupported("production order")),
    };
    values
        .iter_mut()
        .find(|p| p.definition_hash == hash)
        .expect("validated progress")
        .value = value;
    Ok(())
}

/// 285e1b0: salvage is merged before DoBuilds, even for an empty queue.
pub fn do_turn_activate(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    effects: &mut impl Effects,
) -> Result<Outcome, Error> {
    let queue = &mut state.city_mut(owner, city)?.build_queue;
    queue.production_overflow = queue
        .production_overflow
        .wrapping_add(queue.production_salvage);
    queue.production_salvage = 0;
    do_builds(state, owner, city, effects)
}

/// 284dea0: calculate selected-kind yield, then generic yield, then clear the
/// project yield list/hash, before dispatching AddProgress with overflow=true.
pub fn do_builds(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    effects: &mut impl Effects,
) -> Result<Outcome, Error> {
    let queue = &state.city(owner, city)?.build_queue;
    let order = queue.entries.first().map(|e| e.order);
    let yield_index = queue.yield_type;
    let selected = effects.calculate_yield_q8(state, owner, city, yield_index, order)?;
    let generic = effects.calculate_yield_q8(state, owner, city, yield_index, None)?;
    let header = &mut state.city_mut(owner, city)?.header;
    for value in &mut header.project_yield_q8 {
        value.value = 0;
    }
    header.project_yield_project_hash = u32::MAX;
    add_progress(state, owner, city, selected, generic, true, effects)
}

/// Applies actual canonical progress and overflow. Completion services execute
/// before overflow is recomputed, so they observe consumed old overflow=0.
pub fn add_progress(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    selected_yield_q8: i32,
    generic_yield_q8: i32,
    allow_overflow: bool,
    effects: &mut impl Effects,
) -> Result<Outcome, Error> {
    let Some(entry) = state
        .city(owner, city)?
        .build_queue
        .entries
        .first()
        .cloned()
    else {
        if allow_overflow {
            let queue = &mut state.city_mut(owner, city)?.build_queue;
            queue.production_overflow = queue.production_overflow.wrapping_add(generic_yield_q8);
        }
        return Ok(Outcome::NeedsProduction);
    };
    let build = effects.current_build(state, owner, city, &entry)?;
    let before = progress(state, owner, city, entry.order)?.value;
    let remaining = execution::remaining(build.cost, before);
    let overflow = state.city(owner, city)?.build_queue.production_overflow;
    let delta = execution::modified_progress(selected_yield_q8, overflow, &build.factors_q8);
    if before <= 0 {
        effects.start_progress(state, owner, city, &entry)?;
    }
    set_progress(state, owner, city, entry.order, before.wrapping_add(delta))?;
    effects.progress_report(state, owner, city, &entry)?;
    state.city_mut(owner, city)?.build_queue.production_overflow = 0;
    let current = state
        .city(owner, city)?
        .build_queue
        .entries
        .first()
        .cloned();
    let finished = if let Some(current) = current {
        let current_build = effects.current_build(state, owner, city, &current)?;
        progress(state, owner, city, current.order)?.value >= current_build.cost.wrapping_shl(8)
            && effects.finish(state, owner, city, &current)?
    } else {
        false
    };
    if finished {
        state.city_mut(owner, city)?.build_queue.production_overflow =
            execution::completion_overflow(
                selected_yield_q8,
                generic_yield_q8,
                remaining,
                allow_overflow,
            );
        effects.new_front(state, owner, city)?;
        Ok(Outcome::Completed {
            needs_production: state.city(owner, city)?.build_queue.entries.is_empty(),
        })
    } else {
        Ok(Outcome::Progressed)
    }
}

pub trait QueueEffects: Effects {
    /// 92492f56/d3a03c77 report after appending behind an existing front.
    fn queue_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        index: usize,
    ) -> Result<(), Error>;
}
/// External policy's append operation (native insert mode3). Parameters come
/// from the typed Build operation; gameplay legality is resolved before writes.
/// Existing front/progress are preserved. Replace/reorder/remove are separate
/// operations and are not silently mapped to append.
pub fn append_selected(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    entry: QueueEntry,
    effects: &mut impl QueueEffects,
) -> Result<(), Error> {
    if entry.type_hash != c6_core::city_production::BUILD_TYPE_HASH
        || entry.node_type_hash != c6_core::city_production::BUILD_TYPE_HASH
    {
        return Err(Error::Unsupported("Build operation type"));
    }
    effects.current_build(state, owner, city, &entry)?;
    progress(state, owner, city, entry.order)?;
    let queue = &mut state.city_mut(owner, city)?.build_queue;
    queue
        .entries
        .try_reserve(1)
        .map_err(|_| Error::Unsupported("production queue allocation"))?;
    let index = queue.entries.len();
    if index == 0 {
        queue.previous_production_type_hash = 0;
    }
    queue.entries.push(entry);
    if index == 0 {
        effects.new_front(state, owner, city)
    } else {
        effects.queue_changed(state, owner, city, index)
    }
}
