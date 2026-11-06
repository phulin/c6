//! Linux TradeRelations concrete effect and diplomatic tracker writes.
//! Collection membership and subject requirements belong to the caller's one
//! modifier/requirement owner. This module never selects a diplomatic policy.
use c6_core::{
    loaded_state::LoadedState,
    player_diplomacy_blackboard::{DiplomaticModifierTracker as Tracker, ModifierEntry},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Parameters {
    pub trade_bonus: i32,
    pub no_trade_penalty: i32,
    pub bonus_per_route: bool,
    pub only_inbound_trade: bool,
}
impl Parameters {
    /// Native Initialize defaults, overridden by the actual modifier arguments.
    pub fn from_rules(rules: &crate::Rules, definition: &[u8]) -> Result<Self, &'static str> {
        let mut result = Self {
            trade_bonus: 2,
            no_trade_penalty: 0,
            bonus_per_route: false,
            only_inbound_trade: false,
        };
        let table = rules
            .gameplay()
            .table("ModifierArguments")
            .ok_or("TradeRelations ModifierArguments")?;
        let mut seen = Vec::new();
        for row in table.rows() {
            if row
                .text("ModifierId")
                .map_err(|_| "TradeRelations ModifierId")?
                .as_bytes()
                != definition
            {
                continue;
            }
            let name = row
                .text("Name")
                .map_err(|_| "TradeRelations argument name")?;
            if !matches!(
                name,
                "TradeBonus" | "NoTradePenalty" | "BonusPerRoute" | "OnlyInboundTrade"
            ) {
                continue;
            }
            if seen.contains(&name) {
                return Err("duplicate TradeRelations argument");
            }
            seen.push(name);
            let value = row
                .get("Value")
                .map_err(|_| "TradeRelations argument value")?;
            let integer = match value {
                crate::SqlValue::Integer(value) => i32::try_from(*value).ok(),
                crate::SqlValue::Text(value) => {
                    std::str::from_utf8(value).ok().and_then(|s| s.parse().ok())
                }
                _ => None,
            }
            .ok_or("TradeRelations integer argument")?;
            match name {
                "TradeBonus" => result.trade_bonus = integer,
                "NoTradePenalty" => result.no_trade_penalty = integer,
                "BonusPerRoute" if matches!(integer, 0 | 1) => {
                    result.bonus_per_route = integer != 0
                }
                "OnlyInboundTrade" if matches!(integer, 0 | 1) => {
                    result.only_inbound_trade = integer != 0
                }
                _ => return Err("TradeRelations bool argument"),
            }
        }
        Ok(result)
    }
    /// CountOutgoingRoutesTo is directional. Non-per-route mode grants one
    /// bonus for either direction, including when both directions are active.
    pub fn value(
        self,
        outgoing: i32,
        incoming: i32,
        subject_outgoing: i32,
        waiting_to_renew: bool,
    ) -> i32 {
        let mut value = if incoming > 0 {
            self.trade_bonus
                .wrapping_mul(if self.bonus_per_route { incoming } else { 1 })
        } else {
            0
        };
        if !self.only_inbound_trade && outgoing > 0 && (incoming == 0 || self.bonus_per_route) {
            value = value.wrapping_add(self.trade_bonus.wrapping_mul(if self.bonus_per_route {
                outgoing
            } else {
                1
            }));
        }
        if ((!self.only_inbound_trade && outgoing == 0 && incoming == 0)
            || (self.only_inbound_trade && incoming == 0 && subject_outgoing > 0))
            && !waiting_to_renew
        {
            value = value.wrapping_sub(self.no_trade_penalty);
        }
        value
    }
}

/// Actual BEFORE DiplomacyBaseEffect fields, retained independently of rule
/// defaults because AccumulateExtraValue can change their signs at runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Effect {
    pub parameters: Parameters,
    pub increment: i32,
    pub increment_turns: i32,
    pub cooldown_turns: i32,
    pub reduction: i32,
    pub visibility: i32,
    pub remove_immediately: bool,
    pub current_update_instance: i32,
}

/// Concrete Apply after the shared modifier has qualified its current subject.
/// Both flags clear, or the actual human-owner CanSendStatement return, prove
/// the reached SimpleEffect tail cannot send a statement or consume RNG.
pub fn apply(
    state: &mut LoadedState,
    effect: &mut Effect,
    owner: i32,
    subject: i32,
    instance: i32,
    statement_flags: [bool; 2],
) -> Result<(), &'static str> {
    if statement_flags != [false, false]
        && state
            .player_slot_status(owner)
            .map_err(|_| "TradeRelations owner controls")?
            != 3
    {
        return Err("TradeRelations diplomatic statement continuation");
    }
    let graph = state
        .trade_graph()
        .map_err(|_| "TradeRelations current graph")?
        .graph();
    let mut outgoing = 0i32;
    let mut incoming = 0i32;
    let mut subject_outgoing = 0i32;
    for origin in graph.origins() {
        if origin.vertex.kind != 0 {
            continue;
        }
        for destination in &origin.destinations {
            if destination.vertex.kind != 0 || !destination.edge.has_active_route {
                continue;
            }
            // Manager queries enumerate actual current cities, not dead graph
            // endpoints. The graph itself retains inactive historical edges.
            state
                .city(origin.vertex.data1, origin.vertex.data2 as u32)
                .map_err(|_| "TradeRelations origin city")?;
            state
                .city(destination.vertex.data1, destination.vertex.data2 as u32)
                .map_err(|_| "TradeRelations destination city")?;
            if origin.vertex.data1 == owner && destination.vertex.data1 == subject {
                outgoing = outgoing.wrapping_add(1);
            }
            if origin.vertex.data1 == subject {
                subject_outgoing = subject_outgoing.wrapping_add(1);
                if destination.vertex.data1 == owner {
                    incoming = incoming.wrapping_add(1);
                }
            }
        }
    }
    // The renewal predicate is a pure Unit scan. With zero penalty its result
    // cannot affect any store, callback or RNG in native Apply.
    if effect.parameters.no_trade_penalty != 0 {
        return Err(
            "TradeRelations nonzero penalty needs current trader definitions/renewal query",
        );
    }
    let value = effect
        .parameters
        .value(outgoing, incoming, subject_outgoing, false);
    effect.current_update_instance = instance;
    let entries = state
        .war_diplomacy_mut(owner)
        .map_err(|_| "TradeRelations owner diplomacy")?
        .blackboard
        .diplomatic_modifiers
        .get_mut(usize::try_from(subject).map_err(|_| "TradeRelations subject")?)
        .ok_or("TradeRelations subject map")?;
    apply_value(entries, instance as u32, effect, value)
}

/// TradeRelations.Remove → SimpleEffect.Remove → BaseEffect.Remove. Native
/// retains the map entry; Deactivate clears the accumulation counter and, at
/// nonpositive cooldown, its current value. It does not send a statement.
pub fn remove(
    state: &mut LoadedState,
    owner: i32,
    subject: i32,
    instance: i32,
) -> Result<(), &'static str> {
    let entries = state
        .war_diplomacy_mut(owner)
        .map_err(|_| "TradeRelations removal owner")?
        .blackboard
        .diplomatic_modifiers
        .get_mut(usize::try_from(subject).map_err(|_| "TradeRelations removal subject")?)
        .ok_or("TradeRelations removal map")?;
    let at = tracker_slot(entries, instance as u32)?;
    if let Some(row) = entries.get_mut(at) {
        if row.tracker.is_active {
            row.tracker.is_active = false;
            row.tracker.accumulation_turn_count = 0;
            if row.tracker.cooldown_turns < 1 {
                row.tracker.current_value = 0;
            }
        }
    }
    Ok(())
}

fn multiply(left: i32, right: i32) -> i32 {
    ((i64::from(left) * i64::from(right)) >> 8) as i32
}

fn tracker_slot(entries: &mut Vec<ModifierEntry>, key: u32) -> Result<usize, &'static str> {
    if entries.windows(2).any(|w| w[0].key >= w[1].key) {
        return Err("TradeRelations tracker map order");
    }
    let at = entries.partition_point(|row| row.key < key);
    if entries.get(at).is_none_or(|row| row.key != key) {
        // Native Blackboard Activate AND Deactivate use map::operator[].
        entries.insert(
            at,
            ModifierEntry {
                key,
                tracker: Tracker {
                    is_active: false,
                    current_value: 0,
                    initial_value: 0,
                    max_value: 0,
                    increment: 0,
                    increment_turns: 0,
                    cooldown_turns: 0,
                    accumulation_turn_count: 0,
                    reduction: 0,
                    modifier_name: Vec::new(),
                    visibility: -1,
                },
            },
        );
    }
    Ok(at)
}

fn apply_value(
    entries: &mut Vec<ModifierEntry>,
    key: u32,
    effect: &mut Effect,
    value: i32,
) -> Result<(), &'static str> {
    if entries.windows(2).any(|w| w[0].key >= w[1].key) {
        return Err("TradeRelations tracker map order");
    }
    let at = entries.partition_point(|row| row.key < key);
    let old = entries
        .get(at)
        .filter(|row| row.key == key)
        .map(|row| row.tracker.current_value >> 8)
        .unwrap_or(0);
    // BaseApply's comparison uses AsInt (signed arithmetic shift), including
    // for fractional and negative current values.
    if value == old {
        return Ok(());
    }
    if effect.remove_immediately {
        return Err("TradeRelations immediate-remove effect");
    }
    let at = tracker_slot(entries, key)?;
    let initial = value.wrapping_shl(8);
    initialize_and_activate(
        &mut entries[at].tracker,
        &Tracker {
            is_active: false,
            current_value: 0,
            accumulation_turn_count: 0,
            initial_value: initial,
            max_value: initial,
            increment: effect.increment.wrapping_shl(8),
            reduction: effect.reduction.wrapping_shl(8),
            increment_turns: effect.increment_turns,
            cooldown_turns: effect.cooldown_turns,
            modifier_name: b"LOC_TOOLTIP_SAMPLE_DIPLOMACY_TRADE_RELATIONS".to_vec(),
            visibility: effect.visibility,
        },
    );
    Ok(())
}

/// Blackboard.Activate reinitializes existing parameters before conditional
/// tracker activation; keep its arithmetic shared by concrete opinion leaves.
pub(crate) fn initialize_and_activate(tracker: &mut Tracker, parameters: &Tracker) {
    let initial = parameters.initial_value;
    tracker.initial_value = initial;
    tracker.max_value = parameters.max_value;
    tracker.increment = parameters.increment;
    tracker.reduction = parameters.reduction;
    tracker.increment_turns = parameters.increment_turns;
    tracker.cooldown_turns = parameters.cooldown_turns;
    tracker.modifier_name.clone_from(&parameters.modifier_name);
    tracker.visibility = parameters.visibility;
    if multiply(tracker.increment, tracker.max_value) < 0 {
        tracker.increment = tracker.increment.wrapping_neg();
    }
    if multiply(tracker.reduction, initial) > 0 {
        tracker.reduction = tracker.reduction.wrapping_neg();
    }
    if tracker.is_active && tracker.increment_turns < 2 {
        tracker.increment = tracker.max_value.wrapping_sub(tracker.current_value);
        super::cityless_managers::update_tracker(tracker);
    }
    if tracker.current_value.wrapping_abs() > tracker.max_value.wrapping_abs() {
        tracker.current_value = tracker.max_value;
    }
    // Blackboard's value-change logging guard also calls Activate for an
    // already active tracker; that Activate is a literal no-op.
    if !tracker.is_active {
        tracker.is_active = true;
        tracker.accumulation_turn_count = 0;
        if initial > 0 {
            tracker.current_value = tracker.current_value.max(initial);
        } else if initial < 0 {
            tracker.current_value = tracker.current_value.min(initial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> Parameters {
        Parameters {
            trade_bonus: 2,
            no_trade_penalty: 0,
            bonus_per_route: false,
            only_inbound_trade: false,
        }
    }
    fn effect() -> Effect {
        Effect {
            parameters: parameters(),
            increment: 1,
            increment_turns: 0,
            cooldown_turns: 0,
            reduction: 1,
            visibility: 0,
            remove_immediately: false,
            current_update_instance: 0,
        }
    }
    #[test]
    fn directional_routes_and_penalty_guards() {
        let mut p = parameters();
        assert_eq!(p.value(1, 1, 1, false), 2);
        p.bonus_per_route = true;
        assert_eq!(p.value(2, 3, 3, false), 10);
        p.only_inbound_trade = true;
        assert_eq!(p.value(2, 3, 3, false), 6);
        p.no_trade_penalty = 4;
        assert_eq!(p.value(1, 0, 1, false), -4);
        assert_eq!(p.value(1, 0, 1, true), 0);
        assert_eq!(p.value(0, 0, 0, false), 0);
    }
    #[test]
    fn first_positive_then_zero_preserves_native_tracker_transition() {
        let mut rows = Vec::new();
        let mut effect = effect();
        apply_value(&mut rows, 190, &mut effect, 0).unwrap();
        assert!(rows.is_empty());
        apply_value(&mut rows, 190, &mut effect, 2).unwrap();
        let t = &rows[0].tracker;
        assert_eq!(
            (
                t.current_value,
                t.initial_value,
                t.max_value,
                t.increment,
                t.reduction
            ),
            (512, 512, 512, 256, -256)
        );
        super::super::cityless_managers::update_tracker(&mut rows[0].tracker);
        assert_eq!(rows[0].tracker.accumulation_turn_count, 1);
        apply_value(&mut rows, 190, &mut effect, 0).unwrap();
        assert!(rows[0].tracker.is_active);
        assert_eq!(
            (rows[0].tracker.current_value, rows[0].tracker.max_value),
            (0, 0)
        );
    }
    #[test]
    fn active_reinitialization_uses_difference_and_signed_integer_read() {
        let mut rows = Vec::new();
        let mut effect = effect();
        apply_value(&mut rows, 254, &mut effect, 2).unwrap();
        rows[0].tracker.current_value = 511;
        apply_value(&mut rows, 254, &mut effect, 2).unwrap();
        assert_eq!(rows[0].tracker.current_value, 512);
        assert_eq!(rows[0].tracker.increment, 1);
    }
    #[test]
    fn negative_values_normalize_signs_and_preserve_q8_floor() {
        let mut rows = Vec::new();
        let mut effect = effect();
        apply_value(&mut rows, 254, &mut effect, -2).unwrap();
        assert_eq!(
            (
                rows[0].tracker.current_value,
                rows[0].tracker.increment,
                rows[0].tracker.reduction
            ),
            (-512, -256, 256)
        );
        rows[0].tracker.current_value = -257;
        let before = rows.clone();
        // AsInt(-257) = -2, so BaseApply does not reinitialize this tracker.
        apply_value(&mut rows, 254, &mut effect, -2).unwrap();
        assert_eq!(rows, before);
        let mut p = parameters();
        p.bonus_per_route = true;
        p.trade_bonus = i32::MAX;
        assert_eq!(p.value(2, 0, 0, false), -2);
    }
}
