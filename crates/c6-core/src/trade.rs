//! Current Linux XP2 Trade graph wire codec (Graph 1 / Edge 3 / Route 4 / Vertex 1).
//!
//! Evidence: `docs/linux-trade-serialization-contracts.md`, pinned GameCore SHA-256
//! a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! This is a codec for an extracted graph, not a game save loader or trade rules.
//! Wire order is retained; native ID-map callback order is not serialized here.
//! Route's live `m_bFinalizing` and other nonserialized state remain unsupported.
//! Duplicate keys, noncanonical flags/type words and inconsistent endpoints are
//! rejected by this supported contract, without claiming native Read rejects them.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Vertex {
    /// Raw native signed 16-bit enum, sign-extended on the wire. No enum guesswork.
    pub kind: i16,
    pub data1: i32,
    pub data2: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentId {
    pub owner: i16,
    pub component_type: i16,
    pub id: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YieldValue {
    pub type_hash: u32,
    /// Native FixedPointT<8> representation, without floating-point conversion.
    pub raw: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    pub id: i32,
    pub start_turn: i32,
    pub trader_unit: ComponentId,
    pub resource_type_hash: u32,
    pub resource_quantity: i32,
    pub origin_city: ComponentId,
    pub destination_city: ComponentId,
    /// Current definition order, including definition identity and raw value.
    pub origin_yields: Vec<YieldValue>,
    pub destination_yields: Vec<YieldValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    pub id: i32,
    pub most_recent_route: Route,
    pub has_active_route: bool,
    pub routes_started: i32,
    pub routes_completed: i32,
    pub most_recent_path: Vec<i32>,
    pub path_traversal_index: i32,
    pub path_traversal_going_forward: bool,
    pub from_vertex: Vertex,
    pub to_vertex: Vertex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Destination {
    pub vertex: Vertex,
    pub edge: Edge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin {
    pub vertex: Vertex,
    /// Exact inner-map serialization order; never sorted by the codec.
    pub destinations: Vec<Destination>,
}

/// Ordered wire state plus auxiliary indexes. The indexes never control iteration.
#[derive(Clone, Debug)]
pub struct Graph {
    total_edges_created: i32,
    origins: Vec<Origin>,
    origin_index: HashMap<Vertex, usize>,
    edge_index: HashMap<(Vertex, Vertex), (usize, usize)>,
    id_index: HashMap<i32, (usize, usize)>,
}

/// Resource policy, independent of gameplay constraints. Counts are also bounded
/// by remaining input before allocation. `max_items` is an aggregate budget for
/// origins, destinations, path entries and yield pairs, not a native engine cap.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_items: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_items: 1_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    TrailingBytes,
    Version { expected: i32, actual: i32 },
    NegativeCount,
    ResourceLimit,
    NoncanonicalVertexType(i32),
    NoncanonicalBool(u8),
    DuplicateOrigin,
    DuplicateDestination,
    DuplicateEdgeId(i32),
    DuplicateYieldHash(u32),
    EndpointMismatch,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

impl Graph {
    /// Build supported wire state without reordering or assigning new IDs.
    pub fn new(total_edges_created: i32, origins: Vec<Origin>) -> Result<Self, Error> {
        let mut graph = Self {
            total_edges_created,
            origins,
            origin_index: HashMap::new(),
            edge_index: HashMap::new(),
            id_index: HashMap::new(),
        };
        for (i, origin) in graph.origins.iter().enumerate() {
            if graph.origin_index.insert(origin.vertex, i).is_some() {
                return Err(Error::DuplicateOrigin);
            }
            for (j, destination) in origin.destinations.iter().enumerate() {
                let edge = &destination.edge;
                if graph
                    .edge_index
                    .insert((origin.vertex, destination.vertex), (i, j))
                    .is_some()
                {
                    return Err(Error::DuplicateDestination);
                }
                if graph.id_index.insert(edge.id, (i, j)).is_some() {
                    return Err(Error::DuplicateEdgeId(edge.id));
                }
                if edge.from_vertex != origin.vertex || edge.to_vertex != destination.vertex {
                    return Err(Error::EndpointMismatch);
                }
                edge.most_recent_route.validate()?;
            }
        }
        Ok(graph)
    }
    pub fn total_edges_created(&self) -> i32 {
        self.total_edges_created
    }
    pub fn origins(&self) -> &[Origin] {
        &self.origins
    }
    pub fn edge_count(&self) -> usize {
        self.id_index.len()
    }
    pub fn get_edge(&self, from: Vertex, to: Vertex) -> Option<&Edge> {
        self.edge_index
            .get(&(from, to))
            .map(|&(i, j)| &self.origins[i].destinations[j].edge)
    }
    pub fn get_edge_by_id(&self, id: i32) -> Option<&Edge> {
        self.id_index
            .get(&id)
            .map(|&(i, j)| &self.origins[i].destinations[j].edge)
    }
    /// Includes empty origin membership and every keyed Edge payload. Ignores only
    /// outer/inner graph iteration order; path and yield definition order still count.
    /// Does not establish live-state equality or future callback/gameplay equality.
    pub fn logical_eq(&self, other: &Self) -> bool {
        self.total_edges_created == other.total_edges_created
            && self.origins.len() == other.origins.len()
            && self.edge_count() == other.edge_count()
            && self
                .origins
                .iter()
                .all(|origin| other.origin_index.contains_key(&origin.vertex))
            && self.origins.iter().all(|origin| {
                origin
                    .destinations
                    .iter()
                    .all(|d| other.get_edge(origin.vertex, d.vertex) == Some(&d.edge))
            })
    }
    /// Compares observed outer and inner vertex order, independently of payloads.
    pub fn iteration_order_eq(&self, other: &Self) -> bool {
        self.origins.len() == other.origins.len()
            && self.origins.iter().zip(&other.origins).all(|(a, b)| {
                a.vertex == b.vertex
                    && a.destinations.len() == b.destinations.len()
                    && a.destinations
                        .iter()
                        .zip(&b.destinations)
                        .all(|(a, b)| a.vertex == b.vertex)
            })
    }
    pub fn wire_state_eq(&self, other: &Self) -> bool {
        self.total_edges_created == other.total_edges_created && self.origins == other.origins
    }
    /// Requires exactly one extracted Graph: no trailing Manager fields accepted.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, limits)?;
        r.version(1)?;
        let total = r.i32()?;
        let count = r.count(true, 20)?;
        let mut origins = Vec::with_capacity(count);
        for _ in 0..count {
            let vertex = r.vertex()?;
            let count = r.count(true, 126)?;
            let mut destinations = Vec::with_capacity(count);
            for _ in 0..count {
                destinations.push(Destination {
                    vertex: r.vertex()?,
                    edge: r.edge()?,
                });
            }
            origins.push(Origin {
                vertex,
                destinations,
            });
        }
        r.finish()?;
        Self::new(total, origins)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let mut w = Writer::new(limits);
        w.i32(1)?;
        w.i32(self.total_edges_created)?;
        w.count(self.origins.len(), true)?;
        for origin in &self.origins {
            w.vertex(origin.vertex)?;
            w.count(origin.destinations.len(), true)?;
            for destination in &origin.destinations {
                w.vertex(destination.vertex)?;
                w.edge(&destination.edge)?;
            }
        }
        Ok(w.bytes)
    }
}

impl Route {
    fn validate(&self) -> Result<(), Error> {
        for yields in [&self.origin_yields, &self.destination_yields] {
            let mut seen = HashSet::new();
            for value in yields {
                if !seen.insert(value.type_hash) {
                    return Err(Error::DuplicateYieldHash(value.type_hash));
                }
            }
        }
        Ok(())
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, limits)?;
        let route = r.route()?;
        r.finish()?;
        Ok(route)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut w = Writer::new(limits);
        w.route(self)?;
        Ok(w.bytes)
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    items_left: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(Self {
            bytes,
            offset: 0,
            items_left: limits.max_items,
        })
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.offset.checked_add(N).ok_or(Error::Truncated)?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap();
        self.offset = end;
        Ok(result)
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take()?))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn count(&mut self, signed: bool, min_bytes: usize) -> Result<usize, Error> {
        let count = self.u32()?;
        if signed && count > i32::MAX as u32 {
            return Err(Error::NegativeCount);
        }
        let count = usize::try_from(count).map_err(|_| Error::ResourceLimit)?;
        self.items_left = self
            .items_left
            .checked_sub(count)
            .ok_or(Error::ResourceLimit)?;
        if count > (self.bytes.len() - self.offset) / min_bytes {
            return Err(Error::Truncated);
        }
        Ok(count)
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take::<1>()?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::NoncanonicalBool(x)),
        }
    }
    fn vertex(&mut self) -> Result<Vertex, Error> {
        self.version(1)?;
        let raw = self.i32()?;
        let kind = i16::try_from(raw).map_err(|_| Error::NoncanonicalVertexType(raw))?;
        Ok(Vertex {
            kind,
            data1: self.i32()?,
            data2: self.i32()?,
        })
    }
    fn component(&mut self) -> Result<ComponentId, Error> {
        Ok(ComponentId {
            owner: i16::from_le_bytes(self.take()?),
            component_type: i16::from_le_bytes(self.take()?),
            id: self.u32()?,
        })
    }
    fn yields(&mut self) -> Result<Vec<YieldValue>, Error> {
        let count = self.count(false, 8)?;
        let mut values = Vec::with_capacity(count);
        let mut seen = HashSet::new();
        for _ in 0..count {
            let type_hash = self.u32()?;
            let raw = self.i32()?;
            if !seen.insert(type_hash) {
                return Err(Error::DuplicateYieldHash(type_hash));
            }
            values.push(YieldValue { type_hash, raw });
        }
        Ok(values)
    }
    fn route(&mut self) -> Result<Route, Error> {
        self.version(4)?;
        Ok(Route {
            id: self.i32()?,
            start_turn: self.i32()?,
            trader_unit: self.component()?,
            resource_type_hash: self.u32()?,
            resource_quantity: self.i32()?,
            origin_city: self.component()?,
            destination_city: self.component()?,
            origin_yields: self.yields()?,
            destination_yields: self.yields()?,
        })
    }
    fn edge(&mut self) -> Result<Edge, Error> {
        self.version(3)?;
        let id = self.i32()?;
        let most_recent_route = self.route()?;
        let has_active_route = self.boolean()?;
        let routes_started = self.i32()?;
        let routes_completed = self.i32()?;
        let count = self.count(false, 4)?;
        let mut most_recent_path = Vec::with_capacity(count);
        for _ in 0..count {
            most_recent_path.push(self.i32()?);
        }
        Ok(Edge {
            id,
            most_recent_route,
            has_active_route,
            routes_started,
            routes_completed,
            most_recent_path,
            path_traversal_index: self.i32()?,
            path_traversal_going_forward: self.boolean()?,
            from_vertex: self.vertex()?,
            to_vertex: self.vertex()?,
        })
    }
    fn finish(&self) -> Result<(), Error> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::TrailingBytes)
        }
    }
}

struct Writer {
    bytes: Vec<u8>,
    limits: Limits,
    items: usize,
}
impl Writer {
    fn new(limits: Limits) -> Self {
        Self {
            bytes: Vec::new(),
            limits,
            items: 0,
        }
    }
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.limits.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(Error::ResourceLimit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn i32(&mut self, value: i32) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn u32(&mut self, value: u32) -> Result<(), Error> {
        self.put(&value.to_le_bytes())
    }
    fn count(&mut self, count: usize, signed: bool) -> Result<(), Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || (signed && count > i32::MAX as usize) {
            return Err(Error::ResourceLimit);
        }
        self.u32(u32::try_from(count).map_err(|_| Error::ResourceLimit)?)
    }
    fn vertex(&mut self, v: Vertex) -> Result<(), Error> {
        self.i32(1)?;
        self.i32(i32::from(v.kind))?;
        self.i32(v.data1)?;
        self.i32(v.data2)
    }
    fn component(&mut self, c: ComponentId) -> Result<(), Error> {
        self.put(&c.owner.to_le_bytes())?;
        self.put(&c.component_type.to_le_bytes())?;
        self.u32(c.id)
    }
    fn yields(&mut self, values: &[YieldValue]) -> Result<(), Error> {
        self.count(values.len(), false)?;
        for v in values {
            self.u32(v.type_hash)?;
            self.i32(v.raw)?;
        }
        Ok(())
    }
    fn route(&mut self, r: &Route) -> Result<(), Error> {
        self.i32(4)?;
        self.i32(r.id)?;
        self.i32(r.start_turn)?;
        self.component(r.trader_unit)?;
        self.u32(r.resource_type_hash)?;
        self.i32(r.resource_quantity)?;
        self.component(r.origin_city)?;
        self.component(r.destination_city)?;
        self.yields(&r.origin_yields)?;
        self.yields(&r.destination_yields)
    }
    fn edge(&mut self, e: &Edge) -> Result<(), Error> {
        self.i32(3)?;
        self.i32(e.id)?;
        self.route(&e.most_recent_route)?;
        self.put(&[u8::from(e.has_active_route)])?;
        self.i32(e.routes_started)?;
        self.i32(e.routes_completed)?;
        self.count(e.most_recent_path.len(), false)?;
        for &plot in &e.most_recent_path {
            self.i32(plot)?;
        }
        self.i32(e.path_traversal_index)?;
        self.put(&[u8::from(e.path_traversal_going_forward)])?;
        self.vertex(e.from_vertex)?;
        self.vertex(e.to_vertex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vertex(n: i32) -> Vertex {
        Vertex {
            kind: -1,
            data1: n,
            data2: i32::MIN,
        }
    }
    fn route() -> Route {
        let component = ComponentId {
            owner: i16::MIN,
            component_type: i16::MAX,
            id: u32::MAX,
        };
        Route {
            id: -7,
            start_turn: i32::MIN,
            trader_unit: component,
            resource_type_hash: u32::MAX,
            resource_quantity: i32::MAX,
            origin_city: component,
            destination_city: component,
            origin_yields: vec![
                YieldValue {
                    type_hash: 0,
                    raw: i32::MIN,
                },
                YieldValue {
                    type_hash: u32::MAX,
                    raw: i32::MAX,
                },
            ],
            destination_yields: vec![],
        }
    }
    fn edge(id: i32, from: Vertex, to: Vertex) -> Destination {
        Destination {
            vertex: to,
            edge: Edge {
                id,
                most_recent_route: route(),
                has_active_route: true,
                routes_started: -2,
                routes_completed: i32::MAX,
                most_recent_path: vec![i32::MAX, -1, 0, i32::MIN],
                path_traversal_index: -99,
                path_traversal_going_forward: false,
                from_vertex: from,
                to_vertex: to,
            },
        }
    }
    fn graph() -> Graph {
        Graph::new(
            -1,
            vec![
                Origin {
                    vertex: vertex(1),
                    destinations: vec![
                        edge(-1, vertex(1), vertex(2)),
                        edge(5, vertex(1), vertex(3)),
                    ],
                },
                Origin {
                    vertex: vertex(9),
                    destinations: vec![],
                },
            ],
        )
        .unwrap()
    }
    fn limits() -> Limits {
        Limits::default()
    }

    #[test]
    fn independent_route_wire_preserves_integer_boundaries_and_definition_order() {
        let r = route();
        // Independently specified wire words, including packed signed ComponentID halves.
        let words: [u32; 17] = [
            4,
            (-7i32) as u32,
            0x80000000,
            0x7fff8000,
            0xffffffff,
            0xffffffff,
            0x7fffffff,
            0x7fff8000,
            0xffffffff,
            0x7fff8000,
            0xffffffff,
            2,
            0,
            0x80000000,
            0xffffffff,
            0x7fffffff,
            0,
        ];
        let expected: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        assert_eq!(r.encode(limits()).unwrap(), expected);
        assert_eq!(Route::decode(&expected, limits()).unwrap(), r);
        let mut reversed = r.clone();
        reversed.origin_yields.reverse();
        assert_ne!(
            r.encode(limits()).unwrap(),
            reversed.encode(limits()).unwrap()
        );
    }
    #[test]
    fn roundtrip_indexes_and_distinct_order_comparison() {
        let a = graph();
        let bytes = a.encode(limits()).unwrap();
        let b = Graph::decode(&bytes, limits()).unwrap();
        assert!(a.wire_state_eq(&b));
        assert_eq!(b.encode(limits()).unwrap(), bytes);
        assert_eq!(a.get_edge(vertex(1), vertex(2)), a.get_edge_by_id(-1));
        assert!(a.get_edge(vertex(2), vertex(1)).is_none());
        assert!(a.get_edge_by_id(6).is_none());
        let mut origins = a.origins.clone();
        origins.reverse();
        origins[1].destinations.reverse();
        let reordered = Graph::new(-1, origins).unwrap();
        assert!(a.logical_eq(&reordered));
        assert!(!a.iteration_order_eq(&reordered));
        assert!(!a.wire_state_eq(&reordered));
        let mut origins = a.origins.clone();
        origins.pop();
        assert!(!a.logical_eq(&Graph::new(-1, origins).unwrap()));
        let mut origins = a.origins.clone();
        origins[0].destinations[0].edge.most_recent_path.reverse();
        let changed = Graph::new(-1, origins).unwrap();
        assert!(!a.logical_eq(&changed));
        assert!(a.iteration_order_eq(&changed));
    }
    #[test]
    fn truncation_trailing_versions_counts_flags_and_vertex_narrowing_fail_closed() {
        let bytes = graph().encode(limits()).unwrap();
        for n in 0..bytes.len() {
            assert!(Graph::decode(&bytes[..n], limits()).is_err(), "prefix {n}");
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::TrailingBytes)
        ));
        // Graph, outer Vertex, inner Vertex, Edge, Route versions in the first record.
        for offset in [0, 12, 32, 48, 56] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&99i32.to_le_bytes());
            assert!(matches!(
                Graph::decode(&bad, limits()),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::NegativeCount)
        ));
        let mut bad = bytes.clone();
        bad[16..20].copy_from_slice(&65535i32.to_le_bytes());
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::NoncanonicalVertexType(65535))
        ));
        // 56-byte prefix, followed by the independent 68-byte Route.
        let mut bad = bytes.clone();
        bad[124] = 2;
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::NoncanonicalBool(2))
        ));
        let mut bad = bytes.clone();
        bad[133..137].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::ResourceLimit)
        ));
        let mut bad = bytes;
        bad[100..104].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Graph::decode(&bad, limits()),
            Err(Error::ResourceLimit)
        ));
    }
    #[test]
    fn duplicate_and_endpoint_contract_guards() {
        let a = graph();
        let mut origins = a.origins.clone();
        origins.push(origins[0].clone());
        assert!(matches!(
            Graph::new(0, origins),
            Err(Error::DuplicateOrigin)
        ));
        let mut origins = a.origins.clone();
        let duplicate = origins[0].destinations[0].clone();
        origins[0].destinations.push(duplicate);
        assert!(matches!(
            Graph::new(0, origins),
            Err(Error::DuplicateDestination)
        ));
        let mut origins = a.origins.clone();
        origins[0].destinations[1].edge.id = -1;
        assert!(matches!(
            Graph::new(0, origins),
            Err(Error::DuplicateEdgeId(-1))
        ));
        let mut origins = a.origins.clone();
        origins[0].destinations[0].edge.to_vertex = vertex(77);
        assert!(matches!(
            Graph::new(0, origins),
            Err(Error::EndpointMismatch)
        ));
        let mut r = route();
        r.origin_yields[1].type_hash = 0;
        assert!(matches!(
            r.encode(limits()),
            Err(Error::DuplicateYieldHash(0))
        ));
        let mut bytes = route().encode(limits()).unwrap();
        bytes[56..60].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            Route::decode(&bytes, limits()),
            Err(Error::DuplicateYieldHash(0))
        ));
        // Native Read may accept/overwrite malformed duplicates: this codec does not.
        let mut bytes = a.encode(limits()).unwrap();
        let first_len = 126 + 16 + 16; // destination + Edge + two yield pairs + four path entries
        let second_edge_id = 32 + first_len + 16 + 4;
        bytes[second_edge_id..second_edge_id + 4].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(matches!(
            Graph::decode(&bytes, limits()),
            Err(Error::DuplicateEdgeId(-1))
        ));
    }
    #[test]
    fn empty_graph_and_aggregate_resource_boundaries() {
        let empty = Graph::new(i32::MAX, vec![]).unwrap();
        let bytes = empty.encode(limits()).unwrap();
        assert_eq!(bytes.len(), 12);
        assert!(empty.wire_state_eq(
            &Graph::decode(
                &bytes,
                Limits {
                    max_bytes: 12,
                    max_items: 0
                }
            )
            .unwrap()
        ));
        let g = graph();
        let bytes = g.encode(limits()).unwrap();
        // 2 origins + 2 destinations + 4 yield pairs + 8 path entries.
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: 16,
        };
        assert!(Graph::decode(&bytes, exact).is_ok());
        assert_eq!(g.encode(exact).unwrap(), bytes);
        for too_small in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: 15,
                ..exact
            },
        ] {
            assert!(matches!(
                Graph::decode(&bytes, too_small),
                Err(Error::ResourceLimit)
            ));
            assert!(matches!(g.encode(too_small), Err(Error::ResourceLimit)));
        }
    }
}
