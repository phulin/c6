//! Gameplay identity effects of the owned improvement reporting query.
//!
//! Native nonnull contribution queries create and remove a PlotYields object.
//! With an empty collection-created signal the temporary has no modifiers or
//! callbacks, but its consumed instance ID remains in the canonical counter.
use c6_core::city_territory::{plot_yields, CityId};

/// Preserve the temporary identities consumed by Map's improvement cache
/// update: current plot first, then adjacent plots in native direction order.
/// Missing improvements and politically unowned plots do not enter the query.
pub fn improvement_changed(
    state: &c6_core::loaded_state::LoadedState,
    territory: &mut crate::city_territory::Runtime,
    definitions: &super::Definitions,
    plot: usize,
) -> Result<(), &'static str> {
    let map = state.world().map();
    let center = map.plots.get(plot).ok_or("reporting improvement plot")?;
    let width = map.header.width;
    let height = map.header.height;
    if width <= 0 || height <= 0 || map.plots.len() != width as usize * height as usize {
        return Err("reporting improvement map dimensions");
    }
    let x = (plot % width as usize) as i32;
    let y = (plot / width as usize) as i32;
    let mut plots = vec![(plot, center)];
    const X: [i32; 6] = [0, 1, 1, 0, -1, -1];
    const Y: [i32; 6] = [1, 0, -1, -1, 0, 1];
    for direction in 0..6 {
        let ny = y + Y[direction];
        let nx = x - y.div_euclid(2) + X[direction] + ny.div_euclid(2);
        let nx = c6_core::movement_world::coord_range(nx, width, map.header.wrap_x);
        let ny = c6_core::movement_world::coord_range(ny, height, map.header.wrap_y);
        if nx >= 0 && ny >= 0 && nx < width && ny < height {
            let index = ny as usize * width as usize + nx as usize;
            plots.push((index, &map.plots[index]));
        }
    }
    // Resolve all reached inputs before beginning the retained update.
    let mut queries = Vec::new();
    for (index, p) in plots {
        if p.improvement_hash == u32::MAX {
            continue;
        }
        let Some(attributes) = p.city_attributes.as_ref().filter(|a| a.player != -1) else {
            continue;
        };
        let improvement = definitions
            .improvements
            .find_index(p.improvement_hash)
            .ok_or("reporting current improvement definition")? as i32;
        let city = if attributes.owning_city == u32::MAX {
            None
        } else {
            state
                .city(i32::from(attributes.player), attributes.owning_city)
                .map_err(|_| "reporting current owning city")?;
            Some(CityId {
                owner: i32::from(attributes.player),
                id: attributes.owning_city,
            })
        };
        queries.push((index as u32, city, improvement));
    }
    for (index, city, improvement) in queries {
        temporary_plot_yields(territory, index, city, improvement)?;
    }
    Ok(())
}

/// Execute one temporary contribution query on the retained territory owner.
/// Presentation yield/contribution values do not leave this boundary.
pub fn temporary_plot_yields(
    territory: &mut crate::city_territory::Runtime,
    plot: u32,
    city: Option<CityId>,
    improvement: i32,
) -> Result<u32, &'static str> {
    let runtime = &mut territory.plot_yields;
    let created = runtime
        .created
        .as_ref()
        .ok_or("current PlotYields created signal")?;
    if created.ordered_subscribers().next().is_some() {
        return Err("current PlotYields reporting creation subscribers");
    }
    // Native lookup uses ordered IDs. A wrapped/colliding next ID needs the
    // general collection contract before it can be used in this boundary.
    let next = runtime.counter.wrapping_add(1);
    if runtime.objects.last().is_some_and(|v| v.id >= next) {
        return Err("current PlotYields reporting ID ordering");
    }
    struct Query<'a>(&'a mut plot_yields::Runtime);
    impl plot_yields::Context for Query<'_> {
        type Error = &'static str;
        fn runtime(&self) -> &plot_yields::Runtime {
            self.0
        }
        fn runtime_mut(&mut self) -> &mut plot_yields::Runtime {
            self.0
        }
        fn created(&mut self, _: u64, _: u32) -> Result<(), Self::Error> {
            Err("unexpected PlotYields reporting subscriber")
        }
        fn destroyed(&mut self, object: plot_yields::Object) -> Result<(), Self::Error> {
            // Only our new object reaches this call. No creation subscriber
            // could attach modifiers or its initially empty destruction hook.
            if object
                .owned_modifiers
                .as_ref()
                .is_none_or(|v| !v.is_empty())
            {
                return Err("temporary reporting PlotYields modifiers");
            }
            Ok(())
        }
    }
    let mut query = Query(runtime);
    let id = plot_yields::add(
        &mut query,
        plot_yields::Object::new(plot, city, improvement),
    )
    .map_err(|_| "temporary reporting PlotYields creation")?;
    // The new tuple vector is empty and the empty creation ring cannot change
    // it. Native GetPlotYield reads it without gameplay state or RNG effects.
    plot_yields::remove(&mut query, id)
        .map_err(|_| "temporary reporting PlotYields destruction")?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::unit_moved_signal::ListenerRing;

    fn territory() -> crate::city_territory::Runtime {
        let mut existing = plot_yields::Object::new(10, None, -1);
        existing.id = 7;
        existing.yields = vec![(0, 256)];
        existing.owned_modifiers = None;
        existing.affected_modifiers = None;
        crate::city_territory::Runtime {
            attributes: vec![],
            plot_yields: plot_yields::Runtime {
                counter: 8,
                objects: vec![existing],
                created: Some(ListenerRing::from_ordered(&[]).unwrap()),
            },
            players: vec![],
            start_era: None,
            alive_major_ids: None,
        }
    }

    #[test]
    fn query_consumes_identity_and_preserves_existing_unknown_modifier_state() {
        let mut runtime = territory();
        let original = runtime.plot_yields.objects.clone();
        assert_eq!(temporary_plot_yields(&mut runtime, 99, None, 5), Ok(9));
        assert_eq!(runtime.plot_yields.counter, 9);
        assert_eq!(runtime.plot_yields.objects, original);
    }

    #[test]
    fn unavailable_or_nonempty_creation_signal_rejects_before_mutation() {
        let mut runtime = territory();
        runtime.plot_yields.created = None;
        assert!(temporary_plot_yields(&mut runtime, 99, None, 5).is_err());
        assert_eq!(runtime.plot_yields.counter, 8);
        runtime.plot_yields.created = Some(ListenerRing::from_ordered(&[1]).unwrap());
        assert!(temporary_plot_yields(&mut runtime, 99, None, 5).is_err());
        assert_eq!(runtime.plot_yields.counter, 8);
        assert_eq!(runtime.plot_yields.objects.len(), 1);
    }
}
