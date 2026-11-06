//! Rules-ordered research attachments. Unsupported recipes fail when reached;
//! they do not prevent other civic or technology definitions from being used.
use super::Kind;
use crate::{registry::linux_type_hash, Database, Row, Rules, SqlValue};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    TradeCapacity,
    GovernorPoints,
    ReligiousTourismReduction,
    EnforceBorders,
    GrantSpy,
}
#[derive(Clone, Debug)]
pub struct Recipe {
    pub effect: Effect,
    pub id: Vec<u8>,
    pub amount: i32,
}
pub struct Definitions {
    rows: Vec<(Kind, u32, Result<Vec<Recipe>, String>)>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Self {
        let mut rows = vec![];
        for (kind, table, column) in [
            (Kind::Civic, "Civics", "CivicType"),
            (Kind::Technology, "Technologies", "TechnologyType"),
        ] {
            if let Some(table) = rules.gameplay().table(table) {
                for row in table.rows() {
                    if let Ok(name) = text(row, column) {
                        rows.push((
                            kind,
                            linux_type_hash(name),
                            compile(rules.gameplay(), kind, name),
                        ));
                    }
                }
            }
        }
        Self { rows }
    }
    pub fn recipes(&self, kind: Kind, hash: u32) -> Result<&[Recipe], String> {
        match self.rows.iter().find(|r| r.0 == kind && r.1 == hash) {
            Some((_, _, Ok(recipes))) => Ok(recipes),
            Some((_, _, Err(e))) => Err(e.clone()),
            None => Err("research attached modifier definition".into()),
        }
    }
}
fn table<'a>(db: &'a Database, name: &str) -> Result<&'a crate::Table, String> {
    db.table(name)
        .ok_or_else(|| format!("research modifier table {name}"))
}
fn text<'a>(row: Row<'a>, column: &str) -> Result<&'a [u8], String> {
    match row.get(column).map_err(|e| e.to_string())? {
        SqlValue::Text(v) => Ok(v),
        SqlValue::Null => Ok(b""),
        _ => Err(format!("research modifier text {column}")),
    }
}
fn compile(db: &Database, kind: Kind, name: &[u8]) -> Result<Vec<Recipe>, String> {
    let (join, column) = match kind {
        Kind::Civic => ("CivicModifiers", "CivicType"),
        Kind::Technology => ("TechnologyModifiers", "TechnologyType"),
    };
    let mut ids = table(db, join)?
        .rows()
        .filter(|r| text(*r, column).ok() == Some(name))
        .map(|r| text(r, "ModifierId").map(<[u8]>::to_vec))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort(); // DatabaseUtility.GetModifierDefinitions: ORDER BY ModifierId BINARY.
    let mut out = vec![];
    for id in ids {
        let Some(row) = table(db, "Modifiers")?
            .rows()
            .find(|r| text(*r, "ModifierId").ok() == Some(id.as_slice()))
        else {
            continue;
        };
        for flag in [
            "RunOnce",
            "NewOnly",
            "Permanent",
            "Repeatable",
            "OwnerStackLimit",
            "SubjectStackLimit",
        ] {
            let value = row.get(flag).map_err(|e| e.to_string())?;
            if !matches!(value, SqlValue::Null)
                && crate::game_parameters::sql_integer(value) != Some(0)
            {
                return Err(format!("research modifier unsupported {flag}"));
            }
        }
        for requirement in ["OwnerRequirementSetId", "SubjectRequirementSetId"] {
            if !text(row, requirement)?.is_empty() {
                return Err(format!("research modifier unsupported {requirement}"));
            }
        }
        let dynamic = table(db, "DynamicModifiers")?
            .rows()
            .find(|r| text(*r, "ModifierType").ok() == text(row, "ModifierType").ok())
            .ok_or("research DynamicModifier")?;
        let (effect, parameter) = match text(dynamic, "EffectType")? {
            b"EFFECT_ADJUST_TRADE_ROUTE_CAPACITY" => (Effect::TradeCapacity, b"Amount".as_slice()),
            b"EFFECT_ADJUST_PLAYER_GOVERNOR_POINTS" => {
                (Effect::GovernorPoints, b"Delta".as_slice())
            }
            b"EFFECT_ADJUST_PLAYER_RELIGIOUS_TOURISM_REDUCTION" => {
                (Effect::ReligiousTourismReduction, b"Modifier".as_slice())
            }
            b"EFFECT_ADJUST_PLAYER_ENFORCE_BORDERS" => {
                (Effect::EnforceBorders, b"Enable".as_slice())
            }
            b"EFFECT_GRANT_SPY" => (Effect::GrantSpy, b"Amount".as_slice()),
            _ => {
                return Err(format!(
                    "unsupported research modifier {}",
                    String::from_utf8_lossy(&id)
                ))
            }
        };
        if text(dynamic, "CollectionType")? != b"COLLECTION_OWNER" {
            return Err(format!(
                "unsupported research modifier {}",
                String::from_utf8_lossy(&id)
            ));
        }
        // AdjustPlayerEnforceBorders initializes Enable through GetBooleanValue(default=true).
        let mut amount = if effect == Effect::EnforceBorders {
            1
        } else {
            0
        };
        for argument in table(db, "ModifierArguments")?
            .rows()
            .filter(|r| text(*r, "ModifierId").ok() == Some(id.as_slice()))
        {
            if text(argument, "Name")? == parameter {
                if text(argument, "Type")? != b"ARGTYPE_IDENTITY" {
                    return Err("dynamic research Amount".into());
                }
                if effect == Effect::EnforceBorders {
                    amount = match text(argument, "Value")? {
                        b"0" => 0,
                        b"1" => 1,
                        _ => return Err("unsupported research Enable boolean spelling".into()),
                    };
                    continue;
                }
                amount = std::str::from_utf8(text(argument, "Value")?)
                    .map_err(|e| e.to_string())?
                    .parse()
                    .map_err(|_| "research Amount integer")?;
            }
        }
        out.push(Recipe { id, effect, amount });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-research-attachments-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE CivicModifiers(CivicType TEXT, ModifierId TEXT); CREATE TABLE TechnologyModifiers(TechnologyType TEXT, ModifierId TEXT); CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit); CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT); CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT); INSERT INTO CivicModifiers VALUES('C','Z'),('C','A'); INSERT INTO Modifiers VALUES('A','TRADE',0,0,0,0,NULL,NULL,NULL,NULL),('Z','TRADE',0,0,0,0,NULL,NULL,0,0); INSERT INTO DynamicModifiers VALUES('TRADE','COLLECTION_OWNER','EFFECT_ADJUST_TRADE_ROUTE_CAPACITY'); INSERT INTO ModifierArguments VALUES('Z','Amount','ARGTYPE_IDENTITY','-2'),('A','Amount','ARGTYPE_IDENTITY','1');").unwrap();
        connection.execute_batch(extra).unwrap();
        drop(connection);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn research_attachment_uses_join_order_and_rule_amounts() {
        let db = database("INSERT INTO TechnologyModifiers VALUES('T','Z');");
        let civic = compile(&db, Kind::Civic, b"C").unwrap();
        assert_eq!(
            civic
                .iter()
                .map(|r| (r.id.as_slice(), r.amount))
                .collect::<Vec<_>>(),
            [(b"A".as_slice(), 1), (b"Z".as_slice(), -2)]
        );
        assert_eq!(compile(&db, Kind::Technology, b"T").unwrap()[0].amount, -2);
        assert!(compile(&db, Kind::Civic, b"EMPTY").unwrap().is_empty());
    }
    #[test]
    fn research_counter_families_use_distinct_native_parameter_names() {
        for (native, parameter, family) in [
            (
                "EFFECT_ADJUST_PLAYER_GOVERNOR_POINTS",
                "Delta",
                Effect::GovernorPoints,
            ),
            (
                "EFFECT_ADJUST_PLAYER_RELIGIOUS_TOURISM_REDUCTION",
                "Modifier",
                Effect::ReligiousTourismReduction,
            ),
        ] {
            let db = database(&format!("UPDATE DynamicModifiers SET EffectType='{native}'; UPDATE ModifierArguments SET Name='{parameter}';"));
            let recipes = compile(&db, Kind::Civic, b"C").unwrap();
            assert_eq!(
                recipes
                    .iter()
                    .map(|r| (r.effect, r.amount))
                    .collect::<Vec<_>>(),
                [(family, 1), (family, -2)]
            );
            let db = database(&format!(
                "UPDATE DynamicModifiers SET EffectType='{native}';"
            ));
            assert!(compile(&db, Kind::Civic, b"C")
                .unwrap()
                .iter()
                .all(|r| r.amount == 0));
        }
    }
    #[test]
    fn border_boolean_and_spy_amount_preserve_native_defaults() {
        let db = database("UPDATE DynamicModifiers SET EffectType='EFFECT_ADJUST_PLAYER_ENFORCE_BORDERS'; UPDATE ModifierArguments SET Name='Enable',Value='0';");
        let recipes = compile(&db, Kind::Civic, b"C").unwrap();
        assert!(recipes
            .iter()
            .all(|r| r.effect == Effect::EnforceBorders && r.amount == 0));
        let db = database(
            "UPDATE DynamicModifiers SET EffectType='EFFECT_ADJUST_PLAYER_ENFORCE_BORDERS';",
        );
        assert!(compile(&db, Kind::Civic, b"C")
            .unwrap()
            .iter()
            .all(|r| r.amount == 1));
        let db = database("UPDATE DynamicModifiers SET EffectType='EFFECT_GRANT_SPY';");
        assert_eq!(
            compile(&db, Kind::Civic, b"C")
                .unwrap()
                .iter()
                .map(|r| (r.effect, r.amount))
                .collect::<Vec<_>>(),
            [(Effect::GrantSpy, 1), (Effect::GrantSpy, -2)]
        );
    }
    #[test]
    fn research_attachment_rejects_unimplemented_requirements_flags_and_effects() {
        for sql in [
            "UPDATE Modifiers SET OwnerRequirementSetId='R';",
            "UPDATE Modifiers SET SubjectRequirementSetId='R';",
            "UPDATE Modifiers SET Permanent=1;",
            "UPDATE Modifiers SET SubjectStackLimit=1;",
            "UPDATE DynamicModifiers SET CollectionType='COLLECTION_ALL_PLAYERS';",
            "UPDATE DynamicModifiers SET EffectType='UNKNOWN';",
            "UPDATE ModifierArguments SET Type='ARGTYPE_PLAYER';",
        ] {
            let db = database(sql);
            assert!(compile(&db, Kind::Civic, b"C").is_err(), "{sql}");
            assert!(compile(&db, Kind::Civic, b"EMPTY").unwrap().is_empty());
        }
    }
}
