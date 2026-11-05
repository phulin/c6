//! Current inactive controller off-turn work on the same stock runtime.
//! This supports source-proven cold, cityless/unitless analysis guards. It
//! preserves the nonzero CollectorOne result and its status2 stop boundary.
use super::{human_activation::Kind, Runtime};
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_state::{game_update::inactive_control as core, LoadedState},
};
pub use core::Outcome;
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockNoAddedPlanningConsumers,
}
#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub culture_analyzed: Option<bool>,
    pub unit_production_calculated: Option<bool>,
    pub unit_efficiencies_count: Option<usize>,
    pub diplomatic_done: Option<bool>,
    pub military_analyzed: Option<bool>,
    pub economic_analyzed: Option<bool>,
}
pub struct Inputs {
    binding: WireBinding,
    pub owner: i32,
    pub skip_off_turn: bool,
    pub off_turn: Vec<Kind>,
    pub flags: Flags,
    failed: bool,
}
impl Inputs {
    pub fn from_initial(
        state: &LoadedState,
        binding: &WireBinding,
        owner: i32,
        skip_off_turn: bool,
        off_turn: Vec<Kind>,
        flags: Flags,
        _: Scope,
    ) -> Result<Self, Error> {
        if state.binding() != binding {
            return Err(Error::Unsupported("inactive Control initial binding"));
        }
        Ok(Self {
            binding: binding.clone(),
            owner,
            skip_off_turn,
            off_turn,
            flags,
            failed: false,
        })
    }
}
fn flag(value: Option<bool>, name: &'static str) -> Result<bool, Error> {
    value.ok_or(Error::Unsupported(name))
}
fn empty_cities(state: &LoadedState, owner: i32) -> Result<(), Error> {
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("inactive current player"))?;
    if !p.cities.records.entries.is_empty() {
        return Err(Error::Unsupported("inactive CityBuild/yield continuations"));
    }
    if p.ai_city_manager
        .as_ref()
        .is_some_and(|m| !m.city_ais.entries.is_empty())
    {
        return Err(Error::Unsupported("inactive retained CityBuild analysis"));
    }
    Ok(())
}
struct Effects<'s, 'a, P> {
    backend: &'s mut Runtime<'a, P>,
    input: &'s mut Inputs,
}
impl<P> Effects<'_, '_, P> {
    fn status(
        &mut self,
        state: &LoadedState,
        owner: i32,
        kind: Kind,
    ) -> Result<core::Status, Error> {
        use core::Status;
        match kind {
            Kind::City => {
                empty_cities(state, owner)?;
                Ok(Status::Complete)
            }
            Kind::Culture => {
                if flag(
                    self.input.flags.culture_analyzed,
                    "inactive current Culture readiness",
                )? {
                    return Err(Error::Unsupported(
                        "inactive current civic/policy analysis readiness",
                    ));
                }
                let calculated = flag(
                    self.input.flags.unit_production_calculated,
                    "inactive UnitProduction calculated",
                )?;
                if !calculated {
                    return Ok(Status::Complete);
                }
                if self
                    .input
                    .flags
                    .unit_efficiencies_count
                    .ok_or(Error::Unsupported(
                        "inactive UnitProduction efficiencies count",
                    ))?
                    == 0
                {
                    return Ok(Status::Complete);
                }
                Err(Error::Unsupported(
                    "inactive Culture off-turn analysis jobs",
                ))
            }
            Kind::Diplomatic => {
                if flag(
                    self.input.flags.diplomatic_done,
                    "inactive current Diplomatic done",
                )? {
                    return Ok(Status::Complete);
                }
                if !flag(
                    self.input.flags.military_analyzed,
                    "inactive current analyzed forces",
                )? {
                    return Ok(Status::Pending);
                }
                Err(Error::Unsupported(
                    "inactive Diplomatic analysis continuation",
                ))
            }
            Kind::Economic => {
                if flag(
                    self.input.flags.economic_analyzed,
                    "inactive current Economic analyzed",
                )? {
                    return Ok(Status::Complete);
                }
                // Native writes this readiness flag before AnalyzeIncome.
                self.input.flags.economic_analyzed = Some(true);
                empty_cities(state, owner)?;
                if !state
                    .world()
                    .player(owner)
                    .map_err(|_| Error::Unsupported("inactive budget owner"))?
                    .unit_records()
                    .map_err(|_| Error::Unsupported("inactive budget units"))?
                    .is_empty()
                {
                    return Err(Error::Unsupported("inactive current unit budget queries"));
                }
                // AnalyzeIncome20a44b0 stores only AI budget estimates. Its
                // no-city path reads retained maintenance, beliefs and envoy
                // yields, not Treasury setters/commands/signals or game RNG.
                // Policy budget totals and formatting are outside this scope.
                // Manager.DoTurn's continuation201fa80 is literalstatus2.
                Ok(Status::Yield)
            }
            _ => Err(Error::Unsupported("later inactive off-turn consumer")),
        }
    }
}
impl<P> core::Effects for Effects<'_, '_, P> {
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error> {
        let count = self.input.off_turn.len();
        core::collect((0..count).map(|index| self.status(state, owner, self.input.off_turn[index])))
    }

    fn wait_for_jobs(&mut self, state: &mut LoadedState, _: i32) -> Result<(), Error> {
        self.backend
            .check(state)
            .map_err(|_| Error::Unsupported("inactive stock execution context"))?;
        // The supported prefix submits no analysis jobs: City is empty,
        // Culture/Diplomatic return at readiness guards, Economic is scalar.
        // Existing stock human planning jobs have no retained gameplay/RNG
        // consumers in this exclusive external-policy execution context.
        self.backend
            .reached
            .push("inactive off-turn pending stored before stock job join");
        Ok(())
    }
}
pub fn run<P>(
    state: &mut LoadedState,
    backend: &mut Runtime<'_, P>,
    input: &mut Inputs,
) -> Result<Outcome, Error> {
    if input.failed {
        return Err(Error::Unsupported(
            "failed inactive diagnostic cannot be retried",
        ));
    }
    if state.binding() != &input.binding {
        return Err(Error::Unsupported("inactive Control runtime binding"));
    }
    backend
        .check(state)
        .map_err(|_| Error::Unsupported("inactive stock state binding"))?;
    let owner = input.owner;
    let skip = input.skip_off_turn;
    let result = core::run(state, owner, skip, &mut Effects { backend, input });
    if let Err(error) = &result {
        input.failed = true;
        backend.last_error = Some(format!("inactive control: {error:?}"));
    }
    result
}
