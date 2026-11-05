//! Current Player.Congress favor-per-turn query (without presentation steps).
//! Reads the same canonical players, formal relations, trade and Climate owner.
use super::super::round_climate;
use crate::{game_parameters::sql_integer, registry::linux_type_hash, Rules};
use c6_core::{
    core_player::initialization::WireBinding, game_update::Error,
    loaded_components::RetainedPlayer, loaded_state::LoadedState,
};
use std::collections::BTreeMap;

pub struct Definitions {
    binding: WireBinding,
    baseline: i32,
    governments: BTreeMap<u32, i32>,
    government_slot_count: usize,
    co2: Penalty,
    grievances: Penalty,
    grievance_start: i32,
}
#[derive(Clone, Copy)]
struct Penalty {
    divisor: i32,
    minimum: i32,
    maximum: i32,
}
impl Penalty {
    fn calculate(self, value: i32) -> Result<i32, Error> {
        let divided = value
            .checked_div(self.divisor)
            .ok_or(Error::Unsupported("favor signed division"))?;
        Ok(divided.max(self.minimum).min(self.maximum))
    }
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("favor GlobalParameters"))?;
        let parameter = |name: &str| -> Result<i32, Error> {
            parameters
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(sql_integer)
                .ok_or(Error::Unsupported("favor parameter"))
        };
        let penalty = |prefix: &str| -> Result<Penalty, Error> {
            Ok(Penalty {
                divisor: parameter(&format!("{prefix}_DIVISOR"))?,
                minimum: parameter(&format!("{prefix}_MINIMUM"))?,
                maximum: parameter(&format!("{prefix}_MAXIMUM"))?,
            })
        };
        let table = rules
            .gameplay()
            .table("Governments_XP2")
            .ok_or(Error::Unsupported("favor Governments_XP2"))?;
        let mut governments = BTreeMap::new();
        for row in table.rows() {
            governments.insert(
                linux_type_hash(
                    row.text("GovernmentType")
                        .map_err(|_| Error::Unsupported("favor government type"))?
                        .as_bytes(),
                ),
                row.integer("Favor")
                    .map_err(|_| Error::Unsupported("favor government amount"))?
                    as i32,
            );
        }
        Ok(Self {
            binding: state.binding().clone(),
            baseline: parameter("WORLD_CONGRESS_BASELINE_FAVOR_PER_TURN")?,
            governments,
            government_slot_count: crate::unit_initialization_definitions::collection(
                rules,
                "GovernmentSlots",
                "GovernmentSlotType",
            )
            .map_err(|_| Error::Unsupported("favor GovernmentSlots definition order"))?
            .entries()
            .len(),
            co2: penalty("FAVOR_CO2")?,
            grievances: penalty("FAVOR_GRIEVANCES")?,
            grievance_start: parameter("FAVOR_GRIEVANCES_START")?,
        })
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("favor current player"))
}
// GameAttribute's tracked Base accumulates integer constructors in Q8. This
// wrapping shift/add is observable even when the final result is an integer.
fn add(raw: &mut i32, value: i32) {
    *raw = raw.wrapping_add(value.wrapping_shl(8));
}

/// Native GovernmentSlot definition order; vacant policy positions count too.
/// Nonpositive per-slot modifiers are skipped before querying their count.
fn government_favor(
    mut amount: i32,
    modifiers: &[i32],
    active: &[c6_core::player_culture::GovernmentSlot],
    definition_count: usize,
) -> Result<i32, Error> {
    for index in 0..definition_count {
        let extra = *modifiers
            .get(index)
            .ok_or(Error::Unsupported("favor current government slot index"))?;
        if extra <= 0 {
            continue;
        }
        let count = active
            .iter()
            .filter(|slot| slot.slot_ordinal == index as i32)
            .fold(0i32, |count, _| count.wrapping_add(1));
        amount = amount.wrapping_add(extra.wrapping_mul(count));
    }
    Ok(amount)
}

pub fn calculate(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    climate: &round_climate::Runtime,
    alive: &[i32],
    alive_majors: &[i32],
) -> Result<i32, Error> {
    if &defs.binding != state.binding() {
        return Err(Error::Unsupported("favor definitions binding"));
    }
    let current = player(state, owner)?;
    let mut raw = 0;
    add(&mut raw, defs.baseline);
    add(&mut raw, current.congress.favor.extra_per_turn);
    // CalculateSuzerainFavor visits all current alive players, not just minors.
    for &other in alive {
        if player(state, other)?.influence.suzerain == owner {
            return Err(Error::Unsupported("positive suzerain favor"));
        }
    }
    if !current.culture.tourism_into_favor.is_empty() {
        return Err(Error::Unsupported("positive tourism into favor"));
    }
    // Empty modifier lookup returns zero for every current policy. No current
    // policy or building is guessed absent to establish these branches.
    if current
        .culture
        .policy_favor_list
        .iter()
        .any(|v| v.favor_per_turn != 0)
    {
        return Err(Error::Unsupported("current policy favor"));
    }
    if current
        .culture
        .building_favor_list
        .iter()
        .any(|v| v.favor_per_turn > 0)
    {
        return Err(Error::Unsupported("positive building favor"));
    }
    climate
        .validate(state)
        .map_err(|_| Error::Unsupported("favor current Climate"))?;
    let i = usize::try_from(owner).map_err(|_| Error::Unsupported("favor player index"))?;
    let this = *climate
        .manager
        .this_turn_co2_footprint
        .get(i)
        .ok_or(Error::Unsupported("favor current CO2 slot"))?;
    let last = *climate
        .manager
        .last_turn_co2_footprint
        .get(i)
        .ok_or(Error::Unsupported("favor last CO2 slot"))?;
    add(
        &mut raw,
        defs.co2.calculate(
            climate
                .live
                .last_turn_average_co2_footprint
                .wrapping_sub(this)
                .wrapping_add(last),
        )?,
    );
    let mut grievances = defs.grievance_start;
    for &other in alive_majors {
        if other != owner {
            let score = state
                .game_relations()
                .map_err(|_| Error::Unsupported("favor formal relations"))?
                .grievance_score(other, owner)
                .map_err(|_| Error::Unsupported("favor current grievance score"))?;
            grievances = grievances.wrapping_sub(score);
        }
    }
    add(&mut raw, defs.grievances.calculate(grievances)?);
    for city in &current.cities.records.entries {
        if city.value.header.original_capital && city.value.header.original_owner != owner {
            return Err(Error::Unsupported("foreign original capital favor"));
        }
    }
    if current.culture.current_government_hash != u32::MAX && current.culture.anarchy_end_turn < 0 {
        let government = *defs
            .governments
            .get(&current.culture.current_government_hash)
            .unwrap_or(&0);
        let slots = current
            .congress
            .favor_from_government_slots
            .as_ref()
            .ok_or(Error::Unsupported("favor current government slot amounts"))?;
        let government = government_favor(
            government,
            slots,
            &current.culture.active_policies.entries,
            defs.government_slot_count,
        )?;
        if government > 0 {
            add(&mut raw, government);
        }
    }
    if !state
        .trade_graph()
        .map_err(|_| Error::Unsupported("favor current Trade graph"))?
        .graph()
        .origins()
        .is_empty()
    {
        return Err(Error::Unsupported("current trade partner favor"));
    }
    for &other in alive_majors {
        if other != owner
            && state
                .game_relations()
                .map_err(|_| Error::Unsupported("favor formal alliances"))?
                .is_allied(owner, other)
                .map_err(|_| Error::Unsupported("favor alliance state"))?
        {
            return Err(Error::Unsupported("positive alliance favor"));
        }
    }
    // Delegation/embassy getters reject nonpositive per-item amounts before
    // looking up the other players' current diplomatic presence.
    for (amount, embassy) in [
        (current.diplomacy.alliance_and_favor_modifiers[7], false),
        (current.diplomacy.alliance_and_favor_modifiers[8], true),
    ] {
        if amount > 0 {
            let mut count = 0i32;
            for &other in alive_majors {
                if other == owner {
                    continue;
                }
                let p = player(state, other)?;
                if *if embassy {
                    p.diplomacy.has_embassy.get(i)
                } else {
                    p.diplomacy.has_delegation.get(i)
                }
                .ok_or(Error::Unsupported("favor diplomatic presence index"))?
                {
                    count = count.wrapping_add(1);
                }
            }
            add(&mut raw, amount.wrapping_mul(count));
        }
    }
    Ok(raw >> 8)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn government_slot_favor_counts_current_slots_including_vacant_positions() {
        use c6_core::player_culture::GovernmentSlot;
        let active = [
            GovernmentSlot {
                policy_hash: u32::MAX,
                slot_ordinal: 1,
            },
            GovernmentSlot {
                policy_hash: 99,
                slot_ordinal: 1,
            },
            GovernmentSlot {
                policy_hash: 99,
                slot_ordinal: 0,
            },
        ];
        assert_eq!(government_favor(-1, &[-7, 3, 100], &active, 2), Ok(5));
        assert_eq!(government_favor(0, &[1, 0], &active, 2), Ok(1));
        assert_eq!(
            government_favor(4, &[1], &active, 2),
            Err(Error::Unsupported("favor current government slot index"))
        );
    }
    #[test]
    fn penalties_truncate_signed_division_before_clamping() {
        let penalty = Penalty {
            divisor: 3,
            minimum: -20,
            maximum: 0,
        };
        assert_eq!(penalty.calculate(-5), Ok(-1));
        assert_eq!(penalty.calculate(-1000), Ok(-20));
        assert_eq!(penalty.calculate(9), Ok(0));
    }
    #[test]
    fn base_is_q8_wrapping_not_an_unbounded_integer_sum() {
        let mut raw = 0;
        add(&mut raw, 0x7fffff);
        add(&mut raw, 1);
        assert_eq!(raw >> 8, -0x800000);
    }
}
