//! Current canonical Linux ordinary melee strength queries, in native order.
//! Positive branches without admitted inputs remain explicit errors.
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
    Definitions(crate::unit_initialization_definitions::Error),
    Religion(crate::unit_near_religious_city_requirement::QueryError),
    Relations(c6_core::game_diplomatic_relations::Error),
    Player(i32),
}
pub struct Definitions {
    binding: Option<c6_core::core_player::initialization::WireBinding>,
    max_hit_points: i32,
    wounded_multiplier: i32,
    flanking_modifier: Option<i32>,
    bonuses: Option<BonusDefinitions>,
    defense: Option<DefenseDefinitions>,
}
struct ScalarDefinitions {
    registry: crate::registry::LinuxDefinitionRegistry,
    values: Vec<i32>,
}
impl ScalarDefinitions {
    fn load(
        rules: &Rules,
        table: &'static str,
        key: &'static str,
        field: &str,
    ) -> Result<Self, Error> {
        let registry = crate::unit_initialization_definitions::collection(rules, table, key)
            .map_err(Error::Definitions)?;
        let rows = rules
            .gameplay()
            .table(table)
            .ok_or(Error::Missing("strength definition table"))?;
        let values = registry
            .entries()
            .iter()
            .map(|entry| {
                let row = rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("strength definition row"))?;
                match row
                    .get(field)
                    .map_err(|_| Error::Missing("strength definition scalar"))?
                {
                    SqlValue::Integer(value) => Ok(*value as i32),
                    _ => Err(Error::Missing("strength definition integer")),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { registry, values })
    }
    fn index(&self, key: u32) -> Option<usize> {
        self.registry.find_index(key)
    }
}
struct DefenseDefinitions {
    anti_air: ScalarDefinitions,
    terrains: ScalarDefinitions,
    features: ScalarDefinitions,
    improvements: ScalarDefinitions,
    buildings: ScalarDefinitions,
    support: Option<i32>,
    fortify: Option<i32>,
    river: Option<i32>,
    corps_anti_air: Option<i32>,
    army_anti_air: Option<i32>,
}
fn optional_parameter(rules: &Rules, name: &str) -> Option<i32> {
    let mut rows = rules
        .gameplay()
        .table("GlobalParameters")?
        .rows()
        .filter(|row| row.text("Name").ok() == Some(name));
    let first = rows.next()?;
    if rows.next().is_some() {
        return None;
    }
    match first.get("Value").ok()? {
        SqlValue::Integer(value) => Some(*value as i32),
        SqlValue::Text(value) => std::str::from_utf8(value).ok()?.parse().ok(),
        _ => None,
    }
}
struct BonusDefinitions {
    religions: crate::unit_near_religious_city_requirement::Definitions,
    policies: crate::registry::LinuxDefinitionRegistry,
    policy_slots: Vec<Option<usize>>,
    military_slot: Option<usize>,
}
impl Definitions {
    /// Shared District garrison query uses this same quantized combat rule.
    pub fn reduction_for_damage(
        &self,
        state: &LoadedState,
        percent: i32,
        modifier: i32,
    ) -> Result<i32, Error> {
        self.check(state)?;
        Ok(kernel::reduction_for_damage(
            percent,
            modifier,
            self.wounded_multiplier,
        ))
    }
    /// Admit the static religion/policy collections needed by later bonus stages.
    /// Their exact target builder SQL orders GovernmentSlots and Policies by rowid.
    pub fn from_bound_rules(
        rules: &Rules,
        bound: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        use crate::unit_initialization_definitions::collection;
        let mut result = Self::from_rules(rules)?;
        let religions =
            crate::unit_near_religious_city_requirement::Definitions::from_rules(rules, bound)
                .map_err(Error::Definitions)?;
        let slots = collection(rules, "GovernmentSlots", "GovernmentSlotType")
            .map_err(Error::Definitions)?;
        let policies = collection(rules, "Policies", "PolicyType").map_err(Error::Definitions)?;
        let rows = rules
            .gameplay()
            .table("Policies")
            .ok_or(Error::Missing("Policies"))?;
        let mut policy_slots = Vec::with_capacity(rows.rows().len());
        for row in rows.rows() {
            let slot = match row
                .get("GovernmentSlotType")
                .map_err(|_| Error::Missing("Policy GovernmentSlotType"))?
            {
                SqlValue::Text(name) => slots.find_index(crate::registry::linux_type_hash(name)),
                SqlValue::Null => None,
                _ => return Err(Error::Missing("Policy slot reference")),
            };
            policy_slots.push(slot);
        }
        result.binding = Some(bound.destination_binding().clone());
        result.defense = Some(DefenseDefinitions {
            anti_air: ScalarDefinitions::load(rules, "Units", "UnitType", "AntiAirCombat")?,
            terrains: ScalarDefinitions::load(rules, "Terrains", "TerrainType", "DefenseModifier")?,
            features: ScalarDefinitions::load(rules, "Features", "FeatureType", "DefenseModifier")?,
            improvements: ScalarDefinitions::load(
                rules,
                "Improvements",
                "ImprovementType",
                "DefenseModifier",
            )?,
            buildings: ScalarDefinitions::load(
                rules,
                "Buildings",
                "BuildingType",
                "DefenseModifier",
            )?,
            support: optional_parameter(rules, "COMBAT_SUPPORT_BONUS_MODIFIER"),
            fortify: optional_parameter(rules, "FORTIFY_BONUS_PER_TURN"),
            river: optional_parameter(rules, "COMBAT_RIVER_DEFENSE"),
            corps_anti_air: optional_parameter(rules, "COMBAT_CORPS_ANTIAIR_STRENGTH_MODIFIER"),
            army_anti_air: optional_parameter(rules, "COMBAT_ARMY_ANTIAIR_STRENGTH_MODIFIER"),
        });
        result.bonuses = Some(BonusDefinitions {
            religions,
            policies,
            policy_slots,
            military_slot: slots.find_index(crate::registry::linux_type_hash(b"SLOT_MILITARY")),
        });
        Ok(result)
    }

    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        match &self.binding {
            Some(binding) if binding == state.binding() => Ok(()),
            Some(_) => Err(Error::Binding),
            None => Err(Error::Missing(
                "strength definitions require bound resources",
            )),
        }
    }
    /// Raw parameter projection only. Current-state queries require the resource
    /// authority installed by from_bound_rules, preventing cross-world reuse.
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
            binding: None,
            max_hit_points,
            wounded_multiplier,
            flanking_modifier,
            bonuses: None,
            defense: None,
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
    definitions.check(state).map_err(kernel::Error::Query)?;
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
    cached_major_ids: Option<&'a [i32]>,
    ever_alive: Option<&'a [i32]>,
    alive: Option<&'a [i32]>,
}
impl<'a> CurrentPlayerCombat<'a> {
    pub fn new(
        binding: &'a c6_core::core_player::initialization::WireBinding,
        adjacency: &'a [(i32, bool)],
    ) -> Self {
        Self {
            binding,
            adjacency,
            cached_major_ids: None,
            ever_alive: None,
            alive: None,
        }
    }
    pub fn with_cached_major_ids(mut self, values: &'a [i32]) -> Self {
        self.cached_major_ids = Some(values);
        self
    }
    pub fn with_ever_alive(mut self, values: &'a [i32]) -> Self {
        self.ever_alive = Some(values);
        self
    }
    pub fn with_alive(mut self, values: &'a [i32]) -> Self {
        self.alive = Some(values);
        self
    }

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
    definitions.check(state)?;
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
    ranged: bool,
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
        if self.ranged && matches!(stage, B::Amphibious | B::Flanking) {
            // These helpers reject every non-melee kind before any bonus.
            // GetFlankingBonus still reads current owner HasCombatAdjacency.
            if stage == B::Flanking {
                let _ = self
                    .players
                    .has_adjacency(self.state, self.unit(self.attacker)?.e_owner)?;
            }
            return Ok(0);
        }
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
            B::HolyCities => {
                holy_city_bonus(self.state, self.definitions, self.players, self.attacker)
            }
            B::Neighbors => neighbor_bonus(self.state, self.combat, self.attacker),
            B::MilitaryPolicies => {
                military_policy_bonus(self.state, self.definitions, self.attacker)
            }
            B::Diplomatic => diplomatic_bonus(self.state, self.attacker),
            B::Espionage => espionage_bonus(
                self.state,
                self.levels,
                self.attacker,
                self.unit(self.defender)?.e_owner,
            ),
            B::Allies => allies_bonus(self.state, self.players, self.attacker),
            B::AdjacentLevy => adjacent_levy_bonus(self.state, self.combat, self.attacker),
            B::City => city_bonus(
                self.state,
                self.attacker,
                self.unit(self.attacker)?.k_location,
            ),
            B::FriendlyCity => friendly_city_bonus(
                self.state,
                self.players,
                self.attacker,
                self.unit(self.attacker)?.k_location,
            ),
            B::SpecificReligion => {
                specific_religion_bonus(self.state, self.definitions, self.attacker)
            }
            B::UnusedMovement => unused_movement_bonus(self.state, self.attacker),
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
/// Execute the native NULL-Results ordinary attack modifier branch against the current canonical
/// World and proposal owner. Native live-Results preview callbacks are not implemented.
/// Remaining reached bonus branches are explicit errors
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
    definitions
        .check(state)
        .map_err(|e| kernel::AttackError::Opening(kernel::Error::Query(e)))?;
    kernel::ordinary_attack_modifier(&mut CurrentAttack {
        state,
        definitions,
        combat,
        levels,
        players,
        attacker,
        defender,
        proposal_id,
        ranged: false,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ranged_attack_modifier<'a>(
    state: &'a LoadedState,
    definitions: &'a Definitions,
    combat: &'a crate::formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    players: &'a CurrentPlayerCombat<'a>,
    attacker: UnitReference,
    defender: UnitReference,
    proposal_id: u32,
) -> Result<i32, kernel::AttackError<Error>> {
    definitions
        .check(state)
        .map_err(|e| kernel::AttackError::Opening(kernel::Error::Query(e)))?;
    let total = kernel::ordinary_attack_modifier(&mut CurrentAttack {
        state,
        definitions,
        combat,
        levels,
        players,
        attacker,
        defender,
        proposal_id,
        ranged: true,
    })?;
    let attacker_domain = state
        .unit_runtime(attacker.player(), attacker.id)
        .map_err(|e| kernel::AttackError::Opening(kernel::Error::Query(Error::State(e))))?
        .domain;
    let defender_domain = state
        .unit_runtime(defender.player(), defender.id)
        .map_err(|e| kernel::AttackError::Opening(kernel::Error::Query(Error::State(e))))?
        .domain;
    if attacker_domain == 2 && defender_domain == 0 {
        return Err(kernel::AttackError::Opening(kernel::Error::Query(
            Error::Missing("ranged land versus sea penalty"),
        )));
    }
    Ok(total)
}

fn defense_definitions(definitions: &Definitions) -> Result<&DefenseDefinitions, Error> {
    definitions
        .defense
        .as_ref()
        .ok_or(Error::Missing("bound defense definitions"))
}
/// Current GetAntiAirCombat: nonpositive base bypasses all modifiers. The era
/// gate tests x86 signed addition flags, including overflow, before wrapping.
pub fn anti_air_combat(
    state: &LoadedState,
    definitions: &Definitions,
    combat: &crate::formation_rules::Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    let index = combat
        .native_type_index(state, unit)
        .map_err(Error::Combat)?;
    if index == u32::MAX {
        return Ok(0);
    }
    let d = defense_definitions(definitions)?;
    if d.anti_air.index(unit.e_type_index) != Some(index as usize) {
        return Err(Error::Binding);
    }
    let mut value = *d
        .anti_air
        .values
        .get(index as usize)
        .ok_or(Error::Missing("AntiAir definition index"))?;
    if value <= 0 {
        return Ok(value);
    }
    if unit.e_military_formation == 1 {
        value = value.wrapping_add(
            d.corps_anti_air
                .ok_or(Error::Missing("COMBAT_CORPS_ANTIAIR_STRENGTH_MODIFIER"))?,
        );
    } else if unit.e_military_formation == 2 {
        value = value.wrapping_add(
            d.army_anti_air
                .ok_or(Error::Missing("COMBAT_ARMY_ANTIAIR_STRENGTH_MODIFIER"))?,
        );
    }
    let modifier = unit.k_combat_modifers.i_anti_air_strength_modifier.max(0);
    let positive = i64::from(value) + i64::from(modifier) > 0;
    value = value.wrapping_add(modifier);
    if positive {
        value = value.wrapping_add(combat.era_strength(state, unit).map_err(Error::Combat)?);
    }
    Ok(value)
}
pub fn fortification_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let turns = super::actor(state, unit)
        .map_err(Error::Actor)?
        .i_fortify_turns;
    if turns <= 0 {
        return Ok(0);
    }
    let coefficient = defense_definitions(definitions)?
        .fortify
        .ok_or(Error::Missing("FORTIFY_BONUS_PER_TURN"))?;
    Ok(turns.wrapping_mul(coefficient).wrapping_shl(8))
}
/// Unlike flanking, support has no domain/embark filter. IsCombat precedes the
/// owner and full-identity checks. Nonpositive count skips both rule and stat.
pub fn support_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    combat: &crate::formation_rules::Definitions,
    players: &CurrentPlayerCombat<'_>,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if !players.has_adjacency(state, unit.e_owner)? {
        return Ok(0);
    }
    let mut count = 0_i32;
    visit_neighbor_units(state, unit.k_location, |candidate| {
        if is_combat(state, combat, candidate)?
            && candidate.e_owner == unit.e_owner
            && candidate.id != unit.id
        {
            count = count.wrapping_add(1);
        }
        Ok(())
    })?;
    if count <= 0 {
        return Ok(0);
    }
    let global = defense_definitions(definitions)?
        .support
        .ok_or(Error::Missing("COMBAT_SUPPORT_BONUS_MODIFIER"))?;
    Ok(kernel::flanking_from_count(
        count,
        global,
        unit.k_combat_modifers.i_support_bonus_modifier,
    ))
}
/// NULL-Results GetTerrainBonus. An existing improvement bypasses building
/// enumeration even if its defense is nonpositive or it is pillaged.
pub fn terrain_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    defender_plot: c6_core::unit::PlotCoord,
    attacker_plot: c6_core::unit::PlotCoord,
    attacker: UnitReference,
) -> Result<i32, Error> {
    terrain_bonus_with_kind(
        state,
        definitions,
        defender_plot,
        attacker_plot,
        attacker,
        false,
    )
}
fn terrain_bonus_with_kind(
    state: &LoadedState,
    definitions: &Definitions,
    defender_plot: c6_core::unit::PlotCoord,
    attacker_plot: c6_core::unit::PlotCoord,
    attacker: UnitReference,
    ranged: bool,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let d = defense_definitions(definitions)?;
    let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), defender_plot)
        .map_err(Error::Plot)?;
    let mut value = 0_i32;
    if let Some(terrain) = d.terrains.index(plot.terrain_hash) {
        value = d.terrains.values[terrain];
        if let Some(feature) = d.features.index(plot.feature_hash) {
            value = value.wrapping_add(d.features.values[feature]);
        }
    }
    if let Some(improvement) = d.improvements.index(plot.improvement_hash) {
        let modifier = d.improvements.values[improvement];
        if modifier > 0 && !plot.flags.get(c6_core::map::PlotFlag::ImprovementPillaged) {
            value = value.wrapping_add(modifier);
        }
    } else if let Some(attrs) = &plot.city_attributes {
        if attrs.player != -1 && attrs.owning_city != u32::MAX {
            let city = player(state, i32::from(attrs.player))?
                .cities
                .records
                .entries
                .iter()
                .find(|entry| entry.index == (attrs.owning_city & 0xffff) as i32)
                .map(|entry| &entry.value);
            if let Some(city) = city {
                let (plot_index, _) =
                    c6_core::movement_world::plot_at(state.world().map(), defender_plot)
                        .map_err(Error::Plot)?;
                for index in 0..d.buildings.registry.entries().len() {
                    let location = building_location(
                        &d.buildings.registry,
                        &city.buildings.building_locations,
                        index,
                    );
                    if i32::from(location) == plot_index as i32 && d.buildings.values[index] > 0 {
                        value = value.wrapping_add(d.buildings.values[index]);
                    }
                }
            }
        }
    }
    // GetTerrainBonus only reaches river crossing and attacker movement
    // modifiers for MELEE; ranged terrain/improvement defense is retained.
    if ranged {
        return Ok(value.wrapping_shl(8));
    }
    if let Some(direction) =
        c6_core::movement_world::direction_to(state.world().map(), defender_plot, attacker_plot)
            .map_err(Error::Plot)?
    {
        if plot.river_crossing_mask & (1 << direction) != 0 {
            let attacker = super::actor(state, attacker).map_err(Error::Actor)?;
            if !attacker.k_movement_modifiers.b_ignore_rivers {
                value = value.wrapping_add(d.river.ok_or(Error::Missing("COMBAT_RIVER_DEFENSE"))?);
            }
        }
    }
    Ok(value.wrapping_shl(8))
}

/// Buildings.Reset initializes every current definition to -1, then Read's
/// hash remap overlays in wire order. Unknown hashes do not change any cell;
/// index aliases and duplicate hashes overwrite the same initialized cell.
fn building_location(
    registry: &crate::registry::LinuxDefinitionRegistry,
    pairs: &[c6_core::unit::HashValue<i16>],
    index: usize,
) -> i16 {
    pairs.iter().fold(-1, |value, pair| {
        if registry.find_index(pair.definition_hash) == Some(index) {
            pair.value
        } else {
            value
        }
    })
}

struct CurrentDefense<'a>(CurrentAttack<'a>);
impl kernel::Context for CurrentDefense<'_> {
    type Error = Error;
    fn no_damage_reduction(&mut self) -> Result<bool, Error> {
        self.0.no_damage_reduction()
    }
    fn damage(&mut self) -> Result<i32, Error> {
        self.0.damage()
    }
    fn max_hit_points(&mut self) -> Result<i32, Error> {
        self.0.max_hit_points()
    }
    fn damage_reduction_modifier(&mut self) -> Result<i32, Error> {
        self.0.damage_reduction_modifier()
    }
    fn wounded_multiplier(&mut self) -> Result<i32, Error> {
        self.0.wounded_multiplier()
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        self.0.owner_is_major()
    }
    fn fuel_reduction(&mut self) -> Result<i32, Error> {
        self.0.fuel_reduction()
    }
}
impl kernel::DefenseContext for CurrentDefense<'_> {
    fn formation_class(&mut self) -> Result<i32, Error> {
        self.0
            .combat
            .formation_class(self.0.state, self.0.unit(self.0.attacker)?)
            .map_err(Error::Combat)
    }
    fn combat(&mut self) -> Result<i32, Error> {
        self.0
            .combat
            .combat(self.0.state, self.0.unit(self.0.attacker)?)
            .map_err(Error::Combat)
    }
    fn anti_air_combat(&mut self) -> Result<i32, Error> {
        anti_air_combat(
            self.0.state,
            self.0.definitions,
            self.0.combat,
            self.0.attacker,
        )
    }
    fn embarked(&mut self) -> Result<bool, Error> {
        Ok(crate::unit_movement_state::unit_is_embarked(
            self.0.unit(self.0.attacker)?,
        ))
    }
    fn can_fight_embarked(&mut self) -> Result<bool, Error> {
        Ok(self
            .0
            .unit(self.0.attacker)?
            .k_combat_modifers
            .b_fight_while_embarked)
    }
    fn defender_domain(&mut self) -> Result<i32, Error> {
        Ok(self
            .0
            .state
            .unit_runtime(self.0.attacker.player(), self.0.attacker.id)
            .map_err(Error::State)?
            .domain)
    }
    fn attacker_domain(&mut self) -> Result<i32, Error> {
        Ok(self
            .0
            .state
            .unit_runtime(self.0.defender.player(), self.0.defender.id)
            .map_err(Error::State)?
            .domain)
    }
    fn bonus(&mut self, stage: kernel::DefenseBonus) -> Result<i32, Error> {
        use kernel::{AttackBonus as A, AttackContext, DefenseBonus as D};
        let common = match stage {
            D::VersusOpponent => A::VersusOpponent,
            D::HolyCities => A::HolyCities,
            D::Neighbors => A::Neighbors,
            D::MilitaryPolicies => A::MilitaryPolicies,
            D::Diplomatic => A::Diplomatic,
            D::Espionage => A::Espionage,
            D::City => A::City,
            D::AdjacentLevy => A::AdjacentLevy,
            D::Allies => A::Allies,
            D::UnusedMovement => A::UnusedMovement,
            D::FriendlyCity => A::FriendlyCity,
            D::SpecificReligion => A::SpecificReligion,
            D::Territory => {
                return territory_bonus(
                    self.0.state,
                    self.0.attacker,
                    self.0.unit(self.0.attacker)?.k_location,
                )
            }
            D::Terrain => {
                return terrain_bonus_with_kind(
                    self.0.state,
                    self.0.definitions,
                    self.0.unit(self.0.attacker)?.k_location,
                    self.0.unit(self.0.defender)?.k_location,
                    self.0.defender,
                    self.0.ranged,
                )
            }
            D::Fortification => {
                return fortification_bonus(self.0.state, self.0.definitions, self.0.attacker)
            }
            D::Support => {
                if self.0.ranged {
                    let _ = self
                        .0
                        .players
                        .has_adjacency(self.0.state, self.0.unit(self.0.attacker)?.e_owner)?;
                    return Ok(0);
                }
                return support_bonus(
                    self.0.state,
                    self.0.definitions,
                    self.0.combat,
                    self.0.players,
                    self.0.attacker,
                );
            }
            D::AirSupport | D::AntiAirSupport | D::AntiAirDefense => {
                return Err(Error::Missing("current air defense support branch"))
            }
        };
        self.0.bonus(common)
    }
    fn proposal_modifier(&mut self) -> Result<Option<i32>, Error> {
        let registry = self
            .0
            .state
            .combat_proposals()
            .map_err(Error::ProposalAccess)?;
        let Some((_, handle)) = registry
            .active()
            .iter()
            .find(|entry| entry.0 == self.0.proposal_id)
        else {
            return Ok(None);
        };
        Ok(Some(
            registry
                .proposal(*handle)
                .map_err(Error::Proposal)?
                .defender_strength_modifier,
        ))
    }
}
/// Ordinary Unit/Unit defense, NULL Results profile. Both Unit references retain
/// their attacking/defending meaning; only the internal acting-unit view swaps.
#[allow(clippy::too_many_arguments)]
pub fn ordinary_defense_modifier<'a>(
    state: &'a LoadedState,
    definitions: &'a Definitions,
    combat: &'a crate::formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    players: &'a CurrentPlayerCombat<'a>,
    attacker: UnitReference,
    defender: UnitReference,
    proposal_id: u32,
) -> Result<i32, kernel::DefenseError<Error>> {
    definitions
        .check(state)
        .map_err(kernel::DefenseError::Query)?;
    kernel::ordinary_defense_modifier(&mut CurrentDefense(CurrentAttack {
        state,
        definitions,
        combat,
        levels,
        players,
        attacker: defender,
        defender: attacker,
        proposal_id,
        ranged: false,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ranged_defense_modifier<'a>(
    state: &'a LoadedState,
    definitions: &'a Definitions,
    combat: &'a crate::formation_rules::Definitions,
    levels: &'a [(i32, u32)],
    players: &'a CurrentPlayerCombat<'a>,
    attacker: UnitReference,
    defender: UnitReference,
    proposal_id: u32,
) -> Result<i32, kernel::DefenseError<Error>> {
    definitions
        .check(state)
        .map_err(kernel::DefenseError::Query)?;
    kernel::ordinary_defense_modifier(&mut CurrentDefense(CurrentAttack {
        state,
        definitions,
        combat,
        levels,
        players,
        attacker: defender,
        defender: attacker,
        proposal_id,
        ranged: true,
    }))
}

fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Player(owner))
}
/// Outer HolyCities helper queries CreatedReligion before the Unit coefficient.
/// A positive religion still requires current native holy-city lookup storage.
pub fn holy_city_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    players: &CurrentPlayerCombat<'_>,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.e_owner == -1 {
        return Ok(0);
    }
    let defs = definitions
        .bonuses
        .as_ref()
        .ok_or(Error::Missing("bound religion definitions"))?;
    if crate::unit_near_religious_city_requirement::religion_created(
        state,
        &defs.religions,
        unit.e_owner,
    )
    .map_err(Error::Religion)?
        == -1
    {
        return Ok(0);
    }
    // Stats.GetNumHolySitesFollowingReligion(true) repeats CreatedReligion.
    if crate::unit_near_religious_city_requirement::religion_created(
        state,
        &defs.religions,
        unit.e_owner,
    )
    .map_err(Error::Religion)?
        == -1
    {
        return Ok(0);
    }
    if players.binding != state.binding() {
        return Err(Error::Binding);
    }
    for owner in players
        .ever_alive
        .ok_or(Error::Missing("current cached GetWasEverAlive"))?
    {
        let location = &player(state, *owner)?.religion.holy_city_coord;
        if !c6_core::geometry::plot_valid(location.x, location.y) {
            continue;
        }
        return Err(Error::Missing(
            "current native CityContainer holy-city lookups",
        ));
    }
    Ok(0)
}
/// 26ac8d0's signed coefficient gate precedes all map/layer traversal.
pub fn neighbor_bonus(
    state: &LoadedState,
    combat: &crate::formation_rules::Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    let modifier = unit.k_combat_modifers.i_number_neighbor_combat_bonus;
    if unit.e_owner == -1 || modifier <= 0 {
        return Ok(0);
    }
    let mut count = 0_i32;
    visit_neighbor_units(state, unit.k_location, |candidate| {
        if is_combat(state, combat, candidate)?
            && (candidate.e_owner != unit.e_owner || candidate.id != unit.id)
        {
            count = count.wrapping_add(1);
        }
        Ok(())
    })?;
    Ok(if count > 0 {
        count.wrapping_mul(modifier).wrapping_shl(8)
    } else {
        0
    })
}
/// Source military policy query resolves each active Policy's declared slot,
/// independent of the storage slot occupied by that policy. Count precedes coef.
pub fn military_policy_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.e_owner == -1 {
        return Ok(0);
    }
    let owner = player(state, unit.e_owner)?;
    let defs = definitions
        .bonuses
        .as_ref()
        .ok_or(Error::Missing("bound policy definitions"))?;
    let Some(slot) = defs.military_slot else {
        return Ok(0);
    };
    let count = military_policy_count(
        &defs.policies,
        &defs.policy_slots,
        &owner.culture.active_policies.entries,
        slot,
    )?;
    Ok(count
        .wrapping_mul(
            unit.k_combat_modifers
                .i_number_military_policies_combat_modifier,
        )
        .wrapping_shl(8))
}
/// The edit-named Globals.EditPlayer and Instance.EditDiplomacy accessors here
/// are plain pointer lookups, with no writes. Read current timing/records.
pub fn diplomatic_bonus(state: &LoadedState, unit: UnitReference) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.e_owner == -1 {
        return Ok(0);
    }
    let diplomacy = &player(state, unit.e_owner)?.diplomacy;
    Ok(kernel::diplomatic_combat_bonus(
        &diplomacy.combat_modifiers,
        &diplomacy.last_war_and_city_turns,
        state.retained().core.current_turn,
        false,
    )
    .wrapping_shl(8))
}
pub fn espionage_bonus(
    state: &LoadedState,
    levels: &[(i32, u32)],
    unit: UnitReference,
    opponent: i32,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.e_owner == -1 {
        return Ok(0);
    }
    let major = current_level(levels, unit.e_owner)? == 0x253718b0;
    if opponent == -1 || !major {
        return Ok(0);
    }
    if current_level(levels, opponent)? != 0x253718b0 {
        return Ok(0);
    }
    let each = unit.k_combat_modifers.i_each_diplo_viz_combat_mod;
    let higher = unit.k_combat_modifers.i_higher_diplo_viz_combat_mod;
    if (each | higher) == 0 {
        return Ok(0);
    }
    let own_visibility = *player(state, unit.e_owner)?
        .diplomacy
        .visibility
        .get(opponent as usize)
        .ok_or(Error::Missing("current own diplomatic visibility"))?;
    let opponent_visibility = *player(state, opponent)?
        .diplomacy
        .visibility
        .get(unit.e_owner as usize)
        .ok_or(Error::Missing("current opponent diplomatic visibility"))?;
    let mut total = each.wrapping_mul(own_visibility);
    let difference = own_visibility.wrapping_sub(opponent_visibility);
    if higher != 0 && difference > 0 {
        total = total.wrapping_add(difference.wrapping_mul(higher));
    }
    Ok(total.wrapping_shl(8))
}
/// Reads the actual current cached GetAliveMajorIDs in native order, then the
/// current alliance-type matrix, before inspecting the Unit coefficient.
pub fn allies_bonus(
    state: &LoadedState,
    players: &CurrentPlayerCombat<'_>,
    unit: UnitReference,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.e_owner == -1 {
        return Ok(0);
    }
    if players.binding != state.binding() {
        return Err(Error::Binding);
    }
    let ids = players
        .cached_major_ids
        .ok_or(Error::Missing("current cached GetAliveMajorIDs"))?;
    let mut count = 0_i32;
    for owner in ids {
        if *owner == unit.e_owner {
            continue;
        }
        if state
            .game_relations()
            .map_err(Error::State)?
            .alliance_type(unit.e_owner, *owner)
            .map_err(Error::Relations)?
            != -1
        {
            count = count.wrapping_add(1);
        }
    }
    if count <= 0 {
        return Ok(0);
    }
    Ok(count
        .wrapping_mul(unit.k_combat_modifers.i_number_allies_combat_modifier)
        .wrapping_shl(8))
}

fn is_combat(
    state: &LoadedState,
    combat: &crate::formation_rules::Definitions,
    unit: &Unit,
) -> Result<bool, Error> {
    if combat.combat(state, unit).map_err(Error::Combat)? > 0 {
        return Ok(true);
    }
    if combat.ranged(state, unit).map_err(Error::Combat)? > 0 {
        return Ok(true);
    }
    Ok(combat.bombard(state, unit).map_err(Error::Combat)? > 0)
}
fn visit_neighbor_units(
    state: &LoadedState,
    location: c6_core::unit::PlotCoord,
    mut visit: impl FnMut(&Unit) -> Result<(), Error>,
) -> Result<(), Error> {
    for direction in 0..6 {
        let Some((x, y)) = crate::loaded_actions::city_range_attack::adjacent_location(
            state,
            (location.x, location.y),
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
                if let Some(candidate) = state.world().unit(*reference).map_err(Error::World)? {
                    visit(candidate.record)?;
                }
            }
        }
    }
    Ok(())
}
/// Native adjacent-levy always traverses first, even when the acting Unit's
/// coefficient is zero. IsCombat precedes owner and identity filters here.
pub fn adjacent_levy_bonus(
    state: &LoadedState,
    combat: &crate::formation_rules::Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    let mut count = 0_i32;
    visit_neighbor_units(state, unit.k_location, |candidate| {
        if !is_combat(state, combat, candidate)? {
            return Ok(());
        }
        if candidate.e_owner != unit.e_owner || candidate.id == unit.id {
            return Ok(());
        }
        if candidate.b_is_levied {
            count = count.wrapping_add(1);
        }
        Ok(())
    })?;
    if count <= 0 {
        return Ok(0);
    }
    Ok(count
        .wrapping_mul(unit.k_combat_modifers.i_adjacent_levy_combat_modifier)
        .wrapping_shl(8))
}
/// Native CityPlots.GetWorkingCity uses the Plot's current working-city ID and
/// owner, then Cities.Find's recycled slot. It does not require equal generation.
fn working_city(
    state: &LoadedState,
    location: c6_core::unit::PlotCoord,
) -> Result<Option<&c6_core::city::City>, Error> {
    let (_, plot) =
        c6_core::movement_world::plot_at(state.world().map(), location).map_err(Error::Plot)?;
    let owner = crate::movement_placement_state::plot_owner(plot).map_err(Error::Placement)?;
    let Some(attrs) = plot.city_attributes.as_ref() else {
        return Ok(None);
    };
    if owner == -1 || attrs.working_city == u32::MAX {
        return Ok(None);
    }
    let slot = (attrs.working_city & 0xffff) as i32;
    Ok(player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .find(|r| r.index == slot)
        .map(|r| &r.value))
}
pub fn city_bonus(
    state: &LoadedState,
    unit: UnitReference,
    plot: c6_core::unit::PlotCoord,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.k_religion.i_religious_strength > 0 {
        return Ok(0);
    }
    let Some(city) = working_city(state, plot)? else {
        return Ok(0);
    };
    if city.header.owner != unit.e_owner {
        return Ok(0);
    }
    Ok(city.combat.city_combat_bonus.wrapping_shl(8))
}
pub fn friendly_city_bonus(
    state: &LoadedState,
    players: &CurrentPlayerCombat<'_>,
    unit: UnitReference,
    plot: c6_core::unit::PlotCoord,
) -> Result<i32, Error> {
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.k_religion.i_religious_strength > 0 {
        return Ok(0);
    }
    let Some(city) = working_city(state, plot)? else {
        return Ok(0);
    };
    // This getter is reached even for an owned city. Team friendship is queried
    // before the suzerain equality fallback for other owners.
    let suzerain = player(state, city.header.owner)?.influence.suzerain;
    if city.header.owner != unit.e_owner
        && !are_teams_friendly(state, players, city.header.owner, unit.e_owner)?
        && suzerain != unit.e_owner
    {
        return Ok(0);
    }
    Ok(city.combat.city_friendly_combat_bonus.wrapping_shl(8))
}
struct CurrentTeams<'a> {
    state: &'a LoadedState,
    players: &'a CurrentPlayerCombat<'a>,
}
impl kernel::TeamFriendship for CurrentTeams<'_> {
    type Error = Error;
    fn valid(&self, owner: i32) -> Result<bool, Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(Error::World)
    }
    fn team(&self, owner: i32) -> Result<i32, Error> {
        self.state.player_team(owner).map_err(Error::State)
    }
    fn alive(&self) -> Result<&[i32], Error> {
        if self.players.binding != self.state.binding() {
            return Err(Error::Binding);
        }
        self.players
            .alive
            .ok_or(Error::Missing("current cached GetAlive order"))
    }
    fn declared_friendship(&self, owner: i32, other: i32) -> Result<bool, Error> {
        self.state
            .game_relations()
            .map_err(Error::State)?
            .has_declared_friendship(owner, other)
            .map_err(Error::Relations)
    }
    fn allied(&self, owner: i32, other: i32) -> Result<bool, Error> {
        self.state
            .game_relations()
            .map_err(Error::State)?
            .is_allied(owner, other)
            .map_err(Error::Relations)
    }
    fn permanently_allied(&self, owner: i32, other: i32) -> Result<bool, Error> {
        self.state
            .game_relations()
            .map_err(Error::State)?
            .is_permanently_allied(owner, other)
            .map_err(Error::Relations)
    }
}
pub fn are_teams_friendly(
    state: &LoadedState,
    players: &CurrentPlayerCombat<'_>,
    owner: i32,
    other: i32,
) -> Result<bool, Error> {
    kernel::are_teams_friendly(&CurrentTeams { state, players }, owner, other)
}
pub fn specific_religion_bonus(
    state: &LoadedState,
    definitions: &Definitions,
    unit: UnitReference,
) -> Result<i32, Error> {
    definitions.check(state)?;
    let unit = super::actor(state, unit).map_err(Error::Actor)?;
    if unit.k_religion.i_religious_strength > 0 || unit.e_owner == -1 {
        return Ok(0);
    }
    let owner = player(state, unit.e_owner)?;
    let religion = unit.k_religion.e_religion_type;
    if religion == u32::MAX {
        return Ok(0);
    }
    let defs = definitions
        .bonuses
        .as_ref()
        .ok_or(Error::Missing("bound religion definitions"))?;
    let index = defs.religions.religion(religion);
    if index < 0 {
        return Err(Error::Missing("current Unit religion initialized type"));
    }
    if owner
        .religion
        .units_religion_strength_read_effect()
        .is_some()
    {
        return Err(Error::Missing(
            "current initialized religion-strength vector",
        ));
    }
    let value = *owner
        .religion
        .units_religion_strength_by_religion
        .get(index as usize)
        .ok_or(Error::Missing("religion-strength array extent"))?;
    Ok(if value > 0 { value.wrapping_shl(8) } else { 0 })
}

fn military_policy_count(
    policies: &crate::registry::LinuxDefinitionRegistry,
    policy_slots: &[Option<usize>],
    active: &[c6_core::player_culture::GovernmentSlot],
    slot: usize,
) -> Result<i32, Error> {
    let mut count = 0_i32;
    for policy in active {
        if policy.policy_hash == u32::MAX {
            continue;
        }
        let Some(index) = policies.find_index(policy.policy_hash) else {
            continue;
        };
        let reference = policy_slots
            .get(index)
            .copied()
            .flatten()
            .ok_or(Error::Missing("active policy slot reference"))?;
        if reference == slot {
            count = count.wrapping_add(1);
        }
    }
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn building_reset_overlay_preserves_native_alias_order_and_missing_default() {
        use crate::registry::{linux_type_hash, DefinitionEntry, LinuxDefinitionRegistry};
        use c6_core::unit::HashValue;
        let registry = LinuxDefinitionRegistry::from_native_order(vec![
            DefinitionEntry {
                database_rowid: crate::RulesRowId(3),
                type_name: b"SYNTHETIC_BUILDING_A".to_vec(),
            },
            DefinitionEntry {
                database_rowid: crate::RulesRowId(9),
                type_name: b"SYNTHETIC_BUILDING_B".to_vec(),
            },
        ]);
        assert_eq!(building_location(&registry, &[], 1), -1);
        let a = linux_type_hash(b"SYNTHETIC_BUILDING_A");
        let pairs = [
            HashValue {
                definition_hash: a,
                value: 17,
            },
            HashValue {
                definition_hash: u32::MAX,
                value: 99,
            },
            HashValue {
                definition_hash: 0,
                value: 23,
            },
            HashValue {
                definition_hash: a,
                value: -2,
            },
        ];
        assert_eq!(building_location(&registry, &pairs, 0), -2);
        assert_eq!(building_location(&registry, &pairs, 1), -1);
        assert_eq!(building_location(&registry, &pairs[..3], 0), 23);
    }
    #[test]
    fn military_count_uses_declared_policy_type_not_occupied_storage_slot() {
        use crate::{
            registry::{linux_type_hash, DefinitionEntry, LinuxDefinitionRegistry},
            RulesRowId,
        };
        use c6_core::player_culture::GovernmentSlot as S;
        let policies = LinuxDefinitionRegistry::from_native_order(vec![
            DefinitionEntry {
                database_rowid: RulesRowId(7),
                type_name: b"SYNTHETIC_POLICY_A".to_vec(),
            },
            DefinitionEntry {
                database_rowid: RulesRowId(19),
                type_name: b"SYNTHETIC_POLICY_B".to_vec(),
            },
        ]);
        let a = linux_type_hash(b"SYNTHETIC_POLICY_A");
        let b = linux_type_hash(b"SYNTHETIC_POLICY_B");
        let active = [
            S {
                policy_hash: a,
                slot_ordinal: 0,
            },
            S {
                policy_hash: b,
                slot_ordinal: 1,
            },
            S {
                policy_hash: 1,
                slot_ordinal: -1,
            },
            S {
                policy_hash: linux_type_hash(b"MISSING_POLICY"),
                slot_ordinal: 0,
            },
            S {
                policy_hash: u32::MAX,
                slot_ordinal: 0,
            },
        ];
        assert_eq!(
            military_policy_count(&policies, &[Some(1), Some(0)], &active, 0).unwrap(),
            2
        );
        assert!(matches!(
            military_policy_count(&policies, &[None, Some(0)], &active, 0),
            Err(Error::Missing("active policy slot reference"))
        ));
    }
}
