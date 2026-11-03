//! Source-derived fresh Linux Trade Graph Read → Write order.
//!
//! Pinned XP2 and host evidence: `docs/linux-trade-read-order.md`.
//! Rebuilds only serialized origin/destination iteration for a fresh graph with
//! default containers. Does not reconstruct live ID/reverse maps, allocation,
//! archives, omitted Route fields, or gameplay state. Existing graph loads and
//! malformed duplicate-key native behavior are outside this validated boundary.
use crate::trade::{self, Graph, Vertex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ResourceLimit,
    /// Native inner Set_Base reserves this hash as its empty-slot sentinel.
    UnsupportedSentinelHash(Vertex),
    Codec(trade::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

// Host prime-policy growth from initial request 51, default load 1 and growth 2.
// This sequence covers our explicit one-million-origin resource policy.
const BUCKETS: &[usize] = &[
    53, 109, 227, 467, 953, 2029, 4349, 8783, 17749, 35933, 72817, 147793, 299951, 608903, 1236397,
];
const MAX_ITEMS: usize = 1_000_000;

/// Native Vertex::GetHash: OR packed fields (including signed data2 extension),
/// then MSVC2012 FNV-1a over eight little-endian bytes. Result is zero-extended.
pub fn vertex_hash(vertex: Vertex) -> u32 {
    let packed = ((vertex.kind as u16 as u64) << 48)
        | ((vertex.data1 as u32 as u64) << 32)
        | (vertex.data2 as i64 as u64);
    packed.to_le_bytes().into_iter().fold(0x811c9dc5, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x01000193)
    })
}

fn home(hash: u32, capacity: usize) -> usize {
    ((u64::from(hash.wrapping_mul(0x7f4a7c15)) * capacity as u64) >> 32) as usize
}

fn insert(slots: &mut [Option<(u32, usize)>], mut entry: (u32, usize)) {
    let mut at = home(entry.0, slots.len());
    let mut distance = 0;
    loop {
        match slots[at] {
            None => {
                slots[at] = Some(entry);
                return;
            }
            Some(resident) => {
                let resident_distance =
                    (at + slots.len() - home(resident.0, slots.len())) % slots.len();
                if resident_distance < distance {
                    slots[at] = Some(entry);
                    entry = resident;
                    distance = resident_distance;
                }
            }
        }
        distance += 1;
        at = (at + 1) % slots.len();
    }
}

fn inner_order(vertices: impl Iterator<Item = Vertex>) -> Vec<usize> {
    let mut slots = Vec::new();
    for (count, vertex) in vertices.enumerate() {
        // Native CanSet compares current (not prospective) single-precision load
        // and requires at least two empty slots before Set_Base.
        if slots.is_empty()
            || count as f32 / slots.len() as f32 > 0.7_f32
            || slots.len() - count <= 1
        {
            let mut grown = vec![None; (slots.len() * 2).max(4)];
            for entry in slots.into_iter().flatten() {
                insert(&mut grown, entry);
            }
            slots = grown;
        }
        insert(&mut slots, (vertex_hash(vertex), count));
    }
    slots
        .into_iter()
        .flatten()
        .map(|(_, index)| index)
        .collect()
}

/// Produce the next serialized graph order for fresh default native containers.
/// Payloads/IDs are unchanged. Input remains immutable. Resource policy is one
/// million origins and destinations; this is not an engine gameplay limit.
pub fn fresh_read_wire_order(graph: &Graph) -> Result<Graph, Error> {
    if graph.origins().len() > MAX_ITEMS || graph.edge_count() > MAX_ITEMS {
        return Err(Error::ResourceLimit);
    }
    for origin in graph.origins() {
        for destination in &origin.destinations {
            if vertex_hash(destination.vertex) == u32::MAX {
                return Err(Error::UnsupportedSentinelHash(destination.vertex));
            }
        }
    }
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); BUCKETS[0]];
    let mut growth = 0;
    for (index, origin) in graph.origins().iter().enumerate() {
        if index + 1 > buckets.len() {
            growth += 1;
            let capacity = *BUCKETS.get(growth).ok_or(Error::ResourceLimit)?;
            let mut grown = vec![Vec::new(); capacity];
            for old in &buckets {
                for &entry in old {
                    let hash = vertex_hash(graph.origins()[entry].vertex) as usize;
                    grown[hash % capacity].insert(0, entry);
                }
            }
            buckets = grown;
        }
        let at = vertex_hash(origin.vertex) as usize % buckets.len();
        buckets[at].insert(0, index);
    }
    let mut origins = Vec::with_capacity(graph.origins().len());
    for index in buckets.into_iter().flatten() {
        let source = &graph.origins()[index];
        let mut rebuilt = source.clone();
        rebuilt.destinations = inner_order(source.destinations.iter().map(|d| d.vertex))
            .into_iter()
            .map(|index| source.destinations[index].clone())
            .collect();
        origins.push(rebuilt);
    }
    Graph::new(graph.total_edges_created(), origins).map_err(Error::Codec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_origins_survive_and_counter_is_not_renumbered() {
        let graph = Graph::new(
            -1,
            vec![trade::Origin {
                vertex: Vertex {
                    kind: -1,
                    data1: i32::MIN,
                    data2: i32::MAX,
                },
                destinations: vec![],
            }],
        )
        .unwrap();
        let rebuilt = fresh_read_wire_order(&graph).unwrap();
        assert!(rebuilt.wire_state_eq(&graph));
        assert!(
            fresh_read_wire_order(&Graph::new(i32::MAX, vec![]).unwrap())
                .unwrap()
                .wire_state_eq(&Graph::new(i32::MAX, vec![]).unwrap())
        );
    }

    #[test]
    fn signed_or_packing_and_sentinel_boundary() {
        let a = Vertex {
            kind: 0,
            data1: 0,
            data2: -1,
        };
        let b = Vertex {
            kind: i16::MIN,
            data1: i32::MIN,
            data2: -1,
        };
        // The native sign extension fills every packed bit. These unequal
        // vertices deliberately collide; equality must remain separate.
        assert_ne!(a, b);
        assert_eq!(vertex_hash(a), vertex_hash(b));
        assert_eq!(
            vertex_hash(Vertex {
                kind: 0,
                data1: 3,
                data2: 1621429197
            }),
            u32::MAX
        );
    }
}
