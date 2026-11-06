//! Linux XP2 Graph.RemoveRouteByCityPair2bf9ad0 and
//! Edge.RemoveCurrentRoute2bf9400. The graph keeps the last route and path.
use super::{city_vertex, GraphState};
use crate::trade::{ComponentId, Route};
impl GraphState {
    /// Remove only the active route at the exact ordered city pair. Return the
    /// old route for ordered manager cleanup; callers must execute those effects.
    /// An absent/inactive edge changes nothing and produces no cleanup record.
    pub fn remove_route_by_city_pair(
        &mut self,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Option<Route> {
        let edge = self
            .graph
            .get_edge_mut(city_vertex(origin), city_vertex(destination))?;
        if !edge.has_active_route {
            return None;
        }
        let route = edge.most_recent_route.clone();
        edge.has_active_route = false;
        edge.routes_completed = edge.routes_completed.wrapping_add(1);
        Some(route)
    }
}
