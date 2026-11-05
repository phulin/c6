//! District portion of stock Placement.ExecuteForcedUnitRebase.
//! Completion and current air capacity precede city, occupation and pairing
//! queries. Only pillaged or enemy-occupied hangars enter forced rebasing.
use crate::{resource_binding::ResourceAuthority, unit_initialization_definitions, Rules};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error {
    Binding,
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Missing(&'static str),
    AircraftContinuation { district: u32, capacity: i32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "forced district rebase: {self:?}")
    }
}
impl std::error::Error for Error {}
pub struct Definitions {
    binding: WireBinding,
    air_slots: Vec<(u32, i32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != authority.destination_binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        let registry =
            unit_initialization_definitions::collection(rules, "Districts", "DistrictType")
                .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Districts")
            .ok_or(Error::Missing("Districts"))?;
        let mut air_slots = Vec::with_capacity(registry.entries().len());
        for entry in registry.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("district row"))?;
            let value = i32::try_from(row.integer("AirSlots").map_err(Error::Rules)?)
                .map_err(|_| Error::Missing("district AirSlots"))?;
            air_slots.push((crate::registry::linux_type_hash(&entry.type_name), value));
        }
        Ok(Self {
            binding: state.binding().clone(),
            air_slots,
        })
    }
}
fn capacity<E>(
    complete: bool,
    modifier: i32,
    definition: impl FnOnce() -> Result<i32, E>,
) -> Result<Option<i32>, E> {
    if !complete {
        return Ok(None);
    }
    let value = definition()?.wrapping_add(modifier);
    Ok((value > 0).then_some(value))
}
/// Execute the reached ordinary district branch against the current canonical
/// roster. This does not inspect unit counts to infer absent aircraft effects.
/// It returns before city/occupation/pairing queries exactly when capacity <=0.
pub fn check_districts(
    state: &LoadedState,
    definitions: &Definitions,
    owner: i32,
) -> Result<(), Error> {
    if state.binding() != &definitions.binding {
        return Err(Error::Binding);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current district owner"))?;
    for entry in &player.districts.records.entries {
        let district = &entry.value;
        let positive = capacity(district.is_complete, district.air_slots_modifier, || {
            definitions
                .air_slots
                .iter()
                .find(|(hash, _)| *hash == district.district_hash)
                .map(|(_, value)| *value)
                .ok_or(Error::Missing("current district definition"))
        })?;
        if let Some(capacity) = positive {
            if !player
                .cities
                .records
                .entries
                .iter()
                .any(|c| c.value.header.id == district.city_id)
            {
                continue;
            }
            if !district.pillaged {
                // 2befa78..2befabf: an intact district only rebases when
                // IsOccupiedByEnemy is true. An exclusively own-unit plot
                // returns false before any aircraft/pairing traversal.
                let mut foreign = false;
                for p in &state.retained().players {
                    if p.header.owner == owner {
                        continue;
                    }
                    for slot in state
                        .world()
                        .player(p.header.owner)
                        .map_err(|_| Error::Missing("current rebase player"))?
                        .unit_records()
                        .map_err(|_| Error::Missing("current rebase units"))?
                    {
                        if (slot.value.k_location.x, slot.value.k_location.y) == district.location {
                            foreign = true;
                        }
                    }
                }
                if !foreign {
                    continue;
                }
                return Err(Error::Missing("current district enemy occupation"));
            }
            return Err(Error::AircraftContinuation {
                district: district.id,
                capacity,
            });
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_district_does_not_query_air_definition() {
        assert_eq!(
            capacity(false, i32::MAX, || Err::<i32, _>("not reached")),
            Ok(None)
        );
    }
    #[test]
    fn complete_center_and_negative_modifier_skip_aircraft_queries() {
        assert_eq!(capacity(true, 0, || Ok::<_, ()>(0)), Ok(None));
        assert_eq!(capacity(true, -3, || Ok::<_, ()>(2)), Ok(None));
    }
    #[test]
    fn current_modifier_and_native_wrapping_control_gate() {
        assert_eq!(capacity(true, 1, || Ok::<_, ()>(0)), Ok(Some(1)));
        assert_eq!(capacity(true, 1, || Ok::<_, ()>(i32::MAX)), Ok(None));
        assert_eq!(
            capacity(true, 0, || Err::<i32, _>("missing current type")),
            Err("missing current type")
        );
    }
}
