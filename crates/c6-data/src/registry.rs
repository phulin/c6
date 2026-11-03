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
    use sha2::{Digest, Sha256};

    fn check_native_fixture_identity(report: &serde_json::Value) {
        const GAME: &str = "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa";
        const HOST: &str = "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4";
        // This comparator accepts the observed benchmark context only. New
        // native configurations require their own independently audited fixture.
        const CONFIG: &str = "c926ba2347ef6515da4b260f960957dab7389239b2cd59526407c72e74c7a1e4";
        assert_eq!(report["schema"], "c6-linux-definition-ids-v1");
        assert_eq!(report["gamecore_sha256"], GAME);
        assert_eq!(report["host_sha256"], HOST);
        let query = &report["native_query"];
        assert_eq!(query["configuration_sha256"], CONFIG);
        assert_eq!(
            query["runtime_report_sha256"],
            "153b90f6c2d77ef98fd5fe673a2af008d4e518cd024e799515d45ed5120c337e"
        );
        assert_eq!(
            query["canonical_save_sha256"],
            "ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884"
        );
        assert_eq!(query["canonical_save_bytes"], 80_657_462);
        let source = &query["source_identity"];
        let canonical = serde_json::to_vec(source).unwrap();
        let actual = format!("{:x}", Sha256::digest(canonical));
        assert_eq!(actual, CONFIG);
        assert_eq!(source["schema"], "c6-linux-mod-activation-v1");
        assert_eq!(source["assets"]["Civ6"], HOST);
        assert_eq!(source["assets"]["libGameCore_XP2.so"], GAME);
        assert_eq!(source["cleanup_requested"], true);
        let gate: serde_json::Value = serde_json::from_str(include_str!(
            "../../../evidence/linux/definition-ids-static.json"
        ))
        .unwrap();
        assert_eq!(source["lower_core_definition_registry_observer"], gate);
        assert_eq!(query["canonical_core_unchanged"], true);
    }

    #[test]
    fn native_fixture_identity_rejects_host_config_and_source_mutations() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../evidence/linux/definition-ids-native-1.json"
        ))
        .unwrap();
        check_native_fixture_identity(&report);
        for pointer in [
            "/host_sha256",
            "/native_query/configuration_sha256",
            "/native_query/runtime_report_sha256",
            "/native_query/canonical_save_sha256",
            "/native_query/canonical_save_bytes",
            "/native_query/source_identity/assets/Civ6",
            "/native_query/source_identity/sources/native~1linux~1definition_ids.c",
        ] {
            let mut altered = report.clone();
            *altered.pointer_mut(pointer).unwrap() = serde_json::json!("0".repeat(64));
            assert!(std::panic::catch_unwind(|| check_native_fixture_identity(&altered)).is_err());
        }
        let mut altered = report.clone();
        altered["native_query"]["source_identity"]["lower_core_definition_registry_observer"] =
            serde_json::Value::Null;
        assert!(std::panic::catch_unwind(|| check_native_fixture_identity(&altered)).is_err());
    }

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
        check_native_fixture_identity(&report);
        let query = &report["native_query"];
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
