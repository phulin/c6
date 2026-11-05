//! UnitAbilityModifiers attachment and the concrete home-continent requirement.
use super::*;
mod unit_combat;
use crate::{game_parameters::sql_integer, registry::linux_type_hash, Row, Rules, SqlValue};
#[derive(Clone)]
pub(super) struct Recipe {
    id: Vec<u8>,
    effect: RecipeEffect,
    subject: Vec<u8>,
    amount: i32,
    kind: i32,
    permanent: bool,
    repeatable: bool,
    stack: u16,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecipeEffect {
    OwnerStrength,
    UnitCombatStrength,
    SeeHidden,
}
pub(super) struct Ability {
    index: usize,
    hash: u32,
    recipes: Result<Vec<Recipe>, Error>,
}
#[derive(Clone, Copy)]
pub(super) struct Home {
    pub label: u64,
    pub context: u64,
    pub subject: u64,
}
fn text<'a>(r: Row<'a>, k: &str) -> Result<&'a [u8], Error> {
    match r
        .get(k)
        .map_err(|_| Error::Missing("ability rule column"))?
    {
        SqlValue::Text(b) => Ok(b),
        SqlValue::Null => Ok(b""),
        _ => Err(Error::Missing("ability rule text")),
    }
}
fn integer(r: Row<'_>, k: &str) -> Result<i32, Error> {
    sql_integer(
        r.get(k)
            .map_err(|_| Error::Missing("ability rule column"))?,
    )
    .ok_or(Error::Missing("ability rule integer"))
}
fn compile(db: &crate::Database, name: &[u8]) -> Result<Vec<Recipe>, Error> {
    let table = |name: &str| {
        db.table(name)
            .ok_or(Error::Missing("ability modifier rules table"))
    };
    let mut ids = Vec::new();
    for r in table("UnitAbilityModifiers")?.rows() {
        if text(r, "UnitAbilityType")? == name {
            ids.push(text(r, "ModifierId")?.to_vec());
        }
    }
    ids.sort();
    let mut out = vec![];
    for id in ids {
        let Some(row) = table("Modifiers")?
            .rows()
            .find(|r| text(*r, "ModifierId").ok() == Some(id.as_slice()))
        else {
            continue;
        };
        let dynamic_name = text(row, "ModifierType")?;
        let dynamic = table("DynamicModifiers")?
            .rows()
            .find(|r| text(*r, "ModifierType").ok() == Some(dynamic_name))
            .ok_or(Error::Missing("ability DynamicModifier definition"))?;
        let effect = match (
            text(dynamic, "CollectionType")?,
            text(dynamic, "EffectType")?,
        ) {
            (b"COLLECTION_OWNER", b"EFFECT_ADJUST_UNIT_COMBAT_STRENGTH") => {
                RecipeEffect::OwnerStrength
            }
            (b"COLLECTION_OWNER", b"EFFECT_ADJUST_UNIT_SEE_HIDDEN") => RecipeEffect::SeeHidden,
            (b"COLLECTION_UNIT_COMBAT", b"EFFECT_ADJUST_PLAYER_STRENGTH_MODIFIER") => {
                RecipeEffect::UnitCombatStrength
            }
            _ => return Err(Error::Missing("unsupported ability modifier definition")),
        };
        if !text(row, "OwnerRequirementSetId")?.is_empty()
            || integer(row, "RunOnce")? != 0
            || integer(row, "NewOnly")? != 0
            || integer(row, "OwnerStackLimit")? != 0
        {
            return Err(Error::Missing("unsupported ability modifier definition"));
        }
        let mut amount = 0;
        let mut kind = 1;
        for arg in table("ModifierArguments")?.rows() {
            if text(arg, "ModifierId")? != id {
                continue;
            }
            if text(arg, "Type")? != b"ARGTYPE_IDENTITY" {
                return Err(Error::Missing("dynamic ability modifier argument"));
            }
            match text(arg, "Name")? {
                b"Amount" => {
                    amount = std::str::from_utf8(text(arg, "Value")?)
                        .map_err(|_| Error::Missing("ability Amount UTF8"))?
                        .parse::<i32>()
                        .map_err(|_| Error::Missing("ability Amount number"))?;
                    if !(-8_388_608..=8_388_607).contains(&amount) {
                        return Err(Error::Missing("ability Amount Q8 range"));
                    }
                }
                b"Type" if effect == RecipeEffect::OwnerStrength => {
                    kind = match text(arg, "Value")? {
                        b"RANGED" => 2,
                        b"BOMBARD" => 3,
                        b"ANTIAIR" => 4,
                        _ => 1,
                    }
                }
                _ if effect == RecipeEffect::UnitCombatStrength => {
                    return Err(Error::Missing(
                        "unsupported ability player strength argument",
                    ));
                }
                _ => {}
            }
        }
        out.push(Recipe {
            id,
            effect,
            subject: text(row, "SubjectRequirementSetId")?.to_vec(),
            amount,
            kind,
            permanent: integer(row, "Permanent")? != 0,
            repeatable: integer(row, "Permanent")? != 0 && integer(row, "Repeatable")? != 0,
            stack: integer(row, "SubjectStackLimit")? as u16,
        });
    }
    Ok(out)
}
impl Runtime {
    /// Compile immutable arguments once. Unsupported ability families stay lazy;
    /// their error is returned only when that ability is actually earned.
    pub fn install_abilities(
        &mut self,
        rules: &Rules,
        definitions: &super::super::Definitions,
    ) -> Result<(), Error> {
        if !self.abilities.is_empty() {
            return Err(Error::Missing("ability definitions already installed"));
        }
        for (index, entry) in definitions
            .initialization
            .abilities()
            .entries()
            .iter()
            .enumerate()
        {
            self.abilities.push(Ability {
                index,
                hash: linux_type_hash(&entry.type_name),
                recipes: compile(rules.gameplay(), &entry.type_name),
            });
        }
        Ok(())
    }
    /// Triggered is separate from Persistent and must come from the admitted
    /// definition, not the current requirement's evaluated state.
    pub fn install_requirement_flags(&mut self, flags: Vec<(Vec<u8>, bool)>) -> Result<(), Error> {
        if flags
            .iter()
            .enumerate()
            .any(|(i, r)| flags[..i].iter().any(|p| p.0 == r.0))
        {
            return Err(Error::Missing("duplicate requirement flags"));
        }
        self.requirement_flags = flags;
        Ok(())
    }
}
/// OnUnitAbilityEarned23e4180: no recipes means no object allocation. A fresh
/// ability wrapper and normal wrapper remain distinct even with equal Unit CIDs.
pub fn on_ability_earned<E: Effects>(
    owners: &mut Owners<'_>,
    runtime: &mut Runtime,
    effects: &mut E,
    definitions: &super::super::Definitions,
    unit: UnitReference,
    index: usize,
) -> Result<(), Error> {
    let ability = runtime
        .abilities
        .iter()
        .find(|a| a.index == index)
        .ok_or(Error::Missing("ability attachment definitions"))?;
    let hash = ability.hash;
    let recipes = match &ability.recipes {
        Ok(rows) => rows.clone(),
        Err(_) => return Err(Error::Missing("unsupported earned ability recipe")),
    };
    if recipes.is_empty() {
        return Ok(());
    }
    runtime.log_disabled(2)?;
    let context = owners
        .effects
        .make_current_unit_object(owners.state, unit, [1, hash as i32])
        .map_err(Error::Query)?;
    for recipe in recipes {
        let instance = owners
            .effects
            .allocate_modifier_id(owners.state)
            .map_err(Error::Query)?;
        if recipe.effect == RecipeEffect::UnitCombatStrength {
            unit_combat::attach(owners, context, unit, instance, &recipe)?;
            continue;
        }
        let receiver = (1u64 << 59) | u64::from(instance as u32);
        let effect = (1u64 << 58) | u64::from(instance as u32);
        owners
            .effects
            .register_unit_effect(
                owners.state,
                effect,
                match recipe.effect {
                    RecipeEffect::OwnerStrength => {
                        crate::unit_kill::effects::Effect::CombatStrength {
                            amount: recipe.amount,
                            kind: recipe.kind,
                        }
                    }
                    RecipeEffect::SeeHidden => crate::unit_kill::effects::Effect::SeeHidden,
                    RecipeEffect::UnitCombatStrength => unreachable!(),
                },
            )
            .map_err(Error::Query)?;
        owners
            .effects
            .register_unit_modifier(
                owners.state,
                records::Modifier {
                    receiver,
                    context,
                    instance,
                    control: common::Modifier {
                        enabled: true,
                        suppressed: false,
                        owner_requirement_set: None,
                    },
                    permanent: recipe.permanent,
                    definition: recipe.id,
                    stack_limit: recipe.stack,
                    effect,
                    tracked: vec![],
                    applied: vec![],
                },
            )
            .map_err(Error::Query)?;
        let object = owners
            .effects
            .object_mut(context)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_mut()
            .ok_or(Error::Missing("ability owner lifecycle"))?;
        object.owned_modifiers.push(instance);
        object
            .destroyed
            .subscribe(Callback::OwnerDestroyed(receiver))
            .map_err(Error::Ring)?;
        runtime.definitions.push(Definition {
            receiver,
            subject_requirements: recipe.subject,
            repeatable: recipe.repeatable,
        });
        // COLLECTION_OWNER.GetItems2326fc0 yields the context object itself.
        addition::item_changed(
            &mut Apply {
                owners,
                runtime,
                effects,
                definitions,
                receiver,
                subject: context,
            },
            true,
        )?;
    }
    Ok(())
}
fn test(
    owners: &Owners<'_>,
    definitions: &super::super::Definitions,
    home: Home,
) -> Result<i32, Error> {
    let subject = match owners
        .effects
        .object(home.subject)
        .map_err(Error::Query)?
        .kind
    {
        ContextObject::Unit(u) => u,
        _ => return Ok(3),
    };
    let context = match owners
        .effects
        .object(home.context)
        .map_err(Error::Query)?
        .kind
    {
        ContextObject::Unit(u) => u,
        _ => return Ok(1),
    };
    let unit = super::super::actor::<Error>(owners.state, subject.player(), subject.id)
        .map_err(|_| Error::Missing("home subject Unit"))?;
    let Some(plot) = owners
        .state
        .world()
        .map()
        .plot_at(unit.k_location.x, unit.k_location.y)
    else {
        return Ok(1);
    };
    let context = super::super::actor::<Error>(owners.state, context.player(), context.id)
        .map_err(|_| Error::Missing("home context Unit"))?;
    let player = owners
        .state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == context.e_owner)
        .ok_or(Error::Missing("home context Player"))?;
    let lookup = &definitions.continents;
    let from = lookup
        .find_index(player.cities.home_continent_hash)
        .ok_or(Error::Missing("current capital continent"))?;
    let to = lookup
        .find_index(plot.continent_hash)
        .ok_or(Error::Missing("current subject continent"))?;
    Ok(if from == to { 0 } else { 1 })
}
struct Initializer<'a, 's, E> {
    owners: &'a mut Owners<'s>,
    runtime: &'a mut Runtime,
    effects: &'a mut E,
    definitions: &'a super::super::Definitions,
    context: u64,
}
impl<E: Effects> sets::SubjectContext for Initializer<'_, '_, E> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.owners.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.owners.requirements.sets_mut())
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &sets::RequirementDefinition,
        _: i32,
        subject: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Error>> {
        if definition.predicate
            != sets::Predicate::Unsupported(linux_type_hash(b"REQUIREMENT_UNIT_ON_HOME_CONTINENT"))
        {
            return Err(sets::Error::Context(Error::Missing(
                "unsupported Unit ability requirement",
            )));
        }
        let sets::Subject::ModifierObject(subject) = subject else {
            return Err(sets::Error::UnsupportedSubject);
        };
        let home = Home {
            label,
            context: self.context,
            subject,
        };
        // Caller appends to its existing UnitMoved dispatcher before the initial Test.
        self.effects
            .subscribe_home_movement(self.owners, label)
            .map_err(sets::Error::Context)?;
        self.runtime.homes.push(home);
        let tested = test(self.owners, self.definitions, home).map_err(sets::Error::Context)?;
        let triggered = self
            .runtime
            .requirement_flags
            .iter()
            .find(|r| r.0 == definition.id)
            .map(|r| r.1)
            .ok_or(sets::Error::Context(Error::Missing(
                "home requirement Triggered flag",
            )))?;
        let state = if definition.inverse {
            sets::inverse(tested)
        } else {
            tested
        };
        Ok(sets::InitializedRequirement::Simple(
            c6_core::simple_requirement::Requirement {
                active: true,
                state: if triggered && state == 0 { 4 } else { state },
                inverse: definition.inverse,
                persistent: definition.persistent,
                triggered,
            },
        ))
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Error> {
        self.runtime.log_disabled(1)
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), Error> {
        self.runtime.log_disabled(4)
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), Error> {
        self.runtime.log_disabled(4)
    }
}
pub(super) fn create<E: Effects>(apply: &mut Apply<'_, '_, E>) -> Result<Option<u64>, Error> {
    let id = apply
        .runtime
        .definition(apply.receiver)?
        .subject_requirements
        .clone();
    let context = apply.record()?.context;
    let instance = apply
        .owners
        .effects
        .object(context)
        .map_err(Error::Query)?
        .instance_id;
    sets::get_or_create_subject(
        &mut Initializer {
            owners: apply.owners,
            runtime: apply.runtime,
            effects: apply.effects,
            definitions: apply.definitions,
            context,
        },
        apply.receiver,
        instance,
        sets::Subject::ModifierObject(apply.subject),
        &id,
    )
    .map_err(|e| Error::Factory(Box::new(e)))
}
struct Changed<'a, 's, E> {
    owners: &'a mut Owners<'s>,
    runtime: &'a mut Runtime,
    effects: &'a mut E,
    definitions: &'a super::super::Definitions,
}
impl<E: Effects> sets::changes::Context for Changed<'_, '_, E> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.owners.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.owners.requirements.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Error>> {
        let home = *self
            .runtime
            .homes
            .iter()
            .find(|h| h.label == label)
            .ok_or(sets::Error::MissingSet)?;
        test(self.owners, self.definitions, home).map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        listener: sets::SetListener,
        set: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Error>> {
        let sets::SetListener::Subject(receiver) = listener else {
            return Err(sets::Error::Context(Error::Missing(
                "home external set listener",
            )));
        };
        let subject = self
            .owners
            .effects
            .unit_modifier(receiver)
            .map_err(|e| sets::Error::Context(Error::Query(e)))?
            .tracked
            .iter()
            .find(|t| t.requirement_set == Some(set))
            .map(|t| t.object)
            .ok_or(sets::Error::MissingSet)?;
        common::check_apply_effects(&mut Apply {
            owners: self.owners,
            runtime: self.runtime,
            effects: self.effects,
            definitions: self.definitions,
            receiver,
            subject,
        })
        .map(|_| ())
        .map_err(sets::Error::Context)
    }
    fn destroy_requirement(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "home requirement destruction",
        )))
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing("home set destruction")))
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Error>> {
        sets::changes::reset_trigger_state(self.owners.requirements.sets_mut(), set)
    }
}
/// Called at the newly appended UnitMoved slot, after earlier stock listeners.
pub fn on_home_moved<E: Effects>(
    owners: &mut Owners<'_>,
    runtime: &mut Runtime,
    effects: &mut E,
    definitions: &super::super::Definitions,
    label: u64,
    moved: UnitReference,
) -> Result<(), Error> {
    let home = *runtime
        .homes
        .iter()
        .find(|h| h.label == label)
        .ok_or(Error::Missing("home movement binding"))?;
    let ContextObject::Unit(subject) = owners
        .effects
        .object(home.subject)
        .map_err(Error::Query)?
        .kind
    else {
        return Ok(());
    };
    if subject.component_id() != moved.component_id() {
        return Ok(());
    }
    sets::changes::invalidate(
        &mut Changed {
            owners,
            runtime,
            effects,
            definitions,
        },
        label,
    )
    .map_err(|e| Error::Factory(Box::new(e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database(extra: &str) -> crate::Database {
        let path = std::env::temp_dir().join(format!(
            "c6-unit-ability-{}-{:?}.sqlite",
            std::process::id(),
            std::thread::current().id()
        ));
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE UnitAbilityModifiers(UnitAbilityType TEXT,ModifierId TEXT); CREATE TABLE Modifiers(ModifierId TEXT,ModifierType TEXT,RunOnce,NewOnly,Permanent,Repeatable,OwnerRequirementSetId TEXT,SubjectRequirementSetId TEXT,OwnerStackLimit,SubjectStackLimit); CREATE TABLE DynamicModifiers(ModifierType TEXT,CollectionType TEXT,EffectType TEXT); CREATE TABLE ModifierArguments(ModifierId TEXT,Name TEXT,Type TEXT,Value TEXT); INSERT INTO UnitAbilityModifiers VALUES('A','Z'),('A','B'); INSERT INTO Modifiers VALUES('Z','S',0,0,1,1,NULL,'HOME',0,65537),('B','S',0,0,0,1,NULL,NULL,0,0); INSERT INTO DynamicModifiers VALUES('S','COLLECTION_OWNER','EFFECT_ADJUST_UNIT_COMBAT_STRENGTH'); INSERT INTO ModifierArguments VALUES('Z','Amount','ARGTYPE_IDENTITY','5');").unwrap();
        c.execute_batch(extra).unwrap();
        drop(c);
        let db = crate::inspect(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        db
    }
    #[test]
    fn owner_recipes_sort_ids_preserve_flags_and_wrap_native_limit() {
        let db = database("");
        let recipes = compile(&db, b"A").unwrap();
        assert_eq!(
            recipes.iter().map(|r| r.id.as_slice()).collect::<Vec<_>>(),
            [b"B".as_slice(), b"Z".as_slice()]
        );
        assert!(!recipes[0].repeatable);
        assert!(recipes[1].repeatable && recipes[1].permanent);
        assert_eq!(
            (recipes[1].amount, recipes[1].kind, recipes[1].stack),
            (5, 1, 1)
        );
        assert!(compile(&db, b"NO_REFERENCES").unwrap().is_empty());
    }
    #[test]
    fn unit_combat_recipe_retains_deferred_opponent_requirements() {
        let db = database("UPDATE DynamicModifiers SET CollectionType='COLLECTION_UNIT_COMBAT',EffectType='EFFECT_ADJUST_PLAYER_STRENGTH_MODIFIER'; UPDATE Modifiers SET SubjectRequirementSetId='ANTI_CAVALRY_OPPONENT_REQUIREMENTS'; UPDATE ModifierArguments SET Value='10';");
        let recipes = compile(&db, b"A").unwrap();
        assert_eq!(recipes[1].effect, RecipeEffect::UnitCombatStrength);
        assert_eq!(recipes[1].amount, 10);
        assert_eq!(recipes[1].subject, b"ANTI_CAVALRY_OPPONENT_REQUIREMENTS");
    }
    #[test]
    fn new_player_strength_scaling_arguments_remain_explicitly_unsupported() {
        let db = database("UPDATE DynamicModifiers SET CollectionType='COLLECTION_UNIT_COMBAT',EffectType='EFFECT_ADJUST_PLAYER_STRENGTH_MODIFIER'; INSERT INTO ModifierArguments VALUES('Z','StackScale','ARGTYPE_IDENTITY','50');");
        assert!(compile(&db, b"A").is_err());
    }
    #[test]
    fn non_owner_collection_is_not_silently_replaced_by_owner() {
        let db = database("UPDATE DynamicModifiers SET CollectionType='COLLECTION_PLAYER_UNITS';");
        assert!(compile(&db, b"A").is_err());
    }
}
