//! PoweredCity ReportGossip/SendGossip distribution. Current manager history and
//! alive-major order are actual inputs, not reconstructed from sorted players.
//! Presentation/status is a leaf effect: it remains synchronous and may refuse
//! unknown callbacks/RNG after earlier gossip/counter writes.
use super::stock_signals::{
    increment_powered_gossip_reports, CounterError, Occurrence, POWERED_CITY_GOSSIP,
};
use crate::{game_diplomatic_relations::GameDiplomaticRelations, player_diplomacy::GossipReports};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnVisibility {
    pub turn: u16,
    pub viewing_player: i8,
    pub viewed_player: i8,
    pub visibility: i8,
    pub source: i8,
    pub source_name_index: i32,
}
impl TurnVisibility {
    /// Native constructor, not proof that an existing loaded fallback was reset.
    pub const fn constructed() -> Self {
        Self {
            turn: 0,
            viewing_player: -1,
            viewed_player: -1,
            visibility: -1,
            source: -1,
            source_name_index: -1,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Item<T> {
    PoweredCity(Occurrence),
    Other(T),
}
pub struct Manager<T> {
    pub history: Vec<TurnVisibility>,
    pub fallback: TurnVisibility,
    /// Both lists preserve all previous occurrence types and shared identities.
    pub pending: Vec<Item<T>>,
    pub filtered: Vec<Item<T>>,
}
impl<T> Manager<T> {
    pub fn most_recent(&self, viewing: i32, viewed: i32) -> TurnVisibility {
        if viewing != viewed {
            if let Some(value) = self.history.iter().rev().find(|value| {
                i32::from(value.viewing_player) == viewing
                    && i32::from(value.viewed_player) == viewed
            }) {
                return *value;
            }
        }
        self.fallback
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Definition {
    pub type_hash: u32,
    pub visibility_level: i32,
    pub filter: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Status {
    pub status_hash: u32,
    pub id: u32,
    pub recipient: i32,
    pub gossip_type_hash: u32,
    pub visibility: TurnVisibility,
}
#[derive(Debug)]
pub enum Error<E> {
    Input(E),
    Relations(crate::game_diplomatic_relations::Error),
    Counter(CounterError),
    Allocation(std::collections::TryReserveError),
    InvalidatedMajorCursor,
    MissingDefinition,
}

pub trait Context {
    type Error;
    type OtherOccurrence;
    type Formatted;
    fn manager(&mut self) -> Result<&mut Manager<Self::OtherOccurrence>, Self::Error>;
    /// Actual native GetAliveMajors vector order. Each ID belongs to the retained
    /// current Player object; this is not numeric owner sorting.
    fn alive_major_ids(&mut self) -> Result<&[i32], Self::Error>;
    fn definition(&mut self, key: u32) -> Result<Option<Definition>, Self::Error>;
    fn player_is_valid(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn relations(&mut self) -> Result<&GameDiplomaticRelations, Self::Error>;
    fn gossip_reports(&mut self, recipient: i32) -> Result<&mut GossipReports, Self::Error>;
    fn current_turn(&mut self) -> Result<i32, Self::Error>;
    /// Execute actual FormatGossip (including its source-selected localization
    /// and concrete PoweredCity description) before the later definition lookup.
    fn format_gossip(
        &mut self,
        occurrence: Occurrence,
        visibility: TurnVisibility,
    ) -> Result<Self::Formatted, Self::Error>;
    /// Actual Status.Send, including gameplay/RNG-capable subscribers. No default
    /// no-op is supplied; earlier counters/appends remain after an error.
    fn send_status(
        &mut self,
        formatted: Self::Formatted,
        status: Status,
    ) -> Result<(), Self::Error>;
}
fn has_met<C: Context>(c: &mut C, recipient: i32, viewed: i32) -> Result<bool, Error<C::Error>> {
    if !c.player_is_valid(viewed).map_err(Error::Input)? || recipient == viewed {
        return Ok(false);
    }
    c.relations()
        .map_err(Error::Input)?
        .has_met(recipient, viewed)
        .map_err(Error::Relations)
}
/// Exact powered occurrence has GetOtherPlayer() == -1. SendGossip still calls
/// HasMet(-1); the validity gate is preserved, rather than assuming all sent
/// gossip concerns two players. Native valid-player semantics must reject -1.
pub fn send_powered<C: Context>(
    c: &mut C,
    recipient: i32,
    occurrence: Occurrence,
) -> Result<(), Error<C::Error>> {
    let viewed = occurrence.borrow().player;
    let visibility = c
        .manager()
        .map_err(Error::Input)?
        .most_recent(recipient, viewed);
    if has_met(c, recipient, viewed)? {
        increment_powered_gossip_reports(
            c.gossip_reports(recipient).map_err(Error::Input)?,
            viewed,
        )
        .map_err(Error::Counter)?;
    }
    if has_met(c, recipient, -1)? {
        increment_powered_gossip_reports(c.gossip_reports(recipient).map_err(Error::Input)?, -1)
            .map_err(Error::Counter)?;
    }
    let formatted = c
        .format_gossip(occurrence, visibility)
        .map_err(Error::Input)?;
    // Source re-finds the definition only after formatting completes.
    let definition = c
        .definition(POWERED_CITY_GOSSIP)
        .map_err(Error::Input)?
        .ok_or(Error::MissingDefinition)?;
    c.send_status(
        formatted,
        Status {
            status_hash: 0xdf126abf,
            id: 0,
            recipient,
            gossip_type_hash: definition.type_hash,
            visibility,
        },
    )
    .map_err(Error::Input)
}
/// Full stock PoweredCity ReportGossip selection and distribution. It preserves
/// the cached alive-major End, current history re-reads and filter early return.
pub fn report_powered<C: Context>(
    c: &mut C,
    occurrence: Occurrence,
) -> Result<(), Error<C::Error>> {
    let count = c.alive_major_ids().map_err(Error::Input)?.len();
    for index in 0..count {
        let recipient = c
            .alive_major_ids()
            .map_err(Error::Input)?
            .get(index)
            .copied()
            .ok_or(Error::InvalidatedMajorCursor)?;
        let viewed = occurrence.borrow().player;
        let visibility = c
            .manager()
            .map_err(Error::Input)?
            .most_recent(recipient, viewed);
        let Some(definition) = c.definition(POWERED_CITY_GOSSIP).map_err(Error::Input)? else {
            continue;
        };
        if i32::from(visibility.visibility) < definition.visibility_level {
            continue;
        }
        if occurrence.borrow().player == recipient {
            continue;
        }
        // PoweredCity's constant other player -1 is still compared by Report.
        if recipient == -1 {
            continue;
        }
        if definition.filter {
            let manager = c.manager().map_err(Error::Input)?;
            manager.filtered.try_reserve(1).map_err(Error::Allocation)?;
            manager.filtered.push(Item::PoweredCity(occurrence));
            return Ok(());
        }
        send_powered(c, recipient, std::rc::Rc::clone(&occurrence))?;
    }
    Ok(())
}
/// AddGossipItem join for an already constructed PoweredCity occurrence. Its
/// handler obtains the actual City owner/ID before this source-ordered sequence.
pub fn add_powered<C: Context>(c: &mut C, occurrence: Occurrence) -> Result<(), Error<C::Error>> {
    if c.manager().map_err(Error::Input)?.history.is_empty() {
        return Ok(());
    }
    let turn = c.current_turn().map_err(Error::Input)?;
    occurrence.borrow_mut().turn = turn;
    let manager = c.manager().map_err(Error::Input)?;
    manager.pending.try_reserve(1).map_err(Error::Allocation)?;
    manager
        .pending
        .push(Item::PoweredCity(std::rc::Rc::clone(&occurrence)));
    report_powered(c, occurrence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{city_power_activation::stock_signals::PoweredCityOccurrence, unit::Limits};
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
    fn relations() -> GameDiplomaticRelations {
        let mut bytes = Vec::new();
        let put = |bytes: &mut Vec<u8>, v: i32| bytes.extend(v.to_le_bytes());
        put(&mut bytes, 16);
        put(&mut bytes, 3);
        for index in 0..4 {
            let count = if index == 0 { 3 } else { 0 };
            put(&mut bytes, count);
            for _ in 0..count {
                bytes.push(1);
                put(&mut bytes, 0);
            }
        }
        for index in 0..4 {
            let count = if index == 1 { 3 } else { 0 };
            put(&mut bytes, count);
            for _ in 0..count {
                put(&mut bytes, -1);
                put(&mut bytes, 0);
            }
        }
        bytes.push(0);
        put(&mut bytes, 3);
        for _ in 0..3 {
            put(&mut bytes, -1);
            put(&mut bytes, 0);
        }
        GameDiplomaticRelations::decode_prefix(&bytes, Limits::default())
            .unwrap()
            .state
    }
    fn visibility(viewing: i8, level: i8) -> TurnVisibility {
        TurnVisibility {
            viewing_player: viewing,
            viewed_player: 0,
            visibility: level,
            ..TurnVisibility::constructed()
        }
    }
    struct Fixture {
        manager: Manager<&'static str>,
        relations: GameDiplomaticRelations,
        alive: Vec<i32>,
        reports: BTreeMap<i32, GossipReports>,
        definition: Option<Definition>,
        statuses: Vec<Status>,
        log: Vec<&'static str>,
        change_history: bool,
        fail_format: bool,
        fail_status: bool,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                manager: Manager {
                    history: vec![visibility(2, 3), visibility(1, 1)],
                    fallback: TurnVisibility::constructed(),
                    pending: vec![Item::Other("prior")],
                    filtered: vec![],
                },
                relations: relations(),
                alive: vec![2, 1, 0],
                reports: [(1, vec![(0, vec![].into())]), (2, vec![(0, vec![].into())])].into(),
                definition: Some(Definition {
                    type_hash: POWERED_CITY_GOSSIP,
                    visibility_level: 2,
                    filter: false,
                }),
                statuses: vec![],
                log: vec![],
                change_history: false,
                fail_format: false,
                fail_status: false,
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type OtherOccurrence = &'static str;
        type Formatted = String;
        fn manager(&mut self) -> Result<&mut Manager<Self::OtherOccurrence>, Self::Error> {
            Ok(&mut self.manager)
        }
        fn alive_major_ids(&mut self) -> Result<&[i32], Self::Error> {
            Ok(&self.alive)
        }
        fn definition(&mut self, _: u32) -> Result<Option<Definition>, Self::Error> {
            self.log.push("definition");
            Ok(self.definition)
        }
        fn player_is_valid(&mut self, p: i32) -> Result<bool, Self::Error> {
            Ok((0..3).contains(&p))
        }
        fn relations(&mut self) -> Result<&GameDiplomaticRelations, Self::Error> {
            Ok(&self.relations)
        }
        fn gossip_reports(&mut self, p: i32) -> Result<&mut GossipReports, Self::Error> {
            self.log.push("counter");
            self.reports.get_mut(&p).ok_or("player")
        }
        fn current_turn(&mut self) -> Result<i32, Self::Error> {
            self.log.push("turn");
            Ok(592)
        }
        fn format_gossip(
            &mut self,
            o: Occurrence,
            v: TurnVisibility,
        ) -> Result<String, Self::Error> {
            self.log.push("format");
            assert_eq!(o.borrow().city_id, 17);
            if self.fail_format {
                Err("format")
            } else {
                Ok(format!("visibility:{}", v.visibility))
            }
        }
        fn send_status(&mut self, _: String, s: Status) -> Result<(), Self::Error> {
            self.log.push("status");
            self.statuses.push(s);
            if self.change_history {
                self.manager.history.push(visibility(1, 4));
                self.alive.push(2);
            }
            if self.fail_status {
                Err("status")
            } else {
                Ok(())
            }
        }
    }
    fn occurrence() -> Occurrence {
        Rc::new(RefCell::new(PoweredCityOccurrence {
            turn: -1,
            player: 0,
            city_id: 17,
        }))
    }
    #[test]
    fn visibility_uses_reverse_insertion_order_and_self_fallback() {
        let mut f = Fixture::new();
        f.manager.history.push(TurnVisibility {
            turn: 999,
            ..visibility(2, 1)
        });
        f.manager.history.push(TurnVisibility {
            turn: 1,
            ..visibility(2, 4)
        });
        f.manager.history.push(visibility(0, 7));
        assert_eq!(f.manager.most_recent(2, 0).turn, 1);
        assert_eq!(f.manager.most_recent(0, 0), f.manager.fallback);
        assert_eq!(f.manager.most_recent(1, 2), f.manager.fallback);
    }
    #[test]
    fn native_order_threshold_history_reread_and_cached_major_end() {
        let mut f = Fixture::new();
        f.change_history = true;
        let o = occurrence();
        add_powered(&mut f, Rc::clone(&o)).unwrap();
        assert_eq!(o.borrow().turn, 592);
        assert!(matches!(f.manager.pending[0], Item::Other("prior")));
        assert_eq!(f.manager.pending.len(), 2);
        assert_eq!(
            f.statuses.iter().map(|s| s.recipient).collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(
            f.reports[&1][0].1.entries,
            [(POWERED_CITY_GOSSIP as i32, 1)]
        );
        assert_eq!(f.statuses[0].status_hash, 0xdf126abf);
        assert_eq!(f.statuses[0].id, 0);
        assert_eq!(f.statuses[1].visibility.visibility, 4);
        let start = f.log.iter().position(|s| *s == "counter").unwrap();
        assert_eq!(
            &f.log[start..start + 4],
            ["counter", "format", "definition", "status"]
        );
    }
    #[test]
    fn filter_appends_same_occurrence_once_and_returns_before_delivery() {
        let mut f = Fixture::new();
        f.definition.as_mut().unwrap().filter = true;
        f.manager.filtered.push(Item::Other("older filtered"));
        let o = occurrence();
        add_powered(&mut f, Rc::clone(&o)).unwrap();
        assert_eq!(f.manager.filtered.len(), 2);
        let Item::PoweredCity(actual) = &f.manager.filtered[1] else {
            panic!()
        };
        assert!(Rc::ptr_eq(actual, &o));
        assert!(f.statuses.is_empty());
        assert!(f.reports.values().all(|v| v[0].1.entries.is_empty()));
    }
    #[test]
    fn missing_definition_and_low_visibility_do_not_deliver() {
        let mut f = Fixture::new();
        f.definition = None;
        report_powered(&mut f, occurrence()).unwrap();
        assert!(f.statuses.is_empty());
        f.definition = Some(Definition {
            type_hash: POWERED_CITY_GOSSIP,
            visibility_level: 9,
            filter: false,
        });
        report_powered(&mut f, occurrence()).unwrap();
        assert!(f.statuses.is_empty());
    }
    #[test]
    fn failures_preserve_pending_and_diplomacy_counter_prefix() {
        let mut f = Fixture::new();
        f.fail_format = true;
        assert!(matches!(
            add_powered(&mut f, occurrence()),
            Err(Error::Input("format"))
        ));
        assert_eq!(f.manager.pending.len(), 2);
        assert_eq!(f.reports[&2][0].1.entries[0].1, 1);
        assert!(f.statuses.is_empty());
        f.fail_format = false;
        f.fail_status = true;
        assert!(matches!(
            send_powered(&mut f, 2, occurrence()),
            Err(Error::Input("status"))
        ));
        assert_eq!(f.reports[&2][0].1.entries[0].1, 2);
        assert_eq!(f.statuses.len(), 1);
    }
    #[test]
    fn empty_history_returns_before_turn_read_or_pending_append() {
        let mut f = Fixture::new();
        f.manager.history.clear();
        let o = occurrence();
        add_powered(&mut f, Rc::clone(&o)).unwrap();
        assert_eq!(o.borrow().turn, -1);
        assert_eq!(f.manager.pending.len(), 1);
        assert!(f.log.is_empty());
    }
    #[test]
    fn missing_has_met_storage_is_not_unmet_and_self_skips_it() {
        let mut f = Fixture::new();
        f.relations = GameDiplomaticRelations::from_war_values(3, vec![-1; 3]).unwrap();
        assert!(matches!(
            send_powered(&mut f, 2, occurrence()),
            Err(Error::Relations(
                crate::game_diplomatic_relations::Error::MissingHasMet
            ))
        ));
        assert!(f.statuses.is_empty());
        send_powered(&mut f, 0, occurrence()).unwrap();
        assert_eq!(f.statuses.len(), 1);
    }
}
