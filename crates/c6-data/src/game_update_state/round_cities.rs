//! Linux Player.Cities.DoEndTurn2a5b190 and City.Manager.Heal28b94f0.
//! Mutates canonical district garrison damage; city activation is a separate
//! phase. Required reports remain synchronous and unsupported siege consumers
//! stop before their first signal, retaining the already executed prefix.
use crate::{
    game_parameters::{IntegerParameter, Parameters},
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_components::RetainedPlayer,
    loaded_state::{self, ActionStamp, LoadedState},
    player_districts::District,
    unit::{ComponentId, PlotCoord},
};

pub const GARRISON: u32 = 0x5e97_d629;
pub const DAMAGE_CHANGED: u32 = 0xf74a_fc79;

#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    State(loaded_state::Error),
    District(loaded_state::district_initialization::DistrictError),
    Rules(crate::Error),
    /// Native next effect is Game.Signals+270(city), then the siege
    /// notification. Neither has been silently delivered or discarded.
    RequiresBesiegedCity {
        owner: i32,
        city: u32,
        district: u32,
    },
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round cities: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// Bound current district definition order and actual optional HitPoints.
/// Parameters use the existing Linux GlobalParameters initialization contract.
pub struct Definitions {
    binding: WireBinding,
    districts: LinuxDefinitionRegistry,
    hit_points: Vec<Option<i32>>,
    heal_garrison: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        resources: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        if state.binding() != resources.destination_binding() {
            return Err(Error::Missing("round cities resources/state binding"));
        }
        resources
            .validate_rules(rules)
            .map_err(|_| Error::Missing("round cities rules binding"))?;
        let districts =
            crate::unit_initialization_definitions::collection(rules, "Districts", "DistrictType")
                .map_err(|_| Error::Missing("round district definitions"))?;
        let table = rules
            .gameplay()
            .table("Districts")
            .ok_or(Error::Missing("Districts"))?;
        let hit_points = districts
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("round District row"))?;
                match row.get("HitPoints").map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    _ => Ok(Some(
                        i32::try_from(row.integer("HitPoints").map_err(Error::Rules)?)
                            .map_err(|_| Error::Missing("District HitPoints i32"))?,
                    )),
                }
            })
            .collect::<Result<Vec<_>, Error<std::convert::Infallible>>>()?;
        let heal_garrison = Parameters::from_rules(rules)
            .and_then(|p| p.integer(IntegerParameter::HealCityGarrison))
            .map_err(|_| Error::Missing("COMBAT_HEAL_CITY_GARRISON"))?;
        Ok(Self {
            binding: state.binding().clone(),
            districts,
            hit_points,
            heal_garrison,
        })
    }
    fn max_damage(&self, district: &District) -> i32 {
        // 291f6b0 returns zero for a null definition or invalid Optional<int>.
        self.districts
            .find_index(district.district_hash)
            .and_then(|index| self.hit_points.get(index))
            .copied()
            .flatten()
            .unwrap_or(0)
    }
}

pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    /// Same CURRENT fallout manager plot order used by unit/city turn upkeep.
    /// Missing cells remain unknown; no all-zero fallback is constructed.
    pub fallout_turns: &'a [i32],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageChanged {
    pub event: u32,
    pub defense: u32,
    pub district: ComponentId,
    pub damage: i32,
    pub previous_damage: i32,
}
pub trait Effects {
    type Error;
    /// Report Type(f74afc79,GARRISON), actual current component identity,
    /// new damage then old damage; execute every reached gameplay/RNG consumer.
    fn damage_changed(
        &mut self,
        state: &mut LoadedState,
        report: DamageChanged,
    ) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    Advanced {
        city: u32,
        district: Option<u32>,
        damage_changed: bool,
    },
    Complete,
}
#[derive(Debug)]
#[must_use = "the current city/district traversal and required reports remain mandatory"]
pub struct Frame {
    owner: i32,
    cursor: Option<i32>,
    city: Option<(u32, usize)>,
    identity: ActionStamp,
    failed: bool,
    complete: bool,
}
impl Frame {
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
}
fn player<E>(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error<E>> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Missing("round current city owner"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("round current retained city owner"))
}
fn city_at_slot<E>(
    state: &LoadedState,
    owner: i32,
    slot: i32,
) -> Result<&c6_core::city::City, Error<E>> {
    player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .find(|entry| entry.index == slot)
        .map(|entry| &entry.value)
        .ok_or(Error::Missing("round current live city node"))
}
/// Native low-slot Find2ae4290 ignores requested generation bits and treats
/// INVALID(-1) as null. Mutation subsequently uses the resolved full ID.
fn find_district(player: &RetainedPlayer, id: u32) -> Option<&District> {
    if id == u32::MAX {
        return None;
    }
    player
        .districts
        .records
        .entries
        .iter()
        .find(|entry| entry.index == (id & 0xffff) as i32)
        .map(|entry| &entry.value)
}
fn find_city(player: &RetainedPlayer, id: u32) -> Option<&c6_core::city::City> {
    if id == u32::MAX {
        return None;
    }
    player
        .cities
        .records
        .entries
        .iter()
        .find(|entry| entry.index == (id & 0xffff) as i32)
        .map(|entry| &entry.value)
}
fn district_plot<E>(state: &LoadedState, district: &District) -> Result<u32, Error<E>> {
    crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
        state,
        PlotCoord {
            x: district.location.0,
            y: district.location.1,
        },
    )
    .map_err(|_| Error::Missing("round District.GetPlot"))?
    .ok_or(Error::Missing("round district null plot"))
}
fn plot_owner<E>(state: &LoadedState, district: &District) -> Result<i32, Error<E>> {
    let index = district_plot(state, district)?;
    crate::movement_placement_state::plot_owner(
        state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(Error::Missing("round district plot"))?,
    )
    .map_err(|_| Error::Missing("round district plot owner"))
}
/// 291e6a0 resolves District.GetCity using the CURRENT plot owner and low-slot
/// city ID, then compares locations. Definition CityCenter is not this getter.
fn is_city_center<E>(state: &LoadedState, district: &District) -> Result<bool, Error<E>> {
    let owner = plot_owner(state, district)?;
    if owner == -1 {
        return Ok(false);
    }
    Ok(find_city(player(state, owner)?, district.city_id)
        .is_some_and(|city| (city.header.location_x, city.header.location_y) == district.location))
}
/// IsContaminated(type)2885f70 scans ALL current city district IDs of that
/// normalized definition type. Pointer overload2886000 tests falloutTurns>0.
fn contaminated<E>(
    state: &LoadedState,
    owner: i32,
    city: u32,
    kind: u32,
    inputs: &Inputs<'_>,
) -> Result<bool, Error<E>> {
    let Some(kind) = inputs.definitions.districts.find_index(kind) else {
        return Ok(false);
    };
    let owner = player(state, owner)?;
    let city = find_city(owner, city).ok_or(Error::Missing("round contamination city"))?;
    contamination_scan(
        &inputs.definitions.districts,
        kind,
        city.districts
            .district_ids
            .iter()
            .filter_map(|id| find_district(owner, *id)),
        |district| {
            let plot = district_plot(state, district)?;
            inputs
                .fallout_turns
                .get(plot as usize)
                .copied()
                .ok_or(Error::Missing("round current fallout cell"))
        },
    )
}
fn contamination_scan<'a, E>(
    definitions: &LinuxDefinitionRegistry,
    kind: usize,
    districts: impl IntoIterator<Item = &'a District>,
    mut remaining: impl FnMut(&District) -> Result<i32, Error<E>>,
) -> Result<bool, Error<E>> {
    for district in districts {
        if definitions.find_index(district.district_hash) == Some(kind) && remaining(district)? > 0
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// ChangeDamage(GARRISON,-heal,-1)291f840. Attacker -1 skips CanRaze policy;
/// positive delta (a negative configured heal) still sets LastTurnDamaged AFTER
/// the synchronous SetDamage report, including when clamping kept damage equal.
fn heal_district<E: Effects>(
    state: &mut LoadedState,
    manager_owner: i32,
    id: u32,
    defs: &Definitions,
    effects: &mut E,
) -> Result<bool, Error<E::Error>> {
    let district = state.district(manager_owner, id).map_err(Error::District)?;
    let old = district.garrison_damage;
    let (delta, damage) = damage_after_healing(old, defs.heal_garrison, defs.max_damage(district));
    let changed = old != damage;
    if changed {
        state
            .district_mut(manager_owner, id)
            .map_err(Error::District)?
            .garrison_damage = damage;
        let district = state.district(manager_owner, id).map_err(Error::District)?;
        let owner = plot_owner(state, district)?;
        let identity = ComponentId {
            owner: owner as i16,
            component_type: 3,
            id: district.id,
        };
        effects
            .damage_changed(
                state,
                DamageChanged {
                    event: DAMAGE_CHANGED,
                    defense: GARRISON,
                    district: identity,
                    damage,
                    previous_damage: old,
                },
            )
            .map_err(Error::Effect)?;
    }
    if delta > 0 {
        let turn = state.retained().core.current_turn;
        state
            .district_mut(manager_owner, id)
            .map_err(Error::District)?
            .last_turn_damaged = turn;
    }
    Ok(changed)
}
fn damage_after_healing(previous: i32, heal: i32, maximum: i32) -> (i32, i32) {
    let delta = heal.wrapping_neg();
    (delta, clamp_damage(previous.wrapping_add(delta), maximum))
}
fn clamp_damage(candidate: i32, maximum: i32) -> i32 {
    if candidate < 0 {
        0
    } else {
        candidate.min(maximum)
    }
}

pub fn begin(state: &LoadedState, owner: i32) -> Result<Frame, Error<std::convert::Infallible>> {
    let cursor = player(state, owner)?
        .cities
        .records
        .entries
        .first()
        .map(|entry| entry.index);
    Ok(Frame {
        owner,
        cursor,
        city: None,
        identity: state.action_stamp(),
        failed: false,
        complete: false,
    })
}
/// Process one current city district entry, preserving both native traversal
/// orders. Empty/missing-district paths make no damage/report calls. A callback
/// failure is terminal and retains all earlier writes; this frame cannot replay.
pub fn advance<E: Effects>(
    state: &mut LoadedState,
    frame: &mut Frame,
    inputs: &Inputs<'_>,
    effects: &mut E,
) -> Result<Progress, Error<E::Error>> {
    if frame.failed || frame.complete || !frame.identity.same_instance(state.action_stamp()) {
        return Err(Error::Missing("round city continuation state"));
    }
    let result = (|| loop {
        let Some(slot) = frame.cursor else {
            frame.complete = true;
            return Ok(Progress::Complete);
        };
        if state.binding() != &inputs.definitions.binding {
            return Err(Error::Missing("round city definitions binding"));
        }
        let city = city_at_slot(state, frame.owner, slot)?;
        let city_id = city.header.id;
        let (expected, index) = frame.city.get_or_insert((city_id, 0));
        if *expected != city_id {
            return Err(Error::Missing("round city current node replaced"));
        }
        let city_owner = city.header.owner;
        let Some(&requested) = city.districts.district_ids.get(*index) else {
            let entries = &player(state, frame.owner)?.cities.records.entries;
            let current = entries
                .iter()
                .position(|entry| entry.index == slot)
                .ok_or(Error::Missing("round city iterator node removed"))?;
            frame.cursor = entries.get(current + 1).map(|entry| entry.index);
            frame.city = None;
            continue;
        };
        let district = find_district(player(state, city_owner)?, requested);
        let resolved = district.map(|d| d.id);
        let mut changed = false;
        if let Some(district) = district {
            if district.under_siege {
                if is_city_center(state, district)? {
                    return Err(Error::RequiresBesiegedCity {
                        owner: city_owner,
                        city: city_id,
                        district: district.id,
                    });
                }
            } else if !contaminated(state, city_owner, city_id, district.district_hash, inputs)?
                && !district.pillaged
            {
                changed =
                    heal_district(state, city_owner, district.id, inputs.definitions, effects)?;
            }
        }
        frame.city = Some((city_id, index.wrapping_add(1)));
        return Ok(Progress::Advanced {
            city: city_id,
            district: resolved,
            damage_changed: changed,
        });
    })();
    if result.is_err() {
        frame.failed = true;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn registry() -> LinuxDefinitionRegistry {
        LinuxDefinitionRegistry::from_native_order(
            ["DISTRICT_CITY_CENTER", "DISTRICT_ENCAMPMENT"]
                .iter()
                .enumerate()
                .map(|(i, name)| crate::registry::DefinitionEntry {
                    database_rowid: crate::RulesRowId(i as i64),
                    type_name: name.as_bytes().to_vec(),
                })
                .collect(),
        )
    }
    fn district(id: u32, kind: u32) -> District {
        let mut d = c6_core::district_initialization::construct(
            c6_core::district_initialization::Definitions {
                yields: &[],
                great_person_classes: &[],
            },
        )
        .unwrap();
        d.id = id;
        d.district_hash = kind;
        d
    }
    #[test]
    fn contamination_checks_all_same_type_districts_in_source_order_only() {
        let registry = registry();
        let center = crate::registry::linux_type_hash(b"DISTRICT_CITY_CENTER");
        let entries = [
            district(7, center),
            district(4, 1),
            district(2, 0),
            district(1, center),
        ];
        let mut visited = vec![];
        let contaminated =
            contamination_scan::<std::convert::Infallible>(&registry, 0, &entries, |d| {
                visited.push(d.id);
                Ok(match d.id {
                    7 => -1,
                    2 => 1,
                    _ => panic!("native early return/type filter must avoid this query"),
                })
            })
            .unwrap();
        assert!(contaminated);
        assert_eq!(visited, [7, 2]);
    }
    #[test]
    fn contamination_does_not_invent_missing_fallout_cell_as_clean() {
        let entries = [district(0, 0)];
        let result =
            contamination_scan::<std::convert::Infallible>(&registry(), 0, &entries, |_| {
                Err(Error::Missing("actual fallout cell"))
            });
        assert!(matches!(result, Err(Error::Missing("actual fallout cell"))));
        assert!(
            !contamination_scan::<std::convert::Infallible>(&registry(), 0, &entries, |_| Ok(0))
                .unwrap()
        );
    }
    #[test]
    fn healing_clamps_and_preserves_wrapping_and_positive_delta_bookkeeping() {
        assert_eq!(damage_after_healing(5, 20, 200), (-20, 0));
        assert_eq!(damage_after_healing(0, 20, 200), (-20, 0));
        assert_eq!(damage_after_healing(190, -20, 200), (20, 200));
        assert_eq!(damage_after_healing(200, -20, 200), (20, 200));
        assert_eq!(damage_after_healing(i32::MAX, -1, 200), (1, 0));
        assert_eq!(damage_after_healing(1, i32::MIN, 200), (i32::MIN, 0));
        assert_eq!(damage_after_healing(0, -1, -7), (1, -7));
    }
}
