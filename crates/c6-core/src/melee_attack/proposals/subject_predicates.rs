//! Fresh ConstRequirement predicates used by alliance combat modifiers.
//! These compute source state codes; RequirementSet construction/lifecycle,
//! inversion and definition argument lookup remain the caller's responsibility.
use super::registry::{self, Handle, Participant};
use crate::map::UnitReference;

pub const RELIGIOUS_COMBAT: u32 = 0x5e2f_7468;
pub const UNIT_VERSUS_UNIT: u32 = 0x58e7_998b;

pub trait Context: registry::Context {
    /// Actual IAlliance interface result. Unknown is an error, not None.
    fn alliance_pair(&mut self) -> Result<Option<(i32, i32)>, Self::Error>;
    /// Actual IProposedCombat interface result, over this same registry owner.
    fn subject_proposal(&mut self) -> Result<Option<Handle>, Self::Error>;
    fn valid_player(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn is_alive(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn is_minor(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn is_major(&mut self, player: i32) -> Result<bool, Self::Error>;
    /// Current Game.DiplomaticRelations native definition index, not a captured
    /// predicate answer. -1 means no war. Caller admits current definitions.
    fn war_type(&mut self, from: i32, to: i32) -> Result<i32, Self::Error>;
    /// Both religion getters must use the same source-qualified current
    /// identity domain. Saved hashes must not be cast to runtime ordinals.
    fn unit_religion(&mut self, unit: UnitReference) -> Result<i32, Self::Error>;
    fn founded_religion(&mut self, player: i32) -> Result<i32, Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Proposal(registry::Error),
}

fn members<C: registry::Context>(
    c: &C,
    h: Handle,
) -> Result<(Participant, Option<Participant>), Error<C::Error>> {
    let p = c
        .runtime()
        .map_err(Error::Context)?
        .proposal(h)
        .map_err(Error::Proposal)?;
    Ok((p.attacker, p.defender))
}
fn outsider(player: i32, pair: (i32, i32)) -> bool {
    player != pair.0 && player != pair.1
}

/// Player.Diplomacy.IsAtWarWith2aa4bb0 first requires the OTHER player to be
/// valid and alive. It does not add an alive check for the diplomacy owner.
fn at_war<C: Context>(c: &mut C, owner: i32, other: i32) -> Result<bool, Error<C::Error>> {
    if !c.valid_player(other).map_err(Error::Context)?
        || !c.is_alive(other).map_err(Error::Context)?
    {
        return Ok(false);
    }
    Ok(c.war_type(owner, other).map_err(Error::Context)? != -1)
}

/// AllianceAtWarWithCombatTarget.OnInitialize23af850: fixed2 or fixed3.
/// A valid outside attacker wins even if it is not major/minor or not at war;
/// only an INVALID outside attacker allows fallback to the defender.
pub fn alliance_at_war_with_combat_target<C: Context>(c: &mut C) -> Result<i32, Error<C::Error>> {
    let Some(pair) = c.alliance_pair().map_err(Error::Context)? else {
        return Ok(3);
    };
    let Some(h) = c.subject_proposal().map_err(Error::Context)? else {
        return Ok(3);
    };
    let (attacker, defender) = members(c, h)?;
    let first = c.owner(attacker).map_err(Error::Context)?;
    let selected = if outsider(first, pair) && c.valid_player(first).map_err(Error::Context)? {
        Some(first)
    } else {
        // Source only considers a Unit defender when the attacker Unit exists;
        // a District defender is tested independently.
        match defender {
            Some(Participant::Unit(_)) if !matches!(attacker, Participant::Unit(_)) => None,
            Some(p) => {
                let owner = c.owner(p).map_err(Error::Context)?;
                if outsider(owner, pair) && c.valid_player(owner).map_err(Error::Context)? {
                    Some(owner)
                } else {
                    None
                }
            }
            None => None,
        }
    };
    let Some(enemy) = selected else { return Ok(3) };
    if !c.is_minor(enemy).map_err(Error::Context)? && !c.is_major(enemy).map_err(Error::Context)? {
        return Ok(3);
    }
    if !at_war(c, enemy, pair.0)? {
        return Ok(3);
    }
    Ok(3 - i32::from(at_war(c, enemy, pair.1)?))
}

/// AllianceNotCombatTargetReligion.OnInitialize23b4910. Nonreligious combat
/// returns3 before owner/religion queries. Religious success is state0, NOT2.
pub fn alliance_not_combat_target_religion<C: Context>(c: &mut C) -> Result<i32, Error<C::Error>> {
    let Some(pair) = c.alliance_pair().map_err(Error::Context)? else {
        return Ok(3);
    };
    let Some(h) = c.subject_proposal().map_err(Error::Context)? else {
        return Ok(3);
    };
    let p = c
        .runtime()
        .map_err(Error::Context)?
        .proposal(h)
        .map_err(Error::Proposal)?;
    if p.combat_type != RELIGIOUS_COMBAT {
        return Ok(3);
    }
    let (attacker, defender) = (p.attacker, p.defender);
    let Participant::Unit(attacker) = attacker else {
        return Ok(3);
    };
    let owner = c
        .owner(Participant::Unit(attacker))
        .map_err(Error::Context)?;
    let enemy = if outsider(owner, pair) {
        attacker
    } else {
        let Some(Participant::Unit(defender)) = defender else {
            return Ok(3);
        };
        let owner = c
            .owner(Participant::Unit(defender))
            .map_err(Error::Context)?;
        if !outsider(owner, pair) {
            return Ok(3);
        }
        defender
    };
    let religion = c.unit_religion(enemy).map_err(Error::Context)?;
    // Both native GetValidPlayer calls precede either null check.
    let first = c.valid_player(pair.0).map_err(Error::Context)?;
    let second = c.valid_player(pair.1).map_err(Error::Context)?;
    if !first || !second {
        return Ok(3);
    }
    let a = c.founded_religion(pair.0).map_err(Error::Context)?;
    let b = c.founded_religion(pair.1).map_err(Error::Context)?;
    Ok(if religion == a || religion == b { 3 } else { 0 })
}

/// GetVersusType23b4d70 for the canonical constructors' disjoint participant
/// variants. All values are hashes, not guessed enum ordinals.
pub fn versus_type(attacker: Participant, defender: Option<Participant>) -> u32 {
    match (attacker, defender) {
        (Participant::Unit(_), Some(Participant::District(_))) => 0xc2f3_d823,
        (Participant::Unit(_), Some(Participant::Unit(_))) => UNIT_VERSUS_UNIT,
        (Participant::Unit(_), None) => 0xadac_056f,
        (Participant::District(_), Some(Participant::District(_))) => 0xc24a_4f19,
        (Participant::District(_), Some(Participant::Unit(_))) => 0x0682_86ea,
        (Participant::District(_), None) => 0xad15_9255,
    }
}
/// CombatVersusTypeMatches.OnInitialize23b4c90, after its argument hash lookup.
pub fn combat_versus_type_matches<C: Context>(
    c: &mut C,
    expected: u32,
) -> Result<i32, Error<C::Error>> {
    let Some(h) = c.subject_proposal().map_err(Error::Context)? else {
        return Ok(3);
    };
    let (a, b) = members(c, h)?;
    Ok(3 - i32::from(versus_type(a, b) == expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::ComponentId;
    struct Current {
        runtime: registry::Runtime,
        subject: Option<Handle>,
        pair: Option<(i32, i32)>,
        valid: Vec<i32>,
        alive: Vec<i32>,
        kinds: Vec<(i32, bool, bool)>,
        wars: Vec<((i32, i32), i32)>,
        unit_religions: Vec<(UnitReference, i32)>,
        founded: Vec<(i32, i32)>,
        reads: Vec<String>,
    }
    fn u(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            id,
            component_type: 1,
        })
    }
    fn d(owner: i16) -> ComponentId {
        ComponentId {
            owner,
            id: 27,
            component_type: 2,
        }
    }
    fn current(attacker: Participant, defender: Option<Participant>) -> Current {
        let mut runtime = registry::Runtime::from_initial(0, vec![], None, None, None).unwrap();
        let mut p = registry::Proposal::melee_units(u(3, 7), u(4, 9), None);
        p.attacker = attacker;
        p.defender = defender;
        let (_, h) = runtime.register(p).unwrap();
        Current {
            runtime,
            subject: Some(h),
            pair: Some((1, 2)),
            valid: vec![1, 2, 3, 4],
            alive: vec![1, 2],
            kinds: vec![(3, false, true), (4, false, true)],
            wars: vec![((3, 1), 0), ((3, 2), 5), ((4, 1), 0), ((4, 2), 5)],
            unit_religions: vec![(u(3, 7), 9), (u(4, 9), 8)],
            founded: vec![(1, 5), (2, 6)],
            reads: vec![],
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
        fn owner(&mut self, p: Participant) -> Result<i32, Self::Error> {
            let owner = match p {
                Participant::Unit(u) => u.component_id().owner,
                Participant::District(d) => d.owner,
            };
            self.reads.push(format!("owner{owner}"));
            Ok(i32::from(owner))
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, _: u64, _: Handle) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    impl Context for Current {
        fn alliance_pair(&mut self) -> Result<Option<(i32, i32)>, Self::Error> {
            self.reads.push("alliance".into());
            Ok(self.pair)
        }
        fn subject_proposal(&mut self) -> Result<Option<Handle>, Self::Error> {
            self.reads.push("subject".into());
            Ok(self.subject)
        }
        fn valid_player(&mut self, p: i32) -> Result<bool, Self::Error> {
            self.reads.push(format!("valid{p}"));
            Ok(self.valid.contains(&p))
        }
        fn is_alive(&mut self, p: i32) -> Result<bool, Self::Error> {
            self.reads.push(format!("alive{p}"));
            Ok(self.alive.contains(&p))
        }
        fn is_minor(&mut self, p: i32) -> Result<bool, Self::Error> {
            self.reads.push(format!("minor{p}"));
            self.kinds
                .iter()
                .find(|v| v.0 == p)
                .map(|v| v.1)
                .ok_or("missing current level")
        }
        fn is_major(&mut self, p: i32) -> Result<bool, Self::Error> {
            self.reads.push(format!("major{p}"));
            self.kinds
                .iter()
                .find(|v| v.0 == p)
                .map(|v| v.2)
                .ok_or("missing current level")
        }
        fn war_type(&mut self, a: i32, b: i32) -> Result<i32, Self::Error> {
            self.reads.push(format!("war{a}:{b}"));
            self.wars
                .iter()
                .find(|v| v.0 == (a, b))
                .map(|v| v.1)
                .ok_or("missing current war")
        }
        fn unit_religion(&mut self, u: UnitReference) -> Result<i32, Self::Error> {
            self.reads
                .push(format!("unit religion{}", u.component_id().owner));
            self.unit_religions
                .iter()
                .find(|v| v.0 == u)
                .map(|v| v.1)
                .ok_or("missing current unit religion")
        }
        fn founded_religion(&mut self, p: i32) -> Result<i32, Self::Error> {
            self.reads.push(format!("founded{p}"));
            self.founded
                .iter()
                .find(|v| v.0 == p)
                .map(|v| v.1)
                .ok_or("missing current founded religion")
        }
    }
    fn units() -> Current {
        current(Participant::Unit(u(3, 7)), Some(Participant::Unit(u(4, 9))))
    }
    #[test]
    fn missing_alliance_and_native_nonreligious_gate_do_not_query_religion() {
        let mut c = units();
        c.pair = None;
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 3);
        assert_eq!(c.reads, ["alliance"]);
        c.pair = Some((1, 2));
        c.reads.clear();
        c.unit_religions.clear();
        c.founded.clear();
        c.valid.clear();
        assert_eq!(alliance_not_combat_target_religion(&mut c).unwrap(), 3);
        assert_eq!(c.reads, ["alliance", "subject"]);
    }
    #[test]
    fn attacker_priority_war_targets_alive_and_lazy_second_war() {
        let mut c = units();
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 2);
        assert_eq!(
            c.reads,
            [
                "alliance", "subject", "owner3", "valid3", "minor3", "major3", "valid1", "alive1",
                "war3:1", "valid2", "alive2", "war3:2"
            ]
        );
        c.reads.clear();
        c.wars = vec![((3, 1), -1)];
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 3);
        assert!(!c.reads.iter().any(|v| v == "valid2" || v == "owner4"));
        c.reads.clear();
        c.alive = vec![2];
        c.wars.clear();
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 3);
        assert_eq!(c.reads.last().unwrap(), "alive1");
    }
    #[test]
    fn only_invalid_outside_attacker_falls_back_and_minor_skips_major() {
        let mut c = units();
        c.valid.retain(|p| *p != 3);
        c.kinds = vec![(4, true, false)];
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 2);
        assert!(c.reads.iter().any(|v| v == "owner4"));
        assert!(!c.reads.iter().any(|v| v == "major4"));
        c.reads.clear();
        c.valid.push(3);
        c.kinds = vec![(3, false, false)];
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 3);
        assert!(!c.reads.iter().any(|v| v == "owner4"));
    }
    #[test]
    fn district_attacker_preserves_native_unit_defender_guard() {
        let mut c = current(
            Participant::District(d(1)),
            Some(Participant::Unit(u(3, 7))),
        );
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 3);
        assert_eq!(c.reads, ["alliance", "subject", "owner1"]);
        c.runtime.proposal_mut(c.subject.unwrap()).unwrap().defender =
            Some(Participant::District(d(3)));
        assert_eq!(alliance_at_war_with_combat_target(&mut c).unwrap(), 2);
    }
    #[test]
    fn religious_enemy_selection_and_exact_zero_vs_three_states() {
        let mut c = units();
        c.runtime
            .proposal_mut(c.subject.unwrap())
            .unwrap()
            .combat_type = RELIGIOUS_COMBAT;
        assert_eq!(alliance_not_combat_target_religion(&mut c).unwrap(), 0);
        assert_eq!(
            c.reads,
            [
                "alliance",
                "subject",
                "owner3",
                "unit religion3",
                "valid1",
                "valid2",
                "founded1",
                "founded2"
            ]
        );
        c.reads.clear();
        c.founded[1].1 = 9;
        assert_eq!(alliance_not_combat_target_religion(&mut c).unwrap(), 3);
        c.runtime.proposal_mut(c.subject.unwrap()).unwrap().attacker = Participant::Unit(u(1, 7));
        c.unit_religions = vec![(u(4, 9), 8)];
        c.founded = vec![(1, 5), (2, 6)];
        c.reads.clear();
        assert_eq!(alliance_not_combat_target_religion(&mut c).unwrap(), 0);
        assert_eq!(&c.reads[..4], ["alliance", "subject", "owner1", "owner4"]);
    }
    #[test]
    fn religion_queries_both_player_validity_before_failure_and_unknown_is_error() {
        let mut c = units();
        c.runtime
            .proposal_mut(c.subject.unwrap())
            .unwrap()
            .combat_type = RELIGIOUS_COMBAT;
        c.valid = vec![2];
        c.founded.clear();
        assert_eq!(alliance_not_combat_target_religion(&mut c).unwrap(), 3);
        assert_eq!(&c.reads[c.reads.len() - 2..], ["valid1", "valid2"]);
        c.valid.push(1);
        assert!(matches!(
            alliance_not_combat_target_religion(&mut c),
            Err(Error::Context("missing current founded religion"))
        ));
    }
    #[test]
    fn versus_hashes_use_participant_shape_without_owner_or_alliance_reads() {
        let a = Participant::Unit(u(3, 7));
        let d = Participant::District(d(4));
        assert_eq!(versus_type(a, Some(a)), UNIT_VERSUS_UNIT);
        assert_eq!(versus_type(a, Some(d)), 0xc2f3_d823);
        assert_eq!(versus_type(a, None), 0xadac_056f);
        assert_eq!(versus_type(d, Some(d)), 0xc24a_4f19);
        assert_eq!(versus_type(d, Some(a)), 0x0682_86ea);
        assert_eq!(versus_type(d, None), 0xad15_9255);
        let mut c = units();
        c.pair = None;
        c.valid.clear();
        assert_eq!(
            combat_versus_type_matches(&mut c, UNIT_VERSUS_UNIT).unwrap(),
            2
        );
        assert_eq!(c.reads, ["subject"]);
    }
}
