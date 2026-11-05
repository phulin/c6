//! Reached ordinary Farm constructed subscribers, evaluated from current state.
//! An actual positive boost or unsupported moment returns an error; no native
//! callback results, count answers or AFTER state are accepted as inputs.
use super::Definitions;
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{loaded_state::LoadedState, map::PlotFlag};

#[derive(Clone, Copy, Debug)]
pub enum Subscriber {
    AttachModifiers,
    SeasideResort,
    History,
    NumberOfImprovements { boost_row: i64 },
    ImprovementConstructed { boost_row: i64 },
    AirbaseForeignContinent,
    UnitAndImprovement { boost_row: i64 },
    Resources { owner: i32 },
    Barbarian,
    Achievement,
}
struct Boost {
    row: i64,
    items: i32,
    improvement: Option<u32>,
    unit: Option<u32>,
    requires_resource: bool,
    resource: Option<u32>,
    resource_specific: bool,
}
#[derive(Debug)]
pub struct Error(pub &'static str);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.0) }
}
impl std::error::Error for Error {}
/// The same Economic resource owners used by camp construction. They are
/// retained across actions; resource-free Farm construction does not alter them.
pub use crate::barbarian_camps::services::Economic;
pub struct Current {
    binding: c6_core::core_player::initialization::WireBinding,
    order: Vec<Subscriber>,
    history_special: Vec<i32>,
    boosts: Vec<Boost>,
    yields: crate::registry::LinuxDefinitionRegistry,
    modifier_improvements: Vec<u32>,
    resort_hash: Option<u32>,
    civilizations: Vec<(i32, String)>,
    pub economic: Economic,
    pub appeal_changed_subscribers: Option<Vec<u64>>,
}
impl Current {
    pub fn from_initial(state: &LoadedState, rules: &Rules, order: Vec<Subscriber>,
        history_special: Vec<i32>, civilizations: Vec<(i32, String)>, economic: Economic)
        -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error(name));
        let optional = |row: crate::Row<'_>, key| match row.get(key).map_err(|_| Error(key))? {
            SqlValue::Null => Ok(None), SqlValue::Text(value) => Ok(Some(linux_type_hash(value))),
            _ => Err(Error(key)),
        };
        let boosts = table("Boosts")?.rows().map(|r| Ok(Boost {
            row: r.row_id().ok_or(Error("Boost row"))?.0,
            items: r.integer("NumItems").ok().and_then(|v| i32::try_from(v).ok()).ok_or(Error("Boost NumItems"))?,
            improvement: optional(r,"ImprovementType")?, unit: optional(r,"Unit1Type")?,
            resource: optional(r,"ResourceType")?,
            resource_specific: r.text("BoostClass").map_err(|_|Error("BoostClass"))? == "BOOST_TRIGGER_IMPROVE_SPECIFIC_RESOURCE",
            requires_resource: r.integer("RequiresResource").map_err(|_| Error("Boost RequiresResource"))? != 0,
        })).collect::<Result<Vec<_>,Error>>()?;
        let modifier_improvements = table("ImprovementModifiers")?.rows()
            .map(|r|r.text("ImprovementType").map(|v|linux_type_hash(v.as_bytes())).map_err(|_|Error("ImprovementModifiers")))
            .collect::<Result<Vec<_>,_>>()?;
        let registry = crate::unit_initialization_definitions::collection(rules,"Improvements","ImprovementType")
            .map_err(|_| Error("Improvement definitions"))?;
        let resort_hash = registry.find_index(linux_type_hash(b"IMPROVEMENT_BEACH_RESORT"))
            .map(|_| linux_type_hash(b"IMPROVEMENT_BEACH_RESORT"));
        let yields = crate::unit_initialization_definitions::collection(rules,"Yields","YieldType")
            .map_err(|_| Error("Yield definitions"))?;
        if history_special.len()!=8 { return Err(Error("History improvement members")); }
        Ok(Self { binding: state.binding().clone(), order, history_special, boosts,
            yields, modifier_improvements, resort_hash, civilizations, economic, appeal_changed_subscribers: None })
    }
    fn counts(&self, state: &LoadedState, owner: i32, kind: Option<u32>) -> Result<i32, Error> {
        let mut count=0i32;
        for p in &state.world().map().plots {
            let current_owner = crate::movement_placement_state::plot_owner(p)
                .map_err(|_| Error("Stats improvement owner"))?;
            if current_owner != owner || p.flags.get(PlotFlag::ImprovementPillaged) { continue; }
            if let Some(kind)=kind {
                if p.improvement_hash==kind { count=count.wrapping_add(1); }
            } else if p.improvement_hash!=u32::MAX && p.improvement_hash!=state.retained().barbarian.improvement_hash {
                // Stats.GetNumImprovedTiles excludes district plots.
                let district=p.city_attributes.as_ref().map(|a|a.district).unwrap_or(u32::MAX);
                if district==u32::MAX { count=count.wrapping_add(1); }
            }
        }
        Ok(count)
    }
    /// Dispatch the retained live order, using current counts after owner sight.
    pub fn constructed(&mut self, state: &mut LoadedState, plot: usize, defs: &Definitions, gossip_history: &[c6_core::city_power_activation::gossip_delivery::TurnVisibility])
        -> Result<(), Error> {
        if state.binding()!=&self.binding { return Err(Error("Farm constructed binding")); }
        for index in 0..self.order.len() {
            let p=state.world().map().plots.get(plot).ok_or(Error("Farm constructed plot"))?;
            if p.improvement_hash!=defs.improvement_hash { return Err(Error("Farm construction changed definition")); }
            let owner=crate::movement_placement_state::plot_owner(p).map_err(|_| Error("Farm owner"))?;
            match self.order[index] {
                Subscriber::AttachModifiers if !self.modifier_improvements.contains(&defs.improvement_hash) => {},
                Subscriber::SeasideResort => {
                    //29b82d0 rejects a different existing BEACH_RESORT
                    // definition, and falls through when absent. Its occurrence constructor
                    // has no gameplay/RNG effects; AddGossipItem29b25d0 returns
                    // before writes/reporting when the current history is empty.
                    if !self.resort_hash.is_some_and(|h| h != defs.improvement_hash)
                        && !gossip_history.is_empty() {
                        return Err(Error("Farm reached BeachResort gossip delivery"));
                    }
                },
                Subscriber::History => {
                    if self.history_special.contains(&defs.improvement_index) { return Err(Error("Farm special historical moment")); }
                    let feature=c6_core::plot_yield::FeatureYieldStorage::from_world(state.world(),&self.yields)
                        .map_err(|_| Error("Farm Feature yield storage"))?;
                    for y in 0..self.yields.entries().len() {
                        if feature.get(plot as i32,y as i32).map_err(|_| Error("Farm feature yield"))?>0 {
                            return Err(Error("Farm positive feature historical moment"));
                        }
                    }
                },
                Subscriber::NumberOfImprovements {boost_row} => {
                    if owner==-1 { continue; }
                    let boost=self.boosts.iter().find(|b|b.row==boost_row).ok_or(Error("Farm Boost row"))?;
                    if self.counts(state,owner,None)? >= boost.items { return Err(Error("Farm reached count TriggerBoost")); }
                },
                Subscriber::ImprovementConstructed {boost_row} => {
                    if owner==-1 { continue; }
                    let boost=self.boosts.iter().find(|b|b.row==boost_row).ok_or(Error("Farm Boost row"))?;
                    if boost.improvement==Some(defs.improvement_hash)
                        && self.counts(state,owner,Some(defs.improvement_hash))? >= boost.items
                        && (!boost.requires_resource || p.resource_hash!=u32::MAX)
                        && (!boost.resource_specific || boost.resource==Some(p.resource_hash)) {
                        return Err(Error("Farm reached improvement TriggerBoost"));
                    }
                },
                Subscriber::AirbaseForeignContinent if p.city_attributes.as_ref().is_none_or(|a|a.district==u32::MAX) => {},
                Subscriber::UnitAndImprovement {boost_row} => {
                    let boost=self.boosts.iter().find(|b|b.row==boost_row).ok_or(Error("Farm Boost row"))?;
                    if owner!=-1 && boost.improvement.is_some() {
                        if let Some(unit) = boost.unit {
                            let has_unit=state.world().player(owner).map_err(|_|Error("UnitAndImprovement owner"))?
                                .unit_records().map_err(|_|Error("UnitAndImprovement units"))?.iter()
                                .any(|r|r.value.e_type_index==unit);
                            if has_unit && state.world().map().plots.iter().any(|p|
                                crate::movement_placement_state::plot_owner(p).ok()==Some(owner)
                                && boost.improvement==Some(p.improvement_hash)
                                && boost.resource.is_none_or(|r|p.resource_hash==r)) {
                                return Err(Error("positive unit/improvement boost continuation"));
                            }
                        }
                    }
                },
                Subscriber::Resources {owner: receiver} if receiver!=owner || p.resource_hash==u32::MAX => {},
                Subscriber::Barbarian if state.retained().barbarian.flag_168_raw!=0
                    || defs.improvement_hash!=state.retained().barbarian.improvement_hash => {},
                Subscriber::Achievement => {
                    //28f9240: all specialized-type branches reject ordinary Farm.
                    // The expansion2 renewable-set scan cannot fire without any
                    // of those improvements; the generic six-tile guard is live.
                    let civ=self.civilizations.iter().find(|c|c.0==owner).ok_or(Error("Farm achievement civilization"))?;
                    let energy=[linux_type_hash(b"IMPROVEMENT_GEOTHERMAL_PLANT"),linux_type_hash(b"IMPROVEMENT_SOLAR_FARM"),linux_type_hash(b"IMPROVEMENT_WIND_FARM")];
                    if matches!(civ.1.as_str(),"CIVILIZATION_FRANCE"|"CIVILIZATION_SCOTLAND")
                        || self.counts(state,owner,None)?>5
                        || state.world().map().plots.iter().any(|p|energy.contains(&p.improvement_hash)) {
                        return Err(Error("Farm positive achievement continuation"));
                    }
                },
                _ => return Err(Error("Farm positive constructed subscriber continuation")),
            }
        }
        Ok(())
    }
    /// Economic.CalculateMonopolies: all nonpositive totals and unassigned
    /// summaries reject every alive-major update before writes/callbacks/RNG.
    pub fn calculate_monopolies(&self, alive_majors: &[i32]) -> Result<(), Error> {
        let e=&self.economic;
        if e.resource_count!=e.totals.len() || e.resource_count!=e.summaries.len()
            || e.owned.len()!=64 || e.owned.iter().any(|r|r.len()!=e.resource_count) {
            return Err(Error("Farm Economic array dimensions"));
        }
        if e.totals.iter().any(|t|t.wrapping_shl(8)>0)
            || e.summaries.iter().any(|(o,_)|alive_majors.contains(o)) {
            return Err(Error("Farm positive monopoly recalculation"));
        }
        Ok(())
    }
}
