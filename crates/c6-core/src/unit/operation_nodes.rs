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

/// Native retained pointer semantics expressed without a physical address.
/// The slot remains fixed when payloads shift; it is not an occurrence ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationStorageHandle {
    owner: u64,
    allocation: u64,
    slot: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationStorageError {
    InvalidCapacity,
    UnknownCapacityGrowth,
    QueueReplaced,
    /// A second growth could reuse an older freed allocation's address. That
    /// allocator behavior is not inferred from abstract allocation generations.
    OlderAllocationReuse,
}
impl fmt::Display for OperationStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for OperationStorageError {}

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
pub struct OperationNodes {
    entries: Vec<OperationNode>,
    ids: Vec<OperationNodeId>,
    storage_owner: u64,
    allocation: Option<u64>,
    capacity: Option<u32>,
    minimum_capacity: u32,
}
impl Default for OperationNodes {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            ids: Vec::new(),
            storage_owner: fresh_id().0,
            allocation: Some(0),
            capacity: Some(0),
            minimum_capacity: 0,
        }
    }
}
impl OperationNodes {
    /// Install an actual initial native capacity before any retained callback
    /// can grow the queue. Wire count alone does not establish this field.
    pub fn bind_native_capacity(&mut self, capacity: u32) -> Result<(), OperationStorageError> {
        if capacity < self.minimum_capacity
            || (capacity != 0 && capacity < 4)
            || self.capacity.is_some_and(|old| old != capacity)
        {
            return Err(OperationStorageError::InvalidCapacity);
        }
        if self.allocation.is_none() {
            return Err(OperationStorageError::UnknownCapacityGrowth);
        }
        self.capacity = Some(capacity);
        self.minimum_capacity = capacity;
        Ok(())
    }
    pub fn native_capacity(&self) -> Option<u32> {
        self.capacity
    }
    pub fn front_storage(&self) -> Result<Option<OperationStorageHandle>, OperationStorageError> {
        if self.entries.is_empty() {
            return Ok(None);
        }
        Ok(Some(OperationStorageHandle {
            owner: self.storage_owner,
            allocation: self
                .allocation
                .ok_or(OperationStorageError::UnknownCapacityGrowth)?,
            slot: 0,
        }))
    }
    fn storage_index(
        &self,
        handle: OperationStorageHandle,
    ) -> Result<Option<usize>, OperationStorageError> {
        if handle.owner != self.storage_owner {
            return Err(OperationStorageError::QueueReplaced);
        }
        let allocation = self
            .allocation
            .ok_or(OperationStorageError::UnknownCapacityGrowth)?;
        if allocation == handle.allocation {
            return Ok((handle.slot < self.entries.len()).then_some(handle.slot));
        }
        if handle.allocation.checked_add(1) == Some(allocation) {
            // New buffer is allocated before the retained old buffer is freed.
            return Ok(None);
        }
        Err(OperationStorageError::OlderAllocationReuse)
    }
    pub fn storage_is_front(
        &self,
        handle: OperationStorageHandle,
    ) -> Result<bool, OperationStorageError> {
        Ok(self.storage_index(handle)? == Some(0))
    }
    /// Native Queue::Remove scans current element addresses, not payload IDs.
    /// A shifted/replaced payload at the retained slot is the one removed.
    pub fn remove_storage(
        &mut self,
        handle: OperationStorageHandle,
    ) -> Result<Option<OperationNode>, OperationStorageError> {
        Ok(self.storage_index(handle)?.map(|index| self.remove(index)))
    }
    fn grow_for(&mut self, count: usize) {
        let count = u32::try_from(count).expect("operation queue exceeds native u32 extent");
        if let Some(capacity) = self.capacity {
            if count <= capacity {
                return;
            }
            // AllocLinearBlock(count,false), original wrapping u32 arithmetic.
            let next = if count <= 4 {
                4
            } else {
                count.max(capacity.wrapping_mul(2))
            };
            self.capacity = Some(next);
            self.minimum_capacity = next;
            self.allocation = self.allocation.and_then(|epoch| epoch.checked_add(1));
        } else if self.minimum_capacity == 0 && count <= 4 {
            // An initially empty imported queue has no retained element. Its
            // first current allocation can be named without inferring capacity.
            self.minimum_capacity = 4;
        } else if count > self.minimum_capacity {
            // Unknown imported capacity might already fit or require growth.
            // Preserve payload execution but never invent retained-pointer parity.
            self.allocation = None;
            self.minimum_capacity = count;
        }
    }
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
        self.grow_for(self.entries.len() + 1);
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
        let minimum_capacity = if entries.is_empty() {
            0
        } else {
            u32::try_from(entries.len())
                .expect("operation queue exceeds native u32 extent")
                .max(4)
        };
        Self {
            entries,
            ids,
            storage_owner: fresh_id().0,
            allocation: Some(0),
            capacity: None,
            minimum_capacity,
        }
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
    fn occurrence_metadata_survives_insertion_without_claiming_native_pointer_identity() {
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
    #[test]
    fn native_retained_slot_removes_current_shifted_payload_instead_of_old_occurrence() {
        let mut queue = OperationNodes::default();
        queue.push(node());
        queue.push(node());
        let retained = queue.front_storage().unwrap().unwrap();
        let original = queue.front_id().unwrap();
        // PushFront within capacity4 shifts the original payload, while the
        // native retained pointer still addresses slot0 (now the inserted node).
        queue.insert(0, node());
        let inserted = queue.front_id().unwrap();
        assert_ne!(inserted, original);
        assert_eq!(queue.remove_storage(retained), Ok(Some(node())));
        assert_eq!(queue.front_id(), Some(original));
        assert!(queue.get_by_id(inserted).is_none());
    }
    #[test]
    fn clear_and_repopulate_preserve_native_slot_alias_even_for_identical_payload() {
        let mut queue = OperationNodes::default();
        queue.push(node());
        let retained = queue.front_storage().unwrap().unwrap();
        let original = queue.front_id();
        queue.clear();
        queue.push(node());
        assert_ne!(queue.front_id(), original);
        assert_eq!(queue.native_capacity(), Some(4));
        assert_eq!(queue.remove_storage(retained), Ok(Some(node())));
        assert!(queue.is_empty());
    }
    #[test]
    fn first_growth_cannot_reuse_retained_live_allocation_and_clear_keeps_capacity() {
        let mut queue = OperationNodes::default();
        for _ in 0..4 {
            queue.push(node());
        }
        let old = queue.front_storage().unwrap().unwrap();
        queue.push(node());
        assert_eq!(queue.native_capacity(), Some(8));
        assert_eq!(queue.remove_storage(old), Ok(None));
        assert_eq!(queue.len(), 5);
        let current = queue.front_storage().unwrap().unwrap();
        queue.clear();
        queue.push(node());
        assert_eq!(queue.native_capacity(), Some(8));
        assert_eq!(queue.remove_storage(current), Ok(Some(node())));
    }
    #[test]
    fn unknown_imported_capacity_is_required_only_for_ambiguous_growth() {
        let mut queue: OperationNodes = vec![node(); 4].into();
        let retained = queue.front_storage().unwrap().unwrap();
        queue.push(node());
        assert_eq!(
            queue.remove_storage(retained),
            Err(OperationStorageError::UnknownCapacityGrowth)
        );
        assert_eq!(queue.len(), 5);
        let mut admitted: OperationNodes = vec![node(); 4].into();
        admitted.bind_native_capacity(8).unwrap();
        let retained = admitted.front_storage().unwrap().unwrap();
        admitted.push(node());
        assert_eq!(admitted.remove_storage(retained), Ok(Some(node())));
    }
    #[test]
    fn multiple_growths_do_not_invent_old_freed_address_reuse_behavior() {
        let mut queue = OperationNodes::default();
        for _ in 0..4 {
            queue.push(node());
        }
        let retained = queue.front_storage().unwrap().unwrap();
        for _ in 0..5 {
            queue.push(node());
        }
        assert_eq!(queue.native_capacity(), Some(16));
        assert_eq!(
            queue.remove_storage(retained),
            Err(OperationStorageError::OlderAllocationReuse)
        );
        assert_eq!(queue.len(), 9);
    }
}
