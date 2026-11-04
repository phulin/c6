//! Linux XP2 ObjectPairing owner and GetChildren projection.
//! Import requires an attributed complete owner frame; absent state is not empty.
use crate::unit::{ComponentId, Limits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux ObjectPairing: {self:?}")
    }
}
impl std::error::Error for Error {}

pub const NULL_COMPONENT: ComponentId = ComponentId {
    owner: -1,
    component_type: 0,
    id: u32::MAX,
};
fn bits(id: ComponentId) -> u64 {
    u64::from(id.owner as u16)
        | (u64::from(id.component_type as u16) << 16)
        | (u64::from(id.id) << 32)
}

/// Both independent native maps, after ordered Read assignments. This does not
/// infer reverse relationships or silently repair inconsistent saved maps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manager {
    pub wire_version: i32,
    children: Vec<(ComponentId, Vec<ComponentId>)>,
    parents: Vec<(ComponentId, ComponentId)>,
}
pub struct DecodedPrefix<'a> {
    pub state: Manager,
    pub consumed: usize,
    pub items_used: usize,
    pub tail: &'a [u8],
}
impl Manager {
    /// Complete current native map snapshots. Keep both independently captured
    /// maps; never synthesize the reverse map from child relationships.
    pub fn from_current_maps(
        children: Vec<(ComponentId, Vec<ComponentId>)>,
        parents: Vec<(ComponentId, ComponentId)>,
        limits: Limits,
    ) -> Result<Self, Error> {
        let items = children
            .len()
            .checked_add(parents.len())
            .and_then(|n| children.iter().try_fold(n, |n, r| n.checked_add(r.1.len())))
            .ok_or(Error::ResourceLimit)?;
        if items > limits.max_items || items.checked_mul(16).is_none_or(|n| n > limits.max_bytes) {
            return Err(Error::ResourceLimit);
        }
        let mut canonical_children = Vec::new();
        for (key, value) in children {
            assign(&mut canonical_children, key, value)?;
        }
        let mut canonical_parents = Vec::new();
        for (key, value) in parents {
            assign(&mut canonical_parents, key, value)?;
        }
        canonical_children.sort_by_key(|r| bits(r.0));
        canonical_parents.sort_by_key(|r| bits(r.0));
        Ok(Self {
            wire_version: 1,
            children: canonical_children,
            parents: canonical_parents,
        })
    }
    /// Native2696bb0 returns the shared null component for null/absent keys.
    pub fn get_parent(&self, child: ComponentId) -> ComponentId {
        if child == NULL_COMPONENT {
            return NULL_COMPONENT;
        }
        self.parents
            .binary_search_by_key(&bits(child), |r| bits(r.0))
            .ok()
            .map_or(NULL_COMPONENT, |at| self.parents[at].1)
    }
    /// Source26962f0 ignores the version and skips negative signed map counts.
    /// Child vectors use unsigned counts; duplicate map keys overwrite in order.
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<DecodedPrefix<'_>, Error> {
        let mut r = Reader {
            bytes,
            offset: 0,
            items: 0,
            limits,
        };
        let wire_version = r.word()? as i32;
        let mut children = Vec::new();
        for _ in 0..(r.word()? as i32).max(0) {
            r.items(1)?;
            let parent = r.id()?;
            let count = r.word()? as usize;
            r.items(count)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| Error::ResourceLimit)?;
            for _ in 0..count {
                values.push(r.id()?);
            }
            assign(&mut children, parent, values)?;
        }
        let mut parents = Vec::new();
        for _ in 0..(r.word()? as i32).max(0) {
            r.items(1)?;
            let child = r.id()?;
            let parent = r.id()?;
            assign(&mut parents, child, parent)?;
        }
        children.sort_by_key(|row| bits(row.0));
        parents.sort_by_key(|row| bits(row.0));
        Ok(DecodedPrefix {
            state: Self {
                wire_version,
                children,
                parents,
            },
            consumed: r.offset,
            items_used: r.items,
            tail: &bytes[r.offset..],
        })
    }
    /// Native2696c70 returns the shared empty vector for null or absent keys.
    /// All children (including other types, nulls and duplicates) retain order.
    pub fn get_children(&self, parent: ComponentId) -> &[ComponentId] {
        if parent == NULL_COMPONENT {
            return &[];
        }
        self.children
            .binary_search_by_key(&bits(parent), |row| bits(row.0))
            .ok()
            .map_or(&[], |index| self.children[index].1.as_slice())
    }
    pub fn child_entries(&self) -> &[(ComponentId, Vec<ComponentId>)] {
        &self.children
    }
    pub fn parent_entries(&self) -> &[(ComponentId, ComponentId)] {
        &self.parents
    }
}
fn assign<T>(rows: &mut Vec<(ComponentId, T)>, key: ComponentId, value: T) -> Result<(), Error> {
    if let Some(row) = rows.iter_mut().find(|row| row.0 == key) {
        row.1 = value;
    } else {
        rows.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
        rows.push((key, value));
    }
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn read<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.offset.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let value = self.bytes.get(self.offset..end).ok_or(Error::Truncated)?;
        self.offset = end;
        Ok(value.try_into().unwrap())
    }
    fn word(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.read()?))
    }
    fn id(&mut self) -> Result<ComponentId, Error> {
        Ok(ComponentId {
            owner: i16::from_le_bytes(self.read()?),
            component_type: i16::from_le_bytes(self.read()?),
            id: self.word()?,
        })
    }
    fn items(&mut self, count: usize) -> Result<(), Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(owner: i16, component_type: i16, id: u32) -> ComponentId {
        ComponentId {
            owner,
            component_type,
            id,
        }
    }
    fn put_id(b: &mut Vec<u8>, x: ComponentId) {
        b.extend(x.owner.to_le_bytes());
        b.extend(x.component_type.to_le_bytes());
        b.extend(x.id.to_le_bytes());
    }
    #[test]
    fn duplicate_keys_replace_vectors_without_reordering_children_or_repairing_reverse_map() {
        let parent = id(0, 1, 12);
        let a = id(2, 1, 9);
        let b = id(0, 2, 4);
        let mut bytes = Vec::new();
        bytes.extend(73i32.to_le_bytes());
        bytes.extend(3i32.to_le_bytes());
        put_id(&mut bytes, parent);
        bytes.extend(1u32.to_le_bytes());
        put_id(&mut bytes, a);
        put_id(&mut bytes, NULL_COMPONENT);
        bytes.extend(1u32.to_le_bytes());
        put_id(&mut bytes, a);
        put_id(&mut bytes, parent);
        bytes.extend(3u32.to_le_bytes());
        for x in [b, a, b] {
            put_id(&mut bytes, x);
        }
        bytes.extend(0i32.to_le_bytes());
        bytes.extend([0xaa, 0xbb]);
        let decoded = Manager::decode_prefix(&bytes, Limits::default()).unwrap();
        assert_eq!(decoded.state.get_children(parent), &[b, a, b]);
        assert!(decoded.state.get_children(NULL_COMPONENT).is_empty());
        assert!(decoded.state.get_children(id(0, 1, 99)).is_empty());
        assert!(decoded.state.parent_entries().is_empty());
        assert_eq!(decoded.state.wire_version, 73);
        assert_eq!(decoded.tail, &[0xaa, 0xbb]);
        for end in 0..decoded.consumed {
            assert!(Manager::decode_prefix(&bytes[..end], Limits::default()).is_err());
        }
    }
    #[test]
    fn current_independent_maps_preserve_parent_lookup_and_null_early_return() {
        let child = id(2, 1, 7);
        let parent = id(0, 1, 8);
        let state = Manager::from_current_maps(
            vec![],
            vec![(child, parent), (NULL_COMPONENT, parent)],
            Limits::default(),
        )
        .unwrap();
        assert_eq!(state.get_parent(child), parent);
        assert_eq!(state.get_parent(id(2, 1, 99)), NULL_COMPONENT);
        assert_eq!(state.get_parent(NULL_COMPONENT), NULL_COMPONENT);
        assert!(state.get_children(parent).is_empty());
        assert!(matches!(
            Manager::from_current_maps(
                vec![],
                vec![(child, parent)],
                Limits {
                    max_items: 0,
                    max_bytes: 100
                }
            ),
            Err(Error::ResourceLimit)
        ));
    }
    #[test]
    fn signed_map_counts_skip_but_unsigned_child_counts_are_bounded() {
        let mut bytes = Vec::new();
        for x in [1i32, -1, i32::MIN] {
            bytes.extend(x.to_le_bytes());
        }
        assert!(Manager::decode_prefix(&bytes, Limits::default())
            .unwrap()
            .state
            .child_entries()
            .is_empty());
        let mut bytes = Vec::new();
        bytes.extend(1i32.to_le_bytes());
        bytes.extend(1i32.to_le_bytes());
        put_id(&mut bytes, id(0, 1, 12));
        bytes.extend(u32::MAX.to_le_bytes());
        assert!(matches!(
            Manager::decode_prefix(&bytes, Limits::default()),
            Err(Error::ResourceLimit)
        ));
    }
}
