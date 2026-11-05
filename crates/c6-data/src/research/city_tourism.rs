//! City.Culture.RecordTourism current zero-source branch, in native root order.
//! Actual positive tourism is never inferred absent from a City count or name.
use super::*;
use c6_core::yields::{PrerequisiteKind, PrerequisiteState};
pub(super) struct Definitions {
    holy_districts: Vec<u32>,
    wonders: Vec<u32>,
    improvements: Vec<(u32, Option<i32>, Option<i32>)>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error<std::convert::Infallible>> {
        let mut holy_districts = vec![];
        let mut wonders = vec![];
        let mut improvements = vec![];
        for (table, column, flag, output) in [
            (
                "Districts",
                "DistrictType",
                "AllowsHolyCity",
                &mut holy_districts,
            ),
            ("Buildings", "BuildingType", "IsWonder", &mut wonders),
        ] {
            for row in rules
                .gameplay()
                .table(table)
                .ok_or(Error::Missing("City tourism definitions"))?
                .rows()
            {
                if scalar(row, flag)? != 0 {
                    output.push(linux_type_hash(
                        row.text(column).map_err(Error::Rules)?.as_bytes(),
                    ));
                }
            }
        }
        let technologies =
            collection(rules, "Technologies", "TechnologyType").map_err(Error::Definitions)?;
        let civics = collection(rules, "Civics", "CivicType").map_err(Error::Definitions)?;
        for row in rules
            .gameplay()
            .table("Improvement_Tourism")
            .ok_or(Error::Missing("Improvement_Tourism"))?
            .rows()
        {
            let reference = |name,
                             registry: &LinuxDefinitionRegistry|
             -> Result<Option<i32>, Error<std::convert::Infallible>> {
                match row.get(name).map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    SqlValue::Text(name) => registry
                        .find_index(linux_type_hash(name))
                        .map(|i| Some(i as i32))
                        .ok_or(Error::Missing(
                            "improvement tourism prerequisite definition",
                        )),
                    _ => Err(Error::Missing("improvement tourism prerequisite")),
                }
            };
            improvements.push((
                linux_type_hash(
                    row.text("ImprovementType")
                        .map_err(Error::Rules)?
                        .as_bytes(),
                ),
                reference("PrereqCivic", &civics)?,
                reference("PrereqTech", &technologies)?,
            ));
        }
        Ok(Self {
            holy_districts,
            wonders,
            improvements,
        })
    }
    pub fn record<E: Effects>(
        &self,
        context: &mut Context<'_, E>,
        owner: i32,
        city: u32,
    ) -> Result<(), Error<E::Error>> {
        let player = context
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("City tourism player"))?;
        let city = player
            .cities
            .records
            .entries
            .iter()
            .find(|p| p.value.header.id == city)
            .map(|p| &p.value)
            .ok_or(Error::Missing("City tourism current City"))?;
        // District source order is the City's actual ID list, with missing
        // current district pointers skipped before any getters.
        for &id in &city.districts.district_ids {
            let Some(district) = player
                .districts
                .records
                .entries
                .iter()
                .find(|d| d.value.id == id)
                .map(|d| &d.value)
            else {
                continue;
            };
            if district.tourism_change > 0 {
                return Err(Error::Missing("positive district tourism contribution"));
            }
            if self.holy_districts.contains(&district.district_hash) {
                return Err(Error::Missing("current holy-city tourism contribution"));
            }
        }
        // The target iterates CityHexspace and filters owning City first. With
        // no owned eligible improvement, no AddTourismAtPlot or RNG is reached.
        for plot in &context.state.world().map().plots {
            if !plot
                .city_attributes
                .as_ref()
                .is_some_and(|a| i32::from(a.player) == owner && a.owning_city == city.header.id)
            {
                continue;
            }
            let Some((_, civic, tech)) = self
                .improvements
                .iter()
                .find(|r| r.0 == plot.improvement_hash)
            else {
                continue;
            };
            let acquired = |kind, index: i32| -> Result<bool, Error<E::Error>> {
                let mask = context
                    .state
                    .world()
                    .acquired_mask(owner, kind)
                    .map_err(|_| Error::Missing("City tourism acquired mask"))?;
                if !mask
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::Missing("City tourism prerequisite index"))?
                {
                    return Ok(false);
                }
                context
                    .state
                    .world()
                    .globally_available(kind, index)
                    .map_err(|_| Error::Missing("City tourism active prerequisite"))
            };
            if let Some(index) = civic {
                if !acquired(PrerequisiteKind::Civic, *index)? {
                    continue;
                }
            }
            if let Some(index) = tech {
                if !acquired(PrerequisiteKind::Technology, *index)? {
                    continue;
                }
            }
            return Err(Error::Missing("positive improvement tourism contribution"));
        }
        if city.culture.tourism_per_feature > 0 {
            return Err(Error::Missing("positive City feature tourism"));
        }
        if !city.buildings.great_work_buildings.is_empty() {
            return Err(Error::Missing("current GreatWork tourism contribution"));
        }
        if city
            .buildings
            .building_locations
            .iter()
            .any(|b| b.value != -1 && self.wonders.contains(&b.definition_hash))
        {
            return Err(Error::Missing("current Wonder tourism contribution"));
        }
        Ok(())
    }
}
