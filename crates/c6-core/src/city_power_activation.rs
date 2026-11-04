//! Linux city Power activation (0x28be6e0 -> 0x28be6f0).
//! Runtime definition indices are distinct from saved source hashes. Callers
//! resolve the latter through the actual definitions before constructing State.
//! Synchronous signal/report failures retain earlier mutations; this is not an
//! atomic action or a claim of complete city/turn or native parity.

pub mod gossip_delivery;
pub mod stock_signals;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Source {
    pub amount_raw_q8: i32,
    pub building: i32,
    pub project: i32,
}

/// Live fields used by Power.RecalculatePower, after source Read/default effects
/// and definition remapping. Other Power fields/lists are preserved by its owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub free_raw_q8: i32,
    pub temporary_raw_q8: i32,
    pub required_raw_q8: i32,
    pub modifier_required_raw_q8: i32,
    pub modified_free_percent: i32,
    pub ever_generated: bool,
    pub required_sources: Vec<Source>,
}
impl State {
    pub fn free_power_raw_q8(&self) -> i32 {
        if self.modified_free_percent > 0 {
            let percentage = self.modified_free_percent.wrapping_add(100).wrapping_shl(8);
            ((i64::from(self.free_raw_q8) * i64::from(percentage)) / 25_600) as i32
        } else {
            self.free_raw_q8
        }
    }
    pub fn missing_power_raw_q8(&self) -> i32 {
        self.required_raw_q8
            .wrapping_sub(self.free_power_raw_q8())
            .wrapping_sub(self.temporary_raw_q8)
    }
    /// The native total changes before its source entry. Allocation refusal
    /// therefore preserves that earlier total, just like a fallible continuation.
    pub fn change_required(
        &mut self,
        amount: i32,
        building: i32,
        project: i32,
    ) -> Result<(), std::collections::TryReserveError> {
        if (building != -1 && project != -1) || amount == 0 {
            return Ok(());
        }
        self.required_raw_q8 = self.required_raw_q8.wrapping_add(amount).max(0);
        if let Some(source) = self
            .required_sources
            .iter_mut()
            .find(|source| source.building == building && source.project == project)
        {
            source.amount_raw_q8 = source.amount_raw_q8.wrapping_add(amount).max(0);
        } else {
            self.required_sources.try_reserve(1)?;
            self.required_sources.push(Source {
                amount_raw_q8: amount.max(0),
                building,
                project,
            });
        }
        Ok(())
    }
}

/// Definition handles refer to admitted immutable collections. Building_XP2
/// iteration uses its native append order, not base-building order or row IDs.
pub trait Context {
    type Error;
    type Building: Copy;
    type Project: Copy;
    fn power(&mut self) -> Result<&mut State, Self::Error>;
    fn allocation_error(&mut self, error: std::collections::TryReserveError) -> Self::Error;
    fn owner_is_major(&mut self) -> Result<bool, Self::Error>;
    fn buildings(&mut self) -> Result<&[Self::Building], Self::Error>;
    fn required_building_power(&mut self, building: Self::Building) -> Result<i32, Self::Error>;
    /// Building_XP2.GetBuildingTypeHash -> base Building.Find -> collection index.
    fn base_building(&mut self, building: Self::Building) -> Result<Option<i32>, Self::Error>;
    fn has_building(&mut self, index: i32) -> Result<bool, Self::Error>;
    fn building_pillaged(&mut self, index: i32) -> Result<bool, Self::Error>;
    fn current_project(&mut self) -> Result<i32, Self::Error>;
    /// Base Project.Find -> type hash -> Project_XP2.Find. None is a proven miss.
    fn project_definition(&mut self, index: i32) -> Result<Option<Self::Project>, Self::Error>;
    fn required_project_power(&mut self, project: Self::Project) -> Result<i32, Self::Error>;
    fn game_present(&mut self) -> Result<bool, Self::Error>;
    /// Actual Game signal +0x450. It may reenter/mutate power; do not replace
    /// unknown subscribers with a successful no-op or silently omit RNG.
    fn generated_power_signal(&mut self, first_generation: bool) -> Result<(), Self::Error>;
    /// Event d210fbfb, resolving the current City identity at this native site.
    fn power_shortage_changed_report(&mut self) -> Result<(), Self::Error>;
}

fn add<C: Context>(c: &mut C, amount: i32, building: i32, project: i32) -> Result<(), C::Error> {
    c.power()?
        .change_required(amount, building, project)
        .map_err(|error| c.allocation_error(error))
}

pub fn recalculate<C: Context>(c: &mut C) -> Result<(), C::Error> {
    // Native samples old missing-power positivity AFTER ResetRequiredPower.
    let old_missing = {
        let power = c.power()?;
        power.required_raw_q8 = 0;
        power.required_sources.clear();
        power.missing_power_raw_q8() > 0
    };
    if c.owner_is_major()? {
        // Native caches the immutable collection End before Begin traversal.
        let count = c.buildings()?.len();
        for ordinal in 0..count {
            let definition = c.buildings()?[ordinal];
            let amount = c.required_building_power(definition)?.wrapping_shl(8);
            if amount > 0 {
                if let Some(index) = c.base_building(definition)? {
                    if c.has_building(index)? && !c.building_pillaged(index)? {
                        add(c, amount, index, -1)?;
                    }
                }
            }
        }
        let project_index = c.current_project()?;
        if project_index != -1 {
            if let Some(project) = c.project_definition(project_index)? {
                if c.required_project_power(project)? != 0 {
                    // Preserve the second native getter, even when fallible.
                    let amount = c.required_project_power(project)?.wrapping_shl(8);
                    add(c, amount, -1, project_index)?;
                }
            }
        }
        let modifier = c.power()?.modifier_required_raw_q8;
        if modifier != 0 {
            add(c, modifier, -1, -1)?;
        }
    }
    if c.game_present()? {
        let first_generation = {
            let power = c.power()?;
            !power.ever_generated
                && power
                    .free_power_raw_q8()
                    .wrapping_add(power.temporary_raw_q8)
                    > 0
        };
        // Native dispatches even when first_generation is false, including minors.
        c.generated_power_signal(first_generation)?;
        if !c.power()?.ever_generated && first_generation {
            c.power()?.ever_generated = true;
        }
    }
    if (c.power()?.missing_power_raw_q8() > 0) != old_missing {
        c.power_shortage_changed_report()?;
    }
    Ok(())
}

/// Computed join for the existing city combat/project reset prefix and mandatory
/// Power continuation. Both views must borrow the same live city owner. The
/// earlier build/growth/citizen stages must already have executed.
pub fn finish_city_activation<C>(
    c: &mut C,
) -> Result<(), <C as crate::player_turn_stage::CityCombatActivationContext>::Error>
where
    C: crate::player_turn_stage::CityCombatActivationContext
        + Context<Error = <C as crate::player_turn_stage::CityCombatActivationContext>::Error>,
{
    crate::player_turn_stage::restore_city_combat(c)?;
    *c.project_production()? = false;
    recalculate(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        state: State,
        major: bool,
        game: bool,
        log: Vec<String>,
        fail_signal: bool,
        reenter: bool,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                state: State {
                    free_raw_q8: 256,
                    temporary_raw_q8: 0,
                    required_raw_q8: 999,
                    modifier_required_raw_q8: 128,
                    modified_free_percent: 0,
                    ever_generated: false,
                    required_sources: vec![Source {
                        amount_raw_q8: 999,
                        building: 9,
                        project: -1,
                    }],
                },
                major: true,
                game: true,
                log: vec![],
                fail_signal: false,
                reenter: false,
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type Building = i32;
        type Project = i32;
        fn power(&mut self) -> Result<&mut State, Self::Error> {
            Ok(&mut self.state)
        }
        fn allocation_error(&mut self, _: std::collections::TryReserveError) -> Self::Error {
            "allocation"
        }
        fn owner_is_major(&mut self) -> Result<bool, Self::Error> {
            self.log.push("major".into());
            Ok(self.major)
        }
        fn buildings(&mut self) -> Result<&[i32], Self::Error> {
            Ok(&[3, 1, 2, 4])
        }
        fn required_building_power(&mut self, b: i32) -> Result<i32, Self::Error> {
            self.log.push(format!("required{b}"));
            Ok(if b == 4 { 0x0080_0000 } else { b })
        }
        fn base_building(&mut self, b: i32) -> Result<Option<i32>, Self::Error> {
            Ok(Some(b))
        }
        fn has_building(&mut self, b: i32) -> Result<bool, Self::Error> {
            Ok(b != 1)
        }
        fn building_pillaged(&mut self, b: i32) -> Result<bool, Self::Error> {
            Ok(b == 2)
        }
        fn current_project(&mut self) -> Result<i32, Self::Error> {
            Ok(7)
        }
        fn project_definition(&mut self, p: i32) -> Result<Option<i32>, Self::Error> {
            Ok(Some(p))
        }
        fn required_project_power(&mut self, _: i32) -> Result<i32, Self::Error> {
            self.log.push("project".into());
            Ok(-1)
        }
        fn game_present(&mut self) -> Result<bool, Self::Error> {
            Ok(self.game)
        }
        fn generated_power_signal(&mut self, first: bool) -> Result<(), Self::Error> {
            self.log
                .push(format!("signal:{first}:{}", self.state.ever_generated));
            if self.reenter {
                self.state.required_raw_q8 = 0;
            }
            if self.fail_signal {
                Err("signal")
            } else {
                Ok(())
            }
        }
        fn power_shortage_changed_report(&mut self) -> Result<(), Self::Error> {
            self.log.push("report".into());
            Ok(())
        }
    }
    #[test]
    fn rebuilds_in_native_order_and_preserves_project_negative_entry() {
        let mut f = Fixture::new();
        recalculate(&mut f).unwrap();
        assert_eq!(f.state.required_raw_q8, 640);
        assert_eq!(
            f.state.required_sources,
            vec![
                Source {
                    amount_raw_q8: 768,
                    building: 3,
                    project: -1
                },
                Source {
                    amount_raw_q8: 0,
                    building: -1,
                    project: 7
                },
                Source {
                    amount_raw_q8: 128,
                    building: -1,
                    project: -1
                }
            ]
        );
        assert_eq!(
            f.log,
            vec![
                "major",
                "required3",
                "required1",
                "required2",
                "required4",
                "project",
                "project",
                "signal:true:false",
                "report"
            ]
        );
        assert!(f.state.ever_generated);
    }
    #[test]
    fn resets_before_old_shortage_sample_and_skips_major_only_work() {
        let mut f = Fixture::new();
        f.major = false;
        recalculate(&mut f).unwrap();
        assert_eq!(f.log, vec!["major", "signal:true:false"]);
        assert_eq!(f.state.required_raw_q8, 0);
        assert!(f.state.required_sources.is_empty());
    }
    #[test]
    fn callback_failure_retains_prefix_but_not_history_or_later_report() {
        let mut f = Fixture::new();
        f.fail_signal = true;
        assert_eq!(recalculate(&mut f), Err("signal"));
        assert_eq!(f.state.required_raw_q8, 640);
        assert!(!f.state.ever_generated);
        assert!(!f.log.iter().any(|s| s == "report"));
    }
    #[test]
    fn callback_mutation_is_reread_for_final_shortage() {
        let mut f = Fixture::new();
        f.reenter = true;
        recalculate(&mut f).unwrap();
        assert!(f.state.ever_generated);
        assert!(!f.log.iter().any(|s| s == "report"));
    }
    #[test]
    fn no_game_skips_signal_and_history_but_keeps_shortage_report() {
        let mut f = Fixture::new();
        f.game = false;
        recalculate(&mut f).unwrap();
        assert!(!f.state.ever_generated);
        assert_eq!(f.log.last().unwrap(), "report");
    }
    #[test]
    fn wrapping_and_clamping_apply_separately_to_total_and_source() {
        let mut f = Fixture::new();
        f.state.required_raw_q8 = i32::MAX;
        f.state.required_sources.clear();
        f.state.change_required(1, 2, -1).unwrap();
        assert_eq!(f.state.required_raw_q8, 0);
        assert_eq!(f.state.required_sources[0].amount_raw_q8, 1);
        f.state.change_required(-3, 2, -1).unwrap();
        assert_eq!(f.state.required_sources[0].amount_raw_q8, 0);
        let before = f.state.clone();
        f.state.change_required(10, 1, 1).unwrap();
        assert_eq!(f.state, before);
    }
    #[test]
    fn free_percentage_uses_wrapped_q8_percentage_and_signed_division() {
        let mut f = Fixture::new();
        f.state.free_raw_q8 = -257;
        f.state.modified_free_percent = 50;
        assert_eq!(f.state.free_power_raw_q8(), -385);
        f.state.modified_free_percent = -50;
        assert_eq!(f.state.free_power_raw_q8(), -257);
        f.state.modified_free_percent = i32::MAX;
        let p = i32::MAX.wrapping_add(100).wrapping_shl(8);
        assert_eq!(
            f.state.free_power_raw_q8(),
            ((-257_i64 * i64::from(p)) / 25600) as i32
        );
    }
}
