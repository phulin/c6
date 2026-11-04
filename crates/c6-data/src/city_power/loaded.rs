//! Canonical owned City.Power execution. The temporary projection is flushed
//! before synchronous effects and discarded after them; callback writes are
//! reread from the same LoadedState. This is not a replacement city/world.
use super::{Definitions, Error, Live};
use crate::movement_placement_state::CurrentCivilizationLevels;
use c6_core::movement_world::DefinitionLookup;
use c6_core::{
    city_power_activation::{Source, State},
    city_production::Order,
    city_services::RequiredPowerSource,
    loaded_state::LoadedState,
};

/// Current signal subscribers and shortage reporting are synchronous gameplay
/// effects. Implementations may compose the stock requirement/gossip kernels;
/// unknown dynamic subscribers must refuse rather than silently disappear.
pub trait Effects {
    fn power_changed(
        &mut self,
        state: &mut LoadedState,
        city: (i32, u32),
        first: bool,
    ) -> Result<(), Error>;
    fn shortage_changed(&mut self, state: &mut LoadedState, city: (i32, u32)) -> Result<(), Error>;
}
struct Projection {
    before: State,
    current: State,
}
struct Context<'a, E> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    city: (i32, u32),
    levels: CurrentCivilizationLevels<'a>,
    effects: &'a mut E,
    projection: Option<Projection>,
    locations: Vec<i16>,
    pillaged: Vec<bool>,
}
impl<E> Context<'_, E> {
    fn flush(&mut self) -> Result<(), Error> {
        let Some(p) = self.projection.take() else {
            return Ok(());
        };
        if p.current == p.before {
            return Ok(());
        }
        // Resolve all replacement source keys before exposing canonical mutation.
        let sources = if p.current.required_sources != p.before.required_sources {
            Some(
                self.definitions
                    .source_fields(&p.current.required_sources)?,
            )
        } else {
            None
        };
        let out = self
            .state
            .city_power_mut(self.city.0, self.city.1)
            .map_err(Error::State)?;
        if p.current.free_raw_q8 != p.before.free_raw_q8 {
            out.free_power_raw_q8 = p.current.free_raw_q8
        }
        if p.current.temporary_raw_q8 != p.before.temporary_raw_q8 {
            out.temporary_power_raw_q8 = p.current.temporary_raw_q8
        }
        if p.current.required_raw_q8 != p.before.required_raw_q8 {
            out.required_power_total_raw_q8 = p.current.required_raw_q8
        }
        if p.current.modifier_required_raw_q8 != p.before.modifier_required_raw_q8 {
            out.required_power_from_modifiers_raw_q8 = p.current.modifier_required_raw_q8
        }
        if p.current.modified_free_percent != p.before.modified_free_percent {
            out.modified_free_power = p.current.modified_free_percent
        }
        if p.current.ever_generated != p.before.ever_generated {
            out.ever_generated_power = p.current.ever_generated
        }
        if let Some(sources) = sources {
            out.required_power_sources = sources
        }
        Ok(())
    }
}
impl Definitions {
    fn source_fields(&self, sources: &[Source]) -> Result<Vec<RequiredPowerSource>, Error> {
        fn key(
            registry: &crate::registry::LinuxDefinitionRegistry,
            index: i32,
        ) -> Result<u32, Error> {
            if index == -1 {
                return Ok(u32::MAX);
            }
            let index = usize::try_from(index).map_err(|_| Error::Index)?;
            let key = registry.type_hash(index).ok_or(Error::Index)?;
            if registry.find_index(key) != Some(index) {
                return Err(Error::Missing("noninvertible runtime source index"));
            }
            Ok(key)
        }
        let mut result = Vec::new();
        result
            .try_reserve(sources.len())
            .map_err(Error::Allocation)?;
        for s in sources {
            result.push(RequiredPowerSource {
                amount_raw_q8: s.amount_raw_q8,
                building_source_hash: key(&self.buildings, s.building)?,
                project_source_hash: key(&self.projects, s.project)?,
            })
        }
        Ok(result)
    }
    fn building_fields(&self, city: &c6_core::city::City) -> Result<(Vec<i16>, Vec<bool>), Error> {
        // Buildings.Read calls Reset: actual Building-count locations=-1 and
        // pillaged=false, then applies ordered indexed pairs (last write wins).
        let mut locations = vec![-1; self.buildings.entries().len()];
        let mut pillaged = vec![false; self.buildings.entries().len()];
        for pair in &city.buildings.building_locations {
            if let Some(i) = self.buildings.find_index(pair.definition_hash) {
                locations[i] = pair.value
            }
        }
        for pair in &city.buildings.pillaged {
            if let Some(i) = self.buildings.find_index(pair.definition_hash) {
                pillaged[i] = pair.value
            }
        }
        Ok((locations, pillaged))
    }
}
impl<E: Effects> Live for Context<'_, E> {
    fn power(&mut self) -> Result<&mut State, Error> {
        if self.projection.is_none() {
            let p = self.definitions.project_fields(
                &self
                    .state
                    .city(self.city.0, self.city.1)
                    .map_err(Error::State)?
                    .power,
            )?;
            self.projection = Some(Projection {
                before: p.clone(),
                current: p,
            });
        }
        Ok(&mut self.projection.as_mut().unwrap().current)
    }
    fn owner_is_major(&mut self) -> Result<bool, Error> {
        self.flush()?;
        let mut entries = self.levels.values.iter().filter(|p| p.0 == self.city.0);
        let level = entries
            .next()
            .ok_or(Error::Missing("current owner civilization level"))?
            .1;
        if entries.next().is_some() {
            return Err(Error::Missing("duplicate owner civilization level"));
        }
        Ok(level == 0x2537_18b0)
    }
    fn building_locations(&mut self) -> Result<&[i16], Error> {
        self.flush()?;
        Ok(&self.locations)
    }
    fn pillaged_buildings(&mut self) -> Result<&[bool], Error> {
        self.flush()?;
        Ok(&self.pillaged)
    }
    fn current_project(&mut self) -> Result<i32, Error> {
        self.flush()?;
        let city = self
            .state
            .city(self.city.0, self.city.1)
            .map_err(Error::State)?;
        Ok(
            match city.build_queue.entries.first().map(|entry| entry.order) {
                Some(Order::Advance { project_hash }) => self
                    .definitions
                    .projects
                    .find_index(project_hash)
                    .map_or(-1, |i| i as i32),
                _ => -1,
            },
        )
    }
    fn game_present(&mut self) -> Result<bool, Error> {
        self.flush()?;
        Ok(true)
    }
    fn generated_power_signal(&mut self, first: bool) -> Result<(), Error> {
        self.flush()?;
        self.effects.power_changed(self.state, self.city, first)
    }
    fn shortage_changed_report(&mut self) -> Result<(), Error> {
        self.flush()?;
        self.effects.shortage_changed(self.state, self.city)
    }
}
/// Recalculate actual owned Power with current configuration and admitted rules.
/// Earlier writes remain on a callback failure. Dynamic effects are mandatory;
/// this does not imply whole-city activation or native differential acceptance.
pub fn recalculate<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions,
    city: (i32, u32),
    levels: CurrentCivilizationLevels<'_>,
    effects: &mut E,
) -> Result<(), Error> {
    if definitions.binding.as_ref() != Some(state.binding()) || levels.binding != state.binding() {
        return Err(Error::Missing("Power rules/configuration binding"));
    }
    let (locations, pillaged) =
        definitions.building_fields(state.city(city.0, city.1).map_err(Error::State)?)?;
    let mut c = Context {
        state,
        definitions,
        city,
        levels,
        effects,
        projection: None,
        locations,
        pillaged,
    };
    let result = super::recalculate(definitions, &mut c);
    // Allocation/provider errors can follow a native write; preserve its prefix.
    let flush = c.flush();
    result.and(flush)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_source_keys_roundtrip_indices_and_reject_invalid_before_writes() {
        let d = super::super::tests::definitions();
        let sources = [
            Source {
                amount_raw_q8: -7,
                building: 1,
                project: -1,
            },
            Source {
                amount_raw_q8: 9,
                building: -1,
                project: 0,
            },
            Source {
                amount_raw_q8: 0,
                building: -1,
                project: -1,
            },
        ];
        let fields = d.source_fields(&sources).unwrap();
        assert_eq!(
            fields[0].building_source_hash,
            d.buildings.type_hash(1).unwrap()
        );
        assert_eq!(fields[0].amount_raw_q8, -7);
        assert_eq!(
            fields[1].project_source_hash,
            d.projects.type_hash(0).unwrap()
        );
        assert_eq!(
            (
                fields[2].building_source_hash,
                fields[2].project_source_hash
            ),
            (u32::MAX, u32::MAX)
        );
        assert!(matches!(
            d.source_fields(&[Source {
                amount_raw_q8: 3,
                building: 99,
                project: -1
            }]),
            Err(Error::Index)
        ));
    }
}
