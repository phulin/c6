//! Gameplay projection of the stock powered-gossip presentation tail.
//! This models supported source-initialized localization classes, not native
//! observation/admission. A private importer must establish actual current
//! bindings/lifecycle before using these fields for an original-game comparison.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Logger {
    Unknown,
    Absent,
    Unmodeled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Formatters {
    Unknown,
    StockDefaults,
    Unmodeled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Localization {
    pub logger: Logger,
    pub formatters: Formatters,
}
impl Localization {
    /// Semantic result of stock localization initialization with a null logger.
    /// This constructor is not evidence about an existing native process.
    pub const fn initialized_stock_without_logger() -> Self {
        Self {
            logger: Logger::Absent,
            formatters: Formatters::StockDefaults,
        }
    }
    pub fn permits_render_omission(self) -> bool {
        self.logger == Logger::Absent && self.formatters == Formatters::StockDefaults
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Callback {
    pub logical_id: u64,
}
/// Current logical subscription lists, including the source's independent
/// prepublish/pre/linear/post callbacks. Empty is a state value, not a proof token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Callbacks {
    pub prepublish: Vec<Callback>,
    pub pre: Vec<Callback>,
    pub linear: Vec<Callback>,
    pub post: Vec<Callback>,
}
impl Callbacks {
    pub fn is_empty(&self) -> bool {
        self.prepublish.is_empty()
            && self.pre.is_empty()
            && self.linear.is_empty()
            && self.post.is_empty()
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatusState {
    pub enabled: Option<bool>,
    pub unserializing: Option<bool>,
    /// Actual App.Cache.GetLocalPlayer, distinct from Core/configuration local ID.
    pub cached_local_player: Option<i32>,
    pub event247: Option<Callbacks>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OmittedBecause {
    Disabled,
    Unserializing,
    Nonlocal,
    NoCallbacks,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnmodeledLocalization,
    UnknownStatusEffects,
    ReachableStatusCallback,
}
impl StatusState {
    /// Source Status.Send gates and Reader's recipient gate have no gameplay/RNG
    /// effects themselves. Any one proven inert branch is sufficient; unknown
    /// fields that cannot change this conclusion do not block gameplay.
    pub fn omit_status(&self, recipient: i32) -> Result<OmittedBecause, Error> {
        if self.enabled == Some(false) {
            return Ok(OmittedBecause::Disabled);
        }
        if self.unserializing == Some(true) {
            return Ok(OmittedBecause::Unserializing);
        }
        if self
            .cached_local_player
            .is_some_and(|local| local != recipient)
        {
            return Ok(OmittedBecause::Nonlocal);
        }
        if let Some(callbacks) = &self.event247 {
            if callbacks.is_empty() {
                return Ok(OmittedBecause::NoCallbacks);
            }
            return Err(Error::ReachableStatusCallback);
        }
        Err(Error::UnknownStatusEffects)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Callbacks {
        Callbacks {
            prepublish: vec![],
            pre: vec![],
            linear: vec![],
            post: vec![],
        }
    }
    #[test]
    fn stock_construction_is_explicit_and_custom_formatters_or_logger_refuse() {
        let mut l = Localization::initialized_stock_without_logger();
        assert!(l.permits_render_omission());
        l.logger = Logger::Unknown;
        assert!(!l.permits_render_omission());
        l.logger = Logger::Absent;
        l.formatters = Formatters::Unmodeled;
        assert!(!l.permits_render_omission());
    }
    #[test]
    fn nonlocal_proof_does_not_require_unrelated_flags_or_callbacks() {
        let s = StatusState {
            cached_local_player: Some(0),
            ..StatusState::default()
        };
        assert_eq!(s.omit_status(1), Ok(OmittedBecause::Nonlocal));
        assert_eq!(s.omit_status(0), Err(Error::UnknownStatusEffects));
    }
    #[test]
    fn actual_empty_lists_are_inert_without_assuming_missing_lists_empty() {
        let mut s = StatusState::default();
        assert_eq!(s.omit_status(0), Err(Error::UnknownStatusEffects));
        s.event247 = Some(empty());
        assert_eq!(s.omit_status(0), Ok(OmittedBecause::NoCallbacks));
        s.event247
            .as_mut()
            .unwrap()
            .post
            .push(Callback { logical_id: 7 });
        assert_eq!(s.omit_status(0), Err(Error::ReachableStatusCallback));
    }
    #[test]
    fn disabled_or_suspended_status_does_not_reach_present_callbacks() {
        let mut s = StatusState {
            enabled: Some(false),
            event247: Some(Callbacks {
                linear: vec![Callback { logical_id: 2 }],
                ..empty()
            }),
            ..StatusState::default()
        };
        assert_eq!(s.omit_status(0), Ok(OmittedBecause::Disabled));
        s.enabled = None;
        s.unserializing = Some(true);
        assert_eq!(s.omit_status(0), Ok(OmittedBecause::Unserializing));
    }
}
