//! Ordinary Fortify scalar semantics. Queue processing and reporting use the
//! existing manager; turn-dependent progression requires a later activation.
/// CInstance.IsWaiting: five activity values, independent of moves or queue.
pub fn is_waiting(activity: i32) -> bool {
    [
        0x024a74eb,
        0x047c048b,
        0x5ff1bbdf,
        0xb29e7722u32 as i32,
        0xd0aed661u32 as i32,
    ]
    .contains(&activity)
}
/// Update uses wrapping native signed addition, then SetFortifyTurns clamps at 0.
pub fn increment_turns(turns: i32) -> i32 {
    turns.wrapping_add(1).max(0)
}
#[cfg(test)]
mod tests {
    #[test]
    fn waiting_and_wrapping_counter() {
        assert!(super::is_waiting(0xd0aed661u32 as i32));
        assert!(!super::is_waiting(0x490cc8e1));
        assert!(super::is_waiting(c6_core::skip_turn::HOLD));
        assert_eq!(super::increment_turns(0), 1);
        assert_eq!(super::increment_turns(i32::MAX), 0);
        assert_eq!(super::increment_turns(-4), 0);
    }
}
