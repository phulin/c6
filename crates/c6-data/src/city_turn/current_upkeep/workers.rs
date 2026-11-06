//! Borrowed worker services shared by upkeep and synchronous production callbacks.
use super::{appeal, upkeep, yield_calculation, CityError, Current, WorkerAppeal};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};
pub(super) struct Context<'a> {
    pub definitions: &'a yield_calculation::Definitions,
    pub current: &'a Current<'a>,
    pub workers: &'a upkeep::citizens::Definitions,
    pub appeal: &'a appeal::Definitions,
    pub fallout: &'a [i32],
    pub calls: &'a mut Vec<&'static str>,
}
impl Context<'_> {
    fn citizen_current<'q>(
        &'q self,
        territory: &'q crate::city_territory::Runtime,
    ) -> upkeep::citizens::Current<'q> {
        upkeep::citizens::Current {
            definitions: self.definitions,
            territory,
            drought_ids: self.current.drought_ids,
            fallout_turns: self.fallout,
        }
    }
    fn qualify_report(&self, event: u64, expected: &[u64]) -> std::result::Result<(), CityError> {
        // Fixed stock cache consumers below only publish derived city/player
        // snapshots. Scope excludes added synchronous consumers, not gameplay.
        match (event, expected) {
            (0x0242e68c, [0x2717400])
            | (0x89de2474, [0x27173c0])
            | (0x67001fe1, [0x2710a10, 0x27bcd80])
            | (0x30ae2a3d, [0x2716f80, 0x27bcc60]) => Ok(()),
            _ => Err(CityError::Unsupported("unknown city upkeep report")),
        }
    }
}
impl upkeep::citizens::Effects for Context<'_> {
    fn food_surplus_q8(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<i32, CityError> {
        let index = self
            .definitions
            .yield_index(b"YIELD_FOOD")
            .ok_or(CityError::Unsupported("Food definition"))?;
        let amount = self
            .current
            .calculate(s, o, c, index, 0, self.definitions)?;
        Ok(amount.wrapping_sub(
            s.city(o, c)?
                .header
                .population
                .wrapping_mul(self.definitions.growth.food_consumption_per_population)
                .wrapping_shl(8),
        ))
    }
    fn visit_plot(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        p: usize,
    ) -> std::result::Result<(), CityError> {
        let defs = self.appeal;
        // Release the immutable field borrow across the reentrant service call.
        let result = appeal::update_and_test_adjacent(
            s,
            Some(p),
            defs,
            &mut WorkerAppeal {
                subscribers: defs.changed_subscribers(s),
                calls: self.calls,
            },
            &mut |s: &mut LoadedState, _: &mut WorkerAppeal<'_>, a: u32| {
                let p = s
                    .world()
                    .map()
                    .plots
                    .get(a as usize)
                    .ok_or(CityError::Unsupported("adjacent worker plot"))?;
                Ok(!p
                    .city_attributes
                    .as_ref()
                    .is_some_and(|a| i32::from(a.player) == o && a.owning_city == c))
            },
        );
        result.map(|_| ()).map_err(|error| match error {
            appeal::UpdateError::Query(error) => CityError::Production(error),
            appeal::UpdateError::Effect(error) => error,
            appeal::UpdateError::State(error) => CityError::State(error),
            _ => CityError::Unsupported("worker appeal geometry/plot continuation"),
        })
    }
    fn capacity(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        p: PlotCoord,
    ) -> std::result::Result<i32, CityError> {
        let territory = self.current.territory.read()?;
        upkeep::citizens::capacity(s, o, c, p, self.workers, &self.citizen_current(&territory))
    }
    fn plot_yields(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        p: usize,
    ) -> std::result::Result<Vec<c6_core::city_citizen_assignment::Yield>, CityError> {
        let territory = self.current.territory.read()?;
        upkeep::citizens::plot_yields(s, o, c, p, &self.citizen_current(&territory))
    }
    fn food_and_gold_indices(&self) -> std::result::Result<(i32, i32), CityError> {
        Ok((
            self.definitions
                .yield_index(b"YIELD_FOOD")
                .ok_or(CityError::Unsupported("Food index"))?,
            self.definitions
                .yield_index(b"YIELD_GOLD")
                .ok_or(CityError::Unsupported("Gold index"))?,
        ))
    }
    fn workers_changed(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        i: i32,
        old: i32,
    ) -> std::result::Result<(), CityError> {
        let city = s.city(o, c)?;
        let at = i as usize;
        let current = city
            .citizens
            .working_plot
            .get(at)
            .ok_or(CityError::Unsupported("worker index"))?
            .wrapping_add(
                *city
                    .citizens
                    .forced_working_plot
                    .get(at)
                    .ok_or(CityError::Unsupported("forced worker index"))?,
            );
        if current == old {
            return Ok(());
        }
        self.qualify_report(0x67001fe1, &[0x2710a10, 0x27bcd80])?;
        self.calls.push("worker output report");
        let center = PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        };
        if let Some(p) = c6_core::city_citizen_assignment::plot_location(s.world().map(), center, i)
            .map_err(|_| CityError::Unsupported("worker report location"))?
        {
            let (plot, p) = c6_core::movement_world::plot_at(s.world().map(), p)
                .map_err(|_| CityError::Unsupported("worker report plot"))?;
            if p.improvement_hash != u32::MAX
                && i32::from(p.improvement_owner) == o
                && (current == 0 || old == 0)
            {
                self.qualify_report(0x30ae2a3d, &[0x2716f80, 0x27bcc60])?;
                // The generic ImprovementChanged report's Map recipient performs
                // real temporary contribution queries even for activity subtypes.
                // Preserve their canonical IDs/callbacks before omitting DTOs.
                let mut territory = self.current.territory.write()?;
                crate::loaded_actions::pillage::reporting::improvement_changed_with_registry(
                    s,
                    &mut territory,
                    self.workers.improvement_registry(),
                    plot,
                )
                .map_err(CityError::Unsupported)?;
                self.calls.push(if current != 0 {
                    "worker improvement activated output"
                } else {
                    "worker improvement deactivated output"
                });
            }
        }
        Ok(())
    }
}
