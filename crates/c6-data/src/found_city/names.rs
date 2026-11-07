//! Current Rules capital names and real canonical used-name mutation.
use crate::{
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::Definitions as Initialization, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    found_city::names::{self, CapitalRow},
    loaded_state::LoadedState,
};
#[derive(Debug)]
pub enum Error {
    Resources(crate::unit_initialization_definitions::Error),
    Rules(crate::Error),
    State(c6_core::loaded_state::Error),
    Binding,
    Missing(&'static str),
    Scalar(&'static str),
    Naming(names::Error),
    FallbackRequired,
    ContinentNamesUnsupported,
    DuplicateCandidates,
    Random(c6_core::loaded_state::game_sync::AccessError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FoundCity naming: {self:?}")
    }
}
impl std::error::Error for Error {}
pub struct Definitions {
    binding: WireBinding,
    capitals: Vec<CapitalRow>,
    ordinary: Vec<(Vec<u8>, Option<Vec<u8>>, Vec<u8>, i64)>,
    depths: Vec<(Vec<u8>, i32)>,
    continent_names: bool,
}
fn bytes(value: &SqlValue, field: &'static str) -> Result<Vec<u8>, Error> {
    match value {
        SqlValue::Text(s) => Ok(s.split(|c| *c == 0).next().unwrap_or(&[]).to_vec()),
        SqlValue::Null => Ok(vec![]),
        _ => Err(Error::Scalar(field)),
    }
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        Initialization::validate_resources(rules, bound).map_err(Error::Resources)?;
        let table = rules
            .gameplay()
            .table("CivilizationLeaders")
            .ok_or(Error::Missing("CivilizationLeaders"))?;
        let capitals = table
            .rows()
            .map(|row| {
                Ok(CapitalRow {
                    civilization: bytes(
                        row.get("CivilizationType").map_err(Error::Rules)?,
                        "CivilizationType",
                    )?,
                    leader: bytes(row.get("LeaderType").map_err(Error::Rules)?, "LeaderType")?,
                    capital: bytes(row.get("CapitalName").map_err(Error::Rules)?, "CapitalName")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let table = rules
            .gameplay()
            .table("CityNames")
            .ok_or(Error::Missing("CityNames"))?;
        let mut ordinary = Vec::new();
        let mut continent_names = false;
        for row in table.rows() {
            continent_names |= !matches!(
                row.get("ContinentType").map_err(Error::Rules)?,
                SqlValue::Null
            );
            ordinary.push((
                bytes(
                    row.get("CivilizationType").map_err(Error::Rules)?,
                    "CivilizationType",
                )?,
                match row.get("LeaderType").map_err(Error::Rules)? {
                    SqlValue::Null => None,
                    value => Some(bytes(value, "LeaderType")?),
                },
                bytes(row.get("CityName").map_err(Error::Rules)?, "CityName")?,
                row.integer("SortIndex").map_err(Error::Rules)?,
            ));
        }
        // SQLite's ORDER BY SortIndex retains the rowid scan order for ties
        // in this admitted, no-continent stock query profile.
        ordinary.sort_by_key(|row| row.3);
        let depths = rules
            .gameplay()
            .table("Civilizations")
            .ok_or(Error::Missing("Civilizations"))?
            .rows()
            .map(|row| {
                Ok((
                    bytes(
                        row.get("CivilizationType").map_err(Error::Rules)?,
                        "CivilizationType",
                    )?,
                    i32::try_from(row.integer("RandomCityNameDepth").map_err(Error::Rules)?)
                        .map_err(|_| Error::Scalar("RandomCityNameDepth"))?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            capitals,
            ordinary,
            depths,
            continent_names,
        })
    }
    /// GenerateCityName29ffe40's ordinary stock query path when CityNames has
    /// no continent-specific rows. Keep the current GameSync draw and used-name
    /// owner; unsupported fallback/duplicate profiles remain explicit.
    pub fn generate_ordinary(
        &self,
        state: &mut LoadedState,
        civilization: &[u8],
        leader: &[u8],
    ) -> Result<Vec<u8>, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        if self.continent_names {
            return Err(Error::ContinentNamesUnsupported);
        }
        let owner = state.game_city_names().map_err(Error::State)?;
        let mut candidates = Vec::new();
        for (civ, row_leader, name, _) in &self.ordinary {
            if civ != civilization || row_leader.as_ref().is_some_and(|value| value != leader) {
                continue;
            }
            // The positive native UnsortedUnique branch needs its own ordering
            // qualification; never silently choose an invented duplicate order.
            if candidates.contains(name) {
                return Err(Error::DuplicateCandidates);
            }
            candidates.push(name.clone());
        }
        candidates.retain(|name| !names::was_used(owner, name));
        let depth = self
            .depths
            .iter()
            .find(|(civ, _)| civ == civilization)
            .map_or(10, |(_, depth)| *depth);
        let name = names::weighted_name(&mut candidates, depth, |bound, _| {
            state.game_sync_rand(bound).map_err(Error::Random)
        })?
        .ok_or(Error::FallbackRequired)?;
        names::mark_used(state.game_city_names_mut().map_err(Error::State)?, &name)
            .map_err(Error::Naming)?;
        Ok(name)
    }
    /// Source capital branch only; caller must execute the actual general-name
    /// fallback when None. It must not substitute a guessed name or new seed.
    pub fn capital(
        &self,
        state: &LoadedState,
        first_city: bool,
        civilization: &[u8],
        leader: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        names::capital(
            state.game_city_names().map_err(Error::State)?,
            first_city,
            civilization,
            leader,
            &self.capitals,
        )
        .map_err(Error::Naming)
    }
    /// GenerateCityName's successful first-capital branch marks the selected
    /// name before City.SetName/placement. This is a real current-state edit.
    pub fn generate_first_capital(
        &self,
        state: &mut LoadedState,
        civilization: &[u8],
        leader: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let name = self
            .capital(state, true, civilization, leader)?
            .ok_or(Error::FallbackRequired)?;
        names::mark_used(state.game_city_names_mut().map_err(Error::State)?, &name)
            .map_err(Error::Naming)?;
        Ok(name)
    }
}
