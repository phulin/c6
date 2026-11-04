//! Attributed owned Trade input and actual warm-path domain for YieldMonitor.
//!
//! These checks supply the current route domain for the stock headless
//! observer projection. They do not establish arbitrary callback/provider
//! purity or install captured yield answers. The caller must separately admit
//! the supported rules, plain providers and complete observer bindings.
use c6_core::loaded_state::LoadedState;

#[derive(Debug)]
pub enum Error {
    Attribution,
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    MissingWarmRoutePath { edge_id: i32, route_id: i32 },
    InvalidPathPlot { edge_id: i32, plot: i32 },
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "headless YieldMonitor input: {self:?}")
    }
}
impl std::error::Error for Error {}

/// A current-state query result, not a cache-freshness or omission certificate.
/// Native route iteration order is deliberately not provided by wire traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WarmRouteDomain {
    pub revision: u64,
    pub active_routes: usize,
    pub queried_path_nodes: usize,
}

/// Follow the Route's actual city fields through authoritative Graph.GetPath.
/// Native lookup need not return the iterated edge; no endpoint-equality guard
/// is added. A forward miss/empty path rejects before any pooled finder runs.
pub fn warm_route_domain(state: &LoadedState) -> Result<WarmRouteDomain, Error> {
    let owner = state.trade_graph().map_err(Error::State)?;
    let mut result = WarmRouteDomain {
        revision: state.revision(),
        active_routes: 0,
        queried_path_nodes: 0,
    };
    for origin in owner.graph().origins() {
        for destination in &origin.destinations {
            let edge = &destination.edge;
            if !edge.has_active_route {
                continue;
            }
            let route = &edge.most_recent_route;
            let path = owner
                .city_path(route.origin_city, route.destination_city)
                .ok_or(Error::MissingWarmRoutePath {
                    edge_id: edge.id,
                    route_id: route.id,
                })?;
            for &plot in path {
                if plot < 0 || plot as usize >= state.world().map().plots.len() {
                    return Err(Error::InvalidPathPlot {
                        edge_id: edge.id,
                        plot,
                    });
                }
            }
            result.active_routes = result
                .active_routes
                .checked_add(1)
                .ok_or(Error::ResourceLimit)?;
            result.queried_path_nodes = result
                .queried_path_nodes
                .checked_add(path.len())
                .ok_or(Error::ResourceLimit)?;
        }
    }
    Ok(result)
}

/// Actual saved guards relevant to the post-Monitor initialization interval.
/// These are current owned inputs, not proof that native postload ran or that
/// every callback preserved query inputs. Nonempty sight can exclude the
/// PopulateSight branch; empty sight still requires IsDead/location guards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PostLoadGuardInputs {
    pub cities: usize,
    pub force_update_plot_appeal: usize,
    pub units: usize,
    pub empty_unit_sight: usize,
}

pub fn postload_guard_inputs(state: &LoadedState) -> Result<PostLoadGuardInputs, Error> {
    let mut result = PostLoadGuardInputs {
        cities: 0,
        force_update_plot_appeal: 0,
        units: 0,
        empty_unit_sight: 0,
    };
    for player in &state.retained().players {
        for city in &player.cities.records.entries {
            result.cities = result.cities.checked_add(1).ok_or(Error::ResourceLimit)?;
            result.force_update_plot_appeal = result
                .force_update_plot_appeal
                .checked_add(usize::from(city.value.citizens.force_update_plot_appeal))
                .ok_or(Error::ResourceLimit)?;
        }
    }
    for player in state.world().players() {
        for unit in player.unit_records().map_err(Error::World)? {
            result.units = result.units.checked_add(1).ok_or(Error::ResourceLimit)?;
            result.empty_unit_sight = result
                .empty_unit_sight
                .checked_add(usize::from(unit.value.k_sight_plots.is_empty()))
                .ok_or(Error::ResourceLimit)?;
        }
    }
    Ok(result)
}
