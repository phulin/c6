//! Current appeal queries reached by ordinary building completion.
//! Changed appeal still requires its actual Game signal/report continuation.
use super::{buildings::index, Error};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    city::City,
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    map::{Map, Plot, PlotFlag},
    unit::PlotCoord,
};
struct Definition {
    hash: u32,
    appeal: i32,
    water: bool,
    mountain: bool,
    wonder: bool,
    civic: Option<u32>,
}
pub struct Definitions {
    inputs: InputIdentities,
    terrains: Vec<Definition>,
    features: Vec<Definition>,
    improvements: Vec<Definition>,
    districts: Vec<Definition>,
    lake_max: i32,
}
impl Definitions {
    pub fn from_current_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux appeal rules"));
        }
        let integer = |row: crate::Row<'_>, field| -> Result<i32, Error> {
            crate::game_parameters::sql_integer(row.get(field).map_err(Error::Rules)?)
                .ok_or(Error::Unsupported(field))
        };
        let read = |table, key| -> Result<Vec<Definition>, Error> {
            let table_rows = rules
                .gameplay()
                .table(table)
                .ok_or(Error::Unsupported(table))?;
            table_rows
                .rows()
                .map(|r| {
                    Ok(Definition {
                        hash: linux_type_hash(r.text(key).map_err(Error::Rules)?.as_bytes()),
                        appeal: integer(r, "Appeal")?,
                        water: if table == "Terrains" {
                            integer(r, "Water")? != 0
                        } else {
                            false
                        },
                        mountain: if table == "Terrains" {
                            integer(r, "Mountain")? != 0
                        } else {
                            false
                        },
                        wonder: if table == "Features" {
                            integer(r, "NaturalWonder")? != 0
                        } else {
                            false
                        },
                        civic: if table == "Features" {
                            match r.get("AddCivic").map_err(Error::Rules)? {
                                SqlValue::Text(s) => Some(linux_type_hash(s)),
                                SqlValue::Null => None,
                                _ => return Err(Error::Unsupported("AddCivic")),
                            }
                        } else {
                            None
                        },
                    })
                })
                .collect()
        };
        let params = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?;
        let lake = params
            .rows()
            .find(|r| r.text("Name").ok() == Some("LAKE_MAX_AREA_SIZE"))
            .ok_or(Error::Unsupported("LAKE_MAX_AREA_SIZE"))?;
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            terrains: read("Terrains", "TerrainType")?,
            features: read("Features", "FeatureType")?,
            improvements: read("Improvements", "ImprovementType")?,
            districts: read("Districts", "DistrictType")?,
            lake_max: integer(lake, "Value")?,
        })
    }
}
fn find(rows: &[Definition], hash: u32) -> Option<&Definition> {
    rows.iter().find(|r| r.hash == hash)
}
fn city<'a>(state: &'a LoadedState, p: &Plot) -> Option<&'a City> {
    let a = p.city_attributes.as_ref()?;
    state.city(i32::from(a.player), a.owning_city).ok()
}
fn district<'a>(
    state: &'a LoadedState,
    p: &Plot,
) -> Option<&'a c6_core::player_districts::District> {
    let a = p.city_attributes.as_ref()?;
    state.district(i32::from(a.player), a.district).ok()
}
fn plot_index(map: &Map, p: PlotCoord) -> Result<usize, Error> {
    c6_core::movement_world::plot_at(map, p)
        .map_err(|_| Error::Unsupported("appeal plot coordinates"))?;
    Ok((i64::from(p.y) * i64::from(map.header.width) + i64::from(p.x)) as usize)
}
fn neighbours(map: &Map, point: PlotCoord) -> Result<Vec<usize>, Error> {
    let mut result = Vec::with_capacity(6);
    for i in 1..=6 {
        if let Some(p) = c6_core::city_citizen_assignment::plot_location(map, point, i)
            .map_err(|_| Error::Unsupported("appeal neighbor"))?
        {
            result
                .push(plot_index(map, p).map_err(|_| Error::Unsupported("appeal neighbor index"))?);
        }
    }
    Ok(result)
}
fn civic(culture: &c6_core::player_culture::Culture, hash: u32) -> Result<bool, Error> {
    Ok(culture.has_civic.pairs[index(&culture.has_civic.pairs, hash)?].value)
}
fn neighbor_contribution(state: &LoadedState, p: &Plot, defs: &Definitions) -> Result<i32, Error> {
    let c = city(state, p);
    let mut value = 0i32;
    if let (Some(c), Some(d)) = (c, district(state, p)) {
        let a = p.city_attributes.as_ref().expect("owning city attributes");
        if let Some(def) = find(&defs.districts, d.district_hash) {
            if a.wonder_building_hash != u32::MAX {
                if c.buildings.building_locations
                    [index(&c.buildings.building_locations, a.wonder_building_hash)?]
                .value
                    != -1
                {
                    value = value.wrapping_add(def.appeal);
                }
            } else if d.is_complete {
                value = value
                    .wrapping_add(def.appeal)
                    .wrapping_sub(i32::from(d.pillaged));
            }
        }
    }
    if let Some(f) = find(&defs.features, p.feature_hash) {
        value = value.wrapping_add(f.appeal);
        if let Some(c) = c {
            value = value.wrapping_add(
                c.growth
                    .feature_appeal_modifiers
                    .iter()
                    .find(|m| m.definition_hash == p.feature_hash)
                    .map_or(0, |m| m.value),
            );
        }
    }
    if let Some(t) = find(&defs.terrains, p.terrain_hash) {
        value = value.wrapping_add(t.appeal);
    }
    value = value.wrapping_sub(i32::from(p.flags.get(PlotFlag::ImprovementPillaged)));
    if let Some(i) = find(&defs.improvements, p.improvement_hash) {
        value = value.wrapping_add(i.appeal);
    }
    if let Some(c) = c {
        let player = state
            .retained()
            .players
            .iter()
            .find(|r| r.header.owner == c.header.owner)
            .ok_or(Error::Unsupported("appeal governor owner"))?;
        if let Some(g) = player
            .governors
            .entries
            .iter()
            .map(|s| &s.governor)
            .find(|g| {
                g.assigned_city.id == c.header.id
                    && i32::from(g.assigned_city.owner) == c.header.owner
            })
        {
            if g.established
                && p.feature_hash != u32::MAX
                && p.improvement_hash == u32::MAX
                && g.feature_no_improvement_appeal_modifier > 0
            {
                value = value.wrapping_add(g.feature_no_improvement_appeal_modifier);
            }
        }
    }
    Ok(value)
}
/// Rules.Appeal.GetAppeal2bd35c0 and its ordered radius-one contribution.
pub fn calculate(state: &LoadedState, plot: usize, defs: &Definitions) -> Result<i32, Error> {
    if defs.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    let map = state.world().map();
    let p = map
        .plots
        .get(plot)
        .ok_or(Error::Unsupported("appeal plot"))?;
    let mut value = city(state, p).map_or(0, |c| c.growth.extra_appeal);
    if find(&defs.features, p.feature_hash).is_some_and(|f| f.wonder) {
        return Ok(value.wrapping_add(5));
    }
    let t = find(&defs.terrains, p.terrain_hash).ok_or(Error::Unsupported("appeal terrain"))?;
    if t.water {
        return Ok(0);
    }
    if t.mountain {
        return Ok(value.wrapping_add(4));
    }
    let lake = map
        .landmasses
        .entries
        .iter()
        .find(|m| m.value.id == p.landmass_id)
        .is_some_and(|m| m.value.water && m.value.plot_count <= defs.lake_max);
    if p.river_crossing_count > 0 || lake {
        value = value.wrapping_add(1);
    }
    if city(state, p).is_some() && district(state, p).is_some_and(|d| d.is_complete && d.pillaged) {
        value = value.wrapping_sub(1);
    }
    value = value.wrapping_sub(i32::from(p.flags.get(PlotFlag::ImprovementPillaged)));
    let point = PlotCoord {
        x: plot as i32 % map.header.width,
        y: plot as i32 / map.header.width,
    };
    for i in neighbours(map, point)? {
        value = value.wrapping_add(neighbor_contribution(state, &map.plots[i], defs)?);
    }
    if !p.flags.get(PlotFlag::FeatureAdded) {
        if let (Some(feature), Some(attributes)) = (
            find(&defs.features, p.feature_hash),
            p.city_attributes.as_ref(),
        ) {
            if let Some(unlock) = feature.civic {
                if let Some(player) = state
                    .retained()
                    .players
                    .iter()
                    .find(|r| r.header.owner == i32::from(attributes.player))
                {
                    let mut unlocked = civic(&player.culture, unlock)?;
                    if !unlocked {
                        if player.culture.wire_version < 45 {
                            return Err(Error::Unsupported("retained feature unlock context"));
                        }
                        for row in &player.culture.unlock_features {
                            if row.feature_hash == p.feature_hash
                                && civic(&player.culture, row.civic_hash)?
                            {
                                unlocked = true;
                                break;
                            }
                        }
                    }
                    if unlocked {
                        value = value.wrapping_add(1);
                    }
                }
            }
        }
    }
    Ok(value)
}
/// Update2bd3900 visits center then six directions. Recomputed-equal values
/// cause no native signal/report. A changed value needs the actual appeal
/// subscribers; it is never silently discarded or reported as complete.
pub fn refresh_unchanged(
    state: &LoadedState,
    point: [i32; 2],
    defs: &Definitions,
) -> Result<(), Error> {
    let map = state.world().map();
    let point = PlotCoord {
        x: point[0],
        y: point[1],
    };
    let center =
        plot_index(map, point).map_err(|_| Error::Unsupported("appeal refresh location"))?;
    for i in std::iter::once(center).chain(neighbours(map, point)?) {
        if calculate(state, i, defs)? != i32::from(map.plots[i].appeal) {
            return Err(Error::Unsupported("changed appeal Game signal/report"));
        }
    }
    Ok(())
}

/// Gameplay continuations of a changed appeal. Both execute after the i16 cache
/// write; the signal receives the full calculated i32, before numeric narrowing.
pub trait Effects {
    type Error;
    fn appeal_changed(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        value: i32,
    ) -> Result<(), Self::Error>;
    fn appeal_report(&mut self, state: &mut LoadedState, plot: u32) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub enum UpdateError<E> {
    Query(Error),
    State(c6_core::loaded_state::Error),
    Effect(E),
    MissingPlot,
    Geometry(c6_core::movement_world::MissingInput),
}
impl<E: std::fmt::Debug> std::fmt::Display for UpdateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Appeal update: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for UpdateError<E> {}
struct Update<'a, E> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    effects: &'a mut E,
}
impl<E: Effects> c6_core::map::appeal::Context for Update<'_, E> {
    type Error = UpdateError<E::Error>;
    type Plot = usize;
    fn calculate(&mut self, plot: usize) -> Result<i32, Self::Error> {
        calculate(self.state, plot, self.definitions).map_err(UpdateError::Query)
    }
    fn cached(&mut self, plot: usize) -> Result<i16, Self::Error> {
        self.state
            .world()
            .map()
            .plots
            .get(plot)
            .map(|p| p.appeal)
            .ok_or(UpdateError::MissingPlot)
    }
    fn store(&mut self, plot: usize, value: i16) -> Result<(), Self::Error> {
        self.state
            .world_mut()
            .map_err(UpdateError::State)?
            .plot_mut(plot)
            .ok_or(UpdateError::MissingPlot)?
            .appeal = value;
        Ok(())
    }
    fn changed(&mut self, plot: usize, value: i32) -> Result<(), Self::Error> {
        self.effects
            .appeal_changed(self.state, plot as u32, value)
            .map_err(UpdateError::Effect)
    }
    fn report(&mut self, plot: usize) -> Result<(), Self::Error> {
        self.effects
            .appeal_report(self.state, plot as u32)
            .map_err(UpdateError::Effect)
    }
}
/// Update2bd3900 -> Update2bd3870 visits the center, then native directions0..5.
/// Callback-visible order differs from ascending CityHexspace indexes. Wrapped
/// duplicates remain repeated calls; each plot is recalculated after callbacks.
pub fn update<E: Effects>(
    state: &mut LoadedState,
    point: [i32; 2],
    definitions: &Definitions,
    effects: &mut E,
) -> Result<usize, UpdateError<E::Error>> {
    let Some(center) = c6_core::found_city::placement::plot_from_delta(
        state.world().map(),
        PlotCoord {
            x: point[0],
            y: point[1],
        },
        0,
        0,
        0,
    )
    .map_err(UpdateError::Geometry)?
    else {
        return Ok(0);
    };
    let mut context = Update {
        state,
        definitions,
        effects,
    };
    let mut changed = usize::from(c6_core::map::appeal::update_only(
        &mut context,
        Some(center),
    )?);
    let (x, y) = context
        .state
        .world()
        .map()
        .coordinates(center as u32)
        .ok_or(UpdateError::MissingPlot)?;
    let center = PlotCoord { x, y };
    // Source Direction.X=[0,1,1,0,-1,-1], Y=[1,0,-1,-1,0,1].
    for index in [2, 1, 6, 5, 4, 3] {
        let location = c6_core::city_citizen_assignment::plot_location(
            context.state.world().map(),
            center,
            index,
        )
        .map_err(UpdateError::Geometry)?;
        let plot = location
            .map(|p| plot_index(context.state.world().map(), p))
            .transpose()
            .map_err(UpdateError::Query)?;
        changed += usize::from(c6_core::map::appeal::update_only(&mut context, plot)?);
    }
    Ok(changed)
}
