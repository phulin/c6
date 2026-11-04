//! Base InitializeText for source-bound stock XP2 notification subtypes.
//!
//! Base bytes are real definition keys, never translated placeholders. Parsed
//! forms and rendered caches require either the explicit headless projection
//! scope or an actual localization service. Neither path drops reporting hooks.
use super::{Created, Definitions};
use crate::loaded_actions::{choose_tech, city_range_attack, command_units, culture_notifications};
use c6_core::loaded_state::LoadedState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Message,
    Summary,
}

/// Conditional source scope, not a claim about a captured native HKS registry.
/// Admission requires stock localization dictionary/plural rules and stock
/// logging-only (or absent) localization logger, no UI execution and no added
/// native/script consumers of rendered text, forms, caches or allocation identity.
/// `noUI` alone is insufficient. See docs/linux-loaded-notification-text.md.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    StockHeadlessNoAddedTextConsumers,
}

#[derive(Debug)]
pub enum Error<E> {
    Unsupported(&'static str),
    Localization { field: Field, source: E },
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "notification InitializeText: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// The base write has already happened when this service is called. Implement
/// the actual host ParseBaseString and retained runtime effects synchronously;
/// do not return success merely because the base key is available. An error
/// preserves the base write and any service mutations, and skips Summary.
pub trait Localization {
    type Error;
    fn parse_base_string(
        &mut self,
        state: &mut LoadedState,
        candidate: &mut Created,
        field: Field,
    ) -> Result<(), Self::Error>;
}

/// Exact base/key projection under the explicitly admitted presentation scope.
/// This performs real Message/Summary writes; only parsed forms, render cache,
/// text allocations/refcounts and localization-only logging are excluded.
pub fn initialize_headless(
    definitions: &Definitions,
    candidate: &mut Created,
    _: Scope,
) -> Result<(), Error<std::convert::Infallible>> {
    let Some(definition) = definition(definitions, candidate)? else {
        return Ok(());
    };
    for (field, value) in [
        (Field::Message, definition.message.as_deref()),
        (Field::Summary, definition.summary.as_deref()),
    ] {
        if let Some(value) = value {
            assign_base(candidate, field, value);
        }
    }
    Ok(())
}

/// Broader-scope composition retaining the actual eager localization boundary.
/// No successful default service is supplied. Definition absence is the native
/// no-write return, after validating the actual six-subtype virtual binding.
pub fn initialize<L: Localization>(
    definitions: &Definitions,
    state: &mut LoadedState,
    candidate: &mut Created,
    localization: &mut L,
) -> Result<(), Error<L::Error>> {
    let Some(definition) = definition(definitions, candidate)? else {
        return Ok(());
    };
    for (field, value) in [
        (Field::Message, definition.message.as_deref()),
        (Field::Summary, definition.summary.as_deref()),
    ] {
        if let Some(value) = value {
            assign_base(candidate, field, value);
            localization
                .parse_base_string(state, candidate, field)
                .map_err(|source| Error::Localization { field, source })?;
        }
    }
    Ok(())
}

fn definition<'a, E>(
    definitions: &'a Definitions,
    candidate: &Created,
) -> Result<Option<&'a super::Definition>, Error<E>> {
    let hash = candidate.record.type_hash;
    let expected = match hash {
        choose_tech::TYPE_HASH => Some(choose_tech::VTABLE_ADDRESSPOINT),
        command_units::TYPE_HASH => Some(command_units::VTABLE_ADDRESSPOINT),
        city_range_attack::TYPE_HASH => Some(city_range_attack::VTABLE_ADDRESSPOINT),
        _ => culture_notifications::Kind::from_hash(hash)
            .map(|kind| kind.vtable())
            .or_else(|| {
                crate::loaded_actions::stock_notifications::social::Kind::from_hash(hash)
                    .map(|kind| kind.vtable())
            }),
    };
    if expected != Some(candidate.vtable_addresspoint) {
        return Err(Error::Unsupported("stock InitializeText virtual binding"));
    }
    Ok(definitions.find(hash))
}

fn assign_base(candidate: &mut Created, field: Field, value: &str) {
    // operator=(char const*) uses strlen; an embedded NUL ends the input.
    let bytes = value
        .as_bytes()
        .split(|byte| *byte == 0)
        .next()
        .unwrap_or_default();
    let target = match field {
        Field::Message => &mut candidate.record.message,
        Field::Summary => &mut candidate.record.summary,
    };
    target.base.clear();
    target.base.extend_from_slice(bytes);
}
