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
    strategic: bool,
    luxury: bool,
    artifact: bool,
    improvements: Vec<u32>,
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
    visibility: crate::city_turn::resources::Definitions,
    districts: Vec<(u32, bool)>,
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
            let row = rules
                .gameplay()
                .table("Resources")
                .and_then(|t| t.row_by_id(e.database_rowid))
                .ok_or("upgrade Resource row")?;
            let class = row
                .text("ResourceClassType")
                .map_err(|e| format!("{e:?}"))?;
            resources.push(Resource {
                hash: hash(&e.type_name),
                strategic: hash(class.as_bytes()) == hash(b"RESOURCECLASS_STRATEGIC"),
                luxury: hash(class.as_bytes()) == hash(b"RESOURCECLASS_LUXURY"),
                artifact: class == "RESOURCECLASS_ARTIFACT",
                improvements: rules
                    .gameplay()
                    .table("Improvement_ValidResources")
                    .ok_or("Improvement_ValidResources")?
                    .rows()
                    .filter(|r| {
                        r.text("ResourceType").ok().map(str::as_bytes)
                            == Some(e.type_name.as_slice())
                    })
                    .map(|r| {
                        r.text("ImprovementType")
                            .map(|name| hash(name.as_bytes()))
                            .map_err(|e| e.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?,
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
            visibility: crate::city_turn::resources::Definitions::from_rules(rules)
                .map_err(|e| e.to_string())?,
            districts: rules
                .gameplay()
                .table("Districts")
                .ok_or("Districts")?
                .rows()
                .map(|row| {
                    Ok((
                        hash(
                            row.text("DistrictType")
                                .map_err(|e| e.to_string())?
                                .as_bytes(),
                        ),
                        row.get("InternalOnly")
                            .ok()
                            .and_then(sql_integer)
                            .ok_or("District InternalOnly")?
                            != 0,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?,
            luxuries: crate::luxury_allocation::Definitions::from_rules(state, rules)
                .map_err(|e| format!("{e:?}"))?,
        })
    }
}
/// A source-qualified current ResourceChanged subscription. Native addresses
/// are diagnostic identities; the receiver is the canonical Player owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Listener {
    Unknown(u64),
    PlayerEventManagerResourcesChanged { token: u64, receiver_owner: i32 },
}
/// Ordered current ResourceChanged subscriptions. Unknown consumers remain
/// explicit errors even when qualified consumers use an effect-free branch.
pub struct Signals {
    binding: WireBinding,
    instance: ActionStamp,
    listeners: Vec<Listener>,
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
            listeners: listeners.into_iter().map(Listener::Unknown).collect(),
        })
    }
    /// Bind source-qualified callable/member/receiver identities from the same
    /// initial snapshot as the state. This does not certify unknown callables.
    pub fn from_current(
        state: &LoadedState,
        binding: &WireBinding,
        listeners: Vec<Listener>,
    ) -> Result<Self, String> {
        if state.binding() != binding {
            return Err("upgrade resource signal binding".into());
        }
        for listener in &listeners {
            if let Listener::PlayerEventManagerResourcesChanged { receiver_owner, .. } = listener {
                receiver(state, *receiver_owner)?;
            }
        }
        Ok(Self {
            binding: binding.clone(),
            instance: state.action_stamp(),
            listeners,
        })
    }
}
fn receiver(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, String> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or_else(|| "upgrade ResourceChanged receiver Player".into())
}
// Influence's own-owner branch forwards resource changes to a suzerain and
// copy recipients only under these guards. No hidden callback or RNG occurs
// when this predicate is false; the source returns after these tests.
fn forwards(
    def: &Resource,
    suzerain: i32,
    strategics: &[i32],
    luxuries: &[i32],
    owner: i32,
    delta: i32,
) -> bool {
    if delta == 0 {
        return false;
    }
    if suzerain != -1 && (!def.accumulates || delta > 0) {
        return true;
    }
    if def.strategic
        && def.accumulates
        && delta > 0
        && strategics
            .iter()
            .enumerate()
            .any(|(i, n)| *n > 0 && i as i32 != owner)
    {
        return true;
    }
    def.luxury && luxuries.iter().any(|n| *n > 0)
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
    for listener in &signals.listeners {
        match listener {
            Listener::Unknown(token) => {
                return Err(format!(
                    "upgrade unknown ResourceChanged listener {token:#x}"
                ))
            }
            Listener::PlayerEventManagerResourcesChanged { receiver_owner, .. } => {
                let p = receiver(state, *receiver_owner)?;
                // Checking the own-owner branch is conservative here because
                // this preflight has no actor parameter. Actual dispatch below
                // reads the current changed and receiving owner separately.
                if forwards(
                    def,
                    p.influence.suzerain,
                    &p.influence.copy_strategics_count,
                    &p.influence.copy_luxuries_count,
                    *receiver_owner,
                    amount.wrapping_neg(),
                ) {
                    return Err("upgrade ResourceChanged Influence forwarding".into());
                }
            }
        }
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
    change_owned_amount(
        state,
        backend,
        inputs.definitions,
        inputs.signals,
        owner,
        resource,
        amount.wrapping_neg(),
    )?;
    let def = inputs
        .definitions
        .resources
        .iter()
        .find(|r| r.hash == resource)
        .ok_or("upgrade resource definition")?;
    // The fixed stock headless reporting scope has no added report consumers.
    // Source guards precede all climate state writes and every RNG dependency.
    if amount > 0 && def.co2.is_some_and(|v| v > 0) {
        return Err("upgrade positive Climate.ConsumeResource usage3".into());
    }
    Ok(())
}
/// Original ChangeResourceAmount(ownership=0), including luxury allocation,
/// source-ordered ResourceChanged consumers and semantic resource report.
/// The caller retains the same current signal and resource owners as upgrades.
pub fn change_owned_amount<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    defs: &Definitions,
    signals: &Signals,
    owner: i32,
    resource: u32,
    delta: i32,
) -> Result<(), String> {
    backend
        .check(state)
        .map_err(|e| format!("upgrade resources: {e:?}"))?;
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
    *cell = amount_after(*cell, delta, def.accumulates);
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
    if state.binding() != &signals.binding || !signals.instance.same_instance(state.action_stamp())
    {
        return Err("upgrade resource signals binding".into());
    }
    for listener in &signals.listeners {
        match *listener {
            Listener::Unknown(token) => {
                return Err(format!(
                    "upgrade unsupported ResourceChanged listener {token:#x}, resource {index}"
                ))
            }
            Listener::PlayerEventManagerResourcesChanged { receiver_owner, .. } => {
                let p = receiver(state, receiver_owner)?;
                if receiver_owner == owner
                    && forwards(
                        def,
                        p.influence.suzerain,
                        &p.influence.copy_strategics_count,
                        &p.influence.copy_luxuries_count,
                        owner,
                        delta,
                    )
                {
                    return Err("upgrade ResourceChanged Influence forwarding".into());
                }
                backend
                    .reached
                    .push("ResourceChanged Player.EventManager Influence");
            }
        }
    }
    backend
        .turn_report_records
        .push(turn_reports::Record::Report {
            event: 0x1153df52,
            player: owner,
        });
    Ok(())
}
/// Resources.OnDistrictConstructed2b70b40 and HandleResourceChangeOnTile2b708c0.
/// Compute current/previous extraction on the same canonical plot before the
/// normal owned-resource mutation, luxury redistribution and ordered callbacks.
pub fn district_constructed<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    defs: &Definitions,
    signals: &Signals,
    owner: i32,
    district: ComponentId,
) -> Result<(), String> {
    if &defs.binding != state.binding() {
        return Err("district resource definitions binding".into());
    }
    if district.component_type != 3 {
        return Err("district resource component type".into());
    }
    let record = state
        .district(i32::from(district.owner), district.id)
        .map_err(|e| e.to_string())?;
    if i32::from(district.owner) != owner {
        return Ok(());
    }
    let internal = defs
        .districts
        .iter()
        .find(|(hash, _)| *hash == record.district_hash)
        .ok_or("district resource definition")?
        .1;
    if internal {
        return Ok(());
    }
    let plot = state
        .world()
        .map()
        .plot_at(record.location.0, record.location.1)
        .ok_or("district resource plot")?;
    let resource = plot.resource_hash;
    if resource == u32::MAX
        || plot.resource_count == 0
        || plot
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player))
            != owner
    {
        return Ok(());
    }
    if defs
        .visibility
        .visible_resource(state, owner, resource as i32)
        .map_err(|e| e.to_string())?
        == -1
    {
        return Ok(());
    }
    let definition = defs
        .resources
        .iter()
        .find(|r| r.hash == resource)
        .ok_or("district current resource definition")?;
    // With the completed District override the current resource is extractable.
    // The source's previous query supplies NO_DISTRICT, retaining the actual
    // nonpillaged improvement. Equal old/new resources exit HandleChange early.
    if definition.artifact {
        return Err("district construction artifact removal unsupported".into());
    }
    let previous = !plot.flags.get(c6_core::map::PlotFlag::ImprovementPillaged)
        && definition.improvements.contains(&plot.improvement_hash);
    if previous || definition.accumulates {
        return Ok(());
    }
    let amount = i32::from(plot.resource_count);
    change_owned_amount(state, backend, defs, signals, owner, resource, amount)
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
    fn influence_callbacks_preserve_ownership_and_forwarding_guards() {
        let iron = Resource {
            hash: 0,
            accumulates: true,
            co2: Some(0),
            strategic: true,
            luxury: false,
            artifact: false,
            improvements: vec![],
        };
        // Spending stockpiled Iron does not forward a loss even when both
        // suzerainty and strategic-copy relationships are present.
        assert!(!forwards(&iron, 1, &[0, 2], &[1], 0, -20));
        assert!(!forwards(&iron, 1, &[0, 2], &[1], 0, 0));
        assert!(forwards(&iron, 1, &[], &[], 0, 20));
        assert!(forwards(&iron, -1, &[0, 1], &[], 0, 20));
        assert!(!forwards(&iron, -1, &[1, 0], &[], 0, 20));
        let luxury = Resource {
            hash: 0,
            accumulates: false,
            co2: None,
            strategic: false,
            luxury: true,
            artifact: false,
            improvements: vec![],
        };
        assert!(forwards(&luxury, -1, &[], &[1], 0, -1));
        assert!(forwards(&luxury, 1, &[], &[], 0, -1));
    }
    #[test]
    fn strategic_payment_uses_wrapping_arithmetic_and_conditional_clamp() {
        assert_eq!(amount_after(25, -20, true), 5);
        assert_eq!(amount_after(5, -20, true), 0);
        assert_eq!(amount_after(5, -20, false), -15);
        assert_eq!(amount_after(i32::MAX, 1, true), 0);
        assert_eq!(amount_after(i32::MAX, 1, false), i32::MIN);
    }
}
