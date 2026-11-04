//! Current Player.Resources visibility shared by city resource and tile queries.
use super::Error;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState,
    yields::{PrerequisiteKind, PrerequisiteState},
};
struct Resource {
    tech: Option<i32>,
    civic: Option<i32>,
    effect_unlock: bool,
}
pub struct Definitions {
    resources: LinuxDefinitionRegistry,
    values: Vec<Resource>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let resources =
            collection(rules, "Resources", "ResourceType").map_err(Error::Definitions)?;
        let techs =
            collection(rules, "Technologies", "TechnologyType").map_err(Error::Definitions)?;
        let civics = collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Resources")
            .ok_or(Error::Unsupported("Resources"))?;
        let conditions = rules
            .gameplay()
            .table("Resource_Conditions")
            .ok_or(Error::Unsupported("Resource_Conditions"))?;
        let mut values = Vec::new();
        for definition in resources.entries() {
            let row = table
                .row_by_id(definition.database_rowid)
                .ok_or(Error::Unsupported("Resource row"))?;
            let reference =
                |column, lookup: &LinuxDefinitionRegistry| -> Result<Option<i32>, Error> {
                    match row.get(column).map_err(Error::Rules)? {
                        SqlValue::Null => Ok(None),
                        SqlValue::Text(name) => {
                            Ok(lookup.find_index(linux_type_hash(name)).map(|i| i as i32))
                        }
                        _ => Err(Error::Unsupported(column)),
                    }
                };
            let mut effect_unlock = false;
            for condition in conditions.rows() {
                if condition
                    .text("ResourceType")
                    .map_err(Error::Rules)?
                    .as_bytes()
                    == definition.type_name
                {
                    effect_unlock |= crate::game_parameters::sql_integer(
                        condition.get("UnlocksFromEffect").map_err(Error::Rules)?,
                    )
                    .ok_or(Error::Unsupported("UnlocksFromEffect"))?
                        != 0;
                }
            }
            values.push(Resource {
                tech: reference("PrereqTech", &techs)?,
                civic: reference("PrereqCivic", &civics)?,
                effect_unlock,
            });
        }
        Ok(Self { resources, values })
    }
    pub fn visible_resource(
        &self,
        state: &LoadedState,
        owner: i32,
        resource: i32,
    ) -> Result<i32, Error> {
        Ok(if self.visible(state, owner, resource, &mut Vec::new())? {
            resource
        } else {
            -1
        })
    }
    fn visible(
        &self,
        state: &LoadedState,
        owner: i32,
        resource: i32,
        stack: &mut Vec<i32>,
    ) -> Result<bool, Error> {
        if resource == -1 {
            return Ok(false);
        }
        let Some(index) = self.resources.find_index(resource as u32) else {
            return Ok(false);
        };
        if stack.contains(&owner) {
            return Err(Error::Unsupported(
                "cyclic current suzerain resource visibility",
            ));
        }
        stack.push(owner);
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("current resource Player"))?;
        let free = player
            .resources
            .free_resource_visibility
            .iter()
            .rfind(|r| self.resources.find_index(r.resource_hash) == Some(index))
            .ok_or(Error::Unsupported("current free resource visibility"))?
            .value;
        if free {
            stack.pop();
            return Ok(true);
        }
        if player.influence.suzerain != -1
            && self.visible(state, player.influence.suzerain, resource, stack)?
        {
            stack.pop();
            return Ok(true);
        }
        let definition = &self.values[index];
        if definition.effect_unlock {
            stack.pop();
            return Ok(false);
        }
        for (kind, prerequisite) in [
            (PrerequisiteKind::Technology, definition.tech),
            (PrerequisiteKind::Civic, definition.civic),
        ] {
            let Some(index) = prerequisite else { continue };
            let mask = state
                .world()
                .acquired_mask(owner, kind)
                .map_err(|_| Error::Unsupported("current resource prerequisite flags"))?;
            if index as u32 as usize >= mask.len() {
                stack.pop();
                return Ok(false);
            }
            if !state
                .world()
                .globally_available(kind, index)
                .map_err(|_| Error::Unsupported("current resource prerequisite availability"))?
                || !mask[index as usize]
            {
                stack.pop();
                return Ok(false);
            }
        }
        stack.pop();
        Ok(true)
    }
}
