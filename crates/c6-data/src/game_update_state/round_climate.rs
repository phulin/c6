//! Linux XP2 Climate.DoTurn296b470, supported pre-emissions branch.
//! Uses the activation owner's live resource rows. Serialized resource records
//! are consumed at initialization, never retained as a second live owner.
use super::activation::cityless_managers;
use c6_core::{
    game_climate,
    loaded_state::{ActionStamp, LoadedState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Missing(&'static str),
    Requires(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round climate: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Omitted wire fields require live before inputs, never read-default guesses.
#[derive(Debug)]
pub struct LiveFields {
    pub co2_by_usage_type: Vec<i32>,
    pub active_storms: Vec<i32>,
    pub active_droughts: Vec<i32>,
    pub active_one_offs: Vec<i32>,
    pub notification_count: i32,
    pub last_turn_worst_co2_footprint: i32,
    pub last_turn_average_co2_footprint: i32,
    pub next_ice_loss_turns: i32,
    pub next_sea_level_rise_turns: i32,
    pub halt_flood_fertility_raw: u8,
    pub halt_storm_fertility_raw: u8,
    pub fertility_removal_chance: i32,
}
pub struct Runtime {
    instance: ActionStamp,
    /// resources_expended is always empty; current resources live in cityless.
    pub manager: game_climate::Manager,
    pub live: LiveFields,
    pub storms: Option<super::round_storm::Context>,
}
impl Runtime {
    pub fn from_initial(
        state: &LoadedState,
        manager: game_climate::Manager,
        live: LiveFields,
    ) -> Result<Self, Error> {
        // Nonempty wire records need an explicit consuming ownership transfer
        // into cityless; do not silently substitute read-default Q8 values.
        if !manager.resources_expended.is_empty() {
            return Err(Error::Requires(
                "Climate resource wire/live ownership transfer",
            ));
        }
        if live.notification_count < 0 {
            return Err(Error::Missing("Climate notification count"));
        }
        Ok(Self {
            instance: state.action_stamp(),
            manager,
            live,
            storms: None,
        })
    }
    /// Install current rules/configuration for new storms; wire imports with
    /// existing storms require their omitted start plots and are unsupported.
    pub fn install_storm_context(
        &mut self,
        state: &LoadedState,
        context: super::round_storm::Context,
    ) -> Result<(), Error> {
        self.validate(state)?;
        if self.storms.is_some()
            || !self.manager.storms.is_empty()
            || !self.live.active_storms.is_empty()
        {
            return Err(Error::Requires("Climate existing storm live fields"));
        }
        self.storms = Some(context);
        Ok(())
    }
    pub fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::Missing("Climate instance"));
        }
        if !self.manager.resources_expended.is_empty() {
            return Err(Error::Missing("Climate has duplicated resource owner"));
        }
        Ok(())
    }
}

/// Native definition order, collection ordinals and GameSpeed cost multiplier.
pub struct CarbonRules<'a> {
    /// (resource ordinal, PowerProvided, CO2perkWh), in native resource order.
    pub resources: &'a [(i32, i32, i32)],
    pub game_speed_cost_multiplier: i32,
    /// Complete Features.IsRemovable definition projection.
    pub removable_feature_hashes: &'a [u32],
}
fn footprint(
    manager: &game_climate::Manager,
    rows: &[cityless_managers::ClimateConsumption],
    owner: i32,
    rules: &CarbonRules<'_>,
) -> i32 {
    let mut carbon = 0i64;
    for &(resource, power, co2) in rules.resources {
        let amount = rows
            .iter()
            .find(|r| r.owner == owner && r.resource == resource)
            .map_or(0, |r| r.words[0] >> 8);
        let energy = i64::from(amount).wrapping_mul(i64::from(power));
        if energy > 0 {
            carbon = carbon.wrapping_add(energy.wrapping_mul(i64::from(co2)));
        }
    }
    if rules.game_speed_cost_multiplier > 0 {
        carbon = (carbon / i64::from(rules.game_speed_cost_multiplier)).wrapping_mul(100);
    }
    carbon = carbon.wrapping_sub(i64::from(
        manager
            .carbon_sequestered
            .iter()
            .find(|r| r.player_ordinal == owner)
            .map_or(0, |r| r.amount),
    ));
    if carbon > 0x1f3_ffff_ffff {
        i32::MAX
    } else {
        (carbon / 1000) as i32
    }
}

/// Executes all mandatory pre-emission scalar/cache updates; a positive
/// deforestation/sea-level branch reports its next required implementation.
/// Caller supplies the SAME current alive-major list and turn as Game.Update.
pub fn do_turn(
    state: &LoadedState,
    runtime: &mut Runtime,
    cityless: &cityless_managers::Runtime,
    alive_major_ids: &[i32],
    current_turn: i32,
    rules: &CarbonRules<'_>,
) -> Result<(), Error> {
    runtime.validate(state)?;
    cityless
        .validate(state)
        .map_err(|_| Error::Missing("Climate consumption instance"))?;
    runtime.live.last_turn_worst_co2_footprint = 0;
    let mut total = 0i32;
    for &owner in alive_major_ids {
        let index = usize::try_from(owner).map_err(|_| Error::Missing("Climate player ordinal"))?;
        let previous = *runtime
            .manager
            .this_turn_co2_footprint
            .get(index)
            .ok_or(Error::Missing("Climate current footprint slot"))?;
        *runtime
            .manager
            .last_turn_co2_footprint
            .get_mut(index)
            .ok_or(Error::Missing("Climate last footprint slot"))? = previous;
        let current = footprint(&runtime.manager, &cityless.climate, owner, rules);
        runtime.manager.this_turn_co2_footprint[index] = current;
        let delta = current.wrapping_sub(previous);
        runtime.live.last_turn_worst_co2_footprint =
            runtime.live.last_turn_worst_co2_footprint.max(delta);
        total = total.wrapping_add(delta);
    }
    runtime.live.last_turn_average_co2_footprint = total
        / i32::try_from(alive_major_ids.len())
            .map_err(|_| Error::Missing("Climate major count"))?
            .max(1);
    if runtime.manager.total_forested_tiles == -1 {
        let mut count = 0i32;
        for plot in &state.world().map().plots {
            if rules.removable_feature_hashes.contains(&plot.feature_hash) {
                count = count.wrapping_add(1);
            }
        }
        runtime.manager.total_forested_tiles = count;
    }
    if runtime.manager.turn_first_resource_consumed == -1 && !cityless.climate.is_empty() {
        runtime.manager.turn_first_resource_consumed = current_turn.wrapping_sub(1);
    }
    if runtime.manager.turn_first_resource_consumed >= 0 {
        return Err(Error::Requires(
            "Climate deforestation and sea-level branch",
        ));
    }
    Ok(())
}

/// 29722a0 preserves native drought, storm, one-off ordering. Empty lists have
/// no effects or RNG draws; positive effects stop before native mutation.
pub fn continue_events(state: &LoadedState, runtime: &mut Runtime) -> Result<(), Error> {
    runtime.validate(state)?;
    if !runtime.live.active_droughts.is_empty() {
        return Err(Error::Requires("Climate CheckForEndingDroughts"));
    }
    if !runtime.live.active_storms.is_empty() {
        return Err(Error::Requires("Climate MoveActiveStorms"));
    }
    if !runtime.live.active_one_offs.is_empty() {
        return Err(Error::Requires("Climate DoActiveOneOffs"));
    }
    Ok(())
}
pub fn send_notifications(state: &LoadedState, runtime: &mut Runtime) -> Result<(), Error> {
    runtime.validate(state)?;
    if runtime.live.notification_count != 0 {
        return Err(Error::Requires("Climate SendNotifications effects"));
    }
    Ok(())
}

/// Source2968b90: a zero total footprint yields zero temperature for every
/// map definition branch. Nonzero totals require the current map/rules divisor.
pub fn temperature_change(
    state: &LoadedState,
    runtime: &Runtime,
    cityless: &cityless_managers::Runtime,
) -> Result<f32, Error> {
    runtime.validate(state)?;
    cityless
        .validate(state)
        .map_err(|_| Error::Missing("Climate consumption instance"))?;
    if !cityless.climate.is_empty() || !runtime.manager.carbon_sequestered.is_empty() {
        return Err(Error::Requires(
            "Climate nonzero temperature resource/rules calculation",
        ));
    }
    Ok(0.0)
}
/// Source2968b00 adds the live realism contribution to temperature's Q8 term.
/// The supported zero-emissions branch has a zero temperature contribution.
pub fn climate_change_level(
    state: &LoadedState,
    runtime: &Runtime,
    cityless: &cityless_managers::Runtime,
    realism_climate_change_points: i32,
) -> Result<i32, Error> {
    temperature_change(state, runtime, cityless)?;
    Ok(realism_climate_change_points)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carbon_conversion_preserves_q8_and_native_division_order() {
        // Use the existing codec to avoid manufacturing a second Manager layout.
        let mut bytes = Vec::new();
        for word in [14i32, 0, 0, 0, 0, 0, 0, 10, -1, 0, 0, 0, 0, 0, 0] {
            bytes.extend(word.to_le_bytes());
        }
        let manager =
            game_climate::Manager::decode(&bytes, game_climate::Limits::default()).unwrap();
        let rows = [cityless_managers::ClimateConsumption {
            owner: 2,
            resource: 8,
            words: [3 * 256 + 255, 900, 800, 700],
        }];
        let rules = CarbonRules {
            resources: &[(8, 1000, 2)],
            game_speed_cost_multiplier: 150,
            removable_feature_hashes: &[],
        };
        assert_eq!(footprint(&manager, &rows, 2, &rules), 4);
        assert_eq!(footprint(&manager, &rows, 3, &rules), 0);
        assert_eq!(rows[0].words, [1023, 900, 800, 700]);
    }
}
