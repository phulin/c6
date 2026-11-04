//! Trade.Edge.AdvancePathToNextPlot2bf9210, over the canonical edge payload.
use crate::trade::Edge;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Native proceeds to an out-of-range vector access after the cursor write.
    /// Preserve that prefix, but do not manufacture a logical plot value.
    PathIndexOutOfBounds { index: i32, path_len: usize },
}

/// Mutable traversal fields only: graph keys, ID, path and route cannot be
/// replaced through this view, so all canonical indexes/order stay coherent.
pub struct Cursor<'a> {
    edge: &'a mut Edge,
}
impl<'a> Cursor<'a> {
    pub(crate) fn new(edge: &'a mut Edge) -> Self {
        Self { edge }
    }
    pub fn edge(&self) -> &Edge {
        self.edge
    }

    pub fn advance_path_to_next_plot(&mut self) -> Result<i32, Error> {
        let e = &mut self.edge;
        if e.id == -1
            || e.from_vertex.kind == -1
            || e.to_vertex.kind == -1
            || !e.has_active_route
            || e.most_recent_path.is_empty()
            || e.path_traversal_index < 0
        {
            return Ok(-1);
        }
        let last = (e.most_recent_path.len() as u32).wrapping_sub(1) as i32;
        if e.path_traversal_index == last {
            e.path_traversal_going_forward = false;
        }
        // Independent test, not else-if. For a one-element path this wins.
        if e.path_traversal_index == 0 {
            e.path_traversal_going_forward = true;
        }
        let step = if e.path_traversal_going_forward {
            1
        } else {
            -1
        };
        e.path_traversal_index = e.path_traversal_index.wrapping_add(step);
        e.most_recent_path
            .get(e.path_traversal_index as u32 as usize)
            .copied()
            .ok_or(Error::PathIndexOutOfBounds {
                index: e.path_traversal_index,
                path_len: e.most_recent_path.len(),
            })
    }
}
