//! Finite Linux XP2 static gameplay parameters initialized from admitted Rules.
//!
//! This mirrors the supported GlobalParameters::Initialize name/default/type
//! mappings. It does not initialize mutable Game controls such as pause,
//! automation, turn-complete flags, current turn, diplomacy or player state.
//! No live capture of these deterministic rules constants is required.

use crate::{Database, Rules, RulesProfile, SqlValue, Table};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerParameter {
    CityMovementCost,
    MovementWhileEmbarkedBase,
    CombatMaxHitPoints,
    CombatMaxNumAttacks,
    HealCityGarrison,
    HealLandEnemy,
    HealLandFriendly,
    HealLandNeutral,
    HealNavalEnemy,
    HealNavalFriendly,
    HealNavalNeutral,
    ReligionHealingMultiplier,
    StrategicResourceMinimumForUnitHealing,
}

impl IntegerParameter {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CityMovementCost => "CITY_MOVEMENT_COST",
            Self::MovementWhileEmbarkedBase => "MOVEMENT_WHILE_EMBARKED_BASE",
            Self::CombatMaxHitPoints => "COMBAT_MAX_HIT_POINTS",
            Self::CombatMaxNumAttacks => "COMBAT_MAX_NUM_ATTACKS",
            Self::HealCityGarrison => "COMBAT_HEAL_CITY_GARRISON",
            Self::HealLandEnemy => "COMBAT_HEAL_LAND_ENEMY",
            Self::HealLandFriendly => "COMBAT_HEAL_LAND_FRIENDLY",
            Self::HealLandNeutral => "COMBAT_HEAL_LAND_NEUTRAL",
            Self::HealNavalEnemy => "COMBAT_HEAL_NAVAL_ENEMY",
            Self::HealNavalFriendly => "COMBAT_HEAL_NAVAL_FRIENDLY",
            Self::HealNavalNeutral => "COMBAT_HEAL_NAVAL_NEUTRAL",
            Self::ReligionHealingMultiplier => "RELIGION_HEALING_MULTIPLIER",
            Self::StrategicResourceMinimumForUnitHealing => {
                "STRATEGIC_RESOURCE_MINIMUM_FOR_UNIT_HEALING"
            }
        }
    }

    /// Native initializer fallback when the complete query has no row. A present
    /// NULL or nonnumeric value is converted instead of taking this fallback.
    pub const fn native_default(self) -> i32 {
        match self {
            Self::CityMovementCost
            | Self::CombatMaxNumAttacks
            | Self::StrategicResourceMinimumForUnitHealing => 1,
            Self::MovementWhileEmbarkedBase => 2,
            Self::CombatMaxHitPoints => 100,
            Self::HealCityGarrison | Self::HealNavalFriendly => 20,
            Self::HealLandEnemy => 5,
            Self::HealLandFriendly => 15,
            Self::HealLandNeutral => 10,
            Self::HealNavalEnemy | Self::HealNavalNeutral => 0,
            Self::ReligionHealingMultiplier => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedProfile,
    MissingTable,
    MissingColumn(&'static str),
    /// The native LIMIT 1 result is ambiguous without admitted uniqueness/order.
    DuplicateName(&'static str),
    UnsupportedScalar(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "static gameplay parameter: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Immutable rules provider, separate from current mutable Game controls.
/// Rules::load_profile already verifies the complete database and configuration
/// identities. Borrowing those admitted rules needs no additional witness.
pub struct Parameters<'a> {
    table: &'a Table,
}

impl<'a> Parameters<'a> {
    pub fn from_rules(rules: &'a Rules) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            RulesProfile::LinuxXp2Base | RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::UnsupportedProfile);
        }
        Self::from_database(rules.gameplay())
    }

    fn from_database(database: &'a Database) -> Result<Self, Error> {
        let table = database
            .table("GlobalParameters")
            .ok_or(Error::MissingTable)?;
        for column in ["Name", "Value"] {
            if !table.columns().iter().any(|c| c == column) {
                return Err(Error::MissingColumn(column));
            }
        }
        Ok(Self { table })
    }

    fn value(&self, name: &'static str) -> Result<Option<&'a SqlValue>, Error> {
        let mut found = None;
        for row in self.table.rows() {
            if row.text("Name").ok() == Some(name) {
                if found.is_some() {
                    return Err(Error::DuplicateName(name));
                }
                found = Some(
                    row.get("Value")
                        .map_err(|_| Error::MissingColumn("Value"))?,
                );
            }
        }
        Ok(found)
    }

    pub fn integer(&self, parameter: IntegerParameter) -> Result<i32, Error> {
        match self.value(parameter.name())? {
            None => Ok(parameter.native_default()),
            Some(value) => sql_integer(value).ok_or(Error::UnsupportedScalar(parameter.name())),
        }
    }

    /// Native unsigned parameter (default 1), read with Query::GetInt and stored
    /// as the same 32 bits. Negative SQL values therefore wrap to unsigned.
    pub fn plot_unit_limit(&self) -> Result<u32, Error> {
        match self.value("PLOT_UNIT_LIMIT")? {
            None => Ok(1),
            Some(value) => sql_integer(value)
                .map(|n| n as u32)
                .ok_or(Error::UnsupportedScalar("PLOT_UNIT_LIMIT")),
        }
    }

    /// Preserve bits for existing query interfaces whose signed scalar is
    /// subsequently compared as unsigned by the native rules calculation.
    pub fn plot_unit_limit_i32_bits(&self) -> Result<i32, Error> {
        self.plot_unit_limit().map(|n| n as i32)
    }

    /// Native GetBool, default false. TEXT is true only when its FIRST byte is
    /// '1', 'T' or 't'; this differs from integer parsing and Rust bool parsing.
    pub fn unlock_embarkation(&self) -> Result<bool, Error> {
        Ok(match self.value("UNLOCK_EMBARKATION")? {
            None | Some(SqlValue::Null | SqlValue::RealBits(_) | SqlValue::Blob(_)) => false,
            Some(SqlValue::Integer(n)) => (*n as i32) != 0,
            Some(SqlValue::Text(bytes)) => matches!(bytes.first(), Some(b'1' | b'T' | b't')),
        })
    }

    /// Inputs for the supported healing amount calculation, with native
    /// initializer defaults applied only to genuinely absent parameter rows.
    pub fn healing_rules(&self) -> Result<c6_core::unit_healing::HealingRules, Error> {
        use c6_core::unit_healing::{HealingRules, TerritoryHealing};
        use IntegerParameter::*;
        Ok(HealingRules {
            naval: TerritoryHealing {
                friendly: self.integer(HealNavalFriendly)?,
                neutral: self.integer(HealNavalNeutral)?,
                enemy: self.integer(HealNavalEnemy)?,
            },
            land: TerritoryHealing {
                friendly: self.integer(HealLandFriendly)?,
                neutral: self.integer(HealLandNeutral)?,
                enemy: self.integer(HealLandEnemy)?,
            },
            city_garrison: self.integer(HealCityGarrison)?,
        })
    }
}

// Query::GetInt delegates to sqlite3_column_int: convert to i64, then retain
// low 32 bits. TEXT/BLOB use the signed decimal prefix (not exponent/hex),
// skipping ASCII SQL whitespace. The target parser accumulates in wrapping
// u64, then chooses an i64 endpoint when the accumulated sign bit is set.
// Its validation/range status is ignored by sqlite3_value_int; do not replace
// this old target behavior with a modern saturating decimal parser.
fn sql_integer(value: &SqlValue) -> Option<i32> {
    Some(match value {
        SqlValue::Null => 0,
        SqlValue::Integer(n) => *n as i32,
        SqlValue::Text(bytes) | SqlValue::Blob(bytes) => decimal_prefix(bytes) as i32,
        SqlValue::RealBits(bits) => {
            let value = f64::from_bits(*bits);
            // SQLite normalizes bound NaN to NULL; an admitted inspected
            // database cannot supply a REAL NaN. Refuse an invented one.
            if value.is_nan() {
                return None;
            }
            (value as i64) as i32
        }
    })
}

fn decimal_prefix(mut bytes: &[u8]) -> i64 {
    while bytes
        .first()
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12))
    {
        bytes = &bytes[1..];
    }
    let negative = bytes.first() == Some(&b'-');
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        bytes = &bytes[1..];
    }
    let mut magnitude = 0_u64;
    for byte in bytes.iter().take_while(|b| b.is_ascii_digit()) {
        magnitude = magnitude
            .wrapping_mul(10)
            .wrapping_add(u64::from(*byte - b'0'));
    }
    if magnitude > i64::MAX as u64 {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -(magnitude as i64)
    } else {
        magnitude as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn database(sql: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-static-parameters-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        drop(c);
        let database = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        database
    }

    #[test]
    fn absent_row_defaults_differ_from_present_zero_null_and_unsigned_bits() {
        let db = database("CREATE TABLE GlobalParameters(Name TEXT,Value); INSERT INTO GlobalParameters VALUES('PLOT_UNIT_LIMIT',-1),('COMBAT_MAX_HIT_POINTS',NULL),('COMBAT_HEAL_CITY_GARRISON','0');");
        let p = Parameters::from_database(&db).unwrap();
        assert_eq!(p.plot_unit_limit().unwrap(), u32::MAX);
        assert_eq!(p.plot_unit_limit_i32_bits().unwrap(), -1);
        assert_eq!(p.integer(IntegerParameter::CombatMaxHitPoints).unwrap(), 0);
        assert_eq!(p.integer(IntegerParameter::CombatMaxNumAttacks).unwrap(), 1);
        assert_eq!(p.healing_rules().unwrap().city_garrison, 0);
        assert_eq!(p.healing_rules().unwrap().land.friendly, 15);
        assert_eq!(p.healing_rules().unwrap().naval.enemy, 0);
        assert!(!p.unlock_embarkation().unwrap());
        let db = database("CREATE TABLE GlobalParameters(Name TEXT,Value);");
        assert_eq!(
            Parameters::from_database(&db)
                .unwrap()
                .plot_unit_limit()
                .unwrap(),
            1
        );
    }

    #[test]
    fn configured_values_and_duplicates_are_not_replaced_by_defaults() {
        let db = database("CREATE TABLE GlobalParameters(Name TEXT,Value); INSERT INTO GlobalParameters VALUES('PLOT_UNIT_LIMIT','3'),('CITY_MOVEMENT_COST',' -12tail'),('UNLOCK_EMBARKATION','true');");
        let p = Parameters::from_database(&db).unwrap();
        assert_eq!(p.plot_unit_limit().unwrap(), 3);
        assert_eq!(p.integer(IntegerParameter::CityMovementCost).unwrap(), -12);
        assert!(p.unlock_embarkation().unwrap());
        let db = database("CREATE TABLE GlobalParameters(Name TEXT,Value); INSERT INTO GlobalParameters VALUES('PLOT_UNIT_LIMIT',1),('PLOT_UNIT_LIMIT',2);");
        assert_eq!(
            Parameters::from_database(&db).unwrap().plot_unit_limit(),
            Err(Error::DuplicateName("PLOT_UNIT_LIMIT"))
        );
        let db = database("CREATE TABLE Other(Name TEXT);");
        assert!(matches!(
            Parameters::from_database(&db),
            Err(Error::MissingTable)
        ));
    }

    #[test]
    fn sqlite_integer_conversions_match_independent_sqlite_cast_boundaries() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        for text in [
            "",
            "not numeric",
            "0",
            "-0",
            "+13",
            "  \t-2147483649suffix",
            "2147483648",
            "4294967297",
            "9223372036854775807",
            "9223372036854775808",
            "-9223372036854775808",
            "-9223372036854775809",
            "1e3",
            "0x20",
            "12.75",
            "--2",
            "2\0tail",
        ] {
            let expected: i64 = connection
                .query_row("SELECT CAST(?1 AS INTEGER)", [text], |r| r.get(0))
                .unwrap();
            assert_eq!(
                sql_integer(&SqlValue::Text(text.as_bytes().to_vec())),
                Some(expected as i32),
                "{text:?}"
            );
            assert_eq!(
                sql_integer(&SqlValue::Blob(text.as_bytes().to_vec())),
                Some(expected as i32),
                "blob {text:?}"
            );
        }
        for number in [
            0.0_f64,
            -0.0,
            12.75,
            -12.75,
            2147483648.0,
            1e30,
            -1e30,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let expected: i64 = connection
                .query_row("SELECT CAST(?1 AS INTEGER)", [number], |r| r.get(0))
                .unwrap();
            assert_eq!(
                sql_integer(&SqlValue::RealBits(number.to_bits())),
                Some(expected as i32)
            );
        }
        assert_eq!(sql_integer(&SqlValue::RealBits(f64::NAN.to_bits())), None);
    }

    #[test]
    fn target_ignores_decimal_parser_status_after_wrapping_accumulation() {
        // Target 3d6cc17 accumulates with wrapping LEA/ADD; caller 3d52d40
        // reads its written value without checking the returned status.
        for (text, expected) in [
            ("18446744073709551615", -1_i32), // sign bit clamps to i64::MAX
            ("-18446744073709551615", 0),     // sign bit clamps to i64::MIN
            ("18446744073709551616", 0),
            ("18446744073709551617", 1),
            ("-18446744073709551617", -1),
            ("00018446744073709551617trailing", 1),
        ] {
            assert_eq!(
                sql_integer(&SqlValue::Text(text.as_bytes().to_vec())),
                Some(expected),
                "{text}"
            );
        }
    }

    #[test]
    fn native_bool_text_checks_first_byte_without_integer_or_boolean_parsing() {
        for (value, expected) in [
            ("true", true),
            ("TRUE", true),
            ("turtle", true),
            ("1suffix", true),
            ("2", false),
            ("-1", false),
            (" true", false),
            ("false", false),
            ("", false),
        ] {
            let db = database(&format!("CREATE TABLE GlobalParameters(Name TEXT,Value); INSERT INTO GlobalParameters VALUES('UNLOCK_EMBARKATION','{value}');"));
            assert_eq!(
                Parameters::from_database(&db)
                    .unwrap()
                    .unlock_embarkation()
                    .unwrap(),
                expected,
                "{value:?}"
            );
        }
    }
}
