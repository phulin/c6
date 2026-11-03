//! Attributed owned Trade input and actual warm-path domain for YieldMonitor.
//!
//! This module does not yet authorize omission of the monitor. Its unsaved
//! derived-cache initialization/history and complete query effects remain
//! separate admission requirements. No captured numerical answer is installed.
use c6_core::{
    core_player::initialization::WireRole,
    loaded_state::{ComponentSnapshotWitness, LoadedState},
    trade,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const CORE: &str = "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884";
const TRACE: &str = "528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae";
const FRAME: &str = "64a196d77b6bc35a72354226dda4111e10f46742cd62d4e335b43c737a186c33";
const CORE_BYTES: usize = 80_657_462;
const FRAME_RANGE: [usize; 2] = [71_027_800, 71_497_438];

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

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn matches_hash(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)) == expected
}

/// Install exactly the source-attributed Graph frame. Saved origin/destination
/// yield arrays remain ordinary decoded Route data, never effective answers.
/// Core performs one-time pristine/binding checks before accepting the owner.
pub fn install_trade_graph_snapshot(
    state: &mut LoadedState,
    owner_trace: &[u8],
    limits: trade::Limits,
) -> Result<(), Error> {
    let witness = snapshot_witness(state, owner_trace)?;
    let frame_sha256 =
        digest(&state.source_image()[witness.owner_range[0]..witness.owner_range[1]]);
    state
        .install_trade_graph(witness, frame_sha256, limits)
        .map_err(Error::State)
}

fn snapshot_witness(state: &LoadedState, trace: &[u8]) -> Result<ComponentSnapshotWitness, Error> {
    let bytes = state.source_image();
    if bytes.len() != CORE_BYTES
        || !matches_hash(bytes, CORE)
        || state.binding().core_sha256 != digest(bytes)
        || !matches_hash(trace, TRACE)
    {
        return Err(Error::Attribution);
    }
    let mut range = None;
    for line in trace
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let row: Value = serde_json::from_slice(line).map_err(|_| Error::Attribution)?;
        if row["event"] == "owner" && row["save"] == 1 && row["owner"] == "GameCore::Trade::Graph" {
            if range.is_some()
                || row["depth"] != 2
                || row["parent"] != 162110
                || row["call"] != 162111
            {
                return Err(Error::Attribution);
            }
            let offset = |field: &str| {
                row[field]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or(Error::Attribution)
            };
            range = Some([offset("start")?, offset("end")?]);
        }
    }
    let range = range.ok_or(Error::Attribution)?;
    if range != FRAME_RANGE
        || !bytes
            .get(range[0]..range[1])
            .is_some_and(|frame| matches_hash(frame, FRAME))
    {
        return Err(Error::Attribution);
    }
    Ok(ComponentSnapshotWitness {
        source_role: WireRole::SavedOutputCore,
        snapshot_sha256: digest(bytes),
        snapshot_bytes: bytes.len(),
        owner_trace_sha256: digest(trace),
        owner_range: range,
        decoded_prefix_bytes: range[1] - range[0],
    })
}

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
