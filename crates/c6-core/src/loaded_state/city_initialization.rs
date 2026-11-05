//! Exclusive access to canonical retained Cities for live creation effects.
use super::{Error, LoadedState, MissingInput};

impl LoadedState {
    /// Current canonical owner collection. Reserve revision before exposing it;
    /// callbacks may change collection size, so callers must reacquire it.
    pub fn cities_mut(&mut self, owner: i32) -> Result<&mut crate::core_player::Cities, Error> {
        self.world.player(owner).map_err(Error::World)?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Binding)?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[player].cities)
    }

    /// Resolve full current identity before reserving a mutation revision.
    /// Callers keep any partially applied transition explicit on later failure.
    pub fn city_mut(&mut self, owner: i32, id: u32) -> Result<&mut crate::city::City, Error> {
        self.city(owner, id)?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(Error::Missing(MissingInput::City { owner, id }))?;
        let city = self.retained.players[player]
            .cities
            .records
            .entries
            .iter()
            .position(|entry| entry.value.header.owner == owner && entry.value.header.id == id)
            .ok_or(Error::Missing(MissingInput::City { owner, id }))?;
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.retained.players[player].cities.records.entries[city].value)
    }
}

impl LoadedState {
    /// Install the source-created canonical City and factory initialization
    /// prefix. The returned City still requires signal/CityBuild continuation.
    /// Failures after slot allocation retain the explicit incomplete prefix.
    pub fn begin_city_creation(
        &mut self,
        definitions: &crate::city_initialization::Definitions<'_>,
        owner: i32,
        parameters: crate::city_initialization::Parameters,
    ) -> Result<
        Option<crate::unit::ComponentId>,
        crate::city_initialization::CreationError<std::convert::Infallible>,
    > {
        use crate::city_initialization::{self as initialization, CreationError};
        self.world
            .player(owner)
            .map_err(|error| CreationError::State(Error::World(error)))?;
        let player = self
            .retained
            .players
            .iter()
            .position(|p| p.header.owner == owner)
            .ok_or(CreationError::State(Error::Binding))?;
        let next = self
            .revision
            .checked_add(1)
            .ok_or(CreationError::State(Error::ResourceLimit))?;
        let Some(allocation) =
            initialization::allocate(&mut self.retained.players[player].cities.records)
                .map_err(CreationError::Initialization)?
        else {
            return Ok(None);
        };
        self.revision = next;
        let value =
            initialization::constructor::construct(definitions, owner, allocation.id, parameters)
                .map_err(CreationError::Initialization)?;
        allocation
            .insert(&mut self.retained.players[player].cities.records, value)
            .map_err(CreationError::Initialization)?;
        let cities = &mut self.retained.players[player].cities;
        cities.peak_cities = cities.peak_cities.max(cities.records.entries.len() as u32);
        Ok(Some(crate::unit::ComponentId {
            owner: owner as i16,
            component_type: 2,
            id: allocation.id,
        }))
    }
}

#[cfg(test)]
mod placed_tests {
    use super::*;
    use crate::{
        city_initialization as base,
        found_city::{creation, Creation},
        unit::{ComponentId, PlotCoord},
    };
    struct Context {
        state: LoadedState,
        log: Vec<&'static str>,
        reverse_capital: bool,
        placement: i32,
        rename_invalidates_location: bool,
        expected_initializing: bool,
        collection_runtime: crate::melee_attack::proposals::collections::Runtime,
        object_city: Option<ComponentId>,
    }
    impl base::Context for Context {
        type Error = &'static str;
        fn state(&self) -> &LoadedState {
            &self.state
        }
        fn state_mut(&mut self) -> &mut LoadedState {
            &mut self.state
        }
        fn subscribe_city_signals(&mut self, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push("subscribe");
            Ok(())
        }
        fn initialize_city_build(&mut self, _: ComponentId) -> Result<Option<u32>, Self::Error> {
            self.log.push("citybuild");
            Ok(Some(77))
        }
    }
    impl creation::Context for Context {
        fn generate_name(
            &mut self,
            _: ComponentId,
            first: bool,
            _: PlotCoord,
        ) -> Result<Vec<u8>, Self::Error> {
            assert!(first);
            self.log.push("name");
            crate::found_city::names::mark_used(
                self.state.game_city_names_mut().unwrap(),
                b"Capital",
            )
            .unwrap();
            Ok(b"Capital".to_vec())
        }
        fn renamed_signal(&mut self, c: ComponentId) -> Result<(), Self::Error> {
            self.log.push("renamed");
            if self.rename_invalidates_location {
                let v = self.state.city_mut(i32::from(c.owner), c.id).unwrap();
                v.header.location_x = -9999;
            }
            Ok(())
        }
        fn report(&mut self, h: u32, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push(if h == 0x7bbb_e6e0 {
                "capitalreport"
            } else {
                "namereport"
            });
            Ok(())
        }
        fn place(
            &mut self,
            c: ComponentId,
            p: PlotCoord,
            initializing: bool,
        ) -> Result<i32, Self::Error> {
            assert_eq!(initializing, self.expected_initializing);
            self.log.push("place");
            let v = self.state.city_mut(i32::from(c.owner), c.id).unwrap();
            v.header.location_x = p.x;
            v.header.location_y = p.y;
            Ok(self.placement)
        }
        fn destroy_failed_placement(&mut self, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push("destroy");
            Err("required destroy")
        }
        fn city_created(&mut self, _: ComponentId, _: bool) -> Result<(), Self::Error> {
            self.log.push("created");
            Ok(())
        }
        fn current_continent(&mut self, _: ComponentId) -> Result<u32, Self::Error> {
            self.log.push("continent");
            Ok(0x12345678)
        }
        fn settled_on_continent(&mut self, _: ComponentId, _: u32) -> Result<(), Self::Error> {
            self.log.push("settled");
            Ok(())
        }
        fn is_free_cities(&mut self, _: i32) -> Result<bool, Self::Error> {
            self.log.push("freecities");
            Ok(false)
        }
        fn capital_changed(&mut self, c: ComponentId) -> Result<(), Self::Error> {
            self.log.push("capital");
            if self.reverse_capital {
                self.state
                    .city_mut(i32::from(c.owner), c.id)
                    .unwrap()
                    .header
                    .capital = false;
            }
            Ok(())
        }
        fn add_palace(&mut self, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push("palace");
            Ok(())
        }
        fn cultural_identity_created(&mut self, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push("culture");
            Ok(())
        }
        fn check_production(&mut self, _: ComponentId) -> Result<(), Self::Error> {
            self.log.push("production");
            Ok(())
        }
    }
    impl crate::melee_attack::proposals::collections::Context<u64> for Context {
        type Error = &'static str;
        fn collections(
            &self,
        ) -> Result<&crate::melee_attack::proposals::collections::Runtime, Self::Error> {
            Ok(&self.collection_runtime)
        }
        fn collections_mut(
            &mut self,
        ) -> Result<&mut crate::melee_attack::proposals::collections::Runtime, Self::Error>
        {
            Ok(&mut self.collection_runtime)
        }
        fn item_changed(
            &mut self,
            receiver: u64,
            object: &mut u64,
            added: bool,
        ) -> Result<(), Self::Error> {
            assert!(added);
            assert_eq!(*object, 123);
            let city = self.object_city.unwrap();
            let value = self.state.city_mut(i32::from(city.owner), city.id).unwrap();
            value.header.production_unit_changes = value
                .header
                .production_unit_changes
                .wrapping_add(receiver as i32);
            Ok(())
        }
    }
    impl crate::found_city::collections::Context for Context {
        fn state(&self) -> &LoadedState {
            &self.state
        }
        fn make_city_object(&mut self, city: ComponentId) -> Result<u64, Self::Error> {
            self.log.push("make-object");
            self.object_city = Some(city);
            Ok(123)
        }
        fn release_city_object(&mut self, object: u64) -> Result<(), Self::Error> {
            assert_eq!(object, 123);
            self.log.push("release-object");
            Ok(())
        }
    }
    fn context() -> Context {
        let mut state = super::super::tests::state();
        let binding = state.binding().clone();
        state
            .install_game_city_names(
                &binding,
                crate::game_name_manager_wire::NameManager {
                    wire_version: 1,
                    used_names: vec![],
                },
                None,
            )
            .unwrap();
        let owner = state.retained.players[0].header.owner;
        let cities = state.cities_mut(owner).unwrap();
        cities.records = crate::map::Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: u32::MAX,
            free_tail: u32::MAX,
            free_links: vec![],
        };
        cities.settled_continent_hashes.clear();
        Context {
            state,
            log: vec![],
            reverse_capital: false,
            placement: 1,
            rename_invalidates_location: false,
            expected_initializing: false,
            collection_runtime: crate::melee_attack::proposals::collections::Runtime::from_initial(
                vec![(
                    100,
                    crate::melee_attack::proposals::collections::Collection {
                        kind: None,
                        added: crate::unit_moved_signal::ListenerRing::from_ordered(&[11, 12])
                            .unwrap(),
                    },
                )],
            )
            .unwrap(),
            object_city: None,
        }
    }
    fn create(context: &mut Context) -> Result<Option<ComponentId>, creation::Error<&'static str>> {
        let owner = context.state.retained.players[0].header.owner;
        let definitions = base::Definitions {
            yields: &[0, 1],
            units: &[],
            buildings: &[],
            districts: &[],
            projects: &[],
            terrains: &[],
            resources: &[],
            features: &[0x1111, 0x2222],
            production_yield: 1,
            loyalty_start: 100,
            building_support: &[],
        };
        creation::create(
            context,
            &definitions,
            base::Parameters {
                original_owner: owner,
                transfer_type: 0x61723064,
                original_capital: true,
                founded_turn: 8,
            },
            Creation {
                owner,
                location: PlotCoord { x: 18, y: 11 },
                transfer_type: 0x61723064,
                first_city: true,
                original_capital: true,
            },
            false,
        )
    }
    #[test]
    fn supplied_transfer_name_marks_canonical_manager_without_generating_name() {
        let mut c = context();
        c.expected_initializing = true;
        let owner = c.state.retained.players[0].header.owner;
        let definitions = base::Definitions {
            yields: &[0, 1],
            units: &[],
            buildings: &[],
            districts: &[],
            projects: &[],
            terrains: &[],
            resources: &[],
            features: &[0x1111, 0x2222],
            production_yield: 1,
            loyalty_start: 100,
            building_support: &[],
        };
        let city = creation::create_with_name(
            &mut c,
            &definitions,
            base::Parameters::default(),
            Creation {
                owner,
                location: PlotCoord { x: 18, y: 11 },
                transfer_type: 0xba0d34de,
                first_city: false,
                original_capital: false,
            },
            true,
            Some(b"Captured Name\0ignored"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            c.state.city(owner, city.id).unwrap().header.name,
            b"Captured Name"
        );
        assert_eq!(
            c.state.game_city_names().unwrap().used_names,
            [b"Captured Name".to_vec()]
        );
        assert!(!c.log.contains(&"name"));
        assert!(!c.log.contains(&"settled"));
        assert!(!c.log.contains(&"capital"));
        assert_eq!(
            c.log,
            [
                "subscribe",
                "citybuild",
                "place",
                "created",
                "continent",
                "culture",
                "production"
            ]
        );
    }
    #[test]
    fn placed_creation_edits_canonical_identity_and_preserves_synchronous_order() {
        let mut c = context();
        let city = create(&mut c).unwrap().unwrap();
        let v = c.state.city(i32::from(city.owner), city.id).unwrap();
        assert_eq!(city.id, 65536);
        assert_eq!(v.header.city_ai_id, 77);
        assert_eq!(v.turn_city_founded, 8);
        assert_eq!(v.header.name, b"Capital");
        assert!(v.header.capital && v.header.original_capital);
        assert_eq!(
            c.state.game_city_names().unwrap().used_names,
            [b"Capital".to_vec()]
        );
        let p = c
            .state
            .retained
            .players
            .iter()
            .find(|p| p.header.owner == i32::from(city.owner))
            .unwrap();
        assert_eq!(p.cities.home_continent_hash, 0x12345678);
        assert_eq!(p.cities.settled_continent_hashes, [0x12345678]);
        assert_eq!(
            c.log,
            [
                "subscribe",
                "citybuild",
                "name",
                "place",
                "created",
                "continent",
                "settled",
                "freecities",
                "capital",
                "capitalreport",
                "palace",
                "culture",
                "production"
            ]
        );
    }
    #[test]
    fn callbacks_are_reread_and_missing_destroy_retains_explicit_prefix() {
        let mut c = context();
        c.reverse_capital = true;
        let city = create(&mut c).unwrap().unwrap();
        assert!(!c.log.contains(&"palace"));
        assert!(
            !c.state
                .city(i32::from(city.owner), city.id)
                .unwrap()
                .header
                .capital
        );
        c.log.clear();
        c.rename_invalidates_location = true;
        creation::set_name(&mut c, city, b"Renamed\0ignored").unwrap();
        assert_eq!(c.log, ["renamed"]);
        assert_eq!(
            c.state
                .city(i32::from(city.owner), city.id)
                .unwrap()
                .header
                .name,
            b"Renamed"
        );
        let mut failed = context();
        failed.placement = 0;
        assert!(matches!(
            create(&mut failed),
            Err(creation::Error::Effect("required destroy"))
        ));
        assert_eq!(
            failed.log,
            ["subscribe", "citybuild", "name", "place", "destroy"]
        );
        assert_eq!(
            failed.state.retained.players[0]
                .cities
                .records
                .entries
                .len(),
            1
        );
    }
    #[test]
    fn city_collections_filter_current_owner_and_death_without_faking_proposals() {
        use crate::found_city::collections::{self, Collection};
        let mut c = context();
        let city = create(&mut c).unwrap().unwrap();
        c.log.clear();
        collections::city_created(
            &mut c,
            Collection::PlayerCities {
                receiver: 100,
                owner: i32::from(city.owner) + 1,
            },
            city,
        )
        .unwrap();
        assert!(c.log.is_empty());
        collections::city_created(
            &mut c,
            Collection::PlayerCities {
                receiver: 100,
                owner: i32::from(city.owner),
            },
            city,
        )
        .unwrap();
        assert_eq!(c.log, ["make-object", "release-object"]);
        assert_eq!(
            c.state
                .city(i32::from(city.owner), city.id)
                .unwrap()
                .header
                .production_unit_changes,
            23
        );
        c.state
            .city_mut(i32::from(city.owner), city.id)
            .unwrap()
            .header
            .delayed_death = true;
        c.log.clear();
        collections::city_created(
            &mut c,
            Collection::PlayerCities {
                receiver: 100,
                owner: i32::from(city.owner),
            },
            city,
        )
        .unwrap();
        assert!(c.log.is_empty());
        collections::city_created(&mut c, Collection::AllCities { receiver: 100 }, city).unwrap();
        assert_eq!(c.log, ["make-object", "release-object"]);
    }
    #[test]
    fn general_feature_effect_mutates_only_admitted_current_definition_slot() {
        use crate::found_city::feature_production::{self as effects, Effect, Target};
        let mut c = context();
        let city = create(&mut c).unwrap().unwrap();
        let hashes = [0x1111, 0x2222];
        let e = Effect {
            target: Target::District,
            native_feature: 1,
            percent: 50,
        };
        effects::apply(&mut c.state, city, &hashes, e, false).unwrap();
        let q = &c
            .state
            .city(i32::from(city.owner), city.id)
            .unwrap()
            .build_queue;
        assert_eq!(
            q.district_adjacent_natural_wonder_production_modifier
                .iter()
                .map(|v| v.value)
                .collect::<Vec<_>>(),
            [256, 384]
        );
        assert_eq!(
            q.wonder_adjacent_natural_wonder_production_modifier
                .iter()
                .map(|v| v.value)
                .collect::<Vec<_>>(),
            [256, 256]
        );
        let revision = c.state.revision();
        assert!(matches!(
            effects::apply(&mut c.state, city, &[0, 1], e, false),
            Err(effects::Error::CurrentArrayShape)
        ));
        assert_eq!(c.state.revision(), revision);
        effects::apply(&mut c.state, city, &hashes, e, true).unwrap();
        assert_eq!(
            c.state
                .city(i32::from(city.owner), city.id)
                .unwrap()
                .build_queue
                .district_adjacent_natural_wonder_production_modifier[1]
                .value,
            256
        );
        effects::apply(
            &mut c.state,
            ComponentId {
                owner: -1,
                component_type: 0,
                id: u32::MAX,
            },
            &[],
            Effect {
                native_feature: -1,
                ..e
            },
            false,
        )
        .unwrap();
    }
}
