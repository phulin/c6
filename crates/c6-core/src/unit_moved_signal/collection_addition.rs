//! Subject-neutral DynamicModifier collection-change/addition lifecycle.
//! Concrete objects and requirement sets remain in their one canonical owner.
use super::modifiers::{self, Context as ApplyContext, TrackedSubject};

/// The current subject is selected by the adapter. Both City and ProposedCombat
/// adapters may use this kernel; neither changes the subject's logical type.
pub trait Context: ApplyContext {
    type DestructionSubscription;
    fn instance_id(&self) -> Result<i32, Self::Error>;
    fn before_update(&mut self, instance: i32) -> Result<(), Self::Error>;
    fn after_update(&mut self, instance: i32) -> Result<(), Self::Error>;
    fn subscribe_destroyed(&mut self) -> Result<Self::DestructionSubscription, Self::Error>;
    fn tracked_exists(&self) -> Result<bool, Self::Error>;
    /// Insert at the source comparator's unique position, keeping the real
    /// destruction subscription with that tracked object. No cached answers.
    fn insert_tracked(
        &mut self,
        state: TrackedSubject,
        subscription: Self::DestructionSubscription,
    ) -> Result<(), Self::Error>;
    fn unsubscribe_temporary(
        &mut self,
        subscription: Self::DestructionSubscription,
    ) -> Result<(), Self::Error>;
    fn has_subject_requirement_definition(&self) -> Result<bool, Self::Error>;
    /// Create+initialize the actual set and subscribe this modifier to changes.
    /// None is an actual failed lookup/creation, never unavailable metadata.
    fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Self::Error>;
    fn remove_tracked_subject(&mut self) -> Result<(), Self::Error>;
}
/// AddTrackedSubject2334d40. The temporary tracked constructor subscribes even
/// for an existing comparator-equivalent subject; its destructor unsubscribes.
pub fn add<C: Context>(context: &mut C) -> Result<(), C::Error> {
    let subscription = context.subscribe_destroyed()?;
    if context.tracked_exists()? {
        context.unsubscribe_temporary(subscription)?;
    } else {
        context.insert_tracked(
            TrackedSubject {
                requirement_set: None,
                applied: 0,
                removed: false,
            },
            subscription,
        )?;
    }
    if !context.tracked()?.removed
        && context.tracked()?.requirement_set.is_none()
        && context.has_subject_requirement_definition()?
    {
        let set = context.create_subject_requirement_set()?;
        if let Some(id) = set {
            context.tracked_mut()?.requirement_set = Some(id);
        } else {
            context.tracked_mut()?.removed = true;
        }
    }
    let _ = modifiers::check_apply_effects(context)?;
    Ok(())
}
/// OnCollectionItemAddedOrRemoved2332cc0. Before/After are source-class hooks,
/// not universally inert. After uses the same instance read before Before.
pub fn item_changed<C: Context>(context: &mut C, added: bool) -> Result<(), C::Error> {
    if context.modifier()?.suppressed || !context.modifier()?.enabled {
        return Ok(());
    }
    if let Some(set) = context.modifier()?.owner_requirement_set {
        if (context.requirement_set_state(set)? | 2) != 2
            && context.requirement_set_state(set)? != 4
        {
            return Ok(());
        }
    }
    let instance = context.instance_id()?;
    context.before_update(instance)?;
    if added {
        add(context)?;
    } else {
        context.remove_tracked_subject()?;
    }
    context.after_update(instance)
}
#[cfg(test)]
mod tests {
    use super::super::modifiers::Modifier;
    use super::*;
    struct Fixture {
        modifier: Modifier,
        tracked: Option<TrackedSubject>,
        log: Vec<&'static str>,
        has_definition: bool,
        set: Option<u64>,
        instance: i32,
    }
    impl ApplyContext for Fixture {
        type Error = &'static str;
        fn modifier(&self) -> Result<&Modifier, Self::Error> {
            Ok(&self.modifier)
        }
        fn tracked(&self) -> Result<&TrackedSubject, Self::Error> {
            self.tracked.as_ref().ok_or("missing tracked")
        }
        fn tracked_mut(&mut self) -> Result<&mut TrackedSubject, Self::Error> {
            self.tracked.as_mut().ok_or("missing tracked")
        }
        fn requirement_set_state(&self, _: u64) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn ensure_applied_subject(&mut self) -> Result<(), Self::Error> {
            self.log.push("applied-list");
            Ok(())
        }
        fn is_repeatable(&self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn apply_effect(&mut self) -> Result<bool, Self::Error> {
            assert_eq!(self.tracked()?.applied, 1);
            self.log.push("apply");
            Ok(false)
        }
        fn unsubscribe_subject_requirement(&mut self) -> Result<(), Self::Error> {
            Err("unexpected unsubscribe")
        }
        fn clear_subject_requirement(&mut self) -> Result<(), Self::Error> {
            Err("unexpected clear")
        }
        fn remove_effects(&mut self) -> Result<(), Self::Error> {
            self.log.push("remove-effects");
            Ok(())
        }
    }
    impl Context for Fixture {
        type DestructionSubscription = ();
        fn instance_id(&self) -> Result<i32, Self::Error> {
            Ok(self.instance)
        }
        fn before_update(&mut self, id: i32) -> Result<(), Self::Error> {
            assert_eq!(id, 7);
            self.instance = 9;
            self.log.push("before");
            Ok(())
        }
        fn after_update(&mut self, id: i32) -> Result<(), Self::Error> {
            assert_eq!(id, 7);
            self.log.push("after");
            Ok(())
        }
        fn subscribe_destroyed(&mut self) -> Result<(), Self::Error> {
            self.log.push("subscribe");
            Ok(())
        }
        fn tracked_exists(&self) -> Result<bool, Self::Error> {
            Ok(self.tracked.is_some())
        }
        fn insert_tracked(&mut self, v: TrackedSubject, _: ()) -> Result<(), Self::Error> {
            self.log.push("insert");
            self.tracked = Some(v);
            Ok(())
        }
        fn unsubscribe_temporary(&mut self, _: ()) -> Result<(), Self::Error> {
            self.log.push("unsubscribe-temp");
            Ok(())
        }
        fn has_subject_requirement_definition(&self) -> Result<bool, Self::Error> {
            Ok(self.has_definition)
        }
        fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Self::Error> {
            self.log.push("requirements");
            Ok(self.set)
        }
        fn remove_tracked_subject(&mut self) -> Result<(), Self::Error> {
            self.log.push("remove-tracked");
            Ok(())
        }
    }
    fn fixture() -> Fixture {
        Fixture {
            modifier: Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            tracked: None,
            log: vec![],
            has_definition: true,
            set: Some(1),
            instance: 7,
        }
    }
    #[test]
    fn new_subject_initializes_requirements_then_applies_and_runs_real_hooks() {
        let mut f = fixture();
        item_changed(&mut f, true).unwrap();
        assert_eq!(
            f.log,
            [
                "before",
                "subscribe",
                "insert",
                "requirements",
                "applied-list",
                "apply",
                "after"
            ]
        );
        assert_eq!(
            f.tracked.unwrap(),
            TrackedSubject {
                requirement_set: Some(1),
                applied: 1,
                removed: false
            }
        );
    }
    #[test]
    fn duplicate_constructor_subscription_is_released_and_missing_set_is_not_empty() {
        let mut f = fixture();
        f.tracked = Some(TrackedSubject {
            requirement_set: None,
            applied: 0,
            removed: false,
        });
        f.set = None;
        item_changed(&mut f, true).unwrap();
        assert_eq!(
            f.log,
            [
                "before",
                "subscribe",
                "unsubscribe-temp",
                "requirements",
                "after"
            ]
        );
        assert!(f.tracked.unwrap().removed);
        let mut disabled = fixture();
        disabled.modifier.enabled = false;
        item_changed(&mut disabled, true).unwrap();
        assert!(disabled.log.is_empty());
    }
}
