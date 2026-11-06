//! Reached neutral-meeting city queries and capital discovery.
//! Hospitality conversations and arbitrary diplomatic effects remain unsupported.
use super::{Error, LoadedState, PlotCoord, Runtime};

pub struct Visibility<'a> {
    pub runtime: &'a mut crate::unit_sight_state::Runtime,
    pub definitions: &'a crate::loaded_actions::city_range_attack::Definitions,
}
fn player(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("first-contact city parent"))
}
fn header(state: &LoadedState, city: (i32, u32)) -> Result<&c6_core::city_wire::Header, Error> {
    player(state, city.0)?
        .cities
        .records
        .entries
        .iter()
        .find(|c| c.value.header.id == city.1)
        .map(|c| &c.value.header)
        .ok_or(Error::Missing("first-contact current city"))
}
pub(super) fn location(state: &LoadedState, city: (i32, u32)) -> Result<PlotCoord, Error> {
    let h = header(state, city)?;
    Ok(PlotCoord {
        x: h.location_x,
        y: h.location_y,
    })
}
pub(super) fn distance(state: &LoadedState, city: (i32, u32), plot: u32) -> Result<i32, Error> {
    let at = location(state, city)?;
    let map = state.world().map();
    let xy = map
        .coordinates(plot)
        .ok_or(Error::Missing("first-contact plot coordinate"))?;
    Ok(c6_core::geometry::plot_distance(
        map.header.width,
        map.header.wrap_x,
        at.x,
        at.y,
        xy.0,
        xy.1,
    ))
}
pub(super) fn closest(
    state: &LoadedState,
    owner: i32,
    plot: u32,
) -> Result<Option<(u32, i32)>, Error> {
    let mut cities: Vec<_> = player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .collect();
    cities.sort_by_key(|c| c.index);
    let mut closest = None;
    for city in cities {
        let id = city.value.header.id;
        let value = distance(state, (owner, id), plot)?;
        if closest.is_none_or(|(_, minimum)| value < minimum) {
            closest = Some((id, value));
        }
    }
    Ok(closest)
}
pub(super) fn reveal<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    visibility: Option<&mut Visibility<'_>>,
    owner: i32,
    recipient: i32,
) -> Result<(), Error> {
    let capital = player(state, owner)?
        .cities
        .records
        .entries
        .iter()
        .filter(|c| c.value.header.capital)
        .min_by_key(|c| c.index)
        .map(|c| c.value.header.id);
    let Some(capital) = capital else {
        return Ok(());
    };
    let at = location(state, (owner, capital))?;
    let v = visibility.ok_or(Error::Missing(
        "first-contact capital reveal visibility owner",
    ))?;
    let map = state.world().map();
    if map.plot_at(at.x, at.y).is_none() {
        return Err(Error::Missing("first-contact capital plot"));
    }
    let center = at.y as u32 * map.header.width as u32 + at.x as u32;
    let mut plots = vec![center as usize];
    // Native HEXVec_i::TRANSLATIONS order, not clockwise direction order.
    for direction in [0, 4, 2, 3, 1, 5] {
        if let Some(index) = crate::barbarian_camps::queries::adjacent(state, at, direction)
            .map_err(|e| Error::Requirement(format!("first-contact capital neighbors: {e:?}")))?
        {
            plots.push(index);
        }
    }
    for plot in plots {
        let mut context = crate::unit_sight_state::Context {
            state,
            runtime: v.runtime,
            effects: backend,
            sight_definitions: v.definitions,
            actor: (-1, u32::MAX),
        };
        c6_core::visibility_change::change_visibility_count(
            &mut context,
            c6_core::unit_sight::VisibilityChange {
                owner: recipient,
                plot_index: plot as u32,
                delta: 0,
                hidden_type: -1,
                source_flag: false,
                near_origin: false,
                origin_index: -1,
                sight_range: 1,
                source: c6_core::unit::ComponentId {
                    owner: -1,
                    component_type: 0,
                    id: u32::MAX,
                },
            },
        )
        .map_err(|e| Error::Requirement(format!("first-contact capital reveal: {e:?}")))?;
    }
    Ok(())
}
