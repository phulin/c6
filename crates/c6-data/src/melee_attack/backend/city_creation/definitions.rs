//! Static Building attachment definitions; mutable ownership stays in EffectRuntime.
//! 23e048b supplies `ORDER BY ModifierId` to DatabaseUtility23cc2a0.
use super::super::CityEffect;
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash, Database, Row, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    melee_attack::proposals::subject_sets as sets,
};

#[derive(Clone, Debug)]
pub struct Attachment {
    pub id: Vec<u8>,
    pub effect: CityEffect,
    pub subject_requirements: Option<Vec<u8>>,
    pub owner_requirements: Option<Vec<u8>>,
    pub repeatable: bool,
    pub run_once: bool,
    pub permanent: bool,
    pub new_only: bool,
    pub owner_stack_limit: u16,
    pub subject_stack_limit: u16,
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Unsupported(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City attachment definitions: {self:?}")
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, String>;
pub struct Definitions {
    binding: WireBinding,
    buildings: Vec<(u32, Result<Vec<Attachment>>)>,
    sets: Vec<(Vec<u8>, Option<sets::SetDefinition>)>,
    requirements: Vec<sets::RequirementDefinition>,
    maximum_loyalty: Option<i32>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> std::result::Result<Self, Error> {
        let db = rules.gameplay();
        let mut definitions = Self {
            binding: state.binding().clone(),
            buildings: vec![],
            sets: vec![],
            requirements: vec![],
            maximum_loyalty: None,
        };
        let compile = || -> Result<_> {
            let maximum = table(db, "GlobalParameters")?
                .rows()
                .find(|r| text(*r, "Name").ok().flatten() == Some(b"LOYALTY_MAXIMUM".as_slice()))
                .map(|r| int(r, "Value"))
                .transpose()?;
            Ok(maximum)
        };
        definitions.maximum_loyalty = compile().map_err(Error::Unsupported)?;
        for building in table(db, "Buildings").map_err(Error::Unsupported)?.rows() {
            let name = required(building, "BuildingType").map_err(Error::Unsupported)?;
            let result = compile_building(db, name);
            let result = result.map(|(attachments, sets, requirements)| {
                for (id, definition) in sets {
                    if !definitions.sets.iter().any(|r| r.0 == id) {
                        definitions.sets.push((id, definition));
                    }
                }
                for definition in requirements {
                    if !definitions
                        .requirements
                        .iter()
                        .any(|r| r.id == definition.id)
                    {
                        definitions.requirements.push(definition);
                    }
                }
                attachments
            });
            definitions.buildings.push((linux_type_hash(name), result));
        }
        Ok(definitions)
    }
    pub fn validate(&self, state: &LoadedState) -> std::result::Result<(), Error> {
        if state.binding() == &self.binding {
            Ok(())
        } else {
            Err(Error::Binding)
        }
    }
    pub fn building(&self, hash: u32) -> std::result::Result<&[Attachment], Error> {
        match self.buildings.iter().find(|r| r.0 == hash) {
            Some((_, Ok(rows))) => Ok(rows),
            Some((_, Err(error))) => Err(Error::Unsupported(error.clone())),
            None => Err(Error::Unsupported("current Building definition".into())),
        }
    }
    pub fn set_definitions(&self) -> &[(Vec<u8>, Option<sets::SetDefinition>)] {
        &self.sets
    }
    pub fn requirement_definitions(&self) -> &[sets::RequirementDefinition] {
        &self.requirements
    }
    pub fn maximum_loyalty(&self) -> Option<i32> {
        self.maximum_loyalty
    }
}
fn table<'a>(db: &'a Database, name: &str) -> Result<&'a crate::Table> {
    db.table(name).ok_or_else(|| format!("missing {name}"))
}
fn text<'a>(row: Row<'a>, column: &str) -> Result<Option<&'a [u8]>> {
    match row.get(column).map_err(|e| e.to_string())? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(v)),
        _ => Err(format!("nontext {column}")),
    }
}
fn required<'a>(row: Row<'a>, column: &str) -> Result<&'a [u8]> {
    text(row, column)?.ok_or_else(|| format!("null {column}"))
}
fn int(row: Row<'_>, column: &str) -> Result<i32> {
    sql_integer(row.get(column).map_err(|e| e.to_string())?)
        .ok_or_else(|| format!("integer {column}"))
}
fn find<'a>(db: &'a Database, name: &str, column: &str, id: &[u8]) -> Result<Option<Row<'a>>> {
    Ok(table(db, name)?
        .rows()
        .find(|r| text(*r, column).ok().flatten() == Some(id)))
}
fn argument<'a>(db: &'a Database, id: &[u8], name: &[u8]) -> Result<Option<&'a [u8]>> {
    for row in table(db, "ModifierArguments")?.rows() {
        if text(row, "ModifierId")? == Some(id) && text(row, "Name")? == Some(name) {
            if text(row, "Type")? != Some(b"ARGTYPE_IDENTITY".as_slice()) {
                return Err("dynamic City effect argument".into());
            }
            return text(row, "Value");
        }
    }
    Ok(None)
}
fn whole_amount(value: Option<&[u8]>) -> Result<i32> {
    let Some(value) = value else { return Ok(0) };
    let value = std::str::from_utf8(value)
        .map_err(|e| e.to_string())?
        .parse::<i32>()
        .map_err(|_| "nonintegral City effect Amount".to_owned())?;
    // This admitted subset is exactly representable through native numeric Q8
    // parsing and AsInt. Other numeric expressions remain explicit.
    if !(-8_388_608..=8_388_607).contains(&value) {
        return Err("City effect Amount Q8 range".into());
    }
    Ok(value)
}
type Compiled = (
    Vec<Attachment>,
    Vec<(Vec<u8>, Option<sets::SetDefinition>)>,
    Vec<sets::RequirementDefinition>,
);
fn compile_building(db: &Database, building: &[u8]) -> Result<Compiled> {
    let mut ids = table(db, "BuildingModifiers")?
        .rows()
        .filter_map(|row| match text(row, "BuildingType") {
            Ok(Some(name)) if name == building => Some(required(row, "ModifierId").map(Vec::from)),
            _ => None,
        })
        .collect::<Result<Vec<_>>>()?;
    ids.sort(); // exact SQLite default BINARY ORDER BY ModifierId
    let mut attachments = vec![];
    let mut set_rows = vec![];
    let mut requirements = vec![];
    for id in ids {
        // DatabaseUtility omits a reference when no native definition exists.
        let Some(row) = find(db, "Modifiers", "ModifierId", &id)? else {
            continue;
        };
        let kind = required(row, "ModifierType")?;
        let dynamic = find(db, "DynamicModifiers", "ModifierType", kind)?
            .ok_or("current DynamicModifier definition")?;
        if required(dynamic, "CollectionType")? != b"COLLECTION_OWNER" {
            return Err("Building modifier non-owner collection".into());
        }
        let effect = match required(dynamic, "EffectType")? {
            b"EFFECT_ADJUST_CITY_IDENTITY_PER_TURN" => CityEffect::IdentityPerTurn {
                amount: whole_amount(argument(db, &id, b"Amount")?)?,
            },
            b"EFFECT_ADJUST_CITY_YIELD_CHANGE" => {
                let yields = match argument(db, &id, b"YieldType")? {
                    None => vec![],
                    Some(name) => {
                        if find(db, "Yields", "YieldType", name)?.is_none() {
                            return Err("current City effect YieldType".into());
                        }
                        vec![linux_type_hash(name)]
                    }
                };
                let amounts = argument(db, &id, b"Amount")?
                    .map(|value| whole_amount(Some(value)))
                    .transpose()?
                    .into_iter()
                    .collect();
                CityEffect::YieldChanges { yields, amounts }
            }
            _ => return Err("unsupported Building City effect".into()),
        };
        let owner = text(row, "OwnerRequirementSetId")?.map(Vec::from);
        let subject = text(row, "SubjectRequirementSetId")?.map(Vec::from);
        for set_id in owner.iter().chain(subject.iter()) {
            if set_rows
                .iter()
                .any(|r: &(Vec<u8>, Option<sets::SetDefinition>)| r.0 == *set_id)
            {
                continue;
            }
            let Some(set) = find(db, "RequirementSets", "RequirementSetId", set_id)? else {
                set_rows.push((set_id.clone(), None));
                continue;
            };
            if required(set, "RequirementSetType")? != b"REQUIREMENTSET_TEST_ALL" {
                return Err("unsupported City requirement set factory".into());
            }
            let children = table(db, "RequirementSetRequirements")?
                .rows()
                .filter_map(|child| match text(child, "RequirementSetId") {
                    Ok(Some(value)) if value == set_id => {
                        Some(required(child, "RequirementId").map(Vec::from))
                    }
                    _ => None,
                })
                .collect::<Result<Vec<_>>>()?;
            // The current supported set has one child, so query planner order
            // cannot change child initialization/callback order.
            if children.len() > 1 {
                return Err("multi-child City set native query order".into());
            }
            for child in &children {
                if requirements
                    .iter()
                    .any(|r: &sets::RequirementDefinition| r.id == *child)
                {
                    continue;
                }
                let requirement = find(db, "Requirements", "RequirementId", child)?
                    .ok_or("current City requirement definition")?;
                if int(requirement, "Triggered")? != 0 {
                    return Err("triggered City requirement".into());
                }
                requirements.push(sets::RequirementDefinition {
                    id: child.clone(),
                    predicate: sets::Predicate::Unsupported(linux_type_hash(required(
                        requirement,
                        "RequirementType",
                    )?)),
                    inverse: int(requirement, "Inverse")? != 0,
                    persistent: int(requirement, "Persistent")? != 0,
                });
            }
            set_rows.push((
                set_id.clone(),
                Some(sets::SetDefinition {
                    id: set_id.clone(),
                    requirements: children,
                }),
            ));
        }
        attachments.push(Attachment {
            id,
            effect,
            owner_requirements: owner,
            subject_requirements: subject,
            run_once: int(row, "RunOnce")? != 0,
            new_only: int(row, "NewOnly")? != 0,
            permanent: int(row, "Permanent")? != 0,
            repeatable: int(row, "Repeatable")? != 0,
            owner_stack_limit: int(row, "OwnerStackLimit")? as u16,
            subject_stack_limit: int(row, "SubjectStackLimit")? as u16,
        });
    }
    Ok((attachments, set_rows, requirements))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-city-attachment-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE BuildingModifiers(BuildingType TEXT,ModifierId TEXT); CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit); CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT); CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT); CREATE TABLE Yields(YieldType TEXT); CREATE TABLE RequirementSets(RequirementSetId TEXT,RequirementSetType TEXT); CREATE TABLE RequirementSetRequirements(RequirementSetId TEXT,RequirementId TEXT); CREATE TABLE Requirements(RequirementId TEXT,RequirementType TEXT,Inverse,Persistent,Triggered);
        INSERT INTO BuildingModifiers VALUES('B','Z_LOYALTY'),('B','A_CULTURE');
        INSERT INTO Modifiers VALUES('Z_LOYALTY','LOYALTY',1,1,1,1,NULL,NULL,NULL,65537),('A_CULTURE','CULTURE',0,0,0,0,NULL,'FULL',NULL,NULL);
        INSERT INTO DynamicModifiers VALUES('LOYALTY','COLLECTION_OWNER','EFFECT_ADJUST_CITY_IDENTITY_PER_TURN'),('CULTURE','COLLECTION_OWNER','EFFECT_ADJUST_CITY_YIELD_CHANGE');
        INSERT INTO ModifierArguments VALUES('Z_LOYALTY','Amount','ARGTYPE_IDENTITY','-1'),('A_CULTURE','Amount','ARGTYPE_IDENTITY','1'),('A_CULTURE','YieldType','ARGTYPE_IDENTITY','YIELD_CULTURE');
        INSERT INTO Yields VALUES('YIELD_CULTURE'); INSERT INTO RequirementSets VALUES('FULL','REQUIREMENTSET_TEST_ALL'); INSERT INTO RequirementSetRequirements VALUES('FULL','LOYAL'); INSERT INTO Requirements VALUES('LOYAL','REQUIREMENT_CITY_HAS_FULL_LOYALTY',1,1,0);").unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn original_modifier_id_sort_and_flags_are_independent_of_sql_row_order() {
        let db = database("");
        let (rows, sets, requirements) = compile_building(&db, b"B").unwrap();
        assert_eq!(rows[0].id, b"A_CULTURE");
        assert_eq!(rows[1].id, b"Z_LOYALTY");
        assert!(
            matches!(&rows[0].effect,CityEffect::YieldChanges{yields,amounts} if yields==&[linux_type_hash(b"YIELD_CULTURE")] && amounts==&[1])
        );
        assert!(matches!(
            rows[1].effect,
            CityEffect::IdentityPerTurn { amount: -1 }
        ));
        assert!(rows[1].run_once && rows[1].new_only && rows[1].permanent && rows[1].repeatable);
        assert_eq!(rows[1].owner_stack_limit, 0);
        assert_eq!(rows[1].subject_stack_limit, 1);
        assert_eq!(
            sets[0].1.as_ref().unwrap().requirements,
            [b"LOYAL".to_vec()]
        );
        assert!(requirements[0].inverse && requirements[0].persistent);
        assert_eq!(
            requirements[0].predicate,
            sets::Predicate::Unsupported(linux_type_hash(b"REQUIREMENT_CITY_HAS_FULL_LOYALTY"))
        );
    }
    #[test]
    fn triggered_and_dynamic_parameters_are_explicit_reached_errors() {
        assert!(
            compile_building(&database("UPDATE Requirements SET Triggered=1;"), b"B")
                .unwrap_err()
                .contains("triggered")
        );
        assert!(compile_building(
            &database("UPDATE ModifierArguments SET Type='ARGTYPE_PLAYER';"),
            b"B"
        )
        .unwrap_err()
        .contains("dynamic"));
        assert!(compile_building(
            &database("UPDATE ModifierArguments SET Value='1,2' WHERE Name='Amount';"),
            b"B"
        )
        .unwrap_err()
        .contains("nonintegral"));
    }
    #[test]
    fn absent_native_modifier_reference_is_skipped_and_missing_set_retained() {
        let (rows, sets, _) = compile_building(
            &database(
                "INSERT INTO BuildingModifiers VALUES('B','MISSING'); DELETE FROM RequirementSets;",
            ),
            b"B",
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(sets.len(), 1);
        assert!(sets[0].1.is_none());
    }
}
