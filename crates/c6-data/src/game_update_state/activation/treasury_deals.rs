//! Activation tax (2b99d30), followed by current ordered deal upkeep (2c75470).
//! Positive City/siphon and active-deal effects remain explicit until implemented.
use crate::{
    game_parameters::sql_integer, registry::LinuxDefinitionRegistry,
    unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_capabilities::Membership,
    game_update::Error,
    loaded_components::RetainedPlayer,
    loaded_state::{ActionStamp, LoadedState},
};

pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    maintenance: Vec<i32>,
    corps_q8: i32,
    army_q8: i32,
    gold_index: i32,
    park_gold: i32,
    buildings: Vec<(u32, i32, bool)>,
    districts: Vec<(u32, i32)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let units =
            collection(rules, "Units", "UnitType").map_err(|_| Error::Unsupported("tax Units"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("tax Units"))?;
        let maintenance = units
            .entries()
            .iter()
            .map(|entry| {
                table
                    .row_by_id(entry.database_rowid)
                    .and_then(|row| row.get("Maintenance").ok())
                    .and_then(sql_integer)
                    .ok_or(Error::Unsupported("Unit maintenance definition"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parameter = |name: &str| -> Result<i32, Error> {
            let table = rules
                .gameplay()
                .table("GlobalParameters")
                .ok_or(Error::Unsupported("tax parameters"))?;
            let row = table
                .rows()
                .find(|row| row.text("Name").ok() == Some(name))
                .ok_or(Error::Unsupported("tax cost modifier parameter"))?;
            let value = row
                .get("Value")
                .map_err(|_| Error::Unsupported("tax cost modifier value"))?;
            let value = match value {
                SqlValue::Integer(v) => *v as f32,
                SqlValue::RealBits(v) => f64::from_bits(*v) as f32,
                SqlValue::Text(v) => std::str::from_utf8(v)
                    .ok()
                    .and_then(|v| v.parse::<f32>().ok())
                    .ok_or(Error::Unsupported("tax float parameter"))?,
                SqlValue::Null => 0.,
                _ => return Err(Error::Unsupported("tax float parameter")),
            };
            if !value.is_finite() || value.abs() > 1000000. {
                return Err(Error::Unsupported("tax float range"));
            }
            // Native float constructor floors whole, then truncates fractional*256.
            let whole = value.floor();
            Ok((whole as i32).wrapping_shl(8) | (((value - whole) * 256.) as i32 & 255))
        };
        let integer = |row: crate::Row<'_>, column: &'static str| {
            row.get(column)
                .ok()
                .and_then(sql_integer)
                .ok_or(Error::Unsupported("tax city maintenance definition"))
        };
        let mut buildings = Vec::new();
        let building_registry = collection(rules, "Buildings", "BuildingType")
            .map_err(|_| Error::Unsupported("tax Buildings"))?;
        let building_table = rules
            .gameplay()
            .table("Buildings")
            .ok_or(Error::Unsupported("tax Buildings"))?;
        let xp = rules
            .gameplay()
            .table("Buildings_XP2")
            .ok_or(Error::Unsupported("tax Buildings_XP2"))?;
        for entry in building_registry.entries() {
            let row = building_table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("tax Building row"))?;
            let hash = crate::registry::linux_type_hash(&entry.type_name);
            let mut climate = false;
            if let Some(row) = xp.rows().find(|r| {
                r.text("BuildingType")
                    .ok()
                    .map(|v| crate::registry::linux_type_hash(v.as_bytes()))
                    == Some(hash)
            }) {
                climate = integer(row, "CostMultiplierPerTile")? > 0
                    && integer(row, "CostMultiplierPerSeaLevel")? > 0;
            }
            buildings.push((hash, integer(row, "Maintenance")?, climate));
        }
        let district_registry = collection(rules, "Districts", "DistrictType")
            .map_err(|_| Error::Unsupported("tax Districts"))?;
        let district_table = rules
            .gameplay()
            .table("Districts")
            .ok_or(Error::Unsupported("tax Districts"))?;
        let districts = district_registry
            .entries()
            .iter()
            .map(|e| {
                let row = district_table
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("tax District row"))?;
                Ok((
                    crate::registry::linux_type_hash(&e.type_name),
                    integer(row, "Maintenance")?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let yields = collection(rules, "Yields", "YieldType")
            .map_err(|_| Error::Unsupported("tax Yields"))?;
        let gold_index = yields
            .entries()
            .iter()
            .position(|e| e.type_name == b"YIELD_GOLD")
            .and_then(|v| i32::try_from(v).ok())
            .ok_or(Error::Unsupported("tax gold yield index"))?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            gold_index,
            park_gold: parameter("NATIONAL_PARK_GOLD_YIELD_PER_TOURISM")? >> 8,
            buildings,
            districts,
            maintenance,
            corps_q8: parameter("UNIT_CORPS_COST_MODIFIER")?,
            army_q8: parameter("UNIT_ARMY_COST_MODIFIER")?,
        })
    }
    fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() != &self.binding {
            Err(Error::Unsupported("tax definition binding"))
        } else {
            Ok(())
        }
    }
}
/// Same live manager owner, separate from pending negotiation pairs. IDs are
/// semantic native deal IDs; pointers are never manufactured as identities.
pub struct CurrentDeals {
    instance: ActionStamp,
    global: Vec<u32>,
    players: Vec<(i32, Vec<u32>)>,
}
impl CurrentDeals {
    pub fn from_initial(
        state: &LoadedState,
        global: Vec<u32>,
        players: Vec<(i32, Vec<u32>)>,
    ) -> Result<Self, Error> {
        if players
            .iter()
            .enumerate()
            .any(|(i, p)| p.0 < 0 || players[..i].iter().any(|q| q.0 == p.0))
        {
            return Err(Error::Unsupported("current deal owner buckets"));
        }
        Ok(Self {
            instance: state.action_stamp(),
            global,
            players,
        })
    }
    pub fn player_deals(&self) -> &[(i32, Vec<u32>)] {
        &self.players
    }
    pub fn global_deals(&self) -> &[u32] {
        &self.global
    }
    pub fn do_turn(&mut self, state: &LoadedState, owner: i32) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Unsupported("current deal binding"));
        }
        if owner >= 0 {
            let index = match self.players.iter().position(|p| p.0 == owner) {
                Some(i) => i,
                None => {
                    self.players.push((owner, vec![]));
                    self.players.len() - 1
                }
            };
            // EditPlayerDeals may insert its empty bucket BEFORE the first child.
            if !self.players[index].1.is_empty() {
                return Err(Error::Unsupported("active Deal.Instance.DoTurn"));
            }
        }
        if !self.global.is_empty() {
            return Err(Error::Unsupported("current global deal expiration"));
        }
        Ok(())
    }
}
pub struct Queries<'a> {
    pub capabilities: &'a Membership,
    pub cities: Option<crate::loaded_actions::stock_notifications::city_queries::Inputs<'a>>,
    pub city_order: Option<&'a [u32]>,
}
pub trait Effects {
    /// Source SetGoldBalance: report20f2116e THEN GameSignals+9f0. The stored
    /// balance is already changed; the signal argument is current balance SAR8.
    fn gold_changed(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("tax current player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("tax canonical player"))
}
fn treasury(
    state: &mut LoadedState,
    owner: i32,
) -> Result<&mut c6_core::player_treasury::Treasury, Error> {
    state
        .treasury_mut(owner)
        .map_err(|_| Error::Unsupported("tax canonical treasury"))
}
/// Current source no-city contributor path; checks other gameplay contributors
/// independently, including Economic.IsActive's actual capability gate.
pub fn gold_yield(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
) -> Result<i32, Error> {
    let p = player(state, owner)?;
    if p.culture.anarchy_end_turn >= 0 {
        return Ok(0);
    }
    let mut total = 0i32;
    for &city in ordered_cities(state, queries, owner)? {
        total = total.wrapping_add(city_gold(state, defs, queries, owner, city)?);
    }
    if defs.park_gold > 0 && !p.cities.records.entries.is_empty() {
        return Err(Error::Unsupported("current national park gold tourism"));
    }
    if p.religion.belief_yields.iter().any(|r| r.amount != 0) {
        return Err(Error::Unsupported("gold belief contributors"));
    }
    if p.influence
        .yield_per_given_token
        .iter()
        .chain(&p.influence.yield_per_tributary)
        .any(|r| r.1 != 0)
    {
        return Err(Error::Unsupported("gold envoy contributors"));
    }
    if !p.treasury.gold_diplomatic_deals.is_empty() {
        return Err(Error::Unsupported("gold diplomatic deal items"));
    }
    if p.diplomacy
        .yield_per_delegation
        .iter()
        .chain(&p.diplomacy.yield_per_embassy)
        .any(|r| r.value != 0)
    {
        return Err(Error::Unsupported("gold delegation/embassy contributors"));
    }
    if queries.capabilities.has(0x10c40410) {
        return Err(Error::Unsupported("Economic monopoly allowance/yields"));
    }
    if p.treasury.gold_interest_percent_q8 > 0 {
        return Err(Error::Unsupported("gold interest cap parameter"));
    }
    if p.influence
        .yield_modifier_per_tributary
        .iter()
        .chain(&p.great_people_points.yield_modifier_per_earned_great_person)
        .any(|r| r.1 != 0)
    {
        return Err(Error::Unsupported("gold tributary/GP yield modifiers"));
    }
    Ok(total.wrapping_add(p.treasury.gold_rate_change_q8))
}
fn ordered_cities<'a>(
    state: &LoadedState,
    queries: &Queries<'a>,
    owner: i32,
) -> Result<&'a [u32], Error> {
    let p = player(state, owner)?;
    if p.cities.records.entries.is_empty() {
        return Ok(&[]);
    }
    let order = queries
        .city_order
        .ok_or(Error::Unsupported("tax current City order"))?;
    if order.len() != p.cities.records.entries.len()
        || order.iter().enumerate().any(|(i, id)| {
            order[..i].contains(id)
                || !p
                    .cities
                    .records
                    .entries
                    .iter()
                    .any(|e| e.value.header.id == *id)
        })
    {
        return Err(Error::Unsupported("tax current City order membership"));
    }
    Ok(order)
}
fn city_gold(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
    city: u32,
) -> Result<i32, Error> {
    let input = queries
        .cities
        .ok_or(Error::Unsupported("tax current City yield services"))?;
    input
        .current
        .calculate(state, owner, city, defs.gold_index, 0, input.definitions)
        .map_err(|_| Error::Unsupported("tax current City gold yield"))
}
fn city_maintenance(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
) -> Result<i32, Error> {
    let p = player(state, owner)?;
    let mut total = 0i32;
    for &id in ordered_cities(state, queries, owner)? {
        let city = state
            .city(owner, id)
            .map_err(|_| Error::Unsupported("tax maintenance City"))?;
        for &(hash, base, climate) in &defs.buildings {
            let location = city
                .buildings
                .building_locations
                .iter()
                .rfind(|r| r.definition_hash == hash)
                .ok_or(Error::Unsupported("tax current building location"))?
                .value;
            if location < 0 {
                continue;
            }
            let pillaged = city
                .buildings
                .pillaged
                .iter()
                .rfind(|r| r.definition_hash == hash)
                .ok_or(Error::Unsupported("tax current building pillage"))?
                .value;
            if pillaged {
                continue;
            }
            if climate {
                return Err(Error::Unsupported("current flood-barrier maintenance"));
            }
            total = total.wrapping_add(base.wrapping_shl(8));
        }
        for &(hash, base) in &defs.districts {
            let instances = p
                .districts
                .records
                .entries
                .iter()
                .map(|e| &e.value)
                .filter(|d| d.city_id == id && d.district_hash == hash);
            if instances.clone().any(|d| d.is_complete) && instances.clone().any(|d| !d.pillaged) {
                total = total.wrapping_add(base.wrapping_shl(8));
            }
        }
    }
    Ok(total)
}
fn formation_cost(maintenance: i32, multiplier: i32) -> i32 {
    let q8 = c6_core::city_production::execution::multiply(maintenance.wrapping_shl(8), multiplier);
    (q8 >> 8).wrapping_add(((q8 as u32 >> 7) & 1) as i32)
}
fn maintenance(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
) -> Result<i32, Error> {
    let p = player(state, owner)?;
    let city_total = city_maintenance(state, defs, queries, owner)?;
    let discount = p.treasury.maintenance_discount_per_unit;
    let units = state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("maintenance owner"))?
        .unit_records()
        .map_err(|_| Error::Unsupported("maintenance current units"))?;
    let mut total = city_total;
    for (index, &base) in defs.maintenance.iter().enumerate() {
        if base <= 0 {
            continue;
        }
        for (formation, cost) in [
            (0, base),
            (1, formation_cost(base, defs.corps_q8)),
            (2, formation_cost(base, defs.army_q8)),
        ] {
            if cost <= discount {
                continue;
            }
            let mut count = 0i32;
            for slot in units {
                let unit = &slot.value;
                let runtime = state
                    .unit_runtime(owner, unit.id)
                    .map_err(|_| Error::Unsupported("maintenance Unit runtime"))?;
                if runtime.definition_index != index as i32 {
                    continue;
                }
                if defs.units.find_index(unit.e_type_index) != Some(index) {
                    return Err(Error::Unsupported("maintenance Unit definition binding"));
                }
                if unit.b_delayed_death || unit.b_is_levied {
                    continue;
                }
                if unit.e_military_formation == formation
                    || (formation == 0 && unit.e_military_formation == -1)
                {
                    count = count.wrapping_add(1);
                }
            }
            total = total.wrapping_add(
                cost.wrapping_sub(discount)
                    .wrapping_mul(count)
                    .wrapping_shl(8),
            );
        }
    }
    if p.wmds.weapons.iter().any(|w| w.count != 0) {
        return Err(Error::Unsupported("current WMD maintenance"));
    }
    if !p.treasury.gold_diplomatic_deals.is_empty() {
        return Err(Error::Unsupported("deal maintenance"));
    }
    Ok(total)
}
fn siphon_amount<E>(
    stored: i32,
    current_yield: impl FnOnce() -> Result<i32, E>,
) -> Result<Option<i32>, E> {
    if stored <= 0 {
        return Ok(None);
    }
    Ok(Some(stored.min(current_yield()? >> 8)))
}
fn set_balance(
    state: &mut LoadedState,
    effects: &mut impl Effects,
    owner: i32,
    value: i32,
) -> Result<(), Error> {
    if player(state, owner)?.treasury.gold_balance_q8 != value {
        treasury(state, owner)?.gold_balance_q8 = value;
        effects.gold_changed(state, owner)?;
    }
    Ok(())
}
pub fn tax(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    effects: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    if !queries.capabilities.has(0x798f719c) {
        return Ok(());
    }
    let gold = gold_yield(state, defs, queries, owner)?;
    treasury(state, owner)?.cached_net_income_q8 = gold;
    let mut siphoned = 0i32;
    for index in 0..ordered_cities(state, queries, owner)?.len() {
        let city = ordered_cities(state, queries, owner)?[index];
        let current = state
            .city(owner, city)
            .map_err(|_| Error::Unsupported("tax current siphon City"))?
            .gold
            .siphoned_gold;
        let Some(loss) = siphon_amount(current, || city_gold(state, defs, queries, owner, city))?
        else {
            continue;
        };
        siphoned = siphoned.wrapping_add(loss.wrapping_shl(8));
        let gold = &mut state
            .city_mut(owner, city)
            .map_err(|_| Error::Unsupported("tax current siphon store"))?
            .gold;
        gold.siphoned_gold = gold.siphoned_gold.wrapping_sub(loss);
    }
    treasury(state, owner)?.cached_net_income_q8 = gold.wrapping_sub(siphoned);
    let cost = maintenance(state, defs, queries, owner)?;
    let net = player(state, owner)?
        .treasury
        .cached_net_income_q8
        .wrapping_sub(cost);
    treasury(state, owner)?.cached_net_income_q8 = net;
    let excluding = gold_yield(state, defs, queries, owner)?; // source fresh query, not net
    treasury(state, owner)?.cached_net_income_excluding_trade_q8 = excluding;
    let balance = player(state, owner)?
        .treasury
        .gold_balance_q8
        .saturating_add(net);
    set_balance(state, effects, owner, balance)?;
    if player(state, owner)?.treasury.gold_balance_q8 < 0 {
        return Err(Error::Unsupported("bankruptcy penalties"));
    }
    treasury(state, owner)?.bankruptcy_amenities = 0;
    Ok(())
}
pub fn run(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    deals: &mut CurrentDeals,
    effects: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    tax(state, defs, queries, effects, owner)?;
    deals.do_turn(state, owner)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_formation_cost_rounds_half_up_and_wraps_q8() {
        assert_eq!(formation_cost(1, 384), 2);
        assert_eq!(formation_cost(3, 384), 5);
        assert_eq!(formation_cost(1, 383), 1);
        assert_eq!(formation_cost(i32::MAX, 256), -1);
    }
    #[test]
    fn siphon_skips_unneeded_yield_and_caps_to_whole_current_gold() {
        assert_eq!(siphon_amount(0, || Err::<i32, _>("not reached")), Ok(None));
        assert_eq!(siphon_amount(-1, || Err::<i32, _>("not reached")), Ok(None));
        assert_eq!(siphon_amount(5, || Ok::<_, ()>(2 * 256 + 255)), Ok(Some(2)));
        assert_eq!(siphon_amount(1, || Ok::<_, ()>(3 * 256)), Ok(Some(1)));
        assert_eq!(siphon_amount(1, || Ok::<_, ()>(-1)), Ok(Some(-1)));
    }
    #[test]
    fn clamped_balance_differs_from_wrapping_income() {
        assert_eq!(i32::MAX.saturating_add(256), i32::MAX);
        assert_eq!(i32::MAX.wrapping_add(256), i32::MIN + 255);
    }
}
