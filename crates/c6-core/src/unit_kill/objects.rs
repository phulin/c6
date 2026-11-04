//! Unit wrapper destruction over the existing canonical object/effect owners.
//! This is the first UnitRemoved listener, not a separate object simulation.
use crate::unit_moved_signal::{
    modifiers::AffectedModifiers, ListenerRing, RegistryError, Subscription,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Callback {
    OwnerDestroyed(u64),
    TrackedDestroyed(u64),
}
#[derive(Debug)]
pub struct Object {
    pub pool_ordinal: usize,
    pub owned_modifiers: Vec<i32>,
    pub affected: AffectedModifiers,
    pub destroyed: ListenerRing<Callback>,
}
#[derive(Debug)]
pub struct Tracked {
    pub object: u64,
    pub requirement_set: Option<u64>,
    pub applied: u8,
    pub removed: bool,
    pub destruction_subscription: Option<Subscription>,
}
#[derive(Debug)]
pub struct Modifier {
    pub receiver: u64,
    pub context: u64,
    pub instance: i32,
    pub enabled: bool,
    pub suppressed: bool,
    pub permanent: bool,
    pub definition: Vec<u8>,
    pub stack_limit: u16,
    pub effect: u64,
    pub tracked: Vec<Tracked>,
    pub applied: Vec<u64>,
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Ring(RegistryError),
    MissingTracked,
    MissingSubscription,
    ResourceLimit,
}
pub trait Context {
    type Error;
    fn object(&self, identity: u64) -> Result<&Object, Self::Error>;
    fn object_mut(&mut self, identity: u64) -> Result<&mut Object, Self::Error>;
    fn modifier(&self, receiver: u64) -> Result<&Modifier, Self::Error>;
    fn modifier_mut(&mut self, receiver: u64) -> Result<&mut Modifier, Self::Error>;
    fn object_order(&self, first: u64, second: u64) -> Result<std::cmp::Ordering, Self::Error>;
    /// ModifierSystem.ClearModifier removes registration before destruction.
    fn clear_owned_modifier(&mut self, instance: i32) -> Result<(), Self::Error>;
    fn owner_destroyed(&mut self, receiver: u64) -> Result<(), Self::Error>;
    fn unsubscribe_requirement(
        &mut self,
        receiver: u64,
        object: u64,
        set: u64,
    ) -> Result<(), Self::Error>;
    fn release_requirement(&mut self, set: u64) -> Result<(), Self::Error>;
    fn remove_effect(&mut self, effect: u64, object: u64) -> Result<bool, Self::Error>;
    fn log_removing(&mut self, receiver: u64, object: u64) -> Result<(), Self::Error>;
    fn log_failed_remove(&mut self, receiver: u64, object: u64) -> Result<(), Self::Error>;
}
fn tracked<C: Context>(
    c: &C,
    receiver: u64,
    object: u64,
) -> Result<Option<usize>, Error<C::Error>> {
    for (index, entry) in c
        .modifier(receiver)
        .map_err(Error::Context)?
        .tracked
        .iter()
        .enumerate()
    {
        if entry.object == object
            || c.object_order(entry.object, object)
                .map_err(Error::Context)?
                == std::cmp::Ordering::Equal
        {
            return Ok(Some(index));
        }
    }
    Ok(None)
}
fn erase_applied<C: Context>(c: &mut C, receiver: u64, object: u64) -> Result<(), Error<C::Error>> {
    let mut index = 0;
    while index < c.modifier(receiver).map_err(Error::Context)?.applied.len() {
        let other = c.modifier(receiver).map_err(Error::Context)?.applied[index];
        if other == object
            || c.object_order(other, object).map_err(Error::Context)? == std::cmp::Ordering::Equal
        {
            c.modifier_mut(receiver)
                .map_err(Error::Context)?
                .applied
                .remove(index);
            break;
        }
        index += 1;
    }
    Ok(())
}
/// DynamicModifier.RemoveEffects2334770 for a real, non-temporary Unit wrapper.
pub fn remove_effects<C: Context>(
    c: &mut C,
    receiver: u64,
    object: u64,
) -> Result<(), Error<C::Error>> {
    let m = c.modifier(receiver).map_err(Error::Context)?;
    if m.suppressed || m.permanent {
        return Ok(());
    }
    let index = tracked(c, receiver, object)?.ok_or(Error::MissingTracked)?;
    if c.modifier(receiver).map_err(Error::Context)?.tracked[index].applied == 0 {
        return Ok(());
    }
    c.modifier_mut(receiver).map_err(Error::Context)?.tracked[index].applied = 0;
    erase_applied(c, receiver, object)?;
    // RemoveEffect clears affected-instance membership BEFORE the stack count.
    let m = c.modifier(receiver).map_err(Error::Context)?;
    let instance = m.instance;
    c.object_mut(object)
        .map_err(Error::Context)?
        .affected
        .clear_instance(instance);
    let m = c.modifier(receiver).map_err(Error::Context)?;
    if m.stack_limit != 0
        && c.object(object)
            .map_err(Error::Context)?
            .affected
            .count(&m.definition)
            >= i32::from(m.stack_limit)
    {
        return Ok(());
    }
    let effect = m.effect;
    c.log_removing(receiver, object).map_err(Error::Context)?;
    if !c.remove_effect(effect, object).map_err(Error::Context)? {
        c.log_failed_remove(receiver, object)
            .map_err(Error::Context)?;
    }
    Ok(())
}
/// DynamicModifier.ObjectDestroyed2334610; Unit.IsTemporary is source-false.
pub fn tracked_destroyed<C: Context>(
    c: &mut C,
    receiver: u64,
    object: u64,
) -> Result<(), Error<C::Error>> {
    if tracked(c, receiver, object)?.is_some() {
        let m = c.modifier(receiver).map_err(Error::Context)?;
        if !m.suppressed && m.enabled {
            remove_effects(c, receiver, object)?;
        }
        let index = tracked(c, receiver, object)?.ok_or(Error::MissingTracked)?;
        let entry = &c.modifier(receiver).map_err(Error::Context)?.tracked[index];
        let subscription = entry
            .destruction_subscription
            .ok_or(Error::MissingSubscription)?;
        c.object_mut(object)
            .map_err(Error::Context)?
            .destroyed
            .unsubscribe(subscription)
            .map_err(Error::Ring)?;
        let set = c.modifier(receiver).map_err(Error::Context)?.tracked[index].requirement_set;
        if let Some(set) = set {
            c.unsubscribe_requirement(receiver, object, set)
                .map_err(Error::Context)?;
            let index = tracked(c, receiver, object)?.ok_or(Error::MissingTracked)?;
            c.modifier_mut(receiver).map_err(Error::Context)?.tracked[index].requirement_set = None;
            c.release_requirement(set).map_err(Error::Context)?;
        }
        let index = tracked(c, receiver, object)?.ok_or(Error::MissingTracked)?;
        c.modifier_mut(receiver)
            .map_err(Error::Context)?
            .tracked
            .remove(index);
    }
    erase_applied(c, receiver, object)
}
/// ModifierSystem.ObjectDestroyed2331560. The caller supplies the snapshot of
/// CURRENT matching Unit wrapper handles in native pool order, retaining each
/// distinct wrapper. No native predicate/effect answer is a parameter.
pub fn detach<C: Context>(c: &mut C, objects: &[u64]) -> Result<(), Error<C::Error>> {
    for &object in objects {
        let mut owned = Vec::new();
        let source = &c.object(object).map_err(Error::Context)?.owned_modifiers;
        owned
            .try_reserve_exact(source.len())
            .map_err(|_| Error::ResourceLimit)?;
        owned.extend_from_slice(source);
        for instance in owned {
            c.clear_owned_modifier(instance).map_err(Error::Context)?;
        }
        let mut cursor = c
            .object_mut(object)
            .map_err(Error::Context)?
            .destroyed
            .cursor_start()
            .map_err(Error::Ring)?;
        loop {
            let callback = c
                .object(object)
                .map_err(Error::Context)?
                .destroyed
                .cursor_route(cursor)
                .map_err(Error::Ring)?;
            let result = match callback {
                Some(Callback::OwnerDestroyed(receiver)) => {
                    c.owner_destroyed(receiver).map_err(Error::Context)
                }
                Some(Callback::TrackedDestroyed(receiver)) => {
                    tracked_destroyed(c, receiver, object)
                }
                None => Ok(()),
            };
            if let Err(error) = result {
                c.object_mut(object)
                    .map_err(Error::Context)?
                    .destroyed
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
            match c
                .object_mut(object)
                .map_err(Error::Context)?
                .destroyed
                .cursor_next(cursor)
                .map_err(Error::Ring)?
            {
                Some(next) => cursor = next,
                None => break,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{unit::Unit, unit_moved_signal::modifiers::AffectedEntry};
    struct Fixture {
        object: Object,
        modifier: Modifier,
        unit: Unit,
        calls: Vec<&'static str>,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn object(&self, _: u64) -> Result<&Object, Self::Error> {
            Ok(&self.object)
        }
        fn object_mut(&mut self, _: u64) -> Result<&mut Object, Self::Error> {
            Ok(&mut self.object)
        }
        fn modifier(&self, _: u64) -> Result<&Modifier, Self::Error> {
            Ok(&self.modifier)
        }
        fn modifier_mut(&mut self, _: u64) -> Result<&mut Modifier, Self::Error> {
            Ok(&mut self.modifier)
        }
        fn object_order(&self, a: u64, b: u64) -> Result<std::cmp::Ordering, Self::Error> {
            Ok(a.cmp(&b))
        }
        fn clear_owned_modifier(&mut self, _: i32) -> Result<(), Self::Error> {
            Err("unexpected owned")
        }
        fn owner_destroyed(&mut self, _: u64) -> Result<(), Self::Error> {
            Err("unexpected owner")
        }
        fn unsubscribe_requirement(&mut self, _: u64, _: u64, _: u64) -> Result<(), Self::Error> {
            self.calls.push("unsubscribe");
            Ok(())
        }
        fn release_requirement(&mut self, _: u64) -> Result<(), Self::Error> {
            assert!(self.modifier.tracked[0].requirement_set.is_none());
            self.calls.push("release");
            Ok(())
        }
        fn remove_effect(&mut self, _: u64, _: u64) -> Result<bool, Self::Error> {
            assert_eq!(self.modifier.tracked[0].applied, 0);
            assert!(self.modifier.applied.is_empty());
            assert_eq!(self.object.affected.count(b"experience"), 0);
            super::super::effects::remove(
                &mut self.unit,
                super::super::effects::Effect::ExperienceModifier(10),
                2,
            )
            .unwrap();
            self.calls.push("kernel");
            Ok(true)
        }
        fn log_removing(&mut self, _: u64, _: u64) -> Result<(), Self::Error> {
            self.calls.push("log");
            Ok(())
        }
        fn log_failed_remove(&mut self, _: u64, _: u64) -> Result<(), Self::Error> {
            Err("unexpected failure log")
        }
    }
    fn fixture() -> Fixture {
        let mut ring = ListenerRing::from_ordered(&[]).unwrap();
        let subscription = ring.subscribe(Callback::TrackedDestroyed(7)).unwrap();
        let object = Object {
            pool_ordinal: 0,
            owned_modifiers: vec![],
            affected: AffectedModifiers::from_sorted(vec![AffectedEntry {
                definition: b"experience".to_vec(),
                instance: 3,
            }])
            .unwrap(),
            destroyed: ring,
        };
        let modifier = Modifier {
            receiver: 7,
            context: 1,
            instance: 3,
            enabled: true,
            suppressed: false,
            permanent: false,
            definition: b"experience".to_vec(),
            stack_limit: 1,
            effect: 8,
            tracked: vec![Tracked {
                object: 1,
                requirement_set: Some(9),
                applied: 1,
                removed: false,
                destruction_subscription: Some(subscription),
            }],
            applied: vec![1],
        };
        let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
        unit.k_experience.x_general_experience_modifier = 281;
        Fixture {
            object,
            modifier,
            unit,
            calls: vec![],
        }
    }
    #[test]
    fn actual_field_kernel_runs_after_clear_before_requirement_release() {
        let mut f = fixture();
        detach(&mut f, &[1]).unwrap();
        assert_eq!(f.calls, ["log", "kernel", "unsubscribe", "release"]);
        assert_eq!(f.unit.k_experience.x_general_experience_modifier, 255);
        assert!(f.modifier.tracked.is_empty());
        assert!(f.modifier.applied.is_empty());
    }
    #[test]
    fn permanent_keeps_field_effect_and_affected_membership_but_releases_tracking() {
        let mut f = fixture();
        f.modifier.permanent = true;
        detach(&mut f, &[1]).unwrap();
        assert_eq!(f.calls, ["unsubscribe", "release"]);
        assert_eq!(f.unit.k_experience.x_general_experience_modifier, 281);
        assert_eq!(f.object.affected.count(b"experience"), 1);
        assert!(f.modifier.tracked.is_empty());
    }
    #[test]
    fn remaining_instance_at_stack_limit_skips_only_concrete_remove() {
        let mut f = fixture();
        f.object.affected.mark(b"experience", 4).unwrap();
        detach(&mut f, &[1]).unwrap();
        assert_eq!(f.calls, ["unsubscribe", "release"]);
        assert_eq!(f.object.affected.count(b"experience"), 1);
        assert_eq!(f.unit.k_experience.x_general_experience_modifier, 281);
        assert!(f.modifier.applied.is_empty());
    }
}
