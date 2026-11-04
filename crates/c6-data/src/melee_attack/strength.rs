//! Current canonical damage/fuel contributions to Linux melee strength.
//! Situational bonuses and proposal modifier reads remain mandatory next stages.
use crate::{
    game_parameters::{IntegerParameter, Parameters},
    Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState, map::UnitReference, melee_attack::strength as kernel, unit::Unit,
};
#[derive(Debug)]
pub enum Error {
    Actor(super::Error),
    Parameters(crate::game_parameters::Error),
    Missing(&'static str),
    OwnerLevel(i32),
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Plot(c6_core::movement_world::MissingInput),
    Combat(crate::loaded_actions::Error),
    Binding,
    MissingAdjacency(i32),
    BonusUnavailable(kernel::AttackBonus),
    ProposalAccess(c6_core::loaded_state::combat_proposals::AccessError),
    Proposal(c6_core::melee_attack::proposals::registry::Error),
    Placement(crate::movement_placement_state::Error),
}
pub struct Definitions {
    max_hit_points: i32,
    wounded_multiplier: i32,
    flanking_modifier: Option<i32>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let parameters = Parameters::from_rules(rules).map_err(Error::Parameters)?;
        let max_hit_points = parameters
            .integer(IntegerParameter::CombatMaxHitPoints)
            .map_err(Error::Parameters)?;
        let rows = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut values = rows
            .rows()
            .filter(|r| r.text("Name").ok() == Some("COMBAT_WOUNDED_DAMAGE_MULTIPLIER"));
        let row = values
            .next()
            .ok_or(Error::Missing("COMBAT_WOUNDED_DAMAGE_MULTIPLIER"))?;
        if values.next().is_some() {
            return Err(Error::Missing("duplicate COMBAT_WOUNDED_DAMAGE_MULTIPLIER"));
        }
        let wounded_multiplier = match row
            .get("Value")
            .map_err(|_| Error::Missing("wounded multiplier Value"))?
        {
            SqlValue::Integer(n) => *n as i32,
            SqlValue::Text(s) => std::str::from_utf8(s)
                .map_err(|_| Error::Missing("wounded multiplier UTF8"))?
                .parse::<i32>()
                .map_err(|_| Error::Missing("wounded multiplier integer"))?,
            _ => return Err(Error::Missing("wounded multiplier integer")),
        };
        let mut flanking = rows
            .rows()
            .filter(|r| r.text("Name").ok() == Some("COMBAT_FLANKING_BONUS_MODIFIER"));
        let flanking_modifier = flanking.next().and_then(|r| match r.get("Value").ok()? {
            SqlValue::Integer(n) => Some(*n as i32),
            SqlValue::Text(s) => std::str::from_utf8(s).ok()?.parse::<i32>().ok(),
            _ => None,
        });
        let flanking_modifier = if flanking.next().is_some() {
            None
        } else {
            flanking_modifier
        };
        Ok(Self {
            max_hit_points,
            wounded_multiplier,
            flanking_modifier,
        })
    }
}
struct Current<'a> {
    unit: &'a Unit,
    definitions: &'a Definitions,
    levels: &'a [(i32, u32)],
}
impl kernel::Context for Current<'_> {
    type Error = Error;
    fn no_damage_reduction(&mut self) -> Result<bool, Error> {
        Ok(self.unit.k_combat_modifers.b_no_reduction_for_damage)
    }
    fn damage(&mut self) -> Result<i32, Error> {
        Ok(self.unit.i_damage)
    }
    fn max_hit_points(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.max_hit_points)
    }
    fn damage_reduction_modifier(&mut self) -> Result<i32, Error> {
        Ok(self
            .unit
            .k_combat_modifers
            .i_strength_reduction_for_damage_modifier)
    }
    fn wounded_multiplier(&mut self) -> Result<i32, Error> {
        Ok(self.definitions.wounded_multiplier)
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        let mut found = self
            .levels
            .iter()
            .filter(|(owner, _)| *owner == self.unit.e_owner);
        let level = found.next().ok_or(Error::OwnerLevel(self.unit.e_owner))?.1;
        if found.next().is_some() {
            return Err(Error::OwnerLevel(self.unit.e_owner));
        }
        Ok(level == 0x253718b0)
    }
    fn fuel_reduction(&mut self) -> Result<i32, Error> {
        Ok(self
            .unit
            .k_combat_modifers
            .i_strength_reduction_insufficient_fuel)
    }
}
/// Reads the current World Unit, never the retained initial actor. This is only
/// the opening contribution; callers must not use it as a complete modifier.
pub fn opening_terms(
    state: &LoadedState,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    unit: UnitReference,
    side: kernel::Side,
) -> Result<i32, kernel::Error<Error>> {
    let unit = super::actor(state, unit).map_err(|e| kernel::Error::Query(Error::Actor(e)))?;
    kernel::opening_terms(
        &mut Current {
            unit,
            definitions,
            levels,
        },
        side,
    )
}

/// Current unsaved Player.HasCombatAdjacency values, keyed by actual owner.
/// Unknown and duplicate owners stay errors at the native getter's position.
pub struct CurrentPlayerCombat<'a> {
    pub binding: &'a c6_core::core_player::initialization::WireBinding,
    pub adjacency: &'a [(i32, bool)],
}
impl CurrentPlayerCombat<'_> {
    fn has_adjacency(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        if self.binding != state.binding() {
            return Err(Error::Binding);
        }
        let mut rows = self.adjacency.iter().filter(|r| r.0 == owner);
        let result = rows.next().ok_or(Error::MissingAdjacency(owner))?.1;
        if rows.next().is_some() {
            return Err(Error::MissingAdjacency(owner));
        }
        Ok(result)
    }
}
/// Exact ordinary melee flanking traversal: direction0..5, physical layer order,
/// then original per-cell identity order. Same-owner, other-identity combat
/// units count only when non-air and not embarked. No sorting or World copies.
pub fn flanking_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    combat: &crate::formation_rules::Definitions,
    players: &CurrentPlayerCombat<'_>,
    attacker: UnitReference,
    target: Option<c6_core::unit::PlotCoord>,
) -> Result<i32, Error> {
    let attacker = super::actor(state, attacker).map_err(Error::Actor)?;
    if !players.has_adjacency(state, attacker.e_owner)? {
        return Ok(0);
    }
    if state
        .unit_runtime(attacker.e_owner, attacker.id)
        .map_err(Error::State)?
        .domain
        == 1
    {
        return Ok(0);
    }
    let Some(target) = target else {
        return Ok(0);
    };
    c6_core::movement_world::plot_at(state.world().map(), target).map_err(Error::Plot)?;
    let mut count = 0_i32;
    for direction in 0..6 {
        let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
            state,
            (target.x, target.y),
            direction,
        ) else {
            continue;
        };
        let (index, _) = c6_core::movement_world::plot_at(
            state.world().map(),
            c6_core::unit::PlotCoord { x, y },
        )
        .map_err(Error::Plot)?;
        for layer in &state.world().map().units.layers {
            for reference in state
                .world()
                .plot_units(layer.layer_id, index)
                .map_err(Error::World)?
            {
                if reference.player() == -1 || reference.id == u32::MAX {
                    continue;
                }
                let Some(found) = state.world().unit(*reference).map_err(Error::World)? else {
                    continue;
                };
                let candidate = found.record;
                if candidate.e_owner != attacker.e_owner || candidate.id == attacker.id {
                    continue;
                }
                let mut is_combat = combat.combat(state, candidate).map_err(Error::Combat)? > 0;
                if !is_combat {
                    is_combat = combat.ranged(state, candidate).map_err(Error::Combat)? > 0;
                }
                if !is_combat {
                    is_combat = combat.bombard(state, candidate).map_err(Error::Combat)? > 0;
                }
                if !is_combat {
                    continue;
                }
                if state
                    .unit_runtime(candidate.e_owner, candidate.id)
                    .map_err(Error::State)?
                    .domain
                    == 1
                {
                    continue;
                }
                if !crate::unit_movement_state::unit_is_embarked(candidate) {
                    count = count.wrapping_add(1);
                }
            }
        }
    }
    if count <= 0 {
        return Ok(0);
    }
    let global = definitions
        .flanking_modifier
        .ok_or(Error::Missing("COMBAT_FLANKING_BONUS_MODIFIER"))?;
    Ok(kernel::flanking_from_count(
        count,
        global,
        attacker.k_combat_modifers.i_flanking_bonus_modifier,
    ))
}
/// Native coefficient gate before current movement read, retaining Q8 moves.
pub fn unused_movement_bonus(state: &LoadedState, unit: UnitReference) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    let modifier = unit.k_combat_modifers.i_per_unused_movement_combat_modifier;
    Ok(kernel::unused_movement_bonus(
        unit.x_moves_remaining,
        modifier,
    ))
}

fn current_level(levels: &[(i32, u32)], owner: i32) -> Result<u32, Error> {
    let mut rows = levels.iter().filter(|r| r.0 == owner);
    let value = rows.next().ok_or(Error::OwnerLevel(owner))?.1;
    if rows.next().is_some() {
        return Err(Error::OwnerLevel(owner));
    }
    Ok(value)
}
/// 2bda540: actual Manager.IsBarbarian validity/level precedes the Unit stat.
pub fn versus_opponent_bonus(
    state: &LoadedState,
    levels: &[(i32, u32)],
    unit: UnitReference,
    opponent: i32,
) -> Result<i32, Error> {
    if !state
        .world()
        .player_is_valid(opponent)
        .map_err(Error::World)?
    {
        return Ok(0);
    }
    if current_level(levels, opponent)? != 0xd959ec24 {
        return Ok(0);
    }
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    Ok(unit.k_combat_modifers.i_versus_barbarians.wrapping_shl(8))
}
/// Ordinary nonreligious melee branch of2bdadd0. A zero coefficient skips all
/// ownership queries; a present coefficient reads actual Plot.GetOwner.
pub fn territory_bonus(
    state: &LoadedState,
    unit: UnitReference,
    plot: c6_core::unit::PlotCoord,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    let modifier = unit.k_combat_modifers.i_friendly_territory_combat_mod;
    if modifier == 0 {
        return Ok(0);
    }
    let (_, plot) =
        c6_core::movement_world::plot_at(state.world().map(), plot).map_err(Error::Plot)?;
    let owner = crate::movement_placement_state::plot_owner(plot).map_err(Error::Placement)?;
    Ok(if owner == unit.e_owner {
        modifier.wrapping_shl(8)
    } else {
        0
    })
}
/// 2bde0b0: only positive luxury coefficients query Player.Resources. The
/// concrete owned-luxury counter is still required for that reached branch.
pub fn miscellaneous_attack_bonus(state: &LoadedState, unit: UnitReference) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.k_combat_modifers.i_per_luxury_attack_modifier <= 0 {
        return Ok(0);
    }
    Err(Error::Missing(
        "current Player.Resources.GetNumOwnedLuxuries",
    ))
}
/// Ordinary melee branch of2bde220. Other domains or nonembarked units return
/// before coastal/terrain rules. Reached coastal eligibility stays explicit.
pub fn amphibious_penalty(
    state: &LoadedState,
    unit: UnitReference,
    plot: Option<c6_core::unit::PlotCoord>,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(Error::State)?
        .domain
        == 1
    {
        return Ok(0);
    }
    if !crate::unit_movement_state::unit_is_embarked(unit) {
        return Ok(0);
    }
    if unit.k_combat_modifers.b_fight_while_embarked {
        return Ok(0);
    }
    let ignore = unit.k_movement_modifiers.b_ignore_shores;
    if plot.is_none() || ignore {
        return Ok(0);
    }
    Err(Error::Missing(
        "current coastal land and amphibious penalty",
    ))
}

struct CurrentAttack<'a> {
    state: &'a LoadedState,
    definitions: &'a Definitions,
    combat: &'a crate::formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    players: &'a CurrentPlayerCombat<'a>,
    attacker: UnitReference,
    defender: UnitReference,
    proposal_id: u32,
}
impl CurrentAttack<'_> {
    fn unit(&self, id: UnitReference) -> Result<&Unit, Error> {
        super::actor(self.state, id).map_err(Error::Actor)
    }
    fn current(&self) -> Result<Current<'_>, Error> {
        Ok(Current {
            unit: self.unit(self.attacker)?,
            definitions: self.definitions,
            levels: self.levels,
        })
    }
    fn defender_plot(&self) -> Result<c6_core::unit::PlotCoord, Error> {
        Ok(self.unit(self.defender)?.k_location)
    }
}
impl kernel::Context for CurrentAttack<'_> {
    type Error = Error;
    fn no_damage_reduction(&mut self) -> Result<bool, Error> {
        self.current()?.no_damage_reduction()
    }
    fn damage(&mut self) -> Result<i32, Error> {
        self.current()?.damage()
    }
    fn max_hit_points(&mut self) -> Result<i32, Error> {
        self.current()?.max_hit_points()
    }
    fn damage_reduction_modifier(&mut self) -> Result<i32, Error> {
        self.current()?.damage_reduction_modifier()
    }
    fn wounded_multiplier(&mut self) -> Result<i32, Error> {
        self.current()?.wounded_multiplier()
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        self.current()?.owner_is_major()
    }
    fn fuel_reduction(&mut self) -> Result<i32, Error> {
        self.current()?.fuel_reduction()
    }
}
impl kernel::AttackContext for CurrentAttack<'_> {
    fn defender_exists(&mut self) -> Result<bool, Error> {
        Ok(self
            .state
            .world()
            .unit(self.defender)
            .map_err(Error::World)?
            .is_some())
    }
    fn defender_delayed_death(&mut self) -> Result<bool, Error> {
        Ok(self.unit(self.defender)?.b_delayed_death)
    }
    fn attacker_exists(&mut self) -> Result<bool, Error> {
        Ok(self
            .state
            .world()
            .unit(self.attacker)
            .map_err(Error::World)?
            .is_some())
    }
    fn attacker_delayed_death(&mut self) -> Result<bool, Error> {
        Ok(self.unit(self.attacker)?.b_delayed_death)
    }
    fn bonus(&mut self, stage: kernel::AttackBonus) -> Result<i32, Error> {
        use kernel::AttackBonus as B;
        match stage {
            B::Amphibious => {
                amphibious_penalty(self.state, self.attacker, Some(self.defender_plot()?))
            }
            B::VersusOpponent => versus_opponent_bonus(
                self.state,
                self.levels,
                self.attacker,
                self.unit(self.defender)?.e_owner,
            ),
            B::Territory => territory_bonus(self.state, self.attacker, self.defender_plot()?),
            B::Flanking => flanking_bonus(
                self.state,
                self.definitions,
                self.combat,
                self.players,
                self.attacker,
                Some(self.defender_plot()?),
            ),
            B::Miscellaneous => miscellaneous_attack_bonus(self.state, self.attacker),
            B::UnusedMovement => unused_movement_bonus(self.state, self.attacker),
            _ => Err(Error::BonusUnavailable(stage)),
        }
    }
    fn proposal_modifier(&mut self) -> Result<Option<i32>, Error> {
        let registry = self
            .state
            .combat_proposals()
            .map_err(Error::ProposalAccess)?;
        let Some((_, handle)) = registry.active().iter().find(|r| r.0 == self.proposal_id) else {
            return Ok(None);
        };
        Ok(Some(
            registry
                .proposal(*handle)
                .map_err(Error::Proposal)?
                .attacker_strength_modifier,
        ))
    }
}
/// Execute all ordinary attack modifier stages against the current canonical
/// World and proposal owner. Remaining bonus implementations are explicit errors
/// at the reached stage; no damage or successful attack follows such an error.
#[allow(clippy::too_many_arguments)]
pub fn ordinary_attack_modifier<'a>(
    state: &'a LoadedState,
    definitions: &'a Definitions,
    combat: &'a crate::formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    players: &'a CurrentPlayerCombat<'a>,
    attacker: UnitReference,
    defender: UnitReference,
    proposal_id: u32,
) -> Result<i32, kernel::AttackError<Error>> {
    kernel::ordinary_attack_modifier(&mut CurrentAttack {
        state,
        definitions,
        combat,
        levels,
        players,
        attacker,
        defender,
        proposal_id,
    })
}
