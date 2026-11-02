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

impl LinuxDefinitionRegistry {
    pub fn from_native_order(entries: Vec<DefinitionEntry>) -> Self {
        // The native insertion uses lower_bound. Equal hashes therefore resolve
        // to the most recently inserted definition, without string comparison.
        let mut by_hash: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (linux_type_hash(&entry.type_name), index))
            .collect();
        by_hash.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
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
        if (index_or_hash as usize) < self.entries.len() {
            return Some(index_or_hash as usize);
        }
        let at = self
            .by_hash
            .partition_point(|&(hash, _)| hash < index_or_hash);
        self.by_hash
            .get(at)
            .filter(|&&(hash, _)| hash == index_or_hash)
            .map(|&(_, index)| index)
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

    #[test]
    #[ignore = "requires audited original Linux definition query report"]
    fn linux_native_definition_report() {
        let path = std::env::var_os("C6_LINUX_DEFINITION_IDS_REPORT")
            .expect("C6_LINUX_DEFINITION_IDS_REPORT must name an audited native report");
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(report["schema"], "c6-linux-definition-ids-v1");
        assert_eq!(
            report["gamecore_sha256"],
            "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa"
        );
        let query = &report["native_query"];
        assert_eq!(query["canonical_core_unchanged"], true);
        let mapping = &query["mapping"];
        let observed = mapping["entries"].as_array().expect("native entries");
        let entries = observed
            .iter()
            .map(|entry| {
                let hex = entry["type_hex"].as_str().unwrap().as_bytes();
                assert_eq!(hex.len() % 2, 0);
                let bytes = hex
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                DefinitionEntry {
                    database_rowid: RulesRowId(entry["database_rowid"].as_i64().unwrap()),
                    type_name: bytes,
                }
            })
            .collect();
        let registry = LinuxDefinitionRegistry::from_native_order(entries);
        assert_eq!(mapping["count"].as_u64().unwrap() as usize, observed.len());
        for (i, entry) in observed.iter().enumerate() {
            let hash = u32::try_from(entry["hash"].as_u64().unwrap()).unwrap();
            assert_eq!(entry["index"].as_u64().unwrap() as usize, i);
            assert_eq!(linux_type_hash(&registry.entries()[i].type_name), hash);
            assert_eq!(
                entry["native_string_hash"].as_u64().unwrap(),
                u64::from(hash)
            );
            assert_eq!(registry.find_index(hash), Some(i));
            assert_eq!(entry["find_index"].as_u64().unwrap() as usize, i);
        }
        let missing = u32::try_from(mapping["missing_hash"].as_u64().unwrap()).unwrap();
        assert_eq!(mapping["missing_result"], -1);
        assert_eq!(registry.find_index(missing), None);
        assert_eq!(registry.find_index(0), Some(0));
        assert_eq!(registry.at(u32::try_from(observed.len()).unwrap()), None);
        assert_eq!(
            u64::from(linux_type_hash(b"")),
            mapping["empty_string_hash"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(linux_type_hash(b"yield_food")),
            mapping["lowercase_food_hash"].as_u64().unwrap()
        );
    }
}
