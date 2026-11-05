//! Source ordered ordinary improvement pillage callbacks. Missing registrations
//! and positive unsupported continuations are errors at the reached boundary.
use crate::{loaded_actions::stock_notifications, registry::linux_type_hash, Rules};
use c6_core::{
    city_power_activation::gossip_delivery::TurnVisibility,
    core_player::initialization::WireBinding, game_capabilities::Membership,
    loaded_state::LoadedState, map::PlotFlag, unit::ComponentId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImprovementSubscriber {
    DetachModifiers,
    Economic,
    FreeCities { owner: i32 },
    PlayerResources { owner: i32 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PillageSubscriber {
    Barbarian { owner: i32 },
    Gossip,
}
pub struct Inputs {
    binding: WireBinding,
    improvement_order: Vec<ImprovementSubscriber>,
    pillage_order: Vec<PillageSubscriber>,
    industry_or_corporation: Vec<u32>,
    /// Source-backed complete Improvement type domain in the canonical backend.
    /// This includes actual cache misses, not an absent import treated as empty.
    complete_improvement_objects: bool,
}
impl Inputs {
    pub fn from_initial(
        state: &LoadedState,
        rules: &Rules,
        improvement_order: Vec<ImprovementSubscriber>,
        pillage_order: Vec<PillageSubscriber>,
        complete_improvement_objects: bool,
    ) -> Result<Self, String> {
        let mut industry_or_corporation = Vec::new();
        if let Some(table) = rules.gameplay().table("Improvements_MODE") {
            for row in table.rows() {
                if row
                    .integer("Industry")
                    .map_err(|e| format!("Pillage Industry: {e:?}"))?
                    != 0
                    || row
                        .integer("Corporation")
                        .map_err(|e| format!("Pillage Corporation: {e:?}"))?
                        != 0
                {
                    industry_or_corporation.push(linux_type_hash(
                        row.text("ImprovementType")
                            .map_err(|e| format!("Pillage mode improvement: {e:?}"))?
                            .as_bytes(),
                    ));
                }
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            improvement_order,
            pillage_order,
            industry_or_corporation,
            complete_improvement_objects,
        })
    }
}
pub struct Current<'s, 'a, P = c6_core::unit::OperationParameters> {
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub inputs: &'s Inputs,
    pub capabilities: &'s Membership,
    /// Borrow the same current Gossip history used by other actions and turns.
    pub gossip_history: &'s [TurnVisibility],
}
fn fail(e: impl std::fmt::Debug) -> String {
    format!("Pillage callback: {e:?}")
}
impl<P> Current<'_, '_, P> {
    fn check(&self, state: &LoadedState) -> Result<(), String> {
        if state.binding() != &self.inputs.binding {
            return Err("Pillage callback binding".into());
        }
        self.backend.check(state).map_err(fail)
    }
    pub fn improvement_pillaged(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        improvement: u32,
    ) -> Result<(), String> {
        self.check(state)?;
        for subscriber in &self.inputs.improvement_order {
            let p = state
                .world()
                .map()
                .plots
                .get(plot)
                .ok_or("Pillage callback plot")?;
            if p.improvement_hash != improvement || !p.flags.get(PlotFlag::ImprovementPillaged) {
                return Err("Pillage callback improvement state".into());
            }
            let owner = crate::movement_placement_state::plot_owner(p).map_err(fail)?;
            match *subscriber {
                ImprovementSubscriber::DetachModifiers => {
                    let city = p
                        .city_attributes
                        .as_ref()
                        .map(|a| ComponentId {
                            owner: i16::from(a.player),
                            component_type: 2,
                            id: a.owning_city,
                        })
                        .unwrap_or(ComponentId {
                            owner: -1,
                            component_type: 0,
                            id: u32::MAX,
                        });
                    self.backend
                        .modifier_owners_mut()
                        .map_err(fail)?
                        .effects
                        .clear_improvement_modifiers(
                            state,
                            plot as u32,
                            improvement,
                            city,
                            owner,
                            self.inputs.complete_improvement_objects,
                        )
                        .map_err(fail)?;
                }
                ImprovementSubscriber::Economic => {
                    // RecalculateValues26472c0 -> IsActive26471c0. The active
                    // branch preserves resource control and monopoly effects;
                    // it cannot be replaced with the old array snapshot.
                    if self.capabilities.has(0x10c4_0410) {
                        return Err("Pillage active Economic recalculation".into());
                    }
                    if self.inputs.industry_or_corporation.contains(&improvement) {
                        return Err("Pillage Industry/Corporation Economic continuation".into());
                    }
                }
                ImprovementSubscriber::FreeCities { owner: receiver } => {
                    if owner == receiver {
                        return Err("Pillage reached FreeCities spy continuation".into());
                    }
                }
                ImprovementSubscriber::PlayerResources { owner: receiver } => {
                    // EventManager2b01ac0 forwards to Resources2b70a40.
                    // Resource-free GetResourceExtractableAt returns -1 for
                    // both old/new improvements; HandleResourceChange returns
                    // before resource counters, yields, callbacks or RNG.
                    if owner == receiver && p.resource_hash != u32::MAX {
                        return Err("Pillage reached resource extraction change".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub fn on_pillage(
        &mut self,
        state: &mut LoadedState,
        actor: ComponentId,
        _plot: usize,
        _improvement: u32,
    ) -> Result<(), String> {
        self.check(state)?;
        for subscriber in &self.inputs.pillage_order {
            match *subscriber {
                PillageSubscriber::Barbarian { owner } => {
                    if i32::from(actor.owner) == owner {
                        return Err("Pillage reached Barbarian tribe counter".into());
                    }
                }
                PillageSubscriber::Gossip => {
                    // Occurrence construction29c5e60/29cb1e0/29c7e20 only
                    // initializes type/turn/player and payload. AddGossipItem
                    // 29b25d0 returns before writes when history+138 is empty.
                    if !self.gossip_history.is_empty() {
                        return Err("Pillage reached Gossip delivery".into());
                    }
                }
            }
        }
        Ok(())
    }
}
