//! Explicit government transitions over the current Culture and modifier owners.
//! Available choices and source-ordered mutations are supplied by this module;
//! unknown positive effects and anarchy remain incomplete transitions.
pub mod boosts;
pub mod current;
pub mod history;
pub mod modifiers;
pub mod requirements;
pub mod stock;

use super::{culture_notifications as culture, policy_actions};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    Rules, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Debug)]
pub enum Error<E = std::convert::Infallible> {
    State(c6_core::loaded_state::Error),
    Rules(crate::Error),
    Culture(culture::Error),
    Policy(policy_actions::Error<E>),
    Effects(E),
    IllegalGovernment(i32),
    Unsupported(&'static str),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "government action: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
struct Government {
    hash: u32,
    prerequisite: Option<i32>,
    tier: Option<u32>,
    slots: Vec<i32>,
}
pub struct Definitions {
    binding: WireBinding,
    registry: LinuxDefinitionRegistry,
    governments: Vec<Government>,
    policies: policy_actions::Definitions,
    culture: culture::Definitions,
    policy_costs: Vec<Option<i32>>,
    base_anarchy: Option<i32>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        use crate::unit_initialization_definitions::collection;
        let registry = collection(rules, "Governments", "GovernmentType")
            .map_err(|_| Error::Unsupported("Government registry"))?;
        let civics = collection(rules, "Civics", "CivicType")
            .map_err(|_| Error::Unsupported("Civic registry"))?;
        let slot_types = collection(rules, "GovernmentSlots", "GovernmentSlotType")
            .map_err(|_| Error::Unsupported("GovernmentSlot registry"))?;
        let policies = policy_actions::Definitions::from_rules(state, rules, authority)
            .map_err(Error::Policy)?;
        let culture =
            culture::Definitions::from_rules(state, rules, authority).map_err(Error::Culture)?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(Error::Unsupported("government rules table"))
        };
        let mut governments = vec![];
        for entry in registry.entries() {
            let row = table("Governments")?
                .rows()
                .find(|r| {
                    r.text("GovernmentType")
                        .ok()
                        .is_some_and(|v| v.as_bytes() == entry.type_name)
                })
                .ok_or(Error::Unsupported("Government row"))?;
            let reference = |field| -> Result<Option<u32>, Error> {
                match row.get(field).map_err(Error::Rules)? {
                    SqlValue::Null => Ok(None),
                    SqlValue::Text(t) => Ok(Some(linux_type_hash(t))),
                    _ => Err(Error::Unsupported("Government reference")),
                }
            };
            let prerequisite = reference("PrereqCivic")?
                .map(|h| {
                    civics
                        .find_index(h)
                        .map(|i| i as i32)
                        .ok_or(Error::Unsupported("government prerequisite Civic"))
                })
                .transpose()?;
            let mut counts = table("Government_SlotCounts")?
                .rows()
                .filter(|r| {
                    r.text("GovernmentType")
                        .ok()
                        .is_some_and(|v| v.as_bytes() == entry.type_name)
                })
                .collect::<Vec<_>>();
            // GovernmentGovernmentSlotsReferencePropertyBuilder22949a0:
            // SELECT T1.rowid ... ORDER BY T1.rowid ASC.
            if counts.iter().any(|r| r.row_id().is_none()) {
                return Err(Error::Unsupported("Government slot intrinsic rowid"));
            }
            counts.sort_by_key(|r| r.row_id().unwrap().0);
            let mut slots = vec![];
            for r in counts {
                let name = r.text("GovernmentSlotType").map_err(Error::Rules)?;
                let slot = slot_types
                    .find_index(linux_type_hash(name.as_bytes()))
                    .ok_or(Error::Unsupported("government slot reference"))?
                    as i32;
                let count =
                    crate::game_parameters::sql_integer(r.get("NumSlots").map_err(Error::Rules)?)
                        .ok_or(Error::Unsupported("government NumSlots"))?;
                for _ in 0..count {
                    slots.push(slot);
                }
            }
            governments.push(Government {
                hash: linux_type_hash(&entry.type_name),
                prerequisite,
                tier: reference("Tier")?,
                slots,
            });
        }
        let mut policy_costs = vec![];
        for row in table("Policies")?.rows() {
            let cost = match row.get("PrereqCivic").map_err(Error::Rules)? {
                SqlValue::Null => None,
                SqlValue::Text(name) => Some(
                    table("Civics")?
                        .rows()
                        .find(|r| {
                            r.text("CivicType")
                                .ok()
                                .is_some_and(|v| v.as_bytes() == name)
                        })
                        .ok_or(Error::Unsupported("policy prerequisite row"))?
                        .integer("Cost")
                        .map_err(Error::Rules)?
                        .try_into()
                        .map_err(|_| Error::Unsupported("Civic Cost i32"))?,
                ),
                _ => return Err(Error::Unsupported("policy prerequisite")),
            };
            policy_costs.push(cost);
        }
        let base_anarchy = table("GlobalParameters")?
            .rows()
            .find(|r| r.text("Name").ok() == Some("GOVERNMENT_BASE_ANARCHY_TURNS"))
            .map(|r| {
                r.get("Value").map_err(Error::Rules).and_then(|v| {
                    crate::game_parameters::sql_integer(v)
                        .ok_or(Error::Unsupported("base anarchy parameter"))
                })
            })
            .transpose()?;
        Ok(Self {
            binding: state.binding().clone(),
            registry,
            governments,
            policies,
            culture,
            policy_costs,
            base_anarchy,
        })
    }
    pub fn government_count(&self) -> usize {
        self.governments.len()
    }
    pub fn government_hash(&self, index: i32) -> Option<u32> {
        self.governments.get(index as usize).map(|g| g.hash)
    }
    pub fn government_index(&self, hash: u32) -> Option<i32> {
        self.registry.find_index(hash).map(|i| i as i32)
    }
    pub fn government_tier(&self, index: i32) -> Option<u32> {
        self.governments.get(index as usize).and_then(|g| g.tier)
    }
    pub fn policy_definitions(&self) -> &policy_actions::Definitions {
        &self.policies
    }
}

pub struct Current<'a> {
    pub policy: policy_actions::Current<'a>,
}
/// Native CanChangeGovernment2a743f0 checks definition/current identity first,
/// then HasGovernment or the live prerequisite Civic. It does not choose one.
pub fn can_change<E>(
    state: &LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
    index: i32,
) -> Result<bool, Error<E>> {
    if state.binding() != &defs.binding {
        return Err(Error::Unsupported("government binding"));
    }
    let Some(g) = usize::try_from(index)
        .ok()
        .and_then(|i| defs.governments.get(i))
    else {
        return Ok(false);
    };
    let (_, c) = state.research_components(owner).map_err(Error::State)?;
    if defs.registry.find_index(c.current_government_hash) == Some(index as usize) {
        return Ok(false);
    }
    let unlocked = c
        .government_unlocked
        .pairs
        .iter()
        .rfind(|p| p.definition_hash == g.hash)
        .map(|p| p.value)
        .ok_or(Error::Unsupported("current government flag"))?;
    if unlocked {
        return Ok(true);
    }
    let Some(prerequisite) = g.prerequisite else {
        return Ok(true);
    };
    defs.culture
        .player(
            state,
            owner,
            current.policy.graph,
            None,
            current.policy.starting,
        )
        .map_err(Error::Culture)?
        .has_civic(prerequisite)
        .map_err(Error::Culture)
}
pub fn available<E>(
    state: &LoadedState,
    defs: &Definitions,
    current: &Current<'_>,
    owner: i32,
) -> Result<Vec<i32>, Error<E>> {
    let mut result = vec![];
    for i in 0..defs.government_count() {
        if can_change(state, defs, current, owner, i as i32)? {
            result.push(i as i32);
        }
    }
    Ok(result)
}

pub trait Effects: policy_actions::Effects {
    fn government_event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: c6_core::government_policy::government::Event,
        definitions: &Definitions,
    ) -> Result<(), Self::Error>;
    fn slots_rebuilt(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Self::Error>;
    fn government_notifications(
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
    fn culture_mut(&mut self) -> Result<&mut c6_core::player_culture::Culture, Error<E::Error>> {
        self.state
            .research_components_mut(self.owner)
            .map(|(_, c)| c)
            .map_err(Error::State)
    }
    fn policy(&mut self) -> policy_actions::Context<'_, '_, E> {
        policy_actions::Context {
            state: self.state,
            owner: self.owner,
            definitions: &self.definitions.policies,
            current: &self.current.policy,
            effects: self.effects,
        }
    }
}
impl<E: Effects> c6_core::government_policy::Context for Context<'_, '_, E> {
    type Error = Error<E::Error>;
    fn can_slot(&mut self, p: i32, s: i32) -> Result<bool, Self::Error> {
        c6_core::government_policy::Context::can_slot(&mut self.policy(), p, s)
            .map_err(Error::Policy)
    }
    fn policy_at(&self, s: i32) -> Result<Option<i32>, Self::Error> {
        let (_, c) = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?;
        Ok(usize::try_from(s)
            .ok()
            .and_then(|i| c.active_policies.entries.get(i))
            .map(|s| {
                self.definitions
                    .policies
                    .policy_index(s.policy_hash)
                    .unwrap_or(-1)
            }))
    }
    fn set_enacted(&mut self, p: i32) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::set_enacted(&mut self.policy(), p)
            .map_err(Error::Policy)
    }
    fn set_active(&mut self, p: i32, b: bool) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::set_active(&mut self.policy(), p, b)
            .map_err(Error::Policy)
    }
    fn set_slot(&mut self, s: i32, p: i32) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::set_slot(&mut self.policy(), s, p)
            .map_err(Error::Policy)
    }
    fn event(&mut self, event: c6_core::government_policy::Event) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::event(&mut self.policy(), event).map_err(Error::Policy)
    }
    fn set_change_made(&mut self) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::set_change_made(&mut self.policy())
            .map_err(Error::Policy)
    }
    fn update_human_processor(&mut self) -> Result<(), Self::Error> {
        c6_core::government_policy::Context::update_human_processor(&mut self.policy())
            .map_err(Error::Policy)
    }
}
impl<E: Effects> c6_core::government_policy::government::Context for Context<'_, '_, E> {
    fn valid_government(&self, g: i32) -> bool {
        self.definitions.government_hash(g).is_some()
    }
    fn current_government(&self) -> Result<i32, Self::Error> {
        let (_, c) = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?;
        Ok(self
            .definitions
            .government_index(c.current_government_hash)
            .unwrap_or(-1))
    }
    fn government_event(
        &mut self,
        event: c6_core::government_policy::government::Event,
    ) -> Result<(), Self::Error> {
        self.effects
            .government_event(self.state, self.owner, event, self.definitions)
            .map_err(Error::Effects)
    }
    fn store_previous_policies(&mut self) -> Result<(), Self::Error> {
        let policies = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?
            .1
            .active_policies
            .entries
            .clone();
        let mut previous = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?
            .1
            .previous_government_policies
            .entries
            .clone();
        if !policies.is_empty() {
            previous.clear();
        }
        previous.extend(policies);
        // PreviousGovernmentPolicySort2a9aed0: valid cards first; among cards
        // with Civic prerequisites, descending raw Civic Cost. Equal cards do
        // not swap in the source small-vector insertion path.
        let cost = |hash| {
            self.definitions
                .policies
                .policy_index(hash)
                .and_then(|i| self.definitions.policy_costs.get(i as usize).copied())
                .flatten()
        };
        previous.sort_by(|a, b| {
            if a.policy_hash == u32::MAX || b.policy_hash == u32::MAX {
                return match (a.policy_hash == u32::MAX, b.policy_hash == u32::MAX) {
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    _ => std::cmp::Ordering::Equal,
                };
            }
            cost(b.policy_hash).cmp(&cost(a.policy_hash))
        });
        self.culture_mut()?.previous_government_policies.entries = previous;
        Ok(())
    }
    fn slot_count(&self) -> Result<usize, Self::Error> {
        Ok(self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?
            .1
            .active_policies
            .entries
            .len())
    }
    fn clear_slots(&mut self) -> Result<(), Self::Error> {
        self.culture_mut()?.active_policies.entries.clear();
        Ok(())
    }
    fn reset_policy_change(&mut self) -> Result<(), Self::Error> {
        self.culture_mut()?.turn_last_policy_change = -1;
        Ok(())
    }
    fn record_government_tier(&mut self, g: i32) -> Result<(), Self::Error> {
        let def = self
            .definitions
            .governments
            .get(g as usize)
            .ok_or(Error::Unsupported("government tier"))?;
        let Some(tier) = def.tier else {
            return Ok(());
        };
        let hash = def.hash;
        self.culture_mut()?
            .last_government_used
            .iter_mut()
            .rfind(|p| p.definition_hash == tier)
            .ok_or(Error::Unsupported("current government tier history"))?
            .value = hash;
        Ok(())
    }
    fn anarchy_turns(&self, g: i32) -> Result<i32, Self::Error> {
        let hash = self
            .definitions
            .government_hash(g)
            .ok_or(Error::Unsupported("anarchy government"))?;
        let (_, c) = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?;
        let count = c
            .government_usage_count
            .iter()
            .rfind(|p| p.definition_hash == hash)
            .ok_or(Error::Unsupported("current government usage"))?
            .value as u8;
        if count == 0 {
            return Ok(0);
        }
        Ok(self
            .definitions
            .base_anarchy
            .ok_or(Error::Unsupported("base anarchy turns"))?
            .wrapping_add(i32::from(count)))
    }
    fn set_government(&mut self, g: i32) -> Result<(), Self::Error> {
        let hash = self.definitions.government_hash(g).unwrap_or(u32::MAX);
        self.culture_mut()?.current_government_hash = hash;
        Ok(())
    }
    fn set_post_anarchy_government(&mut self, g: i32) -> Result<(), Self::Error> {
        let hash = self
            .definitions
            .government_hash(g)
            .ok_or(Error::Unsupported("post-anarchy government"))?;
        self.culture_mut()?.post_anarchy_government_hash = hash;
        Ok(())
    }
    fn enter_anarchy(&mut self, _: i32, _: i32, _: i32) -> Result<(), Self::Error> {
        Err(Error::Unsupported(
            "positive government anarchy continuation",
        ))
    }
    fn mark_government_used(&mut self, g: i32) -> Result<(), Self::Error> {
        let hash = self
            .definitions
            .government_hash(g)
            .ok_or(Error::Unsupported("used government"))?;
        self.culture_mut()?
            .government_usage_count
            .iter_mut()
            .rfind(|p| p.definition_hash == hash)
            .ok_or(Error::Unsupported("current government usage storage"))?
            .value = 1;
        Ok(())
    }
    fn set_turn_adopted(&mut self) -> Result<(), Self::Error> {
        let turn = self.state.retained().core.current_turn;
        self.culture_mut()?.turn_adopted = turn;
        Ok(())
    }
    fn rebuild_slots(&mut self, g: i32) -> Result<(), Self::Error> {
        use c6_core::player_culture::GovernmentSlot;
        let c = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?
            .1;
        if !c.modified_slots.entries.is_empty()
            || !c.modified_removed_slots.entries.is_empty()
            || !c.replaced_slots.is_empty()
            || !c.government_type_modified_slots.is_empty()
        {
            return Err(Error::Unsupported("modified government slot rebuild"));
        }
        let slots = self
            .definitions
            .governments
            .get(g as usize)
            .ok_or(Error::Unsupported("government slots"))?
            .slots
            .clone();
        self.culture_mut()?
            .active_policies
            .entries
            .extend(slots.into_iter().map(|slot_ordinal| GovernmentSlot {
                policy_hash: u32::MAX,
                slot_ordinal,
            }));
        let count = self
            .state
            .research_components(self.owner)
            .map_err(Error::State)?
            .1
            .active_policies
            .entries
            .len();
        for index in 0..count {
            let c = self
                .state
                .research_components(self.owner)
                .map_err(Error::State)?
                .1;
            if c.active_policies
                .entries
                .iter()
                .filter(|s| {
                    self.definitions
                        .policies
                        .policy_index(s.policy_hash)
                        .is_none()
                })
                .count()
                < 2
            {
                break;
            }
            let slot = c.active_policies.entries[index].slot_ordinal;
            let found = c.previous_government_policies.entries.iter().position(|s| {
                s.slot_ordinal == slot
                    && self
                        .definitions
                        .policies
                        .policy_index(s.policy_hash)
                        .is_some()
            });
            let Some(previous) = found else {
                continue;
            };
            let hash = c.previous_government_policies.entries[previous].policy_hash;
            let policy = self
                .definitions
                .policies
                .policy_index(hash)
                .ok_or(Error::Unsupported("previous policy"))?;
            c6_core::government_policy::Context::set_active(self, policy, true)?;
            self.culture_mut()?.previous_government_policies.entries[previous] = GovernmentSlot {
                policy_hash: u32::MAX,
                slot_ordinal: -1,
            };
            let government_hash = self
                .definitions
                .government_hash(g)
                .ok_or(Error::Unsupported("rebuilt government"))?;
            if policy_actions::eligible_in_government(
                self.state,
                &self.definitions.policies,
                &self.current.policy,
                self.owner,
                policy,
                government_hash,
            )
            .map_err(Error::Policy)?
            {
                self.culture_mut()?.active_policies.entries[index].policy_hash = hash;
            }
        }
        self.effects
            .slots_rebuilt(self.state, self.owner)
            .map_err(Error::Effects)
    }
    fn check_government_notifications(&mut self) -> Result<(), Self::Error> {
        self.effects
            .government_notifications(self.state, self.owner)
            .map_err(Error::Effects)
    }
}
