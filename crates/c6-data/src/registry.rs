//! Exact Linux XP2 byte-hash and definition collection lookup contracts.
//! This module accepts an explicit native collection order; it does not infer
//! that order from SQLite rows or establish an initialized registry/profile.
use crate::RulesRowId;

/// Native `Utilities::MakeHash(char const*)`: reflected CRC32, seed all ones,
/// no final xor, case sensitive, stopping at the first NUL byte.
pub fn linux_type_hash(name: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in name.iter().take_while(|&&byte| byte != 0) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}

/// Separate native identifiers. The collection index is its position in the
/// supplied vector; `database_rowid` remains a database identity, not that index.
/// Names model non-null C strings; native SetYieldType(null) is a separate
/// contract and cannot be represented by an empty byte name here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionEntry {
    pub database_rowid: RulesRowId,
    pub type_name: Vec<u8>,
}

/// Native two-vector collection lookup, without native object/property storage.
/// Caller-supplied entries must already be in observed native collection order.
/// Construction verifies no source identity and proves no native runtime IDs.
#[derive(Clone, Debug)]
pub struct LinuxDefinitionRegistry {
    entries: Vec<DefinitionEntry>,
    by_hash: Vec<(u32, usize)>,
}

fn native_hash_index(hashes: impl IntoIterator<Item = u32>) -> Vec<(u32, usize)> {
    let mut by_hash: Vec<_> = hashes
        .into_iter()
        .enumerate()
        .map(|(i, h)| (h, i))
        .collect();
    by_hash.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    by_hash
}
fn find_native_index(count: usize, by_hash: &[(u32, usize)], key: u32) -> Option<usize> {
    if (key as usize) < count {
        return Some(key as usize);
    }
    let at = by_hash.partition_point(|&(hash, _)| hash < key);
    by_hash
        .get(at)
        .filter(|&&(hash, _)| hash == key)
        .map(|&(_, i)| i)
}

/// Original observed hashes in native collection order, without invented names
/// or database row identities. This accepts the hash-only movement probe export.
#[derive(Clone, Debug)]
pub struct LinuxDefinitionHashRegistry {
    hashes: Vec<u32>,
    by_hash: Vec<(u32, usize)>,
}
impl LinuxDefinitionHashRegistry {
    pub fn from_native_order(hashes: Vec<u32>) -> Self {
        assert!(
            u32::try_from(hashes.len()).is_ok(),
            "definition count exceeds native u32 domain"
        );
        let by_hash = native_hash_index(hashes.iter().copied());
        Self { hashes, by_hash }
    }
    pub fn hashes(&self) -> &[u32] {
        &self.hashes
    }
    pub fn find_index(&self, index_or_hash: u32) -> Option<usize> {
        find_native_index(self.hashes.len(), &self.by_hash, index_or_hash)
    }
}

impl LinuxDefinitionRegistry {
    /// Panics if the collection count cannot fit the native unsigned 32-bit domain.
    pub fn from_native_order(entries: Vec<DefinitionEntry>) -> Self {
        assert!(
            u32::try_from(entries.len()).is_ok(),
            "definition count exceeds the native u32 domain"
        );
        // The native insertion uses lower_bound. Equal hashes therefore resolve
        // to the most recently inserted definition, without string comparison.
        let by_hash = native_hash_index(
            entries
                .iter()
                .map(|entry| linux_type_hash(&entry.type_name)),
        );
        Self { entries, by_hash }
    }

    pub fn entries(&self) -> &[DefinitionEntry] {
        &self.entries
    }

    pub fn at(&self, index: u32) -> Option<&DefinitionEntry> {
        self.entries.get(index as usize)
    }

    /// Native Find/FindIndex semantics: arguments below count are direct indexes;
    /// all remaining arguments are unsigned hashes. A miss returns None (native
    /// Find returns null and native FindIndex returns signed -1).
    pub fn find_index(&self, index_or_hash: u32) -> Option<usize> {
        find_native_index(self.entries.len(), &self.by_hash, index_or_hash)
    }

    pub fn find(&self, index_or_hash: u32) -> Option<&DefinitionEntry> {
        self.find_index(index_or_hash)
            .map(|index| &self.entries[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_byte_case_nul_and_empty_boundaries() {
        assert_eq!(linux_type_hash(b""), u32::MAX);
        assert_eq!(linux_type_hash(b"123456789"), 0x340b_c6d9);
        assert_eq!(linux_type_hash(b"UNIT_MOVED"), 0xbe12_1408);
        assert_eq!(linux_type_hash(b"UNIT_MOVED\0ignored"), 0xbe12_1408);
        assert_ne!(
            linux_type_hash(b"yield_food"),
            linux_type_hash(b"YIELD_FOOD")
        );
        assert_eq!(linux_type_hash(&[0xff]), 0x00ff_ffff);
    }

    #[test]
    fn native_index_hash_miss_and_duplicate_precedence() {
        let entries = vec![
            DefinitionEntry {
                database_rowid: RulesRowId(19),
                type_name: b"YIELD_FOOD".to_vec(),
            },
            DefinitionEntry {
                database_rowid: RulesRowId(-2),
                type_name: b"YIELD_GOLD".to_vec(),
            },
            DefinitionEntry {
                database_rowid: RulesRowId(87),
                type_name: b"YIELD_FOOD".to_vec(),
            },
        ];
        let registry = LinuxDefinitionRegistry::from_native_order(entries);
        assert_eq!(registry.find_index(0), Some(0));
        assert_eq!(registry.find_index(2), Some(2));
        assert_eq!(registry.find_index(linux_type_hash(b"YIELD_FOOD")), Some(2));
        assert_eq!(registry.find_index(linux_type_hash(b"YIELD_GOLD")), Some(1));
        assert_eq!(registry.find_index(3), None);
        assert_eq!(registry.find_index(u32::MAX), None);
        assert_eq!(registry.at(3), None);
        assert_eq!(registry.find(1).unwrap().database_rowid, RulesRowId(-2));
        assert_eq!(registry.entries()[0].database_rowid, RulesRowId(19));
    }
}
