//! Linux XP2 AI.Diplomatic version 15/18 saved wire, distinct from Player.Diplomacy.
//! No native lookups, archive changes, analyzers, callbacks, decisions or RNG run.
use crate::unit::Limits;
pub const VERSION: u32 = 18;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    ConditionalPayload,
    UnsupportedLegacyWrite,
    Version { expected: u32, actual: u32 },
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Diplomatic wire: {self:?}")
    }
}
impl std::error::Error for Error {}
trait Wire: Sized {
    const MIN: usize;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let raw = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(raw.try_into().unwrap())
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = u32::read(self)?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
}
struct Writer {
    data: Option<Vec<u8>>,
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn put(&mut self, raw: &[u8]) -> Result<(), Error> {
        self.pos = self
            .pos
            .checked_add(raw.len())
            .ok_or(Error::ResourceLimit)?;
        if self.pos > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if let Some(out) = &mut self.data {
            out.extend_from_slice(raw)
        }
        Ok(())
    }
}
macro_rules! scalar {
    ($t:ty,$n:expr) => {
        impl Wire for $t {
            const MIN: usize = $n;
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_le_bytes(r.take()?))
            }
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
        }
    };
}
scalar!(u32, 4);
scalar!(i32, 4);
/// Saved native bool object byte; no normalization or truth-value claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoolByte(pub u8);
impl Wire for BoolByte {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self(r.take::<1>()?[0]))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[self.0])
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        r.items = r.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        let end = n
            .checked_mul(T::MIN)
            .and_then(|n| r.pos.checked_add(n))
            .ok_or(Error::ResourceLimit)?;
        if r.items > r.limits.max_items || end > r.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > r.data.len() {
            return Err(Error::Truncated);
        }
        n.checked_mul(std::mem::size_of::<T>())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut v = Self::new();
        v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
        for _ in 0..n {
            v.push(T::read(r)?)
        }
        Ok(v)
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let n = u32::try_from(self.len()).map_err(|_| Error::ResourceLimit)?;
        w.items = w
            .items
            .checked_add(self.len())
            .ok_or(Error::ResourceLimit)?;
        if w.items > w.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        n.write(w)?;
        for v in self {
            v.write(w)?
        }
        Ok(())
    }
}
/// Definition hash and value, preserving saved order, duplicates and sentinels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashEntry<T> {
    pub hash: u32,
    pub value: T,
}
impl<T: Wire> Wire for HashEntry<T> {
    const MIN: usize = 4 + T::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(Self {
            hash: u32::read(r)?,
            value: T::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.hash.write(w)?;
        self.value.write(w)
    }
}
macro_rules! record {
 ($name:ident,$version:expr,{$($field:ident:$ty:ty),* $(,)?})=>{
 #[derive(Clone,Debug,Eq,PartialEq)] pub struct $name {$(pub $field:$ty),*}
 impl Wire for $name {const MIN:usize=4$(+<$ty as Wire>::MIN)*;
 fn read(r:&mut Reader<'_>)->Result<Self,Error>{r.version($version)?;Ok(Self{$($field:<$ty as Wire>::read(r)?),*})}
 fn write(&self,w:&mut Writer)->Result<(),Error>{($version as u32).write(w)?;$(self.$field.write(w)?;)*Ok(())}}
 };
}
impl Wire for u8 {
    const MIN: usize = 1;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(r.take::<1>()?[0])
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        w.put(&[*self])
    }
}
record!(TradeItem,1,{game_turn:i32,other_player:i32,item_type:u32,item_sub_type:u32,item_value:u32});
record!(StateVector,2,{states:Vec<DiplomaticState>,is_updated:BoolByte});
record!(DiplomaticState,5,{owner:i32,opponent:i32,state_name:Vec<u8>,is_updating:BoolByte,is_active:BoolByte,has_changed_state:BoolByte,diplomatic_bonus_q8:i32,state_hash:u32,allowed_action_hashes:Vec<u32>,response_action_hashes:Vec<u32>,desired_action_hashes:Vec<u32>,transitions:Vec<DiplomaticTransition>,triggers:Vec<TriggeredTransition>});
record!(DiplomaticTransition,2,{player_diplomacy_player:i32,opponent_player:i32,current_score_q8:i32,transition_index:i32,next_state:Vec<u8>,next_state_index:i32,has_required_min:BoolByte,required_min:i32,has_required_max:BoolByte,required_max:i32,has_allowed_min:BoolByte,allowed_min:i32,has_allowed_max:BoolByte,allowed_max:i32,must_transition:BoolByte,transition_score_q8:i32,throttle_turns:i32,condition_hash:u32,transition_hash:u32,transition_action_hashes:Vec<u32>,actions_gained_hashes:Vec<u32>,actions_lost_hashes:Vec<u32>});
record!(TriggeredTransition,1,{next_state_name:Vec<u8>,next_state_index:i32,trigger:i32});
record!(ActionEvalVector,2,{handlers:Vec<DiplomaticActionHandler>,is_updated:BoolByte});
record!(ControlHandle,1,{player_id:i32,id:u32});
/// Saved rows remain ordered wire pairs, not initialized native definition arrays.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diplomatic {
    pub wire_version: u32,
    pub player_id: i32,
    pub min_force_to_notice_q8: i32,
    pub min_troubling_force_q8: i32,
    pub diplomatic_scores_q8: Vec<i32>,
    pub base_diplo_scores_q8: Vec<i32>,
    pub visible_scores_q8: Vec<Vec<i32>>,
    pub diplomatic_states: StateVector,
    pub diplo_action_evaluator: ActionEvalVector,
    pub num_majors_met: i32,
    pub player_threats_q8: Vec<i32>,
    pub player_major_threats: Vec<BoolByte>,
    pub player_trusts_q8: Vec<i32>,
    pub managed_contracts: Vec<ControlHandle>,
    pub best_potential_friend: i32,
    pub partner_action_preference: Vec<Vec<HashEntry<i32>>>,
    pub partner_action_worth_q8: Vec<Vec<HashEntry<i32>>>,
    pub closed_partner_action: Vec<Vec<HashEntry<BoolByte>>>,
    pub requested_items: Vec<TradeItem>,
    /// None at version 15: no payload; native initialization is not applied.
    pub excluded_items: Option<Vec<TradeItem>>,
    pub favor_traded_this_turn: i32,
    pub done_off_turn: BoolByte,
}
/// Native Read resets each omitted row at the actual DiplomaticAction definition count.
/// This descriptor does not invent that count or materialize a runtime table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OmittedPartnerTableDefaults {
    pub rows: usize,
    pub worth_q8: i32,
    pub closed: bool,
}
impl Diplomatic {
    pub fn omitted_partner_table_defaults(&self) -> Option<OmittedPartnerTableDefaults> {
        (self.wire_version == 15).then_some(OmittedPartnerTableDefaults {
            rows: 64,
            worth_q8: 0,
            closed: false,
        })
    }
}
impl Wire for Diplomatic {
    // Minimum version-15 wire; omitted tables have no wire count or item charge.
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<Vec<i32>> as Wire>::MIN
        + <StateVector as Wire>::MIN
        + <ActionEvalVector as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<BoolByte> as Wire>::MIN
        + <Vec<i32> as Wire>::MIN
        + <Vec<ControlHandle> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<Vec<HashEntry<i32>>> as Wire>::MIN
        + <Vec<TradeItem> as Wire>::MIN
        + <BoolByte as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 15 | 18) {
            return Err(Error::Version {
                expected: VERSION,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            player_id: <i32 as Wire>::read(r)?,
            min_force_to_notice_q8: <i32 as Wire>::read(r)?,
            min_troubling_force_q8: <i32 as Wire>::read(r)?,
            diplomatic_scores_q8: <Vec<i32> as Wire>::read(r)?,
            base_diplo_scores_q8: <Vec<i32> as Wire>::read(r)?,
            visible_scores_q8: <Vec<Vec<i32>> as Wire>::read(r)?,
            diplomatic_states: <StateVector as Wire>::read(r)?,
            diplo_action_evaluator: <ActionEvalVector as Wire>::read(r)?,
            num_majors_met: <i32 as Wire>::read(r)?,
            player_threats_q8: <Vec<i32> as Wire>::read(r)?,
            player_major_threats: <Vec<BoolByte> as Wire>::read(r)?,
            player_trusts_q8: <Vec<i32> as Wire>::read(r)?,
            managed_contracts: <Vec<ControlHandle> as Wire>::read(r)?,
            best_potential_friend: <i32 as Wire>::read(r)?,
            partner_action_preference: <Vec<Vec<HashEntry<i32>>> as Wire>::read(r)?,
            partner_action_worth_q8: if wire_version == 18 {
                <Vec<Vec<HashEntry<i32>>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            closed_partner_action: if wire_version == 18 {
                <Vec<Vec<HashEntry<BoolByte>>> as Wire>::read(r)?
            } else {
                Vec::new()
            },
            requested_items: <Vec<TradeItem> as Wire>::read(r)?,
            excluded_items: if wire_version == 18 {
                Some(<Vec<TradeItem> as Wire>::read(r)?)
            } else {
                None
            },
            favor_traded_this_turn: if wire_version == 18 {
                <i32 as Wire>::read(r)?
            } else {
                0
            },
            done_off_turn: <BoolByte as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != VERSION {
            return Err(Error::UnsupportedLegacyWrite);
        }
        let excluded = self
            .excluded_items
            .as_ref()
            .ok_or(Error::ConditionalPayload)?;
        VERSION.write(w)?;
        self.player_id.write(w)?;
        self.min_force_to_notice_q8.write(w)?;
        self.min_troubling_force_q8.write(w)?;
        self.diplomatic_scores_q8.write(w)?;
        self.base_diplo_scores_q8.write(w)?;
        self.visible_scores_q8.write(w)?;
        self.diplomatic_states.write(w)?;
        self.diplo_action_evaluator.write(w)?;
        self.num_majors_met.write(w)?;
        self.player_threats_q8.write(w)?;
        self.player_major_threats.write(w)?;
        self.player_trusts_q8.write(w)?;
        self.managed_contracts.write(w)?;
        self.best_potential_friend.write(w)?;
        self.partner_action_preference.write(w)?;
        self.partner_action_worth_q8.write(w)?;
        self.closed_partner_action.write(w)?;
        self.requested_items.write(w)?;
        excluded.write(w)?;
        self.favor_traded_this_turn.write(w)?;
        self.done_off_turn.write(w)?;
        Ok(())
    }
}
/// The two indexed arrays exist only when both saved player IDs differ from -1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiplomaticActionHandler {
    pub owner: i32,
    pub opponent: i32,
    pub desirability: Option<Vec<HashEntry<i32>>>,
    pub cost: Option<Vec<HashEntry<i32>>>,
    pub alliance_value_q8: i32,
    pub war_value_q8: i32,
    pub war_value_penalties_q8: i32,
    pub is_initialized: BoolByte,
}
impl Wire for DiplomaticActionHandler {
    const MIN: usize = 25;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(5)?;
        let owner = i32::read(r)?;
        let opponent = i32::read(r)?;
        let present = owner != -1 && opponent != -1;
        Ok(Self {
            owner,
            opponent,
            desirability: if present { Some(Vec::read(r)?) } else { None },
            cost: if present { Some(Vec::read(r)?) } else { None },
            alliance_value_q8: i32::read(r)?,
            war_value_q8: i32::read(r)?,
            war_value_penalties_q8: i32::read(r)?,
            is_initialized: BoolByte::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let present = self.owner != -1 && self.opponent != -1;
        if self.desirability.is_some() != present || self.cost.is_some() != present {
            return Err(Error::ConditionalPayload);
        }
        5u32.write(w)?;
        self.owner.write(w)?;
        self.opponent.write(w)?;
        if let Some(v) = &self.desirability {
            v.write(w)?
        }
        if let Some(v) = &self.cost {
            v.write(w)?
        }
        self.alliance_value_q8.write(w)?;
        self.war_value_q8.write(w)?;
        self.war_value_penalties_q8.write(w)?;
        self.is_initialized.write(w)
    }
}
impl Diplomatic {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (v, n) = Self::decode_prefix(data, limits)?;
        if n != data.len() {
            return Err(Error::Trailing);
        }
        Ok(v)
    }
    pub fn decode_prefix(data: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        Self::decode_prefix_with_items(data, limits).map(|(v, n, _)| (v, n))
    }
    pub fn decode_prefix_with_items(
        data: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data,
            pos: 0,
            items: 0,
            limits,
        };
        let v = Self::read(&mut r)?;
        Ok((v, r.pos, r.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            data: None,
            pos: 0,
            items: 0,
            limits,
        };
        self.write(&mut w)?;
        let mut out = Vec::new();
        out.try_reserve_exact(w.pos)
            .map_err(|_| Error::ResourceLimit)?;
        let items = w.items;
        w = Writer {
            data: Some(out),
            pos: 0,
            items: 0,
            limits,
        };
        self.write(&mut w)?;
        Ok((w.data.unwrap(), items))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Diplomatic {
        Diplomatic {
            wire_version: VERSION,
            player_id: -7,
            min_force_to_notice_q8: i32::MIN,
            min_troubling_force_q8: i32::MAX,
            diplomatic_scores_q8: vec![i32::MIN],
            base_diplo_scores_q8: vec![-7],
            visible_scores_q8: vec![vec![-7]],
            diplomatic_states: StateVector {
                states: vec![DiplomaticState {
                    owner: i32::MIN,
                    opponent: i32::MAX,
                    state_name: vec![0, 255],
                    is_updating: BoolByte(2),
                    is_active: BoolByte(2),
                    has_changed_state: BoolByte(2),
                    diplomatic_bonus_q8: i32::MAX,
                    state_hash: u32::MAX,
                    allowed_action_hashes: vec![u32::MAX],
                    response_action_hashes: vec![u32::MAX],
                    desired_action_hashes: vec![u32::MAX],
                    transitions: vec![DiplomaticTransition {
                        player_diplomacy_player: -7,
                        opponent_player: i32::MIN,
                        current_score_q8: i32::MAX,
                        transition_index: -7,
                        next_state: vec![0, 255],
                        next_state_index: -7,
                        has_required_min: BoolByte(2),
                        required_min: i32::MAX,
                        has_required_max: BoolByte(2),
                        required_max: i32::MIN,
                        has_allowed_min: BoolByte(2),
                        allowed_min: -7,
                        has_allowed_max: BoolByte(2),
                        allowed_max: i32::MAX,
                        must_transition: BoolByte(2),
                        transition_score_q8: i32::MIN,
                        throttle_turns: i32::MAX,
                        condition_hash: u32::MAX,
                        transition_hash: u32::MAX,
                        transition_action_hashes: vec![u32::MAX],
                        actions_gained_hashes: vec![u32::MAX],
                        actions_lost_hashes: vec![u32::MAX],
                    }],
                    triggers: vec![TriggeredTransition {
                        next_state_name: vec![0, 255],
                        next_state_index: -7,
                        trigger: i32::MIN,
                    }],
                }],
                is_updated: BoolByte(2),
            },
            diplo_action_evaluator: ActionEvalVector {
                handlers: vec![DiplomaticActionHandler {
                    owner: -7,
                    opponent: i32::MIN,
                    desirability: Some(vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }]),
                    cost: Some(vec![HashEntry {
                        hash: u32::MAX,
                        value: i32::MIN,
                    }]),
                    alliance_value_q8: i32::MAX,
                    war_value_q8: -7,
                    war_value_penalties_q8: i32::MIN,
                    is_initialized: BoolByte(2),
                }],
                is_updated: BoolByte(2),
            },
            num_majors_met: i32::MIN,
            player_threats_q8: vec![-7],
            player_major_threats: vec![BoolByte(2)],
            player_trusts_q8: vec![i32::MIN],
            managed_contracts: vec![ControlHandle {
                player_id: i32::MIN,
                id: u32::MAX,
            }],
            best_potential_friend: -7,
            partner_action_preference: vec![vec![HashEntry {
                hash: u32::MAX,
                value: i32::MIN,
            }]],
            partner_action_worth_q8: vec![vec![HashEntry {
                hash: u32::MAX,
                value: i32::MAX,
            }]],
            closed_partner_action: vec![vec![HashEntry {
                hash: u32::MAX,
                value: BoolByte(2),
            }]],
            requested_items: vec![TradeItem {
                game_turn: -7,
                other_player: i32::MIN,
                item_type: u32::MAX,
                item_sub_type: u32::MAX,
                item_value: u32::MAX,
            }],
            excluded_items: Some(vec![TradeItem {
                game_turn: i32::MIN,
                other_player: i32::MAX,
                item_type: u32::MAX,
                item_sub_type: u32::MAX,
                item_value: u32::MAX,
            }]),
            favor_traded_this_turn: -7,
            done_off_turn: BoolByte(2),
        }
    }
    fn legacy15() -> Vec<u8> {
        fn words(raw: &mut Vec<u8>, values: &[u32]) {
            for value in values {
                raw.extend_from_slice(&value.to_le_bytes());
            }
        }
        let mut raw = Vec::new();
        words(
            &mut raw,
            &[
                15,
                u32::MAX,
                i32::MIN as u32,
                i32::MAX as u32,
                2,
                i32::MIN as u32,
                (-257i32) as u32,
                1,
                i32::MAX as u32,
                1,
                2,
                u32::MAX,
                23,
                2,
                0,
            ],
        );
        raw.push(197); // StateVector raw updated byte
        words(&mut raw, &[2, 0]);
        raw.push(2); // ActionEvalVector raw updated byte
        words(&mut raw, &[u32::MAX, 1, (-257i32) as u32, 1]);
        raw.push(197); // raw major-threat byte
        words(
            &mut raw,
            &[
                1,
                i32::MIN as u32,
                1,
                1,
                u32::MAX,
                u32::MAX,
                u32::MAX,
                1,
                2,
                u32::MAX,
                i32::MIN as u32,
                u32::MAX,
                i32::MAX as u32,
                1,
                1,
                u32::MAX,
                i32::MAX as u32,
                u32::MAX,
                0x80000001,
                7,
            ],
        );
        raw.push(197);
        raw
    }
    #[test]
    fn legacy15_omissions_and_shared_budget() {
        let raw = legacy15();
        assert_eq!(raw.len(), 168);
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: 14,
        };
        let (v, used, items) = Diplomatic::decode_prefix_with_items(&raw, cap).unwrap();
        assert_eq!((used, items), (168, 14));
        assert_eq!(v.wire_version, 15);
        assert_eq!(v.player_id, -1);
        assert_eq!(v.diplomatic_scores_q8, [i32::MIN, -257]);
        assert_eq!(
            v.partner_action_preference[0]
                .iter()
                .map(|x| (x.hash, x.value))
                .collect::<Vec<_>>(),
            [(u32::MAX, i32::MIN), (u32::MAX, i32::MAX)]
        );
        assert!(v.partner_action_worth_q8.is_empty());
        assert!(v.closed_partner_action.is_empty());
        assert_eq!(
            v.omitted_partner_table_defaults(),
            Some(OmittedPartnerTableDefaults {
                rows: 64,
                worth_q8: 0,
                closed: false,
            })
        );
        assert_eq!(v.excluded_items, None);
        assert_eq!(v.favor_traded_this_turn, 0);
        assert_eq!(v.requested_items[0].item_sub_type, 0x80000001);
        assert_eq!(v.done_off_turn, BoolByte(197));
        assert_eq!(v.encode(cap), Err(Error::UnsupportedLegacyWrite));
        for n in 0..raw.len() {
            assert!(Diplomatic::decode(&raw[..n], cap).is_err());
        }
        for short in [
            Limits {
                max_bytes: 167,
                ..cap
            },
            Limits {
                max_items: 13,
                ..cap
            },
        ] {
            assert_eq!(Diplomatic::decode(&raw, short), Err(Error::ResourceLimit));
        }
        let mut nested = raw.clone();
        nested[52..56].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            Diplomatic::decode(&nested, cap),
            Err(Error::Version {
                expected: 2,
                actual: 1
            })
        );
        let mut unsupported = raw.clone();
        unsupported[..4].copy_from_slice(&16u32.to_le_bytes());
        assert_eq!(
            Diplomatic::decode(&unsupported, cap),
            Err(Error::Version {
                expected: 18,
                actual: 16
            })
        );
    }
    #[test]
    fn current_excluded_items_must_be_retained() {
        let mut v = sample();
        assert_eq!(v.omitted_partner_table_defaults(), None);
        v.excluded_items = None;
        assert_eq!(v.encode(Limits::default()), Err(Error::ConditionalPayload));
    }
    #[test]
    fn all_nested_boundaries() {
        let v = sample();
        let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
        let cap = Limits {
            max_bytes: raw.len(),
            max_items: items,
        };
        assert_eq!(Diplomatic::decode(&raw, cap), Ok(v.clone()));
        for n in 0..raw.len() {
            assert!(Diplomatic::decode(&raw[..n], cap).is_err())
        }
        for short in [
            Limits {
                max_bytes: raw.len() - 1,
                ..cap
            },
            Limits {
                max_items: items - 1,
                ..cap
            },
        ] {
            assert_eq!(Diplomatic::decode(&raw, short), Err(Error::ResourceLimit));
            assert_eq!(v.encode(short), Err(Error::ResourceLimit));
        }
        let mut tail = raw.clone();
        tail.push(7);
        assert_eq!(Diplomatic::decode(&tail, cap), Err(Error::Trailing));
        assert_eq!(
            Diplomatic::decode_prefix_with_items(&tail, cap),
            Ok((v, raw.len(), items))
        );
    }
    #[test]
    fn conditional_handlers_and_hostile_input() {
        for (owner, opponent) in [(-1, -1), (-1, 3), (3, -1), (3, 4)] {
            let mut v = sample();
            let h = &mut v.diplo_action_evaluator.handlers[0];
            h.owner = owner;
            h.opponent = opponent;
            if owner == -1 || opponent == -1 {
                h.desirability = None;
                h.cost = None;
            }
            let (raw, items) = v.encode_with_items(Limits::default()).unwrap();
            let cap = Limits {
                max_bytes: raw.len(),
                max_items: items,
            };
            assert_eq!(Diplomatic::decode(&raw, cap), Ok(v.clone()));
            for n in 0..raw.len() {
                assert!(Diplomatic::decode(&raw[..n], cap).is_err());
            }
            v.diplo_action_evaluator.handlers[0].cost = None;
            if owner != -1 && opponent != -1 {
                assert_eq!(v.encode(cap), Err(Error::ConditionalPayload));
            }
        }
        let mut raw = sample().encode(Limits::default()).unwrap();
        raw[0..4].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(
            Diplomatic::decode(&raw, Limits::default()),
            Err(Error::Version {
                expected: 18,
                actual: 99
            })
        );
        raw[0..4].copy_from_slice(&18u32.to_le_bytes());
        raw[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Diplomatic::decode(&raw, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn every_nested_version_is_gated() {
        fn rejects<T: Wire>(expected: u32) {
            let data = 99u32.to_le_bytes();
            let mut r = Reader {
                data: &data,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: 4,
                    max_items: 0,
                },
            };
            assert!(
                matches!(T::read(&mut r), Err(Error::Version { expected: e, actual: 99 }) if e == expected)
            );
        }
        rejects::<Diplomatic>(18);
        rejects::<StateVector>(2);
        rejects::<DiplomaticState>(5);
        rejects::<DiplomaticTransition>(2);
        rejects::<TriggeredTransition>(1);
        rejects::<ActionEvalVector>(2);
        rejects::<DiplomaticActionHandler>(5);
        rejects::<TradeItem>(1);
        rejects::<ControlHandle>(1);
    }
}
