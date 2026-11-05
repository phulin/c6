//! Explicit Linux XP2 Unit upgrades over the canonical loaded owners.
//! Unsupported callback branches return errors and retain any executed prefix.
pub mod resources;
pub mod stock;
pub mod transfer;
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash as hash,
    unit_creation::signals::BuildMaps, unit_initialization_definitions::collection, Rules,
    SqlValue,
};
use c6_core::{
    city_production::execution::{from_float, multiply},
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    unit::{ComponentId, HashValue, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error<E> {
    Binding,
    State(c6_core::loaded_state::Error),
    Rules(crate::Error),
    Missing(&'static str),
    Query(E),
    Effect(E),
    /// Execution began and could not complete; the Session must retain failure.
    Incomplete(&'static str),
    AfterMutation(Box<Error<E>>),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    CommandUnavailable,
    CannotMove,
    Dead,
    Barbarian,
    NoUpgrade,
    InsufficientGold,
    Territory,
    InsufficientResource,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected(Rejection),
    Upgraded {
        old: ComponentId,
        new: ComponentId,
        gold_cost: i32,
        resource_cost: i32,
    },
}
struct Definition {
    hash: u32,
    tech: Option<usize>,
    civic: Option<usize>,
    upgrades: Vec<u32>,
    resource: Option<u32>,
}
pub struct Definitions {
    inputs: InputIdentities,
    units: Vec<Definition>,
    speed: i32,
    base: i32,
    net: i32,
    equivalent: i32,
    minimum: i32,
    levy_minimum: i32,
    divisor: i32,
}
fn parameter(rules: &Rules, name: &'static str) -> Result<i32, Error<()>> {
    rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Missing("GlobalParameters"))?
        .rows()
        .find(|r| r.text("Name").ok() == Some(name))
        .and_then(|r| r.get("Value").ok().and_then(sql_integer))
        .ok_or(Error::Missing(name))
}
fn reference(row: crate::Row<'_>, name: &'static str) -> Result<Option<u32>, Error<()>> {
    match row.get(name).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(hash(v))),
        _ => Err(Error::Missing(name)),
    }
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        game_speed: u32,
    ) -> Result<Self, Error<()>> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Missing("Linux upgrade rules profile"));
        }
        let registry =
            collection(rules, "Units", "UnitType").map_err(|_| Error::Missing("upgrade Units"))?;
        let techs = collection(rules, "Technologies", "TechnologyType")
            .map_err(|_| Error::Missing("upgrade Technologies"))?;
        let civics = collection(rules, "Civics", "CivicType")
            .map_err(|_| Error::Missing("upgrade Civics"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let upgrades = rules
            .gameplay()
            .table("UnitUpgrades")
            .ok_or(Error::Missing("UnitUpgrades"))?;
        let mut units = Vec::new();
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("upgrade Unit row"))?;
            let unit_hash = hash(&entry.type_name);
            let mut next = Vec::new();
            for edge in upgrades.rows() {
                if reference(edge, "Unit")? == Some(unit_hash) {
                    if let Some(v) = reference(edge, "UpgradeUnit")? {
                        next.push(v);
                    }
                }
            }
            units.push(Definition {
                hash: unit_hash,
                tech: reference(row, "PrereqTech")?.and_then(|v| techs.find_index(v)),
                civic: reference(row, "PrereqCivic")?.and_then(|v| civics.find_index(v)),
                upgrades: next,
                resource: reference(row, "StrategicResource")?,
            });
        }
        let speeds = collection(rules, "GameSpeeds", "GameSpeedType")
            .map_err(|_| Error::Missing("upgrade GameSpeeds"))?;
        let speed = if let Some(entry) = speeds.find(game_speed) {
            rules
                .gameplay()
                .table("GameSpeeds")
                .and_then(|t| t.row_by_id(entry.database_rowid))
                .and_then(|r| r.get("CostMultiplier").ok().and_then(sql_integer))
                .ok_or(Error::Missing("upgrade speed multiplier"))?
        } else if game_speed <= 0x7ff {
            100
        } else {
            return Err(Error::Missing("upgrade speed definition"));
        };
        let divisor = parameter(rules, "PURCHASE_DIVISOR")?;
        if divisor == 0 {
            return Err(Error::Missing("nonzero upgrade divisor"));
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            units,
            speed,
            base: parameter(rules, "UPGRADE_BASE_COST")?,
            net: parameter(rules, "UPGRADE_NET_PRODUCTION_PERCENT_COST")?,
            equivalent: parameter(rules, "GOLD_EQUIVALENT_OTHER_YIELDS")?,
            minimum: parameter(rules, "UPGRADE_MINIMUM_COST")?,
            levy_minimum: parameter(rules, "UPGRADE_MINIMUM_COST_LEVY")?,
            divisor,
        })
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.inputs == state.binding().inputs {
            Ok(())
        } else {
            Err(Error::Binding)
        }
    }
    fn unit<E>(&self, hash: u32) -> Result<&Definition, Error<E>> {
        self.units
            .iter()
            .find(|v| v.hash == hash)
            .ok_or(Error::Missing("upgrade Unit definition"))
    }
    fn scale(&self, value: i32) -> i32 {
        let raw = i64::from(value.wrapping_shl(8)) * i64::from(self.speed.wrapping_shl(8)) / 25600;
        raw.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
}
fn actor<E>(state: &LoadedState, id: (i32, u32)) -> Result<&Unit, Error<E>> {
    state
        .world()
        .player(id.0)
        .map_err(|_| Error::Missing("upgrade Player"))?
        .unit(id.1)
        .map_err(|_| Error::Missing("upgrade Unit"))?
        .map(|u| u.record)
        .ok_or(Error::Missing("upgrade Unit"))
}
fn player<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("upgrade Player"))
}
fn cell<E>(values: &[HashValue<i32>], hash: u32) -> Result<i32, Error<E>> {
    values
        .iter()
        .rev()
        .find(|v| v.definition_hash == hash)
        .map(|v| v.value)
        .ok_or(Error::Missing("upgrade current Unit cost cell"))
}
fn next<E>(
    defs: &Definitions,
    maps: &BuildMaps,
    owner: i32,
    current: u32,
) -> Result<Option<u32>, Error<E>> {
    let d = defs.unit(current)?;
    let Some(hash) = d
        .upgrades
        .iter()
        .find(|v| defs.units.iter().any(|d| d.hash == **v))
    else {
        return Ok(None);
    };
    let index = defs
        .units
        .iter()
        .position(|d| d.hash == *hash)
        .ok_or(Error::Missing("upgrade next index"))?;
    let replacement = *maps
        .players
        .iter()
        .find(|p| p.0 == owner)
        .and_then(|p| p.1.get(index))
        .ok_or(Error::Missing("upgrade current build map"))?;
    if replacement < 0 {
        Ok(Some(*hash))
    } else {
        Ok(Some(
            defs.units
                .get(replacement as usize)
                .ok_or(Error::Missing("upgrade replacement index"))?
                .hash,
        ))
    }
}
fn cycle<E>(
    defs: &Definitions,
    maps: &BuildMaps,
    owner: i32,
    start: u32,
) -> Result<bool, Error<E>> {
    let mut seen = Vec::new();
    let mut current = start;
    while let Some(v) = next(defs, maps, owner, current)? {
        if seen.contains(&v) {
            return Ok(true);
        }
        seen.push(v);
        current = v;
    }
    Ok(false)
}
/// Furthest prerequisite-available target, using the live replacement map.
pub fn target<E>(
    state: &LoadedState,
    defs: &Definitions,
    maps: &BuildMaps,
    id: (i32, u32),
) -> Result<Option<u32>, Error<E>> {
    defs.check(state)?;
    if maps.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let mut current = actor(state, id)?.e_type_index;
    let mut result = None;
    while let Some(v) = next(defs, maps, id.0, current)? {
        let d = defs.unit(v)?;
        let p = state
            .world()
            .player(id.0)
            .map_err(|_| Error::Missing("upgrade prerequisite Player"))?;
        for (index, mask) in [(d.tech, &p.technologies), (d.civic, &p.civics)] {
            if let Some(i) = index {
                if !mask
                    .as_ref()
                    .and_then(|m| m.get(i))
                    .copied()
                    .ok_or(Error::Missing("upgrade prerequisite mask"))?
                {
                    return Ok(result);
                }
            }
        }
        result = Some(v);
        if cycle(defs, maps, id.0, v)? {
            break;
        }
        current = v;
    }
    Ok(result)
}
fn discount(raw: i32, percent: i32) -> i32 {
    raw.wrapping_sub(multiply(raw, from_float(percent as f32 / 100.0)))
}
/// GetUpgradeCost: speed base, live cost difference, Unit/formation/treasury
/// discounts, native minimum branches, then signed divisor rounding.
pub fn gold_cost<E>(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    target: u32,
) -> Result<i32, Error<E>> {
    defs.check(state)?;
    let u = actor(state, id)?;
    let p = player(state, id.0)?;
    defs.unit(u.e_type_index)?;
    defs.unit(target)?;
    let old = cell(&p.units.current_unit_cost, u.e_type_index)?;
    let new = cell(&p.units.current_unit_cost, target)?;
    let net = (new.wrapping_sub(old).wrapping_mul(defs.net) / 100)
        .max(0)
        .wrapping_mul(defs.equivalent);
    let mut raw = defs
        .scale(defs.base)
        .wrapping_add(net.wrapping_shl(8))
        .wrapping_sub(u.i_upgrade_discount.wrapping_shl(8));
    raw = match u.e_military_formation {
        1 => multiply(raw, 512),
        2 => multiply(raw, 768),
        _ => raw,
    };
    raw = discount(raw, p.treasury.unit_upgrade_discount_percent);
    if raw < defs.minimum.wrapping_shl(8) {
        raw = (defs.scale(defs.minimum) >> 8).wrapping_shl(8);
    }
    if u.b_is_levied {
        raw = discount(raw, p.treasury.levied_unit_upgrade_discount_percent);
        if raw < defs.levy_minimum.wrapping_shl(8) {
            raw = (defs.scale(defs.levy_minimum) >> 8).wrapping_shl(8);
        }
    }
    let value = raw >> 8;
    Ok(value.wrapping_sub(value.wrapping_rem(defs.divisor)))
}
/// Upgrade strategic resource cost uses the current Player.Units array.
pub fn resource_cost<E>(
    state: &LoadedState,
    defs: &Definitions,
    id: (i32, u32),
    target: u32,
) -> Result<i32, Error<E>> {
    defs.check(state)?;
    let u = actor(state, id)?;
    let old = defs.unit(u.e_type_index)?;
    let new = defs.unit(target)?;
    let p = player(state, id.0)?;
    if new.resource.is_none() {
        return Ok(0);
    }
    let mut base = if old.resource == new.resource {
        0
    } else {
        cell(&p.units.current_unit_resource_cost, target)?
    };
    base = match u.e_military_formation {
        1 => base.wrapping_mul(2),
        2 => base.wrapping_mul(3),
        _ => base,
    };
    if u.b_is_levied {
        let mut raw = discount(
            base.wrapping_shl(8),
            p.treasury.levied_unit_upgrade_discount_percent,
        );
        if raw < defs.levy_minimum.wrapping_shl(8) {
            raw = (defs.scale(defs.levy_minimum) >> 8).wrapping_shl(8);
        }
        base = raw >> 8;
    }
    Ok(if base > 0 {
        base.wrapping_sub(multiply(p.units.upgrade_cost_modifier_q8, base.wrapping_shl(8)) >> 8)
            .max(1)
    } else {
        0
    })
}
/// Source-derived current gameplay scope, recomputed for each request.
pub struct Scope {
    pub barbarian: bool,
    pub major: bool,
    pub diplomatic_territory: i32,
    pub ignore_resource: bool,
}
pub trait Effects {
    type Error;
    fn common_guards(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error>;
    fn query_scope(&self, state: &LoadedState, actor: (i32, u32)) -> Result<Scope, Self::Error>;
    fn preflight_transfer(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        target: u32,
    ) -> Result<(), Self::Error>;
    /// Snapshot setter inputs that native XP2 reads before death callbacks.
    fn prepare_transfer(&mut self, state: &LoadedState, old: &Unit) -> Result<(), Self::Error>;
    fn command_started(
        &mut self,
        state: &mut LoadedState,
        old: ComponentId,
    ) -> Result<(), Self::Error>;
    fn kill(&mut self, state: &mut LoadedState, actor: (i32, u32)) -> Result<(), Self::Error>;
    fn create(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        target: u32,
        location: PlotCoord,
    ) -> Result<Option<ComponentId>, Self::Error>;
    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error>;
    fn transfer(
        &mut self,
        state: &mut LoadedState,
        old: &Unit,
        new: ComponentId,
    ) -> Result<(), Self::Error>;
    fn finish_moves(
        &mut self,
        state: &mut LoadedState,
        new: ComponentId,
    ) -> Result<(), Self::Error>;
    fn processed_turn(
        &mut self,
        state: &mut LoadedState,
        new: ComponentId,
    ) -> Result<(), Self::Error>;
    fn preflight_resource(
        &self,
        state: &LoadedState,
        resource: u32,
        amount: i32,
    ) -> Result<(), Self::Error>;
    fn consume_resource(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        resource: u32,
        amount: i32,
    ) -> Result<(), Self::Error>;
    fn upgraded(&mut self, state: &mut LoadedState, new: ComponentId) -> Result<(), Self::Error>;
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        old: ComponentId,
    ) -> Result<(), Self::Error>;
}
pub fn upgrade<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    maps: &BuildMaps,
    id: (i32, u32),
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    defs.check(state)?;
    if !effects.common_guards(state, id).map_err(Error::Query)? {
        return Ok(Outcome::Rejected(Rejection::CommandUnavailable));
    }
    let old = actor(state, id)?;
    if old.b_delayed_death {
        return Ok(Outcome::Rejected(Rejection::Dead));
    }
    if !c6_core::skip_turn::legality::can_move(old) {
        return Ok(Outcome::Rejected(Rejection::CannotMove));
    }
    if !old.k_operations.nodes.is_empty() {
        return Err(Error::Missing("upgrade queued operation interruption"));
    }
    let scope = effects.query_scope(state, id).map_err(Error::Query)?;
    if scope.barbarian {
        return Ok(Outcome::Rejected(Rejection::Barbarian));
    }
    let Some(target) = target(state, defs, maps, id)? else {
        return Ok(Outcome::Rejected(Rejection::NoUpgrade));
    };
    let quoted_gold_cost = gold_cost(state, defs, id, target)?;
    if player(state, id.0)?.treasury.gold_balance_q8 >> 8 < quoted_gold_cost {
        return Ok(Outcome::Rejected(Rejection::InsufficientGold));
    }
    if scope.diplomatic_territory != 1 {
        return Ok(Outcome::Rejected(Rejection::Territory));
    }
    let resource = defs.unit(target)?.resource;
    let quoted_resource_cost = resource_cost(state, defs, id, target)?;
    if let Some(resource) = resource {
        if !scope.ignore_resource {
            let available = player(state, id.0)?
                .resources
                .resource_ownership
                .iter()
                .find(|r| r.resource_hash == resource)
                .and_then(|r| r.value.first())
                .copied()
                .ok_or(Error::Missing("upgrade strategic stockpile"))?;
            if available < quoted_resource_cost {
                return Ok(Outcome::Rejected(Rejection::InsufficientResource));
            }
        }
    }
    if scope.major && !scope.ignore_resource {
        if let Some(resource) = resource {
            effects
                .preflight_resource(state, resource, quoted_resource_cost)
                .map_err(Error::Query)?;
        }
    }
    effects
        .preflight_transfer(state, id, target)
        .map_err(Error::Query)?;
    let old_id = ComponentId {
        owner: id.0 as i16,
        component_type: 1,
        id: id.1,
    };
    effects
        .command_started(state, old_id)
        .map_err(Error::Effect)?;
    let old = actor(state, id)
        .map_err(|e| Error::AfterMutation(Box::new(e)))?
        .clone();
    effects
        .prepare_transfer(state, &old)
        .map_err(Error::Effect)?;
    let cost_target =
        self::target(state, defs, maps, id).map_err(|e| Error::AfterMutation(Box::new(e)))?;
    let cost = match cost_target {
        Some(target) => {
            gold_cost(state, defs, id, target).map_err(|e| Error::AfterMutation(Box::new(e)))?
        }
        None => 0,
    };
    effects.kill(state, id).map_err(Error::Effect)?;
    let new = effects
        .create(state, id.0, target, old.k_location)
        .map_err(Error::Effect)?
        .ok_or(Error::Incomplete(
            "upgrade creation failed after old Unit death",
        ))?;
    let balance = player(state, id.0)
        .map_err(|e| Error::AfterMutation(Box::new(e)))?
        .treasury
        .gold_balance_q8;
    let next = balance.saturating_add(cost.wrapping_neg().wrapping_shl(8));
    if balance != next {
        state
            .treasury_mut(id.0)
            .map_err(|e| Error::AfterMutation(Box::new(Error::State(e))))?
            .gold_balance_q8 = next;
        effects.gold_changed(state, id.0).map_err(Error::Effect)?;
    }
    effects.transfer(state, &old, new).map_err(Error::Effect)?;
    effects.finish_moves(state, new).map_err(Error::Effect)?;
    effects.processed_turn(state, new).map_err(Error::Effect)?;
    // XP2 quotes payment again after creation/transfer callbacks. They may
    // change live Player.Units costs or modifiers; the legality quote is not
    // a cached payment amount.
    let mut resource_cost = 0;
    if let Some(resource) = resource {
        let payment_scope = effects
            .query_scope(state, (id.0, new.id))
            .map_err(Error::Effect)?;
        if payment_scope.major && !payment_scope.ignore_resource {
            let payment_target = self::target(state, defs, maps, id)
                .map_err(|e| Error::AfterMutation(Box::new(e)))?;
            resource_cost = match payment_target {
                Some(target) => self::resource_cost(state, defs, id, target)
                    .map_err(|e| Error::AfterMutation(Box::new(e)))?,
                None => 0,
            };
            effects
                .consume_resource(state, id.0, resource, resource_cost)
                .map_err(Error::Effect)?;
        }
    }
    effects.upgraded(state, new).map_err(Error::Effect)?;
    effects
        .command_complete(state, old_id)
        .map_err(Error::Effect)?;
    Ok(Outcome::Upgraded {
        old: old_id,
        new,
        gold_cost: cost,
        resource_cost,
    })
}

#[cfg(test)]
mod tests {
    use super::discount;

    #[test]
    fn treasury_discount_retains_native_fixed_fraction_before_final_rounding() {
        // 33/100 is first quantized to 84/256. Integer percent arithmetic
        // would incorrectly produce 67 gold rather than raw Q8 17200.
        assert_eq!(discount(100 * 256, 33), 17_200);
        assert_eq!(discount(100 * 256, -33), 34_100);
        assert_eq!(discount(110 * 256, 100), 0);
    }
}
