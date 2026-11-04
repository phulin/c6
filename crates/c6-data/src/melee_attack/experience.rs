//! Current Unit experience eligibility, bonuses and ordinary melee awards.
use crate::{
    formation_rules,
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules,
};
use c6_core::{
    core_player::initialization::InputIdentities, loaded_state::LoadedState, map::UnitReference,
    melee_attack::experience as kernel, movement::MovementPoints, unit::Limits,
};
#[derive(Debug)]
pub enum Error {
    Binding,
    Definitions(unit_initialization_definitions::Error),
    Rule(crate::Error),
    Parameter(super::damage::Error),
    Actor(super::Error),
    State(c6_core::loaded_state::Error),
    Combat(crate::loaded_actions::Error),
    Bonus(c6_core::player_components::Error),
    Missing(&'static str),
}
pub struct Definitions {
    identities: InputIdentities,
    units: LinuxDefinitionRegistry,
    earn: Vec<bool>,
    random_choices: Vec<i32>,
    next_level_multiplier: i32,
    bonuses: LinuxDefinitionRegistry,
    rules: kernel::Rules,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if resources.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, resources)
            .map_err(Error::Definitions)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let xp2 = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or(Error::Missing("Units_XP2"))?;
        let earn = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Unit row"))?;
                let base = row.integer("CanEarnExperience").map_err(Error::Rule)? != 0;
                let name = row.text("UnitType").map_err(Error::Rule)?;
                let mut extensions = xp2.rows().filter(|r| r.text("UnitType").ok() == Some(name));
                let extension = extensions
                    .next()
                    .map(|r| r.integer("CanEarnExperience").map_err(Error::Rule))
                    .transpose()?
                    .unwrap_or(1)
                    != 0;
                if extensions.next().is_some() {
                    return Err(Error::Missing("duplicate Unit_XP2"));
                }
                Ok(base && extension)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let random_choices = units
            .entries()
            .iter()
            .map(|entry| {
                table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Unit row"))?
                    .integer("NumRandomChoices")
                    .map(|v| v as i32)
                    .map_err(Error::Rule)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let p = |name| super::damage::integer(rules, name).map_err(Error::Parameter);
        Ok(Self {
            identities: state.binding().inputs.clone(),
            units,
            earn,
            random_choices,
            next_level_multiplier: p("EXPERIENCE_NEEDED_FOR_NEXT_LEVEL_MULTIPLIER")?,
            bonuses: collection(rules, "GovernmentBonusNames", "GovernmentBonusType")
                .map_err(Error::Definitions)?,
            rules: kernel::Rules {
                maximum_barbarian_level: p("EXPERIENCE_MAX_BARB_LEVEL")?,
                barbarian_soft_cap: p("EXPERIENCE_BARB_SOFT_CAP")?,
                base: p("EXPERIENCE_NOT_COMBAT_RANGED")?,
                kill_bonus: p("EXPERIENCE_KILL_BONUS")?,
                attacker_bonus: p("EXPERIENCE_COMBAT_ATTACKER_BONUS")?,
                maximum: p("EXPERIENCE_MAXIMUM_ONE_COMBAT")?,
            },
        })
    }
}
struct Context<'a> {
    state: &'a LoadedState,
    defs: &'a Definitions,
    formation: &'a formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    attacker: UnitReference,
    defender: UnitReference,
}
impl Context<'_> {
    fn unit(&self, side: kernel::Side) -> Result<&c6_core::unit::Unit, Error> {
        super::actor(
            self.state,
            if side == kernel::Side::Attacker {
                self.attacker
            } else {
                self.defender
            },
        )
        .map_err(Error::Actor)
    }
}
impl kernel::Context for Context<'_> {
    type Error = Error;
    fn can_earn(&mut self, side: kernel::Side) -> Result<bool, Error> {
        let unit = self.unit(side)?;
        let index = self
            .state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index;
        let index = usize::try_from(index)
            .ok()
            .filter(|i| *i < self.defs.earn.len())
            .ok_or(Error::Missing("Unit definition"))?;
        if self.defs.units.find_index(unit.e_type_index) != Some(index) {
            return Err(Error::Missing("current Unit definition"));
        }
        Ok(self.defs.earn[index] && !unit.k_experience.b_experience_locked)
    }
    fn defender_is_combat(&mut self) -> Result<bool, Error> {
        let unit = self.unit(kernel::Side::Defender)?;
        Ok(self
            .formation
            .combat(self.state, unit)
            .map_err(Error::Combat)?
            > 0
            || self
                .formation
                .ranged(self.state, unit)
                .map_err(Error::Combat)?
                > 0
            || self
                .formation
                .bombard(self.state, unit)
                .map_err(Error::Combat)?
                > 0)
    }
    fn opponent_barbarian_or_free(&mut self, side: kernel::Side) -> Result<bool, Error> {
        let other = if side == kernel::Side::Attacker {
            self.defender
        } else {
            self.attacker
        };
        let mut entries = self.levels.iter().filter(|r| r.0 == other.player());
        let level = entries
            .next()
            .ok_or(Error::Missing("current player level"))?
            .1;
        if entries.next().is_some() {
            return Err(Error::Missing("duplicate player level"));
        }
        Ok(level == 0xd959ec24 || level == 0x04e6b0a0)
    }
    fn level(&mut self, side: kernel::Side) -> Result<i32, Error> {
        Ok(self.unit(side)?.k_experience.i_level)
    }
    fn modifier(&mut self, side: kernel::Side) -> Result<i32, Error> {
        let unit = self.unit(side)?;
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == unit.e_owner)
            .ok_or(Error::Missing("current Player"))?;
        let flat = if let Some(index) = self.defs.bonuses.find_index(0xc819e0a3) {
            if player.culture.anarchy_end_turn >= 0 {
                0
            } else {
                *player
                    .bonuses
                    .apply(&self.defs.bonuses, Limits::default())
                    .map_err(Error::Bonus)?
                    .get(index)
                    .ok_or(Error::Missing("Government bonus"))?
            }
        } else {
            0
        };
        let government = MovementPoints::route_float_bits((flat as f32 * 0.01f32).to_bits()).0;
        let raw = unit.k_experience.x_attacking_experience_modifier_percent;
        let attack = if side == kernel::Side::Attacker {
            MovementPoints::route_float_bits(
                (((raw >> 8) as f32 + (raw & 255) as f32 / 256.0) * 0.01f32).to_bits(),
            )
            .0
        } else {
            0
        };
        Ok(unit
            .k_experience
            .x_general_experience_modifier
            .wrapping_add(government)
            .wrapping_add(attack))
    }
    fn combat(&mut self, side: kernel::Side) -> Result<i32, Error> {
        self.formation
            .combat(self.state, self.unit(side)?)
            .map_err(Error::Combat)
    }
}
/// Defender's award is evaluated before the attacker's, matching generation.
pub fn melee(
    state: &LoadedState,
    defs: &Definitions,
    formation: &formation_rules::Definitions,
    levels: &[(i32, u32)],
    attacker: UnitReference,
    defender: UnitReference,
    defender_killed: bool,
) -> Result<(i32, i32), Error> {
    if state.binding().inputs != defs.identities {
        return Err(Error::Binding);
    }
    let mut c = Context {
        state,
        defs,
        formation,
        levels,
        attacker,
        defender,
    };
    let defense = kernel::melee(&mut c, defs.rules, kernel::Side::Defender, defender_killed)?;
    let attack = kernel::melee(&mut c, defs.rules, kernel::Side::Attacker, defender_killed)?;
    Ok((attack, defense))
}

#[derive(Debug)]
pub struct JointWarPartners {
    pub owners: Vec<i32>,
    pub range: i32,
}
/// GetJointWarExperiencePartners2aad070/GetJointWarPartners2aad0d0. The source
/// chooses the LAST eligible supplier in cached major order, then traverses the
/// same order again for its allies. It does not use the receiving player's list.
pub fn joint_war_partners(
    state: &LoadedState,
    levels: &[(i32, u32)],
    cached_majors: &[i32],
    enemy: i32,
) -> Result<JointWarPartners, Error> {
    let mut result = JointWarPartners {
        owners: vec![],
        range: 0,
    };
    if !state
        .world()
        .player_is_valid(enemy)
        .map_err(|_| Error::Missing("current player validity"))?
    {
        return Ok(result);
    }
    let level = levels
        .iter()
        .find(|r| r.0 == enemy)
        .ok_or(Error::Missing("current player level"))?
        .1;
    if level == 0xd959ec24 {
        return Ok(result);
    }
    let relations = state.game_relations().map_err(Error::State)?;
    let mut supplier = None;
    for &owner in cached_majors {
        if !relations
            .is_at_war(owner, enemy)
            .map_err(|_| Error::Missing("current war relationship"))?
        {
            continue;
        }
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("current Player"))?;
        let values = p.diplomacy.war_and_alliance_modifiers;
        if values[3] > 0 || values[4] > 0 {
            supplier = Some(owner);
            result.range = values[3];
        }
    }
    let Some(supplier) = supplier else {
        return Ok(result);
    };
    for &owner in cached_majors {
        if owner == supplier
            || (relations
                .is_allied(owner, supplier)
                .map_err(|_| Error::Missing("current alliance relationship"))?
                && relations
                    .is_at_war(owner, enemy)
                    .map_err(|_| Error::Missing("current war relationship"))?)
        {
            result.owners.push(owner);
        }
    }
    Ok(result)
}

/// Reached reports only; experience arithmetic and promotion flags are computed
/// here on the current owned Unit, never supplied as an opaque handler answer.
pub trait Reports {
    type Error;
    fn observer_can_see_player(
        &mut self,
        state: &LoadedState,
        owner: i32,
    ) -> Result<bool, Self::Error>;
    fn gained(
        &mut self,
        state: &mut LoadedState,
        unit: UnitReference,
        amount: i32,
        promotable: bool,
    ) -> Result<(), Self::Error>;
    fn changed(&mut self, state: &mut LoadedState, unit: UnitReference) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum ChangeError<E> {
    Query(Error),
    State(c6_core::loaded_state::Error),
    Maximum(crate::formation_movement_cost::Error),
    Effect(E),
    RandomPromotionOptions,
}
/// Unit.Experience.ChangeExperience. Delayed/dead Units remain owned but do not
/// gain XP. Random promotion-option generation remains an explicit continuation
/// after the native XP clamp/lock/promotable writes, never a fabricated choice.
#[allow(clippy::too_many_arguments)]
pub fn change<E: Reports>(
    state: &mut LoadedState,
    defs: &Definitions,
    maximum: &crate::formation_movement_cost::Definitions<'_>,
    levels: &[(i32, u32)],
    unit: UnitReference,
    amount: i32,
    effects: &mut E,
) -> Result<(), ChangeError<E::Error>> {
    if amount <= 0 {
        return Ok(());
    }
    if state.binding().inputs != defs.identities {
        return Err(ChangeError::Query(Error::Binding));
    }
    let actor = super::actor(state, unit).map_err(|e| ChangeError::Query(Error::Actor(e)))?;
    if actor.k_experience.b_experience_locked {
        return Ok(());
    }
    if actor.i_damage
        >= maximum
            .maximum_damage(actor)
            .map_err(ChangeError::Maximum)?
    {
        return Ok(());
    }
    let level = levels
        .iter()
        .find(|r| r.0 == actor.e_owner)
        .ok_or(ChangeError::Query(Error::Missing("current player level")))?
        .1;
    if level == 0xd959ec24 {
        return Ok(());
    }
    let experience = &mut state
        .unit_mut(unit.player(), unit.id)
        .map_err(ChangeError::State)?
        .k_experience;
    experience.i_experience = experience.i_experience.wrapping_add(amount);
    let threshold = kernel::next_level_experience(experience.i_level, defs.next_level_multiplier);
    let promoted = experience.i_experience >= threshold;
    if promoted {
        experience.i_experience = threshold;
        experience.b_experience_locked = true;
        experience.b_can_promote = true;
        experience.b_can_promote_notify_needed = true;
        let index = state
            .unit_runtime(unit.player(), unit.id)
            .map_err(ChangeError::State)?
            .definition_index;
        let index = usize::try_from(index)
            .ok()
            .ok_or(ChangeError::Query(Error::Missing("Unit definition")))?;
        if *defs
            .random_choices
            .get(index)
            .ok_or(ChangeError::Query(Error::Missing("Unit definition")))?
            != 0
        {
            return Err(ChangeError::RandomPromotionOptions);
        }
    }
    let owner = super::actor(state, unit)
        .map_err(|e| ChangeError::Query(Error::Actor(e)))?
        .e_owner;
    if effects
        .observer_can_see_player(state, owner)
        .map_err(ChangeError::Effect)?
    {
        effects
            .gained(state, unit, amount, promoted)
            .map_err(ChangeError::Effect)?;
    }
    effects.changed(state, unit).map_err(ChangeError::Effect)
}
