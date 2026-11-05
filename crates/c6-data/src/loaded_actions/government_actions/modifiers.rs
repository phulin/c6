//! Government GME definitions. Live objects, requirements and effects belong to
//! the ongoing canonical ModifierOwners, including future City/building changes.
use crate::melee_attack::backend::{
    city_creation::definitions::{Attachment, Collection},
    CityEffect,
};
use crate::{registry::linux_type_hash, Database, Row, Rules, SqlValue};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    melee_attack::proposals::subject_sets as sets, movement_world::DefinitionLookup,
};
#[derive(Clone, Debug)]
pub enum Recipe {
    City(Attachment),
    FlatBonus {
        id: Vec<u8>,
        bonus: Option<u32>,
        amount: i32,
    },
}
#[derive(Debug)]
pub enum Error {
    Binding,
    Unsupported(String),
}
pub struct Government {
    pub recipes: Vec<Recipe>,
    pub sets: Vec<(Vec<u8>, Option<sets::SetDefinition>)>,
    pub requirements: Vec<sets::RequirementDefinition>,
    pub any_sets: Vec<Vec<u8>>,
    pub buildings: Vec<(Vec<u8>, Option<u32>)>,
}
pub struct Definitions {
    binding: WireBinding,
    governments: Vec<(u32, Result<Government, String>)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let yields =
            crate::unit_initialization_definitions::collection(rules, "Yields", "YieldType")
                .map_err(|e| Error::Unsupported(e.to_string()))?;
        let yields = (0..yields.entries().len())
            .map(|i| yields.type_hash(i).ok_or(Error::Binding))
            .collect::<Result<Vec<_>, _>>()?;
        let mut governments = vec![];
        for row in table(rules.gameplay(), "Governments")
            .map_err(Error::Unsupported)?
            .rows()
        {
            let name = text(row, "GovernmentType").map_err(Error::Unsupported)?;
            governments.push((
                linux_type_hash(name),
                compile(rules.gameplay(), name, &yields),
            ));
        }
        Ok(Self {
            binding: state.binding().clone(),
            governments,
        })
    }
    pub fn government(&self, state: &LoadedState, hash: u32) -> Result<&Government, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        match self.governments.iter().find(|r| r.0 == hash) {
            Some((_, Ok(r))) => Ok(r),
            Some((_, Err(e))) => Err(Error::Unsupported(e.clone())),
            None => Err(Error::Unsupported("current Government definition".into())),
        }
    }
}
fn table<'a>(db: &'a Database, name: &str) -> Result<&'a crate::Table, String> {
    db.table(name).ok_or_else(|| format!("missing {name}"))
}
fn text<'a>(r: Row<'a>, name: &str) -> Result<&'a [u8], String> {
    match r.get(name).map_err(|e| e.to_string())? {
        SqlValue::Text(v) => Ok(v),
        SqlValue::Null => Ok(b""),
        _ => Err(format!("nontext {name}")),
    }
}
fn int(r: Row<'_>, name: &str) -> Result<i32, String> {
    crate::game_parameters::sql_integer(r.get(name).map_err(|e| e.to_string())?)
        .ok_or_else(|| format!("integer {name}"))
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
fn argument<'a>(
    db: &'a Database,
    tab: &str,
    key: &str,
    id: &[u8],
    name: &[u8],
) -> Result<Option<&'a [u8]>, String> {
    for r in table(db, tab)?.rows() {
        if text(r, key)? == id && text(r, "Name")? == name {
            if text(r, "Type")? != b"ARGTYPE_IDENTITY" {
                return Err("dynamic Government argument".into());
            }
            return Ok(Some(text(r, "Value")?));
        }
    }
    Ok(None)
}
fn amount(db: &Database, id: &[u8]) -> Result<i32, String> {
    let Some(v) = argument(db, "ModifierArguments", "ModifierId", id, b"Amount")? else {
        return Ok(0);
    };
    let n = std::str::from_utf8(v)
        .map_err(|e| e.to_string())?
        .parse::<i32>()
        .map_err(|_| "Government Amount integral Q8".to_owned())?;
    if !(-8388608..=8388607).contains(&n) {
        return Err("Government Amount Q8 range".into());
    }
    Ok(n)
}
fn compile(db: &Database, government: &[u8], yields: &[u32]) -> Result<Government, String> {
    let mut result = Government {
        recipes: vec![],
        sets: vec![],
        requirements: vec![],
        any_sets: vec![],
        buildings: vec![],
    };
    let mut ids = vec![];
    for r in table(db, "GovernmentModifiers")?.rows() {
        if text(r, "GovernmentType")? == government {
            ids.push(text(r, "ModifierId")?.to_vec())
        }
    }
    ids.sort(); // GetGovernmentModifiers: ORDER BY ModifierId, BINARY.
    for id in ids {
        let Some(r) = find(db, "Modifiers", "ModifierId", &id)? else {
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
            if int(r, flag)? != 0 {
                return Err(format!("Government modifier {flag}"));
            }
        }
        if !text(r, "OwnerRequirementSetId")?.is_empty() {
            return Err("Government owner requirements".into());
        }
        let d = find(
            db,
            "DynamicModifiers",
            "ModifierType",
            text(r, "ModifierType")?,
        )?
        .ok_or("Government DynamicModifier")?;
        let n = amount(db, &id)?;
        match (text(d, "CollectionType")?, text(d, "EffectType")?) {
            (b"COLLECTION_PLAYER_CITIES", b"EFFECT_ADJUST_CITY_ALL_YIELDS_CHANGE") => {
                let subject = text(r, "SubjectRequirementSetId")?;
                if !subject.is_empty() {
                    compile_set(db, subject, &mut result)?
                }
                result.recipes.push(Recipe::City(Attachment {
                    collection: Collection::PlayerCities,
                    id,
                    effect: CityEffect::YieldChanges {
                        yields: yields.to_vec(),
                        amounts: vec![n; yields.len()],
                    },
                    subject_requirements: (!subject.is_empty()).then(|| subject.to_vec()),
                    owner_requirements: None,
                    repeatable: false,
                    run_once: false,
                    permanent: false,
                    new_only: false,
                    owner_stack_limit: 0,
                    subject_stack_limit: 0,
                }));
            }
            (b"COLLECTION_OWNER", b"EFFECT_ADJUST_FLAT_BONUS") => {
                if !text(r, "SubjectRequirementSetId")?.is_empty() {
                    return Err("FlatBonus subject requirements".into());
                }
                let bonus = argument(db, "ModifierArguments", "ModifierId", &id, b"BonusType")?
                    .filter(|v| !v.is_empty());
                let bonus = match bonus {
                    Some(v)
                        if find(db, "GovernmentBonusNames", "GovernmentBonusType", v)?
                            .is_some() =>
                    {
                        Some(linux_type_hash(v))
                    }
                    _ => None,
                };
                result.recipes.push(Recipe::FlatBonus {
                    id,
                    bonus,
                    amount: n,
                });
            }
            _ => return Err("unsupported Government collection/effect".into()),
        }
    }
    Ok(result)
}
fn compile_set(db: &Database, id: &[u8], out: &mut Government) -> Result<(), String> {
    if out.sets.iter().any(|r| r.0 == id) {
        return Ok(());
    }
    let Some(r) = find(db, "RequirementSets", "RequirementSetId", id)? else {
        out.sets.push((id.to_vec(), None));
        return Ok(());
    };
    match text(r, "RequirementSetType")? {
        b"REQUIREMENTSET_TEST_ALL" => {}
        b"REQUIREMENTSET_TEST_ANY" => out.any_sets.push(id.to_vec()),
        _ => return Err("Government requirement set kind".into()),
    }
    let mut children = vec![];
    for r in table(db, "RequirementSetRequirements")?.rows() {
        if text(r, "RequirementSetId")? == id {
            children.push(text(r, "RequirementId")?.to_vec())
        }
    }
    // The fixed native query uses the (RequirementSetId,RequirementId) PK
    // covering index in the configured Linux database.
    children.sort();
    for child in &children {
        if out.requirements.iter().any(|r| r.id == *child) {
            continue;
        }
        let r = find(db, "Requirements", "RequirementId", child)?
            .ok_or("Government requirement definition")?;
        if text(r, "RequirementType")? != b"REQUIREMENT_CITY_HAS_BUILDING"
            || int(r, "Triggered")? != 0
        {
            return Err("Government requirement factory".into());
        }
        let building = argument(
            db,
            "RequirementArguments",
            "RequirementId",
            child,
            b"BuildingType",
        )?
        .filter(|v| !v.is_empty());
        let building = match building {
            Some(v) if find(db, "Buildings", "BuildingType", v)?.is_some() => {
                Some(linux_type_hash(v))
            }
            _ => None,
        };
        out.buildings.push((child.clone(), building));
        out.requirements.push(sets::RequirementDefinition {
            id: child.clone(),
            predicate: sets::Predicate::Unsupported(linux_type_hash(
                b"REQUIREMENT_CITY_HAS_BUILDING",
            )),
            inverse: int(r, "Inverse")? != 0,
            persistent: int(r, "Persistent")? != 0,
        });
    }
    out.sets.push((
        id.to_vec(),
        Some(sets::SetDefinition {
            id: id.to_vec(),
            requirements: children,
        }),
    ));
    Ok(())
}

#[derive(Debug)]
pub enum EventError {
    Definitions(Error),
    Runtime(crate::melee_attack::backend::city_creation::Error),
    Missing(&'static str),
}
impl<P> crate::loaded_actions::stock_notifications::Runtime<'_, P> {
    /// Invoke at the exact OnGovernmentEnacted/Retracted signal position.
    /// Native definition indices remain the signal argument; wrappers use hash.
    #[allow(clippy::too_many_arguments)]
    pub fn government_modifier_event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        government: i32,
        hash: u32,
        added: bool,
        definitions: &Definitions,
        reports: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), EventError> {
        if government == -1 {
            return Ok(());
        }
        if government < -1 {
            return Err(EventError::Missing("native Government definition index"));
        }
        let definition = definitions
            .government(state, hash)
            .map_err(EventError::Definitions)?;
        if added && definition.recipes.is_empty() {
            return Ok(());
        }
        self.with_modifier_owners(|_, modifier| {
            if !added
                && !definition.recipes.is_empty()
                && !modifier.effects.objects().iter().any(|(_, o)| {
                    o.kind
                        == crate::melee_attack::backend::ContextObject::Government {
                            owner,
                            government_hash: hash,
                        }
                })
            {
                return Err(EventError::Missing(
                    "current imported Government modifier ownership",
                ));
            }
            modifier
                .effects
                .install_government_definitions(state, &mut modifier.requirements, definition)
                .map_err(EventError::Runtime)?;
            let context = modifier
                .effects
                .make_government_object(state, owner, hash)
                .map_err(EventError::Runtime)?;
            if added {
                for recipe in &definition.recipes {
                    modifier
                        .effects
                        .attach_government_recipe(
                            state,
                            &mut modifier.requirements,
                            context,
                            recipe,
                            reports,
                        )
                        .map_err(EventError::Runtime)?;
                }
            } else {
                modifier
                    .effects
                    .remove_government_modifiers(
                        state,
                        &mut modifier.requirements,
                        context,
                        reports,
                    )
                    .map_err(EventError::Runtime)?;
                modifier
                    .effects
                    .retire_policy_object(state, context)
                    .map_err(EventError::Runtime)?;
            }
            Ok(())
        })
        .map_err(|_| EventError::Missing("canonical modifier owner loan"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database(extra: &str) -> Database {
        let path = std::env::temp_dir().join(format!(
            "c6-government-modifiers-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE GovernmentModifiers(GovernmentType TEXT,ModifierId TEXT); CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit); CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT); CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT); CREATE TABLE GovernmentBonusNames(GovernmentBonusType TEXT); CREATE TABLE RequirementSets(RequirementSetId TEXT,RequirementSetType TEXT); CREATE TABLE RequirementSetRequirements(RequirementSetId TEXT,RequirementId TEXT,PRIMARY KEY(RequirementSetId,RequirementId)); CREATE TABLE Requirements(RequirementId TEXT,RequirementType TEXT,Triggered,Inverse,Persistent); CREATE TABLE RequirementArguments(RequirementId TEXT,Name TEXT,Type TEXT,Value TEXT); CREATE TABLE Buildings(BuildingType TEXT);
        INSERT INTO GovernmentModifiers VALUES('G','Z_BONUS'),('G','A_CITY');
        INSERT INTO Modifiers VALUES('Z_BONUS','P',0,0,0,0,NULL,NULL,NULL,NULL),('A_CITY','C',0,0,0,0,NULL,'ANY',NULL,NULL);
        INSERT INTO DynamicModifiers VALUES('P','COLLECTION_OWNER','EFFECT_ADJUST_FLAT_BONUS'),('C','COLLECTION_PLAYER_CITIES','EFFECT_ADJUST_CITY_ALL_YIELDS_CHANGE');
        INSERT INTO ModifierArguments VALUES('A_CITY','Amount','ARGTYPE_IDENTITY','2'),('Z_BONUS','Amount','ARGTYPE_IDENTITY','10'),('Z_BONUS','BonusType','ARGTYPE_IDENTITY','B'); INSERT INTO GovernmentBonusNames VALUES('B');
        INSERT INTO RequirementSets VALUES('ANY','REQUIREMENTSET_TEST_ANY'); INSERT INTO RequirementSetRequirements VALUES('ANY','Z'),('ANY','A');
        INSERT INTO Requirements VALUES('Z','REQUIREMENT_CITY_HAS_BUILDING',0,0,1),('A','REQUIREMENT_CITY_HAS_BUILDING',0,1,0);
        INSERT INTO Buildings VALUES('BUILDING_X'); INSERT INTO RequirementArguments VALUES('Z','BuildingType','ARGTYPE_IDENTITY','BUILDING_X'),('A','BuildingType','ARGTYPE_IDENTITY','MISSING');").unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn rules_not_government_names_drive_effects_and_live_any_children() {
        let d = compile(&database(""), b"G", &[12, 7, 90]).unwrap();
        assert!(
            matches!(&d.recipes[0],Recipe::City(a) if a.id==b"A_CITY"&&matches!(&a.effect,CityEffect::YieldChanges{yields,amounts} if yields==&[12,7,90]&&amounts==&[2,2,2]))
        );
        assert!(
            matches!(&d.recipes[1],Recipe::FlatBonus{bonus:Some(h),amount:10,..} if *h==linux_type_hash(b"B"))
        );
        assert_eq!(
            d.sets[0].1.as_ref().unwrap().requirements,
            [b"A".to_vec(), b"Z".to_vec()]
        );
        assert_eq!(d.any_sets, [b"ANY".to_vec()]);
        assert_eq!(
            d.buildings,
            [
                (b"A".to_vec(), None),
                (b"Z".to_vec(), Some(linux_type_hash(b"BUILDING_X")))
            ]
        );
        assert!(d.requirements[0].inverse && d.requirements[1].persistent);
        assert!(compile(&database(""), b"EMPTY", &[])
            .unwrap()
            .recipes
            .is_empty());
    }
    #[test]
    fn unsupported_reached_factories_and_dynamic_arguments_remain_explicit() {
        assert!(compile(&database("UPDATE Modifiers SET Permanent=1;"), b"G", &[]).is_err());
        assert!(compile(&database("UPDATE Requirements SET Triggered=1;"), b"G", &[]).is_err());
        assert!(compile(
            &database("UPDATE RequirementArguments SET Type='ARGTYPE_PLAYER';"),
            b"G",
            &[]
        )
        .is_err());
        assert!(compile(
            &database("UPDATE DynamicModifiers SET EffectType='UNKNOWN';"),
            b"G",
            &[]
        )
        .is_err());
        let d = compile(&database("DELETE FROM GovernmentBonusNames;"), b"G", &[]).unwrap();
        assert!(matches!(
            d.recipes[1],
            Recipe::FlatBonus { bonus: None, .. }
        ));
    }
}
