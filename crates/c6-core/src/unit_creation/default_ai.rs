//! GlobalAi.DefaultUnitAi21bfc80 reads the initialized runtime role matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingRoleWord(usize),
}
/// `role_words` contains the first word of each native unit's DynamicBitField.
/// No initialized state is inferred from a seed, type name or saved Unit AI.
pub fn get(
    game_instanced: bool,
    global_ai_state: Option<i32>,
    role_words: &[u32],
    index: usize,
) -> Result<i32, Error> {
    if !game_instanced || global_ai_state != Some(2) {
        return Ok(-1);
    }
    let word = *role_words.get(index).ok_or(Error::MissingRoleWord(index))?;
    Ok((0..=3)
        .rev()
        .find(|&role| word & (1 << role) != 0)
        .unwrap_or(-1))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_initialized_roles_are_queried_highest_role_first() {
        assert_eq!(get(false, Some(2), &[], 99), Ok(-1));
        assert_eq!(get(true, None, &[], 99), Ok(-1));
        assert_eq!(get(true, Some(1), &[], 99), Ok(-1));
        assert_eq!(get(true, Some(2), &[0x81, 0x82, 0x20080, 15], 0), Ok(0));
        assert_eq!(get(true, Some(2), &[15], 0), Ok(3));
        assert_eq!(get(true, Some(2), &[0x20080], 0), Ok(-1));
        assert_eq!(get(true, Some(2), &[], 0), Err(Error::MissingRoleWord(0)));
    }
}
