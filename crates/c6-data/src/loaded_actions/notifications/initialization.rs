//! Source Read initialization for the bounded stock notification owner.
//! This is not observed final native runtime. It must run once before actions;
//! callbacks subsequently read/mutate the same owned runtime occurrences.
use super::{Definitions, Error};
use c6_core::{
    loaded_state::LoadedState,
    notification_lifecycle::{
        ActivationBinding, DefinitionIcon, DestructionBinding, LiveNotification,
        ObjectId as LiveObjectId, PoolId, ReleaseBinding,
    },
    player_notifications_owner::Runtime,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    /// Pinned XP2 stock registration and source constructor/Read semantics.
    /// Extra creator registrations and later native load callbacks are outside
    /// this projection; callers must not call it a captured live-state import.
    SourceReadStockXp2,
}

pub const BASE_VTABLE: u64 = 0x3af6238;
pub const PRIDE_VTABLE: u64 = 0x3af6530;
pub const BASE_POOL: PoolId = PoolId(0);
pub const PRIDE_POOL: PoolId = PoolId(0x0c19b649);

/// Install only source-proven omitted fields for the actual ff31 saved family.
/// Source Manager registration specializes PrideMoment; the six other factory
/// hashes here use Base fallback. Both inherit Base Initialize, which clears
/// locks/default blocking; Instance::Read does not serialize those fields.
/// Every saved field remains intact. No native pointer or capacity is required.
pub fn initialize_source_read(
    state: &mut LoadedState,
    definitions: &Definitions,
    scope: Scope,
) -> Result<(), Error> {
    let Scope::SourceReadStockXp2 = scope;
    let owner = state.player_notifications().map_err(Error::State)?;
    let mut inputs = Vec::new();
    for slot in 0..64 {
        let collection = owner.collection(slot).map_err(|_| Error::Owner(slot))?;
        for (ordinal, record) in collection.records.iter().enumerate() {
            let (vtable, pool) = match record.factory_type {
                0x0c19b649 => (PRIDE_VTABLE, PRIDE_POOL),
                0xcb5eed15 | 0x9efbc217 | 0xffad0d16 | 0xba4af0fb | 0xe90d955a | 0x458dcd81 => {
                    (BASE_VTABLE, BASE_POOL)
                }
                _ => {
                    return Err(Error::Unsupported(
                        "source initialization for saved notification factory",
                    ))
                }
            };
            if record.type_hash != record.factory_type {
                return Err(Error::Unsupported(
                    "saved notification factory/type mismatch initialization",
                ));
            }
            let occurrence = owner
                .object(slot, ordinal)
                .map_err(|_| Error::Owner(slot))?;
            let serial = u64::try_from(occurrence.index())
                .map_err(|_| Error::Unsupported("logical notification identity limit"))?;
            if serial >= (1u64 << 48) {
                return Err(Error::Unsupported("logical notification identity limit"));
            }
            let definition = definitions.find(record.factory_type);
            let icon = definition.map_or(DefinitionIcon::MissingDefinition, |d| {
                DefinitionIcon::ShowInSinglePlayer(d.show_icon_single_player)
            });
            let group = definition
                .and_then(|d| d.group_type.as_deref())
                .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
                .unwrap_or(0xb6fa1284);
            inputs.push((
                occurrence,
                Runtime {
                    live: Some(LiveNotification {
                        object: LiveObjectId((slot as u64) << 48 | serial),
                        lock_count: 0,
                        pool_binding: Some(pool),
                        definition_icon: icon,
                        activation: ActivationBinding::Base,
                        release: ReleaseBinding::Base,
                        destruction: DestructionBinding::BaseResources,
                    }),
                    default_blocking: Some(0),
                    group_hash: Some(group),
                    vtable_addresspoint: Some(vtable),
                },
            ));
        }
    }
    let binding = state.binding().clone();
    state
        .initialize_notification_runtime(&binding, inputs)
        .map_err(Error::State)
}
