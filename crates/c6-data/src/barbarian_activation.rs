//! Source-ordered tribe upkeep after CreateCamps (Linux 2826130/26378b0).
//! Spawn callbacks use the same canonical state and game-sync RNG. Native
//! policy trees are not replaced with successful placeholder operations.
use c6_core::{
    game_barbarian::Instance,
    game_capabilities::Membership,
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub boldness_per_turn: i32,
}
impl Parameters {
    /// Cache the current admitted rules value once; missing input is not a
    /// zero-boldness update.
    pub fn from_rules(rules: &crate::Rules) -> Result<Self, &'static str> {
        let boldness_per_turn = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or("GlobalParameters")?
            .rows()
            .find(|row| row.text("Name").ok() == Some("BARBARIAN_BOLDNESS_PER_TURN"))
            .and_then(|row| row.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or("BARBARIAN_BOLDNESS_PER_TURN")?;
        Ok(Self { boldness_per_turn })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitKind {
    Melee,
    Ranged,
    Siege,
    Support,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    Raid,
    CityAttack,
}
#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    Random(c6_core::loaded_state::game_sync::AccessError),
    Effect(E),
}
pub trait Context {
    type Error;
    fn local_food(&mut self, state: &LoadedState, instance: usize) -> Result<i32, Self::Error>;
    fn create_scouts(
        &mut self,
        state: &mut LoadedState,
        instance: usize,
        location: PlotCoord,
        range: i32,
        count: i32,
    ) -> Result<(), Self::Error>;
    fn create_tag(
        &mut self,
        state: &mut LoadedState,
        instance: usize,
        location: PlotCoord,
        tag: &[u8],
        range: i32,
        count: i32,
    ) -> Result<(), Self::Error>;
    fn create_unit(
        &mut self,
        state: &mut LoadedState,
        instance: usize,
        location: PlotCoord,
        kind: UnitKind,
        range: i32,
    ) -> Result<bool, Self::Error>;
    fn behavior_threshold(
        &self,
        state: &LoadedState,
        instance: usize,
        behavior: Behavior,
    ) -> Result<i32, Self::Error>;
    /// False retains the target; a missing target is discarded by this kernel.
    fn start_behavior(
        &mut self,
        state: &mut LoadedState,
        instance: usize,
        target: ComponentId,
        behavior: Behavior,
    ) -> Result<bool, Self::Error>;
}
fn current<E>(state: &LoadedState, index: usize) -> Result<&Instance, Error<E>> {
    state
        .retained()
        .barbarian
        .instances
        .get(index)
        .ok_or(Error::Missing("current barbarian instance"))
}
fn edit<E>(state: &mut LoadedState, index: usize) -> Result<&mut Instance, Error<E>> {
    state
        .barbarian_manager_mut()
        .map_err(Error::State)?
        .instances
        .get_mut(index)
        .ok_or(Error::Missing("current barbarian instance"))
}
fn location(instance: &Instance) -> PlotCoord {
    PlotCoord {
        x: instance.camp_location.x,
        y: instance.camp_location.y,
    }
}
fn spawn_due(instance: &mut Instance) -> bool {
    instance.turns_since_last_warrior_spawn =
        instance.turns_since_last_warrior_spawn.wrapping_add(1);
    if instance.turns_since_last_warrior_spawn >= instance.turns_between_warrior_spawn {
        instance.turns_since_last_warrior_spawn = 0;
        true
    } else {
        false
    }
}
fn scout_due(instance: &mut Instance) -> bool {
    if instance.cur_num_scouts >= instance.max_num_scouts {
        return false;
    }
    let old = instance.turns_since_scout_death;
    instance.turns_since_scout_death = old.wrapping_add(1);
    old >= 4
}
fn ranged_selected(percent: i32, draw: u16) -> bool {
    percent > i32::from(draw)
}
fn cid(value: c6_core::game_barbarian::ComponentId) -> ComponentId {
    ComponentId {
        owner: value.owner,
        component_type: value.component_type,
        id: value.id,
    }
}
fn city_exists<E>(state: &LoadedState, id: ComponentId) -> Result<bool, Error<E>> {
    if id.component_type != 2 {
        return Err(Error::Missing("current barbarian target city identity"));
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == i32::from(id.owner))
        .ok_or(Error::Missing("current barbarian target player"))?;
    Ok(player
        .cities
        .records
        .entries
        .iter()
        .any(|city| city.value.header.id == id.id))
}

fn clean_trees<E>(state: &mut LoadedState, index: usize) -> Result<(), Error<E>> {
    for field in 0..3 {
        let tribe = current(state, index)?;
        let id = match field {
            0 => tribe.scouting_behavior_tree,
            1 => tribe.raiding_behavior_tree,
            _ => tribe.city_attack_behavior_tree,
        };
        if id == u32::MAX {
            continue;
        }
        let owner = tribe.player_ordinal;
        let manager = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| p.ai_behavior_tree.as_ref())
            .ok_or(Error::Missing("current barbarian behavior manager"))?;
        if manager
            .behavior_trees
            .entries
            .iter()
            .any(|e| e.tree.tree_id == id)
        {
            continue;
        }
        let tribe = edit(state, index)?;
        match field {
            0 => tribe.scouting_behavior_tree = u32::MAX,
            1 => tribe.raiding_behavior_tree = u32::MAX,
            _ => tribe.city_attack_behavior_tree = u32::MAX,
        };
    }
    Ok(())
}
fn spawn<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    index: usize,
    plot: PlotCoord,
) -> Result<(), Error<C::Error>> {
    if let Some(tag) = current(state, index)?
        .needed_unit_tags
        .first()
        .map(|t| t.bytes.clone())
    {
        context
            .create_tag(state, index, plot, &tag, 1, 1)
            .map_err(Error::Effect)?;
        let tags = &mut edit(state, index)?.needed_unit_tags;
        if tags.is_empty() {
            return Err(Error::Missing(
                "barbarian tag removed during creation callback",
            ));
        }
        tags.remove(0);
        return Ok(());
    }
    for kind in [
        UnitKind::Melee,
        UnitKind::Ranged,
        UnitKind::Siege,
        UnitKind::Support,
    ] {
        let t = current(state, index)?;
        let needed = match kind {
            UnitKind::Melee => t.required_melee_spawn,
            UnitKind::Ranged => t.required_ranged_spawn,
            UnitKind::Siege => t.required_siege_spawn,
            UnitKind::Support => t.required_support_spawn,
        };
        if needed <= 0 {
            continue;
        }
        if context
            .create_unit(state, index, plot, kind, 1)
            .map_err(Error::Effect)?
        {
            let t = edit(state, index)?;
            let count = match kind {
                UnitKind::Melee => &mut t.required_melee_spawn,
                UnitKind::Ranged => &mut t.required_ranged_spawn,
                UnitKind::Siege => &mut t.required_siege_spawn,
                UnitKind::Support => &mut t.required_support_spawn,
            };
            *count = count.wrapping_sub(1);
            return Ok(());
        }
    }
    let t = current(state, index)?;
    if t.num_standard_military_units >= t.max_standard_military_units {
        return Ok(());
    }
    let draw = state.game_sync_rand(100).map_err(Error::Random)?;
    let kind = if ranged_selected(current(state, index)?.percent_ranged_units, draw) {
        UnitKind::Ranged
    } else {
        UnitKind::Melee
    };
    context
        .create_unit(state, index, plot, kind, 1)
        .map_err(Error::Effect)?;
    Ok(())
}
fn target<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    index: usize,
    behavior: Behavior,
) -> Result<(), Error<C::Error>> {
    let t = current(state, index)?;
    let id = match behavior {
        Behavior::CityAttack => t.raided_cities.last(),
        Behavior::Raid => t.scouted_cities.last(),
    }
    .cloned();
    let Some(id) = id else {
        return Ok(());
    };
    if t.boldness
        < context
            .behavior_threshold(state, index, behavior)
            .map_err(Error::Effect)?
    {
        return Ok(());
    }
    let target = cid(id);
    if !city_exists(state, target)?
        || context
            .start_behavior(state, index, target, behavior)
            .map_err(Error::Effect)?
    {
        let t = edit(state, index)?;
        match behavior {
            Behavior::CityAttack => {
                t.raided_cities.pop();
            }
            Behavior::Raid => {
                t.scouted_cities.pop();
            }
        }
    }
    Ok(())
}
pub fn activate_instance<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    parameters: Parameters,
    capabilities: &Membership,
    index: usize,
) -> Result<(), Error<C::Error>> {
    let t = current(state, index)?;
    let plot = location(t);
    if t.living_raw == 0 || !c6_core::skip_turn::placement::coordinate_valid(plot) {
        return Ok(());
    }
    let food = context.local_food(state, index).map_err(Error::Effect)?;
    state
        .set_barbarian_local_food(index, food)
        .map_err(Error::State)?;
    if capabilities.has(c6_core::game_capabilities::BARBARIAN_CLANS) {
        return Err(Error::Missing("active Barbarian Clan instance upkeep"));
    }
    if spawn_due(edit(state, index)?) {
        spawn(state, context, index, plot)?;
    } else if scout_due(edit(state, index)?) {
        context
            .create_scouts(state, index, plot, 2, 1)
            .map_err(Error::Effect)?;
        edit(state, index)?.turns_since_scout_death = 0;
    }
    clean_trees(state, index)?;
    let t = edit(state, index)?;
    t.boldness = t.boldness.wrapping_add(parameters.boldness_per_turn);
    if !t.raided_cities.is_empty() && t.city_attack_behavior_tree == u32::MAX {
        target(state, context, index, Behavior::CityAttack)?;
    } else if !t.scouted_cities.is_empty() && t.raiding_behavior_tree == u32::MAX {
        target(state, context, index, Behavior::Raid)?;
    }
    Ok(())
}
/// This is the real current capability guard, not an inferred empty-Clans path.
pub fn activate_clans<E>(capabilities: &Membership, full_turn: bool) -> Result<(), Error<E>> {
    if !capabilities.has(c6_core::game_capabilities::BARBARIAN_CLANS) || !full_turn {
        return Ok(());
    }
    Err(Error::Missing("active Barbarian ClansManager upkeep"))
}
/// Called after the camp creator has appended and initialized any new tribes.
/// Keep the current native vector order; no ID or location sorting occurs.
pub fn after_camps<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    parameters: Parameters,
    capabilities: &Membership,
    full_turn: bool,
) -> Result<(), Error<C::Error>> {
    if !full_turn {
        return Ok(());
    }
    let count = state.retained().barbarian.instances.len();
    for index in 0..count {
        activate_instance(state, context, parameters, capabilities, index)?;
    }
    activate_clans(capabilities, full_turn)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Synthetic timer fields; this is not a claim to execute native Initialize.
    fn tribe() -> Instance {
        Instance {
            wire_version: 15,
            player_ordinal: 63,
            tribe_name_hash: 0,
            tribe_index: 0,
            camp_location: c6_core::game_barbarian::PlotCoord { x: 0, y: 0 },
            living_raw: 1,
            rampage_target_ordinal: -1,
            boldness: 0,
            max_num_scouts: 1,
            cur_num_scouts: 0,
            turns_since_scout_death: 0,
            max_standard_military_units: 5,
            num_standard_military_units: 0,
            turns_since_last_warrior_spawn: 0,
            percent_ranged_units: 30,
            turns_between_warrior_spawn: 4,
            required_melee_spawn: 0,
            required_ranged_spawn: 0,
            required_siege_spawn: 0,
            required_support_spawn: 0,
            needed_unit_tags: vec![],
            scouting_behavior_tree: u32::MAX,
            raiding_behavior_tree: u32::MAX,
            city_attack_behavior_tree: u32::MAX,
            scouted_cities: vec![],
            raided_cities: vec![],
            raiding_score: 0,
            turns_in_enemy_territory: 0,
            properties: None,
            clan_tech_hashes: None,
            tech_steal_turns_cooldown: -1,
            conversion_points: 0,
            player_bribe_turns_remaining: None,
            clan_player_stats: None,
            incite_targets: None,
        }
    }
    #[test]
    fn warrior_timer_resets_before_spawn_and_wraps_signed() {
        let mut t = tribe();
        t.turns_since_last_warrior_spawn = 2;
        assert!(!spawn_due(&mut t));
        assert_eq!(t.turns_since_last_warrior_spawn, 3);
        assert!(spawn_due(&mut t));
        assert_eq!(t.turns_since_last_warrior_spawn, 0);
        t.turns_since_last_warrior_spawn = i32::MAX;
        assert!(!spawn_due(&mut t));
        assert_eq!(t.turns_since_last_warrior_spawn, i32::MIN);
    }
    #[test]
    fn scout_uses_old_counter_and_skips_full_roster() {
        let mut t = tribe();
        t.turns_since_scout_death = 3;
        assert!(!scout_due(&mut t));
        assert_eq!(t.turns_since_scout_death, 4);
        assert!(scout_due(&mut t));
        assert_eq!(t.turns_since_scout_death, 5);
        t.cur_num_scouts = 1;
        assert!(!scout_due(&mut t));
        assert_eq!(t.turns_since_scout_death, 5);
    }
    #[test]
    fn ranged_percentage_uses_strict_signed_comparison() {
        assert!(!ranged_selected(-1, 0));
        assert!(!ranged_selected(0, 0));
        assert!(ranged_selected(1, 0));
        assert!(!ranged_selected(30, 30));
        assert!(ranged_selected(100, 99));
    }
    #[test]
    fn disabled_clans_need_no_clan_object_or_rng() {
        let none = Membership::from_sorted(vec![]).unwrap();
        assert!(activate_clans::<()>(&none, true).is_ok());
        let active =
            Membership::from_sorted(vec![c6_core::game_capabilities::BARBARIAN_CLANS]).unwrap();
        assert!(activate_clans::<()>(&active, false).is_ok());
        assert!(matches!(
            activate_clans::<()>(&active, true),
            Err(Error::Missing(_))
        ));
    }
}
