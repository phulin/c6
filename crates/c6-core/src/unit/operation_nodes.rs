//! Logical operation occurrences; payloads retain native queue order and wire grammar.
use super::OperationNode;
use std::{
    fmt,
    ops::{Deref, Index, IndexMut},
    sync::atomic::{AtomicU64, Ordering},
};

/// Internal logical identity, unrelated to native pointers, handler IDs or RNG.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct OperationNodeId(u64);

fn fresh_id() -> OperationNodeId {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    OperationNodeId(
        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .expect("operation occurrence identity space exhausted"),
    )
}

/// One authoritative payload vector with private occurrence bookkeeping.
/// Structural mutation must use these methods; no mutable Vec/slice is exposed.
/// Node-field edits preserve the occurrence. Cloning or replacing this queue
/// creates new occurrences, even when all copied payloads compare equal.
/// IDs do not participate in equality, iteration order, encoding or gameplay RNG.
#[derive(Default)]
pub struct OperationNodes {
    entries: Vec<OperationNode>,
    ids: Vec<OperationNodeId>,
}
impl OperationNodes {
    pub fn as_slice(&self) -> &[OperationNode] {
        &self.entries
    }
    pub fn id_at(&self, index: usize) -> Option<OperationNodeId> {
        self.ids.get(index).copied()
    }
    pub fn front_id(&self) -> Option<OperationNodeId> {
        self.id_at(0)
    }
    pub fn find_id(&self, id: OperationNodeId) -> Option<usize> {
        self.ids.iter().position(|&v| v == id)
    }
    pub fn get_by_id(&self, id: OperationNodeId) -> Option<&OperationNode> {
        self.entries.get(self.find_id(id)?)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut OperationNode> {
        self.entries.get_mut(index)
    }
    pub fn first_mut(&mut self) -> Option<&mut OperationNode> {
        self.entries.first_mut()
    }
    pub fn last_mut(&mut self) -> Option<&mut OperationNode> {
        self.entries.last_mut()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, OperationNode> {
        self.entries.iter_mut()
    }
    pub fn push(&mut self, node: OperationNode) {
        self.insert(self.entries.len(), node);
    }
    pub fn insert(&mut self, index: usize, node: OperationNode) {
        assert!(
            index <= self.entries.len(),
            "operation insertion index out of bounds"
        );
        let id = fresh_id();
        self.entries.insert(index, node);
        self.ids.insert(index, id);
    }
    pub fn remove(&mut self, index: usize) -> OperationNode {
        let node = self.entries.remove(index);
        self.ids.remove(index);
        node
    }
    pub fn remove_by_id(&mut self, id: OperationNodeId) -> Option<OperationNode> {
        Some(self.remove(self.find_id(id)?))
    }
    pub fn pop(&mut self) -> Option<OperationNode> {
        let node = self.entries.pop()?;
        self.ids.pop();
        Some(node)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.ids.clear();
    }
    pub fn truncate(&mut self, len: usize) {
        self.entries.truncate(len);
        self.ids.truncate(len);
    }
    pub fn reverse(&mut self) {
        self.entries.reverse();
        self.ids.reverse();
    }
}
impl From<Vec<OperationNode>> for OperationNodes {
    fn from(entries: Vec<OperationNode>) -> Self {
        let ids = (0..entries.len()).map(|_| fresh_id()).collect();
        Self { entries, ids }
    }
}
impl Clone for OperationNodes {
    fn clone(&self) -> Self {
        self.entries.clone().into()
    }
}
impl fmt::Debug for OperationNodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entries.fmt(f)
    }
}
impl PartialEq for OperationNodes {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
impl Eq for OperationNodes {}
impl Deref for OperationNodes {
    type Target = [OperationNode];
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}
impl Index<usize> for OperationNodes {
    type Output = OperationNode;
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}
impl IndexMut<usize> for OperationNodes {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}
impl<'a> IntoIterator for &'a OperationNodes {
    type Item = &'a OperationNode;
    type IntoIter = std::slice::Iter<'a, OperationNode>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
impl<'a> IntoIterator for &'a mut OperationNodes {
    type Item = &'a mut OperationNode;
    type IntoIter = std::slice::IterMut<'a, OperationNode>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::{Limits, OperationParameters, OperationQueue, SKIP_TURN_HASH};
    fn node() -> OperationNode {
        OperationNode {
            operation_hash: SKIP_TURN_HASH,
            directive: 0,
            flags: 0,
            push_turn: 0,
            previous_activity: 0,
            previous_automation: -1,
            parameters: OperationParameters::None,
        }
    }
    #[test]
    fn retained_duplicate_occurrence_survives_insertion_before_callback() {
        let mut queue: OperationNodes = vec![node(), node()].into();
        let selected = queue.front_id().unwrap();
        let tail = queue.id_at(1).unwrap();
        // A synchronous callback inserts an indistinguishable front occurrence.
        queue.insert(0, node());
        let inserted = queue.front_id().unwrap();
        assert_ne!(selected, inserted);
        assert_eq!(queue.remove_by_id(selected), Some(node()));
        assert_eq!(queue.front_id(), Some(inserted));
        assert_eq!(queue.id_at(1), Some(tail));
    }
    #[test]
    fn removed_occurrence_is_never_replaced_by_equal_payload_or_clone() {
        let mut queue: OperationNodes = vec![node(), node()].into();
        let selected = queue.front_id().unwrap();
        queue.remove(0);
        queue.insert(0, node());
        assert!(queue.remove_by_id(selected).is_none());
        assert_eq!(queue.len(), 2);
        let copy = queue.clone();
        assert_eq!(queue, copy);
        assert_ne!(queue.front_id(), copy.front_id());
    }
    #[test]
    fn node_field_edits_keep_identity_and_ids_never_enter_wire_bytes() {
        let mut queue = OperationQueue {
            nodes: vec![node(), node()].into(),
        };
        let selected = queue.nodes.front_id();
        queue.nodes[0].directive = 7;
        assert_eq!(queue.nodes.front_id(), selected);
        let bytes = queue.encode(Limits::default()).unwrap();
        let decoded = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(queue, decoded);
        assert_ne!(queue.nodes.front_id(), decoded.nodes.front_id());
        assert_eq!(bytes, decoded.encode(Limits::default()).unwrap());
    }
}
