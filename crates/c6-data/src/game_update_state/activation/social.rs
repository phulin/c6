//! Consecutive first-activation Religion/GreatPeople/Influence/Quests rules.
//! City contributions use current canonical districts, buildings and yields, preserving the
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

type PointDefinition = (u32, u32, i32);
pub struct Definitions {
    binding: WireBinding,
    classes: LinuxDefinitionRegistry,
    levels: LinuxDefinitionRegistry,
    eligibility: Vec<(bool, bool)>,
    district_points: Vec<PointDefinition>,
    citizen_points: Vec<PointDefinition>,
    building_points: Vec<PointDefinition>,
    alliances: LinuxDefinitionRegistry,
    government_bonuses: LinuxDefinitionRegistry,
    religious_purchase_heroes: bool,
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
        let points = |table: &str, key: &str| -> Result<Vec<PointDefinition>, Error> {
            let rows = rules
                .gameplay()
                .table(table)
                .ok_or(Error::Unsupported("GreatPerson point definitions"))?;
            rows.rows()
                .map(|row| {
                    let hash = |name| {
                        row.text(name)
                            .map(|s| linux_type_hash(s.as_bytes()))
                            .map_err(|_| Error::Unsupported("GreatPerson point reference"))
                    };
                    Ok((
                        hash(key)?,
                        hash("GreatPersonClassType")?,
                        row.integer("PointsPerTurn")
                            .map_err(|_| Error::Unsupported("GreatPerson point amount"))?
                            as i32,
                    ))
                })
                .collect()
        };
        let units = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("religious purchase Unit definitions"))?;
        let heroes = rules
            .gameplay()
            .table("HeroClasses")
            .ok_or(Error::Unsupported("religious purchase Hero definitions"))?;
        let mut religious_purchase_heroes = false;
        for unit in units.rows() {
            let strength = unit
                .integer("ReligiousStrength")
                .map_err(|_| Error::Unsupported("religious Unit strength"))?;
            let enabled = unit
                .integer("EnabledByReligion")
                .map_err(|_| Error::Unsupported("religious Unit enablement"))?;
            if strength <= 0 && enabled == 0 {
                continue;
            }
            let name = unit
                .text("UnitType")
                .map_err(|_| Error::Unsupported("religious Unit name"))?;
            for hero in heroes.rows() {
                if matches!(hero.get("UnitType"),Ok(crate::SqlValue::Text(bytes)) if bytes.as_slice()==name.as_bytes())
                {
                    religious_purchase_heroes = true;
                }
            }
        }
        Ok(Self {
            religious_purchase_heroes,
            district_points: points("District_GreatPersonPoints", "DistrictType")?,
            citizen_points: points("District_CitizenGreatPersonPoints", "DistrictType")?,
            building_points: points("Building_GreatPersonPoints", "BuildingType")?,
            alliances: collection(rules, "Alliances", "AllianceType")
                .map_err(|_| Error::Unsupported("social alliance definitions"))?,
            government_bonuses: collection(rules, "GovernmentBonusNames", "GovernmentBonusType")
                .map_err(|_| Error::Unsupported("social government bonus definitions"))?,
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
#[derive(Clone, Copy)]
pub struct Queries<'a> {
    pub levels: &'a [(i32, u32)],
    /// Current native alive order, not saved-player or sorted-ID substitution.
    pub alive_order: &'a [i32],
    pub capabilities: &'a Membership,
    pub cities: Option<crate::loaded_actions::stock_notifications::city_queries::Inputs<'a>>,
}
/// Forward these into the SAME stock reporting/notification runtime. No default
/// succeeds without executing the actual current semantic consumer path.
pub trait Effects {
    /// ChoosePantheon2b624c0 queues the request; policy runs later in Religious activation.
    fn queue_pantheon(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error>;
    fn notification(&mut self, state: &mut LoadedState, owner: i32, kind: u32)
        -> Result<(), Error>;
    /// CheckNewPurchases runs before the balance setter. A concrete headless
    /// provider may omit pure presentation scans, but must deliver the current
    /// +618 signal and preserve any gameplay/RNG consumers.
    fn check_new_faith_purchases(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        old_q8: i32,
        new_q8: i32,
    ) -> Result<(), Error>;
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
#[cfg(test)]
fn no_city_points(bonus_q8: i32, percent: i32) -> i32 {
    bonus_percent((bonus_q8 >> 8).wrapping_shl(8), percent)
}
fn bonus_percent(base: i32, percent: i32) -> i32 {
    if percent <= 0 {
        return base;
    }
    let ratio = ((i64::from(percent.wrapping_shl(8)) << 8) / 25600) as i32;
    base.wrapping_add(c6_core::city_production::execution::multiply(base, ratio))
}
fn district_points(points: i32, percent: i32) -> i32 {
    points
        .wrapping_mul(percent.wrapping_add(100))
        .wrapping_div(100)
        .wrapping_shl(8)
}
fn city_percent(points: i32, percent: i32) -> i32 {
    if points == 0 || percent == 0 {
        return points;
    }
    let product = c6_core::city_production::execution::multiply(
        points,
        percent.wrapping_add(100).wrapping_shl(8),
    );
    ((i64::from(product) << 8) / 25600) as i32
}
fn city_great_people(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
    class: u32,
) -> Result<i32, Error> {
    let mut total = 0i32;
    let p = player(state, owner)?;
    for record in &p.cities.records.entries {
        let city = &record.value;
        let mut subtotal = 0i32;
        for &id in &city.districts.district_ids {
            let district = state
                .district(owner, id)
                .map_err(|_| Error::Unsupported("great-person current District"))?;
            if !district.is_complete || district.pillaged {
                continue;
            }
            for &(kind, person, points) in &defs.district_points {
                if kind == district.district_hash && person == class {
                    subtotal =
                        subtotal.wrapping_add(district.great_person_point_adder.wrapping_shl(8));
                    subtotal = subtotal.wrapping_add(district_points(
                        points,
                        district.great_person_point_multiplier,
                    ));
                }
            }
            let change = district
                .great_person_point_changes_q8
                .iter()
                .rev()
                .find(|r| r.definition_hash == class)
                .ok_or(Error::Unsupported("great-person District change cell"))?
                .value;
            subtotal = subtotal.wrapping_add(change);
            for &(kind, person, points) in &defs.citizen_points {
                if kind != district.district_hash || person != class {
                    continue;
                }
                if state
                    .world()
                    .map()
                    .plot_at(district.location.0, district.location.1)
                    .is_none()
                {
                    continue;
                }
                let center = c6_core::unit::PlotCoord {
                    x: city.header.location_x,
                    y: city.header.location_y,
                };
                let at = c6_core::unit::PlotCoord {
                    x: district.location.0,
                    y: district.location.1,
                };
                let ordinary_and_forced = c6_core::city_citizen_assignment::plot_workers(
                    &city.citizens,
                    state.world().map(),
                    center,
                    at,
                )
                .map_err(Error::Unsupported)?;
                let index =
                    c6_core::city_citizen_assignment::plot_index(state.world().map(), center, at)
                        .map_err(|_| Error::Unsupported("great-person specialist City index"))?;
                // Native CalculatePointsPerTurn explicitly adds the separate
                // forced getter after GetPlotWorkers (which includes forced).
                let forced = city
                    .citizens
                    .forced_working_plot
                    .get(index as u32 as usize)
                    .copied()
                    .unwrap_or(0);
                let workers = ordinary_and_forced.wrapping_add(forced);
                if workers > 0 {
                    subtotal = subtotal.wrapping_add(workers.wrapping_mul(points).wrapping_shl(8));
                }
            }
        }
        for &(building, person, points) in &defs.building_points {
            if person != class {
                continue;
            }
            let present = city
                .buildings
                .building_locations
                .iter()
                .rev()
                .find(|r| r.definition_hash == building)
                .ok_or(Error::Unsupported("great-person current Building"))?
                .value
                >= 0;
            if !present {
                continue;
            }
            let pillaged = city
                .buildings
                .pillaged
                .iter()
                .rev()
                .find(|r| r.definition_hash == building)
                .ok_or(Error::Unsupported("great-person current Building pillage"))?
                .value;
            if !pillaged {
                subtotal = subtotal.wrapping_add(points.wrapping_shl(8));
            }
        }
        subtotal = city_percent(subtotal, city.header.great_person_point_bonus_percent);
        total = total.wrapping_add(subtotal);
        // CacheCityPseudoYield writes excluded planner scratch only. It does
        // not alter this earned amount, gameplay state, callbacks or RNG.
    }
    // The second source City pass adds happiness/class rows after all city
    // subtotals and before the player's flat and percentage bonuses.
    for record in &p.cities.records.entries {
        let city = &record.value;
        let inputs = queries.cities.ok_or(Error::Unsupported(
            "great-person current happiness services",
        ))?;
        let happiness = crate::unit_creation::placement_effects::happiness(
            state,
            owner,
            city.header.id,
            inputs.definitions,
            queries.capabilities,
        )
        .map_err(|_| Error::Unsupported("great-person current happiness"))?;
        for row in &city.header.happiness_modifiers {
            if row.happiness_type_index == happiness && row.great_person_class_hash == class {
                total = total.wrapping_add(row.great_person_amount.wrapping_shl(8));
            }
        }
    }
    Ok(total)
}
fn has_alliance_type(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    hash: u32,
) -> Result<bool, Error> {
    let Some(index) = defs.alliances.find_index(hash) else {
        return Ok(false);
    };
    let relations = state
        .game_relations()
        .map_err(|_| Error::Unsupported("social current alliances"))?;
    // Check the complete current relation row: no dead/invalid saved-player
    // omission may silently turn a positive alliance into a zero contribution.
    for other in 0..relations.player_count() {
        if other != owner
            && relations
                .alliance_type(owner, other)
                .map_err(|_| Error::Unsupported("social current alliance pair"))?
                == index as i32
        {
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn influence_per_turn(
    state: &LoadedState,
    defs: &Definitions,
    _queries: &Queries<'_>,
    owner: i32,
) -> Result<i32, Error> {
    defs.validate(state)?;
    let p = player(state, owner)?;
    if no_cities(p) || p.influence.not_earning_points {
        return Ok(0);
    }
    let mut result = p.influence.base_points_per_turn;
    if has_alliance_type(state, defs, owner, 0x5ede01e0)? {
        return Err(Error::Unsupported("current cultural alliance envoy bonus"));
    }
    let bonus = if defs.government_bonuses.find_index(0xe96ca959).is_none()
        || p.culture.anarchy_end_turn >= 0
    {
        0
    } else {
        p.bonuses
            .flat_bonuses
            .iter()
            .rev()
            .find(|r| r.definition_hash == 0xe96ca959)
            .ok_or(Error::Unsupported("influence current government bonus"))?
            .value
    };
    let ratio = c6_core::city_production::execution::from_float(bonus as f32 * 0.01f32);
    result = result.wrapping_add(c6_core::city_production::execution::multiply(result, ratio));
    Ok(result)
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
            let city_points = city_great_people(state, defs, queries, owner, hash)?;
            let base = city_points.wrapping_add(
                (pair(&p.great_people_points.bonus_points_per_turn, hash)? >> 8).wrapping_shl(8),
            );
            bonus_percent(
                base,
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
/// Faith from source CalculateFaithYield2b5fe00, including live City yields.
/// Each other contribution is admitted independently; no-city alone is not a
/// substitute for belief, envoy, delegation, embassy or prior GP earnings.
pub fn faith_yield(
    state: &LoadedState,
    defs: &Definitions,
    queries: &Queries<'_>,
    owner: i32,
) -> Result<i32, Error> {
    let p = player(state, owner)?;
    if p.culture.anarchy_end_turn >= 0 {
        return Ok(0);
    }
    let mut city_faith = 0i32;
    if !no_cities(p) {
        let inputs = queries
            .cities
            .ok_or(Error::Unsupported("current social city query services"))?;
        let faith = inputs
            .definitions
            .yield_index(b"YIELD_FAITH")
            .ok_or(Error::Unsupported("current Faith definition"))?;
        for entry in &p.cities.records.entries {
            city_faith = city_faith.wrapping_add(
                inputs
                    .current
                    .calculate(
                        state,
                        owner,
                        entry.value.header.id,
                        faith,
                        0,
                        inputs.definitions,
                    )
                    .map_err(|_| Error::Unsupported("current City faith yield"))?,
            );
        }
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
    if !no_cities(p) && has_alliance_type(state, defs, owner, 0xd71225de)? {
        return Err(Error::Unsupported("current religious alliance faith"));
    }
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
    Ok(city_faith.wrapping_add(p.religion.faith_rate_change_q8))
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
    let yield_q8 = faith_yield(state, defs, queries, owner)?;
    let old = player(state, owner)?.religion.faith_balance_q8;
    // CheckNewPurchases2b5f550 calls CanTrain2851040(type,true,null,0,
    // false,false), and CanConstruct with a null output map. The ordinary
    // bodies and costs only read current rules/state; their indirect calls
    // are Plot getters and optional output-map writes. CanEverTrain21c10a0
    // only compares the current replacement vector. Conditional Hero CanTrain
    // is kept separate below. Identical clamped balances cannot cross a cost,
    // so the notification constructors/Send (and their effects) are unreachable.
    if !no_cities(player(state, owner)?) && defs.religious_purchase_heroes {
        return Err(Error::Unsupported("religious Hero purchase queries"));
    }
    let new = old.saturating_add(yield_q8);
    if old != new {
        effects.check_new_faith_purchases(state, owner, old, new)?;
    }
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
            effects.queue_pantheon(state, owner)?;
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
        let ids: Vec<u32> = player(state, other)?
            .cities
            .records
            .entries
            .iter()
            .map(|r| r.value.header.id)
            .collect();
        for city in ids {
            let religion = &state
                .city(other, city)
                .map_err(|_| Error::Unsupported("next religion City"))?
                .religion;
            if religion.next_religion_hash == u32::MAX && religion.turns_to_next_religion == -1 {
                // RecomputeNextReligion28c4e70 first writes {INVALID,0}; an
                // empty actual Game.Religions vector then exits immediately.
                let has_religions = !state.retained().religion.records.is_empty();
                let religion = &mut state
                    .city_mut(other, city)
                    .map_err(|_| Error::Unsupported("next religion City mutation"))?
                    .religion;
                religion.turns_to_next_religion = 0;
                if has_religions {
                    return Err(Error::Unsupported(
                        "positive next religion pressure recomputation",
                    ));
                }
            }
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
/// 2b207a0. The no-city/not-earning getter exits before base/bonus queries;
/// SetPointsEarned still reports d35e571d for zero delta.
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
        let earned = influence_per_turn(state, defs, queries, owner)?;
        let value = player(state, owner)?
            .influence
            .points_earned
            .wrapping_add(earned)
            .max(0);
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
    fn district_and_city_percentages_have_distinct_rounding_stages() {
        assert_eq!(district_points(1, 50), 256); // integer district award first
        assert_eq!(city_percent(256, 50), 384); // city modifier retains Q8 fraction
        assert_eq!(city_percent(-1, 50), -1); // SAR multiply, signed division toward zero
        assert_eq!(bonus_percent(384, 50), 576); // player bonus applies after City sum
        assert_eq!(c6_core::city_production::execution::from_float(-0.01), -3);
    }
    #[test]
    fn current_array_lookup_preserves_last_overlay_and_missing_is_unknown() {
        assert_eq!(pair(&[(7, 1), (8, 9), (7, -3)], 7), Ok(-3));
        assert!(matches!(pair(&[], 7), Err(Error::Unsupported(_))));
    }
}
