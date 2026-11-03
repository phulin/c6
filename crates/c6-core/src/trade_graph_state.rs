//! Owned immutable Trade Graph keys/paths rebuilt through the existing codec.
//!
//! Graph.Read reconstructs forward/reverse keyed indexes from the saved entries.
//! This boundary supports point path lookup, not native ID-map callback order,
//! omitted Route state, route edits or the pooled cold pathfinder. Attribution
//! and one-time frame checks belong to LoadedState's component admission.
use crate::trade::{self, ComponentId, Graph, Vertex};
use crate::trade_read_order::vertex_hash;

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
/// Unrelated Unit writes do not alter this owner. A future operation that edits
/// Trade graph keys/paths must implement those source effects or reject before
/// claiming success; it cannot continue querying a detached initial snapshot.
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
}
