//! GovernmentChanged requirement and Player.EventManager subscribers. Live
//! requirement state, IDs, child/set subscriptions and Culture remain canonical.
use crate::melee_attack::backend::requirements::Runtime;
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
};
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    /// Label in the SAME shared requirement owner, preserving captured identity.
    pub label: u64,
    /// Actual queried IPlayer interfaces; None means QueryInterface returned null.
    pub context_player: Option<i32>,
    pub subject_player: Option<i32>,
}
#[derive(Debug)]
pub enum Error<E = std::convert::Infallible> {
    State(c6_core::loaded_state::Error),
    Requirements(Box<sets::Error<Error<E>>>),
    Effects(E),
    Unsupported(&'static str),
}
/// Reached downstream callbacks receive the same live state/requirements. There
/// is no default that accepts positive modifier, release or trigger branches.
pub trait Continuations {
    type Error;
    fn modifier_changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut Runtime,
        listener: sets::SetListener,
        set: u64,
        old: i32,
        new: i32,
    ) -> Result<(), Self::Error>;
    fn destroy_requirement(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut Runtime,
        label: u64,
    ) -> Result<(), Self::Error>;
    fn destroy_set(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut Runtime,
        label: u64,
    ) -> Result<(), Self::Error>;
    fn reset_trigger(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut Runtime,
        set: u64,
    ) -> Result<(), Self::Error>;
}
/// An explicit unsupported continuation service, useful for proven empty rings.
/// A reached positive branch returns its error after preserving mutation prefix.
pub struct UnavailableContinuations;
impl Continuations for UnavailableContinuations {
    type Error = &'static str;
    fn modifier_changed(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), Self::Error> {
        Err("GovernmentChanged modifier continuation")
    }
    fn destroy_requirement(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: u64,
    ) -> Result<(), Self::Error> {
        Err("GovernmentChanged requirement destructor")
    }
    fn destroy_set(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: u64,
    ) -> Result<(), Self::Error> {
        Err("GovernmentChanged set destructor")
    }
    fn reset_trigger(
        &mut self,
        _: &mut LoadedState,
        _: &mut Runtime,
        _: u64,
    ) -> Result<(), Self::Error> {
        Err("GovernmentChanged trigger reset")
    }
}
fn filtered(receiver: Receiver, changed_player: i32) -> bool {
    matches!((receiver.context_player,receiver.subject_player),(Some(a),Some(b)) if a==changed_player||b==changed_player)
}
fn test_pair(context: Option<u32>, subject: Option<u32>) -> i32 {
    match (context, subject) {
        (Some(a), Some(b)) => i32::from(a != b),
        _ => 1,
    }
}
fn government<E>(state: &LoadedState, owner: i32) -> Result<u32, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .map(|p| p.culture.current_government_hash)
        .ok_or(Error::Unsupported(
            "PlayerHasSameGovernment current Player.Culture",
        ))
}
struct Context<'a, C> {
    state: &'a mut LoadedState,
    requirements: &'a mut Runtime,
    receiver: Receiver,
    continuations: &'a mut C,
}
impl<C: Continuations> changes::Context for Context<'_, C> {
    type Error = Error<C::Error>;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.requirements.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Self::Error>> {
        if label != self.receiver.label {
            return Err(sets::Error::Context(Error::Unsupported(
                "PlayerHasSameGovernment receiver identity",
            )));
        }
        let a = self
            .receiver
            .context_player
            .map(|p| government(self.state, p))
            .transpose()
            .map_err(sets::Error::Context)?;
        let b = self
            .receiver
            .subject_player
            .map(|p| government(self.state, p))
            .transpose()
            .map_err(sets::Error::Context)?;
        Ok(test_pair(a, b))
    }
    fn modifier_changed(
        &mut self,
        l: sets::SetListener,
        s: u64,
        o: i32,
        n: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .modifier_changed(self.state, self.requirements, l, s, o, n)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn destroy_requirement(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_requirement(self.state, self.requirements, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn destroy_set(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_set(self.state, self.requirements, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
    fn reset_trigger(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .reset_trigger(self.state, self.requirements, l)
            .map_err(|e| sets::Error::Context(Error::Effects(e)))
    }
}
/// Lambda2403ac0 -> OnGovernmentChanged23852f0. Both interfaces must exist;
/// changed Player must match either. The signal GovernmentTypes is not tested.
/// Import actual SimpleRequirement and listener records before this call.
pub fn changed<C: Continuations>(
    state: &mut LoadedState,
    requirements: &mut Runtime,
    receiver: Receiver,
    changed_player: i32,
    continuations: &mut C,
) -> Result<(), Error<C::Error>> {
    if !filtered(receiver, changed_player) {
        return Ok(());
    }
    let d = requirements
        .sets()
        .requirement_definition(receiver.label)
        .ok_or(Error::Unsupported(
            "current PlayerHasSameGovernment requirement record",
        ))?;
    if d.predicate
        != sets::Predicate::Unsupported(linux_type_hash(b"REQUIREMENT_PLAYER_HAS_SAME_GOVERNMENT"))
    {
        return Err(Error::Unsupported(
            "PlayerHasSameGovernment definition identity",
        ));
    }
    changes::invalidate(
        &mut Context {
            state,
            requirements,
            receiver,
            continuations,
        },
        receiver.label,
    )
    .map_err(|e| Error::Requirements(Box::new(e)))
}

struct InfluenceGovernment {
    points: i32,
    threshold: i32,
    tokens: i32,
}
/// Static rules read by Influence.OnGovernmentEnacted/Retracted, bound to the
/// actual current inputs and game speed. No native calculated totals are inputs.
pub struct InfluenceDefinitions {
    binding: WireBinding,
    governments: Vec<InfluenceGovernment>,
    levels: LinuxDefinitionRegistry,
    can_give: Vec<bool>,
    speed_cost_percent: i32,
}
impl InfluenceDefinitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules, game_speed: u32) -> Result<Self, Error> {
        let registry = |table, column| {
            crate::unit_initialization_definitions::collection(rules, table, column)
                .map_err(|_| Error::Unsupported("government Influence definition registry"))
        };
        let governments = registry("Governments", "GovernmentType")?;
        let speeds = registry("GameSpeeds", "GameSpeedType")?;
        let levels = registry("CivilizationLevels", "CivilizationLevelType")?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(Error::Unsupported("government Influence rules table"))
        };
        let integer = |r: crate::Row<'_>, name| {
            r.get(name)
                .ok()
                .and_then(crate::game_parameters::sql_integer)
                .ok_or(Error::Unsupported("government Influence integer"))
        };
        let speed_cost_percent = match speeds.find(game_speed) {
            Some(e) => integer(
                table("GameSpeeds")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("current GameSpeed row"))?,
                "CostMultiplier",
            )?,
            None => 100,
        };
        let government_rows = governments
            .entries()
            .iter()
            .map(|e| {
                let r = table("Governments")?
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Unsupported("current Government row"))?;
                Ok(InfluenceGovernment {
                    points: integer(r, "InfluencePointsPerTurn")?,
                    threshold: integer(r, "InfluencePointsThreshold")?,
                    tokens: integer(r, "InfluenceTokensPerThreshold")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let can_give = levels
            .entries()
            .iter()
            .map(|e| {
                integer(
                    table("CivilizationLevels")?
                        .row_by_id(e.database_rowid)
                        .ok_or(Error::Unsupported("current CivilizationLevel row"))?,
                    "CanGiveInfluence",
                )
                .map(|v| v != 0)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            governments: government_rows,
            levels,
            can_give,
            speed_cost_percent,
        })
    }
}
pub trait InfluenceEffects {
    type Error;
    /// Every setter emits d35e571d synchronously after its clamped assignment.
    fn report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        event: u32,
    ) -> Result<(), Self::Error>;
    /// Exact CheckSendTokenNotification2b20be0 semantic request, including its
    /// existing notification owner/refresh behavior when no popup is necessary.
    fn notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: u32,
    ) -> Result<(), Self::Error>;
}
fn add_nonnegative(value: i32, amount: i32) -> i32 {
    value.wrapping_add(amount).max(0)
}
/// EventManager2b015f0/2b01620 -> Influence2b234a0/2b23570. Each subscribed
/// Player filters against its own owner before resolving the Government row.
#[allow(clippy::too_many_arguments)]
pub fn player_event<E: InfluenceEffects>(
    state: &mut LoadedState,
    definitions: &InfluenceDefinitions,
    receiver_owner: i32,
    changed_player: Option<i32>,
    government: i32,
    enacted: bool,
    current_levels: &[(i32, u32)],
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    if changed_player != Some(receiver_owner) {
        return Ok(());
    }
    if state.binding() != &definitions.binding {
        return Err(Error::Unsupported("government Influence input binding"));
    }
    if government != -1 {
        let Some(row) = usize::try_from(government)
            .ok()
            .and_then(|i| definitions.governments.get(i))
        else {
            return Ok(());
        };
        let delta = |v: i32| if enacted { v } else { v.wrapping_neg() };
        let points = delta(row.points).wrapping_shl(8);
        {
            let influence = state
                .social_components_mut(receiver_owner)
                .map_err(Error::State)?
                .influence;
            influence.base_points_per_turn =
                add_nonnegative(influence.base_points_per_turn, points);
        }
        effects
            .report(state, receiver_owner, 0xd35e571d)
            .map_err(Error::Effects)?;
        let threshold = crate::melee_attack::resolve::kill_awards::adjust_speed(
            row.threshold,
            definitions.speed_cost_percent,
        );
        {
            let influence = state
                .social_components_mut(receiver_owner)
                .map_err(Error::State)?
                .influence;
            influence.points_threshold =
                add_nonnegative(influence.points_threshold, delta(threshold));
        }
        effects
            .report(state, receiver_owner, 0xd35e571d)
            .map_err(Error::Effects)?;
        {
            let influence = state
                .social_components_mut(receiver_owner)
                .map_err(Error::State)?
                .influence;
            influence.tokens_per_threshold =
                add_nonnegative(influence.tokens_per_threshold, delta(row.tokens));
        }
        effects
            .report(state, receiver_owner, 0xd35e571d)
            .map_err(Error::Effects)?;
    }
    // CheckForTokensEarned queries current IsAlive then current level's
    // CanGiveInfluence, before testing the threshold and current Q8 balance.
    if state.player_alive(receiver_owner).map_err(Error::State)? {
        let level = current_levels
            .iter()
            .find(|r| r.0 == receiver_owner)
            .ok_or(Error::Unsupported("current Influence CivilizationLevel"))?
            .1;
        if definitions
            .levels
            .find_index(level)
            .and_then(|i| definitions.can_give.get(i))
            .copied()
            .unwrap_or(false)
        {
            let p = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == receiver_owner)
                .ok_or(Error::Unsupported("current Influence Player"))?;
            if p.influence.points_threshold > 0
                && p.influence.points_earned >= p.influence.points_threshold.wrapping_shl(8)
            {
                return Err(Error::Unsupported(
                    "positive government Influence earned-token transition",
                ));
            }
        }
    }
    effects
        .notification(state, receiver_owner, 0x579b28d1)
        .map_err(Error::Effects)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_requires_both_interfaces_and_filters_either_player() {
        let r = Receiver {
            label: 1,
            context_player: Some(3),
            subject_player: Some(7),
        };
        assert!(filtered(r, 3) && filtered(r, 7));
        assert!(!filtered(r, 5));
        assert!(!filtered(
            Receiver {
                subject_player: None,
                ..r
            },
            3
        ));
        assert!(!filtered(
            Receiver {
                context_player: None,
                ..r
            },
            7
        ));
        assert_eq!(test_pair(Some(u32::MAX), Some(u32::MAX)), 0);
        assert_eq!(test_pair(Some(4), Some(5)), 1);
        assert_eq!(test_pair(None, Some(5)), 1);
    }
    #[test]
    fn influence_setters_clamp_after_wrapping_add_and_speed_adjustment() {
        assert_eq!(add_nonnegative(256, -256), 0);
        assert_eq!(add_nonnegative(0, -256), 0);
        assert_eq!(add_nonnegative(i32::MAX, 1), 0);
        assert_eq!(add_nonnegative(-1, 2), 1);
        assert_eq!(
            crate::melee_attack::resolve::kill_awards::adjust_speed(100, 67),
            67
        );
        assert_eq!(
            crate::melee_attack::resolve::kill_awards::adjust_speed(-1, 50),
            -1
        );
    }
}
