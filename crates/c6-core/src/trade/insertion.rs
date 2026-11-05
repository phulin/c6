//! Narrow keyed insert used by live Graph.AddEdge. Hash iteration order is not
//! represented as an allocator/container observation by this logical graph.
use super::{Destination, Edge, Error, Graph, Origin};
impl Graph {
    pub(crate) fn insert_edge(&mut self, edge: Edge) -> Result<(), Error> {
        let (from, to) = (edge.from_vertex, edge.to_vertex);
        if self.edge_index.contains_key(&(from, to)) {
            return Err(Error::DuplicateDestination);
        }
        if self.id_index.contains_key(&edge.id) {
            return Err(Error::DuplicateEdgeId(edge.id));
        }
        let i = if let Some(&i) = self.origin_index.get(&from) {
            i
        } else {
            let i = self.origins.len();
            self.origins.push(Origin {
                vertex: from,
                destinations: vec![],
            });
            self.origin_index.insert(from, i);
            i
        };
        let j = self.origins[i].destinations.len();
        self.id_index.insert(edge.id, (i, j));
        self.edge_index.insert((from, to), (i, j));
        self.origins[i]
            .destinations
            .push(Destination { vertex: to, edge });
        self.total_edges_created = self.total_edges_created.wrapping_add(1);
        Ok(())
    }
}
