//! Religion and Influence notification Send on the persistent stock runtime.
//! Manual construction runs before IsNeeded; only a needed, nonduplicate
//! candidate is appended and published. No policy is selected by this module.
use super::Runtime;
use crate::game_update_state::activation::social as upkeep;
use crate::loaded_actions::notifications::{self, lifecycle, processing, Created};
use c6_core::{
    game_update::Error,
    loaded_components::RetainedPlayer,
    loaded_state::LoadedState,
    notification_lifecycle::{ObjectId, PoolId},
    player_notifications_owner::OccurrenceLimits,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Pantheon,
    Religion,
    Belief,
    Influence,
}
impl Kind {
    pub fn from_hash(hash: u32) -> Option<Self> {
        Some(match hash {
            0x9456ed4a => Self::Pantheon,
            0xa6fdd52e => Self::Religion,
            0x276cf66c => Self::Belief,
            0x579b28d1 => Self::Influence,
            _ => return None,
        })
    }
    pub fn vtable(self) -> u64 {
        match self {
            Self::Pantheon => 0x3af4ec0,
            Self::Religion => 0x3af4f90,
            Self::Belief => 0x3af4ab0,
            Self::Influence => 0x3af5e30,
        }
    }
    fn blocking(self) -> i32 {
        match self {
            Self::Pantheon => 0x9be487a6u32 as i32,
            Self::Religion => 0xa94fbfc2u32 as i32,
            Self::Belief => 0x09bf97d5,
            Self::Influence => 0xf7228303u32 as i32,
        }
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .world()
        .player(owner)
        .map_err(|_| Error::Unsupported("social notification player"))?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("social notification retained player"))
}
/// ChoosePantheon IsNeeded and IsExpired use the same live eligibility query.
pub(super) fn pantheon_needed(
    state: &LoadedState,
    capabilities: &c6_core::game_capabilities::Membership,
    owner: i32,
) -> Result<bool, Error> {
    if state
        .player_slot_status(owner)
        .map_err(|_| Error::Unsupported("social notification human"))?
        != 3
    {
        return Ok(false);
    }
    if player(state, owner)?.religion.pantheon_hash != u32::MAX {
        return Ok(false);
    }
    if !capabilities.has(0xb9fc347c) {
        return Ok(false);
    }
    if player(state, owner)?.religion.pantheon_hash != u32::MAX {
        return Ok(false);
    }
    if state
        .retained()
        .religion
        .records
        .iter()
        .any(|r| r.founder == owner)
    {
        return Err(Error::Unsupported(
            "pantheon notification founded religion type index",
        ));
    }
    if state.retained().religion.next_pantheon_founder != owner {
        return Ok(false);
    }
    Ok(player(state, owner)?.religion.faith_balance_q8
        >= state
            .retained()
            .religion
            .minimum_faith_for_next_pantheon
            .wrapping_shl(8))
}
/// Query order is observable even for an expired candidate. In particular,
/// religion lookup precedes the human check for Religion and Belief.
fn needed(
    state: &LoadedState,
    queries: &upkeep::Queries<'_>,
    owner: i32,
    kind: Kind,
) -> Result<bool, Error> {
    let human = || {
        state
            .player_slot_status(owner)
            .map(|v| v == 3)
            .map_err(|_| Error::Unsupported("social notification human"))
    };
    match kind {
        Kind::Pantheon => pantheon_needed(state, queries.capabilities, owner),
        Kind::Religion | Kind::Belief => {
            // Native non-const lookup marks archive bookkeeping only; it does
            // not alter a religion payload, callbacks, policy or RNG.
            let religion = state
                .retained()
                .religion
                .records
                .iter()
                .find(|r| r.founder == owner);
            if kind == Kind::Belief && religion.is_none() {
                return Ok(false);
            }
            if !human()? {
                return Ok(false);
            }
            let earned = player(state, owner)?.religion.num_beliefs_earned;
            if kind == Kind::Belief {
                Ok(i64::from(earned) > religion.expect("checked above").belief_hashes.len() as i64)
            } else if religion.is_some() || earned <= 0 {
                Ok(false)
            } else {
                Ok(true)
            }
        }
        Kind::Influence => influence_needed(
            human,
            || Ok(player(state, owner)?.influence.tokens_to_give),
            || {
                Err(Error::Unsupported(
                    "positive influence notification CanGiveTokensToAnyPlayer",
                ))
            },
            || {
                Err(Error::Unsupported(
                    "influence notification IsGivingTokensConsidered",
                ))
            },
        ),
    }
}
fn influence_needed(
    human: impl FnOnce() -> Result<bool, Error>,
    tokens: impl FnOnce() -> Result<i32, Error>,
    can_give: impl FnOnce() -> Result<bool, Error>,
    considered: impl FnOnce() -> Result<bool, Error>,
) -> Result<bool, Error> {
    if !human()? || tokens()? <= 0 || !can_give()? {
        return Ok(false);
    }
    Ok(!considered()?)
}
/// Borrowed effects preserve the same world, ScriptState and notification owner.
pub struct Effects<'s, 'a, 'q, P> {
    pub backend: &'s mut Runtime<'a, P>,
    pub queries: &'q upkeep::Queries<'q>,
    pub definitions: &'q upkeep::Definitions,
    pub notifications: &'q notifications::Definitions,
    pub limits: OccurrenceLimits,
}
impl<P> Effects<'_, '_, '_, P> {
    fn accepts(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error> {
        use processing::Backend;
        if !self
            .backend
            .manager_initialized(state)
            .map_err(|e| self.failure(e))?
        {
            return Ok(false);
        }
        if self
            .backend
            .game_uninitializing(state)
            .map_err(|e| self.failure(e))?
        {
            return Ok(false);
        }
        state
            .player_slot_status(owner)
            .map(|s| s == 3)
            .map_err(|_| Error::Unsupported("social notification configuration"))
    }
    fn failure(&mut self, error: impl std::fmt::Debug) -> Error {
        self.backend.last_error = Some(format!("social upkeep: {error:?}"));
        Error::Unsupported("stock social effects; inspect backend.last_error")
    }
    fn send(&mut self, state: &mut LoadedState, owner: i32, hash: u32) -> Result<(), Error> {
        // Retain immutable rules membership for later notification Update.
        self.backend
            .bind_social_capabilities(self.queries.capabilities);
        let Some(_) = self.notifications.find(hash) else {
            return Ok(());
        };
        if !self.accepts(state, owner)? {
            return Ok(());
        }
        let kind =
            Kind::from_hash(hash).ok_or(Error::Unsupported("social notification factory"))?;
        // Fresh semantic identity in the same allocator domain. Native pool
        // allocation, text buffers and temporary release have no gameplay/RNG
        // consumers under Runtime's explicit plain headless scope.
        let serial = self.backend.temporary_serial;
        self.backend.temporary_serial = serial
            .checked_add(1)
            .filter(|n| *n < 1 << 63)
            .ok_or(Error::Unsupported("social temporary identity capacity"))?;
        let mut created = notifications::initialize_auto_base(
            self.notifications,
            hash,
            kind.vtable(),
            notifications::LogicalAllocation::Fresh(ObjectId((1 << 63) | serial)),
            PoolId(u64::from(hash)),
            -1,
        )
        .map_err(|e| self.failure(e))?;
        created.record.native_19b = false;
        notifications::text::initialize_headless(
            self.notifications,
            &mut created,
            notifications::text::Scope::StockHeadlessNoAddedTextConsumers,
        )
        .map_err(|e| self.failure(e))?;
        created.record.native_199 = false;
        created.record.native_19a = true;
        created.default_blocking = kind.blocking();
        // Pointer Send repeats lifecycle guards, then sets owner before Needed.
        use processing::Backend;
        if !self
            .backend
            .manager_initialized(state)
            .map_err(|e| self.failure(e))?
            || self
                .backend
                .game_uninitializing(state)
                .map_err(|e| self.failure(e))?
        {
            return Ok(());
        }
        let defined = self.notifications.find(hash).is_some();
        let accepts = self.accepts(state, owner)?;
        if !defined || !accepts {
            return Ok(());
        }
        state
            .player_notifications()
            .map_err(|e| self.failure(e))?
            .collection(owner)
            .map_err(|e| self.failure(e))?;
        created.record.id.owner = owner as i16;
        if !needed(state, self.queries, owner, kind)? {
            return Ok(());
        }
        for existing in &state
            .player_notifications()
            .map_err(|e| self.failure(e))?
            .collection(owner)
            .map_err(|e| self.failure(e))?
            .records
        {
            if !existing.is_dismissed
                && notifications::base_auto_duplicate(&created.record, existing)
                    .map_err(|e| self.failure(e))?
            {
                return Ok(());
            }
        }
        let mut pending: Option<Created> = Some(created);
        lifecycle::add(state, owner, &mut pending, self.limits, self.backend)
            .map_err(|e| self.failure(e))?;
        Ok(())
    }
}
impl<P> upkeep::Effects for Effects<'_, '_, '_, P> {
    fn queue_pantheon(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error> {
        self.backend.queue_pantheon(state, owner)
    }
    fn notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        hash: u32,
    ) -> Result<(), Error> {
        self.backend.check(state).map_err(|e| self.failure(e))?;
        self.send(state, owner, hash)
    }
    fn faith_changed(&mut self, _: &mut LoadedState, _: i32, _: i32) -> Result<(), Error> {
        Err(Error::Unsupported("faith changed signal and report"))
    }
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error> {
        self.backend.check(state).map_err(|e| self.failure(e))?;
        if event != 0xd35e571d {
            return Err(Error::Unsupported("social report event"));
        }
        upkeep::influence_per_turn(state, self.definitions, self.queries, owner)?;
        self.backend
            .reached
            .push("stock influence report current cache consumers");
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
pub fn run<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    definitions: &upkeep::Definitions,
    queries: &upkeep::Queries<'_>,
    notifications: &notifications::Definitions,
    limits: OccurrenceLimits,
    owner: i32,
) -> Result<(), Error> {
    backend.bind_social_capabilities(queries.capabilities);
    let queries = upkeep::Queries {
        cities: backend.city_query_inputs().or(queries.cities),
        ..*queries
    };
    upkeep::run(
        state,
        definitions,
        &queries,
        &mut Effects {
            backend,
            queries: &queries,
            definitions,
            notifications,
            limits,
        },
        owner,
    )
}

impl<P> Runtime<'_, P> {
    /// Bind complete current capability membership for attached Pantheon
    /// notification queries, including notifications imported before upkeep.
    pub fn bind_social_capabilities(
        &mut self,
        capabilities: &c6_core::game_capabilities::Membership,
    ) {
        self.social_capabilities = Some(capabilities.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_tokens_does_not_require_positive_influence_services() {
        assert!(!influence_needed(
            || Ok(true),
            || Ok(0),
            || panic!("CanGiveTokensToAnyPlayer must be lazy"),
            || panic!("IsGivingTokensConsidered must be lazy")
        )
        .unwrap());
        assert!(!influence_needed(
            || Ok(true),
            || Ok(-1),
            || panic!("negative token count is also expired"),
            || panic!()
        )
        .unwrap());
    }
    #[test]
    fn nonhuman_skips_tokens_and_positive_queries() {
        assert!(!influence_needed(
            || Ok(false),
            || panic!("tokens"),
            || panic!("can give"),
            || panic!("considered")
        )
        .unwrap());
    }
    #[test]
    fn positive_tokens_preserve_can_give_before_considered() {
        assert!(!influence_needed(
            || Ok(true),
            || Ok(1),
            || Ok(false),
            || panic!("considered must be lazy")
        )
        .unwrap());
        assert!(influence_needed(|| Ok(true), || Ok(1), || Ok(true), || Ok(false)).unwrap());
        assert!(!influence_needed(|| Ok(true), || Ok(1), || Ok(true), || Ok(true)).unwrap());
        assert!(influence_needed(
            || Ok(true),
            || Ok(1),
            || Err(Error::Unsupported("missing actual recipient query")),
            || panic!()
        )
        .is_err());
    }
}
