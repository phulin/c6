//! Acquisition unit cost inputs from admitted Linux rules and authoritative
//! retained arrays. Current research completion is supplied from the live graph;
//! SQL technology totals are not substituted for runtime graph counts.
use crate::{
    registry::linux_type_hash,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Row, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
    research_acquisition::units::{self, CostArrays, Definition, Economic, ResearchCompletion},
};

#[derive(Debug)]
pub enum Error {
    Rules(crate::Error),
    Definitions(unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    Costs(units::Error<c6_core::loaded_state::Error>),
    Unsupported(&'static str),
}
pub struct Definitions {
    binding: WireBinding,
    units: Vec<Definition>,
    economic: Economic,
}
fn integer(row: Row<'_>, column: &'static str) -> Result<i32, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Integer(value) => i32::try_from(*value).map_err(|_| Error::Unsupported(column)),
        _ => Err(Error::Unsupported(column)),
    }
}
fn optional_hash(row: Row<'_>, column: &'static str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(value) if !value.contains(&0) => Ok(Some(linux_type_hash(value))),
        _ => Err(Error::Unsupported(column)),
    }
}
impl Definitions {
    pub fn units(&self) -> &[Definition] {
        &self.units
    }
    pub fn economic(&self) -> Economic {
        self.economic
    }
    pub fn from_rules(
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
        game_speed_type: u32,
    ) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Definitions)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Definitions)?;
        let native = bound
            .unit_definitions()
            .map_err(|_| Error::Unsupported("bound native Units"))?;
        if units.count() != native.lookup.count()
            || (0..units.count()).any(|i| units.type_hash(i) != native.lookup.type_hash(i))
        {
            return Err(Error::Unsupported("native Units definition order"));
        }
        let xp2 = collection(rules, "Units_XP2", "UnitType").map_err(Error::Definitions)?;
        let resources =
            collection(rules, "Resources", "ResourceType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("Units"))?;
        let xp2_table = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or(Error::Unsupported("Units_XP2"))?;
        let mut definitions = Vec::with_capacity(units.count());
        for entry in units.entries() {
            let hash = linux_type_hash(&entry.type_name);
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Unsupported("Units row"))?;
            let resource_cost = xp2
                .find(hash)
                .map(|entry| {
                    let row = xp2_table
                        .row_by_id(entry.database_rowid)
                        .ok_or(Error::Unsupported("Units_XP2 row"))?;
                    integer(row, "ResourceCost")
                })
                .transpose()?;
            definitions.push(Definition {
                hash,
                cost: integer(row, "Cost")?,
                progression_model_hash: optional_hash(row, "CostProgressionModel")?.unwrap_or(0),
                progression_param: integer(row, "CostProgressionParam1")?,
                strategic_resource_present: optional_hash(row, "StrategicResource")?
                    .is_some_and(|hash| resources.find(hash).is_some()),
                resource_cost,
            });
        }
        let speeds =
            collection(rules, "GameSpeeds", "GameSpeedType").map_err(Error::Definitions)?;
        let speed = if game_speed_type as usize >= speeds.count() && game_speed_type <= 0x7ff {
            None
        } else {
            speeds.find(game_speed_type)
        };
        let speed_cost_multiplier = match speed {
            None => 100,
            Some(entry) => integer(
                rules
                    .gameplay()
                    .table("GameSpeeds")
                    .and_then(|table| table.row_by_id(entry.database_rowid))
                    .ok_or(Error::Unsupported("GameSpeeds row"))?,
                "CostMultiplier",
            )?,
        };
        // Require the admitted row rather than inventing an initializer fallback.
        let mut escalation = None;
        for row in rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Unsupported("GlobalParameters"))?
            .rows()
        {
            if row.text("Name").map_err(Error::Rules)? == "GAME_COST_ESCALATION" {
                if escalation.is_some() {
                    return Err(Error::Unsupported("duplicate GAME_COST_ESCALATION"));
                }
                escalation = Some(match row.get("Value").map_err(Error::Rules)? {
                    SqlValue::Integer(v) => {
                        i32::try_from(*v).map_err(|_| Error::Unsupported("GAME_COST_ESCALATION"))?
                    }
                    SqlValue::Text(v) => std::str::from_utf8(v)
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .ok_or(Error::Unsupported("GAME_COST_ESCALATION"))?,
                    _ => return Err(Error::Unsupported("GAME_COST_ESCALATION")),
                });
            }
        }
        Ok(Self {
            binding: bound.destination_binding().clone(),
            units: definitions,
            economic: Economic {
                speed_cost_multiplier,
                game_cost_escalation: escalation
                    .ok_or(Error::Unsupported("GAME_COST_ESCALATION"))?,
            },
        })
    }

    /// Execute on real retained arrays. Computation validates all reached input
    /// dependencies before the revision-tracked mutable borrow is obtained.
    pub fn recompute_loaded(
        &self,
        state: &mut LoadedState,
        player: i32,
        research: ResearchCompletion,
    ) -> Result<(), Error> {
        if state.binding() != &self.binding {
            return Err(Error::Unsupported("unit cost rules binding"));
        }
        let retained = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == player)
            .ok_or(Error::Unsupported("retained unit cost owner"))?;
        let mut production = retained.units.current_unit_cost.clone();
        let mut resource = retained.units.current_unit_resource_cost.clone();
        units::recompute_all_costs(
            CostArrays {
                production: &mut production,
                resource: &mut resource,
            },
            &retained.stats.num_units_constructed,
            state
                .world()
                .player(player)
                .map_err(|_| Error::Unsupported("unit cost world owner"))?,
            &self.units,
            research,
            self.economic,
            |unit| {
                state
                    .unit_runtime(player, unit.id)
                    .map(|runtime| runtime.definition_index)
            },
        )
        .map_err(Error::Costs)?;
        let arrays = state.player_unit_costs_mut(player).map_err(Error::State)?;
        // Native performs production then resource writes for each definition.
        for definition in &self.units {
            let cost = production
                .iter()
                .rposition(|p| p.definition_hash == definition.hash)
                .expect("checked cost");
            let resource_index = resource
                .iter()
                .rposition(|p| p.definition_hash == definition.hash)
                .expect("checked resource cost");
            arrays.production[cost].value = production[cost].value;
            arrays.resource[resource_index].value = resource[resource_index].value;
        }
        Ok(())
    }
}
