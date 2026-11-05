//! Linux Player.Units.DoEndTurn2ba79e0 over current canonical recycled records.
//! Finite ordinary no-fallout/no-water-damage, nonhero units; reached unsupported
//! effects fail with the executed prefix and gathered delayed list retained.
use crate::{registry::linux_type_hash, unit_placement_state::round as shared, Rules, SqlValue};
use c6_core::{
    loaded_state::{ActionStamp, LoadedState},
    map::UnitReference,
    unit::{
        variants::{
            ArrayVariant, KeyedHeader, TypedEntry, TypedVariant, TypedVariantMap, VariantPayload,
        },
        Unit,
    },
    unit_healing::{
        HealingCity, HealingDomain, HealingGovernor, HealingPlot, HealingResource,
        HealingTerritory, HealingWorldState,
    },
};

#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Rules(crate::Error),
    Placement(String),
    Unsupported {
        actor: (i32, u32),
        operation: &'static str,
    },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round units: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Preserve concrete reached gameplay failures, including partial damage writes.
#[derive(Debug)]
pub enum TransitionError<E> {
    Input(Error),
    Movement(c6_core::improvement_movement::Error<crate::unit_placement_state::Error<E>>),
    Healing(c6_core::unit_healing::UnitHealingError<crate::unit_placement_state::Error<E>>),
}
impl<E> From<Error> for TransitionError<E> {
    fn from(value: Error) -> Self {
        Self::Input(value)
    }
}

pub struct Inputs<'a, 'd, 'r> {
    pub rules: &'a Rules,
    pub resources: &'a crate::resource_binding::ResourceAuthority<'r>,
    /// Same CURRENT fallout manager plot order used by city turn upkeep.
    pub fallout_turns: &'a [i32],
    pub placement: shared::Inputs<'a, 'd, 'r>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    Advanced,
    RequiresDestroy,
    Complete,
}
#[derive(Debug)]
#[must_use = "all unit effects and the final gathered Destroy list remain mandatory"]
pub struct Frame {
    owner: i32,
    cursor: Option<i32>,
    delayed: Vec<(i32, u32)>,
    identity: ActionStamp,
    failed: bool,
    water_base: Option<i32>,
    finished: bool,
}
impl Frame {
    pub fn delayed(&self) -> &[(i32, u32)] {
        &self.delayed
    }
    /// Consume only after traversal. These require Player.Units.Destroy, not a
    /// second delayed Kill. The caller owns their exact immediate removal order.
    pub fn into_delayed(self, state: &LoadedState) -> Result<Vec<(i32, u32)>, Error> {
        if self.failed || !self.finished || !self.identity.same_instance(state.action_stamp()) {
            return Err(Error::Missing("round delayed Destroy continuation"));
        }
        Ok(self.delayed)
    }
}
fn actor(state: &LoadedState, a: (i32, u32)) -> Result<&Unit, Error> {
    state
        .world()
        .unit(UnitReference {
            owner: a.0,
            id: a.1,
        })
        .map_err(Error::World)?
        .map(|u| u.record)
        .ok_or(Error::Missing("round current Unit.Find"))
}
pub fn begin(state: &LoadedState, owner: i32) -> Result<Frame, Error> {
    let records = state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .unit_records()
        .map_err(Error::World)?;
    Ok(Frame {
        owner,
        cursor: records.first().map(|s| s.index),
        delayed: vec![],
        identity: state.action_stamp(),
        failed: false,
        water_base: None,
        finished: false,
    })
}
fn int_property(properties: &TypedVariantMap, key: u32) -> Result<Option<i32>, Error> {
    let Some(entry) = properties.entries.iter().find(|e| e.key == key) else {
        return Ok(None);
    };
    if entry.value.type_bits != 2
        || entry.value.modifier_bits != 0
        || entry.value.value.count_flags != 0
        || entry.value.value.element_size != 0
    {
        return Err(Error::Missing(
            "round native property integer conversion branch",
        ));
    }
    match entry.value.value.payload {
        VariantPayload::Inline32(v) => Ok(Some(v as i32)),
        _ => Err(Error::Missing("round property scalar storage")),
    }
}
fn set_integer(properties: &mut TypedVariantMap, key: u32, value: i32) {
    let scalar = TypedVariant {
        type_bits: 2,
        modifier_bits: 0,
        value: ArrayVariant {
            count_flags: 0,
            element_size: 0,
            payload: VariantPayload::Inline32(value as u32),
        },
    };
    if let Some(e) = properties.entries.iter_mut().find(|e| e.key == key) {
        e.value = scalar;
        return;
    }
    let name = match properties.header {
        KeyedHeader::Prefix(v) if v & 1 != 0 => Some(vec![]),
        _ => None,
    };
    let index = properties.entries.partition_point(|e| e.key < key);
    properties.entries.insert(
        index,
        TypedEntry {
            key,
            name,
            value: scalar,
        },
    );
}
fn water_base(rules: &Rules) -> Result<i32, Error> {
    crate::game_parameters::Parameters::from_rules(rules)
        .and_then(|p| p.required_integer("PLOT_WATER_DAMAGE_BASE"))
        .map_err(|_| Error::Missing("present PLOT_WATER_DAMAGE_BASE native parameter"))
}

/// Advance one native live-chain node. On error the frame is terminal; no
/// partially executed unit can be replayed. Delayed units are still inspectable.
pub fn advance<E: shared::Effects>(
    state: &mut LoadedState,
    frame: &mut Frame,
    inputs: &Inputs<'_, '_, '_>,
    runtime: &mut crate::unit_sight_state::Runtime,
    effects: &mut E,
) -> Result<Progress, TransitionError<crate::unit_placement_state::EffectsError<E>>> {
    if frame.failed || frame.finished || !frame.identity.same_instance(state.action_stamp()) {
        return Err(Error::Missing("round unit cursor state").into());
    }
    let result = (|| {
        let Some(slot) = frame.cursor else {
            frame.finished = true;
            return Ok(if frame.delayed.is_empty() {
                Progress::Complete
            } else {
                Progress::RequiresDestroy
            });
        };
        // Size()==0 returns before querying water damage. Nonempty managers
        // query it even when every entry is delayed or barbarian.
        if inputs.resources.destination_binding() != state.binding() {
            return Err(Error::Missing("round current resource binding").into());
        }
        inputs
            .resources
            .validate_rules(inputs.rules)
            .map_err(|_| Error::Missing("round current rules binding"))?;
        let base = match frame.water_base {
            Some(v) => v,
            None => {
                let v = water_base(inputs.rules)?;
                frame.water_base = Some(v);
                v
            }
        };
        let a = {
            let records = state
                .world()
                .player(frame.owner)
                .map_err(Error::World)?
                .unit_records()
                .map_err(Error::World)?;
            let u = &records
                .iter()
                .find(|s| s.index == slot)
                .ok_or(Error::Missing("round current live node"))?
                .value;
            (u.e_owner, u.id)
        };
        if actor(state, a)?.b_delayed_death {
            frame.delayed.push(a);
        } else {
            let barbarian = inputs
                .placement
                .queries
                .civilization_levels
                .and_then(|v| v.iter().find(|p| p.0 == frame.owner))
                .map(|p| p.1 == 0xd959ec24)
                .ok_or(Error::Missing("round current civilization level"))?;
            if !barbarian {
                ordinary(state, a, base, inputs, runtime, effects)?;
            }
        }
        let records = state
            .world()
            .player(frame.owner)
            .map_err(Error::World)?
            .unit_records()
            .map_err(Error::World)?;
        let index = records
            .iter()
            .position(|s| s.index == slot)
            .ok_or(Error::Missing(
                "round source iterator node removed by callback",
            ))?;
        frame.cursor = records.get(index + 1).map(|s| s.index);
        Ok(Progress::Advanced)
    })();
    if result.is_err() {
        frame.failed = true;
    }
    result
}
fn ordinary<E: shared::Effects>(
    state: &mut LoadedState,
    a: (i32, u32),
    base: i32,
    inputs: &Inputs<'_, '_, '_>,
    runtime: &mut crate::unit_sight_state::Runtime,
    effects: &mut E,
) -> Result<(), TransitionError<crate::unit_placement_state::EffectsError<E>>> {
    let location = actor(state, a)?.k_location;
    let plot_index =
        crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(state, location)
            .map_err(|_| Error::Missing("round current plot lookup"))?;
    if let Some(index) = plot_index {
        let fallout = *inputs
            .fallout_turns
            .get(index as usize)
            .ok_or(Error::Missing("round current fallout plot"))?;
        if fallout > 0 {
            let plot = state
                .world()
                .map()
                .plot_by_index(index)
                .ok_or(Error::Missing("round fallout plot"))?;
            let expiry = plot
                .properties
                .as_ref()
                .map(|p| int_property(p, 0x170a779f))
                .transpose()?
                .flatten();
            // The current-or-future protection turn bypasses fallout damage
            // and its eligibility clear, before the independent water branch.
            if !expiry.is_some_and(|turn| turn >= state.retained().core.current_turn) {
                return Err(Error::Unsupported {
                    actor: a,
                    operation: "unprotected fallout damage/resistance",
                }
                .into());
            }
        }
        if base != 0 {
            let plot = state
                .world()
                .map()
                .plot_by_index(index)
                .ok_or(Error::Missing("round plot"))?;
            let context = inputs
                .placement
                .queries
                .placement_context(state, inputs.placement.definitions.placement)
                .map_err(|e| Error::Placement(format!("{e:?}")))?;
            if context
                .water(plot)
                .map_err(|e| Error::Placement(format!("{e:?}")))?
            {
                return Err(Error::Unsupported {
                    actor: a,
                    operation: "water damage/resistance",
                }
                .into());
            }
        }
    }
    if int_property(&actor(state, a)?.k_properties, 0xb16c65f9)?.is_some() {
        return Err(Error::Unsupported {
            actor: a,
            operation: "lifespan decrement/expiry",
        }
        .into());
    }
    let previous = int_property(&actor(state, a)?.k_properties, 0xa281ca07)?;
    if plot_index.is_some_and(|index| previous != Some(index as i32)) {
        shared::improvement_damage(state, &inputs.placement, runtime, effects, a)
            .map_err(TransitionError::Movement)?;
    }
    let marker = int_property(&actor(state, a)?.k_properties, 0xa281ca07)?;
    if plot_index.is_some_and(|index| marker == Some(index as i32)) {
        state
            .unit_mut(a.0, a.1)
            .map_err(Error::State)?
            .b_eligible_to_heal = false;
    }
    set_integer(
        &mut state.unit_mut(a.0, a.1).map_err(Error::State)?.k_properties,
        0xa281ca07,
        -1,
    );
    if actor(state, a)?.k_hero.e_class != u32::MAX {
        return Err(Error::Unsupported {
            actor: a,
            operation: "Hero.DoEndTurn",
        }
        .into());
    }
    let world = healing_world(state, a, inputs)?;
    let rules = crate::game_parameters::Parameters::from_rules(inputs.rules)
        .map_err(|_| Error::Missing("round healing parameters"))?
        .healing_rules()
        .map_err(|_| Error::Missing("round healing parameters"))?;
    shared::heal(state, &inputs.placement, runtime, effects, a, world, rules)
        .map_err(TransitionError::Healing)?;
    let unit = actor(state, a)?;
    let human = state.player_slot_status(a.0).map_err(Error::State)? == 3;
    let end_operation = match unit.e_activity as u32 {
        0x047c048b => human || unit.i_fortify_turns == 0,
        0xb29e7722 => !human || unit.e_automation != -1 || unit.i_damage == 0,
        _ => false,
    };
    if end_operation {
        return Err(Error::Unsupported {
            actor: a,
            operation: "RequestEndOperation reason6b25012e",
        }
        .into());
    }
    Ok(())
}

/// Actual native territory/owner/Unit/Units/Governors and selected definition
/// reads for the finite ordinary land, nonreligious, no-strategic-resource path.
/// Unsupported reached branches stay explicit, with no fabricated getter result.
pub fn healing_world(
    state: &LoadedState,
    a: (i32, u32),
    inputs: &Inputs<'_, '_, '_>,
) -> Result<HealingWorldState, Error> {
    let unit = actor(state, a)?;
    let index = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
        state,
        unit.k_location,
    )
    .map_err(|_| Error::Missing("healing plot lookup"))?;
    let plot = index.and_then(|i| state.world().map().plot_by_index(i));
    let owner = plot
        .map(crate::movement_placement_state::plot_owner)
        .transpose()
        .map_err(|e| Error::Placement(format!("{e:?}")))?
        .unwrap_or(-1);
    let territory = if a.0 == -1 || plot.is_none() {
        HealingTerritory::Other
    } else if owner == -1 {
        HealingTerritory::Neutral
    } else if owner == a.0 {
        HealingTerritory::Friendly
    } else {
        return Err(Error::Unsupported {
            actor: a,
            operation: "healing suzerain/alliance territory",
        });
    };
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == a.0)
        .ok_or(Error::Missing("healing player"))?;
    let make_world = |plot| HealingWorldState {
        territory,
        no_foreign_heal: player.units.no_foreign_heal,
        no_neutral_heal: player.units.no_neutral_heal,
        resource: HealingResource::NoStrategicResource,
        domain: HealingDomain::Other,
        embarked: unit.b_embarked,
        plot,
    };
    // Native computes territory first but queries definitions/resources only
    // after these actor guards; none of the placeholder fields below are read.
    if !unit.b_eligible_to_heal || unit.b_delayed_death || plot.is_none() {
        return Ok(make_world(None));
    }
    let table = inputs
        .rules
        .gameplay()
        .table("Units")
        .ok_or(Error::Missing("Units"))?;
    let row = table
        .rows()
        .find(|r| {
            r.text("UnitType")
                .is_ok_and(|name| linux_type_hash(name.as_bytes()) == unit.e_type_index)
        })
        .ok_or(Error::Missing("healing Unit definition"))?;
    match row.get("StrategicResource").map_err(Error::Rules)? {
        SqlValue::Null => {}
        SqlValue::Text(v) if v.is_empty() => {}
        _ => {
            return Err(Error::Unsupported {
                actor: a,
                operation: "healing strategic resource access",
            })
        }
    }
    if row.text("Domain").map_err(Error::Rules)? != "DOMAIN_LAND"
        || unit.k_religion.i_religious_strength > 0
    {
        return Err(Error::Unsupported {
            actor: a,
            operation: "nonland/religious healing",
        });
    }
    let plot = plot.expect("checked");
    let district = inputs
        .placement
        .definitions
        .movement
        .district_at(state, plot)
        .map_err(|e| Error::Placement(format!("{e:?}")))?;
    let is_city = district
        .map(|d| inputs.placement.definitions.movement.district_is_city(d))
        .transpose()
        .map_err(|e| Error::Placement(format!("{e:?}")))?
        .unwrap_or(false);
    let purchased = if owner == a.0 {
        plot.city_attributes
            .as_ref()
            .filter(|p| p.owning_city != u32::MAX)
            .map(|p| {
                state.city(owner, p.owning_city).map_err(Error::State)?;
                let governor = player.governors.entries.iter().find(|g| {
                    g.governor.assigned_city.owner as i32 == owner
                        && g.governor.assigned_city.component_type == 2
                        && g.governor.assigned_city.id == p.owning_city
                });
                Ok::<_, Error>(HealingCity {
                    extra_religious_heal: 0,
                    assigned_governor: governor.map(|g| HealingGovernor {
                        established: g.governor.established,
                        extra_heal: g.governor.extra_heal,
                    }),
                })
            })
            .transpose()?
    } else {
        None
    };
    Ok(make_world(Some(HealingPlot {
        owned_by_unit_owner: owner == a.0,
        is_city,
        improvement: None,
        purchased_by_city: purchased,
        district_religion_heal_rate: 0,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_marker_set_preserves_other_properties_and_wire_order() {
        let mut properties = TypedVariantMap {
            header: KeyedHeader::Prefix(0x8000_0001),
            entries: vec![],
        };
        set_integer(&mut properties, u32::MAX, 73);
        set_integer(&mut properties, 1, -19);
        let untouched = properties.entries.clone();
        set_integer(&mut properties, 0xa281ca07, -1);
        assert_eq!(
            properties.entries.iter().map(|p| p.key).collect::<Vec<_>>(),
            vec![1, 0xa281ca07, u32::MAX]
        );
        assert_eq!(properties.entries[0], untouched[0]);
        assert_eq!(properties.entries[2], untouched[1]);
        assert_eq!(int_property(&properties, 0xa281ca07).unwrap(), Some(-1));
        let bytes = properties
            .encode(c6_core::unit::variants::Limits::default())
            .unwrap()
            .0;
        let decoded = TypedVariantMap::decode_prefix(
            &bytes,
            &mut c6_core::unit::variants::DecodeBudget::new(
                c6_core::unit::variants::Limits::default(),
            ),
        )
        .unwrap()
        .0;
        assert_eq!(decoded, properties);
        set_integer(&mut properties, 0xa281ca07, 41);
        assert_eq!(properties.entries.len(), 3);
        assert_eq!(int_property(&properties, 0xa281ca07).unwrap(), Some(41));
        assert_eq!(int_property(&properties, 99).unwrap(), None);
    }
    #[test]
    fn unsupported_property_conversion_does_not_become_absence_or_zero() {
        let mut p = TypedVariantMap::default();
        set_integer(&mut p, 0xb16c65f9, 4);
        p.entries[0].value.type_bits = 3;
        assert!(matches!(
            int_property(&p, 0xb16c65f9),
            Err(Error::Missing(
                "round native property integer conversion branch"
            ))
        ));
        assert_eq!(p.entries[0].value.type_bits, 3);
    }
}
