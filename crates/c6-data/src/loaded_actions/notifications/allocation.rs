//! Logical stock factory allocation, independent of native allocator layout.
//!
//! Pool history is gameplay state: retained entries are consumed in LIFO order;
//! an unknown older prefix is never treated as empty. Fresh allocation still
//! requires a real runtime service, including its failure/effects.
use super::processing::{Allocation, Error};
use c6_core::{
    loaded_state::LoadedState,
    notification_lifecycle::{ObjectId, PoolId},
};

/// Distinct stock factory pools, named by their registered type rather than a
/// process address. This is conditional stock registration authority, not proof
/// that callbacks left a native creator map unchanged.
pub fn stock_auto_pool(type_hash: u32) -> Result<PoolId, Error> {
    match type_hash {
        0x592d_f4c8 | 0xfe4d_413d | 0xb998_5c59 | 0xe1d3_d3ac | 0xb6bf_b191 | 0x6331_a2b6 => {
            Ok(PoolId(u64::from(type_hash)))
        }
        _ => Err(Error::Unsupported("stock auto notification factory pool")),
    }
}

/// Execute the source factory's pool branch. `fresh` runs only after the
/// authoritative pool proves empty; it supplies a new logical identity and
/// performs the reached fresh-constructor service. Unknown prefix and linked
/// Release errors retain the pool. A successful pop transfers the complete
/// prior record and runtime, including logical identity, bindings and omitted
/// live fields, to the caller.
/// Initialization and prior resource destruction happen after this transfer;
/// they must not silently discard an allocation when they fail.
pub fn allocate(
    state: &mut LoadedState,
    type_hash: u32,
    fresh: impl FnOnce(&mut LoadedState, u32) -> Result<ObjectId, Error>,
) -> Result<Allocation, Error> {
    let pool = stock_auto_pool(type_hash)?;
    // Validate the only identity required by the return contract before pop.
    // Do not classify or zero the prior parameters/target based on subtype.
    let object = state
        .last_pooled_notification(pool)?
        .map(|prior| {
            prior
                .runtime
                .live
                .as_ref()
                .map(|live| live.object)
                .ok_or(Error::Unsupported("retained notification logical identity"))
        })
        .transpose()?;
    match object {
        Some(object) => {
            let prior = state
                .pop_pooled_notification(pool)?
                .ok_or(Error::Unsupported(
                    "notification pool changed during allocation",
                ))?;
            Ok(Allocation {
                object,
                pool,
                prior: Some(prior),
            })
        }
        None => Ok(Allocation {
            object: fresh(state, type_hash)?,
            pool,
            prior: None,
        }),
    }
}
