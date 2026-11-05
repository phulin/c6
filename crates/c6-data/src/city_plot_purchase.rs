//! Explicit Linux City plot gold purchase on canonical territory and Treasury.
//! All callback services borrow the ongoing stock owners; unsupported listeners
//! retain their actual mutation prefix instead of completing the request.
use crate::{
    city_territory,
    loaded_actions::{city_range_attack, stock_notifications},
    Rules,
};
use c6_core::{
    city_territory::CityId,
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    maximum: i32,
    distance_multiplier: i32,
    distance_divisor: i32,
    base: i32,
    divisor: i32,
    levels: Vec<(u32, bool)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err("Linux plot purchase rules profile".into());
        }
        let parameters = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or("GlobalParameters")?;
        let parameter = |name: &str| {
            parameters
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .and_then(|r| r.get("Value").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or_else(|| format!("plot purchase {name}"))
        };
        let levels = rules
            .gameplay()
            .table("CivilizationLevels")
            .ok_or("CivilizationLevels")?
            .rows()
            .map(|r| {
                Ok((
                    crate::registry::linux_type_hash(
                        r.text("CivilizationLevelType")
                            .map_err(|e| format!("{e:?}"))?
                            .as_bytes(),
                    ),
                    r.integer("CanAnnexTilesWithGold")
                        .map_err(|e| format!("{e:?}"))?
                        != 0,
                ))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            maximum: parameter("CITY_MAX_BUY_PLOT_RANGE")?,
            distance_multiplier: parameter("PLOT_INFLUENCE_DISTANCE_MULTIPLIER")?,
            distance_divisor: parameter("PLOT_INFLUENCE_DISTANCE_DIVISOR")?,
            base: parameter("PLOT_BUY_BASE_COST")?,
            divisor: parameter("PURCHASE_DIVISOR")?,
            levels,
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), String> {
        if state.binding() != &self.binding {
            return Err("plot purchase definitions binding".into());
        }
        Ok(())
    }
}
/// Immutable current configuration and shared canonical economic definitions.
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub territory: &'a city_territory::Definitions,
    pub sight: &'a city_range_attack::Definitions,
    pub research: &'a crate::research::districts::Definitions,
    pub civics: &'a crate::research::CivicGraph,
    pub unit_costs: &'a crate::research_unit_costs::Definitions,
    /// Actual initialized GameFeatures.HasFeature(0x44f5fe2a).
    pub enabled: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    Disabled,
    CivilizationLevel,
    NoPurchasablePlots,
    InvalidLocation,
    UnpurchasablePlot,
    InsufficientGold,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected(Rejection),
    Purchased { plot: u32, gold_cost: i32 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    PlotPurchased {
        city: ComponentId,
        location: PlotCoord,
    },
    CommandComplete {
        city: ComponentId,
    },
}

fn divide(a: i32, b: i32) -> Result<i32, String> {
    a.checked_div(b)
        .ok_or("plot purchase native integer division".into())
}
fn index(state: &LoadedState, at: PlotCoord) -> Result<Option<u32>, String> {
    crate::unit_placement_services::plot_index_at::<()>(state, at)
        .map_err(|e| format!("plot purchase geometry: {e:?}"))
}
fn adjacent(state: &LoadedState, at: PlotCoord, direction: usize) -> Result<Option<u32>, String> {
    const DX: [i32; 6] = [0, 1, 1, 0, -1, -1];
    const DY: [i32; 6] = [1, 0, -1, -1, 0, 1];
    c6_core::found_city::placement::plot_from_delta(
        state.world().map(),
        at,
        DX[direction],
        DY[direction],
        1,
    )
    .map(|v| v.map(|i| i as u32))
    .map_err(|e| format!("plot purchase adjacency: {e:?}"))
}
/// Gold.GetPlotPurchaseCost: negative means unavailable. Adjacency is to any
/// territory of this player, while GetPurchasablePlots additionally tests city.
pub fn gold_cost(
    state: &LoadedState,
    city: CityId,
    location: PlotCoord,
    ignore_adjacency: bool,
    inputs: &Inputs<'_>,
) -> Result<i32, String> {
    inputs.definitions.check(state)?;
    let city_record = state
        .city(city.owner, city.id)
        .map_err(|e| format!("{e:?}"))?;
    let Some(plot) = index(state, location)? else {
        return Ok(-1);
    };
    let p = &state.world().map().plots[plot as usize];
    let center = PlotCoord {
        x: city_record.header.location_x,
        y: city_record.header.location_y,
    };
    let map = state.world().map();
    let distance = c6_core::geometry::plot_distance(
        map.header.width,
        map.header.wrap_x,
        location.x,
        location.y,
        center.x,
        center.y,
    );
    if distance > inputs.definitions.maximum
        || p.city_attributes.as_ref().is_some_and(|a| a.player != -1)
    {
        return Ok(-1);
    }
    let mut connected = false;
    for direction in 0..6 {
        if let Some(i) = adjacent(state, location, direction)? {
            if state.world().map().plots[i as usize]
                .city_attributes
                .as_ref()
                .is_some_and(|a| i32::from(a.player) == city.owner)
            {
                connected = true;
                break;
            }
        }
    }
    if !connected && !ignore_adjacency {
        return Ok(-1);
    }
    let defs = inputs.definitions;
    let base = divide(
        distance.wrapping_mul(defs.distance_multiplier),
        defs.distance_divisor,
    )?
    .wrapping_mul(defs.base);
    let base = divide(base, 100)?;
    let percent = inputs
        .research
        .completion_percent(state, city.owner, inputs.civics)
        .map_err(|e| format!("plot purchase research: {e:?}"))?;
    let escalated = divide(
        divide(500_i32.wrapping_mul(base), 100)?
            .wrapping_sub(base)
            .wrapping_mul(percent),
        100,
    )?
    .wrapping_add(base);
    let mut cost = c6_core::research_acquisition::units::adjust_for_game_speed(
        escalated,
        inputs.unit_costs.economic().speed_cost_multiplier,
    );
    cost = cost.wrapping_sub(
        cost.checked_rem(defs.divisor)
            .ok_or("plot purchase divisor")?,
    );
    let modifier = city_record.gold.plot_cost_modifier;
    if modifier != 0 {
        cost = divide(modifier.wrapping_add(100).wrapping_mul(cost), 100)?;
    }
    if p.terrain_hash != u32::MAX {
        let modifier = city_record
            .gold
            .plot_cost_terrain_modifier
            .iter()
            .rfind(|r| r.definition_hash == p.terrain_hash)
            .ok_or("current plot terrain cost modifier")?
            .value;
        if modifier != 0 {
            cost = divide(modifier.wrapping_add(100).wrapping_mul(cost), 100)?;
        }
    }
    Ok(cost)
}
/// Exact CityHexspace index order, preserving wrapped duplicates.
pub fn purchasable_plots(
    state: &LoadedState,
    city: CityId,
    inputs: &Inputs<'_>,
) -> Result<Vec<u32>, String> {
    let c = state
        .city(city.owner, city.id)
        .map_err(|e| format!("{e:?}"))?;
    let center = PlotCoord {
        x: c.header.location_x,
        y: c.header.location_y,
    };
    let mut result = Vec::new();
    for i in 1..c6_core::city_citizen_assignment::NUM_PLOTS as i32 {
        let Some(at) =
            c6_core::city_citizen_assignment::plot_location(state.world().map(), center, i)
                .map_err(|e| format!("{e:?}"))?
        else {
            continue;
        };
        let mut connected = false;
        for direction in 0..6 {
            if let Some(adj) = adjacent(state, at, direction)? {
                if state.world().map().plots[adj as usize]
                    .city_attributes
                    .as_ref()
                    .is_some_and(|a| i32::from(a.player) == city.owner && a.owning_city == city.id)
                {
                    connected = true;
                    break;
                }
            }
        }
        if connected && gold_cost(state, city, at, false, inputs)? > 0 {
            result.push(index(state, at)?.ok_or("purchasable plot")?);
        }
    }
    Ok(result)
}
pub trait Effects<'a>: city_territory::Effects<Error = String> {
    fn stock(&mut self) -> &mut stock_notifications::Runtime<'a>;
    /// Dispatch every actual current GameSignals+0x10e0 consumer in order.
    fn plot_purchased(
        &mut self,
        state: &mut LoadedState,
        city: CityId,
        plot: u32,
    ) -> Result<(), String>;
}
pub trait Domains<'a> {
    type Effects<'s>: Effects<'a>
    where
        Self: 's,
        'a: 's;
    fn borrow<'s>(
        &'s mut self,
        stock: &'s mut stock_notifications::Runtime<'a>,
        owners: &'s mut stock_notifications::unit_creation::ModifierOwners,
    ) -> Self::Effects<'s>
    where
        'a: 's;
}
pub struct Current<'a> {
    pub territory: &'a mut city_territory::Runtime,
    pub sight: &'a mut crate::unit_sight_state::Runtime,
    /// Existing actor for visibility's adapter; source visibility CID is NULL.
    pub actor: (i32, u32),
}
/// Purchase.Start's positive-cost/affordability branch, after CanStart's feature
/// and list checks. All canonical callback mutations remain on failure.
pub fn purchase<'a, E: Effects<'a>>(
    state: &mut LoadedState,
    city: CityId,
    location: PlotCoord,
    inputs: &Inputs<'_>,
    current: &mut Current<'_>,
    effects: &mut E,
    reports: &mut Vec<Report>,
) -> Result<Outcome, String> {
    inputs.definitions.check(state)?;
    effects.stock().check(state).map_err(|e| format!("{e:?}"))?;
    if !inputs.enabled {
        return Ok(Outcome::Rejected(Rejection::Disabled));
    }
    state
        .city(city.owner, city.id)
        .map_err(|e| format!("{e:?}"))?;
    let level = current
        .territory
        .players
        .iter()
        .find(|r| r.owner == city.owner)
        .and_then(|r| r.civilization_level)
        .ok_or("current plot purchase civilization level")?;
    if !inputs
        .definitions
        .levels
        .iter()
        .any(|r| r.0 == level && r.1)
    {
        return Ok(Outcome::Rejected(Rejection::CivilizationLevel));
    }
    if purchasable_plots(state, city, inputs)?.is_empty() {
        return Ok(Outcome::Rejected(Rejection::NoPurchasablePlots));
    }
    if !c6_core::geometry::plot_valid(location.x, location.y) {
        return Ok(Outcome::Rejected(Rejection::InvalidLocation));
    }
    let cost = gold_cost(state, city, location, false, inputs)?;
    if cost <= 0 {
        return Ok(Outcome::Rejected(Rejection::UnpurchasablePlot));
    }
    let cost = {
        // CanAffordPlot recomputes the current cost before debit.
        let fresh = gold_cost(state, city, location, false, inputs)?;
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == city.owner)
            .ok_or("current purchase Player")?;
        if fresh > p.treasury.gold_balance_q8 >> 8 {
            return Ok(Outcome::Rejected(Rejection::InsufficientGold));
        }
        cost
    };
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == city.owner)
        .ok_or("current purchase Player")?;
    let before = p.treasury.gold_balance_q8;
    let next = before.saturating_add(cost.wrapping_neg().wrapping_shl(8));
    if before != next {
        state
            .treasury_mut(city.owner)
            .map_err(|e| format!("{e:?}"))?
            .gold_balance_q8 = next;
        crate::game_update_state::activation::treasury_deals::Effects::gold_changed(
            effects.stock(),
            state,
            city.owner,
        )
        .map_err(|e| format!("plot purchase GoldChanged: {e:?}"))?;
    }
    let mut territory = city_territory::Context {
        state,
        runtime: current.territory,
        sight: current.sight,
        sight_definitions: inputs.sight,
        definitions: inputs.territory,
        effects,
        city,
        actor: current.actor,
    };
    c6_core::city_territory::annex_plot(&mut territory, location)
        .map_err(|e| format!("plot purchase AnnexPlot: {e:?}"))?;
    let component = ComponentId {
        owner: city.owner as i16,
        component_type: 2,
        id: city.id,
    };
    reports.push(Report::PlotPurchased {
        city: component,
        location,
    });
    let plot = index(state, location)?.ok_or("purchased plot")?;
    effects.plot_purchased(state, city, plot)?;
    if effects
        .stock()
        .local_player()
        .ok_or("plot purchase current local player")?
        == city.owner
    {
        reports.push(Report::CommandComplete { city: component });
    }
    Ok(Outcome::Purchased {
        plot,
        gold_cost: cost,
    })
}

/// Loan the initialized callback services with the stock runtime's canonical
/// modifier/requirement owners. Rejoin the same mutated owners on every result.
#[allow(clippy::too_many_arguments)]
pub fn purchase_current<'a, D: Domains<'a>>(
    state: &mut LoadedState,
    backend: &mut stock_notifications::Runtime<'a>,
    city: CityId,
    location: PlotCoord,
    inputs: &Inputs<'_>,
    current: &mut Current<'_>,
    domains: &mut D,
    reports: &mut Vec<Report>,
) -> Result<Outcome, String> {
    backend
        .with_modifier_owners(|stock, owners| {
            let mut effects = domains.borrow(stock, owners);
            purchase(
                state,
                city,
                location,
                inputs,
                current,
                &mut effects,
                reports,
            )
        })
        .map_err(|e| format!("plot purchase canonical owners: {e:?}"))?
}
