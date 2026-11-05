//! Owned Trade Graph keys/paths and narrow current traversal edits.
//!
//! Graph.Read reconstructs forward/reverse keyed indexes from the saved entries.
//! This boundary supports point path lookup, not native ID-map callback order,
//! omitted Route state, general route edits or the pooled cold pathfinder. Attribution
//! and one-time frame checks belong to LoadedState's component admission.
use crate::trade::{self, ComponentId, Graph, Vertex};
use crate::trade_read_order::vertex_hash;
pub mod edge;
pub mod initiation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Codec(trade::Error),
    /// Native inner hash tables reserve this hash for empty slots. The codec's
    /// broader wire grammar does not itself establish native insertion support.
    UnsupportedDestinationHash(Vertex),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Trade Graph path state: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Vertex(CityID) at0x2bf85f0 uses signed owner and full ID bits. Component type
/// is not part of the key; low16 recycled-slot lookup must not replace full ID.
pub fn city_vertex(city: ComponentId) -> Vertex {
    Vertex {
        kind: 0,
        data1: i32::from(city.owner),
        data2: city.id as i32,
    }
}

/// A borrowed reverse-path projection, preserving every plot index and repeat.
/// Reversal affects the caller's observation, never the stored Graph path.
#[derive(Clone, Copy, Debug)]
pub struct ReversePath<'a> {
    nodes: &'a [i32],
}
impl ReversePath<'_> {
    pub fn iter(&self) -> impl ExactSizeIterator<Item = i32> + DoubleEndedIterator + '_ {
        self.nodes.iter().rev().copied()
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// No mutable Graph reference or independent captured-answer map is exposed.
/// Cursor edits update this same owner. Key/path/route changes remain outside
/// this traversal API and must not continue querying a detached initial copy.
#[derive(Debug)]
pub struct GraphState {
    graph: Graph,
}
impl GraphState {
    /// Decode exactly one attributed frame, never a searched/broad Core prefix.
    /// The caller must perform source-image/frame admission once before treating
    /// this value as current authority. This constructor is not authentication.
    pub fn decode(frame: &[u8], limits: trade::Limits) -> Result<Self, Error> {
        let graph = Graph::decode(frame, limits).map_err(Error::Codec)?;
        Self::from_graph(graph)
    }
    /// Construct current typed graph values without inventing saved bytes.
    pub fn from_graph(graph: Graph) -> Result<Self, Error> {
        for origin in graph.origins() {
            for destination in &origin.destinations {
                if vertex_hash(destination.vertex) == u32::MAX {
                    return Err(Error::UnsupportedDestinationHash(destination.vertex));
                }
            }
        }
        Ok(Self { graph })
    }
    /// Decoded owned values for source-domain checks. Wire order is not native
    /// ID-map iteration order; unsaved Route fields and definition remapping are
    /// not supplied by this view. Never use it as captured effective yields.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }
    /// Graph.EditEdge is an exact keyed lookup, never insertion or reversal.
    pub fn edit_edge(&mut self, from: Vertex, to: Vertex) -> Option<edge::Cursor<'_>> {
        self.graph.get_edge_mut(from, to).map(edge::Cursor::new)
    }
    pub fn edit_city_edge(
        &mut self,
        from: ComponentId,
        to: ComponentId,
    ) -> Option<edge::Cursor<'_>> {
        self.edit_edge(city_vertex(from), city_vertex(to))
    }
    /// Manager.HasOutgoingRoute2c0b900 -> Graph.GetRoute requires only an
    /// active edge, then compares all three Trader ComponentID fields.
    pub fn has_outgoing_route(
        &self,
        from: ComponentId,
        to: ComponentId,
        trader: ComponentId,
    ) -> bool {
        self.graph
            .get_edge(city_vertex(from), city_vertex(to))
            .is_some_and(|e| e.has_active_route && e.most_recent_route.trader_unit == trader)
    }
    /// Exact Graph.GetPath0x2bfbc50: absent edge or empty vector returns null.
    /// Active flag, traversal cursor/direction and Route endpoints do not filter
    /// lookup. Both native key tables resolve full Vertex equality after hashes.
    pub fn get_path(&self, from: Vertex, to: Vertex) -> Option<&[i32]> {
        let path = &self.graph.get_edge(from, to)?.most_recent_path;
        (!path.is_empty()).then_some(path.as_slice())
    }
    /// Graph.GetPathToCity / Manager.GetExistingPath, with no reverse fallback.
    pub fn city_path(&self, from: ComponentId, to: ComponentId) -> Option<&[i32]> {
        self.get_path(city_vertex(from), city_vertex(to))
    }
    /// Only the reverse Graph lookup/copy projection inside FindNewPath. Its
    /// earlier Map/GetCity checks must be applied by the calling gameplay layer.
    /// A miss does not implement or bypass the pooled pathfinder branch.
    pub fn reverse_city_path(&self, from: ComponentId, to: ComponentId) -> Option<ReversePath<'_>> {
        self.city_path(to, from).map(|nodes| ReversePath { nodes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trade::{Destination, Edge, Origin, Route};
    fn city(owner: i16, id: u32) -> ComponentId {
        ComponentId {
            owner,
            id,
            component_type: 5,
        }
    }
    fn edge(from: Vertex, to: Vertex, id: i32, path: Vec<i32>) -> Destination {
        let c = city(1, 4);
        Destination {
            vertex: to,
            edge: Edge {
                wire_version: 3,
                id,
                most_recent_route: Route {
                    id: 99,
                    start_turn: 0,
                    trader_unit: c,
                    resource_type_hash: 0,
                    resource_quantity: 0,
                    origin_city: c,
                    destination_city: c,
                    origin_yields: vec![],
                    destination_yields: vec![],
                },
                has_active_route: false,
                routes_started: 0,
                routes_completed: 0,
                most_recent_path: path,
                path_traversal_index: 99,
                path_traversal_going_forward: false,
                from_vertex: from,
                to_vertex: to,
            },
        }
    }
    fn state(origins: Vec<Origin>) -> GraphState {
        let graph = Graph::new(42, origins).unwrap();
        GraphState::decode(
            &graph.encode(trade::Limits::default()).unwrap(),
            trade::Limits::default(),
        )
        .unwrap()
    }
    #[test]
    fn saved_edge1_path_queries_do_not_require_legacy_encoding() {
        let (a, b) = (city_vertex(city(1, 2)), city_vertex(city(3, 4)));
        let current = state(vec![Origin {
            vertex: a,
            destinations: vec![edge(a, b, 1, vec![7, 7, 2])],
        }]);
        let mut bytes = current.graph().encode(trade::Limits::default()).unwrap();
        bytes[48..52].copy_from_slice(&1i32.to_le_bytes());
        let legacy = GraphState::decode(&bytes, trade::Limits::default()).unwrap();
        assert_eq!(legacy.get_path(a, b), Some(&[7, 7, 2][..]));
        assert!(legacy.graph().logical_eq(current.graph()));
        assert_eq!(
            legacy.graph().encode(trade::Limits::default()).unwrap_err(),
            trade::Error::UnsupportedLegacyWrite
        );
    }
    #[test]
    fn city_key_uses_signed_owner_full_id_and_ignores_component_type() {
        let a = city(-2, 0xf1230007);
        let mut b = a;
        b.component_type = -1;
        assert_eq!(city_vertex(a), city_vertex(b));
        assert_eq!(
            city_vertex(a),
            Vertex {
                kind: 0,
                data1: -2,
                data2: 0xf1230007u32 as i32
            }
        );
        assert_ne!(city_vertex(a), city_vertex(city(-2, 0x71230007)));
    }
    #[test]
    fn forward_and_reverse_are_distinct_and_do_not_follow_route_or_cursor_flags() {
        let (a, b) = (city(1, 2), city(3, 4));
        let (x, y) = (city_vertex(a), city_vertex(b));
        let s = state(vec![
            Origin {
                vertex: x,
                destinations: vec![edge(x, y, 1, vec![7, 7, 2])],
            },
            Origin {
                vertex: y,
                destinations: vec![edge(y, x, 2, vec![9, 1, 5])],
            },
        ]);
        assert_eq!(s.city_path(a, b), Some([7, 7, 2].as_slice()));
        assert_eq!(
            s.reverse_city_path(a, b)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            [5, 1, 9]
        );
        assert_eq!(s.city_path(b, a), Some([9, 1, 5].as_slice()));
        assert!(!s.graph().get_edge(x, y).unwrap().has_active_route);
    }
    #[test]
    fn empty_forward_is_miss_even_if_reverse_exists_and_unplaced_values_are_not_normalized() {
        let (a, b) = (city(1, 2), city(3, 4));
        let (x, y) = (city_vertex(a), city_vertex(b));
        let s = state(vec![
            Origin {
                vertex: x,
                destinations: vec![edge(x, y, 1, vec![])],
            },
            Origin {
                vertex: y,
                destinations: vec![edge(y, x, 2, vec![i32::MIN, -1, i32::MAX])],
            },
        ]);
        assert!(s.city_path(a, b).is_none());
        assert_eq!(
            s.reverse_city_path(a, b)
                .unwrap()
                .iter()
                .collect::<Vec<_>>(),
            [i32::MAX, -1, i32::MIN]
        );
        assert!(s.city_path(a, city(3, 0x10004)).is_none());
    }
    #[test]
    fn native_equal_hash_vertices_remain_distinct_keys() {
        let a = city_vertex(city(1, u32::MAX));
        let b = city_vertex(city(2, u32::MAX));
        assert_ne!(a, b);
        assert_eq!(vertex_hash(a), vertex_hash(b));
        let x = city_vertex(city(4, 8));
        let s = state(vec![
            Origin {
                vertex: x,
                destinations: vec![edge(x, a, 1, vec![11]), edge(x, b, 2, vec![22])],
            },
            Origin {
                vertex: a,
                destinations: vec![edge(a, x, 3, vec![33])],
            },
            Origin {
                vertex: b,
                destinations: vec![edge(b, x, 4, vec![44])],
            },
        ]);
        assert_eq!(s.get_path(x, a), Some([11].as_slice()));
        assert_eq!(s.get_path(x, b), Some([22].as_slice()));
        assert_eq!(s.get_path(a, x), Some([33].as_slice()));
        assert_eq!(s.get_path(b, x), Some([44].as_slice()));
    }
    #[test]
    fn native_inner_sentinel_rejects_only_destination_and_exact_frame_consumption() {
        let v = Vertex {
            kind: 0,
            data1: 3,
            data2: 1621429197,
        };
        assert_eq!(vertex_hash(v), u32::MAX);
        let x = city_vertex(city(0, 1));
        let graph = Graph::new(
            0,
            vec![Origin {
                vertex: x,
                destinations: vec![edge(x, v, 1, vec![2])],
            }],
        )
        .unwrap();
        assert!(
            matches!(GraphState::decode(&graph.encode(trade::Limits::default()).unwrap(),trade::Limits::default()),Err(Error::UnsupportedDestinationHash(key)) if key==v)
        );
        let valid = state(vec![Origin {
            vertex: v,
            destinations: vec![],
        }]);
        assert_eq!(valid.graph().origins().len(), 1);
        let mut bytes = valid.graph().encode(trade::Limits::default()).unwrap();
        bytes.push(0);
        assert!(matches!(
            GraphState::decode(&bytes, trade::Limits::default()),
            Err(Error::Codec(trade::Error::TrailingBytes))
        ));
    }
    #[test]
    fn current_cursor_bounces_in_place_without_changing_graph_keys_or_order() {
        let (a, b) = (city(1, 2), city(3, 4));
        let (x, y) = (city_vertex(a), city_vertex(b));
        let mut forward = edge(x, y, 17, vec![7, 7, 9]);
        forward.edge.has_active_route = true;
        forward.edge.path_traversal_index = 0;
        forward.edge.path_traversal_going_forward = false;
        let mut s = state(vec![
            Origin {
                vertex: x,
                destinations: vec![forward],
            },
            Origin {
                vertex: y,
                destinations: vec![edge(y, x, 18, vec![99, 88])],
            },
        ]);
        let before = s.graph.clone();
        let values = (0..5)
            .map(|_| {
                s.edit_city_edge(a, b)
                    .unwrap()
                    .advance_path_to_next_plot()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(values, [7, 9, 7, 7, 7]);
        let current = s.graph().get_edge_by_id(17).unwrap();
        assert_eq!(current.path_traversal_index, 1);
        assert!(current.path_traversal_going_forward);
        let mut expected = before.clone();
        let changed = expected.get_edge_mut(x, y).unwrap();
        changed.path_traversal_index = 1;
        changed.path_traversal_going_forward = true;
        assert!(s.graph().wire_state_eq(&expected));
        assert!(s.graph().iteration_order_eq(&before));
        assert_eq!(s.city_path(a, b), Some([7, 7, 9].as_slice()));
        assert_eq!(s.city_path(b, a), Some([99, 88].as_slice()));
        assert!(s.edit_city_edge(a, city(3, 0x10004)).is_none());
    }
    #[test]
    fn native_advance_guards_are_noops_but_bad_next_index_retains_prefix() {
        let (x, y) = (city_vertex(city(1, 2)), city_vertex(city(3, 4)));
        let mut valid = edge(x, y, 17, vec![10, 11, 12]).edge;
        valid.has_active_route = true;
        valid.path_traversal_index = 0;
        for kind in 0..6 {
            let mut e = valid.clone();
            match kind {
                0 => e.id = -1,
                1 => e.from_vertex.kind = -1,
                2 => e.to_vertex.kind = -1,
                3 => e.has_active_route = false,
                4 => e.most_recent_path.clear(),
                _ => e.path_traversal_index = -1,
            }
            let before = e.clone();
            assert_eq!(
                edge::Cursor::new(&mut e).advance_path_to_next_plot(),
                Ok(-1)
            );
            assert_eq!(e, before);
        }
        let mut e = valid.clone();
        e.most_recent_path = vec![10];
        assert_eq!(
            edge::Cursor::new(&mut e).advance_path_to_next_plot(),
            Err(edge::Error::PathIndexOutOfBounds {
                index: 1,
                path_len: 1
            })
        );
        assert_eq!(e.path_traversal_index, 1);
        assert!(e.path_traversal_going_forward);
        let mut e = valid.clone();
        e.path_traversal_index = 3;
        e.path_traversal_going_forward = false;
        assert_eq!(
            edge::Cursor::new(&mut e).advance_path_to_next_plot(),
            Ok(12)
        );
        assert_eq!(e.path_traversal_index, 2);
        let mut e = valid;
        e.path_traversal_index = i32::MAX;
        e.path_traversal_going_forward = true;
        assert!(matches!(
            edge::Cursor::new(&mut e).advance_path_to_next_plot(),
            Err(edge::Error::PathIndexOutOfBounds {
                index: i32::MIN,
                ..
            })
        ));
        assert_eq!(e.path_traversal_index, i32::MIN);
    }
    #[test]
    fn outgoing_route_requires_active_directed_edge_and_full_trader_identity() {
        let (a, b) = (city(1, 2), city(3, 4));
        let (x, y) = (city_vertex(a), city_vertex(b));
        let mut d = edge(x, y, -1, vec![]);
        d.edge.has_active_route = true;
        let trader = d.edge.most_recent_route.trader_unit;
        let s = state(vec![Origin {
            vertex: x,
            destinations: vec![d],
        }]);
        assert!(s.has_outgoing_route(a, b, trader)); // no Edge.IsValid/path guard
        assert!(!s.has_outgoing_route(b, a, trader));
        for changed in [
            ComponentId { owner: 2, ..trader },
            ComponentId {
                id: trader.id + 0x10000,
                ..trader
            },
            ComponentId {
                component_type: 1,
                ..trader
            },
        ] {
            assert!(!s.has_outgoing_route(a, b, changed));
        }
        assert!(s.has_outgoing_route(
            ComponentId {
                component_type: -1,
                ..a
            },
            b,
            trader
        ));
        let mut inactive = edge(x, y, 17, vec![1, 2]);
        inactive.edge.most_recent_route.trader_unit = trader;
        let s = state(vec![Origin {
            vertex: x,
            destinations: vec![inactive],
        }]);
        assert!(!s.has_outgoing_route(a, b, trader));
    }
}
