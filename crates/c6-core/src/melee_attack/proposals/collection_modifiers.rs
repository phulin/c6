//! DynamicModifier collection additions for newly proposed combat objects.
//! Reuses the ordinary CheckApplyEffects and ApplyEffect state machines.
use super::registry::{Handle, Runtime as Proposals};
use crate::unit_moved_signal::{modifiers as common, RegistryError, Subscription};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectFamily {
    PlayerStrength,
    AllianceStrength,
}
#[derive(Debug)]
pub struct Modifier {
    pub effect_family: EffectFamily,
    pub receiver: u64,
    /// Stable context-wrapper identity, distinct from the effect owner even
    /// when both wrappers reference the same Unit ComponentID.
    pub context: u64,
    pub control: common::Modifier,
    pub instance: i32,
    pub definition: Vec<u8>,
    pub repeatable: bool,
    pub stack_limit: u16,
    pub subject_requirements: Vec<u8>,
    pub effect: u64,
    pub tracked: Vec<Tracked>,
    pub applied: Vec<Handle>,
}
#[derive(Debug)]
pub struct Tracked {
    pub proposal: Handle,
    pub state: common::TrackedSubject,
    pub destruction_subscription: Subscription,
}
#[derive(Debug)]
pub struct Runtime {
    modifiers: Vec<Modifier>,
}
impl Runtime {
    pub fn from_initial(modifiers: Vec<Modifier>) -> Result<Self, u64> {
        for (i, m) in modifiers.iter().enumerate() {
            if modifiers[..i].iter().any(|n| n.receiver == m.receiver) {
                return Err(m.receiver);
            }
        }
        Ok(Self { modifiers })
    }
    pub fn modifier(&self, receiver: u64) -> Option<&Modifier> {
        self.modifiers.iter().find(|m| m.receiver == receiver)
    }
    pub fn modifier_mut(&mut self, receiver: u64) -> Option<&mut Modifier> {
        self.modifiers.iter_mut().find(|m| m.receiver == receiver)
    }
    /// ClearModifier erases registration before releasing the modifier owner.
    /// The returned value remains live while its synchronous destructor runs.
    pub fn remove_registration(&mut self, receiver: u64) -> Option<Modifier> {
        let index = self.modifiers.iter().position(|m| m.receiver == receiver)?;
        Some(self.modifiers.remove(index))
    }
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Runtime(super::registry::Error),
    MissingModifier(u64),
    MissingTrackedSubject,
    Allocation,
    Ring(RegistryError),
    Affected(common::AffectedError),
    Effect(common::EffectError<Box<Error<E>>>),
    RemovalUnsupported,
}
pub trait Context {
    type Error;
    fn proposals(&self) -> Result<&Proposals, Self::Error>;
    fn proposals_mut(&mut self) -> Result<&mut Proposals, Self::Error>;
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error>;
    /// Source-defined DynamicModifierComparator for distinct proposal objects.
    fn proposal_order(
        &self,
        first: Handle,
        second: Handle,
    ) -> Result<std::cmp::Ordering, Self::Error>;
    /// Native RequirementSystem.GetRequirementSet(context,proposal,definitionID)
    /// and subscription of the current modifier to its changes. None is the real
    /// lookup failure (which marks the tracked subject removed), never unknown.
    fn create_subject_requirements(
        &mut self,
        modifier: u64,
        proposal: Handle,
    ) -> Result<Option<u64>, Self::Error>;
    fn unsubscribe_subject_requirements(
        &mut self,
        receiver: u64,
        proposal: Handle,
        set: u64,
    ) -> Result<(), Self::Error>;
    fn release_subject_requirements(&mut self, set: u64) -> Result<(), Self::Error>;
    fn apply_strength_effect(&mut self, effect: u64, proposal: Handle)
        -> Result<bool, Self::Error>;
    fn log_applying(&mut self, modifier: u64, proposal: Handle) -> Result<(), Self::Error>;
    fn log_failed(&mut self, modifier: u64, proposal: Handle) -> Result<(), Self::Error>;
}
struct Adapter<'a, C> {
    context: &'a mut C,
    receiver: u64,
    proposal: Handle,
}
impl<C: Context> Adapter<'_, C> {
    fn record(&self) -> Result<&Modifier, Error<C::Error>> {
        self.context
            .proposals()
            .map_err(Error::Context)?
            .collection_modifiers()
            .map_err(Error::Runtime)?
            .modifier(self.receiver)
            .ok_or(Error::MissingModifier(self.receiver))
    }
    fn record_mut(&mut self) -> Result<&mut Modifier, Error<C::Error>> {
        self.context
            .proposals_mut()
            .map_err(Error::Context)?
            .collection_modifiers_mut()
            .map_err(Error::Runtime)?
            .modifier_mut(self.receiver)
            .ok_or(Error::MissingModifier(self.receiver))
    }
    fn tracked_position(&self) -> Result<(usize, bool), Error<C::Error>> {
        for (i, t) in self.record()?.tracked.iter().enumerate() {
            let order = if t.proposal == self.proposal {
                std::cmp::Ordering::Equal
            } else {
                self.context
                    .proposal_order(t.proposal, self.proposal)
                    .map_err(Error::Context)?
            };
            match order {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => return Ok((i, true)),
                std::cmp::Ordering::Greater => return Ok((i, false)),
            }
        }
        Ok((self.record()?.tracked.len(), false))
    }
    fn tracked(&self) -> Result<&Tracked, Error<C::Error>> {
        let (i, found) = self.tracked_position()?;
        if !found {
            return Err(Error::MissingTrackedSubject);
        }
        Ok(&self.record()?.tracked[i])
    }
    fn tracked_mut(&mut self) -> Result<&mut Tracked, Error<C::Error>> {
        let (i, found) = self.tracked_position()?;
        if !found {
            return Err(Error::MissingTrackedSubject);
        }
        Ok(&mut self.record_mut()?.tracked[i])
    }
}
/// ModifierSystem.ObjectDestroyed: ClearModifiers precedes the reentrant
/// per-object destruction signal. A genuinely reached owned modifier remains
/// unsupported; new native ProposedCombat objects start with an empty list.
pub fn forget_combat<C: Context>(c: &mut C, id: u32) -> Result<(), Error<C::Error>> {
    let Some(proposal) = c
        .proposals_mut()
        .map_err(Error::Context)?
        .forget_registration(id)
    else {
        return Ok(());
    };
    if !c
        .proposals()
        .map_err(Error::Context)?
        .proposal(proposal)
        .map_err(Error::Runtime)?
        .owned_modifiers
        .is_empty()
    {
        return Err(Error::RemovalUnsupported);
    }
    let mut cursor = c
        .proposals_mut()
        .map_err(Error::Context)?
        .destruction_mut(proposal)
        .map_err(Error::Runtime)?
        .cursor_start()
        .map_err(Error::Ring)?;
    loop {
        let receiver = c
            .proposals_mut()
            .map_err(Error::Context)?
            .destruction_mut(proposal)
            .map_err(Error::Runtime)?
            .cursor_route(cursor)
            .map_err(Error::Ring)?;
        if let Some(receiver) = receiver {
            if let Err(error) = object_destroyed(c, receiver, proposal) {
                c.proposals_mut()
                    .map_err(Error::Context)?
                    .destruction_mut(proposal)
                    .map_err(Error::Runtime)?
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(error);
            }
        }
        match c
            .proposals_mut()
            .map_err(Error::Context)?
            .destruction_mut(proposal)
            .map_err(Error::Runtime)?
            .cursor_next(cursor)
            .map_err(Error::Ring)?
        {
            Some(next) => cursor = next,
            None => break,
        }
    }
    Ok(())
}

/// DynamicModifier.ObjectDestroyed2334610. Temporary proposed combats bypass
/// RemoveEffects; tracked destruction still unsubscribes both signal handles
/// and releases the owned requirement set before removing the applied entry.
pub fn object_destroyed<C: Context>(
    c: &mut C,
    receiver: u64,
    proposal: Handle,
) -> Result<(), Error<C::Error>> {
    let mut a = Adapter {
        context: c,
        receiver,
        proposal,
    };
    let (position, found) = a.tracked_position()?;
    if found {
        if !a.record()?.control.suppressed
            && a.record()?.control.enabled
            && !a
                .context
                .proposals()
                .map_err(Error::Context)?
                .proposal(proposal)
                .map_err(Error::Runtime)?
                .temporary
        {
            common::Context::remove_effects(&mut a)?;
        }
        // TrackedSubject destructor: destruction unsubscribe, requirement
        // unsubscribe, clear/release requirement, then release subject.
        let subscription = a.tracked()?.destruction_subscription;
        a.context
            .proposals_mut()
            .map_err(Error::Context)?
            .destruction_mut(proposal)
            .map_err(Error::Runtime)?
            .unsubscribe(subscription)
            .map_err(Error::Ring)?;
        if a.tracked()?.state.requirement_set.is_some() {
            common::Context::unsubscribe_subject_requirement(&mut a)?;
            common::Context::clear_subject_requirement(&mut a)?;
        }
        a.record_mut()?.tracked.remove(position);
    }
    // Native always edits the applied vector, even if tracked lookup missed.
    // Comparator equality, rather than handle identity, selects its entry.
    let mut index = 0;
    while index < a.record()?.applied.len() {
        let other = a.record()?.applied[index];
        let equal = if other == proposal {
            true
        } else {
            a.context
                .proposal_order(other, proposal)
                .map_err(Error::Context)?
                == std::cmp::Ordering::Equal
        };
        if equal {
            a.record_mut()?.applied.remove(index);
            break;
        }
        index += 1;
    }
    Ok(())
}

/// Source OnCollectionItemAddedOrRemoved2332cc0. The actual strength-effect
/// BeforeUpdate/AfterUpdate slots are one-byte RETs; this entry is restricted to
/// the two typed player/alliance strength families. Other classes require their own hooks.
pub fn item_changed<C: Context>(
    c: &mut C,
    receiver: u64,
    proposal: Handle,
    added: bool,
) -> Result<(), Error<C::Error>> {
    crate::unit_moved_signal::collection_addition::item_changed(
        &mut Adapter {
            context: c,
            receiver,
            proposal,
        },
        added,
    )
}
impl<C: Context> crate::unit_moved_signal::collection_addition::Context for Adapter<'_, C> {
    type DestructionSubscription = Subscription;
    fn instance_id(&self) -> Result<i32, Self::Error> {
        Ok(self.record()?.instance)
    }
    // These two admitted strength classes have source-qualified literal RET
    // BeforeUpdate/AfterUpdate slots. The common kernel does not assume this.
    fn before_update(&mut self, _: i32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn after_update(&mut self, _: i32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn subscribe_destroyed(&mut self) -> Result<Subscription, Self::Error> {
        self.context
            .proposals_mut()
            .map_err(Error::Context)?
            .destruction_mut(self.proposal)
            .map_err(Error::Runtime)?
            .subscribe(self.receiver)
            .map_err(Error::Ring)
    }
    fn tracked_exists(&self) -> Result<bool, Self::Error> {
        Ok(self.tracked_position()?.1)
    }
    fn insert_tracked(
        &mut self,
        state: common::TrackedSubject,
        subscription: Subscription,
    ) -> Result<(), Self::Error> {
        let (position, exists) = self.tracked_position()?;
        if exists {
            return Err(Error::MissingTrackedSubject);
        }
        let proposal = self.proposal;
        self.record_mut()?
            .tracked
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        self.record_mut()?.tracked.insert(
            position,
            Tracked {
                proposal,
                state,
                destruction_subscription: subscription,
            },
        );
        Ok(())
    }
    fn unsubscribe_temporary(&mut self, subscription: Subscription) -> Result<(), Self::Error> {
        self.context
            .proposals_mut()
            .map_err(Error::Context)?
            .destruction_mut(self.proposal)
            .map_err(Error::Runtime)?
            .unsubscribe(subscription)
            .map_err(Error::Ring)
    }
    fn has_subject_requirement_definition(&self) -> Result<bool, Self::Error> {
        Ok(!self.record()?.subject_requirements.is_empty())
    }
    fn create_subject_requirement_set(&mut self) -> Result<Option<u64>, Self::Error> {
        self.context
            .create_subject_requirements(self.receiver, self.proposal)
            .map_err(Error::Context)
    }
    fn remove_tracked_subject(&mut self) -> Result<(), Self::Error> {
        Err(Error::RemovalUnsupported)
    }
}

impl<C: Context> common::Context for Adapter<'_, C> {
    type Error = Error<C::Error>;
    fn modifier(&self) -> Result<&common::Modifier, Self::Error> {
        Ok(&self.record()?.control)
    }
    fn tracked(&self) -> Result<&common::TrackedSubject, Self::Error> {
        Ok(&Adapter::tracked(self)?.state)
    }
    fn tracked_mut(&mut self) -> Result<&mut common::TrackedSubject, Self::Error> {
        Ok(&mut Adapter::tracked_mut(self)?.state)
    }
    fn requirement_set_state(&self, id: u64) -> Result<i32, Self::Error> {
        self.context
            .requirement_set_state(id)
            .map_err(Error::Context)
    }
    fn ensure_applied_subject(&mut self) -> Result<(), Self::Error> {
        let p = self.proposal;
        let mut position = 0;
        while position < self.record()?.applied.len() {
            let other = self.record()?.applied[position];
            let order = if other == p {
                std::cmp::Ordering::Equal
            } else {
                self.context
                    .proposal_order(other, p)
                    .map_err(Error::Context)?
            };
            match order {
                std::cmp::Ordering::Less => position += 1,
                std::cmp::Ordering::Equal => return Ok(()),
                std::cmp::Ordering::Greater => break,
            }
        }
        let m = self.record_mut()?;
        m.applied.try_reserve(1).map_err(|_| Error::Allocation)?;
        m.applied.insert(position, p);
        Ok(())
    }
    fn is_repeatable(&self) -> Result<bool, Self::Error> {
        Ok(self.record()?.repeatable)
    }
    fn apply_effect(&mut self) -> Result<bool, Self::Error> {
        common::apply_effect(self).map_err(|e| {
            Error::Effect(match e {
                common::EffectError::Context(e) => common::EffectError::Context(Box::new(e)),
                common::EffectError::DefinitionDisappeared => {
                    common::EffectError::DefinitionDisappeared
                }
            })
        })?;
        Ok(true)
    }
    fn unsubscribe_subject_requirement(&mut self) -> Result<(), Self::Error> {
        let set = Adapter::tracked(self)?
            .state
            .requirement_set
            .ok_or(Error::MissingTrackedSubject)?;
        self.context
            .unsubscribe_subject_requirements(self.receiver, self.proposal, set)
            .map_err(Error::Context)
    }
    fn clear_subject_requirement(&mut self) -> Result<(), Self::Error> {
        let set = Adapter::tracked_mut(self)?
            .state
            .requirement_set
            .take()
            .ok_or(Error::MissingTrackedSubject)?;
        self.context
            .release_subject_requirements(set)
            .map_err(Error::Context)
    }
    fn remove_effects(&mut self) -> Result<(), Self::Error> {
        // 2334770 returns before any removal when suppressed or applied==0.
        // The intervening admitted SimpleModifierDefinition permanent getter
        // is read-only; a reached applied-effect removal remains unsupported.
        if self.record()?.control.suppressed || Adapter::tracked(self)?.state.applied == 0 {
            return Ok(());
        }
        Err(Error::RemovalUnsupported)
    }
}
impl<C: Context> common::EffectContext for Adapter<'_, C> {
    type Error = Error<C::Error>;
    fn mark_subject_affected(&mut self) -> Result<(), Self::Error> {
        self.context
            .proposals_mut()
            .map_err(Error::Context)?
            .mark_subject_affected(self.receiver, self.proposal)
            .map_err(Error::Runtime)
    }
    fn suppressed(&self) -> Result<bool, Self::Error> {
        Ok(self.record()?.control.suppressed)
    }
    fn subject_stack_limit(&self) -> Result<Option<u16>, Self::Error> {
        Ok(Some(self.record()?.stack_limit))
    }
    fn affected_count(&self) -> Result<i32, Self::Error> {
        let r = self.record()?;
        Ok(self
            .context
            .proposals()
            .map_err(Error::Context)?
            .proposal(self.proposal)
            .map_err(Error::Runtime)?
            .modifiers_affecting
            .count(&r.definition))
    }
    fn log_applying(&mut self) -> Result<(), Self::Error> {
        self.context
            .log_applying(self.receiver, self.proposal)
            .map_err(Error::Context)
    }
    fn apply_concrete_effect(&mut self) -> Result<bool, Self::Error> {
        let effect = self.record()?.effect;
        self.context
            .apply_strength_effect(effect, self.proposal)
            .map_err(Error::Context)
    }
    fn log_failed(&mut self) -> Result<(), Self::Error> {
        self.context
            .log_failed(self.receiver, self.proposal)
            .map_err(Error::Context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::UnitReference, melee_attack::proposals::registry::Proposal};
    struct Fixture {
        proposals: Proposals,
        calls: Vec<&'static str>,
        active: bool,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn proposals(&self) -> Result<&Proposals, Self::Error> {
            Ok(&self.proposals)
        }
        fn proposals_mut(&mut self) -> Result<&mut Proposals, Self::Error> {
            Ok(&mut self.proposals)
        }
        fn proposal_order(
            &self,
            first: Handle,
            second: Handle,
        ) -> Result<std::cmp::Ordering, Self::Error> {
            if first == second {
                Ok(std::cmp::Ordering::Equal)
            } else {
                Err("distinct proposal comparator unavailable")
            }
        }
        fn requirement_set_state(&self, _: u64) -> Result<i32, Self::Error> {
            Err("unknown owner set")
        }
        fn create_subject_requirements(
            &mut self,
            _: u64,
            _: Handle,
        ) -> Result<Option<u64>, Self::Error> {
            self.calls.push("requirements");
            Err("actual subject predicates unavailable")
        }
        fn unsubscribe_subject_requirements(
            &mut self,
            _: u64,
            _: Handle,
            _: u64,
        ) -> Result<(), Self::Error> {
            assert!(self.proposals.active().is_empty());
            self.calls.push("unsubscribe");
            Ok(())
        }
        fn release_subject_requirements(&mut self, _: u64) -> Result<(), Self::Error> {
            assert!(self
                .proposals
                .collection_modifiers()
                .unwrap()
                .modifier(7)
                .unwrap()
                .tracked[0]
                .state
                .requirement_set
                .is_none());
            self.calls.push("release");
            Ok(())
        }
        fn apply_strength_effect(&mut self, _: u64, p: Handle) -> Result<bool, Self::Error> {
            assert_eq!(
                self.proposals
                    .proposal(p)
                    .unwrap()
                    .modifiers_affecting
                    .count(b"strength"),
                1
            );
            assert_eq!(
                self.proposals
                    .collection_modifiers()
                    .unwrap()
                    .modifier(7)
                    .unwrap()
                    .tracked[0]
                    .state
                    .applied,
                1
            );
            self.proposals
                .proposal_mut(p)
                .unwrap()
                .attacker_strength_modifier += 5;
            self.calls.push("kernel");
            Ok(false)
        }
        fn log_applying(&mut self, _: u64, _: Handle) -> Result<(), Self::Error> {
            self.calls.push("applying");
            Ok(())
        }
        fn log_failed(&mut self, _: u64, _: Handle) -> Result<(), Self::Error> {
            self.calls.push("failed");
            Ok(())
        }
    }
    fn fixture(subject: Vec<u8>, active: bool) -> (Fixture, Handle) {
        let modifier = Modifier {
            effect_family: EffectFamily::PlayerStrength,
            receiver: 7,
            context: 9,
            control: common::Modifier {
                enabled: active,
                suppressed: false,
                owner_requirement_set: None,
            },
            instance: 3,
            definition: b"strength".to_vec(),
            repeatable: false,
            stack_limit: 1,
            subject_requirements: subject,
            effect: 8,
            tracked: vec![],
            applied: vec![],
        };
        let mut proposals = Proposals::from_initial(0, vec![], None, None, None)
            .unwrap()
            .with_collection_modifiers(Runtime::from_initial(vec![modifier]).unwrap());
        let u = UnitReference {
            owner: 0x1000c,
            id: 1,
        };
        let (_, p) = proposals
            .register(Proposal::melee_units(u, u, None))
            .unwrap();
        (
            Fixture {
                proposals,
                calls: vec![],
                active,
            },
            p,
        )
    }
    #[test]
    fn temporary_forget_unsubscribes_without_reversing_strength() {
        let (mut f, p) = fixture(vec![], true);
        item_changed(&mut f, 7, p, true).unwrap();
        forget_combat(&mut f, 1).unwrap();
        assert!(f.proposals.active().is_empty());
        assert_eq!(f.proposals.object_id(p).unwrap(), 1);
        assert_eq!(
            f.proposals.proposal(p).unwrap().attacker_strength_modifier,
            5
        );
        let m = f
            .proposals
            .collection_modifiers()
            .unwrap()
            .modifier(7)
            .unwrap();
        assert!(m.tracked.is_empty());
        assert!(m.applied.is_empty());
        assert_eq!(f.calls, ["applying", "kernel", "failed"]);
        forget_combat(&mut f, 1).unwrap();
        assert_eq!(f.calls.len(), 3);
    }
    #[test]
    fn forget_unregisters_before_requirement_unsubscribe_and_clears_before_release() {
        let (mut f, p) = fixture(vec![], true);
        item_changed(&mut f, 7, p, true).unwrap();
        f.proposals
            .collection_modifiers_mut()
            .unwrap()
            .modifier_mut(7)
            .unwrap()
            .tracked[0]
            .state
            .requirement_set = Some(9);
        forget_combat(&mut f, 1).unwrap();
        assert_eq!(
            f.calls,
            ["applying", "kernel", "failed", "unsubscribe", "release"]
        );
    }
    #[test]
    fn unsupported_owned_modifier_preserves_unregistration_prefix() {
        let (mut f, p) = fixture(vec![], true);
        f.proposals.proposal_mut(p).unwrap().owned_modifiers.push(9);
        assert!(matches!(
            forget_combat(&mut f, 1),
            Err(Error::RemovalUnsupported)
        ));
        assert!(f.proposals.active().is_empty());
        assert_eq!(f.proposals.object_id(p).unwrap(), 1);
    }
    #[test]
    fn non_temporary_applied_removal_is_an_explicit_prefix_gap() {
        let (mut f, p) = fixture(vec![], true);
        item_changed(&mut f, 7, p, true).unwrap();
        f.proposals.proposal_mut(p).unwrap().temporary = false;
        assert!(matches!(
            forget_combat(&mut f, 1),
            Err(Error::RemovalUnsupported)
        ));
        assert!(f.proposals.active().is_empty());
        assert_eq!(
            f.proposals
                .collection_modifiers()
                .unwrap()
                .modifier(7)
                .unwrap()
                .tracked
                .len(),
            1
        );
    }
    #[test]
    fn canonical_apply_marks_before_kernel_and_duplicate_add_does_not_reapply() {
        let (mut f, p) = fixture(vec![], true);
        assert!(f.active);
        item_changed(&mut f, 7, p, true).unwrap();
        item_changed(&mut f, 7, p, true).unwrap();
        assert_eq!(f.calls, ["applying", "kernel", "failed"]);
        assert_eq!(
            f.proposals.proposal(p).unwrap().attacker_strength_modifier,
            5
        );
        let m = f
            .proposals
            .collection_modifiers()
            .unwrap()
            .modifier(7)
            .unwrap();
        assert_eq!(m.tracked.len(), 1);
        assert_eq!(m.applied, vec![p]);
    }
    #[test]
    fn inactive_modifier_never_constructs_subject_or_calls_queries() {
        let (mut f, p) = fixture(b"unknown requirement".to_vec(), false);
        assert!(!f.active);
        item_changed(&mut f, 7, p, true).unwrap();
        assert!(f.calls.is_empty());
        assert!(f
            .proposals
            .collection_modifiers()
            .unwrap()
            .modifier(7)
            .unwrap()
            .tracked
            .is_empty());
    }
    #[test]
    fn missing_subject_predicate_preserves_constructed_tracked_state_without_applying() {
        let (mut f, p) = fixture(b"real requirement".to_vec(), true);
        assert!(matches!(
            item_changed(&mut f, 7, p, true),
            Err(Error::Context("actual subject predicates unavailable"))
        ));
        assert_eq!(f.calls, ["requirements"]);
        let m = f
            .proposals
            .collection_modifiers()
            .unwrap()
            .modifier(7)
            .unwrap();
        assert_eq!(m.tracked.len(), 1);
        assert_eq!(m.tracked[0].state.applied, 0);
        assert_eq!(
            f.proposals.proposal(p).unwrap().attacker_strength_modifier,
            0
        );
    }
}
