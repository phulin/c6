//! Linux XP2 ordinary Builder forest removal. The action uses current retained
//! state; reached callback errors retain their mutation prefix. Source support
//! alone does not establish native action or complete state/turn parity.
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{PlotFlag, UnitReference},
    skip_turn::StartData,
    unit::{ComponentId, OperationNode, OperationParameters, Unit},
};
pub mod stock;
pub const OPERATION_HASH: u32 = c6_core::unit::REMOVE_FEATURE_HASH;
#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Unsupported(&'static str),
    State(c6_core::loaded_state::Error),
    Effect(E),
}
pub struct Definitions {
    binding: WireBinding,
    pub feature_hash: u32,
    pub feature_index: i32,
    remove_tech: Option<(u32, i32)>,
    pub production: i32,
    pub(crate) technologies: Vec<u32>,
    pub(crate) civics: Vec<u32>,
    pub(crate) passable_terrains: Vec<u32>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error<()>> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux forest removal rules"));
        }
        let builder = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Builder definition table"))?
            .rows()
            .find(|r| r.text("UnitType").ok() == Some("UNIT_BUILDER"))
            .ok_or(Error::Missing("Builder definition"))?;
        for field in ["Combat", "RangedCombat", "Bombard"] {
            if builder
                .integer(field)
                .map_err(|_| Error::Missing("Builder combat definition"))?
                != 0
            {
                return Err(Error::Unsupported(
                    "ordinary noncombat Builder forest removal",
                ));
            }
        }
        let feature_hash = linux_type_hash(b"FEATURE_FOREST");
        let registry =
            crate::unit_initialization_definitions::collection(rules, "Features", "FeatureType")
                .map_err(|_| Error::Missing("forest definitions"))?;
        let index = registry
            .find_index(feature_hash)
            .ok_or(Error::Missing("forest definition"))?;
        let row = rules
            .gameplay()
            .table("Features")
            .and_then(|t| t.row_by_id(registry.entries()[index].database_rowid))
            .ok_or(Error::Missing("forest row"))?;
        if row
            .integer("Removable")
            .map_err(|_| Error::Missing("forest Removable"))?
            == 0
        {
            return Err(Error::Unsupported("nonremovable forest definition"));
        }
        for (field, value) in [("NaturalWonder", 0), ("Tiles", 1), ("Impassable", 0)] {
            if row
                .integer(field)
                .map_err(|_| Error::Missing("ordinary forest rule"))?
                != value
            {
                return Err(Error::Unsupported(
                    "ordinary single-plot passable forest definition",
                ));
            }
        }
        let passable_terrains = rules
            .gameplay()
            .table("Terrains")
            .ok_or(Error::Missing("forest terrain definitions"))?
            .rows()
            .filter(|r| r.integer("Impassable").ok() == Some(0))
            .map(|r| {
                r.text("TerrainType")
                    .map(|n| linux_type_hash(n.as_bytes()))
                    .map_err(|_| Error::Missing("forest terrain type"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let remove_tech = match row
            .get("RemoveTech")
            .map_err(|_| Error::Missing("forest RemoveTech"))?
        {
            SqlValue::Null => None,
            SqlValue::Text(value) => {
                let hash = linux_type_hash(value);
                let techs = crate::unit_initialization_definitions::collection(
                    rules,
                    "Technologies",
                    "TechnologyType",
                )
                .map_err(|_| Error::Missing("forest technology definitions"))?;
                Some((
                    hash,
                    techs
                        .find_index(hash)
                        .ok_or(Error::Missing("forest technology"))? as i32,
                ))
            }
            _ => return Err(Error::Missing("forest RemoveTech reference")),
        };
        let mut production = None;
        for row in rules
            .gameplay()
            .table("Feature_Removes")
            .ok_or(Error::Missing("Feature_Removes"))?
            .rows()
            .filter(|r| r.text("FeatureType").ok() == Some("FEATURE_FOREST"))
        {
            if row
                .text("YieldType")
                .map_err(|_| Error::Missing("forest removal yield type"))?
                != "YIELD_PRODUCTION"
            {
                return Err(Error::Unsupported("additional forest removal yields"));
            }
            if production.is_some() {
                return Err(Error::Unsupported("multiple forest removal yield rows"));
            }
            production = Some(
                row.integer("Yield")
                    .map_err(|_| Error::Missing("forest removal yield"))? as i32,
            );
        }
        let hashes = |table, column| -> Result<Vec<u32>, Error<()>> {
            let definitions =
                crate::unit_initialization_definitions::collection(rules, table, column)
                    .map_err(|_| Error::Missing("forest research definitions"))?;
            Ok(definitions
                .entries()
                .iter()
                .map(|e| linux_type_hash(&e.type_name))
                .collect())
        };
        Ok(Self {
            binding: state.binding().clone(),
            feature_hash,
            feature_index: index as i32,
            remove_tech,
            production: production.ok_or(Error::Missing("forest removal production row"))?,
            passable_terrains,
            technologies: hashes("Technologies", "TechnologyType")?,
            civics: hashes("Civics", "CivicType")?,
        })
    }
}
/// Actual current park and fallout managers. These are state, never cached
/// native legality answers. Positive fallout requires exact work-permission.
pub struct Inputs<'a> {
    pub fallout_turns: &'a [i32],
    pub national_parks: &'a [c6_core::game_national_parks::ParkData],
}
pub fn can_start(
    state: &LoadedState,
    defs: &Definitions,
    inputs: &Inputs<'_>,
    unit: &Unit,
    visible: bool,
) -> Result<bool, Error<()>> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("forest removal binding"));
    }
    if unit.i_build_charges <= 0 || !c6_core::skip_turn::legality::can_move(unit) {
        return Ok(false);
    }
    if unit.e_type_index != linux_type_hash(b"UNIT_BUILDER") {
        return Err(Error::Unsupported("ordinary Builder forest removal"));
    }
    let (index, plot) = c6_core::movement_world::plot_at(state.world().map(), unit.k_location)
        .map_err(|_| Error::Missing("forest removal plot"))?;
    if crate::movement_placement_state::plot_owner(plot)
        .map_err(|_| Error::Missing("forest plot owner"))?
        != unit.e_owner
    {
        return Ok(false);
    }
    // Handler's visible-only return precedes terrain builder feature checks.
    if visible {
        return Ok(true);
    }
    if plot.improvement_hash != u32::MAX
        || plot
            .city_attributes
            .as_ref()
            .is_some_and(|a| a.district != u32::MAX)
    {
        return Ok(false);
    }
    if plot.feature_hash == u32::MAX {
        return Ok(false);
    }
    if plot.feature_hash != defs.feature_hash {
        return Err(Error::Unsupported("ordinary forest removal feature"));
    }
    if inputs
        .national_parks
        .iter()
        .any(|p| p.plot_indices.contains(&(index as i32)))
    {
        return Ok(false);
    }
    if *inputs
        .fallout_turns
        .get(index)
        .ok_or(Error::Missing("forest current Fallout"))?
        > 0
    {
        return Err(Error::Unsupported("forest fallout work permission"));
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == unit.e_owner)
        .ok_or(Error::Missing("forest player"))?;
    if let Some((hash, _)) = defs.remove_tech {
        if !player
            .techs
            .has_tech
            .pairs
            .iter()
            .rfind(|r| r.definition_hash == hash)
            .ok_or(Error::Missing("forest remove technology flag"))?
            .value
        {
            return Ok(false);
        }
    }
    let alive = player
        .tail
        .as_ref()
        .and_then(|t| t.alive.as_ref())
        .ok_or(Error::Missing("forest current chop behavior"))?;
    if alive
        .chop_behaviors
        .iter()
        .any(|b| b.feature_wire == defs.feature_index && b.ban)
    {
        return Ok(false);
    }
    Ok(true)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    Observer {
        x: i32,
        y: i32,
    },
    Operation(ComponentId),
    Visible(ComponentId),
    Plot {
        index: usize,
    },
    Charges {
        unit: ComponentId,
        before: i32,
        after: i32,
    },
    LastCharge(ComponentId),
}
pub trait Effects {
    type Error;
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    fn local_plot_visible(
        &self,
        state: &LoadedState,
        actor: UnitReference,
    ) -> Result<bool, Self::Error>;
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Self::Error>;
    /// Full SetFeatureType(NO_FEATURE), including terrain effects and reports.
    fn remove_feature(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Self::Error>;
    fn feature_removed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        feature: i32,
    ) -> Result<(), Self::Error>;
    /// Actual ObtainYield resolves its city and applies production synchronously.
    fn obtain_yield(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        feature: i32,
        base: i32,
    ) -> Result<i32, Self::Error>;
    fn harvest_bonuses(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        feature: i32,
        total: i32,
    ) -> Result<(), Self::Error>;
    fn delayed_destroy(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Self::Error>;
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
        || node.parameters != OperationParameters::None
    {
        return Err(Error::Unsupported("forest removal request/binding"));
    }
    let u = state
        .world()
        .unit(actor)
        .map_err(|_| Error::Missing("forest actor"))?
        .ok_or(Error::Missing("forest actor"))?
        .record;
    if u.e_type_index != linux_type_hash(b"UNIT_BUILDER") {
        return Err(Error::Unsupported(
            "forest actor changed from ordinary Builder",
        ));
    }
    let location = u.k_location;
    let owner = u.e_owner;
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
        .map_err(|_| Error::Missing("forest current plot"))?;
    if p.feature_hash == u32::MAX {
        return Ok(StartData {
            delete: false,
            action: false,
        });
    }
    if p.feature_hash != defs.feature_hash {
        return Err(Error::Unsupported("forest current feature"));
    }
    let id = ComponentId {
        owner: owner as i16,
        component_type: 1,
        id: actor.id,
    };
    // Native samples visibility before the operation report.
    let visible = effects
        .local_plot_visible(state, actor)
        .map_err(Error::Effect)?;
    effects
        .report(state, Report::Operation(id))
        .map_err(Error::Effect)?;
    if visible {
        effects
            .report(state, Report::Visible(id))
            .map_err(Error::Effect)?;
    }
    effects
        .report(state, Report::Plot { index: plot })
        .map_err(Error::Effect)?;
    effects.remove_feature(state, plot).map_err(Error::Effect)?;
    effects
        .feature_removed(state, plot, defs.feature_index)
        .map_err(Error::Effect)?;
    let added = state
        .world()
        .map()
        .plots
        .get(plot)
        .ok_or(Error::Missing("forest removed plot"))?
        .flags
        .get(PlotFlag::FeatureAdded);
    let total = if added {
        0
    } else {
        effects
            .obtain_yield(state, actor, defs.feature_index, defs.production)
            .map_err(Error::Effect)?
    };
    effects
        .harvest_bonuses(state, actor, defs.feature_index, total)
        .map_err(Error::Effect)?;
    let unit = state
        .unit_mut(actor.player(), actor.id)
        .map_err(Error::State)?;
    let before = unit.i_build_charges;
    unit.i_build_charges = before.wrapping_sub(1).max(0);
    let after = unit.i_build_charges;
    effects
        .report(
            state,
            Report::Charges {
                unit: id,
                before,
                after,
            },
        )
        .map_err(Error::Effect)?;
    if after < 1 {
        effects
            .report(state, Report::LastCharge(id))
            .map_err(Error::Effect)?;
        effects
            .delayed_destroy(state, actor)
            .map_err(Error::Effect)?;
    } else {
        effects.finish_moves(state, actor).map_err(Error::Effect)?;
    }
    Ok(StartData {
        delete: false,
        action: true,
    })
}
