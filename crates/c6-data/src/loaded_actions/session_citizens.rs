//! External ManageCitizen requests on the persistent ordinary-action boundary.
//! This is the Linux command's lock-cycle operation, followed by the existing
//! deterministic allocation rules; it never selects a player policy.
use super::{Error, Session};
use crate::{builder_improvement::workers, city_turn::upkeep::citizens};
use c6_core::{city_citizen_assignment as kernel, loaded_state::LoadedState, unit::PlotCoord};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedPlot {
    pub city_plot_index: i32,
    pub location: PlotCoord,
    pub workers: i32,
    pub capacity: i32,
    pub forced: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    /// Reporting c2f385c0/15d11e18, followed by BaseHandler completion.
    CommandComplete { owner: i32, city: u32 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Outcome {
    /// ForceWorkerToPlot may reject its index/population branch even though
    /// ManageCitizen.CanStart succeeded. The command still completes then.
    pub reallocated: bool,
}
fn missing(_: impl std::fmt::Debug) -> crate::city_turn::Error {
    crate::city_turn::Error::Unsupported("ManageCitizen current worker storage")
}
/// GetManagedPlots2899050: native CityHexspace order, including the center
/// regardless of its zero capacity. These are command hints, not a replacement
/// for the original command's independent index/population/capacity rules.
pub fn managed_plots(
    state: &LoadedState,
    owner: i32,
    city: u32,
    effects: &mut impl citizens::Effects,
) -> Result<Vec<ManagedPlot>, crate::city_turn::Error> {
    let c = state.city(owner, city)?;
    let center = PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    let mut plots = Vec::new();
    for index in 0..kernel::NUM_PLOTS as i32 {
        let Some(location) =
            kernel::plot_location(state.world().map(), center, index).map_err(missing)?
        else {
            continue;
        };
        let capacity = effects.capacity(state, owner, city, location)?;
        if capacity == 0 && location != center {
            continue;
        }
        // Both getters resolve the coordinate back to its canonical index;
        // small wrapped maps can visit the same location more than once.
        let at = kernel::plot_index(state.world().map(), center, location).map_err(missing)?;
        let c = &state.city(owner, city)?.citizens;
        let workers =
            kernel::plot_workers(c, state.world().map(), center, location).map_err(missing)?;
        let forced = c
            .forced_working_plot
            .get(at as u32 as usize)
            .copied()
            .unwrap_or(0);
        plots.push(ManagedPlot {
            city_plot_index: index,
            location,
            workers,
            capacity,
            forced,
        });
    }
    Ok(plots)
}
/// ManageCitizen281a9d0 → ForceWorkerToPlot286f530 → ReallocateCitizens.
/// Unsupported reached effects return errors retaining the mutated prefix.
pub fn manage_citizen(
    state: &mut LoadedState,
    owner: i32,
    city: u32,
    location: PlotCoord,
    effects: &mut impl citizens::Effects,
    reports: &mut Vec<Report>,
) -> Result<Outcome, crate::city_turn::Error> {
    // CanStart checks GetManagedPlots, not requested-coordinate membership.
    if managed_plots(state, owner, city, effects)?.is_empty() {
        return Err(crate::city_turn::Error::Unsupported(
            "ManageCitizen has no managed plots",
        ));
    }
    let c = state.city(owner, city)?;
    let center = PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    // Map.GetPlot(PlotCoord)2a48ba0 checks the invalid sentinel BEFORE
    // CoordRange on each axis. Resolve the actual plot's canonical location,
    // then pass it to Citizens, as the native command passes a plot pointer.
    let map = state.world().map();
    let location = if c6_core::geometry::plot_valid(location.x, location.y) {
        PlotCoord {
            x: c6_core::movement_world::coord_range(
                location.x,
                map.header.width,
                map.header.wrap_x,
            ),
            y: c6_core::movement_world::coord_range(
                location.y,
                map.header.height,
                map.header.wrap_y,
            ),
        }
    } else {
        location
    };
    // Map.GetPlot may return null; the native inverse then yields -1.
    let index = if c6_core::movement_world::plot_at(state.world().map(), location).is_ok() {
        kernel::plot_index(state.world().map(), center, location).map_err(missing)?
    } else {
        -1
    };
    let mut remaining = c.header.population;
    for other in 0..kernel::NUM_PLOTS as i32 {
        if remaining <= 0 {
            break;
        }
        let Some(point) =
            kernel::plot_location(state.world().map(), center, other).map_err(missing)?
        else {
            continue;
        };
        let at = kernel::plot_index(state.world().map(), center, point).map_err(missing)?;
        let forced = state
            .city(owner, city)?
            .citizens
            .forced_working_plot
            .get(at as u32 as usize)
            .copied()
            .unwrap_or(0);
        if other != index {
            remaining = remaining.wrapping_sub(forced);
        }
    }
    let mut reallocated = false;
    if index >= 0
        && remaining > 0
        && (index as usize) < state.city(owner, city)?.citizens.forced_working_plot.len()
    {
        let old = kernel::plot_workers(
            &state.city(owner, city)?.citizens,
            state.world().map(),
            center,
            location,
        )
        .map_err(missing)?;
        let capacity = effects.capacity(state, owner, city, location)?;
        let forced = state.city(owner, city)?.citizens.forced_working_plot[index as usize];
        state.city_mut(owner, city)?.citizens.forced_working_plot[index as usize] =
            if forced >= capacity || forced >= remaining {
                0
            } else {
                forced.wrapping_add(1)
            };
        effects.workers_changed(state, owner, city, index, old)?;
        citizens::reallocate(state, owner, city, effects)?;
        reallocated = true;
    }
    // These two native reporting calls publish City stream/host presentation.
    reports.push(Report::CommandComplete { owner, city });
    Ok(Outcome { reallocated })
}
impl Session<'_, '_> {
    /// Query command hints using this Session's canonical current yield and
    /// territory owners. Capacity queries do not visit/update appeal or emit
    /// worker reports, so no appeal subscriber registry is needed here.
    pub fn managed_city_plots(
        &mut self,
        owner: i32,
        city: u32,
        definitions: &citizens::Definitions,
        appeal: &crate::city_production_turn::appeal::Definitions,
        fallout: &[i32],
    ) -> Result<Vec<ManagedPlot>, Error<crate::city_turn::Error>> {
        self.ordinary_player_action(owner)?;
        managed_plots(
            self.context.notifications.state,
            owner,
            city,
            &mut workers::Current {
                backend: &mut self.context.notifications.backend,
                definitions,
                appeal,
                fallout,
                appeal_subscribers: None,
                reports: &mut Vec::new(),
            },
        )
        .map_err(Error::Action)
    }
    /// Apply a location request using the same stock yield, territory and
    /// gameplay/RNG owners already retained by this Session.
    #[allow(clippy::too_many_arguments)]
    pub fn manage_city_citizen(
        &mut self,
        owner: i32,
        city: u32,
        location: PlotCoord,
        definitions: &citizens::Definitions,
        appeal: &crate::city_production_turn::appeal::Definitions,
        fallout: &[i32],
        appeal_subscribers: Option<&[u64]>,
        worker_reports: &mut Vec<workers::Report>,
        reports: &mut Vec<Report>,
    ) -> Result<Outcome, Error<crate::city_turn::Error>> {
        self.ordinary_player_action(owner)?;
        let result = manage_citizen(
            self.context.notifications.state,
            owner,
            city,
            location,
            &mut workers::Current {
                backend: &mut self.context.notifications.backend,
                definitions,
                appeal,
                fallout,
                appeal_subscribers,
                reports: worker_reports,
            },
            reports,
        );
        self.retain(result, true)
    }
}
