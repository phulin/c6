//! Linux XP2 camp construction. Source entries: SetImprovementType 0x2a391d0,
//! OnImprovementCreated 0x282f190, FindTribeOfType 0x2830640, Initialize 0x28256e0.
//! Construction callbacks use the live registration order; unit creation and
//! owner sight operate on the same state, including all reached random draws.
use c6_core::{
    game_barbarian::{CampLocation, ClanPlayerStats, Instance},
    loaded_state::LoadedState,
    map::{DefinitionValue, PlotFlag},
    unit::PlotCoord,
};

#[derive(Clone, Debug)]
pub struct Tribe {
    pub index: i32,
    pub coastal: bool,
    pub required_resource: Option<(u32, u32)>,
    pub percent_ranged: i32,
    pub turns_to_spawn: i32,
    /// Native GetTribeNames iteration order, expressed as indices into names.
    pub names: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct Name {
    pub hash: u32,
    pub tribe: i32,
    pub scouts: Option<i32>,
    pub military: Option<i32>,
    pub turns_to_spawn: Option<i32>,
    pub percent_ranged: Option<i32>,
}
#[derive(Clone, Debug)]
pub struct Rules {
    pub tribes: Vec<Tribe>,
    pub names: Vec<Name>,
    pub clans_active: bool,
    pub world_builder: bool,
}
#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Unsupported(&'static str),
    Effect(E),
    State(c6_core::loaded_state::Error),
    Random(c6_core::loaded_state::game_sync::AccessError),
}
/// Reached stock services. No successful defaults: callers dispatch subscribers
/// in their retained order, invoking `on_created` at the Barbarian subscriber.
pub trait Context {
    type Error;
    fn is_coastal(&self, state: &LoadedState, location: PlotCoord) -> Result<bool, Self::Error>;
    /// Iterate native neighbors in source order; accept the first matching
    /// resource only if that plot is unowned (0x28318d0).
    fn required_resource(
        &self,
        state: &LoadedState,
        location: PlotCoord,
        resource: u32,
        radius: u32,
    ) -> Result<bool, Self::Error>;
    fn scaled_spawn_turns(&self, state: &LoadedState, turns: i32) -> Result<i32, Self::Error>;
    /// SetImprovementOwner: valid-player check, source plot and six neighbors'
    /// sight, Player.Improvements.AddImprovement, then owner-changed reporting event.
    fn set_owner(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn constructed(
        &mut self,
        state: &mut LoadedState,
        rules: &Rules,
        plot: usize,
    ) -> Result<(), Self::Error>;
    fn spawn_initial_units(
        &mut self,
        state: &mut LoadedState,
        index: usize,
        location: PlotCoord,
        scouts: i32,
    ) -> Result<(), Self::Error>;
    /// Native radius-one unowned plots only, including exact live plot yields.
    fn local_food(&self, state: &LoadedState, location: PlotCoord) -> Result<i32, Self::Error>;
    fn camp_created(&mut self, state: &mut LoadedState, index: usize) -> Result<(), Self::Error>;
    fn restore_visual_state(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        improvement_hash: u32,
    ) -> Result<(), Self::Error>;
    fn resource_monopolies(&mut self, state: &mut LoadedState) -> Result<(), Self::Error>;
    fn revealed_major_notifications(
        &mut self,
        state: &mut LoadedState,
        location: PlotCoord,
    ) -> Result<(), Self::Error>;
}
fn location(state: &LoadedState, plot: usize) -> Result<PlotCoord, &'static str> {
    let map = state.world().map();
    if map.header.width <= 0 || plot >= map.plots.len() {
        return Err("camp plot");
    }
    Ok(PlotCoord {
        x: (plot % map.header.width as usize) as i32,
        y: (plot / map.header.width as usize) as i32,
    })
}
/// First placement on an unimproved plot; replacement is a distinct transition.
/// A callback error leaves the reached prefix visible and is never completion.
pub fn set_camp<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    rules: &Rules,
    plot: usize,
    improvement_hash: u32,
    owner: i32,
) -> Result<(), Error<C::Error>> {
    let loc = location(state, plot).map_err(Error::Missing)?;
    let current = &state.world().map().plots[plot];
    if current.improvement_hash == improvement_hash {
        return context
            .revealed_major_notifications(state, loc)
            .map_err(Error::Effect);
    }
    if current.improvement_hash != u32::MAX {
        return Err(Error::Unsupported("camp replaces existing improvement"));
    }
    if current.improvement_owner != -1 {
        return Err(Error::Unsupported("unimproved plot has improvement owner"));
    }
    let area = current.area_id;
    {
        let world = state.world_mut().map_err(Error::State)?;
        let p = world.plot_mut(plot).ok_or(Error::Missing("camp plot"))?;
        p.improvement_hash = improvement_hash;
        p.flags.set(PlotFlag::ImprovementPillaged, false);
        crate::builder_improvement::placement::reset_improvement_direction(&mut p.properties)
            .map_err(Error::Unsupported)?;
        if let Some(area) = world.area_mut(area) {
            if let Some(count) = area
                .improvement_counts
                .iter_mut()
                .find(|c| c.hash == improvement_hash)
            {
                count.value = count.value.wrapping_add(1);
            } else {
                area.improvement_counts.push(DefinitionValue {
                    hash: improvement_hash,
                    value: 1,
                });
            }
        }
    }
    context
        .set_owner(state, plot, owner)
        .map_err(Error::Effect)?;
    context
        .constructed(state, rules, plot)
        .map_err(Error::Effect)?;
    // RestoreVisualState also updates gameplay-consumed revealed improvements.
    context
        .restore_visual_state(state, plot, improvement_hash)
        .map_err(Error::Effect)?;
    context.resource_monopolies(state).map_err(Error::Effect)?;
    context
        .revealed_major_notifications(state, loc)
        .map_err(Error::Effect)?;
    Ok(())
}

fn find_name<E>(
    state: &mut LoadedState,
    rules: &Rules,
    tribe: &Tribe,
) -> Result<Option<usize>, Error<E>> {
    if tribe.names.iter().any(|&i| {
        rules
            .names
            .get(i)
            .is_none_or(|name| name.tribe != tribe.index)
    }) {
        return Err(Error::Missing("tribe name definition index"));
    }
    let mut unused = tribe.names.clone();
    let mut dead = Vec::new();
    let mut living = Vec::new();
    for instance in &state.retained().barbarian.instances {
        let name = rules
            .names
            .iter()
            .position(|name| name.hash == instance.tribe_name_hash)
            .ok_or(Error::Missing("instance tribe name definition"))?;
        if rules.names[name].tribe == tribe.index {
            unused.retain(|&i| i != name);
            if instance.living_raw == 0 {
                dead.push(name);
            } else {
                living.push(name);
            }
        }
    }
    let available = if !unused.is_empty() {
        &unused
    } else {
        dead.retain(|i| !living.contains(i));
        if !dead.is_empty() {
            &dead
        } else {
            &tribe.names
        }
    };
    if available.is_empty() {
        return Ok(None);
    }
    let draw = state
        .game_sync_rand(available.len() as u16)
        .map_err(Error::Random)?;
    Ok(Some(available[usize::from(draw)]))
}

/// Actual Barbarian constructed-signal subscriber, called at its retained slot.
pub fn on_created<C: Context>(
    state: &mut LoadedState,
    context: &mut C,
    rules: &Rules,
    plot: usize,
) -> Result<(), Error<C::Error>> {
    let manager = &state.retained().barbarian;
    if manager.flag_168_raw != 0
        || state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("camp plot"))?
            .improvement_hash
            != manager.improvement_hash
    {
        return Ok(());
    }
    if rules.clans_active {
        return Err(Error::Unsupported("clan tribe selection"));
    }
    let loc = location(state, plot).map_err(Error::Missing)?;
    let mut selected = None;
    for tribe in &rules.tribes {
        if tribe.coastal && !context.is_coastal(state, loc).map_err(Error::Effect)? {
            continue;
        }
        if let Some((resource, radius)) = tribe.required_resource {
            if !context
                .required_resource(state, loc, resource, radius)
                .map_err(Error::Effect)?
            {
                continue;
            }
        }
        selected = Some(tribe);
        break;
    }
    let tribe = selected.ok_or(Error::Missing("eligible tribe definition"))?;
    let name_index = find_name::<C::Error>(state, rules, tribe)?.ok_or(Error::Unsupported(
        "no tribe name: improvement removal callback",
    ))?;
    let name = rules
        .names
        .get(name_index)
        .ok_or(Error::Missing("tribe name"))?;
    let turns = context
        .scaled_spawn_turns(state, name.turns_to_spawn.unwrap_or(tribe.turns_to_spawn))
        .map_err(Error::Effect)?;
    let index = state.retained().barbarian.instances.len();
    let owner = state.retained().barbarian.player_ordinal;
    let instance = initialize(owner, index as i32, loc, name, tribe.percent_ranged, turns);
    let scouts = instance.max_num_scouts;
    {
        let manager = state.barbarian_manager_mut().map_err(Error::State)?;
        manager.value_e0 = manager.value_e0.wrapping_add(1);
        manager.value_f8 = manager.value_f8.wrapping_add(1);
        manager
            .camp_locations
            .as_mut()
            .ok_or(Error::Missing("camp location history"))?
            .push(CampLocation {
                plot_index: plot as i32,
            });
        manager.instances.push(instance);
    }
    if !rules.world_builder {
        context
            .spawn_initial_units(state, index, loc, scouts)
            .map_err(Error::Effect)?;
    }
    let food = context
        .local_food(state, loc)
        .map_err(Error::Effect)?
        .max(1);
    state
        .set_barbarian_local_food(index, food)
        .map_err(Error::State)?;
    context.camp_created(state, index).map_err(Error::Effect)?;
    Ok(())
}

fn initialize(
    owner: i32,
    index: i32,
    location: PlotCoord,
    name: &Name,
    ranged: i32,
    turns: i32,
) -> Instance {
    let stats = ClanPlayerStats {
        raid_count: 0,
        hire_count: 0,
        bribe_count: 0,
        incite_count: 0,
        ransom_count: 0,
        unit_hashes_hired: Vec::new(),
        last_turn_raided: -1,
        last_turn_hired: -1,
        last_turn_bribed: -1,
        last_turn_incited: -1,
        last_turn_ransomed: -1,
    };
    Instance {
        wire_version: 15,
        player_ordinal: owner,
        tribe_name_hash: name.hash,
        tribe_index: index,
        camp_location: c6_core::game_barbarian::PlotCoord {
            x: location.x,
            y: location.y,
        },
        living_raw: 1,
        rampage_target_ordinal: -1,
        boldness: 0,
        max_num_scouts: name.scouts.unwrap_or(1),
        cur_num_scouts: 0,
        turns_since_scout_death: 0,
        max_standard_military_units: name.military.unwrap_or(5),
        num_standard_military_units: 0,
        turns_since_last_warrior_spawn: 0,
        percent_ranged_units: name.percent_ranged.unwrap_or(ranged),
        turns_between_warrior_spawn: turns,
        required_melee_spawn: 0,
        required_ranged_spawn: 0,
        required_siege_spawn: 0,
        required_support_spawn: 0,
        needed_unit_tags: Vec::new(),
        scouting_behavior_tree: u32::MAX,
        raiding_behavior_tree: u32::MAX,
        city_attack_behavior_tree: u32::MAX,
        scouted_cities: Vec::new(),
        raided_cities: Vec::new(),
        raiding_score: 0,
        turns_in_enemy_territory: 0,
        properties: None,
        clan_tech_hashes: Some(Vec::new()),
        tech_steal_turns_cooldown: -1,
        conversion_points: 0,
        player_bribe_turns_remaining: Some(vec![0; 64]),
        clan_player_stats: Some(vec![stats; 64]),
        incite_targets: Some(Vec::new()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialized_instance_has_live_clan_arrays_and_name_overrides() {
        let name = Name {
            hash: 3,
            tribe: 0,
            scouts: Some(2),
            military: None,
            turns_to_spawn: None,
            percent_ranged: Some(33),
        };
        let instance = initialize(63, 4, PlotCoord { x: 22, y: 4 }, &name, 25, 17);
        assert_eq!(
            (
                instance.tribe_index,
                instance.max_num_scouts,
                instance.max_standard_military_units,
                instance.turns_between_warrior_spawn,
                instance.percent_ranged_units
            ),
            (4, 2, 5, 17, 33)
        );
        assert_eq!(
            instance.player_bribe_turns_remaining.as_ref().unwrap(),
            &[0; 64]
        );
        let stats = instance.clan_player_stats.as_ref().unwrap();
        assert_eq!(stats.len(), 64);
        assert!(stats.iter().all(|s| s.last_turn_raided == -1
            && s.raid_count == 0
            && s.unit_hashes_hired.is_empty()));
    }
}

/// The reached first-placement SetImprovementOwner services (0x2bc9260).
/// Visibility means ChangeVisibilityCount(+1, NO_HIDDEN, true, false, -1, 1,
/// default ComponentID), including the actual subscriber and RNG effects.
pub trait OwnerContext {
    type Error;
    fn valid_player(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    fn adjacent(
        &self,
        state: &LoadedState,
        plot: usize,
        direction: u8,
    ) -> Result<Option<usize>, Self::Error>;
    fn visibility(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        plot: usize,
    ) -> Result<(), Self::Error>;
    fn owner_changed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
pub fn set_improvement_owner<C: OwnerContext>(
    state: &mut LoadedState,
    context: &mut C,
    plot: usize,
    owner: i32,
) -> Result<(), Error<C::Error>> {
    let old = state
        .world()
        .map()
        .plots
        .get(plot)
        .ok_or(Error::Missing("improvement owner plot"))?
        .improvement_owner;
    if i32::from(old) == owner {
        return Ok(());
    }
    if old != -1 {
        return Err(Error::Unsupported("improvement owner replacement"));
    }
    let valid = owner != -1 && context.valid_player(state, owner).map_err(Error::Effect)?;
    let new_owner = if valid {
        i8::try_from(owner).map_err(|_| Error::Missing("improvement owner ordinal"))?
    } else {
        -1
    };
    state
        .world_mut()
        .map_err(Error::State)?
        .plot_mut(plot)
        .ok_or(Error::Missing("improvement owner plot"))?
        .improvement_owner = new_owner;
    if valid {
        context
            .visibility(state, owner, plot)
            .map_err(Error::Effect)?;
        for direction in 0..6 {
            if let Some(adjacent) = context
                .adjacent(state, plot, direction)
                .map_err(Error::Effect)?
            {
                context
                    .visibility(state, owner, adjacent)
                    .map_err(Error::Effect)?;
            }
        }
        state
            .add_player_improvement(owner, plot as i32)
            .map_err(Error::State)?;
    }
    context
        .owner_changed(state, plot, i32::from(new_owner))
        .map_err(Error::Effect)
}

impl Rules {
    /// Stock builders 0x21e9f30 and 0x21ea600 both select ORDER BY rowid.
    pub fn from_bound_rules(
        rules: &crate::Rules,
        authority: &crate::resource_binding::ResourceAuthority<'_>,
        clans_active: bool,
        world_builder: bool,
    ) -> Result<Self, crate::Error> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|e| crate::Error(format!("camp rules resource binding: {e}")))?;
        let tribes = crate::unit_initialization_definitions::collection(
            rules,
            "BarbarianTribes",
            "TribeType",
        )
        .map_err(|e| crate::Error(format!("camp tribe registry: {e}")))?;
        let names = crate::unit_initialization_definitions::collection(
            rules,
            "BarbarianTribeNames",
            "TribeNameType",
        )
        .map_err(|e| crate::Error(format!("camp name registry: {e}")))?;
        Self::from_rules(rules, &tribes, &names, clans_active, world_builder)
    }
    /// Resolve values from the bound database using explicitly supplied native
    /// collection order. SQL row IDs do not stand in for native collection IDs.
    pub fn from_rules(
        rules: &crate::Rules,
        tribes: &crate::registry::LinuxDefinitionRegistry,
        names: &crate::registry::LinuxDefinitionRegistry,
        clans_active: bool,
        world_builder: bool,
    ) -> Result<Self, crate::Error> {
        let tribe_table = rules
            .gameplay()
            .table("BarbarianTribes")
            .ok_or_else(|| crate::Error("BarbarianTribes table absent".into()))?;
        let name_table = rules
            .gameplay()
            .table("BarbarianTribeNames")
            .ok_or_else(|| crate::Error("BarbarianTribeNames table absent".into()))?;
        fn int(row: crate::Row<'_>, key: &str) -> Result<i32, crate::Error> {
            i32::try_from(row.integer(key)?).map_err(|_| crate::Error(format!("{key} outside i32")))
        }
        fn optional_int(row: crate::Row<'_>, key: &str) -> Result<Option<i32>, crate::Error> {
            if matches!(row.get(key)?, crate::SqlValue::Null) {
                Ok(None)
            } else {
                int(row, key).map(Some)
            }
        }
        let mut projected_tribes = Vec::new();
        for (index, entry) in tribes.entries().iter().enumerate() {
            let row = tribe_table
                .rows()
                .find(|row| row.row_id() == Some(entry.database_rowid))
                .ok_or_else(|| crate::Error("tribe native row absent".into()))?;
            if row.text("TribeType")?.as_bytes() != entry.type_name {
                return Err(crate::Error("tribe native row identity".into()));
            }
            let resource = match row.get("RequiredResource")? {
                crate::SqlValue::Null => None,
                _ => Some((
                    crate::registry::linux_type_hash(row.text("RequiredResource")?.as_bytes()),
                    int(row, "ResourceRange")? as u32,
                )),
            };
            projected_tribes.push(Tribe {
                index: index as i32,
                coastal: int(row, "IsCoastal")? != 0,
                required_resource: resource,
                percent_ranged: int(row, "PercentRangedUnits")?,
                turns_to_spawn: int(row, "TurnsToWarriorSpawn")?,
                names: Vec::new(),
            });
        }
        let mut projected_names = Vec::new();
        for entry in names.entries() {
            let row = name_table
                .rows()
                .find(|row| row.row_id() == Some(entry.database_rowid))
                .ok_or_else(|| crate::Error("tribe name native row absent".into()))?;
            if row.text("TribeNameType")?.as_bytes() != entry.type_name {
                return Err(crate::Error("tribe name native row identity".into()));
            }
            let tribe = tribes
                .find_index(crate::registry::linux_type_hash(
                    row.text("TribeType")?.as_bytes(),
                ))
                .ok_or_else(|| crate::Error("tribe name type reference".into()))?;
            projected_tribes[tribe].names.push(projected_names.len());
            projected_names.push(Name {
                hash: crate::registry::linux_type_hash(&entry.type_name),
                tribe: tribe as i32,
                scouts: optional_int(row, "NumScouts")?,
                military: optional_int(row, "NumMilitary")?,
                turns_to_spawn: optional_int(row, "TurnsToWarriorSpawn")?,
                percent_ranged: optional_int(row, "PercentRangedUnits")?,
            });
        }
        Ok(Self {
            tribes: projected_tribes,
            names: projected_names,
            clans_active,
            world_builder,
        })
    }
}

/// RestoreVisualState's reporting service; revealed layer updates are performed
/// directly on canonical visibility records, as in the existing route setter.
pub trait VisualContext {
    type Error;
    fn alive_ids(&self) -> Result<Vec<i32>, Self::Error>;
    fn messages_disabled(&self) -> bool;
    fn local_observer(&self) -> Option<i32>;
    fn is_revealed(
        &self,
        state: &LoadedState,
        player: i32,
        plot: usize,
    ) -> Result<bool, Self::Error>;
    fn revealed_state(
        &self,
        state: &LoadedState,
        player: i32,
        plot: usize,
    ) -> Result<i32, Self::Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        event: u32,
        argument: i32,
    ) -> Result<(), Self::Error>;
}
/// Source 0x2a39380. Improvement knowledge uses native ordinals, never hashes.
/// 0x2bc1356 binds manager+0x18 to layer hash f7a356cf; 0x2bc1910
/// edits that own-group i16 cell without callbacks or RNG.
pub fn restore_visual_state<C: VisualContext>(
    state: &mut LoadedState,
    context: &mut C,
    plot: usize,
    improvement: i32,
) -> Result<(), Error<C::Error>> {
    const LAYER: u32 = 0xf7a356cf;
    if improvement == -1 {
        return Ok(());
    }
    let owner = i32::from(
        state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("restored improvement plot"))?
            .improvement_owner,
    );
    if !context.messages_disabled() {
        context
            .report(state, plot, 0xe5b20b82, owner)
            .map_err(Error::Effect)?;
    }
    for player in context.alive_ids().map_err(Error::Effect)? {
        if !context
            .is_revealed(state, player, plot)
            .map_err(Error::Effect)?
        {
            continue;
        }
        let visibility = state
            .world_mut()
            .map_err(Error::State)?
            .visibility_mut(player)
            .ok_or(Error::Missing("restored improvement visibility"))?;
        // Native EditLayerIndexEntry may return null for an absent layer/cell.
        if let Some(cell) = visibility
            .layers
            .iter_mut()
            .find(|l| l.type_hash == LAYER)
            .and_then(|l| l.values.get_mut(plot))
        {
            *cell = improvement as i16;
        }
    }
    // Player1000 is the separate omniscient display instance. Its setter only
    // writes the same layer cell, with no callbacks or RNG (0x2bc1910).
    if let Some(observer) = state
        .world_mut()
        .map_err(Error::State)?
        .visibility_mut(1000)
    {
        if let Some(cell) = observer
            .layers
            .iter_mut()
            .find(|l| l.type_hash == LAYER)
            .and_then(|l| l.values.get_mut(plot))
        {
            *cell = improvement as i16;
        }
    }
    if !context.messages_disabled() {
        let observer = context
            .local_observer()
            .ok_or(Error::Missing("restored improvement local observer"))?;
        let visibility = state
            .world()
            .visibility(observer)
            .ok_or(Error::Missing("restored improvement observer"))?;
        let known = visibility
            .layers
            .iter()
            .find(|l| l.type_hash == LAYER)
            .and_then(|l| l.values.get(plot))
            .copied();
        if known == Some(improvement as i16) {
            let revealed_state = context
                .revealed_state(state, observer, plot)
                .map_err(Error::Effect)?;
            context
                .report(state, plot, 0xf391d7b3, revealed_state)
                .map_err(Error::Effect)?;
        }
    }
    Ok(())
}
