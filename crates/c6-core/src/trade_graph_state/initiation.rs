//! Linux XP2 Graph.AddRoute2bfb700 and Edge.AddNewRoute2bf9350.
//! Only an existing nonempty path accepts a route. No graph key, edge ID or
//! path is created; cold pathfinding and callback processing belong to callers.
use super::{city_vertex, GraphState};
use crate::trade::{ComponentId, Route};

impl GraphState {
    /// AddPath2bfbb70 creates a missing Edge using the manager's current
    /// counter, then replaces only its path. Existing active routes/cursors and
    /// route counts remain unchanged. Yield order is the actual definitions.
    pub fn add_path_to_city(
        &mut self,
        from: ComponentId,
        to: ComponentId,
        path: &[i32],
        yields: &[u32],
    ) -> Result<(), super::Error> {
        let (a, b) = (city_vertex(from), city_vertex(to));
        if self.graph.get_edge(a, b).is_none() {
            if crate::trade_read_order::vertex_hash(b) == u32::MAX {
                return Err(super::Error::UnsupportedDestinationHash(b));
            }
            let null = |kind| ComponentId {
                owner: -1,
                component_type: kind,
                id: u32::MAX,
            };
            let zeros = || {
                yields
                    .iter()
                    .map(|&type_hash| crate::trade::YieldValue { type_hash, raw: 0 })
                    .collect()
            };
            let route = Route {
                id: -1,
                start_turn: -1,
                trader_unit: null(1),
                resource_type_hash: u32::MAX,
                resource_quantity: 0,
                origin_city: null(2),
                destination_city: null(2),
                origin_yields: zeros(),
                destination_yields: zeros(),
            };
            self.graph
                .insert_edge(crate::trade::Edge {
                    wire_version: 3,
                    id: self.graph.total_edges_created(),
                    most_recent_route: route,
                    has_active_route: false,
                    routes_started: 0,
                    routes_completed: 0,
                    most_recent_path: vec![],
                    path_traversal_index: 0,
                    path_traversal_going_forward: false,
                    from_vertex: a,
                    to_vertex: b,
                })
                .map_err(super::Error::Codec)?;
        }
        self.graph.get_edge_mut(a, b).unwrap().most_recent_path = path.to_vec();
        Ok(())
    }
    /// Native AddRoute returns without writing for an absent edge, empty path,
    /// or already active route. This returns whether AddNewRoute wrote fields.
    pub fn add_route_to_city(&mut self, from: ComponentId, to: ComponentId, route: &Route) -> bool {
        let Some(edge) = self.graph.get_edge_mut(city_vertex(from), city_vertex(to)) else {
            return false;
        };
        if edge.most_recent_path.is_empty() || edge.has_active_route {
            return false;
        }
        edge.most_recent_route = route.clone();
        edge.has_active_route = true;
        edge.path_traversal_index = 0;
        edge.path_traversal_going_forward = true;
        edge.routes_started = edge.routes_started.wrapping_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::{Destination, Edge, Graph, Origin};
    fn city(id: u32) -> ComponentId {
        ComponentId {
            owner: 0,
            component_type: 2,
            id,
        }
    }
    fn route(id: i32) -> Route {
        Route {
            id,
            start_turn: 7,
            trader_unit: ComponentId {
                owner: 0,
                component_type: 1,
                id: 65538,
            },
            resource_type_hash: u32::MAX,
            resource_quantity: 0,
            origin_city: city(65536),
            destination_city: city(65537),
            origin_yields: vec![],
            destination_yields: vec![],
        }
    }
    fn graph(path: Vec<i32>, active: bool) -> GraphState {
        let (from, to) = (city_vertex(city(65536)), city_vertex(city(65537)));
        GraphState::from_graph(
            Graph::new(
                9,
                vec![Origin {
                    vertex: from,
                    destinations: vec![Destination {
                        vertex: to,
                        edge: Edge {
                            wire_version: 3,
                            id: 8,
                            most_recent_route: route(-1),
                            has_active_route: active,
                            routes_started: i32::MAX,
                            routes_completed: 12,
                            most_recent_path: path,
                            path_traversal_index: 99,
                            path_traversal_going_forward: false,
                            from_vertex: from,
                            to_vertex: to,
                        },
                    }],
                }],
            )
            .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn activation_preserves_path_keys_and_completion_count() {
        let mut graph = graph(vec![7, 8, 7, 9], false);
        let started = route(43);
        assert!(graph.add_route_to_city(city(65536), city(65537), &started));
        let edge = graph.graph().get_edge_by_id(8).unwrap();
        assert_eq!(
            (
                &edge.most_recent_route,
                edge.has_active_route,
                edge.routes_started,
                edge.routes_completed,
                edge.path_traversal_index,
                edge.path_traversal_going_forward
            ),
            (&started, true, i32::MIN, 12, 0, true)
        );
        assert_eq!(edge.most_recent_path, [7, 8, 7, 9]);
        assert_eq!(
            (
                graph.graph().total_edges_created(),
                graph.graph().edge_count()
            ),
            (9, 1)
        );
        assert!(!graph.add_route_to_city(city(65536), city(65537), &route(44)));
        assert_eq!(
            graph
                .graph()
                .get_edge_by_id(8)
                .unwrap()
                .most_recent_route
                .id,
            43
        );
    }
    #[test]
    fn cold_path_allocates_one_canonical_edge_and_preserves_replacement_cursor() {
        let mut graph = GraphState::from_graph(Graph::new(17, vec![]).unwrap()).unwrap();
        graph
            .add_path_to_city(city(65536), city(65537), &[3, 8, 10], &[11, 22])
            .unwrap();
        let edge = graph.graph().get_edge_by_id(17).unwrap();
        assert_eq!(
            (
                graph.graph().total_edges_created(),
                edge.has_active_route,
                edge.routes_started,
                edge.routes_completed,
                edge.path_traversal_index,
                edge.path_traversal_going_forward
            ),
            (18, false, 0, 0, 0, false)
        );
        assert_eq!(edge.most_recent_route.id, -1);
        assert_eq!(
            edge.most_recent_route
                .origin_yields
                .iter()
                .map(|v| (v.type_hash, v.raw))
                .collect::<Vec<_>>(),
            [(11, 0), (22, 0)]
        );
        assert!(graph.add_route_to_city(city(65536), city(65537), &route(5)));
        graph
            .edit_city_edge(city(65536), city(65537))
            .unwrap()
            .advance_path_to_next_plot()
            .unwrap();
        graph
            .add_path_to_city(city(65536), city(65537), &[7, 2], &[99])
            .unwrap();
        let edge = graph.graph().get_edge_by_id(17).unwrap();
        assert_eq!(
            (
                graph.graph().total_edges_created(),
                edge.most_recent_route.id,
                edge.path_traversal_index,
                edge.path_traversal_going_forward,
                edge.routes_started
            ),
            (18, 5, 1, true, 1)
        );
        assert_eq!(edge.most_recent_path, [7, 2]);
    }
    #[test]
    fn absent_reverse_empty_or_active_edge_is_untouched() {
        for (path, active) in [(vec![], false), (vec![7, 8], true)] {
            let mut graph = graph(path, active);
            let before = graph.graph().clone();
            assert!(!graph.add_route_to_city(city(65536), city(65537), &route(4)));
            assert!(!graph.add_route_to_city(city(65537), city(65536), &route(4)));
            assert!(graph.graph().logical_eq(&before));
        }
    }
}
