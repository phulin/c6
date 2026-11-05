//! PlotPurchased Boost.Manager lambda34 calls OnImprovementConstructed for
//! each ordered Boost definition, using the newly annexed plot's current owner.
pub use crate::melee_attack::resolve::completion::boosts::{ResearchTrigger, Trigger};
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{loaded_state::LoadedState, map::PlotFlag};
struct Rule {
    id: i32,
    index: usize,
    improvement: Option<u32>,
    resource: Option<u32>,
    value: i32,
    requires_resource: bool,
    count: i32,
}
pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    rows: Vec<Rule>,
    resources: crate::city_turn::resources::Definitions,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|e| format!("{e:?}"))?;
        let resources =
            crate::unit_initialization_definitions::collection(rules, "Resources", "ResourceType")
                .map_err(|e| format!("{e:?}"))?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or_else(|| format!("purchase boosts {name}"))
        };
        let names = table("BoostNames")?;
        let mut rows = Vec::new();
        for (index, row) in table("Boosts")?.rows().enumerate() {
            let reference = |column,
                             registry: &crate::registry::LinuxDefinitionRegistry|
             -> Result<Option<u32>, String> {
                match row.get(column).map_err(|e| format!("{e:?}"))? {
                    SqlValue::Null => Ok(None),
                    SqlValue::Text(v) => Ok(registry
                        .find_index(linux_type_hash(v))
                        .map(|_| linux_type_hash(v))),
                    _ => Err("purchase boost reference".into()),
                }
            };
            let class = row.text("BoostClass").map_err(|e| format!("{e:?}"))?;
            let value = names
                .rows()
                .find(|n| n.text("BoostType").ok() == Some(class))
                .map(|n| n.integer("BoostValue"))
                .transpose()
                .map_err(|e| format!("{e:?}"))?
                .unwrap_or(-1) as i32;
            rows.push(Rule {
                id: row.integer("BoostID").map_err(|e| format!("{e:?}"))? as i32,
                index,
                improvement: reference("ImprovementType", &improvements)?,
                resource: reference("ResourceType", &resources)?,
                value,
                requires_resource: row
                    .integer("RequiresResource")
                    .map_err(|e| format!("{e:?}"))?
                    != 0,
                count: row.integer("NumItems").map_err(|e| format!("{e:?}"))? as i32,
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            rows,
            resources: crate::city_turn::resources::Definitions::from_rules(rules)
                .map_err(|e| format!("{e:?}"))?,
        })
    }
    /// Execute one observed native subscription. A positive predicate invokes the
    /// existing canonical research manager through a mandatory caller-owned trigger.
    pub fn purchased<T: Trigger>(
        &self,
        state: &mut LoadedState,
        plot: u32,
        boost_id: i32,
        trigger: &mut T,
    ) -> Result<(), String>
    where
        T::Error: std::fmt::Debug,
    {
        if state.binding() != &self.binding {
            return Err("purchase boost binding".into());
        }
        let rule = self
            .rows
            .iter()
            .find(|r| r.id == boost_id)
            .ok_or("purchase Boost definition ID")?;
        let Some(improvement) = rule.improvement else {
            return Ok(());
        };
        let p = state
            .world()
            .map()
            .plots
            .get(plot as usize)
            .ok_or("purchase Boost plot")?;
        let owner = p
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        if owner == -1 {
            return Ok(());
        }
        // This source query precedes the improvement comparison, including NO_TYPE.
        let revealed = self
            .resources
            .visible_resource(state, owner, p.resource_hash as i32)
            .map_err(|e| format!("purchase Boost resource visibility: {e:?}"))?;
        if p.improvement_hash != improvement {
            return Ok(());
        }
        let mut count = 0i32;
        for p in &state.world().map().plots {
            if p.city_attributes
                .as_ref()
                .is_some_and(|a| i32::from(a.player) == owner)
                && p.improvement_hash == improvement
                && !p.flags.get(PlotFlag::ImprovementPillaged)
            {
                count = count.wrapping_add(1);
            }
        }
        if count < rule.count {
            return Ok(());
        }
        if (rule.value == 21 && (!rule.requires_resource || revealed != -1))
            || (rule.value == 11 && rule.resource.is_some_and(|r| r as i32 == revealed))
        {
            trigger
                .trigger(state, owner, rule.index)
                .map_err(|e| format!("purchase TriggerBoost: {e:?}"))?;
        }
        Ok(())
    }
}
