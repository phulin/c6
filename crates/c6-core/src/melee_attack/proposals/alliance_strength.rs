//! AdjustAlliancePlayerStrengthModifier, separate from plain player strength.
//! Requirements, Before/After and affected/applied bookkeeping belong to the
//! modifier lifecycle. This kernel uses the one canonical current proposal.
use super::{alliance, player_strength, registry};

pub trait Context: registry::Context {
    /// Actual effect context QueryInterface<IAllianceInterface> presence.
    fn context_alliance_interface(&mut self) -> Result<bool, Self::Error>;
    /// Actual subject QueryInterface<IProposedCombatInterface> result.
    fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, Self::Error>;
    /// Current IAlliance.GetAlliance tuple's first two players, in stored order.
    /// These are effect-owner fields, not collection cached-player substitutes.
    fn alliance_players(&mut self) -> Result<(i32, i32), Self::Error>;
    fn amount(&mut self) -> Result<i32, Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    HasPlayer(alliance::Error<E>),
    Adjust(player_strength::Error<E>),
}

/// 23a1b40. Both interface queries precede the null guards. HasPlayer examines
/// the first stored alliance player; false selects the second even if neither
/// is a participant. AdjustPlayer then rereads CURRENT attacker ownership.
pub fn apply<C: Context>(context: &mut C) -> Result<bool, Error<C::Error>> {
    change(context, false)
}
/// 23a1bc0: wrapping negation of CURRENT amount and fresh tuple/side selection.
/// No saved applied-total exists in this class; repeated Remove repeats writes.
pub fn remove<C: Context>(context: &mut C) -> Result<bool, Error<C::Error>> {
    change(context, true)
}
fn change<C: Context>(c: &mut C, remove: bool) -> Result<bool, Error<C::Error>> {
    let alliance = c.context_alliance_interface().map_err(Error::Context)?;
    let proposal = c.subject_proposal().map_err(Error::Context)?;
    let (true, Some(proposal)) = (alliance, proposal) else {
        return Ok(true);
    };
    let (first, second) = c.alliance_players().map_err(Error::Context)?;
    let first_present = alliance::has_player(c, proposal, first).map_err(Error::HasPlayer)?;
    let amount = c.amount().map_err(Error::Context)?;
    let amount = if remove {
        amount.wrapping_neg()
    } else {
        amount
    };
    let player = if first_present { first } else { second };
    player_strength::adjust(c, proposal, player, amount).map_err(Error::Adjust)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::UnitReference, unit::ComponentId};
    struct Current {
        runtime: registry::Runtime,
        proposal: registry::Handle,
        context_present: bool,
        subject_present: bool,
        players: (i32, i32),
        amount: i32,
        calls: Vec<&'static str>,
        owners: Vec<i32>,
        amount_error: bool,
    }
    fn unit(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            component_type: 1,
            id,
        })
    }
    impl Current {
        fn new(players: (i32, i32), amount: i32) -> Self {
            let mut runtime = registry::Runtime::from_initial(0, vec![], None, None, None).unwrap();
            let (_, proposal) = runtime
                .register(registry::Proposal::melee_units(
                    unit(12, 1),
                    unit(62, 2),
                    None,
                ))
                .unwrap();
            Self {
                runtime,
                proposal,
                context_present: true,
                subject_present: true,
                players,
                amount,
                calls: vec![],
                owners: vec![],
                amount_error: false,
            }
        }
        fn strengths(&self) -> (i32, i32) {
            let p = self.runtime.proposal(self.proposal).unwrap();
            (p.attacker_strength_modifier, p.defender_strength_modifier)
        }
    }
    impl registry::Context for Current {
        type Error = &'static str;
        fn runtime(&self) -> Result<&registry::Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn runtime_mut(&mut self) -> Result<&mut registry::Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn owner(&mut self, p: registry::Participant) -> Result<i32, Self::Error> {
            let registry::Participant::Unit(unit) = p else {
                unreachable!()
            };
            let owner = i32::from(unit.component_id().owner);
            self.owners.push(owner);
            Ok(owner)
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, _: u64, _: registry::Handle) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    impl Context for Current {
        fn context_alliance_interface(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("context");
            Ok(self.context_present)
        }
        fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, Self::Error> {
            self.calls.push("subject");
            Ok(self.subject_present.then_some(self.proposal))
        }
        fn alliance_players(&mut self) -> Result<(i32, i32), Self::Error> {
            self.calls.push("players");
            Ok(self.players)
        }
        fn amount(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("amount");
            if self.amount_error {
                Err("amount")
            } else {
                Ok(self.amount)
            }
        }
    }
    #[test]
    fn second_alliance_member_is_selected_then_current_attacker_owner_is_reread() {
        let mut c = Current::new((3, 12), 5);
        assert!(apply(&mut c).unwrap());
        assert_eq!(c.strengths(), (5, 0));
        assert_eq!(c.owners, [12, 62, 12]);
        assert_eq!(c.calls, ["context", "subject", "players", "amount"]);
    }
    #[test]
    fn missing_first_interface_still_queries_subject_but_never_reads_tuple() {
        let mut c = Current::new((3, 12), 5);
        c.context_present = false;
        assert!(apply(&mut c).unwrap());
        assert_eq!(c.calls, ["context", "subject"]);
        assert_eq!(c.strengths(), (0, 0));
    }
    #[test]
    fn first_player_defending_or_neither_player_present_writes_defender() {
        let mut c = Current::new((62, 12), 5);
        apply(&mut c).unwrap();
        assert_eq!(c.strengths(), (0, 5));
        c.players = (3, 4);
        apply(&mut c).unwrap();
        assert_eq!(c.strengths(), (0, 10));
    }
    #[test]
    fn remove_uses_current_tuple_amount_and_wraps_without_an_applied_total() {
        let mut c = Current::new((3, 12), 5);
        apply(&mut c).unwrap();
        c.players = (4, 62);
        c.amount = 7;
        remove(&mut c).unwrap();
        remove(&mut c).unwrap();
        assert_eq!(c.strengths(), (5, -14));
        c.amount = i32::MIN;
        remove(&mut c).unwrap();
        assert_eq!(c.strengths(), (5, (-14i32).wrapping_add(i32::MIN)));
    }
    #[test]
    fn reached_amount_error_does_not_write_a_strength_field() {
        let mut c = Current::new((3, 12), 5);
        c.amount_error = true;
        assert!(matches!(apply(&mut c), Err(Error::Context("amount"))));
        assert_eq!(c.owners, [12, 62]);
        assert_eq!(c.strengths(), (0, 0));
    }
}
