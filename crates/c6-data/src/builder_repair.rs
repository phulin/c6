//! Ordinary Linux XP2 Builder Repair over the retained world. Unsupported
//! callback branches fail at their reached boundary and preserve the prefix.
use crate::{registry::linux_type_hash, Rules};
use c6_core::{
    loaded_state::LoadedState,
    map::{Component, PlotFlag, UnitReference},
    skip_turn::StartData,
    unit::{ComponentId, OperationNode, Unit},
};
pub mod stock;
pub const OPERATION_HASH: u32 = c6_core::unit::REPAIR_HASH;
#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Unsupported(&'static str),
    State(c6_core::loaded_state::Error),
    Effect(E),
}
pub(crate) struct Boost {
    pub row: i64,
    pub improvement: Option<u32>,
    pub items: i32,
    pub requires_resource: bool,
    pub resource_specific: bool,
    pub resource: Option<u32>,
}
pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    pub(crate) improvement_hash: u32,
    pub(crate) improvement_index: i32,
    pub(crate) modifier_improvements: Vec<u32>,
    pub(crate) boosts: Vec<Boost>,
    pub(crate) reporting: crate::loaded_actions::pillage::Definitions,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error<()>> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux Repair rules"));
        }
        let hash = linux_type_hash(b"IMPROVEMENT_FARM");
        let registry = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Error::Missing("Repair improvements"))?;
        let index = registry
            .find_index(hash)
            .ok_or(Error::Missing("Repair Farm"))? as i32;
        // CanStart scans in SQL definition order. A non-repair-only entry admits
        // any charged unit; the ordinary Farm has a Builder construction entry.
        let build = rules
            .gameplay()
            .table("Improvement_ValidBuildUnits")
            .ok_or(Error::Missing("Repair build units"))?;
        let mut admitted = false;
        for row in build
            .rows()
            .filter(|r| r.text("ImprovementType").ok() == Some("IMPROVEMENT_FARM"))
        {
            if row
                .integer("ValidRepairOnly")
                .map_err(|_| Error::Missing("ValidRepairOnly"))?
                == 0
                || row.text("UnitType").ok() == Some("UNIT_BUILDER")
            {
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Err(Error::Unsupported("ordinary Farm repair unit rule"));
        }
        let modifier_improvements = rules
            .gameplay()
            .table("ImprovementModifiers")
            .ok_or(Error::Missing("Repair improvement modifiers"))?
            .rows()
            .map(|r| {
                r.text("ImprovementType")
                    .map(|n| linux_type_hash(n.as_bytes()))
                    .map_err(|_| Error::Missing("Repair modifier type"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let boosts = rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("Repair boosts"))?
            .rows()
            .map(|r| {
                let optional = |key| match r
                    .get(key)
                    .map_err(|_| Error::Missing("Repair boost reference"))?
                {
                    crate::SqlValue::Null => Ok(None),
                    crate::SqlValue::Text(v) => Ok(Some(linux_type_hash(v))),
                    _ => Err(Error::Missing("Repair boost reference")),
                };
                Ok(Boost {
                    row: r.row_id().ok_or(Error::Missing("Repair boost row"))?.0,
                    improvement: optional("ImprovementType")?,
                    items: r
                        .integer("NumItems")
                        .map_err(|_| Error::Missing("Repair boost count"))?
                        as i32,
                    requires_resource: r
                        .integer("RequiresResource")
                        .map_err(|_| Error::Missing("Repair boost resource"))?
                        != 0,
                    resource_specific: r
                        .text("BoostClass")
                        .map_err(|_| Error::Missing("Repair boost class"))?
                        == "BOOST_TRIGGER_IMPROVE_SPECIFIC_RESOURCE",
                    resource: optional("ResourceType")?,
                })
            })
            .collect::<Result<Vec<_>, Error<()>>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            improvement_hash: hash,
            improvement_index: index,
            modifier_improvements,
            boosts,
            reporting: crate::loaded_actions::pillage::Definitions::from_rules(state, rules)
                .map_err(|_| Error::Missing("Repair reporting definitions"))?,
        })
    }
}
/// Current drought objects, never a cached legality answer. Empty is a complete
/// admitted domain; nonempty objects require their exact membership continuation.
pub struct Inputs<'a> {
    pub drought_ids: &'a [i32],
}
pub fn can_start(
    state: &LoadedState,
    defs: &Definitions,
    inputs: &Inputs<'_>,
    unit: &Unit,
    visible_only: bool,
) -> Result<bool, Error<()>> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("Repair binding"));
    }
    if unit.i_build_charges <= 0 || !c6_core::skip_turn::legality::can_move(unit) {
        return Ok(false);
    }
    let (index, plot) = c6_core::movement_world::plot_at(state.world().map(), unit.k_location)
        .map_err(|_| Error::Missing("Repair plot"))?;
    if plot.improvement_hash == u32::MAX || !plot.flags.get(PlotFlag::ImprovementPillaged) {
        return Ok(false);
    }
    let terrain = state
        .world()
        .map()
        .components
        .iter()
        .find_map(|c| match c {
            Component::Terrain(t) => Some(t),
            _ => None,
        })
        .ok_or(Error::Missing("Repair terrain"))?;
    if terrain
        .coastal_lowlands
        .iter()
        .any(|p| p.plot_index == index as i32 && p.flooded)
    {
        return Ok(false);
    }
    if unit.e_type_index != linux_type_hash(b"UNIT_BUILDER")
        || plot.improvement_hash != defs.improvement_hash
    {
        return Err(Error::Unsupported("ordinary Builder Farm Repair"));
    }
    let owner = crate::movement_placement_state::plot_owner(plot)
        .map_err(|_| Error::Missing("Repair political owner"))?;
    if owner != -1 && owner != unit.e_owner {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("Repair territory player"))?;
        if player.influence.suzerain != unit.e_owner {
            return Ok(false);
        }
    }
    if !visible_only && !inputs.drought_ids.is_empty() {
        return Err(Error::Unsupported("Repair active drought membership"));
    }
    Ok(true)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    Observer { x: i32, y: i32 },
    Operation { unit: ComponentId, improvement: i32 },
    Visible(ComponentId),
    ImprovementChanged { plot: usize },
}
pub trait Effects {
    type Error;
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Self::Error>;
    fn repaired(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        improvement: u32,
    ) -> Result<(), Self::Error>;
    fn update_appeal(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Self::Error>;
    fn messages_disabled(&self) -> bool;
    fn local_plot_visible(
        &self,
        state: &LoadedState,
        actor: UnitReference,
    ) -> Result<bool, Self::Error>;
    fn finish_moves(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Self::Error>;
}
pub fn start<E: Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    defs: &Definitions,
    actor: UnitReference,
    node: &OperationNode,
) -> Result<StartData, Error<E::Error>> {
    if state.binding() != &defs.binding
        || node.operation_hash != OPERATION_HASH
        || !matches!(
            node.parameters,
            c6_core::unit::OperationParameters::Repair { .. }
        )
    {
        return Err(Error::Unsupported("Repair request/binding"));
    }
    let unit = state
        .world()
        .unit(actor)
        .map_err(|_| Error::Missing("Repair unit"))?
        .ok_or(Error::Missing("Repair unit"))?
        .record;
    let location = unit.k_location;
    let owner = unit.e_owner;
    if effects
        .observer_can_see_owner(state, owner)
        .map_err(Error::Effect)?
    {
        effects
            .report(
                state,
                Report::Observer {
                    x: location.x,
                    y: location.y,
                },
            )
            .map_err(Error::Effect)?;
    }
    let (plot, p) = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Missing("Repair current plot"))?;
    if p.improvement_hash == u32::MAX {
        return Ok(StartData {
            delete: true,
            action: false,
        });
    }
    if p.improvement_hash != defs.improvement_hash {
        return Err(Error::Unsupported("Repair current improvement"));
    }
    let id = ComponentId {
        owner: owner as i16,
        component_type: 1,
        id: actor.id,
    };
    effects
        .report(
            state,
            Report::Operation {
                unit: id,
                improvement: defs.improvement_index,
            },
        )
        .map_err(Error::Effect)?;
    // SetImprovementPillaged rechecks current state after operation reporting.
    let p = state
        .world()
        .map()
        .plots
        .get(plot)
        .ok_or(Error::Missing("Repair current plot"))?;
    if p.flags.get(PlotFlag::ImprovementPillaged) && !p.flags.get(PlotFlag::Impassable) {
        state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(plot)
            .ok_or(Error::Missing("Repair mutable plot"))?
            .flags
            .set(PlotFlag::ImprovementPillaged, false);
        effects
            .repaired(state, plot, defs.improvement_hash)
            .map_err(Error::Effect)?;
        if !effects.messages_disabled() {
            effects
                .report(state, Report::ImprovementChanged { plot })
                .map_err(Error::Effect)?;
            effects.update_appeal(state, plot).map_err(Error::Effect)?;
        }
    }
    if effects
        .local_plot_visible(state, actor)
        .map_err(Error::Effect)?
    {
        effects
            .report(state, Report::Visible(id))
            .map_err(Error::Effect)?;
    }
    effects.finish_moves(state, actor).map_err(Error::Effect)?;
    // Repair never changes charges, reallocates citizens or destroys the unit.
    Ok(StartData {
        delete: true,
        action: true,
    })
}
