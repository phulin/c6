//! Consecutive first-activation Religion/GreatPeople/Influence/Quests rules.
//! This bounded adapter supports the source no-city branches, preserving the
//! actual contributor fields, setter reports and semantic notifications.
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, game_capabilities::Membership, game_update::Error,
    loaded_components::RetainedPlayer, loaded_state::LoadedState,
};

pub struct Definitions {
    binding: WireBinding,
    classes: LinuxDefinitionRegistry,
    levels: LinuxDefinitionRegistry,
    eligibility: Vec<(bool, bool)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let classes = collection(rules, "GreatPersonClasses", "GreatPersonClassType")
            .map_err(|_| Error::Unsupported("GreatPersonClasses"))?;
        let levels = collection(rules, "CivilizationLevels", "CivilizationLevelType")
            .map_err(|_| Error::Unsupported("CivilizationLevels"))?;
        let table = rules
            .gameplay()
            .table("CivilizationLevels")
            .ok_or(Error::Unsupported("CivilizationLevels"))?;
        let mut eligibility = vec![];
        for entry in levels.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("current civilization level"))?;
            let value = |name| {
                row.get(name)
                    .ok()
                    .and_then(crate::game_parameters::sql_integer)
                    .map(|v| v != 0)
                    .ok_or(Error::Unsupported("civilization eligibility"))
            };
            eligibility.push((value("CanEarnGreatPeople")?, value("CanGiveInfluence")?));
        }
        Ok(Self {
            binding: state.binding().clone(),
            classes,
            levels,
            eligibility,
        })
    }
    fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() != &self.binding {
            Err(Error::Unsupported("social definition binding"))
        } else {
            Ok(())
        }
    }
    fn eligible(
        &self,
        state: &LoadedState,
        queries: &Queries<'_>,
        owner: i32,
    ) -> Result<(bool, bool), Error> {
        if !state
            .player_alive(owner)
            .map_err(|_| Error::Unsupported("social current alive"))?
        {
            return Ok((false, false));
        }
        let level = queries
            .levels
            .iter()
            .find(|r| r.0 == owner)
            .ok_or(Error::Unsupported("social current civilization level"))?
            .1;
        Ok(self
            .levels
            .find_index(level)
            .and_then(|i| self.eligibility.get(i))
            .copied()
            .unwrap_or((false, false)))
    }
}
pub struct Queries<'a> {
    pub levels: &'a [(i32, u32)],
    /// Current native alive order, not saved-player or sorted-ID substitution.
    pub alive_order: &'a [i32],
    pub capabilities: &'a Membership,
}
/// Forward these into the SAME stock reporting/notification runtime. No default
/// succeeds without executing the actual current semantic consumer path.
pub trait Effects {
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error>;
    fn notification(&mut self, state: &mut LoadedState, owner: i32, kind: u32)
        -> Result<(), Error>;
    /// Changed faith writes precede +600 dispatch and the b3495127 report.
    fn faith_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        new_q8: i32,
    ) -> Result<(), Error>;
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("social current player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("social canonical player"))
}
fn no_cities(player: &RetainedPlayer) -> bool {
    player.cities.records.entries.is_empty()
}
fn pair(rows: &[(u32, i32)], hash: u32) -> Result<i32, Error> {
    rows.iter()
        .rev()
        .find(|r| r.0 == hash)
        .map(|r| r.1)
        .ok_or(Error::Unsupported("social current definition array cell"))
}
fn no_city_points(bonus_q8: i32, percent: i32) -> i32 {
    let base = (bonus_q8 >> 8).wrapping_shl(8); // native AsInt is SAR, not /256.
    if percent <= 0 {
        return base;
    }
    let ratio = ((i64::from(percent.wrapping_shl(8)) << 8) / 25600) as i32;
    base.wrapping_add(c6_core::city_production::execution::multiply(base, ratio))
}
/// 2b11b20: source eligibility before class iteration. Source assigns each
/// earned-this-turn cell BEFORE changing that class total.
pub fn great_people(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    if !defs.eligible(state, queries, owner)?.0 {
        return Ok(());
    }
    for entry in defs.classes.entries() {
        let hash = linux_type_hash(&entry.type_name);
        let p = player(state, owner)?;
        let value = if p
            .great_people_points
            .great_person_class_no_points
            .contains(&hash)
        {
            0
        } else {
            if !no_cities(p) {
                return Err(Error::Unsupported("great-person City contributions"));
            }
            no_city_points(
                pair(&p.great_people_points.bonus_points_per_turn, hash)?,
                pair(&p.great_people_points.bonus_points_per_turn_percent, hash)?,
            )
        };
        let components = state
            .social_components_mut(owner)
            .map_err(|_| Error::Unsupported("great-person canonical mutation"))?;
        let cell = components
            .great_people
            .points_earned_this_turn
            .iter_mut()
            .rev()
            .find(|r| r.0 == hash)
            .ok_or(Error::Unsupported("great-person earned cell"))?;
        cell.1 = value;
        if value != 0 {
            return Err(Error::Unsupported("great-person total change callbacks"));
        }
    }
    Ok(())
}
/// Faith from source CalculateFaithYield2b5fe00 in the current no-city case.
/// Each other contribution is admitted independently; no-city alone is not a
/// substitute for belief, envoy, delegation, embassy or prior GP earnings.
pub fn faith_yield(state: &LoadedState, owner: i32) -> Result<i32, Error> {
    let p = player(state, owner)?;
    if p.culture.anarchy_end_turn >= 0 {
        return Ok(0);
    }
    if !no_cities(p) {
        return Err(Error::Unsupported("current City faith contributions"));
    }
    if p.religion.belief_yields.iter().any(|r| r.amount != 0) {
        return Err(Error::Unsupported("current belief faith"));
    }
    if p.influence
        .yield_per_given_token
        .iter()
        .chain(&p.influence.yield_per_tributary)
        .any(|r| r.1 != 0)
    {
        return Err(Error::Unsupported("current envoy faith"));
    }
    if p.diplomacy
        .yield_per_delegation
        .iter()
        .chain(&p.diplomacy.yield_per_embassy)
        .any(|r| r.value != 0)
    {
        return Err(Error::Unsupported("current delegation faith"));
    }
    if p.religion.faith_per_peaceful_religion_founder != 0 {
        return Err(Error::Unsupported("peaceful founder faith"));
    }
    // Alliance faith is a sum over this player's City followers: no cities=0.
    if p.great_people_points
        .points_earned_this_turn
        .iter()
        .any(|r| r.1 != 0)
    {
        return Err(Error::Unsupported("faith from unused great-person points"));
    }
    if p.influence
        .yield_modifier_per_tributary
        .iter()
        .chain(&p.great_people_points.yield_modifier_per_earned_great_person)
        .any(|r| r.1 != 0)
    {
        return Err(Error::Unsupported("current faith yield modifiers"));
    }
    Ok(p.religion.faith_rate_change_q8)
}
/// 2b5f430, including setter's synchronous effects and the actual capability
/// controlled notification requests. External policy never chooses a pantheon.
pub fn religion(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    effects: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    let yield_q8 = faith_yield(state, owner)?;
    // CheckNewPurchases2b5f550 scans religious Units/Buildings over current
    // Cities. Both inner loops are actually empty in this supported branch.
    if !no_cities(player(state, owner)?) {
        return Err(Error::Unsupported("faith purchase availability"));
    }
    let old = player(state, owner)?.religion.faith_balance_q8;
    let new = old.saturating_add(yield_q8);
    if old != new {
        state
            .social_components_mut(owner)
            .map_err(|_| Error::Unsupported("canonical faith balance"))?
            .religion
            .faith_balance_q8 = new;
        effects.faith_changed(state, owner, new)?;
    }
    if queries.capabilities.has(0xb9fc347c)
        && player(state, owner)?.religion.pantheon_hash == u32::MAX
    {
        let founded = state
            .retained()
            .religion
            .records
            .iter()
            .any(|r| r.founder == owner);
        if !founded
            && state.retained().religion.next_pantheon_founder == owner
            && player(state, owner)?.religion.faith_balance_q8
                >= state
                    .retained()
                    .religion
                    .minimum_faith_for_next_pantheon
                    .wrapping_shl(8)
            && state
                .player_slot_status(owner)
                .map_err(|_| Error::Unsupported("pantheon current human"))?
                != 3
        {
            return Err(Error::ExternalDecisionRequired("pantheon selection"));
        }
    }
    if state
        .retained()
        .religion
        .records
        .iter()
        .any(|r| r.founder == owner)
    {
        return Err(Error::Unsupported("founded religion pressure spread"));
    }
    // UpdateNextReligionForCities traverses current alive order at this point.
    for &other in queries.alive_order {
        if !no_cities(player(state, other)?) {
            return Err(Error::Unsupported("City next religion recomputation"));
        }
    }
    if queries.capabilities.has(0xb9fc347c) {
        effects.notification(state, owner, 0x9456ed4a)?;
    }
    if queries.capabilities.has(0xf3113a05) {
        effects.notification(state, owner, 0xa6fdd52e)?;
        effects.notification(state, owner, 0x276cf66c)?;
    }
    Ok(())
}
/// 2b207a0. The no-city points getter returns zero BEFORE base points, bonuses
/// or alliance reads, yet SetPointsEarned still reports d35e571d for zero delta.
pub fn influence(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    effects: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    let p = player(state, owner)?;
    if p.influence.suzerain_visibility_counter > 0 && p.influence.suzerain != -1 {
        return Err(Error::Unsupported("suzerain visibility refresh"));
    }
    if defs.eligible(state, queries, owner)?.1 {
        if !no_cities(player(state, owner)?) {
            return Err(Error::Unsupported("influence City points per turn"));
        }
        let value = player(state, owner)?.influence.points_earned.max(0);
        state
            .social_components_mut(owner)
            .map_err(|_| Error::Unsupported("canonical influence points"))?
            .influence
            .points_earned = value;
        effects.report(state, owner, 0xd35e571d)?;
        let p = player(state, owner)?;
        if defs.eligible(state, queries, owner)?.1
            && p.influence.points_threshold > 0
            && p.influence.points_earned >= p.influence.points_threshold.wrapping_shl(8)
        {
            return Err(Error::Unsupported("earned envoy tokens"));
        }
        effects.notification(state, owner, 0x579b28d1)?;
        // ManageIndividualLevies only changes live levied foreign units.
        if state
            .world()
            .player(owner)
            .map_err(|_| Error::Unsupported("current levy owner"))?
            .unit_records()
            .map_err(|_| Error::Unsupported("current levy units"))?
            .iter()
            .any(|r| {
                r.value.b_is_levied
                    && !r.value.b_delayed_death
                    && r.value.e_original_owner != r.value.e_owner
            })
        {
            return Err(Error::Unsupported("individual levy turn transition"));
        }
    }
    if player(state, owner)?.influence.levy_turn_counter >= 0 {
        let counter = &mut state
            .social_components_mut(owner)
            .map_err(|_| Error::Unsupported("canonical levy counter"))?
            .influence
            .levy_turn_counter;
        *counter = counter.wrapping_add(1);
    }
    Ok(())
}
/// Exact target Quests.DoTurn2b5cd40 is RET; there is no quest-manager call.
pub fn quests(_: &mut LoadedState, _: i32) -> Result<(), Error> {
    Ok(())
}

/// TurnManager first-activation sequence2ba0bf9..2ba0c31. Later stages are not
/// executed after a reached callback/decision failure; prior writes remain.
pub fn run(
    state: &mut LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    effects: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    religion(state, defs, queries, effects, owner)?;
    great_people(state, defs, queries, owner)?;
    influence(state, defs, queries, effects, owner)?;
    quests(state, owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bonus_points_round_before_percentage_and_use_native_q8_ratio() {
        assert_eq!(no_city_points(255, 100), 0);
        assert_eq!(no_city_points(-1, 0), -256);
        assert_eq!(no_city_points(512, 33), 680); // ratio truncates to84Q8.
        assert_eq!(no_city_points(256, -50), 256); // source ignores <=0.
        assert_eq!(no_city_points(i32::MAX, 100), -512);
    }
    #[test]
    fn current_array_lookup_preserves_last_overlay_and_missing_is_unknown() {
        assert_eq!(pair(&[(7, 1), (8, 9), (7, -3)], 7), Ok(-3));
        assert!(matches!(pair(&[], 7), Err(Error::Unsupported(_))));
    }
}
