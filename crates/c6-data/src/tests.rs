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
    let f = Fixture::new("CREATE TABLE incomplete(value);");
    let db = inspect(&f.0).unwrap();
    assert!(validate(&db, &manifest().unwrap().databases[0])
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

/// Requires genuinely exported private artifacts; absence is not a passing native test.
#[test]
#[ignore = "historical Windows coverage; requires private original-native run-1 and run-2 artifacts"]
fn historical_windows_native_exports_match_every_manifest_table() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../c6-ghidra/evidence/rules");
    let first = Rules::load(base.join("run-1"), &xp2_source_identity()).unwrap();
    let second = Rules::load(base.join("run-2"), &xp2_source_identity()).unwrap();
    assert_eq!(
        first.configuration().fingerprint(),
        second.configuration().fingerprint()
    );
    assert_eq!(
        first.gameplay().fingerprint(),
        second.gameplay().fingerprint()
    );
    assert_eq!(
        first.configuration().tables().len() + first.gameplay().tables().len(),
        506
    );
    assert_eq!(
        first.configuration().fingerprint().row_count + first.gameplay().fingerprint().row_count,
        43570
    );
    // Typed projection is compared to original SQL fields, not memorized unit values.
    for unit in first.units() {
        let row = first
            .gameplay()
            .table("Units")
            .unwrap()
            .row_by_id(unit.rules_row_id)
            .unwrap();
        assert_eq!(unit.unit_type, row.text("UnitType").unwrap());
        for (name, value) in [
            ("Cost", unit.cost),
            ("BaseMoves", unit.base_moves),
            ("Combat", unit.combat),
            ("RangedCombat", unit.ranged_combat),
            ("Range", unit.range),
            ("Maintenance", unit.maintenance),
        ] {
            assert_eq!(value, row.integer(name).unwrap());
        }
    }
}

#[test]
fn linux_profile_never_falls_back_to_historical_windows() {
    let windows = xp2_source_identity();
    assert!(
        Rules::load_profile(RulesProfile::LinuxXp2Base, "/nonexistent", &windows)
            .unwrap_err()
            .to_string()
            .contains("Linux XP2")
    );
    let mut linux = windows;
    linux.profile["target_sha256"] = json!(RulesProfile::LinuxXp2Base.gamecore_sha256());
    assert!(Rules::load("/nonexistent", &linux)
        .unwrap_err()
        .to_string()
        .contains("no Windows fallback"));
}

/// Explicitly requested Linux artifact test; missing evidence is a failure.
#[test]
#[ignore = "requires two actual private Linux native base exports"]
fn linux_native_exports_match_every_manifest_table() {
    let first_path = std::env::var_os("C6_LINUX_RULES_RUN_1")
        .expect("C6_LINUX_RULES_RUN_1 must identify actual Linux-native export");
    let second_path = std::env::var_os("C6_LINUX_RULES_RUN_2")
        .expect("C6_LINUX_RULES_RUN_2 must identify independent Linux-native export");
    let profile = RulesProfile::LinuxXp2Base;
    let source = profile.source_identity().unwrap();
    let first = Rules::load_profile(profile, Path::new(&first_path), &source).unwrap();
    let second = Rules::load_profile(profile, Path::new(&second_path), &source).unwrap();
    assert_eq!(first.profile(), RulesProfile::LinuxXp2Base);
    assert_eq!(first.source(), &source);
    assert_eq!(
        first.configuration().fingerprint(),
        second.configuration().fingerprint()
    );
    assert_eq!(
        first.gameplay().fingerprint(),
        second.gameplay().fingerprint()
    );
    for unit in first.units() {
        let row = first
            .gameplay()
            .table("Units")
            .unwrap()
            .row_by_id(unit.rules_row_id)
            .unwrap();
        assert_eq!(unit.unit_type, row.text("UnitType").unwrap());
        for (name, value) in [
            ("Cost", unit.cost),
            ("BaseMoves", unit.base_moves),
            ("Combat", unit.combat),
            ("RangedCombat", unit.ranged_combat),
            ("Range", unit.range),
            ("Maintenance", unit.maintenance),
        ] {
            assert_eq!(value, row.integer(name).unwrap());
        }
    }
}
