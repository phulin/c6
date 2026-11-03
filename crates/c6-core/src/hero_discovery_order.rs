//! Pinned Linux Heroes discovery-order planning, bounded to 28 output records.
//!
//! This module has no production configuration/definition/eligibility provider and
//! does not admit a loaded-state bridge. A saved Heroes record is not such a provider. In
//! particular, arbitrary provider booleans are not captured native authority.
//! Missing inputs must fail before a caller commits any gameplay/archive effects.
use crate::player_heroes::{DiscoveryWeight, Heroes};

pub const CAPABILITY: u32 = 0x82ef_2304;
pub const MAX_INSERTION_SORT_RECORDS: usize = 28;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Missing(&'static str),
    InvalidDefinitionIdentity,
    SortUnsupported { records: usize },
    InvalidRandomResult(u16),
    ResourceLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeroDefinition {
    pub native_index: i32,
    pub type_hash: u32,
    pub discovery_min_era_chronology: Option<i32>,
}

/// Query results from the actual city methods, in native city iteration order.
/// Synthetic instances test arithmetic only; they do not admit live eligibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CityClaimQueries {
    pub has_prerequisite_district: bool,
    pub can_build_unit_domain: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimInputs {
    pub valid_class: bool,
    pub claim_player: i32,
    pub creation_project_exists: bool,
    pub unit_domain: Option<i32>,
    pub prerequisite_district_exists: bool,
    pub city_queries: Vec<CityClaimQueries>,
}

impl ClaimInputs {
    /// CouldClaimHeroNow at 0x269c900. District/domain witnesses may be
    /// different cities. Domain 2 starts with domain support already true.
    pub fn could_claim_now(&self) -> bool {
        if !self.valid_class || self.claim_player != -1 || !self.creation_project_exists {
            return false;
        }
        let Some(domain) = self.unit_domain else {
            return false;
        };
        let mut district = !self.prerequisite_district_exists;
        let mut domain_supported = domain == 2;
        for city in &self.city_queries {
            if district && domain_supported {
                break;
            }
            district |= self.prerequisite_district_exists && city.has_prerequisite_district;
            domain_supported |= domain != 2 && city.can_build_unit_domain;
        }
        district && domain_supported
    }
}

/// Fallible source queries. Implementations must supply exact target-version
/// configuration and native registry order/identity, not SQL row order or
/// guessed defaults. No production implementation is admitted by this module.
pub trait Inputs {
    fn is_major(&self) -> Result<bool, Error>;
    fn capability_present(&self, capability: u32) -> Result<bool, Error>;
    fn any_multiplayer(&self) -> Result<bool, Error>;
    fn is_human(&self) -> Result<bool, Error>;
    fn local_player(&self) -> Result<i32, Error>;
    fn allowance(&self, capability: u32) -> Result<u32, Error>;
    /// None means the native current-era lookup returned null, not unknown.
    fn current_era_chronology(&self) -> Result<Option<i32>, Error>;
    fn ordered_hero_definitions(&self) -> Result<Vec<HeroDefinition>, Error>;
    fn claim_inputs(&self, native_index: i32) -> Result<ClaimInputs, Error>;
}

/// A cloneable preview of the caller's owned Game RNG. `draw` must use the
/// exact stream/counters and preserve locks and other state. Cloning/queries/
/// draws must not mutate external state. Native wrapper edits are returned as
/// ordered diagnostic effects, not prerequisites for local field mutation.
/// The wire Random record lacks live locks and has no implementation here.
pub trait GameRandomOwner: Clone {
    fn draw(&mut self, bound: u16) -> Result<u16, Error>;
}

/// Caller must establish that this is its actual owned Game sync stream.
/// Implementing arithmetic does not establish stream identity or loaded-state
/// authority. Linux independently validates this primitive; see linux-random.md.
impl GameRandomOwner for crate::rng::RandomState {
    fn draw(&mut self, bound: u16) -> Result<u16, Error> {
        Ok(crate::rng::RandomState::draw(self, bound))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    /// Even clearing an empty order edits the archive wrapper (+0x148).
    EditAndClearDiscoveryOrder,
    EditGameRandom,
    GameRandomDraw {
        native_index: i32,
        bound: u16,
    },
    Append {
        native_index: i32,
        type_hash: u32,
        weight: i32,
    },
    SortDescending,
    LogRebuiltOrder,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan<R> {
    pub discovery_order: Vec<DiscoveryWeight>,
    pub random_after: R,
    pub effects: Vec<Effect>,
}

/// Validate(false) bypasses rebuilding exactly when the saved order is nonempty.
/// Caller must separately establish alive-major manager iteration membership and
/// successful EditValidPlayer resolution; this does not establish either fact.
pub fn needs_rebuild(heroes: &Heroes, force: bool) -> bool {
    force || heroes.discovery_order.is_empty()
}

fn allowed(inputs: &impl Inputs) -> Result<bool, Error> {
    if !inputs.capability_present(CAPABILITY)? {
        return Ok(false);
    }
    if inputs.any_multiplayer()? {
        if !inputs.is_human()? {
            return Ok(true);
        }
        return Ok(inputs.allowance(CAPABILITY)?.wrapping_sub(1) < 2);
    }
    if inputs.local_player()? == -1 {
        return Ok(true);
    }
    Ok(inputs.allowance(CAPABILITY)? == 1)
}

/// Prepare exact ordered effects without changing Heroes or the supplied owner.
/// Applying the returned plan requires admitted live values and native hash/index
/// remapping. Native archive pointer histories are diagnostic effects. No
/// production configuration/definition/eligibility bridge is supplied here.
/// No complete live rebuild/action/turn acceptance is claimed.
pub fn plan_rebuild<R: GameRandomOwner>(
    heroes: &Heroes,
    inputs: &impl Inputs,
    random: &R,
) -> Result<Plan<R>, Error> {
    let mut plan = Plan {
        discovery_order: Vec::new(),
        random_after: random.clone(),
        effects: vec![Effect::EditAndClearDiscoveryOrder],
    };
    if !inputs.is_major()? || !allowed(inputs)? {
        return Ok(plan);
    }
    let Some(current_era) = inputs.current_era_chronology()? else {
        return Ok(plan);
    };
    let definitions = inputs.ordered_hero_definitions()?;
    for (i, definition) in definitions.iter().enumerate() {
        if definition.native_index < 0
            || definitions[..i].iter().any(|other| {
                other.native_index == definition.native_index
                    || other.type_hash == definition.type_hash
            })
        {
            return Err(Error::InvalidDefinitionIdentity);
        }
    }
    let undiscovered = definitions
        .iter()
        .filter(|definition| {
            !heroes
                .discovered
                .iter()
                .any(|entry| entry.hero_class_hash == definition.type_hash && entry.discovered)
        })
        .count();
    if undiscovered > MAX_INSERTION_SORT_RECORDS {
        return Err(Error::SortUnsupported {
            records: undiscovered,
        });
    }
    plan.discovery_order
        .try_reserve_exact(undiscovered)
        .map_err(|_| Error::ResourceLimit)?;
    for definition in definitions {
        if heroes
            .discovered
            .iter()
            .any(|entry| entry.hero_class_hash == definition.type_hash && entry.discovered)
        {
            continue;
        }
        let claim = inputs.claim_inputs(definition.native_index)?;
        let eligible = claim.could_claim_now()
            && definition
                .discovery_min_era_chronology
                .is_none_or(|minimum| minimum <= current_era);
        let weight = if eligible {
            plan.effects.push(Effect::EditGameRandom);
            let draw = plan.random_after.draw(20)?;
            if draw >= 20 {
                return Err(Error::InvalidRandomResult(draw));
            }
            plan.effects.push(Effect::GameRandomDraw {
                native_index: definition.native_index,
                bound: 20,
            });
            100i32.wrapping_add(i32::from(draw)).wrapping_sub(10)
        } else {
            0
        };
        plan.discovery_order.push(DiscoveryWeight {
            hero_class_hash: definition.type_hash,
            weight,
        });
        plan.effects.push(Effect::Append {
            native_index: definition.native_index,
            type_hash: definition.type_hash,
            weight,
        });
    }
    // Native ≤28 branch uses strict signed-greater insertion sort: ties retain
    // definition collection order. Larger native introsort is not ported here.
    for i in 1..plan.discovery_order.len() {
        let mut j = i;
        while j > 0 && plan.discovery_order[j].weight > plan.discovery_order[j - 1].weight {
            plan.discovery_order.swap(j, j - 1);
            j -= 1;
        }
    }
    plan.effects.push(Effect::SortDescending);
    plan.effects.push(Effect::LogRebuiltOrder);
    Ok(plan)
}

/// Commit both owned Heroes order and the caller's actual owned Game RNG only
/// after every required query, allocation, identity and sort check succeeds.
/// Other Heroes fields and RNG fields stay intact. Effects retain native edit,
/// append, sort and logger order for diagnostics; no native Save is produced.
pub fn rebuild<R: GameRandomOwner>(
    heroes: &mut Heroes,
    inputs: &impl Inputs,
    game_random: &mut R,
) -> Result<Vec<Effect>, Error> {
    let plan = plan_rebuild(heroes, inputs, game_random)?;
    heroes.discovery_order = plan.discovery_order;
    *game_random = plan.random_after;
    Ok(plan.effects)
}
