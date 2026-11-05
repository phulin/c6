//! Ordinary Linux XP2 Builder Farm and Mine operations, over the ongoing loaded state.
//!
//! The operation body is separate from legality, queue processing and stock
//! subscriber delivery. A returned StartData is meaningful only after every
//! required current-state service has completed; this module alone does not
//! establish native action parity or support arbitrary improvements.
use crate::{registry::linux_type_hash, Rules, SqlValue};
pub mod legality;
pub mod placement;
pub mod services;
pub mod workers;
pub mod stock;

use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::UnitReference,
    skip_turn::StartData,
    unit::{ComponentId, OperationNode, OperationParameters, PlotCoord, BUILD_IMPROVEMENT_HASH},
};

#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    Unsupported(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Effect { stage: &'static str, error: E },
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Builder Farm: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind { Farm, Mine }
impl Kind {
    pub fn type_name(self) -> &'static str {
        match self { Self::Farm => "IMPROVEMENT_FARM", Self::Mine => "IMPROVEMENT_MINE" }
    }
}
/// Immutable source inputs, prepared once for the admitted rules identity.
pub struct Definitions {
    binding: WireBinding,
    pub improvement_hash: u32,
    pub improvement_index: i32,
    builder_hash: u32,
    pub kind: Kind,
    prerequisite_technology: Option<(u32, i32)>,
    prerequisite_civic: Option<(u32, i32)>,
    legality: legality::Static,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error<crate::Error>> {
        Self::for_kind(state, rules, Kind::Farm)
    }
    pub fn for_kind(state: &LoadedState, rules: &Rules, kind: Kind) -> Result<Self, Error<crate::Error>> {
        if !matches!(
            rules.profile(),
            crate::RulesProfile::LinuxXp2Base | crate::RulesProfile::LinuxXp2ActivatedDiagnostic
        ) {
            return Err(Error::Unsupported("Linux Farm rules"));
        }
        let name = kind.type_name();
        let improvement_hash = linux_type_hash(name.as_bytes());
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Error::Missing("Farm collection"))?;
        let index = improvements
            .find_index(improvement_hash)
            .ok_or(Error::Missing("Farm definition"))?;
        let row = rules
            .gameplay()
            .table("Improvements")
            .and_then(|t| t.row_by_id(improvements.entries()[index].database_rowid))
            .ok_or(Error::Missing("Farm row"))?;
        for (field, expected) in [
            ("Buildable", 1),
            ("BarbarianCamp", 0),
            ("Goody", 0),
            ("GrantFortification", 0),
        ] {
            if row.integer(field).map_err(|error| Error::Effect {
                stage: field,
                error,
            })? != expected
            {
                return Err(Error::Unsupported("ordinary Farm definition"));
            }
        }
        for field in ["TraitType"] {
            if !matches!(
                row.get(field).map_err(|error| Error::Effect {
                    stage: field,
                    error
                })?,
                SqlValue::Null
            ) {
                return Err(Error::Unsupported("Farm prerequisite/trait"));
            }
        }
        let prerequisite = |key| match row.get(key).map_err(|error|Error::Effect {stage:key,error})? {
            SqlValue::Null => Ok(None), SqlValue::Text(value) => {
                let (table, column) = if key=="PrereqTech" {("Technologies","TechnologyType")} else {("Civics","CivicType")};
                let registry=crate::unit_initialization_definitions::collection(rules,table,column)
                    .map_err(|_|Error::Missing("improvement prerequisite definitions"))?;
                let hash=linux_type_hash(value);
                let index=registry.find_index(hash).ok_or(Error::Missing("improvement prerequisite definition"))?;
                Ok(Some((hash,index as i32)))
            },
            _ => Err(Error::Unsupported("ordinary improvement prerequisite reference")),
        };
        let prerequisite_technology = prerequisite("PrereqTech")?;
        let prerequisite_civic = prerequisite("PrereqCivic")?;
        let builder_row = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Builder Units table"))?
            .rows()
            .find(|r| r.text("UnitType").ok() == Some("UNIT_BUILDER"))
            .ok_or(Error::Missing("Builder definition"))?;
        for field in ["Combat", "RangedCombat", "Bombard"] {
            if builder_row.integer(field).map_err(|error| Error::Effect {
                stage: field,
                error,
            })? != 0
            {
                return Err(Error::Unsupported("noncombat ordinary Builder"));
            }
        }
        let build = rules
            .gameplay()
            .table("Improvement_ValidBuildUnits")
            .ok_or(Error::Missing("Improvement_ValidBuildUnits"))?;
        let mut builder = build.rows().filter(|r| {
            r.text("ImprovementType").ok() == Some(name)
                && r.text("UnitType").ok() == Some("UNIT_BUILDER")
        });
        let row = builder
            .next()
            .ok_or(Error::Missing("Farm Builder construction"))?;
        if builder.next().is_some()
            || row
                .integer("ConsumesCharge")
                .map_err(|error| Error::Effect {
                    stage: "Farm ConsumesCharge",
                    error,
                })?
                != 1
        {
            return Err(Error::Unsupported("Farm charge rule"));
        }
        Ok(Self {
            binding: state.binding().clone(), kind, prerequisite_technology, prerequisite_civic,
            improvement_hash,
            improvement_index: index as i32,
            builder_hash: linux_type_hash(b"UNIT_BUILDER"),
            legality: legality::Static::from_rules(rules, kind)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    BuildStarted(PlotCoord),
    Operation {
        unit: ComponentId,
        improvement: i32,
    },
    VisibleUnit(ComponentId),
    CityYieldChanged(ComponentId),
    ChargesChanged {
        unit: ComponentId,
        current: i32,
        previous: i32,
    },
    LastCharge(ComponentId),
}
/// Reached source services over the same state, Script/RNG and mutable owners.
/// There are no successful default methods or supplied callback Test answers.
pub trait Effects {
    type Error;
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    /// Ordinary SetImprovementType, including owner sight, constructed listeners,
    /// revealed-improvement knowledge and Economic.CalculateMonopolies.
    fn set_improvement(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        definition: &Definitions,
        owner: i32,
    ) -> Result<(), Self::Error>;
    /// Read the current Culture.IsCultureBombTrigger and execute it when reached.
    fn culture_bomb(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        improvement: i32,
    ) -> Result<(), Self::Error>;
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Self::Error>;
    fn local_unit_visible(
        &self,
        state: &LoadedState,
        actor: UnitReference,
    ) -> Result<bool, Self::Error>;
    /// Uses current CityAttributes owning city; absent cities return None.
    /// A present city must complete the existing citizen reassignment routine.
    fn reallocate_owning_city(
        &mut self,
        state: &mut LoadedState,
        location: PlotCoord,
    ) -> Result<Option<ComponentId>, Self::Error>;
    fn finish_moves(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Self::Error>;
    fn delayed_destroy(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
    ) -> Result<(), Self::Error>;
}
fn identity(
    state: &LoadedState,
    actor: UnitReference,
) -> Result<ComponentId, Error<std::convert::Infallible>> {
    let u = state
        .world()
        .unit(actor)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Farm Builder"))?
        .record;
    Ok(ComponentId {
        owner: u.e_owner as i16,
        component_type: 1,
        id: u.id,
    })
}
/// XP2 BuildImprovement.Start 0x2dc32f0 for the ordinary Builder/Farm pair.
/// The caller owns legal Add/activation/completion and must retain any failed
/// prefix. No state is cloned, reimported or rolled back across callbacks.
pub fn start<E: Effects>(
    state: &mut LoadedState,
    effects: &mut E,
    definitions: &Definitions,
    actor: UnitReference,
    node: &OperationNode,
) -> Result<StartData, Error<E::Error>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Unsupported("Farm binding"));
    }
    if node.operation_hash != BUILD_IMPROVEMENT_HASH
        || !matches!(node.parameters,
        OperationParameters::BuildImprovement { improvement_hash, .. } if improvement_hash == definitions.improvement_hash)
    {
        return Err(Error::Unsupported("ordinary Farm request"));
    }
    let u = state
        .world()
        .unit(actor)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Farm Builder"))?
        .record;
    if u.e_type_index != definitions.builder_hash {
        return Err(Error::Unsupported("ordinary Builder type"));
    }
    let location = u.k_location;
    let owner = u.e_owner;
    let plot = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Missing("Farm plot"))?
        .0;
    let source = |error| Error::Effect {
        stage: "Farm initial reporting",
        error,
    };
    if effects
        .observer_can_see_owner(state, owner)
        .map_err(source)?
    {
        effects
            .report(state, Report::BuildStarted(location))
            .map_err(source)?;
    }
    effects
        .set_improvement(state, plot, definitions, owner)
        .map_err(|error| Error::Effect {
            stage: "SetImprovementType",
            error,
        })?;
    effects
        .culture_bomb(state, actor, definitions.improvement_index)
        .map_err(|error| Error::Effect {
            stage: "Farm culture bomb",
            error,
        })?;
    let unit_id = |state: &LoadedState| {
        identity(state, actor).map_err(|e| match e {
            Error::World(e) => Error::World(e),
            _ => Error::Missing("Farm Builder identity"),
        })
    };
    effects
        .report(
            state,
            Report::Operation {
                unit: unit_id(state)?,
                improvement: definitions.improvement_index,
            },
        )
        .map_err(|error| Error::Effect {
            stage: "Farm operation report",
            error,
        })?;
    if effects
        .local_unit_visible(state, actor)
        .map_err(|error| Error::Effect {
            stage: "Farm local visibility",
            error,
        })?
    {
        effects
            .report(state, Report::VisibleUnit(unit_id(state)?))
            .map_err(|error| Error::Effect {
                stage: "Farm visible report",
                error,
            })?;
    }
    // Source's visible-resource getter is pure and its return is unused.
    let location = state
        .world()
        .unit(actor)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Farm Builder"))?
        .record
        .k_location;
    if let Some(city) = effects
        .reallocate_owning_city(state, location)
        .map_err(|error| Error::Effect {
            stage: "Farm ReallocateCitizens",
            error,
        })?
    {
        effects
            .report(state, Report::CityYieldChanged(city))
            .map_err(|error| Error::Effect {
                stage: "Farm city report",
                error,
            })?;
    }
    let unit = state
        .unit_mut(actor.player(), actor.id)
        .map_err(Error::State)?;
    let previous = unit.i_build_charges;
    unit.i_build_charges = previous.wrapping_sub(1).max(0);
    let current = unit.i_build_charges;
    effects
        .report(
            state,
            Report::ChargesChanged {
                unit: unit_id(state)?,
                current,
                previous,
            },
        )
        .map_err(|error| Error::Effect {
            stage: "Farm charge report",
            error,
        })?;
    let current = state
        .world()
        .unit(actor)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Farm Builder"))?
        .record
        .i_build_charges;
    if state
        .world()
        .unit(actor)
        .map_err(Error::World)?
        .ok_or(Error::Missing("Farm Builder"))?
        .record
        .e_type_index
        != definitions.builder_hash
    {
        return Err(Error::Unsupported(
            "Farm callback changed Builder definition",
        ));
    }
    if current > 0 {
        effects
            .finish_moves(state, actor)
            .map_err(|error| Error::Effect {
                stage: "Farm FinishMoves",
                error,
            })?;
    } else {
        effects
            .report(state, Report::LastCharge(unit_id(state)?))
            .map_err(|error| Error::Effect {
                stage: "Farm last charge report",
                error,
            })?;
        effects
            .delayed_destroy(state, actor)
            .map_err(|error| Error::Effect {
                stage: "Farm DelayedDestroy",
                error,
            })?;
    }
    Ok(StartData {
        delete: true,
        action: true,
    })
}
