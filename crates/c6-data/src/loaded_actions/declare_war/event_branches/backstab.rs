//! Bounded negative Backstab StartEmergency target check. The temporary
//! Emergency and requirement objects consume their ordinary shared identities.
use super::{backstab_alliance_requirement, AllianceThresholds};
use crate::{melee_attack::backend, registry::linux_type_hash, Rules};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
    simple_requirement::Requirement,
};

pub struct Inputs {
    binding: WireBinding,
    emergency_type: i32,
    minimum_level: i32,
    target_set: Vec<u8>,
    child: sets::RequirementDefinition,
}
impl Inputs {
    /// The native collection index comes from the current definition provider.
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        emergency_type: i32,
    ) -> Result<Self, String> {
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or_else(|| format!("backstab {name}"))
        };
        let emergency = table("EmergencyAlliances")?
            .rows()
            .find(|r| r.text("EmergencyType").ok() == Some("EMERGENCY_BACKSTAB"))
            .ok_or("backstab definition")?;
        let target_set = emergency
            .text("TargetRequirementSet")
            .map_err(debug)?
            .as_bytes()
            .to_vec();
        let set_row = table("RequirementSets")?
            .rows()
            .find(|r| {
                r.text("RequirementSetId").ok().map(str::as_bytes) == Some(target_set.as_slice())
            })
            .ok_or("backstab target set definition")?;
        if set_row.text("RequirementSetType").map_err(debug)? != "REQUIREMENTSET_TEST_ALL" {
            return Err("unsupported backstab target set factory".into());
        }
        let rows = table("RequirementSetRequirements")?
            .rows()
            .filter(|r| {
                r.text("RequirementSetId").ok().map(str::as_bytes) == Some(target_set.as_slice())
            })
            .map(|r| r.text("RequirementId").map(str::to_owned).map_err(debug))
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() != 1 {
            return Err("unsupported backstab target children".into());
        }
        let child_row = table("Requirements")?
            .rows()
            .find(|r| r.text("RequirementId").ok() == Some(rows[0].as_str()))
            .ok_or("backstab child definition")?;
        let predicate = child_row.text("RequirementType").map_err(debug)?;
        if predicate != "REQUIREMENT_PLAYER_IS_AT_LEAST_ALLIANCE_LEVEL_WITH_EMERGENCY_PLAYER" {
            return Err("unsupported backstab target predicate".into());
        }
        for flag in ["Inverse", "Persistent", "Triggered"] {
            if child_row
                .get(flag)
                .map_err(debug)?
                .integer()
                .map_err(debug)?
                != 0
            {
                return Err(format!("unsupported backstab {flag}"));
            }
        }
        let argument = table("RequirementArguments")?
            .rows()
            .find(|r| {
                r.text("RequirementId").ok() == Some(rows[0].as_str())
                    && r.text("Name").ok() == Some("Level")
            })
            .ok_or("backstab minimum alliance level")?;
        let minimum_level = argument
            .text("Value")
            .map_err(debug)?
            .parse::<i32>()
            .map_err(debug)?;
        Ok(Self {
            binding: state.binding().clone(),
            emergency_type,
            minimum_level,
            target_set,
            child: sets::RequirementDefinition {
                id: rows[0].as_bytes().to_vec(),
                predicate: sets::Predicate::Unsupported(linux_type_hash(predicate.as_bytes())),
                inverse: false,
                persistent: false,
            },
        })
    }
}
fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

/// Call only at the source StartEmergency boundary: capability enabled, no
/// existing/pending matching emergency, XP2 definition present, major target,
/// and complete current Emergency wrapper-cache domain. Positive eligibility
/// remains unsupported and is rejected before changing the shared owners.
#[allow(clippy::too_many_arguments)]
pub fn check_target(
    state: &LoadedState,
    effects: &mut backend::EffectRuntime,
    requirements: &mut backend::requirements::Runtime,
    input: &Inputs,
    issuer: i32,
    victim: i32,
    thresholds: &AllianceThresholds,
) -> Result<(), String> {
    if state.binding() != &input.binding {
        return Err("backstab binding".into());
    }
    if effects
        .objects()
        .iter()
        .any(|(_, o)| matches!(o.kind, backend::ContextObject::Emergency { .. }))
    {
        return Err("backstab current Emergency wrapper collision/domain".into());
    }
    let subject = effects
        .objects()
        .iter()
        .find(|(_, o)| o.kind == backend::ContextObject::Player(issuer))
        .map(|(handle, _)| *handle)
        .ok_or("backstab current Player wrapper")?;
    // Preflight the unsupported positive branch before creating partial pending
    // state. This read has no callbacks/RNG; the supported negative path uses
    // exactly the native pending Emergency player-data [victim].
    let eligible =
        backstab_alliance_requirement(state, issuer, &[victim], input.minimum_level, thresholds)
            .map_err(debug)?;
    if eligible {
        return Err("positive Backstab emergency member/invitation continuation".into());
    }
    let definitions = sets::Runtime::from_initial(
        0,
        vec![(
            input.target_set.clone(),
            Some(sets::SetDefinition {
                id: input.target_set.clone(),
                requirements: vec![input.child.id.clone()],
            }),
        )],
        vec![input.child.clone()],
    )
    .map_err(str::to_owned)?;
    requirements
        .sets_mut()
        .install_factory_definitions(&definitions)
        .map_err(str::to_owned)?;
    let object = effects
        .make_emergency_object(state, issuer, input.emergency_type, -1)
        .map_err(debug)?;
    let context = effects.object(object).map_err(debug)?.instance_id;
    let mut factory = Factory {
        runtime: requirements.sets_mut(),
        child: &input.child,
        eligible,
        live: None,
    };
    let set = sets::get_or_create_unsubscribed_subject(
        &mut factory,
        context,
        sets::Subject::ModifierObject(subject),
        &input.target_set,
    )
    .map_err(debug)?
    .ok_or("backstab target set missing")?;
    changes::release(&mut factory, set).map_err(debug)?;
    if factory.live.is_some() {
        return Err("backstab target requirement still live".into());
    }
    effects.release_emergency_object(object).map_err(debug)
}
struct Factory<'a> {
    runtime: &'a mut sets::Runtime,
    child: &'a sets::RequirementDefinition,
    eligible: bool,
    live: Option<u64>,
}
impl sets::SubjectContext for Factory<'_> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.runtime)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.runtime)
    }
    fn initialize_requirement(
        &mut self,
        label: u64,
        definition: &sets::RequirementDefinition,
        _: i32,
        _: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Self::Error>> {
        if definition.id != self.child.id || self.live.is_some() {
            return Err(sets::Error::Context("backstab predicate factory"));
        }
        // OnInitialize subscribes AllianceLevelChanged(+d80), then Test reads
        // current state. StartEmergency dispatches no such event before final
        // release; the temporary subscription cannot receive a callback here.
        self.live = Some(label);
        Ok(sets::InitializedRequirement::Simple(Requirement {
            active: true,
            state: i32::from(!self.eligible),
            inverse: false,
            persistent: false,
            triggered: false,
        }))
    }
    fn destroy_initial_simple_requirement(
        &mut self,
        _: u64,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("backstab unexpected terminal child"))
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl changes::Context for Factory<'_> {
    type Error = &'static str;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.runtime)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.runtime)
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, sets::Error<Self::Error>> {
        Err(sets::Error::Context("backstab release Test"))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("backstab release callback"))
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<Self::Error>> {
        if self.live != Some(label) {
            return Err(sets::Error::Context("backstab requirement destruction"));
        }
        self.live = None;
        Ok(())
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Ok(())
    }
    fn reset_trigger(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context("backstab release trigger"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_query_retains_dynamic_child_until_final_release() {
        let child = sets::RequirementDefinition {
            id: b"alliance".to_vec(),
            predicate: sets::Predicate::Unsupported(1),
            inverse: false,
            persistent: false,
        };
        let mut runtime = sets::Runtime::from_initial(
            71,
            vec![(
                b"target".to_vec(),
                Some(sets::SetDefinition {
                    id: b"target".to_vec(),
                    requirements: vec![child.id.clone()],
                }),
            )],
            vec![child.clone()],
        )
        .unwrap();
        let mut factory = Factory {
            runtime: &mut runtime,
            child: &child,
            eligible: false,
            live: None,
        };
        let set = sets::get_or_create_unsubscribed_subject(
            &mut factory,
            25,
            sets::Subject::ModifierObject(10),
            b"target",
        )
        .unwrap()
        .unwrap();
        assert_eq!(factory.runtime.state(set), Some(1));
        assert_eq!(factory.runtime.next_native_id(), 73);
        let requirement = factory.live.unwrap();
        assert_eq!(
            factory.runtime.children(set),
            Some([requirement].as_slice())
        );
        changes::release(&mut factory, set).unwrap();
        assert!(factory.live.is_none());
        assert_eq!(factory.runtime.state(set), None);
        assert_eq!(factory.runtime.next_native_id(), 73);
        assert_eq!(
            factory.runtime.requirement_reference_count(requirement),
            Some(0)
        );
    }
}
