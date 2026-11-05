//! Explicit policy requests on canonical Culture. No government or policy is
//! selected by the library; synchronous consumers share the caller's owners.
pub mod modifiers;
use super::{choose_tech, culture_notifications as culture};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, government_policy as kernel,
    loaded_state::LoadedState, player_culture::Culture, player_prerequisites::IndexedBools,
};

#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Culture(culture::Error),
    Rules(crate::Error),
    Effects(E),
    Processor(c6_core::game_update::Error),
    Notifications(c6_core::unit_operation_aftermath::Error),
    Aftermath(super::current_aftermath::Error<super::notifications::processing::Error>),
    Unsupported(&'static str),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "policy action: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

struct Policy {
    slot: i32,
    restrictions: bool,
    governments: Vec<u32>,
}
pub struct Definitions {
    binding: WireBinding,
    policies: LinuxDefinitionRegistry,
    entries: Vec<Policy>,
    wildcard: Vec<bool>,
    government_unlocked_policy: Vec<(u32, u32)>,
    unlocks: culture::Definitions,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        use crate::unit_initialization_definitions::collection;
        let policies = collection(rules, "Policies", "PolicyType")
            .map_err(|_| Error::Unsupported("Policy collection"))?;
        let slots = collection(rules, "GovernmentSlots", "GovernmentSlotType")
            .map_err(|_| Error::Unsupported("GovernmentSlot collection"))?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(Error::Unsupported("policy rules table"))
        };
        let mut entries = Vec::new();
        for entry in policies.entries() {
            let hash = linux_type_hash(&entry.type_name);
            let row=table("Policies")?.rows().find(|r|matches!(r.get("PolicyType"),Ok(SqlValue::Text(t)) if linux_type_hash(t)==hash)).ok_or(Error::Unsupported("Policy row"))?;
            let slot = match row.get("GovernmentSlotType").map_err(Error::Rules)? {
                SqlValue::Text(t) => slots
                    .find_index(linux_type_hash(t))
                    .ok_or(Error::Unsupported("Policy slot definition"))?
                    as i32,
                _ => return Err(Error::Unsupported("Policy slot")),
            };
            let mut restrictions = false;
            for row in table("Policies_XP1")?.rows().filter(
                |r| matches!(r.get("PolicyType"),Ok(SqlValue::Text(t)) if linux_type_hash(t)==hash),
            ) {
                for field in ["MinimumGameEra", "MaximumGameEra"] {
                    restrictions |=
                        !matches!(row.get(field).map_err(Error::Rules)?, SqlValue::Null);
                }
                for field in ["RequiresDarkAge", "RequiresGoldenAge"] {
                    restrictions |=
                        !matches!(row.get(field).map_err(Error::Rules)?, SqlValue::Integer(0));
                }
            }
            let mut governments = Vec::new();
            for row in table("Policy_GovernmentExclusives_XP2")?.rows().filter(
                |r| matches!(r.get("PolicyType"),Ok(SqlValue::Text(t)) if linux_type_hash(t)==hash),
            ) {
                match row.get("GovernmentType").map_err(Error::Rules)? {
                    SqlValue::Text(t) => governments.push(linux_type_hash(t)),
                    _ => return Err(Error::Unsupported("exclusive policy government")),
                }
            }
            entries.push(Policy {
                slot,
                restrictions,
                governments,
            });
        }
        let mut wildcard = Vec::new();
        for entry in slots.entries() {
            let row=table("GovernmentSlots")?.rows().find(|r|matches!(r.get("GovernmentSlotType"),Ok(SqlValue::Text(t)) if t==&entry.type_name)).ok_or(Error::Unsupported("GovernmentSlot row"))?;
            wildcard.push(match row.get("AllowsAnyPolicy").map_err(Error::Rules)? {
                SqlValue::Integer(0) => false,
                SqlValue::Integer(1) => true,
                _ => return Err(Error::Unsupported("AllowsAnyPolicy")),
            });
        }
        let mut government_unlocked_policy = Vec::new();
        for row in table("Governments")?.rows() {
            if let SqlValue::Text(policy) = row.get("PolicyToUnlock").map_err(Error::Rules)? {
                let SqlValue::Text(government) = row.get("GovernmentType").map_err(Error::Rules)?
                else {
                    return Err(Error::Unsupported("GovernmentType"));
                };
                government_unlocked_policy
                    .push((linux_type_hash(government), linux_type_hash(policy)));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            policies,
            entries,
            wildcard,
            government_unlocked_policy,
            unlocks: culture::Definitions::from_rules(state, rules, authority)
                .map_err(Error::Culture)?,
        })
    }
    pub fn policy_hash(&self, index: i32) -> Option<u32> {
        self.policies
            .at(index as u32)
            .map(|r| linux_type_hash(&r.type_name))
    }
    pub fn policy_index(&self, hash: u32) -> Option<i32> {
        self.policies.find_index(hash).map(|i| i as i32)
    }
}
#[derive(Default)]
pub struct Current<'a> {
    pub graph: Option<&'a culture::GameCulture>,
    pub technology: Option<(
        &'a choose_tech::Definitions,
        Option<&'a choose_tech::GameTechs>,
    )>,
    pub starting: Option<&'a culture::StartingContext>,
}
fn boolean<E>(array: &IndexedBools, hash: u32) -> Result<bool, Error<E>> {
    array
        .pairs
        .iter()
        .rfind(|p| p.definition_hash == hash)
        .map(|p| p.value)
        .ok_or(Error::Unsupported("current policy indexed flag"))
}
fn store<E>(array: &mut IndexedBools, hash: u32, value: bool) -> Result<(), Error<E>> {
    let p = array
        .pairs
        .iter_mut()
        .rfind(|p| p.definition_hash == hash)
        .ok_or(Error::Unsupported("current policy flag storage"))?;
    p.value = value;
    Ok(())
}
/// Actual CanSlotPolicy gates, including already-active cards and wildcard
/// slots. Unsupported age restrictions fail when reached, never become legal.
pub fn can_slot<E>(
    state: &LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
    policy: i32,
    index: i32,
) -> Result<bool, Error<E>> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("policy binding"));
    }
    let Some(hash) = defs.policy_hash(policy) else {
        return Ok(false);
    };
    let (_, c) = state.research_components(owner).map_err(Error::State)?;
    if boolean(&c.policy_banned, hash)? || boolean(&c.policy_is_in_place, hash)? {
        return Ok(false);
    }
    let technology = current
        .technology
        .map(|(defs, graph)| defs.player(state, owner, graph))
        .transpose()
        .map_err(|error| Error::Culture(culture::Error::Tech(error)))?;
    if !defs
        .unlocks
        .player(
            state,
            owner,
            current.graph,
            technology.as_ref(),
            current.starting,
        )
        .map_err(Error::Culture)?
        .policy_unlocked(policy)
        .map_err(Error::Culture)?
    {
        return Ok(false);
    }
    let p = &defs.entries[policy as usize];
    if p.restrictions {
        return Err(Error::Unsupported(
            "current age-restricted policy eligibility",
        ));
    }
    if defs
        .government_unlocked_policy
        .contains(&(c.current_government_hash, hash))
        || (!p.governments.is_empty() && !p.governments.contains(&c.current_government_hash))
    {
        return Ok(false);
    }
    let Some(slot) = usize::try_from(index)
        .ok()
        .and_then(|i| c.active_policies.entries.get(i))
    else {
        return Ok(false);
    };
    let Some(&any) = usize::try_from(slot.slot_ordinal)
        .ok()
        .and_then(|i| defs.wildcard.get(i))
    else {
        return Ok(false);
    };
    Ok(slot.slot_ordinal == p.slot || any)
}

pub trait Effects {
    type Error;
    fn event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: kernel::Event,
        definitions: &Definitions,
    ) -> Result<(), Self::Error>;
    /// The native command updates the human processor after its mutations.
    fn update_human_processor(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
pub struct Context<'a, 'b, E> {
    pub state: &'a mut LoadedState,
    pub owner: i32,
    pub definitions: &'a Definitions,
    pub current: &'a Current<'b>,
    pub effects: &'a mut E,
}
impl<E: Effects> Context<'_, '_, E> {
    fn culture_mut(&mut self) -> Result<&mut Culture, Error<E::Error>> {
        self.state
            .research_components_mut(self.owner)
            .map(|(_, c)| c)
            .map_err(Error::State)
    }
}
impl<E: Effects> kernel::Context for Context<'_, '_, E> {
    type Error = Error<E::Error>;
    fn can_slot(&mut self, p: i32, s: i32) -> Result<bool, Self::Error> {
        can_slot(self.state, self.definitions, self.current, self.owner, p, s)
    }
    fn policy_at(&self, index: i32) -> Result<Option<i32>, Self::Error> {
        let (_, c) = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| c.active_policies.entries.get(i))
            .map(|s| self.definitions.policy_index(s.policy_hash).unwrap_or(-1)))
    }
    fn set_enacted(&mut self, p: i32) -> Result<(), Self::Error> {
        let hash = self
            .definitions
            .policy_hash(p)
            .ok_or(Error::Unsupported("enacted policy"))?;
        store(&mut self.culture_mut()?.policy_has_been_enacted, hash, true)
    }
    fn set_active(&mut self, p: i32, b: bool) -> Result<(), Self::Error> {
        let hash = self
            .definitions
            .policy_hash(p)
            .ok_or(Error::Unsupported("active policy"))?;
        store(&mut self.culture_mut()?.policy_is_in_place, hash, b)
    }
    fn set_slot(&mut self, s: i32, p: i32) -> Result<(), Self::Error> {
        let hash = self.definitions.policy_hash(p).unwrap_or(u32::MAX);
        self.culture_mut()?
            .active_policies
            .entries
            .get_mut(s as usize)
            .ok_or(Error::Unsupported("policy slot index"))?
            .policy_hash = hash;
        Ok(())
    }
    fn event(&mut self, e: kernel::Event) -> Result<(), Self::Error> {
        self.effects
            .event(self.state, self.owner, e, self.definitions)
            .map_err(Error::Effects)
    }
    fn set_change_made(&mut self) -> Result<(), Self::Error> {
        let turn = self.state.retained().core.current_turn;
        self.culture_mut()?.turn_last_policy_change = turn;
        Ok(())
    }
    fn update_human_processor(&mut self) -> Result<(), Self::Error> {
        if self
            .state
            .player_slot_status(self.owner)
            .map_err(Error::State)?
            == 3
        {
            self.effects
                .update_human_processor(self.state, self.owner)
                .map_err(Error::Effects)?;
        }
        Ok(())
    }
}
