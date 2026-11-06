//! Reached Linux military-capture Emergency construction and temporary lifetime.
//! Requirement IDs, states and children use the canonical requirement owner;
//! active Emergency instances remain in the ordinary Congress manager.
use crate::{
    game_update_state::activation::cityless_managers,
    melee_attack::backend::{self, ContextObject},
};
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
    unit::ComponentId,
    unit_moved_signal::{ListenerRing, Subscription},
};

#[derive(Debug)]
pub struct Pending {
    pub target: i32,
    pub emergency_type: i32,
    pub data: Vec<u32>,
    pub data_type: u32,
    pub string_data: Vec<Vec<u8>>,
}
/// Current native PlayerTurnStarted(+990) and friendship(+d20) signal owners.
/// Existing logical listener identities retain their order while temporary
/// requirements append and detach during the synchronous capture callback.
pub struct Current {
    pub pending: Vec<Pending>,
    turn_started: ListenerRing<u64>,
    friendship: ListenerRing<u64>,
    subscriptions: Vec<(u64, Subscription, Option<Subscription>)>,
}
impl Current {
    pub fn turn_started_listeners(&self) -> impl Iterator<Item = u64> + '_ {
        self.turn_started.ordered_subscribers()
    }
    pub fn friendship_listeners(&self) -> impl Iterator<Item = u64> + '_ {
        self.friendship.ordered_subscribers()
    }
    pub fn from_initial_empty(turn_started: &[u64], friendship: &[u64]) -> Result<Self, String> {
        Ok(Self {
            pending: Vec::new(),
            turn_started: ListenerRing::from_ordered(turn_started).map_err(debug)?,
            friendship: ListenerRing::from_ordered(friendship).map_err(debug)?,
            subscriptions: Vec::new(),
        })
    }
}
pub struct Definition {
    pub collection_index: i32,
    pub type_hash: u32,
    pub target_set: Option<Vec<u8>>,
    pub member_set: Vec<u8>,
    pub hostile: bool,
    pub xp2_definition_present: bool,
}
/// Progress routes contain canonical requirement labels and definition weights,
/// never a precomputed native GetProgress or target-eligibility result.
pub enum Progress {
    SimpleTestAll { set: u64, children: Vec<(u64, i32)> },
    Unsupported,
}
pub struct TeamVictory {
    pub team: i32,
    pub victory_index: i32,
    pub progress: Progress,
}
pub struct Inputs {
    pub binding: c6_core::core_player::initialization::WireBinding,
    pub definition: Definition,
    pub alive_majors: Vec<i32>,
    pub victory_indices: Vec<i32>,
    pub victories: Vec<TeamVictory>,
    /// Already-live primary Player wrappers in the shared ObjectPool.
    pub player_objects: Vec<(i32, u64)>,
    /// Complete current Emergency ObjectPool index. A live collision needs its
    /// strong lifetime imported; unknown membership is not a constructor miss.
    pub emergency_objects_empty: bool,
    pub logging_priority: i32,
}
pub struct Context<'a> {
    pub manager: &'a mut cityless_managers::Runtime,
    pub effects: &'a mut backend::EffectRuntime,
    pub requirements: &'a mut backend::requirements::Runtime,
    pub inputs: &'a Inputs,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
}
fn debug(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn met(state: i32) -> bool {
    state | 2 == 2
}
fn progress(sets: &sets::Runtime, value: &Progress) -> Result<(i32, i32), String> {
    let Progress::SimpleTestAll { set, children } = value else {
        return Err("reached additional victory progress implementation".into());
    };
    let state = sets.state(*set).ok_or("current victory requirement set")?;
    if met(state) {
        return Ok((1, 1));
    }
    let actual = sets.children(*set).ok_or("current victory children")?;
    if !actual.iter().copied().eq(children.iter().map(|r| r.0)) {
        return Err("current victory progress child order changed".into());
    }
    if children.is_empty() {
        return Ok((0, 1));
    }
    let leaf = |id| -> Result<i32, String> {
        let r = sets
            .simple_requirement(id)
            .ok_or("current Simple victory progress leaf")?;
        Ok(i32::from(met(r.state)))
    };
    if children.len() == 1 {
        return Ok((leaf(children[0].0)?, 1));
    }
    let (mut completed, mut total) = (0i32, 0i32);
    for &(child, weight) in children {
        if weight > 0 {
            // Simple.GetProgress returns IsMet/1; native TestAll scales each
            // positive-weight child to100 using double before integer truncation.
            completed = completed.wrapping_add(leaf(child)?.wrapping_mul(100).wrapping_mul(weight));
            total = total.wrapping_add(weight.wrapping_mul(100));
        }
    }
    Ok((completed, total))
}
fn winning(
    state: &LoadedState,
    requirements: &sets::Runtime,
    inputs: &Inputs,
    target: i32,
) -> Result<i32, String> {
    let target_team = state.player_team(target).map_err(debug)?;
    for &victory in &inputs.victory_indices {
        let get = |team| {
            let route = inputs
                .victories
                .iter()
                .find(|r| r.team == team && r.victory_index == victory)
                .ok_or_else(|| "current VictoryManager progress route".to_owned())?;
            progress(requirements, &route.progress)
        };
        let (done, total) = get(target_team)?;
        let ratio = done as f32 / total as f32;
        let mut highest = true;
        for &other in &inputs.alive_majors {
            let team = state.player_team(other).map_err(debug)?;
            if team == target_team {
                continue;
            }
            let (done, total) = get(team)?;
            // UCOMISS/JAE: unordered fractions do not reject the target.
            if ratio <= done as f32 / total as f32 {
                highest = false;
                break;
            }
        }
        if highest {
            return Ok(0);
        }
    }
    Ok(1)
}
struct Factory<'a> {
    state: &'a LoadedState,
    requirements: &'a mut backend::requirements::Runtime,
    effects: &'a backend::EffectRuntime,
    inputs: &'a Inputs,
    current: &'a mut Current,
    target: i32,
}
impl sets::SubjectContext for Factory<'_> {
    type Error = String;
    fn subject_sets(&self) -> Result<&sets::Runtime, String> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, String> {
        Ok(self.requirements.sets_mut())
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        d: &sets::RequirementDefinition,
        _: i32,
        subject: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<String>> {
        let run = || -> Result<(bool, i32), String> {
            let sets::Subject::ModifierObject(subject) = subject else {
                return Err("Emergency subject interface".into());
            };
            let ContextObject::Player(player) = self.effects.object(subject).map_err(debug)?.kind
            else {
                return Err("Emergency subject IPlayer".into());
            };
            let sets::Predicate::Unsupported(kind) = d.predicate else {
                return Err("Emergency requirement concrete class".into());
            };
            if kind == 0xfeb3_e4c3 {
                return Ok((
                    true,
                    winning(self.state, self.requirements.sets(), self.inputs, player)?,
                ));
            }
            if kind == 0xd58b_7e5c {
                return Ok((false, if player == self.target { 2 } else { 3 }));
            }
            if kind == 0xa6b7_6afd {
                let p = self
                    .state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == player)
                    .ok_or("current Emergency member Cities")?;
                let pending = self
                    .current
                    .pending
                    .last()
                    .ok_or("current pending Emergency")?;
                let mut result = 1;
                for &plot in &pending.data {
                    let continent = self
                        .state
                        .world()
                        .map()
                        .plot_by_index(plot)
                        .ok_or("pending Emergency data Plot")?
                        .continent_hash;
                    if p.cities.settled_continent_hashes.contains(&continent)
                        && !p.cities.records.entries.is_empty()
                    {
                        result = 0;
                        break;
                    }
                }
                return Ok((true, result));
            }
            let relations = self.state.game_relations().map_err(debug)?;
            if kind == 0x5e3f_5ccc {
                return Ok((
                    true,
                    i32::from(
                        !relations
                            .has_declared_friendship(player, self.target)
                            .map_err(debug)?,
                    ),
                ));
            }
            if kind == 0x9d70_8111 {
                return Ok((
                    true,
                    i32::from(!relations.is_allied(player, self.target).map_err(debug)?),
                ));
            }
            Err(format!(
                "unsupported reached Emergency requirement {kind:08x}"
            ))
        };
        // Native derived OnInitialize subscribes BEFORE Simple's initial Test.
        let kind = match d.predicate {
            sets::Predicate::Unsupported(k) => k,
            _ => 0,
        };
        let constant = kind == 0xd58b_7e5c;
        if !constant {
            let turn = self
                .current
                .turn_started
                .subscribe(label)
                .map_err(sets::Error::Ring)?;
            let friend = if kind == 0x5e3f_5ccc {
                Some(
                    self.current
                        .friendship
                        .subscribe(label)
                        .map_err(sets::Error::Ring)?,
                )
            } else {
                None
            };
            self.current.subscriptions.push((label, turn, friend));
        }
        let (dynamic, tested) = run().map_err(sets::Error::Context)?;
        let value = if d.inverse {
            sets::inverse(tested)
        } else {
            tested
        };
        Ok(if dynamic {
            sets::InitializedRequirement::Simple(c6_core::simple_requirement::Requirement {
                active: true,
                state: value,
                inverse: d.inverse,
                persistent: d.persistent,
                triggered: false,
            })
        } else {
            sets::InitializedRequirement::Const(value)
        })
    }
    fn destroy_initial_simple_requirement(
        &mut self,
        label: u64,
    ) -> Result<(), sets::Error<String>> {
        <Self as changes::Context>::destroy_requirement(self, label)
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), String> {
        if self.inputs.logging_priority >= 1 {
            Err("Emergency missing-set logging".into())
        } else {
            Ok(())
        }
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), String> {
        if self.inputs.logging_priority >= 4 {
            Err("Emergency requirement logging".into())
        } else {
            Ok(())
        }
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), String> {
        if self.inputs.logging_priority >= 4 {
            Err("Emergency set logging".into())
        } else {
            Ok(())
        }
    }
}
impl changes::Context for Factory<'_> {
    type Error = String;
    fn subject_sets(&self) -> Result<&sets::Runtime, String> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, String> {
        Ok(self.requirements.sets_mut())
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, sets::Error<String>> {
        Err(sets::Error::Context(
            "unexpected Emergency invalidation".into(),
        ))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(
            "unexpected local Emergency set listener".into(),
        ))
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<String>> {
        if let Some(at) = self.current.subscriptions.iter().position(|r| r.0 == label) {
            let (_, turn, friend) = self.current.subscriptions.remove(at);
            self.current
                .turn_started
                .unsubscribe(turn)
                .map_err(sets::Error::Ring)?;
            if let Some(token) = friend {
                self.current
                    .friendship
                    .unsubscribe(token)
                    .map_err(sets::Error::Ring)?;
            }
        }
        Ok(())
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<String>> {
        Ok(())
    }
    fn reset_trigger(&mut self, _: u64) -> Result<(), sets::Error<String>> {
        Err(sets::Error::Context(
            "unexpected Emergency trigger reset".into(),
        ))
    }
}
impl super::callbacks::EmergencyCallbacks for Context<'_> {
    fn start_military(
        &mut self,
        state: &mut LoadedState,
        previous_owner: i32,
        city: ComponentId,
    ) -> Result<(), String> {
        if self.inputs.binding != *state.binding() {
            return Err("Emergency inputs binding".into());
        }
        if !self.capabilities.has(0x3705_21e0) {
            return Ok(());
        }
        if !self
            .manager
            .congress(state)
            .map_err(debug)?
            .emergencies
            .is_empty()
        {
            return Err("existing Emergency matching/start guards".into());
        }
        if !self.inputs.alive_majors.contains(&previous_owner) {
            return Err("military Emergency previous owner outside surviving-major scope".into());
        }
        let Some(set_id) = self
            .inputs
            .definition
            .target_set
            .as_deref()
            .filter(|s| !s.is_empty())
        else {
            return Ok(());
        };
        let target = i32::from(city.owner);
        if !self.inputs.alive_majors.contains(&target) {
            return Err("nonmajor Emergency target branch".into());
        }
        let native_city = state.city(target, city.id).map_err(debug)?;
        let map = state.world().map();
        let plot = (native_city.header.location_y as u32)
            .wrapping_mul(map.header.width as u32)
            .wrapping_add(native_city.header.location_x as u32);
        map.plot_by_index(plot)
            .ok_or("military Emergency City Plot")?;
        let pending = Pending {
            target,
            emergency_type: self.inputs.definition.collection_index,
            data: vec![plot],
            data_type: 0,
            string_data: vec![native_city
                .header
                .name
                .split(|byte| *byte == 0)
                .next()
                .unwrap_or_default()
                .to_vec()],
        };
        let current = self.manager.capture_emergency_mut(state).map_err(debug)?;
        if !current.pending.is_empty() {
            return Err("nested pending Emergency construction".into());
        }
        current.pending.push(pending);
        if !self.inputs.emergency_objects_empty {
            return Err("current Emergency wrapper cache collision/domain".into());
        }
        let object = self
            .effects
            .make_emergency_object(state, target, self.inputs.definition.collection_index, -1)
            .map_err(debug)?;
        if !self.inputs.definition.xp2_definition_present {
            current.pending.clear();
            return self.effects.release_emergency_object(object).map_err(debug);
        }
        let context = self.effects.object(object).map_err(debug)?.instance_id;
        let player_object = |owner| -> Result<u64, String> {
            let handle = self
                .inputs
                .player_objects
                .iter()
                .find(|r| r.0 == owner)
                .ok_or("current Player wrapper cache domain")?
                .1;
            if self.effects.object(handle).map_err(debug)?.kind != ContextObject::Player(owner) {
                return Err("current Emergency Player wrapper identity".into());
            }
            Ok(handle)
        };
        let subject = player_object(target)?;
        let mut factory = Factory {
            state,
            requirements: self.requirements,
            effects: self.effects,
            inputs: self.inputs,
            current,
            target,
        };
        let target_set = sets::get_or_create_unsubscribed_subject(
            &mut factory,
            context,
            sets::Subject::ModifierObject(subject),
            set_id,
        )
        .map_err(debug)?;
        let target_met =
            target_set.is_some_and(|s| factory.requirements.sets().state(s).is_some_and(met));
        if target_met {
            let mut members = Vec::new();
            for &owner in &self.inputs.alive_majors {
                let subject = self
                    .inputs
                    .player_objects
                    .iter()
                    .find(|r| r.0 == owner)
                    .ok_or("current member Player wrapper")?
                    .1;
                let set = sets::get_or_create_unsubscribed_subject(
                    &mut factory,
                    context,
                    sets::Subject::ModifierObject(subject),
                    &self.inputs.definition.member_set,
                )
                .map_err(debug)?;
                let has_met = state
                    .game_relations()
                    .map_err(debug)?
                    .has_met(owner, target)
                    .map_err(debug)?;
                if has_met
                    && set.is_some_and(|s| factory.requirements.sets().state(s).is_some_and(met))
                    && (state.player_team(owner).map_err(debug)?
                        != state.player_team(target).map_err(debug)?
                        || !self.inputs.definition.hostile)
                {
                    members.push(owner);
                }
                if let Some(set) = set {
                    changes::release(&mut factory, set).map_err(debug)?;
                }
            }
            if members.len() > 1 {
                return Err(
                    "military Emergency active-instance creation/invitation boundary".into(),
                );
            }
        }
        if let Some(set) = target_set {
            changes::release(&mut factory, set).map_err(debug)?;
        }
        factory.current.pending.clear();
        self.effects
            .release_emergency_object(object)
            .map_err(debug)?;
        Ok(())
    }
    fn goal(&mut self, state: &mut LoadedState, _: i32, _: ComponentId) -> Result<(), String> {
        if self
            .manager
            .congress(state)
            .map_err(debug)?
            .emergencies
            .is_empty()
        {
            Ok(())
        } else {
            Err("positive military Emergency goal".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn score_progress_reads_live_canonical_simple_states_and_ignores_zero_weight() {
        let mut runtime = sets::Runtime::from_initial(100, vec![], vec![]).unwrap();
        let make = |label, id: &[u8], state| changes::CurrentRequirement {
            label,
            native_id: label as u32,
            context: 7,
            subject: sets::Subject::ModifierObject(9),
            definition: sets::RequirementDefinition {
                id: id.to_vec(),
                predicate: sets::Predicate::Unsupported(1),
                inverse: false,
                persistent: false,
            },
            value: sets::InitializedRequirement::Simple(c6_core::simple_requirement::Requirement {
                active: true,
                state,
                inverse: false,
                persistent: false,
                triggered: false,
            }),
            references: 1,
            subscribers: vec![3],
        };
        runtime
            .import_current::<String>(
                vec![make(1, b"TURN_MAX", 1), make(2, b"HIGHEST_SCORE", 0)],
                vec![changes::CurrentSet {
                    label: 3,
                    native_id: 3,
                    context: 7,
                    subject: sets::Subject::ModifierObject(9),
                    definition: sets::SetDefinition {
                        id: b"SCORE".to_vec(),
                        requirements: vec![b"TURN_MAX".to_vec(), b"HIGHEST_SCORE".to_vec()],
                    },
                    state: 1,
                    references: 1,
                    children: vec![1, 2],
                    subscribers: vec![],
                }],
            )
            .unwrap();
        let route = Progress::SimpleTestAll {
            set: 3,
            children: vec![(1, 0), (2, 1)],
        };
        assert_eq!(progress(&runtime, &route).unwrap(), (100, 100));
        runtime.simple_requirement_mut(2).unwrap().state = 1;
        assert_eq!(progress(&runtime, &route).unwrap(), (0, 100));
        let reordered = Progress::SimpleTestAll {
            set: 3,
            children: vec![(2, 1), (1, 0)],
        };
        assert!(progress(&runtime, &reordered).is_err());
    }
}
