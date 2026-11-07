//! Explicit player-selected Linux XP2 promotions. This is a bounded action,
//! not a complete gameplay/turn implementation. Unknown callbacks stop explicitly.
pub mod stock;
use crate::{
    registry::linux_type_hash as hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    unit::{ComponentId, HashValue, Unit},
};

#[derive(Debug)]
pub enum Error<E> {
    Binding,
    Missing(&'static str),
    Rules(crate::Error),
    State(c6_core::loaded_state::Error),
    Unsupported(&'static str),
    Query(E),
    /// A synchronous effect failed after execution began; retain the prefix.
    Effect(E),
    AfterMutation(Box<Error<E>>),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    CommandUnavailable,
    CannotMove,
    NotReady,
    PendingOperations,
    Ineligible,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected(Rejection),
    Promoted { unit: ComponentId, promotion: u32 },
}
struct Promotion {
    hash: u32,
    class: Option<u32>,
    prerequisites: Vec<u32>,
}
struct UnitDefinition {
    hash: u32,
    class: Option<u32>,
    random_choices: i32,
}
pub struct Definitions {
    inputs: InputIdentities,
    promotions: Vec<Promotion>,
    units: Vec<UnitDefinition>,
    heal: i32,
}
fn reference(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error<()>> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(hash(v))),
        _ => Err(Error::Missing(column)),
    }
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, Error<()>> {
        if resources.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(|_| Error::Missing("bound promotion rules resources"))?;
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Missing("Linux promotion rules"));
        }
        let registry = collection(rules, "UnitPromotions", "UnitPromotionType")
            .map_err(|_| Error::Missing("UnitPromotions registry"))?;
        let table = rules
            .gameplay()
            .table("UnitPromotions")
            .ok_or(Error::Missing("UnitPromotions"))?;
        let edges = rules
            .gameplay()
            .table("UnitPromotionPrereqs")
            .ok_or(Error::Missing("UnitPromotionPrereqs"))?;
        let mut promotions = Vec::new();
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("promotion row"))?;
            let promotion = hash(&entry.type_name);
            let mut prerequisites = Vec::new();
            for edge in edges.rows() {
                if reference(edge, "UnitPromotion")? == Some(promotion) {
                    prerequisites.push(
                        reference(edge, "PrereqUnitPromotion")?
                            .ok_or(Error::Missing("prerequisite promotion"))?,
                    );
                }
            }
            promotions.push(Promotion {
                hash: promotion,
                class: reference(row, "PromotionClass")?,
                prerequisites,
            });
        }
        let registry =
            collection(rules, "Units", "UnitType").map_err(|_| Error::Missing("Units registry"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mut units = Vec::new();
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Unit row"))?;
            units.push(UnitDefinition {
                hash: hash(&entry.type_name),
                class: reference(row, "PromotionClass")?,
                random_choices: row.integer("NumRandomChoices").map_err(Error::Rules)? as i32,
            });
        }
        let heal = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?
            .rows()
            .find(|r| r.text("Name").ok() == Some("EXPERIENCE_PROMOTE_HEALED"))
            .and_then(|r| {
                r.get("Value")
                    .ok()
                    .and_then(crate::game_parameters::sql_integer)
            })
            .ok_or(Error::Missing("EXPERIENCE_PROMOTE_HEALED"))?;
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            promotions,
            units,
            heal,
        })
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.inputs != state.binding().inputs {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
}
fn actor<E>(state: &LoadedState, id: (i32, u32)) -> Result<&Unit, Error<E>> {
    state
        .world()
        .player(id.0)
        .map_err(|_| Error::Missing("promotion Player"))?
        .unit(id.1)
        .map_err(|_| Error::Missing("promotion Unit slot"))?
        .map(|v| v.record)
        .ok_or(Error::Missing("promotion Unit"))
}
fn has(unit: &Unit, promotion: u32) -> bool {
    unit.k_experience
        .ab_promotions_earned
        .iter()
        .rev()
        .find(|v| v.definition_hash == promotion)
        .is_some_and(|v| v.value)
}
/// Experience::CanEarnPromotion, with the random-choice continuation explicit.
/// Prerequisites are OR, not AND. Availability itself reads the native ready flag.
pub fn can_earn(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    promotion: u32,
) -> Result<bool, Error<()>> {
    defs.check(state)?;
    let u = actor(state, id)?;
    let Some(p) = defs.promotions.iter().find(|p| p.hash == promotion) else {
        return Ok(false);
    };
    if has(u, promotion) {
        return Ok(false);
    }
    let unit = defs
        .units
        .iter()
        .find(|v| v.hash == u.e_type_index)
        .ok_or(Error::Missing("Unit definition"))?;
    if unit.random_choices > 0 {
        return Err(Error::Unsupported("random proposed promotion selection"));
    }
    Ok(unit.class == p.class
        && (p.prerequisites.is_empty() || p.prerequisites.iter().any(|v| has(u, *v))))
}
/// Visible-only CanStart results enumerate all current promotions in definition
/// order, independently of phase/turn/pause guards. This is also used by the
/// experience-changed report translator; it never selects a player policy.
pub fn earnable_choices(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
) -> Result<Vec<u32>, Error<()>> {
    defs.check(state)?;
    let mut choices = Vec::new();
    for promotion in &defs.promotions {
        if can_earn(state, defs, id, promotion.hash)? {
            choices.push(promotion.hash);
        }
    }
    Ok(choices)
}

/// Mandatory synchronous effects use the same live world and runtime owners.
pub trait Effects {
    type Error;
    fn common_guards(&self, state: &LoadedState, id: (i32, u32)) -> Result<bool, Self::Error>;
    fn preflight(
        &self,
        state: &LoadedState,
        id: (i32, u32),
        promotion: u32,
    ) -> Result<(), Self::Error>;
    fn started(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        index: usize,
    ) -> Result<(), Self::Error>;
    fn earned(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        promotion: u32,
        index: usize,
    ) -> Result<(), Self::Error>;
    fn is_combat(&self, state: &LoadedState, id: (i32, u32)) -> Result<bool, Self::Error>;
    fn change_damage(
        &mut self,
        state: &mut LoadedState,
        id: (i32, u32),
        delta: i32,
    ) -> Result<(), Self::Error>;
    fn remove_notification(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), Self::Error>;
    fn level_changed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        level: i32,
    ) -> Result<(), Self::Error>;
    fn completed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        promotion: u32,
    ) -> Result<(), Self::Error>;
    fn finish_moves(&mut self, state: &mut LoadedState, id: (i32, u32)) -> Result<(), Self::Error>;
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), Self::Error>;
}
fn query<E>(e: Error<()>) -> Error<E> {
    match e {
        Error::Binding => Error::Binding,
        Error::Missing(v) => Error::Missing(v),
        Error::Rules(v) => Error::Rules(v),
        Error::State(v) => Error::State(v),
        Error::Unsupported(v) => Error::Unsupported(v),
        _ => unreachable!(),
    }
}
pub fn availability<E: Effects>(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    promotion: u32,
    effects: &E,
) -> Result<Option<Rejection>, Error<E::Error>> {
    defs.check(state)?;
    if !effects.common_guards(state, id).map_err(Error::Query)? {
        return Ok(Some(Rejection::CommandUnavailable));
    }
    let u = actor(state, id)?;
    if !c6_core::skip_turn::legality::can_move(u) {
        return Ok(Some(Rejection::CannotMove));
    }
    if !u.k_experience.b_can_promote {
        return Ok(Some(Rejection::NotReady));
    }
    if !u.k_operations.nodes.is_empty() {
        return Ok(Some(Rejection::PendingOperations));
    }
    if !can_earn(state, defs, id, promotion).map_err(query)? {
        return Ok(Some(Rejection::Ineligible));
    }
    Ok(None)
}
/// Legal ordinary promotion choices in native definition order. A positive
/// result is a semantic external-policy decision; no native AI is invoked.
pub fn choices<E: Effects>(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    effects: &E,
) -> Result<Vec<u32>, Error<E::Error>> {
    defs.check(state)?;
    let mut choices = Vec::new();
    for promotion in &defs.promotions {
        if availability(state, defs, id, promotion.hash, effects)?.is_none() {
            choices.push(promotion.hash)
        }
    }
    Ok(choices)
}
pub fn promote<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    promotion: u32,
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    if let Some(r) = availability(state, defs, id, promotion, effects)? {
        return Ok(Outcome::Rejected(r));
    }
    effects
        .preflight(state, id, promotion)
        .map_err(Error::Query)?;
    let index = defs
        .promotions
        .iter()
        .position(|v| v.hash == promotion)
        .ok_or(Error::Missing("promotion definition"))?;
    let cid = ComponentId {
        owner: id.0 as i16,
        component_type: 1,
        id: id.1,
    };
    effects.started(state, cid, index).map_err(Error::Effect)?;
    let result = (|| {
        let earned = &mut state
            .unit_mut(id.0, id.1)
            .map_err(Error::State)?
            .k_experience
            .ab_promotions_earned;
        if let Some(v) = earned
            .iter_mut()
            .rev()
            .find(|v| v.definition_hash == promotion)
        {
            v.value = true
        } else {
            earned.push(HashValue {
                definition_hash: promotion,
                value: true,
            })
        }
        effects
            .earned(state, cid, promotion, index)
            .map_err(Error::Effect)?;
        if effects.is_combat(state, id).map_err(Error::Effect)? {
            effects
                .change_damage(state, id, defs.heal.wrapping_neg())
                .map_err(Error::Effect)?;
        }
        let xp = &mut state
            .unit_mut(id.0, id.1)
            .map_err(Error::State)?
            .k_experience;
        xp.i_level = xp.i_level.wrapping_add(1);
        xp.b_can_promote = false;
        xp.b_can_promote_notify_needed = false;
        effects
            .remove_notification(state, cid)
            .map_err(Error::Effect)?;
        let level = actor(state, id)?.k_experience.i_level;
        effects
            .level_changed(state, cid, level)
            .map_err(Error::Effect)?;
        if actor(state, id)?.k_experience.i_stored_promotions > 0 {
            return Err(Error::Unsupported(
                "stored-promotion ChangeExperience continuation",
            ));
        }
        state
            .unit_mut(id.0, id.1)
            .map_err(Error::State)?
            .k_experience
            .b_experience_locked = false;
        effects
            .completed(state, cid, promotion)
            .map_err(Error::Effect)?;
        if actor(state, id)?.k_experience.i_promote_no_finish_moves <= 0 {
            effects.finish_moves(state, id).map_err(Error::Effect)?;
        }
        effects
            .command_complete(state, cid)
            .map_err(Error::Effect)?;
        Ok(Outcome::Promoted {
            unit: cid,
            promotion,
        })
    })();
    result.map_err(|e| Error::AfterMutation(Box::new(e)))
}
