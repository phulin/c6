//! External replacement for the reached empty-queue technology/civic decisions.
//! The caller retains the activation continuation; this does not repeat upkeep,
//! select a research target, consume a native policy draw or complete the turn.
use super::{Context, Effects, Error};
#[derive(Debug, Eq, PartialEq)]
pub enum ResearchPolicy {
    Continue,
    ChoicePending {
        player: i32,
        legal_indices: Vec<i32>,
    },
}
/// The caller retains the decision kind in its activation continuation.
pub type TechnologyPolicy = ResearchPolicy;
pub type CivicPolicy = ResearchPolicy;

/// Explicit external response to a selection request nested in an automatic
/// acquisition. DEFER does not assert that no legal technology exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionResponse {
    Unbound,
    Defer,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeferredAcquisition {
    pub player: i32,
    pub legal_indices: Vec<i32>,
}
#[derive(Default)]
pub(crate) struct AcquisitionPolicies {
    pub(crate) defer_players: Vec<i32>,
    pub(crate) decisions: Vec<DeferredAcquisition>,
}
impl AcquisitionPolicies {
    pub(crate) fn respond(&mut self, player: i32, legal_indices: &[i32]) -> AcquisitionResponse {
        if !self.defer_players.contains(&player) {
            return AcquisitionResponse::Unbound;
        }
        self.decisions.push(DeferredAcquisition {
            player,
            legal_indices: legal_indices.to_vec(),
        });
        AcquisitionResponse::Defer
    }
}
fn boundary<E>(
    player: i32,
    queued: bool,
    legal: impl FnOnce() -> Result<Vec<i32>, E>,
) -> Result<TechnologyPolicy, E> {
    if queued {
        return Ok(TechnologyPolicy::Continue);
    }
    let legal_indices = legal()?;
    Ok(if legal_indices.is_empty() {
        TechnologyPolicy::Continue
    } else {
        TechnologyPolicy::ChoicePending {
            player,
            legal_indices,
        }
    })
}
impl<E: Effects> Context<'_, E> {
    /// Current queue head after source GetIndex normalization; an opaque or
    /// invalid saved hash never satisfies a pending external selection.
    pub fn selected_technology(&self, player: i32) -> Result<Option<i32>, Error<E::Error>> {
        self.check()?;
        let current = super::research_turn::current(self, player, super::Kind::Technology)?;
        Ok((current >= 0).then_some(current))
    }
    pub fn technology_choice_satisfied(
        &self,
        player: i32,
        selected_index: i32,
    ) -> Result<bool, Error<E::Error>> {
        Ok(self.selected_technology(player)? == Some(selected_index))
    }
    pub fn selected_civic(&self, player: i32) -> Result<Option<i32>, Error<E::Error>> {
        self.check()?;
        let current = super::research_turn::current(self, player, super::Kind::Civic)?;
        Ok((current >= 0).then_some(current))
    }
    pub fn civic_choice_satisfied(
        &self,
        player: i32,
        selected_index: i32,
    ) -> Result<bool, Error<E::Error>> {
        Ok(self.selected_civic(player)? == Some(selected_index))
    }
    /// Source QueueNextCivic frontier after its human/barbarian bypass. The
    /// caller resumes this same activation with request_civic(...Replace).
    pub fn civic_policy_boundary(&self, player: i32) -> Result<CivicPolicy, Error<E::Error>> {
        self.check()?;
        let queued = self.selected_civic(player)?.is_some();
        boundary(player, queued, || self.civic_choices(player))
    }
    /// Invoke at the actual empty-queue controller policy stage, after the
    /// native human bypass. Missing legality inputs remain errors. Resume with
    /// request_technology(...,InsertMode::Replace), then continue AFTER this
    /// stage; retain the same state/research/script owners throughout.
    pub fn technology_policy_boundary(
        &self,
        player: i32,
    ) -> Result<TechnologyPolicy, Error<E::Error>> {
        self.check()?;
        let queued = self.selected_technology(player)?.is_some();
        boundary(player, queued, || self.technology_choices(player))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acquisition_defer_is_explicit_owner_scoped_and_retains_legal_choices() {
        let mut policies = AcquisitionPolicies::default();
        assert_eq!(policies.respond(63, &[4, 1]), AcquisitionResponse::Unbound);
        assert!(policies.decisions.is_empty());
        policies.defer_players.push(63);
        assert_eq!(policies.respond(62, &[2]), AcquisitionResponse::Unbound);
        assert_eq!(policies.respond(63, &[4, 1]), AcquisitionResponse::Defer);
        assert_eq!(policies.respond(63, &[5]), AcquisitionResponse::Defer);
        assert_eq!(
            policies.decisions,
            vec![
                DeferredAcquisition {
                    player: 63,
                    legal_indices: vec![4, 1]
                },
                DeferredAcquisition {
                    player: 63,
                    legal_indices: vec![5]
                },
            ]
        );
    }
    #[test]
    fn supplied_queue_skips_unneeded_legality_inputs() {
        assert_eq!(
            boundary(62, true, || Err::<Vec<i32>, _>("not reached")),
            Ok(TechnologyPolicy::Continue)
        );
    }
    #[test]
    fn empty_legal_list_continues_without_selecting_or_drawing() {
        assert_eq!(
            boundary(62, false, || Ok::<_, ()>(vec![])),
            Ok(TechnologyPolicy::Continue)
        );
    }
    #[test]
    fn external_choice_preserves_native_legal_order() {
        assert_eq!(
            boundary(62, false, || Ok::<_, ()>(vec![2, 0])),
            Ok(TechnologyPolicy::ChoicePending {
                player: 62,
                legal_indices: vec![2, 0]
            })
        );
        assert_eq!(
            boundary(62, false, || Err::<Vec<i32>, _>("missing graph")),
            Err("missing graph")
        );
    }
}
