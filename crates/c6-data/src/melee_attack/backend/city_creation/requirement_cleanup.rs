//! Destruction of the bounded current Golden/Dark Age and National Park predicates.
//! Their effects need not be applied: the derived predicate still owns a live
//! GameSignals subscription. Other predicate classes remain unsupported.
use super::{EffectRuntime, Error};
use c6_core::{
    loaded_state::LoadedState,
    unit_moved_signal::{ListenerRing, Subscription},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    GoldenAge,
    DarkAge,
    NationalPark,
}

/// One complete, ordered source ring. Listener labels identify source links,
/// including unrelated callbacks which must survive destruction.
pub struct InitialSignal {
    pub kind: Kind,
    pub listeners: Vec<u64>,
}
pub struct InitialRequirement {
    pub requirement: u64,
    pub kind: Kind,
    pub listener: u64,
}
struct Signal {
    kind: Kind,
    ring: ListenerRing<u64>,
}
struct Requirement {
    label: u64,
    kind: Kind,
    token: Option<Subscription>,
}
#[derive(Default)]
pub(super) struct State {
    installed: bool,
    signals: Vec<Signal>,
    requirements: Vec<Requirement>,
}
impl State {
    fn from_initial(
        signals: Vec<InitialSignal>,
        requirements: Vec<InitialRequirement>,
    ) -> Result<Self, Error> {
        let mut result = Self::default();
        for signal in signals {
            if result.signals.iter().any(|s| s.kind == signal.kind)
                || signal
                    .listeners
                    .iter()
                    .enumerate()
                    .any(|(i, v)| *v == 0 || signal.listeners[..i].contains(v))
            {
                return Err(Error::Missing("unique derived requirement source ring"));
            }
            result.signals.push(Signal {
                kind: signal.kind,
                ring: ListenerRing::from_ordered(&signal.listeners).map_err(Error::Ring)?,
            });
        }
        for row in requirements {
            if result
                .requirements
                .iter()
                .any(|r| r.label == row.requirement)
            {
                return Err(Error::Missing("unique derived requirement receiver"));
            }
            let ring = &result
                .signals
                .iter()
                .find(|s| s.kind == row.kind)
                .ok_or(Error::Missing("derived requirement source signal"))?
                .ring;
            let token = ring
                .subscription_for(|label| label == row.listener)
                .map_err(Error::Ring)?;
            if result
                .requirements
                .iter()
                .any(|r| r.kind == row.kind && r.token == Some(token))
            {
                return Err(Error::Missing("unique derived requirement owned handle"));
            }
            result.requirements.push(Requirement {
                label: row.requirement,
                kind: row.kind,
                token: Some(token),
            });
        }
        result.installed = true;
        Ok(result)
    }
    /// D1 23bd610/23bcbc0/2376660: Unsubscribe(+88), Handle dtor
    /// (already cleared), then SimpleRequirement D2. The caller deactivates
    /// only after this returns; no callback dispatch or RNG occurs here.
    pub(super) fn destroy(&mut self, label: u64) -> Result<bool, Error> {
        let Some(row) = self.requirements.iter_mut().find(|r| r.label == label) else {
            return Ok(false);
        };
        let token = row
            .token
            .ok_or(Error::Missing("destroyed derived requirement handle"))?;
        self.signals
            .iter_mut()
            .find(|s| s.kind == row.kind)
            .ok_or(Error::Missing("derived requirement source signal"))?
            .ring
            .unsubscribe(token)
            .map_err(Error::Ring)?;
        row.token = None;
        Ok(true)
    }
}
impl EffectRuntime {
    /// Whether this owner has imported the canonical age/park source rings.
    pub fn has_requirement_cleanup(&self) -> bool {
        self.city_creation
            .as_ref()
            .is_some_and(|creation| creation.requirement_cleanup.installed)
    }

    /// Concrete OnInitialize registers before the first Test. A newly allocated
    /// requirement supplies a fresh logical listener label in the same ring.
    pub fn subscribe_requirement_cleanup(
        &mut self,
        state: &LoadedState,
        requirement: u64,
        kind: Kind,
        listener: u64,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let cleanup = &mut self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("derived requirement lifecycle"))?
            .requirement_cleanup;
        if listener == 0 || cleanup.requirements.iter().any(|r| r.label == requirement) {
            return Err(Error::Missing("fresh derived requirement receiver"));
        }
        let signal = cleanup
            .signals
            .iter_mut()
            .find(|s| s.kind == kind)
            .ok_or(Error::Missing("derived requirement source signal"))?;
        if signal.ring.ordered_subscribers().any(|v| v == listener) {
            return Err(Error::Missing("fresh derived requirement listener"));
        }
        let token = signal.ring.subscribe(listener).map_err(Error::Ring)?;
        cleanup.requirements.push(Requirement {
            label: requirement,
            kind,
            token: Some(token),
        });
        Ok(())
    }
    /// Bind source-observed handles before advancing the same canonical owner.
    pub fn install_requirement_cleanup(
        &mut self,
        state: &LoadedState,
        requirements: &super::super::requirements::Runtime,
        signals: Vec<InitialSignal>,
        bindings: Vec<InitialRequirement>,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if !state.is_pristine() {
            return Err(Error::Missing("initial derived requirement handles"));
        }
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("derived requirement lifecycle"))?;
        if creation.requirement_cleanup.installed {
            return Err(Error::Missing(
                "derived requirement handles already installed",
            ));
        }
        for row in &bindings {
            if requirements
                .sets()
                .simple_requirement(row.requirement)
                .is_none()
            {
                return Err(Error::Missing("canonical derived SimpleRequirement"));
            }
        }
        creation.requirement_cleanup = State::from_initial(signals, bindings)?;
        Ok(())
    }
    pub fn current_requirement_cleanup_listeners(&self, kind: Kind) -> Result<Vec<u64>, Error> {
        let state = &self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("derived requirement lifecycle"))?
            .requirement_cleanup;
        Ok(state
            .signals
            .iter()
            .find(|s| s.kind == kind)
            .ok_or(Error::Missing("derived requirement source signal"))?
            .ring
            .ordered_subscribers()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destroys_only_its_actual_handle_and_preserves_order() {
        for kind in [Kind::GoldenAge, Kind::DarkAge, Kind::NationalPark] {
            let mut state = State::from_initial(
                vec![InitialSignal {
                    kind,
                    listeners: vec![8, 9, 10],
                }],
                vec![InitialRequirement {
                    requirement: 41,
                    kind,
                    listener: 9,
                }],
            )
            .unwrap();
            assert!(!state.destroy(42).unwrap());
            assert!(state.destroy(41).unwrap());
            assert_eq!(
                state.signals[0]
                    .ring
                    .ordered_subscribers()
                    .collect::<Vec<_>>(),
                vec![8, 10]
            );
            assert!(state.destroy(41).is_err());
        }
    }
    #[test]
    fn rejects_missing_and_shared_owned_handles() {
        assert!(State::from_initial(
            vec![],
            vec![InitialRequirement {
                requirement: 1,
                kind: Kind::GoldenAge,
                listener: 9
            }]
        )
        .is_err());
        assert!(State::from_initial(
            vec![InitialSignal {
                kind: Kind::DarkAge,
                listeners: vec![9]
            }],
            vec![
                InitialRequirement {
                    requirement: 1,
                    kind: Kind::DarkAge,
                    listener: 9
                },
                InitialRequirement {
                    requirement: 2,
                    kind: Kind::DarkAge,
                    listener: 9
                }
            ]
        )
        .is_err());
    }
}
