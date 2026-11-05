//! Explicit City gold purchases on canonical Treasury and unit factory owners.
//! Faith and specialized placement/resource branches remain explicit.
pub mod stock;
use crate::{
    city_production_turn::{self as production, unit_selection as selection},
    game_parameters::sql_integer,
    registry::linux_type_hash as hash,
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    unit::{ComponentId, HashValue},
};
#[derive(Debug)]
pub enum Error<E> {
    Binding,
    State(c6_core::loaded_state::Error),
    Rules(crate::Error),
    Production(production::Error),
    Missing(&'static str),
    Effect(E),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    CannotTrain,
    CannotPlace,
    BannedYield,
    InsufficientGold,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected(Rejection),
    Purchased {
        unit: Option<ComponentId>,
        gold_cost: i32,
    },
}
struct Unit {
    hash: u32,
    domain: u32,
    formation: u32,
    combat: i32,
    stackable: bool,
    trade: bool,
    religion: bool,
    population_cost: Option<i32>,
    strategic: bool,
    native_index: usize,
}
pub struct Definitions {
    inputs: InputIdentities,
    pub selection: selection::Definitions,
    units: Vec<Unit>,
    gold_yield: i32,
    production_yield: i32,
    multiplier: i32,
    gold_equivalent: i32,
    divisor: i32,
    discount_defined: bool,
}
fn parameter(rules: &Rules, name: &'static str) -> Result<i32, Error<()>> {
    let table = rules
        .gameplay()
        .table("GlobalParameters")
        .ok_or(Error::Missing("GlobalParameters"))?;
    for row in table.rows() {
        if matches!(row.get("Name").map_err(Error::Rules)?,SqlValue::Text(v)if v==name.as_bytes()) {
            return sql_integer(row.get("Value").map_err(Error::Rules)?)
                .ok_or(Error::Missing(name));
        }
    }
    Err(Error::Missing(name))
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error<()>> {
        let registry = collection(rules, "Units", "UnitType")
            .map_err(|_| Error::Missing("purchase Units registry"))?;
        let yields = collection(rules, "Yields", "YieldType")
            .map_err(|_| Error::Missing("purchase Yields registry"))?;
        let index = |name: &[u8]| {
            yields
                .find_index(hash(name))
                .map(|v| v as i32)
                .ok_or(Error::Missing("purchase Yield index"))
        };
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mut units = Vec::new();
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Unit row"))?;
            let number = |name| {
                sql_integer(row.get(name).map_err(Error::Rules)?).ok_or(Error::Missing(name))
            };
            let reference = |name| match row.get(name).map_err(Error::Rules)? {
                SqlValue::Text(v) => Ok(hash(v)),
                SqlValue::Null => Ok(0),
                _ => Err(Error::Missing(name)),
            };
            let unit_hash = hash(&entry.type_name);
            units.push(Unit {
                hash: unit_hash,
                domain: reference("Domain")?,
                formation: reference("FormationClass")?,
                combat: number("Combat")?,
                stackable: number("Stackable")? != 0,
                trade: number("MakeTradeRoute")? != 0,
                religion: number("TrackReligion")? != 0,
                population_cost: match row.get("PopulationCost").map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    v => Some(sql_integer(v).ok_or(Error::Missing("PopulationCost"))?),
                },
                strategic: reference("StrategicResource")? != 0,
                native_index: units.len(),
            });
        }
        let bonuses = collection(rules, "GovernmentBonuses", "GovernmentBonusType")
            .map_err(|_| Error::Missing("purchase GovernmentBonuses"))?;
        let divisor = parameter(rules, "PURCHASE_DIVISOR")?;
        if divisor == 0 {
            return Err(Error::Missing("nonzero PURCHASE_DIVISOR"));
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            selection: selection::Definitions::from_rules(state, rules)
                .map_err(Error::Production)?,
            units,
            gold_yield: index(b"YIELD_GOLD")?,
            production_yield: index(b"YIELD_PRODUCTION")?,
            multiplier: parameter(rules, "GOLD_PURCHASE_MULTIPLIER")?,
            gold_equivalent: parameter(rules, "GOLD_EQUIVALENT_OTHER_YIELDS")?,
            divisor,
            discount_defined: bonuses.find_index(0x7530bd65).is_some(),
        })
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.inputs != state.binding().inputs {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    fn unit<E>(&self, unit: u32) -> Result<&Unit, Error<E>> {
        self.units
            .iter()
            .find(|d| d.hash == unit)
            .ok_or(Error::Missing("purchase Unit definition"))
    }
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
        .ok_or(Error::Missing("purchase Player"))
}
fn cell<E>(values: &[HashValue<i32>], hash: u32) -> Result<i32, Error<E>> {
    values
        .iter()
        .rev()
        .find(|v| v.definition_hash == hash)
        .map(|v| v.value)
        .ok_or(Error::Missing("purchase current definition cell"))
}
fn discounted(base: i32, modifier: i32) -> i32 {
    if modifier == 0 {
        base
    } else {
        100i32
            .wrapping_sub(modifier)
            .wrapping_mul(base)
            .wrapping_div(100)
            .max(1)
    }
}
/// Live GetUnitPurchaseCost2887bc0, including native wrapping and final divisor.
pub fn unit_gold_cost<E>(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    city: u32,
    unit: u32,
    formation: i32,
) -> Result<i32, Error<E>> {
    defs.check(state)?;
    let d = defs.unit(unit)?;
    let p = player(state, owner)?;
    let base = selection::production_cost(state, owner, unit, formation, &defs.selection)
        .map_err(Error::Production)?;
    let base = base
        .wrapping_mul(defs.multiplier)
        .wrapping_mul(defs.gold_equivalent);
    let modifier = cell(
        &state
            .city(owner, city)
            .map_err(Error::State)?
            .gold
            .unit_cost_modifier,
        unit,
    )?;
    let bonus = if !defs.discount_defined || p.culture.anarchy_end_turn >= 0 {
        0
    } else {
        cell(&p.bonuses.flat_bonuses, 0x7530bd65)?
    };
    let mut cost = discounted(base, modifier.wrapping_add(bonus));
    if d.formation != hash(b"FORMATION_CLASS_CIVILIAN") {
        let m = p.units.favored_production_modifier_q8;
        if p.units.favored_production_yield == defs.gold_yield {
            cost = cost.wrapping_sub(
                c6_core::city_production::execution::multiply(m, cost.wrapping_shl(8)) >> 8,
            )
        } else if p.units.favored_production_yield == defs.production_yield {
            if m == 256 {
                cost = cell(&p.units.current_unit_cost, unit)?
                    .wrapping_mul(defs.multiplier)
                    .wrapping_mul(defs.gold_equivalent)
            } else {
                let denominator = 256i32.wrapping_sub(m);
                let q8 = ((i64::from(cost.wrapping_shl(8)) << 8) / i64::from(denominator)) as i32;
                cost = q8 >> 8;
            }
        }
    }
    Ok(cost.wrapping_sub(cost.wrapping_rem(defs.divisor)))
}
/// Purchase legality queries read the same canonical City, rosters and build maps.
pub fn can_place_unit<E>(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    city: u32,
    unit: u32,
) -> Result<bool, Error<E>> {
    defs.check(state)?;
    let d = defs.unit(unit)?;
    if d.stackable {
        return Ok(true);
    }
    if d.religion {
        return Err(Error::Missing("purchase religious spawn district"));
    }
    if d.domain != hash(b"DOMAIN_LAND") {
        return Err(Error::Missing("purchase nonland spawn domain"));
    }
    let c = state.city(owner, city).map_err(Error::State)?;
    if d.combat > 0 {
        for id in &c.districts.district_ids {
            let p = player(state, owner)?;
            let district = p
                .districts
                .records
                .entries
                .iter()
                .find(|v| v.value.id == *id)
                .ok_or(Error::Missing("purchase current District"))?;
            if district.value.district_hash != hash(b"DISTRICT_CITY_CENTER") {
                return Err(Error::Missing("purchase military district spawn selection"));
            }
        }
    }
    let location = c6_core::unit::PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    let index = crate::unit_placement_services::plot_index_at::<()>(state, location)
        .map_err(|_| Error::Missing("purchase City plot"))?
        .ok_or(Error::Missing("purchase valid City plot"))?;
    // Ordinary land definitions use GetDefaultMapLayer0. Religious/hero/special
    // layer profiles were excluded above rather than borrowing another roster.
    for reference in state
        .world()
        .plot_units(0, index as usize)
        .map_err(|_| Error::Missing("purchase live layered roster"))?
    {
        let Some(found) = state
            .world()
            .unit(*reference)
            .map_err(|_| Error::Missing("purchase current occupant"))?
        else {
            continue;
        };
        let other = defs.unit(found.record.e_type_index)?;
        // AreTheSameType26af8c0 compares domains, exempts traders, then uses the
        // actual formation class. No owner/friendliness guard exists at this leaf.
        if !d.trade && !other.trade && d.domain == other.domain && d.formation == other.formation {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Explicit typed callbacks for gold change, creation and the command tail.
/// Implementations must use the same stock backend; no default successful sink.
pub trait Effects {
    type Error: std::fmt::Debug;
    fn can_move_after_purchase(
        &self,
        state: &LoadedState,
        native_unit_index: usize,
    ) -> Result<bool, Self::Error>;
    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error>;
    fn create_unit(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        unit: u32,
        formation: i32,
    ) -> Result<Option<ComponentId>, Self::Error>;
    fn finish_moves(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), Self::Error>;
    fn purchased_report(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        unit: u32,
    ) -> Result<(), Self::Error>;
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), Self::Error>;
}
#[allow(clippy::too_many_arguments)]
pub fn purchase_unit<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    current: &selection::Current<'_>,
    owner: i32,
    city: u32,
    unit: u32,
    formation: i32,
    effects: &mut E,
) -> Result<Outcome, Error<E::Error>> {
    defs.check(state)?;
    if !selection::can_train(
        state,
        owner,
        city,
        unit,
        formation,
        &defs.selection,
        current,
    )
    .map_err(Error::Production)?
    {
        return Ok(Outcome::Rejected(Rejection::CannotTrain));
    }
    if !can_place_unit(state, defs, owner, city, unit)? {
        return Ok(Outcome::Rejected(Rejection::CannotPlace));
    }
    let d = defs.unit(unit)?;
    let p = player(state, owner)?;
    if d.formation != hash(b"FORMATION_CLASS_CIVILIAN")
        && p.units.banned_production_yield == defs.gold_yield
    {
        return Ok(Outcome::Rejected(Rejection::BannedYield));
    }
    let cost = unit_gold_cost(state, defs, owner, city, unit, formation)?;
    if cost > p.treasury.gold_balance_q8 >> 8 {
        return Ok(Outcome::Rejected(Rejection::InsufficientGold));
    }
    // This factory supports exactly these source-closed ordinary classes. Reject
    // missing positive work before deduction rather than manufacture a free Unit.
    if ![
        hash(b"UNIT_SCOUT"),
        hash(b"UNIT_WARRIOR"),
        hash(b"UNIT_BUILDER"),
    ]
    .contains(&unit)
    {
        return Err(Error::Missing("ordinary purchase Unit factory profile"));
    }
    if d.strategic {
        return Err(Error::Missing("purchase strategic resource consumption"));
    }
    if d.population_cost.is_some() {
        return Err(Error::Missing("purchase population-cost effects"));
    }
    let move_after = effects
        .can_move_after_purchase(state, d.native_index)
        .map_err(Error::Effect)?;
    let next = p
        .treasury
        .gold_balance_q8
        .saturating_add(cost.wrapping_neg().wrapping_shl(8));
    if p.treasury.gold_balance_q8 != next {
        state
            .treasury_mut(owner)
            .map_err(Error::State)?
            .gold_balance_q8 = next;
        effects.gold_changed(state, owner).map_err(Error::Effect)?;
    }
    let city = ComponentId {
        owner: owner as i16,
        component_type: 2,
        id: city,
    };
    let created = effects
        .create_unit(state, city, unit, if formation < 0 { 0 } else { formation })
        .map_err(Error::Effect)?;
    if let Some(created) = created {
        if !move_after {
            effects
                .finish_moves(state, created)
                .map_err(Error::Effect)?;
        }
        effects
            .purchased_report(state, city, unit)
            .map_err(Error::Effect)?;
    }
    effects
        .command_complete(state, city)
        .map_err(Error::Effect)?;
    Ok(Outcome::Purchased {
        unit: created,
        gold_cost: cost,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discount_applies_after_wrapping_product_and_clamps_before_rounding() {
        assert_eq!(discounted(120, 15), 102);
        assert_eq!(discounted(120, 100), 1);
        assert_eq!(discounted(120, 0), 120);
        assert_eq!(discounted(120, -10), 132);
        assert_eq!(discounted(i32::MAX, 50), 1);
    }
}
