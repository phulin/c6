//! Ordered Linux Player.Manager.Meet and Player.Diplomacy.SetHasMet effects.
//!
//! Required callbacks are gameplay providers, not native answers or AI policy.
//! This component is not a complete loaded-world adapter or RequestSession
//! backend. In particular FirstMeeting may perform effects before a genuine
//! external decision. Unsupported support must fail preflight before writes;
//! an admitted callback failure retains its completed native-order prefix.
use crate::unit::{ComponentId, PlotCoord, Unit};
use crate::unit_placement_transition::{
    self as movement, PlacementReport, PlacementStateContext, PlacementStateError,
};
use crate::{game_diplomatic_relations as relations, player_diplomacy::GossipReports};

pub const MAJOR_LEVEL: u32 = 0x2537_18b0;
pub const CITY_STATE_LEVEL: u32 = 0xc257_7075;
pub const BARBARIAN_MET: u32 = 0x09a2_85e8;
pub const PLAYERS_MET: u32 = 0x133d_411c;
pub const CITY_STATE_MET: u32 = 0x08b3_bf96;
pub const MAJORS_MET: u32 = 0x892f_171c;
pub const VISIBILITY_CHANGED: u32 = 0x3b09_d4be;
pub const FIRST_CONTACT_NOTIFICATION: u32 = 0x8953_b839;
pub const FIRST_CITY_MEETING: u32 = 0x411e_77e0;
pub const SECOND_CITY_MEETING: u32 = 0x6c01_793e;
pub const NEUTRAL_MEETING: u32 = 0xd69a_0de2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibilitySource {
    pub index: i32,
    /// Actual presence of Tech/Trait/GreatPerson/XP1 TradePost references.
    pub dynamic: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FirstMeetingRequest {
    pub statement_type: u32,
    pub from: i32,
    pub to: i32,
    pub hospitality_city: PlotCoord,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub event: u32,
    pub source: ComponentId,
    pub first: i32,
    pub second: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    EffectsUnavailable(E),
    Relations(relations::Error),
    Movement(PlacementStateError<E>),
    MissingVisibilityCell(i32),
    InvalidGossipOrder,
    ResourceLimit,
    MissingHospitalityCity,
}
pub trait Context {
    type Error;
    type Player: Copy;
    type Unit: Copy;
    type Plot: Copy;
    type City: Copy;
    /// GetPlayer, not GetValidPlayer. An unavailable/null required object is an
    /// error, never a fabricated dead player.
    fn player(&mut self, owner: i32) -> Result<Self::Player, Self::Error>;
    fn player_valid(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn id(&mut self, player: Self::Player) -> Result<i32, Self::Error>;
    fn alive(&mut self, player: Self::Player) -> Result<bool, Self::Error>;
    fn major(&mut self, player: Self::Player) -> Result<bool, Self::Error>;
    fn barbarian(&mut self, player: Self::Player) -> Result<bool, Self::Error>;
    fn human(&mut self, player: Self::Player) -> Result<bool, Self::Error>;
    fn level(&mut self, player: Self::Player) -> Result<u32, Self::Error>;
    fn any_multiplayer(&mut self) -> Result<bool, Self::Error>;
    fn generating(&mut self) -> Result<bool, Self::Error>;
    fn relations(&self) -> Result<&relations::GameDiplomaticRelations, Self::Error>;
    fn relations_mut(&mut self) -> Result<&mut relations::GameDiplomaticRelations, Self::Error>;
    fn current_turn(&mut self) -> Result<i32, Self::Error>;
    /// Pure support admission for all possible reached/reentrant effects,
    /// including logger, visibility reports, movement and session processing.
    fn preflight_meet(
        &self,
        first: i32,
        second: i32,
        plot: Option<Self::Plot>,
        source: ComponentId,
    ) -> Result<(), Self::Error>;
    fn gossip_mut(&mut self, player: Self::Player) -> Result<&mut GossipReports, Self::Error>;
    /// Current enum-indexed gameplay state, not wire hashes/default arrays.
    fn visibility_mut(&mut self, player: Self::Player) -> Result<&mut [i32], Self::Error>;
    fn visibility_source_mut(&mut self, player: Self::Player) -> Result<&mut [i32], Self::Error>;
    fn visibility_sources(&self) -> &[VisibilitySource];
    /// Recheck against the canonical current diplomatic owner. Wire-backed
    /// adapters may override this to translate source enums through their rules
    /// registry while preserving the synchronous report before the source write.
    fn recheck_contact_visibility(
        &mut self,
        player: Self::Player,
        other: i32,
    ) -> Result<(), Error<Self::Error>>
    where
        Self: Sized,
    {
        recheck_visibility(self, player, other)
    }
    fn visibility_source_active(
        &mut self,
        player: Self::Player,
        other: i32,
        source: i32,
    ) -> Result<bool, Self::Error>;
    /// Full GetDynamicVisibility algorithm in native order. Only its first
    /// returned amount contributes here; producing the list may query rules.
    fn dynamic_visibility(
        &mut self,
        player: Self::Player,
        source: i32,
    ) -> Result<Vec<i32>, Self::Error>;
    fn visibility_changed(&mut self, event: u32, owner: i32, other: i32)
        -> Result<(), Self::Error>;
    fn log_has_met(&mut self, owner: i32, other: i32) -> Result<(), Self::Error>;
    fn find_unit(&mut self, source: ComponentId) -> Result<Option<Self::Unit>, Self::Error>;
    fn unit(&self, unit: Self::Unit) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self, unit: Self::Unit) -> Result<&mut Unit, Self::Error>;
    fn preflight_movement_report(&self, unit: Self::Unit, event: u32) -> Result<(), Self::Error>;
    fn movement_report(&mut self, report: PlacementReport) -> Result<(), Self::Error>;
    /// Includes reached text/resource/report effects; omitting presentation
    /// requires its separate stock-consumer proof. Called first then second.
    fn notify_contact(
        &mut self,
        recipient: i32,
        other: Self::Player,
        plot: Self::Plot,
    ) -> Result<(), Self::Error>;
    fn report(&mut self, report: Report) -> Result<(), Self::Error>;
    /// Actual Cities.FindClosest(plot,false,NO_PLAYER), preserving native order.
    fn closest_city(
        &mut self,
        player: Self::Player,
        plot: Self::Plot,
    ) -> Result<Option<Self::City>, Self::Error>;
    fn city_distance(&mut self, city: Self::City, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn city_location(&mut self, city: Self::City) -> Result<PlotCoord, Self::Error>;
    fn hospitality_distance(&mut self) -> Result<i32, Self::Error>;
    /// Create the real typed FirstMeeting payload, set hospitality location,
    /// then execute RequestSession. Preserve pre-decision gameplay/RNG effects;
    /// do not satisfy a player choice with native AI policy.
    fn request_first_meeting(&mut self, request: FirstMeetingRequest) -> Result<(), Self::Error>;
    /// Game.Signals+0xc78, retaining the original cached Player handles.
    fn signal_meet(&mut self, first: Self::Player, second: Self::Player)
        -> Result<(), Self::Error>;
}

pub fn recheck_visibility<C: Context>(
    c: &mut C,
    player: C::Player,
    other: i32,
) -> Result<(), Error<C::Error>> {
    let other_player = c.player(other).map_err(Error::Context)?;
    if c.level(other_player).map_err(Error::Context)? != MAJOR_LEVEL {
        return Ok(());
    }
    let mut total = 0i32;
    let mut last = -1;
    for ordinal in 0..c.visibility_sources().len() {
        let source = c.visibility_sources()[ordinal];
        if !c
            .visibility_source_active(player, other, source.index)
            .map_err(Error::Context)?
        {
            continue;
        } else if source.dynamic {
            let values = c
                .dynamic_visibility(player, source.index)
                .map_err(Error::Context)?;
            if let Some(value) = values.first() {
                total = total.wrapping_add(*value);
                last = source.index;
            }
        } else {
            total = total.wrapping_add(1);
            last = source.index;
        }
    }
    let index = usize::try_from(other).map_err(|_| Error::MissingVisibilityCell(other))?;
    let level = total.min(4); // signed clamp above only; negatives survive.
    let cell = c
        .visibility_mut(player)
        .map_err(Error::Context)?
        .get_mut(index)
        .ok_or(Error::MissingVisibilityCell(other))?;
    if *cell != level {
        *cell = level;
        let owner = c.id(player).map_err(Error::Context)?;
        c.visibility_changed(VISIBILITY_CHANGED, owner, other)
            .map_err(Error::Context)?;
    }
    // This write follows the synchronous report and happens even for equality.
    *c.visibility_source_mut(player)
        .map_err(Error::Context)?
        .get_mut(index)
        .ok_or(Error::MissingVisibilityCell(other))? = last;
    Ok(())
}
fn set_has_met<C: Context>(
    c: &mut C,
    player: C::Player,
    other: i32,
) -> Result<(), Error<C::Error>> {
    if c.major(player).map_err(Error::Context)? {
        let other_player = c.player(other).map_err(Error::Context)?;
        if c.major(other_player).map_err(Error::Context)? {
            let gossip = c.gossip_mut(player).map_err(Error::Context)?;
            if gossip.windows(2).any(|w| w[0].0 >= w[1].0) {
                return Err(Error::InvalidGossipOrder);
            }
            if let Err(index) = gossip.binary_search_by_key(&other, |v| v.0) {
                gossip.try_reserve(1).map_err(|_| Error::ResourceLimit)?;
                gossip.insert(index, (other, Vec::new().into()));
            }
        }
    }
    c.recheck_contact_visibility(player, other)?;
    let owner = c.id(player).map_err(Error::Context)?;
    let turn = c.current_turn().map_err(Error::Context)?;
    c.relations_mut()
        .map_err(Error::Context)?
        .set_has_met(owner, other, turn)
        .map_err(Error::Relations)?;
    c.log_has_met(owner, other).map_err(Error::Context)
}
struct Movement<'a, C: Context> {
    context: &'a mut C,
    unit: C::Unit,
}
impl<C: Context> PlacementStateContext for Movement<'_, C> {
    type Error = C::Error;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.context.unit(self.unit)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.context.unit_mut(self.unit)
    }
    fn preflight_placement_report(&self, event: u32) -> Result<(), Self::Error> {
        self.context.preflight_movement_report(self.unit, event)
    }
    fn send_placement_report(&mut self, report: PlacementReport) -> Result<(), Self::Error> {
        self.context.movement_report(report)
    }
}
fn complete_source<C: Context>(c: &mut C, source: ComponentId) -> Result<(), Error<C::Error>> {
    if source.owner == -1 || source.id == u32::MAX || source.component_type != 1 {
        return Ok(());
    }
    if let Some(unit) = c.find_unit(source).map_err(Error::Context)? {
        movement::complete_movement(&mut Movement { context: c, unit }).map_err(Error::Movement)?;
    }
    Ok(())
}
fn send<C: Context>(
    c: &mut C,
    event: u32,
    source: ComponentId,
    first: i32,
    second: i32,
) -> Result<(), Error<C::Error>> {
    c.report(Report {
        event,
        source,
        first,
        second,
    })
    .map_err(Error::Context)
}

pub fn meet<C: Context>(
    c: &mut C,
    first: i32,
    second: i32,
    plot: Option<C::Plot>,
    source: ComponentId,
) -> Result<bool, Error<C::Error>> {
    if first == -1 || second == -1 {
        return Ok(false);
    }
    let a = c.player(first).map_err(Error::Context)?;
    let b = c.player(second).map_err(Error::Context)?;
    if first == second
        || !c.alive(a).map_err(Error::Context)?
        || !c.alive(b).map_err(Error::Context)?
    {
        return Ok(false);
    }
    if c.player_valid(second).map_err(Error::Context)? {
        let owner = c.id(a).map_err(Error::Context)?;
        if owner != second
            && c.relations()
                .map_err(Error::Context)?
                .has_met(owner, second)
                .map_err(Error::Relations)?
        {
            return Ok(false);
        }
    }
    c.preflight_meet(first, second, plot, source)
        .map_err(Error::EffectsUnavailable)?;
    set_has_met(c, a, second)?;
    set_has_met(c, b, first)?;
    complete_source(c, source)?;
    if c.barbarian(a).map_err(Error::Context)? || c.barbarian(b).map_err(Error::Context)? {
        send(c, BARBARIAN_MET, source, first, second)?;
        return Ok(true);
    }
    let suppress = if c.any_multiplayer().map_err(Error::Context)?
        && c.major(a).map_err(Error::Context)?
        && c.major(b).map_err(Error::Context)?
    {
        let a_human = c.human(a).map_err(Error::Context)?;
        let b_human = c.human(b).map_err(Error::Context)?;
        a_human ^ b_human
    } else {
        false
    };
    if let Some(plot) = plot.filter(|_| !suppress) {
        c.notify_contact(first, b, plot).map_err(Error::Context)?;
        c.notify_contact(second, a, plot).map_err(Error::Context)?;
    }
    send(c, PLAYERS_MET, source, first, second)?;
    let city_state_pair = if c.major(a).map_err(Error::Context)?
        && c.level(b).map_err(Error::Context)? == CITY_STATE_LEVEL
    {
        true
    } else {
        c.level(a).map_err(Error::Context)? == CITY_STATE_LEVEL
            && c.major(b).map_err(Error::Context)?
    };
    if city_state_pair {
        send(c, CITY_STATE_MET, source, first, second)?;
    }
    if c.major(a).map_err(Error::Context)? {
        let second_major = c.major(b).map_err(Error::Context)?;
        if let Some(plot) = plot.filter(|_| second_major) {
            send(c, MAJORS_MET, source, first, second)?;
            if !c.generating().map_err(Error::Context)? {
                let first_human = c.human(a).map_err(Error::Context)?;
                let second_human = c.human(b).map_err(Error::Context)?;
                if first_human ^ second_human {
                    let first_city = c.closest_city(a, plot).map_err(Error::Context)?;
                    let first_distance = match first_city {
                        Some(city) => c.city_distance(city, plot).map_err(Error::Context)?,
                        None => i32::MAX,
                    };
                    let second_city = c.closest_city(b, plot).map_err(Error::Context)?;
                    let second_distance = match second_city {
                        Some(city) => c.city_distance(city, plot).map_err(Error::Context)?,
                        None => i32::MAX,
                    };
                    let threshold = c.hospitality_distance().map_err(Error::Context)?;
                    let (kind, city) =
                        if first_distance <= threshold && first_distance < second_distance {
                            (FIRST_CITY_MEETING, first_city)
                        } else if second_distance <= threshold || first_distance <= threshold {
                            (SECOND_CITY_MEETING, second_city)
                        } else {
                            (NEUTRAL_MEETING, None)
                        };
                    let location = if kind == NEUTRAL_MEETING {
                        PlotCoord { x: -9999, y: -9999 }
                    } else {
                        c.city_location(city.ok_or(Error::MissingHospitalityCity)?)
                            .map_err(Error::Context)?
                    };
                    // Source rereads first.IsHuman independently twice here.
                    let from = if c.human(a).map_err(Error::Context)? {
                        second
                    } else {
                        first
                    };
                    let to = if c.human(a).map_err(Error::Context)? {
                        first
                    } else {
                        second
                    };
                    c.request_first_meeting(FirstMeetingRequest {
                        statement_type: kind,
                        from,
                        to,
                        hospitality_city: location,
                    })
                    .map_err(Error::Context)?;
                }
            }
        }
    }
    c.signal_meet(a, b).map_err(Error::Context)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone)]
    struct Player {
        major: bool,
        barbarian: bool,
        human: bool,
        level: u32,
        alive: bool,
        gossip: GossipReports,
        visibility: [i32; 2],
        sources: [i32; 2],
    }
    struct Fixture {
        players: [Player; 2],
        relations: relations::GameDiplomaticRelations,
        definitions: Vec<VisibilitySource>,
        trace: Vec<String>,
        multiplayer: bool,
        generating: bool,
        distances: [Option<i32>; 2],
        threshold: i32,
        request: Option<FirstMeetingRequest>,
        unavailable: bool,
        fail_visibility: bool,
        turn: i32,
    }
    fn relation_state() -> relations::GameDiplomaticRelations {
        let mut bytes = Vec::new();
        let put = |b: &mut Vec<u8>, n: i32| b.extend(n.to_le_bytes());
        put(&mut bytes, 16);
        put(&mut bytes, 2);
        for _ in 0..4 {
            put(&mut bytes, 1);
            bytes.push(0);
            put(&mut bytes, -1);
        }
        for _ in 0..4 {
            put(&mut bytes, 1);
            put(&mut bytes, -1);
            put(&mut bytes, -1);
        }
        bytes.push(0);
        put(&mut bytes, 1);
        put(&mut bytes, -1);
        put(&mut bytes, -1);
        relations::GameDiplomaticRelations::decode_prefix(&bytes, crate::unit::Limits::default())
            .unwrap()
            .state
    }
    fn fixture() -> Fixture {
        let p = Player {
            major: true,
            barbarian: false,
            human: false,
            level: MAJOR_LEVEL,
            alive: true,
            gossip: vec![],
            visibility: [0; 2],
            sources: [-1; 2],
        };
        let mut players = [p.clone(), p];
        players[0].human = true;
        Fixture {
            players,
            relations: relation_state(),
            definitions: vec![],
            trace: vec![],
            multiplayer: false,
            generating: false,
            distances: [Some(3), Some(3)],
            threshold: 4,
            request: None,
            unavailable: false,
            fail_visibility: false,
            turn: 10,
        }
    }
    const SOURCE: ComponentId = ComponentId {
        owner: 0,
        component_type: 1,
        id: 42,
    };
    impl Context for Fixture {
        type Error = &'static str;
        type Player = usize;
        type Unit = ();
        type Plot = ();
        type City = usize;
        fn player(&mut self, p: i32) -> Result<usize, Self::Error> {
            usize::try_from(p).ok().filter(|p| *p < 2).ok_or("player")
        }
        fn player_valid(&mut self, p: i32) -> Result<bool, Self::Error> {
            Ok((0..2).contains(&p))
        }
        fn id(&mut self, p: usize) -> Result<i32, Self::Error> {
            Ok(p as i32)
        }
        fn alive(&mut self, p: usize) -> Result<bool, Self::Error> {
            Ok(self.players[p].alive)
        }
        fn major(&mut self, p: usize) -> Result<bool, Self::Error> {
            Ok(self.players[p].major)
        }
        fn barbarian(&mut self, p: usize) -> Result<bool, Self::Error> {
            Ok(self.players[p].barbarian)
        }
        fn human(&mut self, p: usize) -> Result<bool, Self::Error> {
            Ok(self.players[p].human)
        }
        fn level(&mut self, p: usize) -> Result<u32, Self::Error> {
            Ok(self.players[p].level)
        }
        fn any_multiplayer(&mut self) -> Result<bool, Self::Error> {
            self.trace.push("mode".into());
            Ok(self.multiplayer)
        }
        fn generating(&mut self) -> Result<bool, Self::Error> {
            Ok(self.generating)
        }
        fn relations(&self) -> Result<&relations::GameDiplomaticRelations, Self::Error> {
            Ok(&self.relations)
        }
        fn relations_mut(
            &mut self,
        ) -> Result<&mut relations::GameDiplomaticRelations, Self::Error> {
            Ok(&mut self.relations)
        }
        fn current_turn(&mut self) -> Result<i32, Self::Error> {
            Ok(self.turn)
        }
        fn preflight_meet(
            &self,
            _: i32,
            _: i32,
            _: Option<()>,
            _: ComponentId,
        ) -> Result<(), Self::Error> {
            if self.unavailable {
                Err("unsupported")
            } else {
                Ok(())
            }
        }
        fn gossip_mut(&mut self, p: usize) -> Result<&mut GossipReports, Self::Error> {
            Ok(&mut self.players[p].gossip)
        }
        fn visibility_mut(&mut self, p: usize) -> Result<&mut [i32], Self::Error> {
            Ok(&mut self.players[p].visibility)
        }
        fn visibility_source_mut(&mut self, p: usize) -> Result<&mut [i32], Self::Error> {
            Ok(&mut self.players[p].sources)
        }
        fn visibility_sources(&self) -> &[VisibilitySource] {
            &self.definitions
        }
        fn visibility_source_active(
            &mut self,
            _: usize,
            _: i32,
            s: i32,
        ) -> Result<bool, Self::Error> {
            self.trace.push(format!("active{s}"));
            Ok(s != 9)
        }
        fn dynamic_visibility(&mut self, _: usize, s: i32) -> Result<Vec<i32>, Self::Error> {
            self.trace.push(format!("dynamic{s}"));
            Ok(if s == 8 { vec![] } else { vec![-7, 999] })
        }
        fn visibility_changed(&mut self, event: u32, a: i32, b: i32) -> Result<(), Self::Error> {
            assert_eq!(event, VISIBILITY_CHANGED);
            self.trace.push(format!(
                "visibility{a}:{b}:{}",
                self.relations.has_met(a, b).unwrap()
            ));
            if self.fail_visibility {
                return Err("callback failed");
            }
            self.players[a as usize].sources[b as usize] = 123;
            Ok(())
        }
        fn log_has_met(&mut self, a: i32, b: i32) -> Result<(), Self::Error> {
            assert!(self.relations.has_met(a, b).unwrap());
            self.trace.push(format!("log{a}:{b}"));
            self.turn += 1;
            Ok(())
        }
        fn find_unit(&mut self, _: ComponentId) -> Result<Option<()>, Self::Error> {
            self.trace.push("find source".into());
            Ok(None)
        }
        fn unit(&self, _: ()) -> Result<&Unit, Self::Error> {
            Err("missing unit")
        }
        fn unit_mut(&mut self, _: ()) -> Result<&mut Unit, Self::Error> {
            Err("missing unit")
        }
        fn preflight_movement_report(&self, _: (), _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn movement_report(&mut self, _: PlacementReport) -> Result<(), Self::Error> {
            Err("unreached")
        }
        fn notify_contact(&mut self, p: i32, _: usize, _: ()) -> Result<(), Self::Error> {
            self.trace.push(format!("notify{p}"));
            Ok(())
        }
        fn report(&mut self, r: Report) -> Result<(), Self::Error> {
            assert_eq!(r.source, SOURCE);
            self.trace.push(format!("report{:08x}", r.event));
            Ok(())
        }
        fn closest_city(&mut self, p: usize, _: ()) -> Result<Option<usize>, Self::Error> {
            self.trace.push(format!("city{p}"));
            Ok(self.distances[p].map(|_| p))
        }
        fn city_distance(&mut self, c: usize, _: ()) -> Result<i32, Self::Error> {
            self.trace.push(format!("distance{c}"));
            self.distances[c].ok_or("absent")
        }
        fn city_location(&mut self, c: usize) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: c as i32, y: 7 })
        }
        fn hospitality_distance(&mut self) -> Result<i32, Self::Error> {
            Ok(self.threshold)
        }
        fn request_first_meeting(&mut self, r: FirstMeetingRequest) -> Result<(), Self::Error> {
            self.trace.push("request".into());
            self.request = Some(r);
            Ok(())
        }
        fn signal_meet(&mut self, a: usize, b: usize) -> Result<(), Self::Error> {
            assert_eq!((a, b), (0, 1));
            self.trace.push("signal".into());
            Ok(())
        }
    }
    #[test]
    fn full_major_contact_orders_mutations_movement_reports_request_and_signal() {
        let mut c = fixture();
        assert_eq!(meet(&mut c, 0, 1, Some(()), SOURCE), Ok(true));
        assert_eq!(c.players[0].gossip, vec![(1, Vec::new().into())]);
        assert_eq!(c.players[1].gossip, vec![(0, Vec::new().into())]);
        assert!(c.relations.has_met(1, 0).unwrap());
        // The second setter writes the live turn after the first log callback.
        assert_eq!(c.relations.read_prefix().unwrap().at_c8[0].changed_turn, 11);
        assert_eq!(
            c.trace,
            [
                "log0:1",
                "log1:0",
                "find source",
                "mode",
                "notify0",
                "notify1",
                "report133d411c",
                "report892f171c",
                "city0",
                "distance0",
                "city1",
                "distance1",
                "request",
                "signal"
            ]
        );
        let request = c.request.unwrap();
        assert_eq!(request.statement_type, SECOND_CITY_MEETING); // tie chooses second.
        assert_eq!((request.from, request.to), (1, 0));
        assert_eq!(request.hospitality_city, PlotCoord { x: 1, y: 7 });
        assert_eq!(meet(&mut c, 0, 1, Some(()), SOURCE), Ok(false));
    }
    #[test]
    fn multiplayer_suppresses_only_notifications_not_first_meeting_request() {
        let mut c = fixture();
        c.multiplayer = true;
        meet(&mut c, 0, 1, Some(()), SOURCE).unwrap();
        assert!(!c.trace.iter().any(|s| s.starts_with("notify")));
        assert!(c.request.is_some());
        assert_eq!(c.trace.last().unwrap(), "signal");
        let mut c = fixture();
        c.multiplayer = true;
        c.players[1].human = true;
        meet(&mut c, 0, 1, Some(()), SOURCE).unwrap();
        assert!(c.trace.contains(&"notify0".into()));
        assert!(c.request.is_none());
    }
    #[test]
    fn hospitality_uses_distance_threshold_ties_and_neutral_invalid_location() {
        for (distances, kind, x) in [
            ([Some(2), Some(3)], FIRST_CITY_MEETING, 0),
            ([Some(8), Some(4)], SECOND_CITY_MEETING, 1),
            ([None, None], NEUTRAL_MEETING, -9999),
            ([Some(5), Some(6)], NEUTRAL_MEETING, -9999),
        ] {
            let mut c = fixture();
            c.distances = distances;
            meet(&mut c, 0, 1, Some(()), SOURCE).unwrap();
            let r = c.request.unwrap();
            assert_eq!((r.statement_type, r.hospitality_city.x), (kind, x));
        }
    }
    #[test]
    fn barbarian_early_report_has_no_general_report_or_final_signal() {
        let mut c = fixture();
        c.players[0].major = false;
        c.players[0].barbarian = true;
        c.players[0].level = 0xd959ec24;
        meet(&mut c, 0, 1, Some(()), SOURCE).unwrap();
        assert_eq!(
            c.trace,
            ["log0:1", "log1:0", "find source", "report09a285e8"]
        );
    }
    #[test]
    fn visibility_sum_uses_first_dynamic_value_and_preserves_last_contributing_source() {
        let mut c = fixture();
        c.definitions = vec![
            VisibilitySource {
                index: 2,
                dynamic: false,
            },
            VisibilitySource {
                index: 4,
                dynamic: true,
            },
            VisibilitySource {
                index: 9,
                dynamic: false,
            },
        ];
        recheck_visibility(&mut c, 0, 1).unwrap();
        assert_eq!(c.players[0].visibility[1], -6);
        assert_eq!(c.players[0].sources[1], 4); // inactive sources preserve the last contribution.
        assert_eq!(
            c.trace,
            [
                "active2",
                "active4",
                "dynamic4",
                "active9",
                "visibility0:1:false"
            ]
        );
        c.definitions = vec![
            VisibilitySource {
                index: 2,
                dynamic: false,
            },
            VisibilitySource {
                index: 8,
                dynamic: true,
            },
            VisibilitySource {
                index: 9,
                dynamic: false,
            },
        ];
        recheck_visibility(&mut c, 0, 1).unwrap();
        assert_eq!(c.players[0].visibility[1], 1);
        assert_eq!(c.players[0].sources[1], 2);
        c.definitions = vec![
            VisibilitySource {
                index: 2,
                dynamic: false
            };
            6
        ];
        recheck_visibility(&mut c, 0, 1).unwrap();
        assert_eq!(c.players[0].visibility[1], 4);
    }
    #[test]
    fn unavailable_preflight_changes_nothing_and_real_callback_failure_keeps_prefix() {
        let mut c = fixture();
        c.unavailable = true;
        assert_eq!(
            meet(&mut c, 0, 1, Some(()), SOURCE),
            Err(Error::EffectsUnavailable("unsupported"))
        );
        assert!(!c.relations.has_met(0, 1).unwrap());
        assert!(c.players[0].gossip.is_empty());
        assert!(c.trace.is_empty());
        let mut c = fixture();
        c.fail_visibility = true;
        c.definitions = vec![VisibilitySource {
            index: 2,
            dynamic: false,
        }];
        assert_eq!(
            meet(&mut c, 0, 1, Some(()), SOURCE),
            Err(Error::Context("callback failed"))
        );
        assert_eq!(c.players[0].visibility[1], 1);
        assert_eq!(c.players[0].sources[1], -1);
        assert!(!c.relations.has_met(0, 1).unwrap());
        assert_eq!(c.players[0].gossip.len(), 1);
    }
    #[test]
    fn visibility_reports_precede_contact_and_sources_follow_reentrant_callbacks() {
        let mut c = fixture();
        c.definitions = vec![VisibilitySource {
            index: 2,
            dynamic: false,
        }];
        meet(&mut c, 0, 1, None, SOURCE).unwrap();
        let first = c
            .trace
            .iter()
            .position(|s| s == "visibility0:1:false")
            .unwrap();
        let first_log = c.trace.iter().position(|s| s == "log0:1").unwrap();
        let second = c
            .trace
            .iter()
            .position(|s| s == "visibility1:0:true")
            .unwrap();
        let second_log = c.trace.iter().position(|s| s == "log1:0").unwrap();
        assert!(first < first_log && first_log < second && second < second_log);
        // Callback writes 123; the native post-report assignment overwrites it.
        assert_eq!(c.players[0].sources[1], 2);
        assert_eq!(c.players[1].sources[0], 2);
    }
    #[test]
    fn dead_and_same_player_guards_and_null_plot_do_not_invent_effects() {
        let mut c = fixture();
        c.players[0].alive = false;
        assert_eq!(meet(&mut c, 0, 1, Some(()), SOURCE), Ok(false));
        assert!(c.trace.is_empty());
        assert_eq!(meet(&mut c, -1, 1, Some(()), SOURCE), Ok(false));
        assert_eq!(meet(&mut c, 0, 0, Some(()), SOURCE), Ok(false));
        let mut c = fixture();
        meet(&mut c, 0, 1, None, SOURCE).unwrap();
        assert_eq!(
            c.trace,
            [
                "log0:1",
                "log1:0",
                "find source",
                "mode",
                "report133d411c",
                "signal"
            ]
        );
    }
}
