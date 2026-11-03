use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

// Generic SQLite codec fixtures. These are not Civ VI gameplay fixtures.
struct Fixture(PathBuf);
impl Fixture {
    fn new(sql: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "c6-data-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(sql).unwrap();
        drop(connection);
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn typed_codec_matches_independent_python_vector() {
    let rows = vec![vec![
        SqlValue::Null,
        SqlValue::Integer(i64::MIN),
        SqlValue::RealBits((-0.0_f64).to_bits()),
        SqlValue::Text(vec![0, 255]),
        SqlValue::Blob(vec![0, 255]),
    ]];
    // hashlib.sha256(FORMAT+b'\0'+b'A'+pack('>Q',5)+...).hexdigest()
    assert_eq!(
        rows_digest(&rows),
        "e36d3781d98142c0453bde6362e56afa220fb3ff9facb62555de3c5b40825f59"
    );
    assert_ne!(
        rows_digest(&[vec![SqlValue::Integer(1)]]),
        rows_digest(&[vec![SqlValue::RealBits(1.0_f64.to_bits())]])
    );
    assert_ne!(
        rows_digest(&[vec![SqlValue::RealBits(0.0_f64.to_bits())]]),
        rows_digest(&[vec![SqlValue::RealBits((-0.0_f64).to_bits())]])
    );
    assert!(SqlValue::Text(vec![255]).text().is_err());
    assert!(SqlValue::Blob(b"text".to_vec()).text().is_err());
}

#[test]
fn retains_schema_rowids_unknown_fields_and_explicit_order() {
    let f = Fixture::new("CREATE TABLE generic(key TEXT, amount, extra BLOB); INSERT INTO generic(rowid,key,amount,extra) VALUES(8,'late',1.5,X'00ff'),(-3,'early',NULL,NULL); CREATE UNIQUE INDEX custom_index ON generic(key); CREATE TRIGGER custom_trigger AFTER UPDATE ON generic BEGIN SELECT 1; END; CREATE TABLE keyed(a TEXT,b INTEGER,unknown TEXT,PRIMARY KEY(b,a)) WITHOUT ROWID; INSERT INTO keyed VALUES('z',2,'preserved'),('a',1,'preserved');");
    let db = inspect(&f.0).unwrap();
    let table = db.table("generic").unwrap();
    assert_eq!(
        table
            .rows()
            .map(|r| r.row_id().unwrap())
            .collect::<Vec<_>>(),
        vec![RulesRowId(-3), RulesRowId(8)]
    );
    let late = table.row_by_id(RulesRowId(8)).unwrap();
    assert_eq!(
        late.get("amount").unwrap().real_bits().unwrap(),
        1.5_f64.to_bits()
    );
    assert_eq!(late.get("extra").unwrap(), &SqlValue::Blob(vec![0, 255]));
    assert_eq!(
        table
            .text_index("key")
            .unwrap()
            .get("late")
            .unwrap()
            .row_id(),
        Some(RulesRowId(8))
    );
    assert!(late
        .integer("amount")
        .unwrap_err()
        .to_string()
        .contains("column amount"));
    assert_eq!(
        db.table("keyed")
            .unwrap()
            .rows()
            .map(|r| r.integer("b").unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(db
        .table("keyed")
        .unwrap()
        .rows()
        .all(|r| r.row_id().is_none()));
    assert_eq!(db.schema().len(), 4);
    assert!(db
        .schema()
        .iter()
        .any(|s| s.values()[1].text().unwrap() == "trigger"));
    assert!(db
        .schema()
        .iter()
        .any(|s| s.values()[1].text().unwrap() == "index"));
}

#[test]
fn rejects_shadowed_aliases_duplicate_keys_and_invalid_utf8_views() {
    let f = Fixture::new("CREATE TABLE generic(rowid, _rowid_, oid);");
    assert!(inspect(&f.0)
        .unwrap_err()
        .to_string()
        .contains("all intrinsic rowid aliases shadowed"));
    let f = Fixture::new(
        "CREATE TABLE generic(key TEXT); INSERT INTO generic VALUES('duplicate'),('duplicate');",
    );
    assert!(inspect(&f.0)
        .unwrap()
        .table("generic")
        .unwrap()
        .text_index("key")
        .err()
        .unwrap()
        .to_string()
        .contains("duplicate TEXT key"));
    let f = Fixture::new(
        "CREATE TABLE generic(key TEXT); INSERT INTO generic VALUES(CAST(X'ff' AS TEXT));",
    );
    let db = inspect(&f.0).unwrap();
    assert_eq!(
        db.table("generic").unwrap().rows().next().unwrap().values(),
        &[SqlValue::Text(vec![255])]
    );
    assert!(db
        .table("generic")
        .unwrap()
        .text_index("key")
        .err()
        .unwrap()
        .to_string()
        .contains("invalid UTF-8"));
}

#[test]
fn schema_defaults_and_type_changes_are_detected() {
    let a = Fixture::new(
        "CREATE TABLE generic(key TEXT, n INTEGER DEFAULT 1); INSERT INTO generic VALUES('a',1);",
    );
    let b = Fixture::new(
        "CREATE TABLE generic(key TEXT, n REAL DEFAULT 1); INSERT INTO generic VALUES('a',1);",
    );
    let a = inspect(&a.0).unwrap();
    let b = inspect(&b.0).unwrap();
    assert_ne!(a.fingerprint.semantic_sha256, b.fingerprint.semantic_sha256);
    assert!(validate(&b, &a.fingerprint)
        .unwrap_err()
        .to_string()
        .contains("table generic: column metadata digest mismatch"));
}

#[test]
fn source_gate_precedes_io_and_partial_snapshots_fail() {
    let mut wrong = xp2_source_identity();
    wrong.profile["ruleset"] = json!("different");
    assert!(Rules::load("/nonexistent", &wrong)
        .unwrap_err()
        .to_string()
        .contains("source/profile"));
    let complete = Fixture::new("CREATE TABLE required(value INTEGER);");
    let expected = inspect(&complete.0).unwrap();
    let f = Fixture::new("CREATE TABLE incomplete(value);");
    let db = inspect(&f.0).unwrap();
    assert!(validate(&db, expected.fingerprint())
        .unwrap_err()
        .to_string()
        .contains("missing required table"));
    let mut sidecar = f.0.as_os_str().to_owned();
    sidecar.push("-wal");
    fs::write(&sidecar, []).unwrap();
    assert!(inspect(&f.0)
        .unwrap_err()
        .to_string()
        .contains("external -wal"));
    fs::remove_file(sidecar).unwrap();
}

#[test]
fn linux_profile_never_falls_back_to_historical_windows() {
    let profile = RulesProfile::LinuxXp2Base;
    let windows = xp2_source_identity();
    assert!(Rules::load_profile(profile, "/nonexistent", &windows)
        .unwrap_err()
        .to_string()
        .contains("source/profile differs"));
    let linux = profile.source_identity().unwrap();
    assert_eq!(linux.profile["target_sha256"], profile.gamecore_sha256());
    assert_eq!(linux.profile["id"], "linux-base-native-generated-v1");
    let mut disguised_windows = windows;
    disguised_windows.profile["target_sha256"] = json!(profile.gamecore_sha256());
    assert!(Rules::load("/nonexistent", &disguised_windows)
        .unwrap_err()
        .to_string()
        .contains("source/profile differs"));
    let mut mutations = Vec::new();
    let mut source = linux.clone();
    source.native_configuration_sha256.clear();
    mutations.push(source);
    let mut source = linux.clone();
    source.profile["coverage"] = json!("full_game");
    mutations.push(source);
    let mut source = linux.clone();
    source.module_identities = json!([]);
    mutations.push(source);
    let mut source = linux.clone();
    source.source_sequence_sha256.clear();
    mutations.push(source);
    let mut source = linux;
    source.language_identity = json!({"localization": "en_US"});
    mutations.push(source);
    for source in mutations {
        assert!(Rules::load_profile(profile, "/nonexistent", &source)
            .unwrap_err()
            .to_string()
            .contains("source/profile differs"));
    }
}

#[test]
fn shared_linux_binary_requires_exact_input_profile_before_io() {
    let profiles = [
        RulesProfile::LinuxXp2Base,
        RulesProfile::LinuxXp2ActivatedDiagnostic,
    ];
    assert_eq!(profiles[0].gamecore_sha256(), profiles[1].gamecore_sha256());
    for profile in profiles {
        let source = profile.source_identity().unwrap();
        for other in profiles {
            if other == profile {
                continue;
            }
            assert!(Rules::load_profile(other, "/nonexistent", &source)
                .unwrap_err()
                .to_string()
                .contains("source/profile differs"));
        }
        // Keeping the right binary while stripping/changing each remaining
        // descriptor field must never select another profile or access files.
        let mut mutations = Vec::new();
        let mut changed = source.clone();
        changed.native_configuration_sha256.clear();
        mutations.push(changed);
        let mut changed = source.clone();
        changed.profile["id"] = json!("unregistered-input-context");
        mutations.push(changed);
        let mut changed = source.clone();
        changed.module_identities = json!([]);
        mutations.push(changed);
        let mut changed = source.clone();
        changed.source_sequence_sha256.clear();
        mutations.push(changed);
        let mut changed = source;
        changed.language_identity = json!({"localization": "different"});
        mutations.push(changed);
        for changed in mutations {
            assert!(Rules::load("/nonexistent", &changed)
                .unwrap_err()
                .to_string()
                .contains("source/profile differs"));
        }
    }
}

#[test]
fn synthetic_database_identity_detects_rows_rowids_schema_and_physical_bytes() {
    // These are generic SQLite inputs, independent of registered native exports.
    let original = Fixture::new(
        "CREATE TABLE generic(key TEXT, value); INSERT INTO generic(rowid,key,value) VALUES(7,'a',1);",
    );
    let expected = inspect(&original.0).unwrap();
    for mutation in [
        "UPDATE generic SET value=X'01'",
        "UPDATE generic SET rowid=-7",
        "ALTER TABLE generic ADD COLUMN extra",
        "CREATE TABLE extra(value)",
        "physical-only",
    ] {
        let copy = Fixture::new("");
        fs::copy(&original.0, &copy.0).unwrap();
        if mutation == "physical-only" {
            use std::io::Write;
            fs::OpenOptions::new()
                .append(true)
                .open(&copy.0)
                .unwrap()
                .write_all(&[0])
                .unwrap();
        } else {
            Connection::open(&copy.0)
                .unwrap()
                .execute_batch(mutation)
                .unwrap();
        }
        let actual = inspect(&copy.0).unwrap();
        if mutation == "physical-only" {
            assert_eq!(
                actual.fingerprint().semantic_sha256,
                expected.fingerprint().semantic_sha256
            );
            assert_ne!(
                actual.fingerprint().file_sha256,
                expected.fingerprint().file_sha256
            );
        } else {
            assert_ne!(
                actual.fingerprint().semantic_sha256,
                expected.fingerprint().semantic_sha256
            );
        }
        assert!(
            validate(&actual, expected.fingerprint()).is_err(),
            "accepted {mutation}"
        );
    }
}
