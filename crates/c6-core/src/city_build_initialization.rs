//! Normal unplaced-City AI association, not native production policy selection.
//! Create1fcff20 checks manager initialization and allocates an 8-slot/16-bit ID.
//! Initialize1fd74c0 binds the City/owner/trade object and initializes scalar
//! state. Production evaluation arrays, analyzers, leader event lists and native
//! policy contract subscriptions are explicitly omitted for external policies.
//! No constructor/initialization RNG call is replaced with a host PRNG.
use crate::loaded_state::{ActionStamp, LoadedState};
use crate::map::{Recycled, Slot};
use crate::player_ai_city_manager::{
    BoolByte, CityBuild, CityBuildEntry, CityManager, ComponentId, ControlHandle, EvaluationLayers,
    PlanningData, RecycledCityBuilds, TradeCity, CITY_BUILD_VERSION,
};

#[derive(Debug)]
pub enum Error {
    State(crate::loaded_state::Error),
    Binding,
    Owner,
    AlreadyPlacedCity,
    MissingManager(i32),
    Allocator(crate::city_initialization::Error),
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CityBuild creation: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Unsaved manager status supplied by canonical initialization/live input. It
/// is not inferred from a saved CityManager, a player ID or alive flag. Ordinary
/// City insertion may advance revision; the input remains bound to this owner
/// and same exclusive state instance. The caller owns status freshness.
#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    stamp: ActionStamp,
    owner: i32,
    manager_initialized: bool,
}
impl Inputs {
    pub fn bind(state: &LoadedState, owner: i32, manager_initialized: bool) -> Self {
        Self {
            stamp: state.action_stamp(),
            owner,
            manager_initialized,
        }
    }
}

/// Called after the canonical City is inserted and before founded-turn is set.
/// The City constructor's caller writes the returned ID into City.city_ai_id,
/// matching the native ordering. Unsupported placed-City reinitialization is
/// explicit; it must not silently omit InitializePlotAnalyzers.
pub fn create(
    state: &mut LoadedState,
    owner: i32,
    city_id: u32,
    inputs: &Inputs,
) -> Result<Option<u32>, Error> {
    if inputs.owner != owner || !inputs.stamp.same_instance(state.action_stamp()) {
        return Err(Error::Binding);
    }
    if !inputs.manager_initialized {
        return Ok(None);
    }
    let city = state.city(owner, city_id).map_err(Error::State)?;
    if [city.header.location_x, city.header.location_y] != [-9999, -9999] {
        return Err(Error::AlreadyPlacedCity);
    }
    let city = ComponentId {
        owner: i16::try_from(owner).map_err(|_| Error::Owner)?,
        component_type: 2,
        id: city_id,
    };
    create_in_manager(state.city_build_manager_mut(owner)?, city)
}

/// Only the current manager collection is moved into the common allocator view;
/// existing CityBuild records retain identity and values. No World is cloned.
fn create_in_manager(manager: &mut CityManager, city: ComponentId) -> Result<Option<u32>, Error> {
    if manager.player_id != i32::from(city.owner) {
        return Err(Error::Owner);
    }
    let saved = &mut manager.city_ais;
    let mut records = Recycled {
        capacity: saved.capacity,
        entries: std::mem::take(&mut saved.entries)
            .into_iter()
            .map(|entry| Slot {
                index: entry.index,
                value: entry.city_build,
            })
            .collect(),
        current_id_bits: saved.current_id_bits,
        free_head: saved.first_free,
        free_tail: saved.last_free,
        free_links: std::mem::take(&mut saved.free_indices),
    };
    let result = (|| {
        let Some(allocation) =
            crate::city_initialization::allocate(&mut records).map_err(Error::Allocator)?
        else {
            return Ok(None);
        };
        allocation
            .insert(&mut records, initialized_record(city, allocation.id))
            .map_err(Error::Allocator)?;
        Ok(Some(allocation.id))
    })();
    *saved = RecycledCityBuilds {
        capacity: records.capacity,
        entries: records
            .entries
            .into_iter()
            .map(|entry| CityBuildEntry {
                index: entry.index,
                city_build: entry.value,
            })
            .collect(),
        current_id_bits: records.current_id_bits,
        first_free: records.free_head,
        last_free: records.free_tail,
        free_indices: records.free_links,
    };
    result
}

fn initialized_record(city: ComponentId, city_ai_id: u32) -> CityBuild {
    // Constructor1fd5c70/Initialize1fd74c0 values, not CityBuild::default() or an
    // observed/template City. Omitted planner vectors are guarded unavailable
    // storage and cannot be queried, encoded or counted as native cache parity.
    CityBuild {
        planning_data: PlanningData::ExternalPolicyOmitted,
        wire_version: CITY_BUILD_VERSION,
        city_id: city.id,
        player_owner: i32::from(city.owner),
        city_ai_id,
        city: city.clone(),
        current_yields: Vec::new(),
        current_base_yields: None,
        current_gpps: Vec::new(),
        best_improvement_hash: u32::MAX,
        city_threat: -1,
        under_attack: BoolByte(0),
        desired_defenders: 0,
        defenders_needed: 0,
        reservations: Vec::new(),
        event_trees: vec![u32::MAX; 6],
        managed_working_contract: ControlHandle {
            player_id: -1,
            id: u32::MAX,
        },
        managed_contracts: Vec::new(),
        done_off_turn: BoolByte(0),
        best_plot_to_clear: [-9999, -9999],
        best_plot_to_buy: [-9999, -9999],
        food_loss_amounts: Vec::new(),
        food_loss_turns: Vec::new(),
        food_loss_populations: Vec::new(),
        city_unit_efficiencies: Vec::new(),
        city_trade: TradeCity {
            city,
            current_routes: 0,
            current_units: 0,
            connected_cities: Vec::new(),
            known_failed_connections: Vec::new(),
        },
        yield_preferences: Vec::new(),
        plot_evaluations: Vec::new(),
        evaluation_layers: EvaluationLayers(Vec::new()),
        unit_evaluations: Vec::new(),
        cached_unit_evaluations: Vec::new(),
        project_evaluations: Vec::new(),
        power_surplus: 0,
        default_build_present_value: 0,
        unimproved_resources: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> CityManager {
        CityManager {
            player_id: 3,
            city_ais: RecycledCityBuilds {
                capacity: 0,
                entries: vec![],
                current_id_bits: 0,
                first_free: u32::MAX,
                last_free: u32::MAX,
                free_indices: vec![],
            },
        }
    }
    fn city() -> ComponentId {
        ComponentId {
            owner: 3,
            component_type: 2,
            id: 0x50007,
        }
    }
    #[test]
    fn first_block_real_identity_and_scalar_initialization_do_not_clone_planning() {
        let mut manager = empty();
        let id = create_in_manager(&mut manager, city()).unwrap().unwrap();
        assert_eq!(id, 0x10000);
        assert_eq!(manager.city_ais.capacity, 8);
        assert_eq!(manager.city_ais.free_indices, (1..8).collect::<Vec<_>>());
        let record = &manager.city_ais.entries[0].city_build;
        assert_eq!(
            (record.city_ai_id, record.city_id, record.player_owner),
            (id, 0x50007, 3)
        );
        assert_eq!(record.city, city());
        assert_eq!(record.city_trade.city, city());
        assert_eq!(
            (
                record.city_threat,
                record.desired_defenders,
                record.defenders_needed
            ),
            (-1, 0, 0)
        );
        assert_eq!((record.under_attack.0, record.done_off_turn.0), (0, 0));
        assert_eq!(record.best_improvement_hash, u32::MAX);
        assert_eq!(record.best_plot_to_buy, [-9999, -9999]);
        assert_eq!(record.event_trees, [u32::MAX; 6]);
        assert_eq!(
            (
                record.managed_working_contract.player_id,
                record.managed_working_contract.id
            ),
            (-1, u32::MAX)
        );
        assert!(matches!(
            record.require_retained_planning(),
            Err(crate::player_ai_city_manager::Error::ExternalPolicyPlanningOmitted)
        ));
        assert_eq!(
            manager.encode(crate::unit::Limits::default()),
            Err(crate::player_ai_city_manager::Error::ExternalPolicyPlanningOmitted)
        );
    }
    #[test]
    fn recycled_slot_order_generation_wrap_and_existing_records_survive() {
        let mut manager = empty();
        create_in_manager(&mut manager, city()).unwrap();
        manager.city_ais.current_id_bits = 0xffff0000;
        manager.city_ais.free_indices = vec![5, 3, 1, 6, 2, 4, 7];
        manager.city_ais.first_free = 5;
        let old = manager.city_ais.entries[0].city_build.clone();
        assert_eq!(
            create_in_manager(&mut manager, ComponentId { id: 9, ..city() }).unwrap(),
            Some(5)
        );
        assert_eq!(manager.city_ais.entries[0].city_build, old);
        assert_eq!(
            create_in_manager(&mut manager, ComponentId { id: 10, ..city() }).unwrap(),
            Some(0x10003)
        );
        assert_eq!(
            manager
                .city_ais
                .entries
                .iter()
                .map(|e| e.index)
                .collect::<Vec<_>>(),
            [0, 3, 5]
        );
        assert_eq!(manager.city_ais.free_indices, [1, 6, 2, 4, 7]);
    }
    #[test]
    fn invalid_allocator_or_owner_is_not_repaired_or_assigned_an_id() {
        let mut manager = empty();
        manager.city_ais.first_free = 0;
        let before = manager.clone();
        assert!(matches!(
            create_in_manager(&mut manager, city()),
            Err(Error::Allocator(_))
        ));
        assert_eq!(manager, before);
        assert!(matches!(
            create_in_manager(&mut manager, ComponentId { owner: 4, ..city() }),
            Err(Error::Owner)
        ));
        assert_eq!(manager, before);
    }
}
