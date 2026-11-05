//! Policy modifier definitions; live objects/effects remain in ModifierOwners.
use crate::melee_attack::backend::{
    city_creation::definitions::{Attachment, Collection},
    CityEffect,
};
use crate::{registry::linux_type_hash, Database, Row, Rules, SqlValue};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Clone, Debug)]
pub enum Recipe {
    City(Attachment),
    Units { id: Vec<u8>, amount: i32 },
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Unsupported(String),
}
pub struct Definitions {
    binding: WireBinding,
    policies: Vec<(u32, Result<Vec<Recipe>, String>)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let db = rules.gameplay();
        let mut policies = vec![];
        for row in table(db, "Policies").map_err(Error::Unsupported)?.rows() {
            let name = text(row, "PolicyType").map_err(Error::Unsupported)?;
            policies.push((linux_type_hash(name), compile(db, name)));
        }
        Ok(Self {
            binding: state.binding().clone(),
            policies,
        })
    }
    pub fn policy(&self, state: &LoadedState, hash: u32) -> Result<&[Recipe], Error> {
        if *state.binding() != self.binding {
            return Err(Error::Binding);
        }
        match self.policies.iter().find(|r| r.0 == hash) {
            Some((_, Ok(r))) => Ok(r),
            Some((_, Err(e))) => Err(Error::Unsupported(e.clone())),
            None => Err(Error::Unsupported(
                "current policy modifier definition".into(),
            )),
        }
    }
}
fn table<'a>(db: &'a Database, name: &str) -> Result<&'a crate::Table, String> {
    db.table(name).ok_or_else(|| format!("missing {name}"))
}
fn text<'a>(row: Row<'a>, column: &str) -> Result<&'a [u8], String> {
    match row.get(column).map_err(|e| e.to_string())? {
        SqlValue::Text(v) => Ok(v),
        SqlValue::Null => Ok(b""),
        _ => Err(format!("nontext {column}")),
    }
}
fn int(row: Row<'_>, column: &str) -> Result<i32, String> {
    crate::game_parameters::sql_integer(row.get(column).map_err(|e| e.to_string())?)
        .ok_or_else(|| format!("integer {column}"))
}
fn find<'a>(
    db: &'a Database,
    tab: &str,
    key: &str,
    value: &[u8],
) -> Result<Option<Row<'a>>, String> {
    Ok(table(db, tab)?
        .rows()
        .find(|r| text(*r, key).ok() == Some(value)))
}
fn argument<'a>(db: &'a Database, id: &[u8], name: &[u8]) -> Result<Option<&'a [u8]>, String> {
    for row in table(db, "ModifierArguments")?.rows() {
        if text(row, "ModifierId")? == id && text(row, "Name")? == name {
            if text(row, "Type")? != b"ARGTYPE_IDENTITY" {
                return Err("dynamic Policy modifier argument".into());
            }
            return Ok(Some(text(row, "Value")?));
        }
    }
    Ok(None)
}
fn amount(db: &Database, id: &[u8]) -> Result<i32, String> {
    let Some(v) = argument(db, id, b"Amount")? else {
        return Ok(0);
    };
    let value = std::str::from_utf8(v)
        .map_err(|e| e.to_string())?
        .parse::<i32>()
        .map_err(|_| "nonintegral Policy Amount".to_owned())?;
    if !(-8_388_608..=8_388_607).contains(&value) {
        return Err("Policy Amount Q8 range".into());
    }
    Ok(value)
}
fn compile(db: &Database, policy: &[u8]) -> Result<Vec<Recipe>, String> {
    let mut ids = vec![];
    for row in table(db, "PolicyModifiers")?.rows() {
        if text(row, "PolicyType")? == policy {
            ids.push(text(row, "ModifierId")?.to_vec());
        }
    }
    ids.sort(); // GetPolicyModifiersSql23cc560: ORDER BY ModifierId, BINARY.
    let mut out = vec![];
    for id in ids {
        let Some(row) = find(db, "Modifiers", "ModifierId", &id)? else {
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
            if int(row, flag)? != 0 {
                return Err(format!("unsupported Policy modifier {flag}"));
            }
        }
        if !text(row, "OwnerRequirementSetId")?.is_empty()
            || !text(row, "SubjectRequirementSetId")?.is_empty()
        {
            return Err("unsupported Policy modifier requirements".into());
        }
        let dynamic = find(
            db,
            "DynamicModifiers",
            "ModifierType",
            text(row, "ModifierType")?,
        )?
        .ok_or("Policy DynamicModifier definition")?;
        let effect = text(dynamic, "EffectType")?;
        let collection = text(dynamic, "CollectionType")?;
        let n = amount(db, &id)?;
        if collection == b"COLLECTION_PLAYER_UNITS"
            && effect == b"EFFECT_ADJUST_UNIT_BARBARIAN_COMBAT"
        {
            out.push(Recipe::Units { id, amount: n });
            continue;
        }
        let collection = match collection {
            b"COLLECTION_PLAYER_CITIES" => Collection::PlayerCities,
            b"COLLECTION_PLAYER_CAPITAL_CITY" => Collection::PlayerCapitalCity,
            _ => return Err("unsupported Policy collection".into()),
        };
        if effect != b"EFFECT_ADJUST_CITY_YIELD_CHANGE" {
            return Err("unsupported Policy City effect".into());
        }
        let yields = match argument(db, &id, b"YieldType")? {
            None => vec![],
            Some(name) => {
                if find(db, "Yields", "YieldType", name)?.is_none() {
                    return Err("Policy YieldType".into());
                }
                vec![linux_type_hash(name)]
            }
        };
        let amounts = argument(db, &id, b"Amount")?
            .map(|_| n)
            .into_iter()
            .collect();
        out.push(Recipe::City(Attachment {
            collection,
            id,
            effect: CityEffect::YieldChanges { yields, amounts },
            subject_requirements: None,
            owner_requirements: None,
            repeatable: false,
            run_once: false,
            permanent: false,
            new_only: false,
            owner_stack_limit: 0,
            subject_stack_limit: 0,
        }));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-policy-modifiers-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE PolicyModifiers(PolicyType TEXT,ModifierId TEXT); CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit); CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT); CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT); CREATE TABLE Yields(YieldType TEXT);
        INSERT INTO PolicyModifiers VALUES('P','Z_GOLD'),('P','A_FAITH'),('D','D');
        INSERT INTO Modifiers VALUES('Z_GOLD','C',0,0,0,0,NULL,NULL,NULL,NULL),('A_FAITH','C',0,0,0,0,NULL,NULL,NULL,NULL),('D','U',0,0,0,0,NULL,NULL,NULL,NULL);
        INSERT INTO DynamicModifiers VALUES('C','COLLECTION_PLAYER_CAPITAL_CITY','EFFECT_ADJUST_CITY_YIELD_CHANGE'),('U','COLLECTION_PLAYER_UNITS','EFFECT_ADJUST_UNIT_BARBARIAN_COMBAT');
        INSERT INTO ModifierArguments VALUES('A_FAITH','Amount','ARGTYPE_IDENTITY','1'),('A_FAITH','YieldType','ARGTYPE_IDENTITY','YIELD_FAITH'),('Z_GOLD','Amount','ARGTYPE_IDENTITY','1'),('Z_GOLD','YieldType','ARGTYPE_IDENTITY','YIELD_GOLD'),('D','Amount','ARGTYPE_IDENTITY','5');
        INSERT INTO Yields VALUES('YIELD_FAITH'),('YIELD_GOLD');").unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn native_modifier_sort_and_real_capital_and_unit_collections() {
        let db = database("");
        let rows = compile(&db, b"P").unwrap();
        assert!(
            matches!(&rows[0],Recipe::City(r) if r.id==b"A_FAITH" && r.collection==Collection::PlayerCapitalCity)
        );
        assert!(matches!(&rows[1],Recipe::City(r) if r.id==b"Z_GOLD"));
        assert!(matches!(
            compile(&db, b"D").unwrap().as_slice(),
            [Recipe::Units { amount: 5, .. }]
        ));
        assert!(compile(&db, b"ABSENT").unwrap().is_empty());
    }
    #[test]
    fn unsupported_requirements_flags_and_dynamic_arguments_fail_explicitly() {
        assert!(compile(&database("UPDATE Modifiers SET NewOnly=1;"), b"D").is_err());
        assert!(compile(
            &database("UPDATE Modifiers SET SubjectRequirementSetId='X';"),
            b"D"
        )
        .is_err());
        assert!(compile(
            &database("UPDATE ModifierArguments SET Type='ARGTYPE_LINEAR_SCALE';"),
            b"D"
        )
        .is_err());
    }
}
