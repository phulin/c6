//! Linux Combat.Manager.TestAttackInto prefix and CanAttackDomain.
//! These are mandatory query stages, NOT a successful attack or resolution.
use crate::unit::PlotCoord;
pub mod damage;
pub mod experience;
pub mod proposals;
pub mod results;
pub mod strength;
pub mod target;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejected {
    NoAttacksRemaining,
    CannotAttack,
    FormationPlacement(i32),
}
#[derive(Debug, Eq, PartialEq)]
pub enum Prefix {
    Rejected(Rejected),
    RequiresTargetSelection(TargetSelection),
}
/// Internal readonly continuation. Defender/district/religious selection,
/// CanAttackTarget and complete AttackInto/Generate/Resolve remain mandatory.
#[derive(Debug, Eq, PartialEq)]
pub struct TargetSelection {
    destination: PlotCoord,
}
impl TargetSelection {
    pub fn destination(&self) -> PlotCoord {
        self.destination
    }
}

pub trait Context {
    type Error;
    fn attacks_remaining(&mut self) -> Result<i32, Self::Error>;
    fn can_attack(&mut self) -> Result<bool, Self::Error>;
    /// Native Formation overload, flags1, null output, INT_MAX distance and
    /// empty filter. The result must be EXACTLY1, not merely nonzero.
    fn formation_placement(&mut self, destination: PlotCoord) -> Result<i32, Self::Error>;
}
/// 28cc330..28cc398. No extra busy, movement, distance, domain or alive gate is
/// invented. The retained native Unit is queried in this exact short-circuit order.
pub fn test_attack_into_prefix<C: Context>(
    c: &mut C,
    destination: PlotCoord,
) -> Result<Prefix, C::Error> {
    if c.attacks_remaining()? <= 0 {
        return Ok(Prefix::Rejected(Rejected::NoAttacksRemaining));
    }
    if !c.can_attack()? {
        return Ok(Prefix::Rejected(Rejected::CannotAttack));
    }
    let result = c.formation_placement(destination)?;
    if result != 1 {
        return Ok(Prefix::Rejected(Rejected::FormationPlacement(result)));
    }
    Ok(Prefix::RequiresTargetSelection(TargetSelection {
        destination,
    }))
}

pub trait DomainContext {
    type Error;
    fn domain(&mut self) -> Result<i32, Self::Error>;
    fn bombard(&mut self) -> Result<i32, Self::Error>;
    fn ranged(&mut self) -> Result<i32, Self::Error>;
}
/// Rules.Combat.CanAttackDomain2be0720. The flags argument is unused in this
/// target. Non-air targets return true before ANY Unit read. Air targets require
/// an air attacker and read Bombard before Ranged, accepting signed equality.
pub fn can_attack_domain<C: DomainContext>(
    c: &mut C,
    target_domain: i32,
    _flags: u32,
) -> Result<bool, C::Error> {
    if target_domain != 1 {
        return Ok(true);
    }
    if c.domain()? != 1 {
        return Ok(false);
    }
    let bombard = c.bombard()?;
    let ranged = c.ranged()?;
    Ok(bombard <= ranged)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        attacks: i32,
        can: bool,
        placement: i32,
        domain: i32,
        bombard: i32,
        ranged: i32,
        calls: Vec<&'static str>,
        fail: Option<&'static str>,
    }
    impl Default for C {
        fn default() -> Self {
            Self {
                attacks: 1,
                can: true,
                placement: 1,
                domain: 2,
                bombard: 0,
                ranged: 0,
                calls: vec![],
                fail: None,
            }
        }
    }
    impl C {
        fn query<T>(&mut self, name: &'static str, value: T) -> Result<T, &'static str> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(name)
            } else {
                Ok(value)
            }
        }
    }
    impl Context for C {
        type Error = &'static str;
        fn attacks_remaining(&mut self) -> Result<i32, Self::Error> {
            self.query("attacks", self.attacks)
        }
        fn can_attack(&mut self) -> Result<bool, Self::Error> {
            self.query("can", self.can)
        }
        fn formation_placement(&mut self, _: PlotCoord) -> Result<i32, Self::Error> {
            self.query("placement", self.placement)
        }
    }
    impl DomainContext for C {
        type Error = &'static str;
        fn domain(&mut self) -> Result<i32, Self::Error> {
            self.query("domain", self.domain)
        }
        fn bombard(&mut self) -> Result<i32, Self::Error> {
            self.query("bombard", self.bombard)
        }
        fn ranged(&mut self) -> Result<i32, Self::Error> {
            self.query("ranged", self.ranged)
        }
    }
    const DESTINATION: PlotCoord = PlotCoord { x: 5, y: 7 };
    #[test]
    fn exhausted_attacks_and_nonattacker_preserve_native_short_circuits() {
        for attacks in [i32::MIN, -1, 0] {
            let mut c = C {
                attacks,
                fail: Some("can"),
                ..C::default()
            };
            assert_eq!(
                test_attack_into_prefix(&mut c, DESTINATION),
                Ok(Prefix::Rejected(Rejected::NoAttacksRemaining))
            );
            assert_eq!(c.calls, ["attacks"]);
        }
        let mut c = C {
            can: false,
            fail: Some("placement"),
            ..C::default()
        };
        assert_eq!(
            test_attack_into_prefix(&mut c, DESTINATION),
            Ok(Prefix::Rejected(Rejected::CannotAttack))
        );
        assert_eq!(c.calls, ["attacks", "can"]);
    }
    #[test]
    fn placement_requires_exact_one_and_success_only_requires_next_stage() {
        for placement in [i32::MIN, -1, 0, 2, i32::MAX] {
            let mut c = C {
                placement,
                ..C::default()
            };
            assert_eq!(
                test_attack_into_prefix(&mut c, DESTINATION),
                Ok(Prefix::Rejected(Rejected::FormationPlacement(placement)))
            );
            assert_eq!(c.calls, ["attacks", "can", "placement"]);
        }
        let mut c = C::default();
        let Prefix::RequiresTargetSelection(next) =
            test_attack_into_prefix(&mut c, DESTINATION).unwrap()
        else {
            panic!("missing continuation")
        };
        assert_eq!(next.destination(), DESTINATION);
    }
    #[test]
    fn unsupported_placement_propagates_without_fabricating_attack_acceptance() {
        let mut c = C {
            fail: Some("placement"),
            ..C::default()
        };
        assert_eq!(
            test_attack_into_prefix(&mut c, DESTINATION),
            Err("placement")
        );
        assert_eq!(c.calls, ["attacks", "can", "placement"]);
    }
    #[test]
    fn nonair_target_never_reads_actor_air_target_rejects_nonair_actor_early() {
        let mut c = C {
            fail: Some("domain"),
            ..C::default()
        };
        for domain in [-1, 0, 2, 3, i32::MAX] {
            assert_eq!(can_attack_domain(&mut c, domain, u32::MAX), Ok(true));
        }
        assert!(c.calls.is_empty());
        let mut c = C {
            fail: Some("bombard"),
            ..C::default()
        };
        assert_eq!(can_attack_domain(&mut c, 1, 0), Ok(false));
        assert_eq!(c.calls, ["domain"]);
    }
    #[test]
    fn air_comparison_is_signed_and_orders_bombard_before_ranged() {
        for (bombard, ranged, expected) in [
            (0, 0, true),
            (1, 0, false),
            (i32::MIN, -1, true),
            (i32::MAX, i32::MIN, false),
        ] {
            let mut c = C {
                domain: 1,
                bombard,
                ranged,
                ..C::default()
            };
            assert_eq!(can_attack_domain(&mut c, 1, 0), Ok(expected));
            assert_eq!(c.calls, ["domain", "bombard", "ranged"]);
        }
        let mut c = C {
            domain: 1,
            fail: Some("bombard"),
            ..C::default()
        };
        assert_eq!(can_attack_domain(&mut c, 1, 0), Err("bombard"));
        assert_eq!(c.calls, ["domain", "bombard"]);
    }
}
