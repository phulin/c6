//! PlayerUnits policy attachments over the existing Unit modifier owner.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Subscription {
    pub owner: i32,
    pub context: u64,
    pub receiver: u64,
}
impl Runtime {
    /// Source PlayerUnits creation subscriptions, in factory registration order.
    pub fn policy_unit_subscriptions(&self) -> &[Subscription] {
        &self.policy_units
    }
}
#[allow(clippy::too_many_arguments)]
pub fn enact<E: Effects>(
    owners: &mut Owners<'_>,
    runtime: &mut Runtime,
    effects: &mut E,
    definitions: &super::super::Definitions,
    context: u64,
    id: &[u8],
    amount: i32,
) -> Result<(), Error> {
    let ContextObject::Policy { owner, .. } =
        owners.effects.object(context).map_err(Error::Query)?.kind
    else {
        return Err(Error::Missing("Policy Player interface"));
    };
    let subjects = owners
        .state
        .world()
        .valid_player(owner)
        .map_err(|_| Error::Missing("Policy current Player"))?
        .ok_or(Error::Missing("Policy Player"))?
        .unit_records()
        .map_err(|_| Error::Missing("PlayerUnits current ordered records"))?
        .iter()
        .filter(|r| !r.value.b_delayed_death)
        .map(|r| UnitReference::from_component_id(c6_core::unit::ComponentId {
            owner: owner as i16,
            component_type: 1,
            id: r.value.id,
        }))
        .collect::<Vec<_>>();
    let instance = owners
        .effects
        .allocate_modifier_id(owners.state)
        .map_err(Error::Query)?;
    let receiver = (1u64 << 59) | u64::from(instance as u32);
    let effect = (1u64 << 58) | u64::from(instance as u32);
    owners
        .effects
        .register_unit_effect(
            owners.state,
            effect,
            crate::unit_kill::effects::Effect::Common(
                c6_core::unit_modifier_effect::Effect::BarbarianCombat { amount },
            ),
        )
        .map_err(Error::Query)?;
    owners
        .effects
        .register_unit_modifier(
            owners.state,
            records::Modifier {
                receiver,
                context,
                instance,
                control: common::Modifier {
                    enabled: true,
                    suppressed: false,
                    owner_requirement_set: None,
                },
                permanent: false,
                definition: id.to_vec(),
                stack_limit: 0,
                effect,
                tracked: vec![],
                applied: vec![],
            },
        )
        .map_err(Error::Query)?;
    let life = owners
        .effects
        .object_mut(context)
        .map_err(Error::Query)?
        .unit_lifecycle
        .as_mut()
        .ok_or(Error::Missing("Policy lifecycle"))?;
    life.owned_modifiers.push(instance);
    life.destroyed
        .subscribe(Callback::OwnerDestroyed(receiver))
        .map_err(Error::Ring)?;
    runtime.definitions.push(Definition {
        receiver,
        subject_requirements: vec![],
        repeatable: false,
    });
    runtime.policy_units.push(Subscription {
        owner,
        context,
        receiver,
    });
    let mut handles = Vec::with_capacity(subjects.len());
    for unit in subjects {
        let subject = owners
            .effects
            .make_current_unit_object(owners.state, unit, [0, 0])
            .map_err(Error::Query)?;
        handles.push((unit_key(owners.effects, subject)?, subject));
    }
    handles.sort_by(|a, b| a.0.compare(b.0));
    for (_, subject) in handles {
        addition::add(&mut Apply {
            owners,
            runtime,
            effects,
            definitions,
            receiver,
            subject,
        })?;
    }
    Ok(())
}
/// OnUnitCreated's actual PlayerUnits listener; no player policy is executed.
pub fn unit_created<E: Effects>(
    owners: &mut Owners<'_>,
    runtime: &mut Runtime,
    effects: &mut E,
    definitions: &super::super::Definitions,
    route: Subscription,
    unit: UnitReference,
) -> Result<(), Error> {
    let actor = super::super::actor::<Error>(owners.state, unit.player(), unit.id)
        .map_err(|_| Error::Missing("current PlayerUnits actor"))?;
    if actor.e_owner != route.owner {
        return Ok(());
    }
    let subject = owners
        .effects
        .make_current_unit_object(owners.state, unit, [0, 0])
        .map_err(Error::Query)?;
    addition::item_changed(
        &mut Apply {
            owners,
            runtime,
            effects,
            definitions,
            receiver: route.receiver,
            subject,
        },
        true,
    )
}
struct Remove<'a, 's> {
    owners: &'a mut Owners<'s>,
}
impl records::Context for Remove<'_, '_> {
    type Error = Error;
    fn object(&self, id: u64) -> Result<&records::Object, Error> {
        self.owners
            .effects
            .object(id)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_ref()
            .ok_or(Error::Missing("Policy Unit lifecycle"))
    }
    fn object_mut(&mut self, id: u64) -> Result<&mut records::Object, Error> {
        self.owners
            .effects
            .object_mut(id)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_mut()
            .ok_or(Error::Missing("Policy Unit lifecycle"))
    }
    fn modifier(&self, id: u64) -> Result<&records::Modifier, Error> {
        self.owners.effects.unit_modifier(id).map_err(Error::Query)
    }
    fn modifier_mut(&mut self, id: u64) -> Result<&mut records::Modifier, Error> {
        self.owners
            .effects
            .unit_modifier_mut(id)
            .map_err(Error::Query)
    }
    fn object_order(&self, a: u64, b: u64) -> Result<std::cmp::Ordering, Error> {
        Ok(unit_key(self.owners.effects, a)?.compare(unit_key(self.owners.effects, b)?))
    }
    fn clear_owned_modifier(&mut self, _: i32) -> Result<(), Error> {
        Err(Error::Missing("Policy nested owned modifier clear"))
    }
    fn owner_destroyed(&mut self, _: u64) -> Result<(), Error> {
        Err(Error::Missing("Policy nested owner destruction"))
    }
    fn unsubscribe_requirement(&mut self, _: u64, _: u64, _: u64) -> Result<(), Error> {
        Err(Error::Missing("Policy Unit requirement unsubscribe"))
    }
    fn release_requirement(&mut self, _: u64) -> Result<(), Error> {
        Err(Error::Missing("Policy Unit requirement release"))
    }
    fn remove_effect(&mut self, effect: u64, object: u64) -> Result<bool, Error> {
        let backend::EffectState::Unit(crate::unit_kill::effects::Effect::Common(effect)) =
            *self.owners.effects.effect(effect).map_err(Error::Query)?
        else {
            return Err(Error::Missing("Policy concrete Unit Remove"));
        };
        let ContextObject::Unit(unit) = self
            .owners
            .effects
            .object(object)
            .map_err(Error::Query)?
            .kind
        else {
            return Err(Error::Missing("Policy Remove IUnit"));
        };
        crate::unit_modifier_effect::remove(self.owners.state, Some(unit), effect)
            .map_err(|_| Error::Missing("Policy Unit effect Remove"))
    }
    fn log_removing(&mut self, _: u64, _: u64) -> Result<(), Error> {
        Ok(())
    }
    fn log_failed_remove(&mut self, _: u64, _: u64) -> Result<(), Error> {
        Ok(())
    }
}
pub fn retract(owners: &mut Owners<'_>, runtime: &mut Runtime, context: u64) -> Result<(), Error> {
    let owned = &owners
        .effects
        .object(context)
        .map_err(Error::Query)?
        .unit_lifecycle
        .as_ref()
        .ok_or(Error::Missing("current Policy Unit owner lifecycle"))?
        .owned_modifiers;
    for instance in owned {
        if !runtime
            .policy_units
            .iter()
            .filter(|r| r.context == context)
            .any(|r| {
                owners
                    .effects
                    .unit_modifier(r.receiver)
                    .is_ok_and(|m| m.instance == *instance)
            })
        {
            return Err(Error::Missing("imported Policy Unit modifier destruction"));
        }
    }
    if !runtime.policy_units.iter().any(|r| r.context == context) {
        return Err(Error::Missing("current Policy Unit modifier registration"));
    }
    runtime.log_disabled(3)?;
    loop {
        let Some(at) = runtime
            .policy_units
            .iter()
            .position(|r| r.context == context)
        else {
            return Ok(());
        };
        let receiver = runtime.policy_units[at].receiver;
        records::deactivate(&mut Remove { owners }, receiver)
            .map_err(|_| Error::Missing("Policy Unit modifier Deactivate"))?;
        let m = owners
            .effects
            .remove_unit_modifier_registration(receiver)
            .map_err(Error::Query)?;
        let life = owners
            .effects
            .object_mut(context)
            .map_err(Error::Query)?
            .unit_lifecycle
            .as_mut()
            .ok_or(Error::Missing("Policy lifecycle"))?;
        life.owned_modifiers.retain(|i| *i != m.instance);
        let token = life
            .destroyed
            .subscription_for(|r| r == Callback::OwnerDestroyed(receiver))
            .map_err(Error::Ring)?;
        life.destroyed.unsubscribe(token).map_err(Error::Ring)?;
        owners
            .effects
            .retire_effect(owners.state, m.effect)
            .map_err(Error::Query)?;
        runtime.definitions.retain(|r| r.receiver != receiver);
        runtime.policy_units.remove(at);
    }
}
