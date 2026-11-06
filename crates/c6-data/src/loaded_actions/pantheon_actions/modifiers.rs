//! Configured Initiation Rites attachment graph. Definitions preserve parent
//! order, global subjects and the PlayerHasPantheon requirement, rather than
//! treating the belief as a direct unconditional player bonus.
use super::Error;
use crate::{registry::linux_type_hash, Database, Row, Rules, SqlValue};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    FaithFromDispersal,
    HealingFromDispersal,
}
#[derive(Clone, Debug)]
pub struct Recipe {
    pub id: Vec<u8>,
    pub child: Vec<u8>,
    pub requirement: Vec<u8>,
    pub requirement_set: Vec<u8>,
    pub effect: Effect,
    pub amount: i32,
}
pub struct Definitions {
    pub belief_hash: u32,
    pub recipes: Vec<Recipe>,
}
fn table<'a>(db: &'a Database, name: &str) -> Result<&'a crate::Table, Error> {
    db.table(name)
        .ok_or(Error::Unsupported("pantheon modifier table"))
}
fn text<'a>(r: Row<'a>, n: &str) -> Result<&'a [u8], Error> {
    match r.get(n).map_err(Error::Rules)? {
        SqlValue::Text(t) => Ok(t),
        SqlValue::Null => Ok(b""),
        _ => Err(Error::Unsupported("pantheon modifier text")),
    }
}
fn row<'a>(db: &'a Database, t: &str, key: &str, id: &[u8]) -> Result<Row<'a>, Error> {
    table(db, t)?
        .rows()
        .find(|r| text(*r, key).ok() == Some(id))
        .ok_or(Error::Unsupported("pantheon modifier definition"))
}
fn arg<'a>(db: &'a Database, id: &[u8], name: &[u8]) -> Result<&'a [u8], Error> {
    let r = table(db, "ModifierArguments")?
        .rows()
        .find(|r| text(*r, "ModifierId").ok() == Some(id) && text(*r, "Name").ok() == Some(name))
        .ok_or(Error::Unsupported("pantheon modifier argument"))?;
    if text(r, "Type")? != b"ARGTYPE_IDENTITY" {
        return Err(Error::Unsupported("dynamic pantheon argument"));
    }
    text(r, "Value")
}
fn flags(r: Row<'_>) -> Result<(), Error> {
    for n in [
        "RunOnce",
        "NewOnly",
        "Permanent",
        "Repeatable",
        "OwnerStackLimit",
        "SubjectStackLimit",
    ] {
        if r.integer(n).map_err(Error::Rules)? != 0 {
            return Err(Error::Unsupported("pantheon modifier flags"));
        }
    }
    Ok(())
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Option<Self>, Error> {
        Self::from_database(rules.gameplay())
    }
    fn from_database(db: &Database) -> Result<Option<Self>, Error> {
        let belief = b"BELIEF_INITIATION_RITES";
        if !table(db, "Beliefs")?
            .rows()
            .any(|r| text(r, "BeliefType").ok() == Some(belief))
        {
            return Ok(None);
        }
        let mut ids = table(db, "BeliefModifiers")?
            .rows()
            .filter(|r| text(*r, "BeliefType").ok() == Some(belief))
            .map(|r| text(r, "ModifierID").map(Vec::from))
            .collect::<Result<Vec<_>, _>>()?;
        ids.sort();
        let mut recipes = vec![];
        for id in ids {
            let m = row(db, "Modifiers", "ModifierId", &id)?;
            flags(m)?;
            let dynamic = row(
                db,
                "DynamicModifiers",
                "ModifierType",
                text(m, "ModifierType")?,
            )?;
            if text(dynamic, "CollectionType")? != b"COLLECTION_ALL_PLAYERS"
                || text(dynamic, "EffectType")? != b"EFFECT_ATTACH_MODIFIER"
                || !text(m, "OwnerRequirementSetId")?.is_empty()
            {
                return Err(Error::Unsupported("Initiation Rites parent modifier"));
            }
            let set = text(m, "SubjectRequirementSetId")?;
            if text(
                row(db, "RequirementSets", "RequirementSetId", set)?,
                "RequirementSetType",
            )? != b"REQUIREMENTSET_TEST_ALL"
            {
                return Err(Error::Unsupported("pantheon requirement set"));
            }
            let reqs = table(db, "RequirementSetRequirements")?
                .rows()
                .filter(|r| text(*r, "RequirementSetId").ok() == Some(set))
                .collect::<Vec<_>>();
            if reqs.len() != 1 {
                return Err(Error::Unsupported("pantheon requirement count"));
            }
            let requirement = text(reqs[0], "RequirementId")?.to_vec();
            let req = row(db, "Requirements", "RequirementId", &requirement)?;
            if text(req, "RequirementType")? != b"REQUIREMENT_PLAYER_HAS_PANTHEON" {
                return Err(Error::Unsupported("pantheon requirement type"));
            }
            for n in ["Inverse", "Reverse", "Persistent", "Triggered"] {
                if req.integer(n).map_err(Error::Rules)? != 0 {
                    return Err(Error::Unsupported("pantheon requirement flags"));
                }
            }
            if table(db, "RequirementArguments")?
                .rows()
                .any(|r| text(r, "RequirementId").ok() == Some(&requirement))
            {
                return Err(Error::Unsupported("pantheon requirement arguments"));
            }
            let child = arg(db, &id, b"ModifierId")?.to_vec();
            let m = row(db, "Modifiers", "ModifierId", &child)?;
            flags(m)?;
            let dynamic = row(
                db,
                "DynamicModifiers",
                "ModifierType",
                text(m, "ModifierType")?,
            )?;
            if text(dynamic, "CollectionType")? != b"COLLECTION_OWNER"
                || !text(m, "OwnerRequirementSetId")?.is_empty()
                || !text(m, "SubjectRequirementSetId")?.is_empty()
            {
                return Err(Error::Unsupported("Initiation Rites child modifier"));
            }
            let effect = match text(dynamic, "EffectType")? {
                b"EFFECT_ADJUST_PLAYER_FAITH_FROM_DISPERSAL" => Effect::FaithFromDispersal,
                b"EFFECT_ADJUST_PLAYER_HEALING_FROM_DISPERSAL" => Effect::HealingFromDispersal,
                _ => return Err(Error::Unsupported("Initiation Rites effect")),
            };
            let amount = std::str::from_utf8(arg(db, &child, b"Amount")?)
                .ok()
                .and_then(|v| v.parse().ok())
                .ok_or(Error::Unsupported("pantheon effect Amount"))?;
            recipes.push(Recipe {
                id,
                child,
                requirement,
                requirement_set: set.to_vec(),
                effect,
                amount,
            });
        }
        if recipes.len() != 2
            || !recipes
                .iter()
                .any(|r| r.effect == Effect::FaithFromDispersal)
            || !recipes
                .iter()
                .any(|r| r.effect == Effect::HealingFromDispersal)
        {
            return Err(Error::Unsupported(
                "Initiation Rites complete modifier graph",
            ));
        }
        Ok(Some(Self {
            belief_hash: linux_type_hash(belief),
            recipes,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-pantheon-modifiers-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE Beliefs(BeliefType TEXT);CREATE TABLE BeliefModifiers(BeliefType TEXT,ModifierID TEXT);CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit);CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT);CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT);CREATE TABLE RequirementSets(RequirementSetId TEXT,RequirementSetType TEXT);CREATE TABLE RequirementSetRequirements(RequirementSetId TEXT,RequirementId TEXT);CREATE TABLE Requirements(RequirementId TEXT,RequirementType TEXT,Inverse,Reverse,Persistent,Triggered);CREATE TABLE RequirementArguments(RequirementId TEXT);
        INSERT INTO Beliefs VALUES('BELIEF_INITIATION_RITES');
        INSERT INTO BeliefModifiers VALUES('BELIEF_INITIATION_RITES','Z_HEAL'),('BELIEF_INITIATION_RITES','A_FAITH');
        INSERT INTO Modifiers VALUES('A_FAITH','ATTACH',0,0,0,0,NULL,'S',0,0),('Z_HEAL','ATTACH',0,0,0,0,NULL,'S',0,0),('FAITH','F',0,0,0,0,NULL,NULL,0,0),('HEAL','H',0,0,0,0,NULL,NULL,0,0);
        INSERT INTO DynamicModifiers VALUES('ATTACH','COLLECTION_ALL_PLAYERS','EFFECT_ATTACH_MODIFIER'),('F','COLLECTION_OWNER','EFFECT_ADJUST_PLAYER_FAITH_FROM_DISPERSAL'),('H','COLLECTION_OWNER','EFFECT_ADJUST_PLAYER_HEALING_FROM_DISPERSAL');
        INSERT INTO ModifierArguments VALUES('A_FAITH','ModifierId','ARGTYPE_IDENTITY','FAITH'),('Z_HEAL','ModifierId','ARGTYPE_IDENTITY','HEAL'),('FAITH','Amount','ARGTYPE_IDENTITY','50'),('HEAL','Amount','ARGTYPE_IDENTITY','100');
        INSERT INTO RequirementSets VALUES('S','REQUIREMENTSET_TEST_ALL');INSERT INTO RequirementSetRequirements VALUES('S','R');INSERT INTO Requirements VALUES('R','REQUIREMENT_PLAYER_HAS_PANTHEON',0,0,0,0);").unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn compiles_complete_sorted_global_player_graph_with_both_real_effects() {
        let d = Definitions::from_database(&database("")).unwrap().unwrap();
        assert_eq!(d.recipes.len(), 2);
        assert_eq!(d.recipes[0].id, b"A_FAITH");
        assert_eq!(d.recipes[0].effect, Effect::FaithFromDispersal);
        assert_eq!(d.recipes[0].amount, 50);
        assert_eq!(d.recipes[1].effect, Effect::HealingFromDispersal);
        assert_eq!(d.recipes[1].amount, 100);
    }
    #[test]
    fn unknown_or_missing_reached_effects_and_requirements_are_explicit() {
        for sql in ["DELETE FROM BeliefModifiers WHERE ModifierId='Z_HEAL';","UPDATE Modifiers SET Permanent=1 WHERE ModifierId='HEAL';","UPDATE Requirements SET Inverse=1;","UPDATE ModifierArguments SET Type='ARGTYPE_PROPERTY';","UPDATE DynamicModifiers SET CollectionType='COLLECTION_ALL_CITIES' WHERE ModifierType='ATTACH';"] {
            assert!(Definitions::from_database(&database(sql)).is_err(),"{sql}");
        }
    }
}
