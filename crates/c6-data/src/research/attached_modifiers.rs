//! Rules-ordered research attachments. Unsupported recipes fail when reached;
//! they do not prevent other civic or technology definitions from being used.
use super::Kind;
use crate::{registry::linux_type_hash, Database, Row, Rules, SqlValue};
#[derive(Clone, Debug)]
pub struct Recipe {
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
        if text(dynamic, "CollectionType")? != b"COLLECTION_OWNER"
            || text(dynamic, "EffectType")? != b"EFFECT_ADJUST_TRADE_ROUTE_CAPACITY"
        {
            return Err(format!(
                "unsupported research modifier {}",
                String::from_utf8_lossy(&id)
            ));
        }
        let mut amount = 0;
        for argument in table(db, "ModifierArguments")?
            .rows()
            .filter(|r| text(*r, "ModifierId").ok() == Some(id.as_slice()))
        {
            if text(argument, "Name")? == b"Amount" {
                if text(argument, "Type")? != b"ARGTYPE_IDENTITY" {
                    return Err("dynamic research Amount".into());
                }
                amount = std::str::from_utf8(text(argument, "Value")?)
                    .map_err(|e| e.to_string())?
                    .parse()
                    .map_err(|_| "research Amount integer")?;
            }
        }
        out.push(Recipe { id, amount });
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
