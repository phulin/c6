//! Rules-backed inputs for nine definition-only World Congress filters.
//!
//! This constructs a fresh, ordinary Linux AppGameInfo cache projection once
//! from the initialized rules snapshot. It derives Hash from Types.Hash and
//! computes numeric-key array growth before admitting row order. No per-call
//! native export, SQLite-rowid-as-iteration assumption, or new type hash is used.
//! Actual startup must bind GameInfo to this database and ordinary cache builder;
//! existing mutated rows, replacement bindings and rebuilds need their real state.
//! This does not register callbacks or establish native gameplay parity.
use crate::world_congress_options::*;
use crate::{Database, Row, Rules, RulesProfile, SqlValue, Table};

#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    UnsupportedProfile,
    MissingTable(&'static str),
    RowKeys(&'static str),
    HashTail(&'static str),
    PrimaryKey(&'static str),
    Scalar(&'static str, &'static str),
    Hash(&'static str),
    NavigationOverride(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "World Congress rules projection: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<crate::Error> for Error {
    fn from(e: crate::Error) -> Self {
        Self::Rules(e)
    }
}

/// Complete successfully acquired definition inputs. Borrowed values stay bound
/// to the immutable Rules snapshot. Iterator acquisition errors are excluded.
pub struct Definitions<'a> {
    resources: Vec<ResourceConsumptionRow<'a>>,
    yields: Vec<YieldRow<'a>>,
    buildings: Vec<BuildingRow<'a>>,
    governors: Vec<GovernorRow<'a>>,
    governments: Vec<GovernmentRow<'a>>,
    slots: Vec<GovernmentSlotRow<'a>>,
    projects: Vec<ProjectRow<'a>>,
    features: Vec<FeatureRow<'a>>,
    operations: Vec<OperationRow<'a>>,
    promotions: Vec<PromotionClassRow<'a>>,
}
impl<'a> Definitions<'a> {
    /// Model fresh ordinary AppGameInfo initialization, not an assertion that an
    /// existing script cache is pristine. Linux Rules identity is already gated
    /// by Rules::load_profile; actual runtime database/cache binding is separate.
    pub fn from_fresh_rules_cache(rules: &'a Rules) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            RulesProfile::LinuxXp2Base | RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::UnsupportedProfile);
        }
        Self::from_database(rules.gameplay())
    }
    fn from_database(db: &'a Database) -> Result<Self, Error> {
        let types = db.table("Types").ok_or(Error::MissingTable("Types"))?;
        let resource_table = CacheTable::new(db, "Resources", &["ResourceType", "PowerProvided"])?;
        if resource_table.primary != Some("ResourceType")
            || resource_table
                .table
                .columns()
                .iter()
                .any(|c| c == "PrimaryKey")
        {
            return Err(Error::PrimaryKey("Resources"));
        }
        let mut resource_rows = Vec::new();
        for &row in &resource_table.rows {
            let key = resource_table.text(row, "ResourceType")?;
            resource_rows.push((
                key,
                ResourceRow {
                    hash: resource_table.hash(row, types)?,
                    power_provided: resource_table.scalar(row, "PowerProvided")?,
                },
            ));
        }
        let consumption = CacheTable::new(db, "Resource_Consumption", &["ResourceType"])?;
        let resources = consumption
            .rows
            .iter()
            .map(|&row| {
                let key = consumption.text(row, "ResourceType")?;
                // IndexTable string lookup consults PrimaryKey. Later identical keys
                // overwrite earlier entries during cache construction.
                Ok(ResourceConsumptionRow {
                    resource: key.and_then(|key| {
                        resource_rows
                            .iter()
                            .rev()
                            .find(|(k, _)| *k == Some(key))
                            .map(|(_, r)| *r)
                    }),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        macro_rules! names {
            ($table:literal, $column:literal, $ty:ident, $field:ident) => {{
                let t = CacheTable::new(db, $table, &[$column])?;
                t.rows
                    .iter()
                    .map(|&row| {
                        Ok($ty {
                            hash: t.hash(row, types)?,
                            $field: t.text(row, $column)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?
            }};
        }
        let yields = names!("Yields", "YieldType", YieldRow, yield_type);
        let buildings = names!("Buildings", "BuildingType", BuildingRow, building_type);
        let governors = names!("Governors", "TraitType", GovernorRow, trait_type);
        let governments = names!(
            "Governments",
            "GovernmentType",
            GovernmentRow,
            government_type
        );
        let features = names!("Features", "FeatureType", FeatureRow, feature_type);
        let promotions = names!(
            "UnitPromotionClasses",
            "PromotionClassType",
            PromotionClassRow,
            promotion_class_type
        );
        let t = CacheTable::new(
            db,
            "Government_SlotCounts",
            &["GovernmentType", "GovernmentSlotType"],
        )?;
        let slots = t
            .rows
            .iter()
            .map(|&r| {
                Ok(GovernmentSlotRow {
                    government_type: t.text(r, "GovernmentType")?,
                    slot_type: t.text(r, "GovernmentSlotType")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let t = CacheTable::new(db, "Projects", &["SpaceRace", "WMD"])?;
        let projects = t
            .rows
            .iter()
            .map(|&r| {
                Ok(ProjectRow {
                    hash: t.hash(r, types)?,
                    space_race: t.scalar(r, "SpaceRace")?,
                    wmd: t.scalar(r, "WMD")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let t = CacheTable::new(db, "UnitOperations", &["CategoryInUI", "BaseProbability"])?;
        let operations = t
            .rows
            .iter()
            .map(|&r| {
                let LuaScalar::Number(base_probability) = t.scalar(r, "BaseProbability")? else {
                    return Err(Error::Scalar(t.name, "BaseProbability"));
                };
                Ok(OperationRow {
                    hash: t.hash(r, types)?,
                    category_in_ui: t.text(r, "CategoryInUI")?,
                    base_probability,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            resources,
            yields,
            buildings,
            governors,
            governments,
            slots,
            projects,
            features,
            operations,
            promotions,
        })
    }
    pub fn power_resource_ban<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.power_resource_ban(&self.resources)
    }
    pub fn yield_ban<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.yield_ban(&self.yields)
    }
    pub fn power_building<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.power_building(&self.buildings)
    }
    pub fn governance_doctrine<'b>(
        &self,
        c: &'b mut WorldCongressOptionCaches,
    ) -> &'b [OptionHash] {
        c.governance_doctrine(&self.governors)
    }
    pub fn world_ideology<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.world_ideology(&self.governments, &self.slots)
    }
    pub fn public_works<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.public_works(&self.projects)
    }
    pub fn deforestation_feature<'b>(
        &self,
        c: &'b mut WorldCongressOptionCaches,
    ) -> &'b [OptionHash] {
        c.deforestation_feature(&self.features)
    }
    pub fn espionage_pact<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.espionage_pact(&self.operations)
    }
    pub fn military_advisory<'b>(&self, c: &'b mut WorldCongressOptionCaches) -> &'b [OptionHash] {
        c.military_advisory(&self.promotions)
    }
}

struct CacheTable<'a> {
    name: &'static str,
    table: &'a Table,
    rows: Vec<Row<'a>>,
    primary: Option<&'a str>,
}
impl<'a> CacheTable<'a> {
    fn new(db: &'a Database, name: &'static str, fields: &[&str]) -> Result<Self, Error> {
        let table = db.table(name).ok_or(Error::MissingTable(name))?;
        let rows: Vec<_> = table.rows().collect();
        let keys = rows
            .iter()
            .map(|r| r.row_id().map(|x| x.0).ok_or(Error::RowKeys(name)))
            .collect::<Result<Vec<_>, Error>>()?;
        if !array_only_order(&keys) {
            return Err(Error::HashTail(name));
        }
        let mut primary = None;
        for meta in table.column_metadata() {
            let pk = meta.get(5).ok_or(Error::PrimaryKey(name))?.integer()?;
            if pk != 0 {
                if pk != 1 || primary.is_some() {
                    primary = None;
                    break;
                }
                primary = Some(meta.get(1).ok_or(Error::PrimaryKey(name))?.text()?);
            }
        }
        if let Some(nav) = db.table("NavigationProperties") {
            for r in nav.rows() {
                if r.text("BaseTable")? == name {
                    let p = r.text("PropertyName")?;
                    if matches!(p, "Hash" | "PrimaryKey" | "RowId" | "Index")
                        || primary == Some(p)
                        || fields.contains(&p)
                    {
                        return Err(Error::NavigationOverride(name));
                    }
                }
            }
        }
        Ok(Self {
            name,
            table,
            rows,
            primary,
        })
    }
    fn scalar(&self, row: Row<'a>, field: &'static str) -> Result<LuaScalar<'a>, Error> {
        if !self.table.columns().iter().any(|c| c == field) {
            return Ok(LuaScalar::Nil);
        }
        let v = row.get(field)?;
        let declaration = self
            .table
            .column_metadata()
            .iter()
            .find(|m| m.get(1).and_then(|v| v.text().ok()) == Some(field))
            .and_then(|m| m.get(2))
            .and_then(|v| v.text().ok())
            .ok_or(Error::Scalar(self.name, field))?;
        match v {
            SqlValue::Null => Ok(LuaScalar::Nil),
            SqlValue::Integer(n) if declaration.eq_ignore_ascii_case("boolean") => {
                Ok(LuaScalar::Boolean((*n as i32) != 0))
            }
            SqlValue::Integer(n) => Ok(LuaScalar::Number(f64::from(*n as i32))),
            SqlValue::RealBits(n) => Ok(LuaScalar::Number(f64::from_bits(*n))),
            SqlValue::Text(s) => Ok(LuaScalar::String(
                std::str::from_utf8(s.split(|&b| b == 0).next().unwrap())
                    .map_err(|_| Error::Scalar(self.name, field))?,
            )),
            SqlValue::Blob(_) => Err(Error::Scalar(self.name, field)),
        }
    }
    fn text(&self, row: Row<'a>, field: &'static str) -> Result<Option<&'a str>, Error> {
        match self.scalar(row, field)? {
            LuaScalar::Nil => Ok(None),
            LuaScalar::String(s) => Ok(Some(s)),
            _ => Err(Error::Scalar(self.name, field)),
        }
    }
    fn hash(&self, row: Row<'a>, types: &Table) -> Result<OptionHash, Error> {
        // Actual SQL columns are applied after synthetic fields.
        if self.table.columns().iter().any(|c| c == "Hash") {
            return match (row.get("Hash")?, self.scalar(row, "Hash")?) {
                (SqlValue::Integer(n), LuaScalar::Number(_)) => Ok(*n as u32),
                _ => Err(Error::Hash(self.name)),
            };
        }
        let primary = self.primary.ok_or(Error::PrimaryKey(self.name))?;
        let key = row.get(primary)?.text()?;
        if key.as_bytes().contains(&0) {
            return Err(Error::Hash(self.name));
        }
        let mut found = None;
        for r in types.rows() {
            if r.text("Type")? == key {
                if found.is_some() {
                    return Err(Error::Hash(self.name));
                }
                found = Some(match r.get("Hash")? {
                    SqlValue::Integer(n) => *n as u32,
                    _ => return Err(Error::Hash(self.name)),
                });
            }
        }
        found.ok_or(Error::Hash(self.name))
    }
}

/// Abstract only array/hash occupancy: fresh table, ascending unique positive
/// signed keys, no deletes. Rehash density is computed before the new key is
/// inserted. A final hash tail is refused rather than assigned sorted order.
fn array_only_order(keys: &[i64]) -> bool {
    let mut array = 0usize;
    let mut hash_capacity = 0usize;
    let mut hash_count = 0usize;
    let mut previous = 0i64;
    for (i, &key) in keys.iter().enumerate() {
        if key <= previous || key > 0x1000000 {
            return false;
        }
        previous = key;
        if key as usize <= array {
            continue;
        }
        if hash_count < hash_capacity {
            hash_count += 1;
            continue;
        }
        array = 0;
        let mut power = 1usize;
        while power <= 0x1000000 {
            let count = keys[..=i].partition_point(|&x| x <= power as i64);
            if count > power / 2 {
                array = power;
            }
            if power >= key as usize {
                break;
            }
            power *= 2;
        }
        hash_count = keys[..=i].iter().filter(|&&x| x as usize > array).count();
        hash_capacity = if hash_count == 0 {
            0
        } else {
            hash_count.next_power_of_two()
        };
    }
    hash_count == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-congress-rules-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE Types(Type TEXT PRIMARY KEY,Hash INTEGER);
        CREATE TABLE Resources(ResourceType TEXT PRIMARY KEY);
        CREATE TABLE Resource_Consumption(ResourceType TEXT);
        CREATE TABLE Yields(YieldType TEXT PRIMARY KEY);
        CREATE TABLE Buildings(BuildingType TEXT PRIMARY KEY);
        CREATE TABLE Governors(GovernorType TEXT PRIMARY KEY,TraitType TEXT);
        CREATE TABLE Governments(GovernmentType TEXT PRIMARY KEY);
        CREATE TABLE Government_SlotCounts(GovernmentType TEXT,GovernmentSlotType TEXT);
        CREATE TABLE Projects(ProjectType TEXT PRIMARY KEY,SpaceRace BOOLEAN,WMD BOOLEAN);
        CREATE TABLE Features(FeatureType TEXT PRIMARY KEY);
        CREATE TABLE UnitOperations(OperationType TEXT PRIMARY KEY,CategoryInUI TEXT,BaseProbability INTEGER);
        CREATE TABLE UnitPromotionClasses(PromotionClassType TEXT PRIMARY KEY);
        CREATE TABLE NavigationProperties(BaseTable TEXT,PropertyName TEXT);
        INSERT INTO Resources VALUES('RESOURCE_COAL');
        INSERT INTO Resource_Consumption VALUES('RESOURCE_COAL'),('MISSING'),('RESOURCE_COAL');
        INSERT INTO Yields VALUES('YIELD_FAITH'),('YIELD_GOLD'),('YIELD_FOOD');
        INSERT INTO Buildings VALUES('BUILDING_POWER_PLANT');
        INSERT INTO Governors VALUES('GOVERNOR_A',NULL),('GOVERNOR_B','');
        INSERT INTO Governments VALUES('GOVERNMENT_A');
        INSERT INTO Government_SlotCounts VALUES('GOVERNMENT_A','SLOT_WILDCARD'),('GOVERNMENT_A','SLOT_WILDCARD');
        INSERT INTO Projects VALUES('PROJECT_FALSE',0,0),('PROJECT_SPACE',1,0),('PROJECT_WMD',0,1);
        INSERT INTO Features VALUES('FEATURE_FOREST');
        INSERT INTO UnitOperations VALUES('UNITOP_A','OFFENSIVESPY',1),('UNITOP_B','OFFENSIVESPY',0);
        INSERT INTO UnitPromotionClasses VALUES('PROMOTION_CLASS_MELEE');
        INSERT INTO Types VALUES('RESOURCE_COAL',-1),('YIELD_FAITH',11),('YIELD_GOLD',12),('YIELD_FOOD',13),('BUILDING_POWER_PLANT',21),('GOVERNOR_A',31),('GOVERNOR_B',32),('GOVERNMENT_A',41),('PROJECT_FALSE',51),('PROJECT_SPACE',52),('PROJECT_WMD',53),('FEATURE_FOREST',61),('UNITOP_A',71),('UNITOP_B',72),('PROMOTION_CLASS_MELEE',81);") .unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn all_nine_read_rules_hashes_order_and_boolean_types() {
        let db = database("");
        let d = Definitions::from_database(&db).unwrap();
        let mut c = WorldCongressOptionCaches::new_script();
        // Missing PowerProvided is Lua nil; nil ~= 0 is true, including duplicates.
        assert_eq!(d.power_resource_ban(&mut c), &[u32::MAX, u32::MAX]);
        assert_eq!(d.yield_ban(&mut c), &[11, 12]);
        assert_eq!(d.power_building(&mut c), &[21]);
        assert_eq!(d.governance_doctrine(&mut c), &[31]);
        assert_eq!(d.world_ideology(&mut c), &[41]);
        assert_eq!(d.public_works(&mut c), &[52, 53]);
        assert_eq!(d.deforestation_feature(&mut c), &[61]);
        assert_eq!(d.espionage_pact(&mut c), &[71]);
        assert_eq!(d.military_advisory(&mut c), &[81]);
        c.unit_yields = Some(vec![99, 99]);
        assert_eq!(d.yield_ban(&mut c), &[99, 99]);
    }
    #[test]
    fn scalar_conversion_distinguishes_declared_boolean_and_integer_zero() {
        let db = database(
            "ALTER TABLE Projects ADD COLUMN Ordinary INTEGER; UPDATE Projects SET Ordinary=0;",
        );
        let t = CacheTable::new(&db, "Projects", &[]).unwrap();
        let r = t.rows[0];
        assert_eq!(t.scalar(r, "SpaceRace").unwrap(), LuaScalar::Boolean(false));
        assert_eq!(t.scalar(r, "Ordinary").unwrap(), LuaScalar::Number(0.0));
        assert_eq!(t.scalar(r, "Absent").unwrap(), LuaScalar::Nil);
    }
    #[test]
    fn row_order_requires_array_closure_not_sql_sorting_alone() {
        assert!(array_only_order(&[1, 3, 4]));
        assert!(!array_only_order(&[1, 3]));
        assert!(!array_only_order(&[0, 1]));
        let db = database("UPDATE Yields SET rowid=100 WHERE YieldType='YIELD_FOOD';");
        assert!(matches!(
            Definitions::from_database(&db),
            Err(Error::HashTail("Yields"))
        ));
    }
    #[test]
    fn actual_columns_override_synthetic_hash_and_scalar_navigation_refuses() {
        let db = database(
            "ALTER TABLE Yields ADD COLUMN Hash INTEGER; UPDATE Yields SET Hash=-2147483648;",
        );
        let d = Definitions::from_database(&db).unwrap();
        assert_eq!(
            d.yield_ban(&mut WorldCongressOptionCaches::new_script()),
            &[0x80000000, 0x80000000]
        );
        let db = database("INSERT INTO NavigationProperties VALUES('Resources','PrimaryKey');");
        assert!(matches!(
            Definitions::from_database(&db),
            Err(Error::NavigationOverride("Resources"))
        ));
        let db = database("ALTER TABLE Resources ADD COLUMN PrimaryKey TEXT; UPDATE Resources SET PrimaryKey='OTHER';");
        assert!(matches!(
            Definitions::from_database(&db),
            Err(Error::PrimaryKey("Resources"))
        ));
        let db = database("ALTER TABLE Yields ADD COLUMN Hash BOOLEAN; UPDATE Yields SET Hash=1;");
        assert!(matches!(
            Definitions::from_database(&db),
            Err(Error::Hash("Yields"))
        ));
    }
}
