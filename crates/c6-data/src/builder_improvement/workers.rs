//! Farm's synchronous owning-city reassignment, using existing citizen/yield rules.
use crate::{city_production_turn::appeal, city_turn::{upkeep::citizens, Error},
    loaded_actions::stock_notifications::Runtime};
use c6_core::{loaded_state::LoadedState, unit::{ComponentId, PlotCoord}};

#[derive(Clone, Copy, Debug)]
pub enum Report {
    WorkersChanged { city: ComponentId, location: PlotCoord, current: i32, previous: i32 },
    AppealChanged { plot: u32, current: i32 },
    AppealReported { plot: u32 },
    ImprovementActivated { location: PlotCoord, active: bool },
}
pub struct Current<'a, 'd, P> {
    pub backend: &'a mut Runtime<'d, P>,
    pub definitions: &'a citizens::Definitions,
    pub appeal: &'a appeal::Definitions,
    pub fallout: &'a [i32],
    pub appeal_subscribers: Option<&'a [u64]>,
    pub reports: &'a mut Vec<Report>,
}
struct Appeal<'a> { subscribers: Option<&'a [u64]>, reports: &'a mut Vec<Report> }
impl appeal::Effects for Appeal<'_> {
    type Error = Error;
    fn appeal_changed(&mut self, _: &mut LoadedState, plot: u32, current: i32) -> Result<(), Error> {
        match self.subscribers {
            Some([]) => { self.reports.push(Report::AppealChanged { plot, current }); Ok(()) },
            Some(_) => Err(Error::Unsupported("positive appeal requirement subscribers")),
            None => Err(Error::Unsupported("current appeal changed subscriber registry")),
        }
    }
    fn appeal_report(&mut self, _: &mut LoadedState, plot: u32) -> Result<(), Error> {
        // Native PlotAppealChanged reporting reaches presentation cache/event
        // consumers. The gameplay signal above has already completed.
        self.reports.push(Report::AppealReported { plot }); Ok(())
    }
}
impl<P> Current<'_, '_, P> {
    fn current<'q>(&'q self, territory: &'q crate::city_territory::Runtime) -> Result<citizens::Current<'q>, Error> {
        let query=self.backend.city_query_inputs().ok_or(Error::Unsupported("Farm current city queries"))?;
        Ok(citizens::Current { definitions: query.definitions, territory,
            drought_ids: query.current.drought_ids, fallout_turns: self.fallout })
    }
}
impl<P> citizens::Effects for Current<'_, '_, P> {
    fn food_surplus_q8(&mut self,s:&LoadedState,o:i32,c:u32)->Result<i32,Error>{
        let q=self.backend.city_query_inputs().ok_or(Error::Unsupported("Farm current city queries"))?;
        let food=q.definitions.yield_index(b"YIELD_FOOD").ok_or(Error::Unsupported("Farm Food index"))?;
        let amount=self.backend.current_city_yield(s,o,c,food,0)?;
        Ok(amount.wrapping_sub(s.city(o,c)?.header.population
            .wrapping_mul(q.definitions.growth.food_consumption_per_population).wrapping_shl(8)))
    }
    fn visit_plot(&mut self,s:&mut LoadedState,o:i32,c:u32,p:usize)->Result<(),Error>{
        appeal::update_and_test_adjacent(s,Some(p),self.appeal,&mut Appeal {subscribers:self.appeal_subscribers,reports:self.reports},
            &mut |s:&mut LoadedState,_:&mut Appeal<'_>,a:u32| {
                let p=s.world().map().plots.get(a as usize).ok_or(Error::Unsupported("Farm worker adjacent plot"))?;
                Ok(!p.city_attributes.as_ref().is_some_and(|a|i32::from(a.player)==o&&a.owning_city==c))
            }).map(|_|()).map_err(|_|Error::Unsupported("Farm worker appeal continuation"))
    }
    fn capacity(&mut self,s:&LoadedState,o:i32,c:u32,p:PlotCoord)->Result<i32,Error>{
        let q=self.backend.city_query_inputs().ok_or(Error::Unsupported("Farm current city queries"))?;
        let territory=q.current.territory.read()?;
        citizens::capacity(s,o,c,p,self.definitions,&self.current(&territory)?)
    }
    fn plot_yields(&mut self,s:&LoadedState,o:i32,c:u32,p:usize)
        ->Result<Vec<c6_core::city_citizen_assignment::Yield>,Error>{
        let q=self.backend.city_query_inputs().ok_or(Error::Unsupported("Farm current city queries"))?;
        let territory=q.current.territory.read()?;
        citizens::plot_yields(s,o,c,p,&self.current(&territory)?)
    }
    fn food_and_gold_indices(&self)->Result<(i32,i32),Error>{
        let q=self.backend.city_query_inputs().ok_or(Error::Unsupported("Farm current city queries"))?;
        Ok((q.definitions.yield_index(b"YIELD_FOOD").ok_or(Error::Unsupported("Farm Food index"))?,
            q.definitions.yield_index(b"YIELD_GOLD").ok_or(Error::Unsupported("Farm Gold index"))?))
    }
    fn workers_changed(&mut self,s:&mut LoadedState,o:i32,c:u32,i:i32,old:i32)->Result<(),Error>{
        let city=s.city(o,c)?;
        let current=city.citizens.working_plot.get(i as usize).ok_or(Error::Unsupported("Farm worker index"))?
            .wrapping_add(*city.citizens.forced_working_plot.get(i as usize).ok_or(Error::Unsupported("Farm forced worker index"))?);
        if current==old { return Ok(()); }
        let center=PlotCoord{x:city.header.location_x,y:city.header.location_y};
        let Some(location)=c6_core::city_citizen_assignment::plot_location(s.world().map(),center,i)
            .map_err(|_|Error::Unsupported("Farm worker location"))? else {return Ok(());};
        // Native286f2b0 sends output-only PlotWorkersChanged, then the
        // ImprovementActivated visual event only when activity crosses zero.
        self.reports.push(Report::WorkersChanged { city:ComponentId{owner:o as i16,component_type:2,id:city.header.id},
            location,current,previous:old });
        let (_,p)=c6_core::movement_world::plot_at(s.world().map(),location)
            .map_err(|_|Error::Unsupported("Farm worker plot"))?;
        if p.improvement_hash!=u32::MAX && i32::from(p.improvement_owner)==o && (current>0)!=(old>0) {
            self.reports.push(Report::ImprovementActivated{location,active:current>0});
        }
        Ok(())
    }
}
