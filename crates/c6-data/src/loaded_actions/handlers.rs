//! Checked immutable runtime handler inputs and source registration remapping.
//!
//! These checks establish consistency of supplied facts, not native observation
//! authenticity. Capture authentication belongs to the caller. Registration
//! remapping never supplies an observed reverse-lookup answer.
use crate::{resource_binding::ResourceAuthority, unit_operation_manager::OperationDefinitions};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{LoadedState, ObservedSnapshotWitness},
    messages::{HandlerId, OperationHash},
    unit::{OperationNode, SKIP_TURN_HASH},
    unit_operation_interruption::{
        self as interruption, ObservationIdentity, ObservedHandler, ObservedRegistry,
    },
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum Error {
    Identity(&'static str),
    Shape(&'static str),
    Registry(interruption::Error),
    World(c6_core::world::MissingInput),
    Queue {
        owner: i32,
        id: u32,
        field: &'static str,
    },
    UnregisteredHash(u32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime handler authority: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Complete ordered slots and explicit code identities. This value validates
/// supported code/slot identities; its construction does not authenticate memory.
#[derive(Debug)]
pub struct RuntimeHandlerFacts<'a> {
    identity: ObservationIdentity<'a>,
    slots: &'a [Option<ObservedHandler>],
}
impl<'a> RuntimeHandlerFacts<'a> {
    pub fn new(
        identity: ObservationIdentity<'a>,
        slots: &'a [Option<ObservedHandler>],
    ) -> Result<Self, Error> {
        ObservedRegistry::new(identity, slots, &[]).map_err(Error::Registry)?;
        if slots.len() > 0x800 {
            return Err(Error::Shape("numeric handler slot namespace"));
        }
        for (index, handler) in slots
            .iter()
            .enumerate()
            .filter_map(|(i, h)| h.map(|h| (i, h)))
        {
            if slots[..index]
                .iter()
                .flatten()
                .any(|earlier| earlier.operation_hash == handler.operation_hash)
            {
                return Err(Error::Identity("duplicate handler operation hash"));
            }
        }
        Ok(Self { identity, slots })
    }
    fn registry(&self) -> Result<ObservedRegistry<'_>, Error> {
        // Deliberately empty: source Register remapping is not a native lookup.
        ObservedRegistry::new(self.identity, self.slots, &[]).map_err(Error::Registry)
    }
    fn remap(
        &self,
        definitions: &OperationDefinitions,
    ) -> Result<Vec<(OperationHash, HandlerId)>, Error> {
        if definitions.count() != self.slots.len() {
            return Err(Error::Identity("complete handler/definition slots"));
        }
        let mut remap = Vec::new();
        for handler in self.slots.iter().flatten() {
            let id = definitions
                .find_index(handler.operation_hash.0)
                .ok_or(Error::UnregisteredHash(handler.operation_hash.0))?;
            if id.0 != handler.runtime_id.0 {
                return Err(Error::Identity(
                    "source Register definition/handler correspondence",
                ));
            }
            remap.push((handler.operation_hash, handler.runtime_id));
        }
        Ok(remap)
    }
}

/// Known numeric Node fields from the complete initial runtime queues. Unknown
/// source bytes remain the provider's responsibility and are not classified here.
#[derive(Clone, Debug)]
pub struct InitialNodeFacts {
    pub runtime_id: HandlerId,
    pub directive: i32,
    pub flags: i32,
    pub push_turn: i32,
    pub activity: i32,
    pub automation: i32,
}
#[derive(Clone, Debug)]
pub struct InitialQueueFacts {
    pub owner: i32,
    pub id: u32,
    pub nodes: Vec<InitialNodeFacts>,
}
#[derive(Debug)]
pub struct RuntimeNode {
    pub runtime_id: HandlerId,
    pub wire: OperationNode,
}
#[derive(Debug)]
pub struct RuntimeQueue {
    pub owner: i32,
    pub id: u32,
    pub nodes: Vec<RuntimeNode>,
}
fn check_queues<'a>(
    wire: impl IntoIterator<Item = (i32, u32, &'a [OperationNode])>,
    initial: &[InitialQueueFacts],
    remap: &[(OperationHash, HandlerId)],
) -> Result<Vec<RuntimeQueue>, Error> {
    let mut initialized = Vec::new();
    for (index, (owner, id, wire_nodes)) in wire.into_iter().enumerate() {
        let actual = initial
            .get(index)
            .ok_or(Error::Shape("complete Unit queue comparison"))?;
        if (owner, id) != (actual.owner, actual.id) || wire_nodes.len() != actual.nodes.len() {
            return Err(Error::Queue {
                owner,
                id,
                field: "ordered Unit/count",
            });
        }
        if initialized
            .iter()
            .any(|queue: &RuntimeQueue| (queue.owner, queue.id) == (owner, id))
        {
            return Err(Error::Queue {
                owner,
                id,
                field: "duplicate Unit identity",
            });
        }
        let mut nodes = Vec::with_capacity(actual.nodes.len());
        for (wire, actual) in wire_nodes.iter().zip(&actual.nodes) {
            let runtime_id = remap
                .iter()
                .find(|(hash, _)| hash.0 == wire.operation_hash)
                .map(|(_, id)| *id)
                .ok_or(Error::UnregisteredHash(wire.operation_hash))?;
            if (
                runtime_id,
                wire.directive,
                wire.flags,
                wire.push_turn,
                wire.previous_activity,
                wire.previous_automation,
            ) != (
                actual.runtime_id,
                actual.directive,
                actual.flags,
                actual.push_turn,
                actual.activity,
                actual.automation,
            ) {
                return Err(Error::Queue {
                    owner,
                    id,
                    field: "remapped ID/known Node fields",
                });
            }
            nodes.push(RuntimeNode {
                runtime_id,
                wire: wire.clone(),
            });
        }
        initialized.push(RuntimeQueue { owner, id, nodes });
    }
    if initialized.len() != initial.len() {
        return Err(Error::Shape("unfiltered complete owned roster"));
    }
    Ok(initialized)
}

/// Immutable consistency authority, bound once to an untouched snapshot and its
/// complete initial queues. Native evidence classification is external to it.
pub struct BoundHandlers<'a> {
    facts: RuntimeHandlerFacts<'a>,
    destination: WireBinding,
    snapshot: ObservedSnapshotWitness,
    remap: Vec<(OperationHash, HandlerId)>,
    requested: HandlerId,
    initialized: Vec<RuntimeQueue>,
}
impl<'a> BoundHandlers<'a> {
    pub fn bind(
        state: &LoadedState,
        resources: &ResourceAuthority<'_>,
        facts: RuntimeHandlerFacts<'a>,
        initial: &[InitialQueueFacts],
    ) -> Result<Self, Error> {
        if !state.is_pristine() {
            return Err(Error::Identity(
                "handler snapshot admission requires pristine state",
            ));
        }
        if resources.destination_binding() != state.binding() {
            return Err(Error::Identity("handler snapshot destination binding"));
        }
        let snapshot = state
            .observed_snapshot()
            .ok_or(Error::Identity("associated snapshot controls required"))?;
        if snapshot.snapshot_bytes != state.source_image().len()
            || snapshot.snapshot_sha256 != <[u8; 32]>::from(Sha256::digest(state.source_image()))
        {
            return Err(Error::Identity("associated complete snapshot identity"));
        }
        let remap = facts.remap(resources.operations())?;
        let mut wire = Vec::new();
        for player in state.world().players() {
            for entry in player.unit_records().map_err(Error::World)? {
                wire.push((
                    player.owner(),
                    entry.value.id,
                    entry.value.k_operations.nodes.as_slice(),
                ));
            }
        }
        let initialized = check_queues(wire, initial, &remap)?;
        let requested = remap
            .iter()
            .find(|(hash, _)| *hash == OperationHash(SKIP_TURN_HASH))
            .map(|(_, id)| *id)
            .ok_or(Error::UnregisteredHash(SKIP_TURN_HASH))?;
        Ok(Self {
            facts,
            destination: state.binding().clone(),
            snapshot: snapshot.clone(),
            remap,
            requested,
            initialized,
        })
    }
    pub fn registry(&self) -> Result<ObservedRegistry<'_>, Error> {
        self.facts.registry()
    }
    pub fn initial_runtime_queues(&self) -> &[RuntimeQueue] {
        &self.initialized
    }
    pub fn requested_skip_turn(&self) -> HandlerId {
        self.requested
    }
    pub fn runtime_id(&self, hash: u32) -> Result<HandlerId, Error> {
        self.remap
            .iter()
            .find(|(h, _)| h.0 == hash)
            .map(|(_, id)| *id)
            .ok_or(Error::UnregisteredHash(hash))
    }
    pub fn validate_execution(
        &self,
        state: &LoadedState,
        registry: &ObservedRegistry<'_>,
        definitions: &OperationDefinitions,
    ) -> Result<(), Error> {
        if state.binding() != &self.destination
            || state.observed_snapshot() != Some(&self.snapshot)
            || registry.identity() != self.facts.identity
            || definitions.count() != self.facts.slots.len()
        {
            return Err(Error::Identity("handler execution authority"));
        }
        for (index, expected) in self.facts.slots.iter().enumerate() {
            let actual = registry
                .handler(HandlerId(index as i32))
                .map_err(Error::Registry)?
                .map(|h| h.identity());
            if actual != *expected
                || expected.is_some_and(|h| {
                    definitions
                        .find_index(h.operation_hash.0)
                        .is_none_or(|d| d.0 != h.runtime_id.0)
                })
            {
                return Err(Error::Identity("handler/definition authority changed"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_operation_manager::OperationDefinition;
    use c6_core::{unit::OperationParameters, unit_operation_interruption::InterruptionTarget};
    // Synthetic consistency inputs only; these are not original-code observations.
    fn identity() -> ObservationIdentity<'static> {
        ObservationIdentity {
            gamecore_sha256: "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa",
            configuration_sha256:
                "1111111111111111111111111111111111111111111111111111111111111111",
            capture_sha256: "2222222222222222222222222222222222222222222222222222222222222222",
        }
    }
    fn slot(id: i32, hash: u32) -> Option<ObservedHandler> {
        Some(ObservedHandler {
            runtime_id: HandlerId(id),
            operation_hash: OperationHash(hash),
            interruption_target: InterruptionTarget::LiveSlotObservation(0x2d95660),
        })
    }
    fn definitions(hashes: &[u32]) -> OperationDefinitions {
        OperationDefinitions::from_native_order(
            &hashes
                .iter()
                .enumerate()
                .map(|(i, &hash)| OperationDefinition {
                    collection_index: i as u32,
                    hash,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn wire() -> OperationNode {
        OperationNode {
            operation_hash: SKIP_TURN_HASH,
            directive: -1,
            flags: 2,
            push_turn: 17,
            previous_activity: 3,
            previous_automation: 4,
            parameters: OperationParameters::None,
        }
    }
    fn queue() -> InitialQueueFacts {
        InitialQueueFacts {
            owner: 7,
            id: 91,
            nodes: vec![InitialNodeFacts {
                runtime_id: HandlerId(0),
                directive: -1,
                flags: 2,
                push_turn: 17,
                activity: 3,
                automation: 4,
            }],
        }
    }
    #[test]
    fn slots_require_ids_unique_hashes_supported_code_and_complete_definitions() {
        assert!(matches!(
            RuntimeHandlerFacts::new(identity(), &[slot(1, SKIP_TURN_HASH)]),
            Err(Error::Registry(interruption::Error::SlotIdentity(0)))
        ));
        assert!(matches!(
            RuntimeHandlerFacts::new(
                identity(),
                &[slot(0, SKIP_TURN_HASH), slot(1, SKIP_TURN_HASH)]
            ),
            Err(Error::Identity(_))
        ));
        let mut predicted = slot(0, SKIP_TURN_HASH).unwrap();
        predicted.interruption_target = InterruptionTarget::OriginalImagePrediction(0x2d95660);
        assert!(matches!(
            RuntimeHandlerFacts::new(identity(), &[Some(predicted)]),
            Err(Error::Registry(interruption::Error::UnobservedLiveBinding(
                _
            )))
        ));
        let slots = [slot(0, SKIP_TURN_HASH), slot(1, 0x580f2f68)];
        let facts = RuntimeHandlerFacts::new(identity(), &slots).unwrap();
        assert!(facts.remap(&definitions(&[SKIP_TURN_HASH])).is_err());
        assert!(facts
            .remap(&definitions(&[0x580f2f68, SKIP_TURN_HASH]))
            .is_err());
        assert_eq!(
            facts
                .remap(&definitions(&[SKIP_TURN_HASH, 0x580f2f68]))
                .unwrap(),
            [
                (OperationHash(SKIP_TURN_HASH), HandlerId(0)),
                (OperationHash(0x580f2f68), HandlerId(1))
            ]
        );
        assert!(matches!(
            facts
                .registry()
                .unwrap()
                .saved_hash_runtime_id(OperationHash(SKIP_TURN_HASH)),
            Err(interruption::Error::UnobservedHash(_))
        ));
    }
    #[test]
    fn initial_queues_require_full_ordered_roster_including_empty_queues() {
        let nodes = [wire()];
        let empty = InitialQueueFacts {
            owner: 8,
            id: 2,
            nodes: vec![],
        };
        let remap = [(OperationHash(SKIP_TURN_HASH), HandlerId(0))];
        let queues = [queue(), empty.clone()];
        assert!(check_queues(
            [(7, 91, nodes.as_slice()), (8, 2, &[] as &[OperationNode])],
            &queues,
            &remap
        )
        .is_ok());
        assert!(check_queues([(7, 91, nodes.as_slice())], &queues, &remap).is_err());
        assert!(check_queues(
            [(8, 2, &[] as &[OperationNode]), (7, 91, nodes.as_slice())],
            &queues,
            &remap
        )
        .is_err());
        assert!(check_queues(
            [(6, 91, nodes.as_slice()), (8, 2, &[] as &[OperationNode])],
            &queues,
            &remap
        )
        .is_err());
        assert!(check_queues(
            [(7, 91, nodes.as_slice()), (8, 2, &[] as &[OperationNode])],
            &[queue()],
            &remap
        )
        .is_err());
        assert!(check_queues(
            [
                (8, 2, &[] as &[OperationNode]),
                (8, 2, &[] as &[OperationNode])
            ],
            &[empty.clone(), empty],
            &remap
        )
        .is_err());
    }
    #[test]
    fn every_known_initial_node_field_is_checked_without_reverse_lookup() {
        let nodes = [wire()];
        let remap = [(OperationHash(SKIP_TURN_HASH), HandlerId(0))];
        for field in 0..6 {
            let mut queue = queue();
            let node = &mut queue.nodes[0];
            match field {
                0 => node.runtime_id = HandlerId(1),
                1 => node.directive += 1,
                2 => node.flags += 1,
                3 => node.push_turn += 1,
                4 => node.activity += 1,
                _ => node.automation += 1,
            }
            assert!(matches!(
                check_queues([(7, 91, nodes.as_slice())], &[queue], &remap),
                Err(Error::Queue {
                    field: "remapped ID/known Node fields",
                    ..
                })
            ));
        }
        assert!(matches!(
            check_queues([(7, 91, nodes.as_slice())], &[queue()], &[]),
            Err(Error::UnregisteredHash(SKIP_TURN_HASH))
        ));
        let result = check_queues([(7, 91, nodes.as_slice())], &[queue()], &remap).unwrap();
        assert_eq!(result[0].nodes[0].wire, nodes[0]);
        assert_eq!(result[0].nodes[0].runtime_id, HandlerId(0));
    }
}
