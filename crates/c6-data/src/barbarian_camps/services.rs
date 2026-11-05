//! Stock first-camp subscribers over the canonical turn backend.
//! Guards below are the reached Linux functions, not replayed callback answers.
use super::stock::{Constructed, Services};
use crate::{
    barbarian_activation::Behavior,
    city_turn::{tiles, yield_definitions},
    loaded_actions::stock_notifications as stock,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

#[derive(Clone, Copy, Debug)]
pub struct Subscriber {
    pub receiver: u64,
    pub invoke: u64,
}
/// Actual initial Economic manager inputs. Camp construction on unowned plots
/// does not alter these resource-control counters (Resources callback 2b70840).
pub struct Economic {
    pub resource_count: usize,
    pub totals: Vec<i32>,
    pub owned: Vec<Vec<i32>>,
    pub summaries: Vec<(i32, i32)>,
}
pub struct Initial {
    pub binding: WireBinding,
    pub constructed: Vec<Subscriber>,
    pub camp_created_count: usize,
    pub history_special_improvements: Vec<i32>,
    pub economic: Economic,
}
#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub location: PlotCoord,
    pub event: u32,
    pub argument: i32,
}
pub struct Current<'a> {
    binding: WireBinding,
    order: Vec<Constructed>,
    subscribers: Vec<Subscriber>,
    camp_created_count: usize,
    history_special_improvements: Vec<i32>,
    economic: Economic,
    pub definitions: &'a yield_definitions::Definitions,
    pub tiles: tiles::Current<'a>,
    pub local_observer: i32,
    speed_multiplier: i32,
    camp_index: i32,
    camp_hash: u32,
    camp_has_modifiers: bool,
    camp_has_trait: bool,
    beach_index: Option<i32>,
    pub alive_major_ids: &'a [i32],
    pub reports: Vec<Report>,
}
impl<'a> Current<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn from_initial(
        state: &LoadedState,
        initial: Initial,
        rules: &crate::Rules,
        definitions: &'a yield_definitions::Definitions,
        tiles: tiles::Current<'a>,
        local_observer: i32,
        game_speed_hash: u32,
        alive_major_ids: &'a [i32],
    ) -> Result<Self, Error> {
        if initial.binding != *state.binding() {
            return Err(Error::Unsupported("camp services initial binding"));
        }
        let registry = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Error::Unsupported("camp service improvement definitions"))?;
        let camp_hash = state.retained().barbarian.improvement_hash;
        let camp_index = registry
            .find_index(camp_hash)
            .ok_or(Error::Unsupported("camp service definition"))? as i32;
        let entry = registry
            .at(camp_index as u32)
            .ok_or(Error::Unsupported("camp service definition"))?;
        let table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Unsupported("camp service rules"))?;
        let row = table
            .rows()
            .find(|r| r.row_id() == Some(entry.database_rowid))
            .ok_or(Error::Unsupported("camp service definition row"))?;
        let camp_has_trait = !matches!(
            row.get("TraitType")
                .map_err(|_| Error::Unsupported("camp trait"))?,
            crate::SqlValue::Null
        );
        let modifiers = rules
            .gameplay()
            .table("ImprovementModifiers")
            .ok_or(Error::Unsupported("camp modifier rules"))?;
        let camp_has_modifiers = modifiers.rows().any(|r| {
            r.text("ImprovementType")
                .ok()
                .is_some_and(|s| s.as_bytes() == entry.type_name)
        });
        let speeds = rules
            .gameplay()
            .table("GameSpeeds")
            .ok_or(Error::Unsupported("camp game speed rules"))?;
        let speed_multiplier = speeds
            .rows()
            .find(|r| {
                r.text("GameSpeedType").ok().is_some_and(|s| {
                    crate::registry::linux_type_hash(s.as_bytes()) == game_speed_hash
                })
            })
            .map(|r| r.integer("CostMultiplier"))
            .transpose()
            .map_err(|_| Error::Unsupported("camp game speed multiplier"))?
            .map(i32::try_from)
            .transpose()
            .map_err(|_| Error::Unsupported("camp game speed multiplier"))?
            .unwrap_or(100);
        let mut order = Vec::new();
        let mut barbarian = 0;
        for (slot, subscriber) in initial.constructed.iter().enumerate() {
            if subscriber.invoke == 0x283bef0 {
                order.push(Constructed::Barbarian);
                barbarian += 1;
            } else if matches!(
                subscriber.invoke,
                0x2625f90
                    | 0x29c8a80
                    | 0x29ea410
                    | 0x2848700
                    | 0x2847120
                    | 0x2845380
                    | 0x2847ab0
                    | 0x2849540
                    | 0x2b03c60
                    | 0x2903390
            ) {
                order.push(Constructed::Gameplay(slot));
            } else {
                return Err(Error::Unsupported(
                    "unknown improvement constructed subscriber",
                ));
            }
        }
        if barbarian != 1 {
            return Err(Error::Unsupported(
                "camp constructed Barbarian subscriber count",
            ));
        }
        Ok(Self {
            binding: initial.binding,
            order,
            subscribers: initial.constructed,
            camp_created_count: initial.camp_created_count,
            history_special_improvements: initial.history_special_improvements,
            economic: initial.economic,
            definitions,
            tiles,
            local_observer,
            speed_multiplier,
            camp_index,
            camp_hash,
            camp_has_modifiers,
            camp_has_trait,
            beach_index: registry
                .find_index(crate::registry::linux_type_hash(
                    b"IMPROVEMENT_SEASIDE_RESORT",
                ))
                .map(|i| i as i32),
            alive_major_ids,
            reports: Vec::new(),
        })
    }
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if self.binding != *state.binding()
            || self.camp_hash != state.retained().barbarian.improvement_hash
        {
            return Err(Error::Unsupported("camp services current binding"));
        }
        Ok(())
    }
    fn plot<'s>(
        &self,
        state: &'s LoadedState,
        index: usize,
    ) -> Result<&'s c6_core::map::Plot, Error> {
        state
            .world()
            .map()
            .plots
            .get(index)
            .ok_or(Error::Unsupported("camp services plot"))
    }
}
impl<P> Services<P> for Current<'_> {
    fn constructed_order(&self) -> &[Constructed] {
        &self.order
    }
    fn constructed_gameplay(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut stock::Runtime<'_, P>,
        plot: usize,
        receiver: usize,
    ) -> Result<(), Error> {
        self.check(state)?;
        let subscriber = self
            .subscribers
            .get(receiver)
            .ok_or(Error::Unsupported("camp constructed receiver"))?;
        let p = self.plot(state, plot)?;
        if p.improvement_hash != self.camp_hash {
            return Err(Error::Unsupported("camp constructed improvement changed"));
        }
        let owner = p
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        match subscriber.invoke {
            // 23e13b0: attach only definitions returned by ImprovementModifiers.
            0x2625f90 if !self.camp_has_modifiers => Ok(()),
            // 29b82d0: present resort definition compares against new type.
            0x29c8a80 if self.beach_index.is_some_and(|i| i != self.camp_index) => Ok(()),
            // 29ea460: trait moment, natural-feature moment (requires owning
            // City), then eight cached improvement-type comparisons.
            0x29ea410
                if !self.camp_has_trait
                    && p.city_attributes.is_none()
                    && self.history_special_improvements.len() == 8
                    && !self.history_special_improvements.contains(&self.camp_index) =>
            {
                Ok(())
            }
            // CheckNumImprovements, OnImprovementConstructed and
            // CheckUnitAndImprovement all reject NO_PLAYER before writes/RNG.
            0x2848700 | 0x2847120 | 0x2847ab0 | 0x2849540 if owner == -1 => Ok(()),
            // CheckAirbaseForeignContinent begins with district existence.
            0x2845380
                if p.city_attributes
                    .as_ref()
                    .is_none_or(|a| a.district == u32::MAX) =>
            {
                Ok(())
            }
            // All four registered Player.Resource callbacks compare actual
            // territory owner to their valid Player ID (2b70840).
            0x2b03c60 if owner == -1 => Ok(()),
            // Achievement handler rejects NO_PLAYER (28f9269).
            0x2903390 if owner == -1 => Ok(()),
            _ => Err(Error::Unsupported(
                "positive camp constructed subscriber continuation",
            )),
        }
    }
    fn spawn_turns(&self, state: &LoadedState, turns: i32) -> Result<i32, Error> {
        self.check(state)?;
        Ok(c6_core::research_acquisition::units::adjust_for_game_speed(
            turns,
            self.speed_multiplier,
        ))
    }
    fn local_food(&self, state: &LoadedState, location: PlotCoord) -> Result<i32, Error> {
        self.check(state)?;
        let food = self
            .definitions
            .yield_indices()
            .find(|&i| {
                self.definitions.yield_hash(i)
                    == Some(crate::registry::linux_type_hash(b"YIELD_FOOD"))
            })
            .ok_or(Error::Unsupported("camp food yield definition"))?;
        let mut plots = Vec::new();
        c6_core::unit_placement_tails::neighborhood(location, 1, |at| {
            let Some(plot) = crate::unit_placement_services::plot_index_at::<
                std::convert::Infallible,
            >(state, at)
            .map_err(|_| Error::Unsupported("camp local food plot"))?
            else {
                return Ok(());
            };
            if self
                .plot(state, plot as usize)?
                .city_attributes
                .as_ref()
                .is_none_or(|a| a.player == -1)
            {
                plots.push(plot);
            }
            Ok(())
        })?;
        let yields = tiles::get_plot_yields(
            state,
            self.local_observer,
            &plots,
            &[food],
            self.definitions,
            &self.tiles,
        )
        .map_err(|_| Error::Unsupported("camp current local food yields"))?;
        Ok(yields
            .into_iter()
            .fold(0i32, |sum, row| sum.wrapping_add(row[0]))
            .max(1))
    }
    fn camp_created(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut stock::Runtime<'_, P>,
        _: usize,
    ) -> Result<(), Error> {
        self.check(state)?;
        if self.camp_created_count != 0 {
            return Err(Error::Unsupported("camp created subscribers"));
        }
        Ok(())
    }
    fn monopolies(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut stock::Runtime<'_, P>,
    ) -> Result<(), Error> {
        self.check(state)?;
        let e = &self.economic;
        if e.resource_count != e.totals.len()
            || e.resource_count != e.summaries.len()
            || e.owned.len() != 64
            || e.owned.iter().any(|row| row.len() != e.resource_count)
        {
            return Err(Error::Unsupported("camp current Economic resource arrays"));
        }
        // HasResourceMonopolyOf 264a63b..264a666 returns false when the
        // total's Q8 constructor is <=0. Calculate then compares summary owner
        // with each alive major; NO_PLAYER summaries leave it unchanged.
        if e.totals.iter().any(|&total| total.wrapping_shl(8) > 0)
            || e.summaries
                .iter()
                .any(|&(owner, _)| self.alive_major_ids.contains(&owner))
        {
            return Err(Error::Unsupported(
                "positive resource monopoly recalculation",
            ));
        }
        Ok(())
    }
    fn revealed_notifications(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut stock::Runtime<'_, P>,
        location: PlotCoord,
    ) -> Result<(), Error> {
        self.check(state)?;
        let plot = crate::unit_placement_services::plot_index_at::<std::convert::Infallible>(
            state, location,
        )
        .map_err(|_| Error::Unsupported("camp notification plot"))?
        .ok_or(Error::Unsupported("camp notification plot"))? as usize;
        if state.world().map().visibility.iter().any(|v| {
            v.visibility
                .outgoing_visibility
                .iter()
                .any(|grant| grant.count != 0)
        }) {
            return Err(Error::Unsupported("camp shared revelation grants"));
        }
        for &player in self.alive_major_ids {
            let revealed = state
                .world()
                .visibility(player)
                .ok_or(Error::Unsupported("camp revealed major visibility"))?
                .revealed
                .get(plot)
                .ok_or(Error::Unsupported("camp revealed major plot"))?;
            match revealed {
                0 => {}
                1 => return Err(Error::Unsupported("revealed camp notification send")),
                _ => return Err(Error::Unsupported("camp canonical revealed byte")),
            }
        }
        Ok(())
    }
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock::Runtime<'_, P>,
        plot: usize,
        event: u32,
        argument: i32,
    ) -> Result<(), Error> {
        self.check(state)?;
        if !matches!(event, 0xa625c726 | 0xe5b20b82 | 0xf391d7b3) {
            return Err(Error::Unsupported("camp scalar report type"));
        }
        let map = state.world().map();
        if map.header.width <= 0 || plot >= map.plots.len() {
            return Err(Error::Unsupported("camp scalar report plot"));
        }
        self.reports.push(Report {
            location: PlotCoord {
                x: (plot % map.header.width as usize) as i32,
                y: (plot / map.header.width as usize) as i32,
            },
            event,
            argument,
        });
        backend
            .turn_report_records
            .push(stock::turn_reports::Record::Report {
                event,
                player: argument,
            });
        Ok(())
    }
    fn behavior_threshold(
        &self,
        _state: &LoadedState,
        _index: usize,
        _behavior: Behavior,
    ) -> Result<i32, Error> {
        Err(Error::Unsupported("camp positive behavior threshold"))
    }
    fn start_behavior(
        &mut self,
        _state: &mut LoadedState,
        _backend: &mut stock::Runtime<'_, P>,
        _index: usize,
        _target: ComponentId,
        _behavior: Behavior,
    ) -> Result<bool, Error> {
        Err(Error::Unsupported("camp positive behavior start"))
    }
}
