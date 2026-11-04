//! AdjustPlayerStrengthModifier Apply23505f0 / Remove23508d0.
//! Requirement gating and interface/provider admission belong to the caller.
//! Native prefix writes are retained if a later current query is unsupported.
use super::registry;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameters {
    pub amount: i32,
    pub stack_scale: i32,
    pub maximum_property_adjustment: i32,
    /// Raw property hash; u32::MAX is the native no-property sentinel.
    pub property: u32,
    pub property_percentage_q8: i32,
    pub game_turn_scale: i32,
    pub divide_by_ever_alive_majors: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Effect {
    pub parameters: Parameters,
    /// One effect-owned total, not a separate total for each subject.
    pub applied_total: i32,
}

pub trait Context: registry::Context {
    /// None means that the actual context has no IPlayer interface.
    fn context_player(&mut self) -> Result<Option<i32>, Self::Error>;
    /// None means that the actual subject has no IProposedCombat interface.
    fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, Self::Error>;
    fn affected_by_count(&mut self) -> Result<i32, Self::Error>;
    /// Compute the target Linux powf result. An unvalidated math provider must
    /// return an error, never substitute a captured per-call oracle answer.
    fn stack_power(&mut self, base: f32, exponent: f32) -> Result<f32, Self::Error>;
    fn game_end_turn(&mut self) -> Result<i32, Self::Error>;
    fn game_start_turn(&mut self) -> Result<i32, Self::Error>;
    /// None is a source-defined missing interface/instance, not unknown data.
    fn unit_property(&mut self, key: u32) -> Result<Option<i32>, Self::Error>;
    fn player_property(&mut self, key: u32) -> Result<Option<i32>, Self::Error>;
    /// The source queries this interface even after a nonzero Unit/Player value.
    fn has_game_interface(&mut self) -> Result<bool, Self::Error>;
    /// None means that an existing IGame interface returns a null Game.
    fn game_property(&mut self, key: u32) -> Result<Option<i32>, Self::Error>;
    fn ever_alive_majors(&mut self) -> Result<i32, Self::Error>;
}

#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Proposal(registry::Error),
    DivisionByZero,
    DivisionOverflow,
}

fn native_div<E>(numerator: i32, denominator: i32) -> Result<i32, Error<E>> {
    if denominator == 0 {
        Err(Error::DivisionByZero)
    } else if numerator == i32::MIN && denominator == -1 {
        Err(Error::DivisionOverflow)
    } else {
        Ok(numerator / denominator)
    }
}

/// cvttss2si returns integer-indefinite on overflow/NaN; Rust's cast saturates.
fn truncate_f32(value: f32) -> i32 {
    if !value.is_finite() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

fn adjust<C: registry::Context>(
    c: &mut C,
    proposal: registry::Handle,
    player: i32,
    amount: i32,
) -> Result<(), Error<C::Error>> {
    let attacker = c
        .runtime()
        .map_err(Error::Context)?
        .proposal(proposal)
        .map_err(Error::Proposal)?
        .attacker;
    // Native PlayerIsAttacking checks only the attacker. Every other player,
    // including a nonparticipant, takes the defender-field branch.
    let attacking = c.owner(attacker).map_err(Error::Context)? == player;
    let proposal = c
        .runtime_mut()
        .map_err(Error::Context)?
        .proposal_mut(proposal)
        .map_err(Error::Proposal)?;
    let field = if attacking {
        &mut proposal.attacker_strength_modifier
    } else {
        &mut proposal.defender_strength_modifier
    };
    *field = field.wrapping_add(amount);
    Ok(())
}

/// Returns native true for a missing interface or a completed Apply. Queries
/// stay lazy, in source order; there is no rollback of an already applied term.
pub fn apply<C: Context>(c: &mut C, effect: &mut Effect) -> Result<bool, Error<C::Error>> {
    effect.applied_total = 0;
    let player = c.context_player().map_err(Error::Context)?;
    let proposal = c.subject_proposal().map_err(Error::Context)?;
    let (Some(player), Some(proposal)) = (player, proposal) else {
        return Ok(true);
    };
    let mut amount = effect.parameters.amount;
    if effect.parameters.stack_scale != 100 {
        let count = c.affected_by_count().map_err(Error::Context)?;
        if count > 0 {
            // Keep both mulss stages and the Linux powf call. This is not
            // amount*(scale/100)^count nor repeated integer multiplication.
            let scale = (effect.parameters.stack_scale as f32) * f32::from_bits(0x3c23_d70a);
            let base = scale * effect.parameters.amount as f32;
            let power = c.stack_power(base, count as f32).map_err(Error::Context)?;
            amount = amount.wrapping_add(truncate_f32(power));
        }
    }
    adjust(c, proposal, player, amount)?;
    effect.applied_total = effect.applied_total.wrapping_add(amount);
    let first_end = c.game_end_turn().map_err(Error::Context)?;
    let turn_amount = if first_end != 0 {
        let start = c.game_start_turn().map_err(Error::Context)?;
        let numerator = effect.parameters.game_turn_scale.wrapping_mul(start);
        let second_end = c.game_end_turn().map_err(Error::Context)?;
        native_div(numerator, second_end)?
    } else {
        0
    };
    adjust(c, proposal, player, turn_amount)?;
    effect.applied_total = effect.applied_total.wrapping_add(turn_amount);
    if effect.parameters.property == u32::MAX {
        return Ok(true);
    }
    let key = effect.parameters.property;
    let mut value = c.unit_property(key).map_err(Error::Context)?.unwrap_or(0);
    if value == 0 {
        value = c.player_property(key).map_err(Error::Context)?.unwrap_or(0);
    }
    let game_interface = c.has_game_interface().map_err(Error::Context)?;
    if value == 0 && game_interface {
        let Some(game_value) = c.game_property(key).map_err(Error::Context)? else {
            return Ok(true);
        };
        value = game_value;
    }
    if value == 0 {
        return Ok(true);
    }
    // Fixed8(percent)*Fixed8(100): signed64 product, shift8, low32 store.
    let mut percentage =
        ((i64::from(effect.parameters.property_percentage_q8) * 25_600) >> 8) as i32;
    if effect.parameters.divide_by_ever_alive_majors {
        let count = c.ever_alive_majors().map_err(Error::Context)?;
        let denominator = count.wrapping_shl(8);
        if denominator == 0 {
            return Err(Error::DivisionByZero);
        }
        percentage = ((i64::from(percentage) << 8) / i64::from(denominator)) as i32;
    }
    // ScaleByPercentage(nullptr) stores low32 without clamping; AsInt sar8.
    let scaled = ((i64::from(value.wrapping_shl(8)) * i64::from(percentage)) / 25_600) as i32;
    let property_amount = (scaled >> 8).min(effect.parameters.maximum_property_adjustment);
    adjust(c, proposal, player, property_amount)?;
    effect.applied_total = effect.applied_total.wrapping_add(property_amount);
    Ok(true)
}

/// Native Remove negates the stored total with wrapping and leaves it intact.
/// Repeated Remove therefore subtracts the same total again.
pub fn remove<C: Context>(c: &mut C, effect: &Effect) -> Result<bool, Error<C::Error>> {
    let player = c.context_player().map_err(Error::Context)?;
    let proposal = c.subject_proposal().map_err(Error::Context)?;
    if let (Some(player), Some(proposal)) = (player, proposal) {
        adjust(c, proposal, player, effect.applied_total.wrapping_neg())?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::UnitReference, unit::ComponentId};
    struct Test {
        runtime: registry::Runtime,
        handle: Option<registry::Handle>,
        player: Option<i32>,
        count: i32,
        ends: Vec<i32>,
        start: Result<i32, &'static str>,
        unit: Option<i32>,
        player_value: Option<i32>,
        game_interface: bool,
        game: Option<i32>,
        majors: i32,
        calls: Vec<&'static str>,
    }
    fn unit(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            id,
            component_type: 1,
        })
    }
    impl registry::Context for Test {
        type Error = &'static str;
        fn runtime(&self) -> Result<&registry::Runtime, Self::Error> {
            Ok(&self.runtime)
        }
        fn runtime_mut(&mut self) -> Result<&mut registry::Runtime, Self::Error> {
            Ok(&mut self.runtime)
        }
        fn owner(&mut self, p: registry::Participant) -> Result<i32, Self::Error> {
            self.calls.push("attacker owner");
            Ok(match p {
                registry::Participant::Unit(u) => i32::from(u.component_id().owner),
                registry::Participant::District(d) => i32::from(d.owner),
            })
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, _: u64, _: registry::Handle) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    impl Context for Test {
        fn context_player(&mut self) -> Result<Option<i32>, Self::Error> {
            self.calls.push("player interface");
            Ok(self.player)
        }
        fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, Self::Error> {
            self.calls.push("proposal interface");
            Ok(self.handle)
        }
        fn affected_by_count(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("affected count");
            Ok(self.count)
        }
        fn stack_power(&mut self, base: f32, exponent: f32) -> Result<f32, Self::Error> {
            self.calls.push("power");
            // Synthetic arithmetic cases only; not a target libm parity claim.
            Ok(base.powf(exponent))
        }
        fn game_end_turn(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("end");
            Ok(self.ends.remove(0))
        }
        fn game_start_turn(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("start");
            self.start
        }
        fn unit_property(&mut self, _: u32) -> Result<Option<i32>, Self::Error> {
            self.calls.push("unit property");
            Ok(self.unit)
        }
        fn player_property(&mut self, _: u32) -> Result<Option<i32>, Self::Error> {
            self.calls.push("player property");
            Ok(self.player_value)
        }
        fn has_game_interface(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("game interface");
            Ok(self.game_interface)
        }
        fn game_property(&mut self, _: u32) -> Result<Option<i32>, Self::Error> {
            self.calls.push("game property");
            Ok(self.game)
        }
        fn ever_alive_majors(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("majors");
            Ok(self.majors)
        }
    }
    fn current() -> (Test, Effect) {
        let mut runtime = registry::Runtime::from_initial(0, vec![], None, None, None).unwrap();
        let (_, handle) = runtime
            .register(registry::Proposal::melee_units(
                unit(3, 7),
                unit(8, 9),
                None,
            ))
            .unwrap();
        (
            Test {
                runtime,
                handle: Some(handle),
                player: Some(3),
                count: 0,
                ends: vec![0],
                start: Ok(0),
                unit: None,
                player_value: None,
                game_interface: false,
                game: None,
                majors: 1,
                calls: vec![],
            },
            Effect {
                parameters: Parameters {
                    amount: 10,
                    stack_scale: 100,
                    maximum_property_adjustment: i32::MAX,
                    property: u32::MAX,
                    property_percentage_q8: 256,
                    game_turn_scale: 0,
                    divide_by_ever_alive_majors: false,
                },
                applied_total: 123,
            },
        )
    }
    fn fields(c: &Test) -> (i32, i32) {
        let p = c.runtime.proposal(c.handle.unwrap()).unwrap();
        (p.attacker_strength_modifier, p.defender_strength_modifier)
    }
    #[test]
    fn actual_parameter_shape_applies_to_current_side_and_remove_keeps_total() {
        let (mut c, mut e) = current();
        e.parameters.amount = 5;
        assert!(apply(&mut c, &mut e).unwrap());
        assert_eq!(fields(&c), (5, 0));
        assert_eq!(e.applied_total, 5);
        assert_eq!(
            c.calls,
            [
                "player interface",
                "proposal interface",
                "attacker owner",
                "end",
                "attacker owner"
            ]
        );
        remove(&mut c, &e).unwrap();
        remove(&mut c, &e).unwrap();
        assert_eq!(fields(&c), (-5, 0));
        assert_eq!(e.applied_total, 5);
        c.player = Some(42);
        c.ends = vec![0];
        apply(&mut c, &mut e).unwrap();
        assert_eq!(fields(&c), (-5, 5)); // nonparticipant also takes defender
    }
    #[test]
    fn missing_interface_clears_apply_total_but_never_remove_total() {
        let (mut c, mut e) = current();
        c.player = None;
        apply(&mut c, &mut e).unwrap();
        assert_eq!(e.applied_total, 0);
        assert_eq!(c.calls, ["player interface", "proposal interface"]);
        e.applied_total = 9;
        remove(&mut c, &e).unwrap();
        assert_eq!(e.applied_total, 9);
        assert_eq!(fields(&c), (0, 0));
    }
    #[test]
    fn stack_term_uses_power_of_scaled_amount_and_native_invalid_conversion() {
        let (mut c, mut e) = current();
        e.parameters.amount = 4;
        e.parameters.stack_scale = 50;
        c.count = 2;
        apply(&mut c, &mut e).unwrap();
        assert_eq!(fields(&c), (8, 0)); // 4+powf((50*.01f)*4,2)
        assert_eq!(e.applied_total, 8);
        let (mut c, mut e) = current();
        e.parameters.stack_scale = 200;
        c.count = i32::MAX;
        apply(&mut c, &mut e).unwrap();
        assert_eq!(e.applied_total, 10_i32.wrapping_add(i32::MIN));
        assert_eq!(truncate_f32(f32::NAN), i32::MIN);
        assert_eq!(truncate_f32(2_147_483_648.0), i32::MIN);
        assert_eq!(truncate_f32(-1.9), -1);
    }
    #[test]
    fn turn_stage_rereads_denominator_and_preserves_prefix_on_fault() {
        let (mut c, mut e) = current();
        e.parameters.game_turn_scale = -7;
        c.ends = vec![9, 4];
        c.start = Ok(3);
        apply(&mut c, &mut e).unwrap();
        assert_eq!(fields(&c), (5, 0)); // 10+trunc(-21/4)
        assert_eq!(e.applied_total, 5);
        assert_eq!(
            c.calls,
            [
                "player interface",
                "proposal interface",
                "attacker owner",
                "end",
                "start",
                "end",
                "attacker owner"
            ]
        );
        c.ends = vec![1, 0];
        e.applied_total = 99;
        assert!(matches!(apply(&mut c, &mut e), Err(Error::DivisionByZero)));
        assert_eq!(fields(&c), (15, 0));
        assert_eq!(e.applied_total, 10);
        c.ends = vec![1];
        c.start = Err("unavailable current start");
        assert!(matches!(
            apply(&mut c, &mut e),
            Err(Error::Context("unavailable current start"))
        ));
        assert_eq!(fields(&c), (25, 0));
        assert_eq!(e.applied_total, 10);
    }
    #[test]
    fn property_zero_fallback_percentage_division_cap_and_game_null() {
        let (mut c, mut e) = current();
        e.parameters.property = 17;
        e.parameters.property_percentage_q8 = 128;
        e.parameters.divide_by_ever_alive_majors = true;
        e.parameters.maximum_property_adjustment = 2;
        c.unit = Some(0);
        c.player_value = Some(20);
        c.game_interface = true;
        c.majors = 2;
        apply(&mut c, &mut e).unwrap();
        assert_eq!(e.applied_total, 12); // min((20*.5)/2,2)
        assert!(!c.calls.contains(&"game property"));
        assert!(c.calls.contains(&"game interface"));
        assert_eq!(
            &c.calls[5..],
            [
                "unit property",
                "player property",
                "game interface",
                "majors",
                "attacker owner"
            ]
        );
        let (mut c, mut e) = current();
        e.parameters.property = 17;
        c.game_interface = true;
        c.game = None;
        apply(&mut c, &mut e).unwrap();
        assert_eq!(e.applied_total, 10);
        assert_eq!(c.calls.last(), Some(&"game property"));
    }
    #[test]
    fn negative_fraction_asint_wrap_and_zero_divisor_are_not_clamped() {
        let (mut c, mut e) = current();
        e.parameters.property = 17;
        e.parameters.property_percentage_q8 = 128;
        c.unit = Some(-3);
        apply(&mut c, &mut e).unwrap();
        assert_eq!(e.applied_total, 8); // AsInt(-1.5)=-2
        let (mut c, mut e) = current();
        e.parameters.property = 17;
        e.parameters.property_percentage_q8 = i32::MAX;
        c.unit = Some(i32::MAX);
        apply(&mut c, &mut e).unwrap();
        let p = e.parameters.property_percentage_q8.wrapping_mul(100);
        let raw = ((i64::from(i32::MAX.wrapping_shl(8)) * i64::from(p)) / 25_600) as i32;
        assert_eq!(e.applied_total, 10_i32.wrapping_add(raw >> 8));
        let (mut c, mut e) = current();
        e.parameters.property = 17;
        e.parameters.divide_by_ever_alive_majors = true;
        c.unit = Some(5);
        c.majors = 0;
        assert!(matches!(apply(&mut c, &mut e), Err(Error::DivisionByZero)));
        assert_eq!(fields(&c), (10, 0));
        assert_eq!(e.applied_total, 10);
        assert!(matches!(
            native_div::<()>(i32::MIN, -1),
            Err(Error::DivisionOverflow)
        ));
    }
}
