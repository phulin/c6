//! Strategic stockpile payment and trained-strength statistics for upgrades.
//! Positive climate emissions and unknown signal consumers remain explicit errors.
use crate::loaded_actions::stock_notifications::{turn_reports, Runtime};
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash as hash,
    unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{ActionStamp, LoadedState},
    unit::ComponentId,
};

struct Resource {
    hash: u32,
    accumulates: bool,
    co2: Option<i32>,
}
struct Unit {
    hash: u32,
    index: usize,
    sea: bool,
    military: bool,
}
pub struct Definitions {
    binding: WireBinding,
    resources: Vec<Resource>,
    units: Vec<Unit>,
    luxuries: crate::luxury_allocation::Definitions,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        let consumption = rules
            .gameplay()
            .table("Resource_Consumption")
            .ok_or("upgrade Resource_Consumption")?;
        let mut resources = Vec::new();
        for e in collection(rules, "Resources", "ResourceType")
            .map_err(|e| format!("{e:?}"))?
            .entries()
        {
            let row = consumption.rows().find(|r| {
                r.text("ResourceType").ok().map(str::as_bytes) == Some(e.type_name.as_slice())
            });
            let integer = |name| -> Result<Option<i32>, String> {
                row.map(|r| {
                    r.get(name)
                        .ok()
                        .and_then(sql_integer)
                        .ok_or_else(|| format!("upgrade {name}"))
                })
                .transpose()
            };
            resources.push(Resource {
                hash: hash(&e.type_name),
                accumulates: integer("Accumulate")?.is_some_and(|v| v != 0),
                co2: integer("CO2perkWh")?,
            });
        }
        let table = rules.gameplay().table("Units").ok_or("upgrade Units")?;
        let xp2 = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or("upgrade Units_XP2")?;
        let mut units = Vec::new();
        for (index, e) in collection(rules, "Units", "UnitType")
            .map_err(|e| format!("{e:?}"))?
            .entries()
            .iter()
            .enumerate()
        {
            let row = table
                .row_by_id(e.database_rowid)
                .ok_or("upgrade Unit row")?;
            let military = xp2
                .rows()
                .find(|r| {
                    r.text("UnitType").ok().map(str::as_bytes) == Some(e.type_name.as_slice())
                })
                .map(|r| {
                    r.get("CanFormMilitaryFormation")
                        .ok()
                        .and_then(sql_integer)
                        .map(|v| v != 0)
                        .ok_or("upgrade military flag")
                })
                .transpose()?
                .unwrap_or(true);
            units.push(Unit {
                hash: hash(&e.type_name),
                index,
                sea: row.text("Domain").map_err(|e| format!("{e:?}"))? == "DOMAIN_SEA",
                military,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            resources,
            units,
            luxuries: crate::luxury_allocation::Definitions::from_rules(state, rules)
                .map_err(|e| format!("{e:?}"))?,
        })
    }
}
/// Current GameSignals+1230 registration tokens. Unknown and nonempty lists
/// cannot be silently treated as absence of gameplay callbacks.
pub struct Signals {
    binding: WireBinding,
    instance: ActionStamp,
    listeners: Vec<u64>,
}
impl Signals {
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        listeners: Vec<u64>,
    ) -> Result<Self, String> {
        if state.binding() != binding {
            return Err("upgrade resource signal binding".into());
        }
        Ok(Self {
            binding: binding.clone(),
            instance: state.action_stamp(),
            listeners,
        })
    }
}
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub signals: &'a Signals,
    pub creation_runtime: &'a crate::unit_creation::Runtime,
    pub combat: &'a crate::formation_rules::Definitions,
}
fn amount_after(stored: i32, delta: i32, accumulates: bool) -> i32 {
    let next = stored.wrapping_add(delta);
    if accumulates {
        next.max(0)
    } else {
        next
    }
}
/// Validate the supported emissions and callback branch before destruction.
pub fn preflight_resource(
    state: &LoadedState,
    inputs: &Inputs<'_>,
    resource: u32,
    amount: i32,
) -> Result<(), String> {
    let signals = inputs.signals;
    if state.binding() != &inputs.definitions.binding
        || state.binding() != &signals.binding
        || !signals.instance.same_instance(state.action_stamp())
    {
        return Err("upgrade resource inputs binding".into());
    }
    let def = inputs
        .definitions
        .resources
        .iter()
        .find(|r| r.hash == resource)
        .ok_or("upgrade resource definition")?;
    if !signals.listeners.is_empty() {
        return Err("upgrade positive ResourceChanged listeners".into());
    }
    if amount > 0 && def.co2.is_some_and(|v| v > 0) {
        return Err("upgrade positive climate emissions".into());
    }
    Ok(())
}
/// ChangeResourceAmount(ownership=0,-amount), its callbacks, then climate usage3.
/// Callback failures retain the already stored strategic payment.
pub fn consume_resource<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    inputs: &Inputs<'_>,
    owner: i32,
    resource: u32,
    amount: i32,
) -> Result<(), String> {
    backend
        .check(state)
        .map_err(|e| format!("upgrade resources: {e:?}"))?;
    let defs = inputs.definitions;
    if &defs.binding != state.binding() {
        return Err("upgrade resource definitions binding".into());
    }
    let def = defs
        .resources
        .iter()
        .find(|r| r.hash == resource)
        .ok_or("upgrade resource definition")?;
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or("upgrade resource owner")?;
    let index = p
        .resources
        .resource_ownership
        .iter()
        .position(|r| r.resource_hash == resource)
        .ok_or("upgrade native resource index")?;
    let cell = state
        .player_resources_mut(owner)
        .map_err(|e| format!("{e:?}"))?
        .resource_ownership
        .get_mut(index)
        .and_then(|r| r.value.first_mut())
        .ok_or("upgrade owned resource cell")?;
    *cell = amount_after(*cell, amount.wrapping_neg(), def.accumulates);
    let uninitializing =
        crate::loaded_actions::notifications::processing::Backend::game_uninitializing(
            backend, state,
        )
        .map_err(|e| format!("upgrade resource game state: {e:?}"))?;
    if !uninitializing {
        crate::game_update_state::activation::cityless_managers::Effects::allocate_luxuries(
            backend,
            state,
            &defs.luxuries,
            owner,
        )
        .map_err(|e| format!("upgrade luxury allocation: {e:?}"))?;
    }
    let signals = inputs.signals;
    if state.binding() != &signals.binding || !signals.instance.same_instance(state.action_stamp())
    {
        return Err("upgrade resource signals binding".into());
    }
    if let Some(token) = signals.listeners.first() {
        return Err(format!(
            "upgrade unsupported ResourceChanged listener {token:#x}, resource {index}"
        ));
    }
    backend
        .turn_report_records
        .push(turn_reports::Record::Report {
            event: 0x1153df52,
            player: owner,
        });
    // The fixed stock headless reporting scope has no added report consumers.
    // Source guards precede all climate state writes and every RNG dependency.
    if amount > 0 && def.co2.is_some_and(|v| v > 0) {
        return Err("upgrade positive Climate.ConsumeResource usage3".into());
    }
    Ok(())
}
/// UnitUpgraded updates maxima in melee/ranged/bombard order, without changing
/// UnitsConstructed. Queries use the transferred new Unit and canonical state.
pub fn upgraded(
    state: &mut LoadedState,
    inputs: &Inputs<'_>,
    new: ComponentId,
) -> Result<(), String> {
    let owner = i32::from(new.owner);
    if state.binding() != &inputs.definitions.binding
        || state.binding() != &inputs.creation_runtime.binding
    {
        return Err("upgrade Stats binding".into());
    }
    let unit = super::actor::<()>(state, (owner, new.id)).map_err(|e| format!("{e:?}"))?;
    let def = inputs
        .definitions
        .units
        .iter()
        .find(|u| u.hash == unit.e_type_index)
        .ok_or("upgrade Stats Unit definition")?;
    if def.sea {
        return Ok(());
    }
    let map = inputs
        .creation_runtime
        .definition_properties
        .as_ref()
        .and_then(|maps| maps.iter().find(|m| m.native_unit_index == def.index))
        .ok_or("upgrade Stats definition properties")?;
    if map.values.entries.iter().any(|e| e.key == 0x36c125c9) {
        return Err("upgrade Stats strength exclusion property conversion".into());
    }
    if !def.military {
        return Ok(());
    }
    for mode in 0..3 {
        let unit = super::actor::<()>(state, (owner, new.id)).map_err(|e| format!("{e:?}"))?;
        let value = match mode {
            0 => inputs.combat.combat(state, unit),
            1 => inputs.combat.ranged(state, unit),
            _ => inputs.combat.bombard(state, unit),
        }
        .map_err(|e| format!("upgrade Stats strength: {e:?}"))?;
        let stats = state
            .player_stats_mut(owner)
            .map_err(|e| format!("{e:?}"))?;
        let cell = match mode {
            0 => &mut stats.max_melee_strength_trained,
            1 => &mut stats.max_ranged_strength_trained,
            _ => &mut stats.max_bombard_strength_trained,
        };
        *cell = (*cell).max(value);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strategic_payment_uses_wrapping_arithmetic_and_conditional_clamp() {
        assert_eq!(amount_after(25, -20, true), 5);
        assert_eq!(amount_after(5, -20, true), 0);
        assert_eq!(amount_after(5, -20, false), -15);
        assert_eq!(amount_after(i32::MAX, 1, true), 0);
        assert_eq!(amount_after(i32::MAX, 1, false), i32::MIN);
    }
}
