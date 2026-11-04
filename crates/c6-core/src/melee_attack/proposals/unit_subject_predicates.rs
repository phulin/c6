//! Constant proposed-combat predicates over current interfaces and participants.
//! Definition argument lookup, inverse and ConstRequirement state storage belong
//! to the RequirementSet caller. These leaves never apply combat or choose policy.
use super::registry::{self, Handle, Participant};
use crate::map::UnitReference;

pub const OPPONENT_PROMOTION_CLASS_MATCHES: u32 = 0x42da_4161;
pub const PLAYER_IS_ATTACKING: u32 = 0xc941_6d41;

pub trait Context: registry::Context {
    type PlayerInterface: Copy;
    fn context_player_interface(&mut self) -> Result<Option<Self::PlayerInterface>, Self::Error>;
    fn subject_proposal_interface(&mut self) -> Result<Option<Handle>, Self::Error>;
    fn context_player_id(&mut self, interface: Self::PlayerInterface) -> Result<i32, Self::Error>;
    /// Nullable current Unit definition promotion text. Native strlen ends at
    /// its first NUL; SQL NULL/missing definition is distinct from empty text.
    fn promotion_class(&mut self, unit: UnitReference) -> Result<Option<&[u8]>, Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Proposal(registry::Error),
}
type Interfaces<C> = Option<(<C as Context>::PlayerInterface, Handle)>;

fn interfaces<C: Context>(c: &mut C) -> Result<Interfaces<C>, Error<C::Error>> {
    // Both QueryInterface calls precede either null check in both initializers.
    let player = c.context_player_interface().map_err(Error::Context)?;
    let proposal = c.subject_proposal_interface().map_err(Error::Context)?;
    Ok(player.zip(proposal))
}
fn participants<C: Context>(
    c: &C,
    proposal: Handle,
) -> Result<(Participant, Option<Participant>), Error<C::Error>> {
    let p = c
        .runtime()
        .map_err(Error::Context)?
        .proposal(proposal)
        .map_err(Error::Proposal)?;
    Ok((p.attacker, p.defender))
}
/// PlayerIsAttacking.OnInitialize2385e10. Native PlayerIsAttacking236d660
/// compares the current attacker owner; a nonparticipant is false. Canonical
/// proposal constructors have disjoint Unit/District attacker members.
pub fn player_is_attacking<C: Context>(c: &mut C) -> Result<i32, Error<C::Error>> {
    let Some((player, proposal)) = interfaces(c)? else {
        return Ok(3);
    };
    let player = c.context_player_id(player).map_err(Error::Context)?;
    let (attacker, _) = participants(c, proposal)?;
    Ok(3 - i32::from(c.owner(attacker).map_err(Error::Context)? == player))
}
/// OpponentUnitPromotionClassMatches.OnInitialize237bba0, after the caller's
/// exact GetStringValue("UnitPromotionClass", "") lookup. Any player who does
/// not own the attacker takes the attacker-Unit branch, including outsiders.
pub fn opponent_promotion_class_matches<C: Context>(
    c: &mut C,
    expected: &[u8],
) -> Result<i32, Error<C::Error>> {
    let Some((player, proposal)) = interfaces(c)? else {
        return Ok(3);
    };
    let player = c.context_player_id(player).map_err(Error::Context)?;
    let (attacker, defender) = participants(c, proposal)?;
    let attacking = c.owner(attacker).map_err(Error::Context)? == player;
    let opponent = if attacking { defender } else { Some(attacker) };
    let Some(Participant::Unit(unit)) = opponent else {
        return Ok(3);
    };
    let Some(value) = c.promotion_class(unit).map_err(Error::Context)? else {
        return Ok(3);
    };
    let length = value.iter().position(|b| *b == 0).unwrap_or(value.len());
    Ok(3 - i32::from(&value[..length] == expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::ComponentId;
    struct Current {
        runtime: registry::Runtime,
        subject: Option<Handle>,
        player: Option<i32>,
        attacker_owner: i32,
        classes: Vec<(UnitReference, Option<Vec<u8>>)>,
        reads: Vec<&'static str>,
        fail_promotion: bool,
    }
    fn unit(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            component_type: 1,
            id,
        })
    }
    fn current() -> Current {
        let a = unit(0, 1);
        let b = unit(1, 2);
        let mut runtime = registry::Runtime::from_initial(0, vec![], None, None, None).unwrap();
        let (_, subject) = runtime
            .register(registry::Proposal::melee_units(a, b, None))
            .unwrap();
        Current {
            runtime,
            subject: Some(subject),
            player: Some(0),
            attacker_owner: 0,
            classes: vec![
                (a, Some(b"PROMOTION_CLASS_MELEE".to_vec())),
                (b, Some(b"PROMOTION_CLASS_ANTI_CAVALRY".to_vec())),
            ],
            reads: vec![],
            fail_promotion: false,
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
        fn owner(&mut self, _: Participant) -> Result<i32, Self::Error> {
            self.reads.push("owner");
            Ok(self.attacker_owner)
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, _: u64, _: Handle) -> Result<(), Self::Error> {
            Err("unused callback")
        }
    }
    impl Context for Current {
        type PlayerInterface = i32;
        fn context_player_interface(&mut self) -> Result<Option<i32>, Self::Error> {
            self.reads.push("context");
            Ok(self.player)
        }
        fn subject_proposal_interface(&mut self) -> Result<Option<Handle>, Self::Error> {
            self.reads.push("subject");
            Ok(self.subject)
        }
        fn context_player_id(&mut self, p: i32) -> Result<i32, Self::Error> {
            self.reads.push("player-id");
            Ok(p)
        }
        fn promotion_class(&mut self, u: UnitReference) -> Result<Option<&[u8]>, Self::Error> {
            self.reads.push("promotion");
            if self.fail_promotion {
                return Err("missing promotion input");
            }
            Ok(self
                .classes
                .iter()
                .find(|p| p.0 == u)
                .ok_or("unknown current Unit")?
                .1
                .as_deref())
        }
    }
    #[test]
    fn both_interfaces_are_queried_before_missing_short_circuit() {
        let mut c = current();
        c.player = None;
        assert_eq!(opponent_promotion_class_matches(&mut c, b"X").unwrap(), 3);
        assert_eq!(c.reads, ["context", "subject"]);
        c.player = Some(0);
        c.subject = None;
        c.reads.clear();
        assert_eq!(player_is_attacking(&mut c).unwrap(), 3);
        assert_eq!(c.reads, ["context", "subject"]);
    }
    #[test]
    fn attacker_uses_defender_but_defender_and_outsider_use_attacker() {
        let mut c = current();
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"PROMOTION_CLASS_ANTI_CAVALRY").unwrap(),
            2
        );
        c.player = Some(1);
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"PROMOTION_CLASS_MELEE").unwrap(),
            2
        );
        c.player = Some(7);
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"PROMOTION_CLASS_MELEE").unwrap(),
            2
        );
        assert_eq!(player_is_attacking(&mut c).unwrap(), 3);
    }
    #[test]
    fn current_owner_is_read_instead_of_component_id_owner() {
        let mut c = current();
        c.attacker_owner = 7;
        c.player = Some(7);
        assert_eq!(player_is_attacking(&mut c).unwrap(), 2);
        assert_eq!(c.reads, ["context", "subject", "player-id", "owner"]);
    }
    #[test]
    fn district_opponent_is_not_substituted_with_a_unit() {
        let mut c = current();
        c.runtime.proposal_mut(c.subject.unwrap()).unwrap().defender =
            Some(Participant::District(ComponentId {
                owner: 1,
                component_type: 2,
                id: 6,
            }));
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"PROMOTION_CLASS_MELEE").unwrap(),
            3
        );
        assert!(!c.reads.contains(&"promotion"));
    }
    #[test]
    fn nullable_empty_cstring_and_exact_argument_bytes_differ() {
        let mut c = current();
        c.classes[1].1 = None;
        assert_eq!(opponent_promotion_class_matches(&mut c, b"").unwrap(), 3);
        c.classes[1].1 = Some(vec![]);
        assert_eq!(opponent_promotion_class_matches(&mut c, b"").unwrap(), 2);
        c.classes[1].1 = Some(b"ANTI\0ignored".to_vec());
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"ANTI").unwrap(),
            2
        );
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"ANTI\0").unwrap(),
            3
        );
        assert_eq!(
            opponent_promotion_class_matches(&mut c, b"anti").unwrap(),
            3
        );
    }
    #[test]
    fn missing_current_definition_input_is_error_not_false() {
        let mut c = current();
        c.fail_promotion = true;
        assert!(matches!(
            opponent_promotion_class_matches(&mut c, b"X"),
            Err(Error::Context("missing promotion input"))
        ));
    }
}
