//! XP2 replacement setters. Mutable effects use the ongoing stock owners.
use super::super::stock_notifications::Runtime;
use crate::{melee_attack::experience, unit_creation, Rules};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, HashValue, Limits, Unit},
};
#[derive(Clone, Debug)]
pub enum Report {
    Embarked(ComponentId, bool),
    VeteranName(ComponentId),
    Level(ComponentId, i32),
    Experience(experience::reporting::Event),
    Promotion(ComponentId, usize),
    Ability(ComponentId, usize),
}
/// Subscriber qualifications describe the actual initialized fixture. They are
/// not defaults: unqualified report/game signal consumers reject before Kill.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromotionConsumer {
    AttachGameEffects,
    NavalGossip,
    LandGossip,
    Achievements,
}
pub struct Inputs<'d> {
    pub rules: &'d Rules,
    pub experience: &'d experience::Definitions,
    pub maximum: &'d crate::formation_movement_cost::Definitions<'d>,
    pub levels: &'d [(i32, u32)],
    pub creation: &'d unit_creation::Definitions,
    pub tags: &'d crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub presentation_reports_qualified: bool,
    /// Actual ordered +1638 subscriptions, including nested std::function targets.
    pub promotion_consumers: Option<Vec<PromotionConsumer>>,
    pub levied_consumers_empty: bool,
    pub reports: Vec<Report>,
    /// Captured at the native pre-destruction read point, consumed once.
    pub saved_modifier: Option<((i32, u32), i32)>,
}
fn fail(e: impl std::fmt::Debug) -> String {
    format!("upgrade transfer: {e:?}")
}
pub fn preflight<P>(
    state: &LoadedState,
    actor: (i32, u32),
    _: u32,
    input: &Inputs<'_>,
    backend: &Runtime<'_, P>,
) -> Result<(), String> {
    backend.check(state).map_err(fail)?;
    if !input.presentation_reports_qualified
        || input.promotion_consumers.is_none()
        || !input.levied_consumers_empty
    {
        return Err("unqualified upgrade setter report/promotion consumers".into());
    }
    let old = super::actor::<()>(state, actor).map_err(fail)?;

    // SetMilitaryFormation reaches additional recalculation; retain an explicit
    // unsupported branch until that setter is implemented over the same owners.
    if !matches!(old.e_military_formation, -1 | 0) {
        return Err("upgrade military formation setter".into());
    }
    let parent = state
        .object_pairing()
        .map_err(fail)?
        .get_parent(ComponentId {
            owner: actor.0 as i16,
            component_type: 1,
            id: actor.1,
        });
    if parent != c6_core::object_pairing::NULL_COMPONENT {
        return Err("upgrade paired parent restoration".into());
    }
    let promotions = crate::unit_initialization_definitions::collection(
        input.rules,
        "UnitPromotions",
        "UnitPromotionType",
    )
    .map_err(fail)?;
    for entry in promotions.entries() {
        let hash = crate::registry::linux_type_hash(&entry.type_name);
        if old
            .k_experience
            .ab_promotions_earned
            .iter()
            .rev()
            .find(|v| v.definition_hash == hash)
            .is_some_and(|v| v.value)
        {
            unit_creation::collections::ability::preflight_promotion(input.rules, &entry.type_name)
                .map_err(fail)?;
        }
    }
    Ok(())
}
/// Exact early-return predicates of the reached stock gossip/achievement
/// consumers. Positive publication remains explicit unsupported gameplay work.
pub(crate) fn promotion_leaf(
    input: &Inputs<'_>,
    consumer: PromotionConsumer,
    unit_type: u32,
    domain: i32,
    level: i32,
) -> Result<(), String> {
    use PromotionConsumer::*;
    let units =
        crate::unit_initialization_definitions::collection(input.rules, "Units", "UnitType")
            .map_err(fail)?;
    if consumer == Achievements {
        let monk = crate::registry::linux_type_hash(b"UNIT_WARRIOR_MONK");
        if units.find_index(monk).is_none() || unit_type != monk || level < 4 {
            return Ok(());
        }
        return Err("promotion WarriorMonk achievement publication".into());
    }
    let name = match consumer {
        NavalGossip => b"GOSSIP_NAVAL_UNIT_LEVEL".as_slice(),
        LandGossip => b"GOSSIP_LAND_UNIT_LEVEL".as_slice(),
        AttachGameEffects => return Ok(()),
        Achievements => unreachable!(),
    };
    let matches = match consumer {
        NavalGossip => domain == 0,
        LandGossip => domain == 2,
        _ => false,
    };
    if !matches {
        return Ok(());
    }
    let table = input
        .rules
        .gameplay()
        .table("Gossips")
        .ok_or("promotion Gossips")?;
    if let Some(row) = table
        .rows()
        .find(|r| r.text("GossipType").ok().map(str::as_bytes) == Some(name))
    {
        let required = i32::try_from(row.integer("LevelRequired").map_err(fail)?).map_err(fail)?;
        if level < required {
            return Ok(());
        }
    }
    Err("promotion gossip occurrence creation/reporting".into())
}
struct Reports<'a, 'd>(
    &'a mut Vec<Report>,
    i32,
    &'a crate::formation_movement_cost::Definitions<'d>,
);
impl experience::Reports for Reports<'_, '_> {
    type Error = String;
    fn observer_can_see_player(&mut self, _: &LoadedState, owner: i32) -> Result<bool, String> {
        Ok(experience::reporting::observer_can_see_player(
            self.1, owner,
        ))
    }
    fn gained(
        &mut self,
        _: &mut LoadedState,
        unit: UnitReference,
        amount: i32,
        promotable: bool,
    ) -> Result<(), String> {
        self.0
            .push(Report::Experience(experience::reporting::Event::Gained {
                unit,
                amount,
                promotable,
            }));
        Ok(())
    }
    fn changed(&mut self, state: &mut LoadedState, unit: UnitReference) -> Result<(), String> {
        self.0
            .push(Report::Experience(experience::reporting::Event::Changed {
                unit,
            }));
        // Translator 320cee0 queries real Promote availability after the cache
        // update. Preserve that query even when the host display DTO is omitted.
        if experience::reporting::promotion_available_after_change(state, self.2, unit)
            .map_err(fail)?
        {
            return Err("upgrade XPChanged positive Promote publication".into());
        }
        Ok(())
    }
}
fn general_modifier(state: &LoadedState, input: &Inputs<'_>, old: &Unit) -> Result<i32, String> {
    let registry = crate::unit_initialization_definitions::collection(
        input.rules,
        "GovernmentBonusNames",
        "GovernmentBonusType",
    )
    .map_err(fail)?;
    let player = super::player::<()>(state, old.e_owner).map_err(fail)?;
    let flat = if player.culture.anarchy_end_turn >= 0 {
        0
    } else if let Some(index) = registry.find_index(0xc819e0a3) {
        *player
            .bonuses
            .apply(&registry, Limits::default())
            .map_err(fail)?
            .get(index)
            .ok_or("government XP bonus")?
    } else {
        0
    };
    Ok(old.k_experience.x_general_experience_modifier.wrapping_add(
        c6_core::movement::MovementPoints::route_float_bits((flat as f32 * 0.01f32).to_bits()).0,
    ))
}
/// Snapshot the original getter before destruction/creation callbacks can change
/// Player bonuses. The old Unit identity binds the one-shot scalar.
pub fn prepare(state: &LoadedState, old: &Unit, input: &mut Inputs<'_>) -> Result<(), String> {
    let value = general_modifier(state, input, old)?;
    input.saved_modifier = Some(((old.e_owner, old.id), value));
    Ok(())
}
pub fn transfer<P>(
    state: &mut LoadedState,
    old: &Unit,
    new: ComponentId,
    input: &mut Inputs<'_>,
    backend: &mut Runtime<'_, P>,
) -> Result<(), String> {
    backend.check(state).map_err(fail)?;
    let owner = i32::from(new.owner);
    let (identity, modifier) = input
        .saved_modifier
        .take()
        .ok_or("upgrade pre-destruction XP modifier snapshot")?;
    if identity != (old.e_owner, old.id) {
        return Err("upgrade XP modifier snapshot identity".into());
    }
    let current = state.unit_mut(owner, new.id).map_err(fail)?;
    let embarked = old.e_activity != 0x5ff1bbdf && old.b_embarked;
    if current.b_embarked != embarked {
        current.b_embarked = embarked;
        input.reports.push(Report::Embarked(new, embarked));
    }
    if !old.k_experience.s_veteran_name.is_empty() {
        current.k_experience.s_veteran_name = old.k_experience.s_veteran_name.clone();
        input.reports.push(Report::VeteranName(new));
    }
    if old.i_damage > 0 {
        let previous = current.i_damage;
        let maximum = input.maximum.maximum_damage(current).map_err(fail)?;
        let damage = old.i_damage.clamp(0, maximum);
        current.i_damage = damage;
        if damage != previous {
            crate::unit_placement_state::Effects::damage_changed(
                backend,
                state,
                c6_core::unit_healing::DamageChangedReport {
                    event_hash: 0xf8a5ab14,
                    identity: new,
                    damage,
                    previous_damage: previous,
                    maximum_damage: maximum,
                },
            )
            .map_err(fail)?;
        }
    }
    if old.e_military_formation != -1
        && state
            .unit_mut(owner, new.id)
            .map_err(fail)?
            .e_military_formation
            != old.e_military_formation
    {
        // Transition to ordinary formation 0 only writes the scalar; corps/army
        // dispatch their dedicated signals and are rejected by preflight.
        state
            .unit_mut(owner, new.id)
            .map_err(fail)?
            .e_military_formation = old.e_military_formation;
    }
    if old.k_experience.i_level > 0 {
        state
            .unit_mut(owner, new.id)
            .map_err(fail)?
            .k_experience
            .i_level = old.k_experience.i_level;
        input
            .reports
            .push(Report::Level(new, old.k_experience.i_level));
    }
    if old.k_experience.i_experience > 0 {
        let observer = backend.local_player().ok_or("current XP observer")?;
        experience::change(
            state,
            input.experience,
            input.maximum,
            input.levels,
            UnitReference::from_component_id(new),
            old.k_experience.i_experience,
            &mut Reports(&mut input.reports, observer, input.maximum),
        )
        .map_err(fail)?;
    }
    let current = state.unit_mut(owner, new.id).map_err(fail)?;
    if modifier > 0 {
        // Native AsInt then ChangeGeneralExperienceModifier(int percent).
        let increment = c6_core::movement::MovementPoints::route_float_bits(
            ((modifier >> 8) as f32 * 0.01f32).to_bits(),
        )
        .0;
        current.k_experience.x_general_experience_modifier = current
            .k_experience
            .x_general_experience_modifier
            .wrapping_add(increment);
    }
    current.b_is_levied = old.b_is_levied;
    current.e_original_owner = old.e_original_owner;
    if old.k_experience.b_can_promote {
        current.k_experience.b_can_promote = true;
        current.k_experience.b_experience_locked = true;
    }
    let promotions = crate::unit_initialization_definitions::collection(
        input.rules,
        "UnitPromotions",
        "UnitPromotionType",
    )
    .map_err(fail)?;
    // Native scans the still-owned delayed old instance here, after creation
    // and preceding setters, then snapshots all selected indices BEFORE any
    // destination promotion callbacks. Other scalar transfers used pre-Kill data.
    let earned = super::actor::<()>(state, (old.e_owner, old.id))
        .map_err(fail)?
        .k_experience
        .ab_promotions_earned
        .clone();
    for (index, entry) in promotions.entries().iter().enumerate() {
        let hash = crate::registry::linux_type_hash(&entry.type_name);
        if !earned
            .iter()
            .rev()
            .find(|v| v.definition_hash == hash)
            .is_some_and(|v| v.value)
        {
            continue;
        }
        let fields = &mut state
            .unit_mut(owner, new.id)
            .map_err(fail)?
            .k_experience
            .ab_promotions_earned;
        if let Some(cell) = fields.iter_mut().rev().find(|v| v.definition_hash == hash) {
            cell.value = true;
        } else {
            fields.push(HashValue {
                definition_hash: hash,
                value: true,
            });
        }
        let consumers = input
            .promotion_consumers
            .clone()
            .ok_or("current promotion consumers")?;
        for consumer in consumers {
            if consumer == PromotionConsumer::AttachGameEffects {
                backend.upgrade_promotion_earned(state, input.rules, new, &entry.type_name)?;
            } else {
                let current = super::actor::<()>(state, (owner, new.id)).map_err(fail)?;
                promotion_leaf(
                    input,
                    consumer,
                    current.e_type_index,
                    state.unit_runtime(owner, new.id).map_err(fail)?.domain,
                    current.k_experience.i_level,
                )?;
            }
        }
        input.reports.push(Report::Promotion(new, index));
    }
    let abilities = crate::unit_initialization_definitions::collection(
        input.rules,
        "UnitAbilities",
        "UnitAbilityType",
    )
    .map_err(fail)?;
    for (index, entry) in abilities.entries().iter().enumerate() {
        let hash = crate::registry::linux_type_hash(&entry.type_name);
        if !old
            .ai_abilities_possessed
            .iter()
            .rev()
            .find(|v| v.definition_hash == hash)
            .is_some_and(|v| v.value > 0)
        {
            continue;
        }
        if state
            .unit_runtime(owner, new.id)
            .map_err(fail)?
            .abilities
            .get(index)
            .copied()
            .ok_or("current ability cell")?
            > 0
        {
            continue;
        }
        let row = input
            .rules
            .gameplay()
            .table("UnitAbilities")
            .and_then(|t| t.row_by_id(entry.database_rowid))
            .ok_or("ability row")?;
        if row.integer("Permanent").map_err(fail)? == 0 {
            continue;
        }
        if !input
            .creation
            .can_have_ability::<std::convert::Infallible>(state, (owner, new.id), input.tags, index)
            .map_err(fail)?
        {
            continue;
        }
        state
            .set_unit_ability_count(owner, new.id, index, hash, 1)
            .map_err(fail)?;
        unit_creation::Effects::ability_changed(backend, state, new, index).map_err(fail)?;
        input.reports.push(Report::Ability(new, index));
    }
    crate::loaded_upkeep::set_unit_activity(state, owner, new.id, backend, old.e_activity)
        .map_err(fail)?;
    Ok(())
}
