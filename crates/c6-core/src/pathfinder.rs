//! Ordered Linux XP2 two-layer path search (Instance, 0x2a50940 onwards).
//!
//! This engine preserves native node identity and callback order. It supplies no
//! approximate movement policy: Formation callbacks and their inputs are required.
use crate::unit::{PathNode, PlotCoord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(pub usize);

#[derive(Clone, Debug)]
pub struct WorkingNode {
    pub parent: Option<NodeId>,
    pub total_cost_raw: i32,
    pub known_cost_raw: i32,
    pub exit_cost_raw: i32,
    pub heuristic: i32,
    pub moves_remaining_raw: i32,
    pub turn: i32,
    pub x: i16,
    pub y: i16,
    /// -1 unlinked, 0 open, 1 closed.
    pub status: i16,
    pub children: Vec<NodeId>,
    pub cache_flags: u32,
    pub cache_other: u32,
}
impl WorkingNode {
    pub fn location(&self) -> PlotCoord {
        PlotCoord {
            x: i32::from(self.x),
            y: i32::from(self.y),
        }
    }
    fn clear(&mut self) {
        self.parent = None;
        self.total_cost_raw = 0;
        self.known_cost_raw = 0;
        self.exit_cost_raw = 0;
        self.heuristic = 0;
        self.moves_remaining_raw = 0;
        self.turn = 0;
        self.status = -1;
        self.children.clear();
        self.cache_flags &= !1;
    }
}

pub trait PathCallbacks: Sized {
    type Error;
    fn initialize(&mut self, search: &mut TwoLayer) -> Result<(), Self::Error>;
    fn uninitialize(&mut self, search: &mut TwoLayer) -> Result<(), Self::Error>;
    fn destination_valid(&mut self, search: &mut TwoLayer) -> Result<bool, Self::Error>;
    fn is_destination(
        &mut self,
        search: &mut TwoLayer,
        location: PlotCoord,
    ) -> Result<bool, Self::Error>;
    /// Integer heuristic; Instance converts it to Q8 using native wrapping shift.
    fn heuristic(
        &mut self,
        search: &mut TwoLayer,
        from: PlotCoord,
        edge_cost_raw: i32,
    ) -> Result<i32, Self::Error>;
    fn valid(
        &mut self,
        search: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<i32, Self::Error>;
    fn cost(
        &mut self,
        search: &mut TwoLayer,
        parent: NodeId,
        child: NodeId,
    ) -> Result<i32, Self::Error>;
    fn add(
        &mut self,
        search: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Self::Error>;
    fn node_add(
        &mut self,
        search: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Self::Error>;
    fn extra_children_count(
        &mut self,
        search: &mut TwoLayer,
        parent: NodeId,
    ) -> Result<i32, Self::Error>;
    fn extra_child(
        &mut self,
        search: &mut TwoLayer,
        parent: NodeId,
        index: i32,
    ) -> Result<PlotCoord, Self::Error>;
}

pub struct TwoLayer {
    width: i32,
    height: i32,
    wrap_x: bool,
    wrap_y: bool,
    nodes: Vec<WorkingNode>,
    open: Vec<NodeId>,
    closed: Vec<NodeId>,
    pending: Vec<NodeId>,
    force_reset: bool,
    pub flags: u32,
    pub source: PlotCoord,
    pub destination: PlotCoord,
    pub aborted: bool,
    pub last_node: Option<NodeId>,
}
impl TwoLayer {
    /// Working-node coordinates are signed shorts in the target.
    pub fn new(width: i32, height: i32, wrap_x: bool, wrap_y: bool) -> Option<Self> {
        if width <= 0 || height <= 0 || width > i32::from(i16::MAX) || height > i32::from(i16::MAX)
        {
            return None;
        }
        let count = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(2)?;
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(count).ok()?;
        for _layer in 0..2 {
            for x in 0..width {
                for y in 0..height {
                    nodes.push(WorkingNode {
                        parent: None,
                        total_cost_raw: 0,
                        known_cost_raw: 0,
                        exit_cost_raw: 0,
                        heuristic: 0,
                        moves_remaining_raw: 0,
                        turn: 0,
                        x: x as i16,
                        y: y as i16,
                        status: -1,
                        children: Vec::new(),
                        cache_flags: 0,
                        cache_other: 0,
                    });
                }
            }
        }
        Some(Self {
            width,
            height,
            wrap_x,
            wrap_y,
            nodes,
            open: Vec::new(),
            closed: Vec::new(),
            pending: Vec::new(),
            force_reset: true,
            flags: 0,
            source: PlotCoord { x: -1, y: -1 },
            destination: PlotCoord { x: -1, y: -1 },
            aborted: false,
            last_node: None,
        })
    }
    pub fn node(&self, id: NodeId) -> &WorkingNode {
        &self.nodes[id.0]
    }
    pub fn node_mut(&mut self, id: NodeId) -> &mut WorkingNode {
        &mut self.nodes[id.0]
    }
    pub fn normal_node(&self, x: i32, y: i32) -> Option<NodeId> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| NodeId(x as usize * self.height as usize + y as usize))
    }
    pub fn partial_node(&self, x: i32, y: i32) -> Option<NodeId> {
        self.normal_node(x, y)
            .map(|id| NodeId(id.0 + self.width as usize * self.height as usize))
    }
    pub fn invalidate(&mut self) {
        self.force_reset = true;
    }
    fn key(&self, id: NodeId) -> (i32, i32) {
        let n = self.node(id);
        (n.total_cost_raw, n.known_cost_raw)
    }
    /// AddToOpen's direction-dependent equal-key insertion is significant. It
    /// chooses a traversal end using binary32 distances, then inserts before the
    /// first equal key encountered from that end.
    pub fn add_to_open<C: PathCallbacks>(&mut self, c: &mut C, id: NodeId) -> Result<(), C::Error> {
        self.node_mut(id).status = 0;
        let key = self.key(id);
        let (at, parent, event) = if self.open.is_empty() {
            (0, None, 1)
        } else if key < self.key(self.open[0]) {
            (0, Some(self.open[0]), 1)
        } else if key > self.key(*self.open.last().unwrap()) {
            (self.open.len(), Some(*self.open.last().unwrap()), 0)
        } else {
            let head = self.key(self.open[0]).0;
            let tail = self.key(*self.open.last().unwrap()).0;
            let distance = |other: i32| ((key.0.wrapping_sub(other) as f32) / 256.0).abs();
            if distance(head) > distance(tail) {
                let at = self
                    .open
                    .iter()
                    .rposition(|&old| key >= self.key(old))
                    .map_or(0, |i| i + 1);
                (at, at.checked_sub(1).map(|i| self.open[i]), 0)
            } else {
                let at = self
                    .open
                    .iter()
                    .position(|&old| key <= self.key(old))
                    .unwrap_or(self.open.len());
                (
                    at,
                    at.checked_sub(1).map(|i| self.open[i]),
                    if at == 0 { 1 } else { 0 },
                )
            }
        };
        self.open.insert(at, id);
        c.node_add(self, parent, id, event)
    }
    /// UpdateOpenNode compares only total cost, preserving its existing position
    /// when it ties its predecessor; this differs from AddToOpen.
    pub fn update_open(&mut self, id: NodeId) {
        let Some(at) = self.open.iter().position(|&n| n == id) else {
            return;
        };
        if at == 0 || self.node(id).total_cost_raw >= self.node(self.open[at - 1]).total_cost_raw {
            return;
        }
        self.open.remove(at);
        let value = self.node(id).total_cost_raw;
        let new_at = self.open[..at - 1]
            .iter()
            .rposition(|&n| self.node(n).total_cost_raw <= value)
            .map_or(0, |i| i + 1);
        self.open.insert(new_at, id);
    }
    pub fn link_child<C: PathCallbacks>(
        &mut self,
        c: &mut C,
        parent: NodeId,
        child: NodeId,
    ) -> Result<(), C::Error> {
        let cost = c.cost(self, parent, child)?;
        let known = self.node(parent).known_cost_raw.saturating_add(cost);
        c.add(self, Some(parent), child, 5)?;
        let status = self.node(child).status;
        if status == 0 || status == 1 {
            self.node_mut(parent).children.push(child);
            if known < self.node(child).known_cost_raw {
                let heuristic = self.node(child).heuristic.wrapping_shl(8);
                let node = self.node_mut(child);
                node.parent = Some(parent);
                node.known_cost_raw = known;
                node.total_cost_raw = known.saturating_add(heuristic);
                if status == 0 {
                    self.update_open(child);
                }
                c.add(self, Some(parent), child, if status == 0 { 1 } else { 2 })?;
                if status == 1 {
                    self.update_parents(c, child)?;
                }
            }
        } else {
            self.node_mut(child).parent = Some(parent);
            self.node_mut(child).known_cost_raw = known;
            let at = self.node(child).location();
            let edge = known.wrapping_sub(self.node(parent).known_cost_raw);
            let heuristic = c.heuristic(self, at, edge)?;
            self.node_mut(child).heuristic = heuristic;
            self.node_mut(child).total_cost_raw = known.saturating_add(heuristic.wrapping_shl(8));
            c.add(self, Some(parent), child, 4)?;
            self.add_to_open(c, child)?;
            self.node_mut(parent).children.push(child);
        }
        Ok(())
    }
    fn update_parents<C: PathCallbacks>(
        &mut self,
        c: &mut C,
        first: NodeId,
    ) -> Result<(), C::Error> {
        let mut current = Some(first);
        while let Some(parent) = current {
            let count = self.node(parent).children.len();
            for index in 0..count {
                // Native PackedCoord resolves every retained edge to layer zero.
                let at = self.node(self.node(parent).children[index]).location();
                let child = self
                    .normal_node(at.x, at.y)
                    .expect("retained child coordinate");
                let edge = c.cost(self, parent, child)?;
                let known = self.node(parent).known_cost_raw.saturating_add(edge);
                if known < self.node(child).known_cost_raw
                    && self.node(parent).parent != Some(child)
                {
                    let heuristic = self.node(child).heuristic.wrapping_shl(8);
                    let node = self.node_mut(child);
                    node.known_cost_raw = known;
                    node.total_cost_raw = known.saturating_add(heuristic);
                    node.parent = Some(parent);
                    if node.status == 0 {
                        self.update_open(child);
                    }
                    c.add(self, Some(parent), child, 3)?;
                    if !self.pending.contains(&child) {
                        self.pending.push(child);
                    }
                }
            }
            current = self.pending.pop();
        }
        Ok(())
    }
    fn range(value: i32, length: i32, wrap: bool) -> i32 {
        if !wrap {
            value
        } else if value < 0 {
            value % length + length
        } else if value >= length {
            value % length
        } else {
            value
        }
    }
    fn half(value: i32) -> i32 {
        if value >= 0 {
            value >> 1
        } else {
            let a = value.wrapping_sub(1);
            a.wrapping_sub(a >> 31) >> 1
        }
    }
    fn create_children<C: PathCallbacks>(
        &mut self,
        c: &mut C,
        parent: NodeId,
    ) -> Result<(), C::Error> {
        let at = self.node(parent).location();
        let axial_x = at.x.wrapping_sub(Self::half(at.y));
        // Static target arrays at 0x2f566c0/0x2f566e0, SHA-gated.
        for (dx, dy) in [(0, 1), (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1)] {
            let y = Self::range(at.y.wrapping_add(dy), self.height, self.wrap_y);
            let x = Self::range(
                axial_x.wrapping_add(dx).wrapping_add(Self::half(y)),
                self.width,
                self.wrap_x,
            );
            if let Some(child) = self.normal_node(x, y) {
                if c.valid(self, Some(parent), child)? > 0 {
                    self.link_child(c, parent, child)?;
                }
            }
            if self.aborted {
                break;
            }
        }
        if !self.aborted {
            let count = c.extra_children_count(self, parent)?;
            for index in 0..count {
                let at = c.extra_child(self, parent, index)?;
                if let Some(child) = self.normal_node(at.x, at.y) {
                    if c.valid(self, Some(parent), child)? > 0 {
                        self.link_child(c, parent, child)?;
                    }
                }
                if self.aborted {
                    break;
                }
            }
        }
        Ok(())
    }
    /// Instance::GeneratePath. A selected destination is expanded before its
    /// destination callback. Invalid destinations still execute initialization,
    /// root callbacks and uninitialization in the native order.
    pub fn generate<C: PathCallbacks>(
        &mut self,
        c: &mut C,
        source: PlotCoord,
        destination: PlotCoord,
        flags: u32,
        reuse: bool,
    ) -> Result<bool, C::Error> {
        let reuse = reuse && !self.force_reset && self.source == source && self.flags == flags;
        self.source = source;
        self.destination = destination;
        self.flags = flags;
        c.initialize(self)?;
        let result = self.generate_inner(c, reuse);
        // Provider failures are unsupported execution, not native false results.
        match result {
            Ok(found) => {
                c.uninitialize(self)?;
                Ok(found)
            }
            Err(error) => Err(error),
        }
    }
    fn generate_inner<C: PathCallbacks>(
        &mut self,
        c: &mut C,
        reuse: bool,
    ) -> Result<bool, C::Error> {
        let Some(root) = self.normal_node(self.source.x, self.source.y) else {
            return Ok(false);
        };
        if !reuse {
            while let Some(id) = self.open.pop() {
                self.node_mut(id).clear();
            }
            while let Some(id) = self.closed.pop() {
                self.node_mut(id).clear();
            }
            let count = self.width as usize * self.height as usize;
            for node in &mut self.nodes[..count] {
                node.cache_flags &= !1;
            }
            self.last_node = None;
            self.pending.clear();
            self.force_reset = false;
            self.aborted = false;
            self.node_mut(root).clear();
            let heuristic = c.heuristic(self, self.source, -256)?;
            self.node_mut(root).heuristic = heuristic;
            self.node_mut(root).total_cost_raw = heuristic.wrapping_shl(8);
            self.open.push(root);
            // Native root remains unlinked (-1) until GetBest closes it.
            c.node_add(self, None, root, 1)?;
            c.valid(self, None, root)?;
            c.add(self, None, root, 0)?;
        }
        if !c.destination_valid(self)? {
            return Ok(false);
        }
        if let Some(destination) = self.normal_node(self.destination.x, self.destination.y) {
            if self.node(destination).status == 1 {
                self.last_node = Some(destination);
                return Ok(true);
            }
        }
        loop {
            if self.open.is_empty() {
                self.last_node = None;
                return Ok(false);
            }
            let best = self.open.remove(0);
            c.node_add(self, None, best, 2)?;
            self.node_mut(best).status = 1;
            self.closed.insert(0, best);
            c.node_add(self, None, best, 3)?;
            self.last_node = Some(best);
            self.create_children(c, best)?;
            if self.aborted {
                self.last_node = None;
                return Ok(false);
            }
            if let Some(last) = self.last_node {
                let at = self.node(last).location();
                if c.is_destination(self, at)? {
                    return Ok(true);
                }
            } else {
                return Ok(false);
            }
        }
    }
    pub fn copy_path(&self) -> Vec<PathNode> {
        let mut path = Vec::new();
        let mut current = self.last_node;
        while let Some(id) = current {
            let n = self.node(id);
            path.push(PathNode {
                data1_raw: n.moves_remaining_raw,
                data2: n.turn,
                flags: 0,
                x: n.x,
                y: n.y,
            });
            current = n.parent;
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[derive(Default)]
    struct Callbacks {
        trace: Vec<(char, Option<NodeId>, NodeId, i32)>,
        expanded: Vec<PlotCoord>,
        destination_allowed: bool,
    }
    impl PathCallbacks for Callbacks {
        type Error = Infallible;
        fn initialize(&mut self, _: &mut TwoLayer) -> Result<(), Self::Error> {
            Ok(())
        }
        fn uninitialize(&mut self, _: &mut TwoLayer) -> Result<(), Self::Error> {
            Ok(())
        }
        fn destination_valid(&mut self, _: &mut TwoLayer) -> Result<bool, Self::Error> {
            Ok(self.destination_allowed)
        }
        fn is_destination(&mut self, s: &mut TwoLayer, p: PlotCoord) -> Result<bool, Self::Error> {
            self.expanded.push(p);
            Ok(p == s.destination)
        }
        fn heuristic(
            &mut self,
            _: &mut TwoLayer,
            _: PlotCoord,
            _: i32,
        ) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn valid(
            &mut self,
            _: &mut TwoLayer,
            p: Option<NodeId>,
            n: NodeId,
        ) -> Result<i32, Self::Error> {
            self.trace.push(('v', p, n, 0));
            Ok(1)
        }
        fn cost(&mut self, _: &mut TwoLayer, p: NodeId, n: NodeId) -> Result<i32, Self::Error> {
            self.trace.push(('c', Some(p), n, 0));
            Ok(256)
        }
        fn add(
            &mut self,
            _: &mut TwoLayer,
            p: Option<NodeId>,
            n: NodeId,
            e: i32,
        ) -> Result<(), Self::Error> {
            self.trace.push(('a', p, n, e));
            Ok(())
        }
        fn node_add(
            &mut self,
            _: &mut TwoLayer,
            p: Option<NodeId>,
            n: NodeId,
            e: i32,
        ) -> Result<(), Self::Error> {
            self.trace.push(('n', p, n, e));
            Ok(())
        }
        fn extra_children_count(
            &mut self,
            _: &mut TwoLayer,
            _: NodeId,
        ) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn extra_child(
            &mut self,
            _: &mut TwoLayer,
            _: NodeId,
            _: i32,
        ) -> Result<PlotCoord, Self::Error> {
            unreachable!()
        }
    }
    fn at(x: i32, y: i32) -> PlotCoord {
        PlotCoord { x, y }
    }

    #[test]
    fn destination_is_expanded_and_root_callbacks_precede_destination_validation() {
        let mut search = TwoLayer::new(4, 4, false, false).unwrap();
        let mut c = Callbacks {
            destination_allowed: true,
            ..Default::default()
        };
        let root = search.normal_node(1, 1).unwrap();
        assert!(search
            .generate(&mut c, at(1, 1), at(1, 1), 0, false)
            .unwrap());
        assert_eq!(
            &c.trace[..3],
            &[
                ('n', None, root, 1),
                ('v', None, root, 0),
                ('a', None, root, 0)
            ]
        );
        assert_eq!(c.expanded, vec![at(1, 1)]);
        let children: Vec<_> = c
            .trace
            .iter()
            .filter(|(kind, parent, _, _)| *kind == 'v' && parent.is_some())
            .map(|(_, _, child, _)| search.node(*child).location())
            .collect();
        assert_eq!(
            children,
            vec![at(2, 2), at(2, 1), at(2, 0), at(1, 0), at(0, 1), at(1, 2)]
        );
        assert_eq!(search.node(root).children.len(), 6);
        c.destination_allowed = false;
        c.trace.clear();
        assert!(!search
            .generate(&mut c, at(1, 1), at(3, 3), 0, false)
            .unwrap());
        assert_eq!(c.trace.len(), 3);
    }

    #[test]
    fn equal_open_priorities_insert_from_the_nearer_end() {
        let mut s = TwoLayer::new(8, 1, false, false).unwrap();
        let mut c = Callbacks::default();
        let ids: Vec<_> = (0..8).map(|x| s.normal_node(x, 0).unwrap()).collect();
        for (i, cost) in [0, 100, 80, 80, 20, 20].into_iter().enumerate() {
            s.node_mut(ids[i]).total_cost_raw = cost;
            s.add_to_open(&mut c, ids[i]).unwrap();
        }
        // First equal 80 is nearer the tail: insert after existing 80. Equal
        // 20 is nearer the head: insert before existing 20.
        assert_eq!(s.open, vec![ids[0], ids[5], ids[4], ids[2], ids[3], ids[1]]);
        s.node_mut(ids[3]).total_cost_raw = 20;
        s.update_open(ids[3]);
        // Decrease-key compares only total and follows existing equal totals.
        assert_eq!(s.open, vec![ids[0], ids[5], ids[4], ids[3], ids[2], ids[1]]);
    }

    #[test]
    fn closed_cache_reuse_skips_search_but_still_validates_destination() {
        let mut s = TwoLayer::new(3, 3, false, false).unwrap();
        let mut c = Callbacks {
            destination_allowed: true,
            ..Default::default()
        };
        assert!(s.generate(&mut c, at(1, 1), at(1, 1), 7, false).unwrap());
        c.trace.clear();
        c.expanded.clear();
        assert!(s.generate(&mut c, at(1, 1), at(1, 1), 7, true).unwrap());
        assert!(c.trace.is_empty());
        assert!(c.expanded.is_empty());
        c.destination_allowed = false;
        assert!(!s.generate(&mut c, at(1, 1), at(1, 1), 7, true).unwrap());
    }

    #[test]
    fn failed_search_retains_no_last_node_and_copies_destination_first() {
        let mut s = TwoLayer::new(2, 1, false, false).unwrap();
        let mut c = Callbacks {
            destination_allowed: true,
            ..Default::default()
        };
        assert!(s.generate(&mut c, at(0, 0), at(1, 0), 0, false).unwrap());
        assert_eq!(
            s.copy_path().iter().map(|n| n.x).collect::<Vec<_>>(),
            vec![1, 0]
        );
        assert!(!s.generate(&mut c, at(0, 0), at(9, 0), 0, false).unwrap());
        assert!(s.copy_path().is_empty());
    }
}
