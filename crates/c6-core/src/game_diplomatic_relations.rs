//! Owned Linux XP2 Game.DiplomaticRelations query state.
//!
//! Current v16 Read prefix only: no relation setters, callbacks, complete live
//! initialization or turn equivalence. Player.Diplomacy is a separate owner.
use crate::unit::Limits;

pub const VERSION: i32 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
    Version(i32),
    NoncanonicalBool(u8),
    MissingWar,
    MissingAlliances,
    DefinitionInputs,
    AlreadyInitialized,
    InvalidStorage,
    InvalidPlayer(i32),
    Index,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.DiplomaticRelations: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Complete ordered value vector and, when captured, its companion game turns.
/// First-word-only native metadata never invents the missing second words.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relationships {
    values: Vec<i32>,
    changed_turns: Option<Vec<i32>>,
}
impl Relationships {
    pub fn values(&self) -> &[i32] {
        &self.values
    }
    pub fn changed_turns(&self) -> Option<&[i32]> {
        self.changed_turns.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoolRelationship {
    pub value: bool,
    pub changed_turn: i32,
}

/// The current Read's preceding fields, identified by the original owner offset.
/// Offset labels preserve attribution without inventing gameplay meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadPrefixFields {
    pub at_c8: Vec<BoolRelationship>,
    pub at_f0: Vec<BoolRelationship>,
    pub at_118: Vec<BoolRelationship>,
    pub at_140: Vec<BoolRelationship>,
    pub at_168: Relationships,
    pub at_1b8: Relationships,
    pub at_1e0: Relationships,
    pub at_320: bool,
    /// Saved WarTypes hashes before native ValidateHash.
    pub saved_war: Relationships,
    /// Saved AllianceTypes hashes before native HashToIndex.
    pub saved_alliances: Relationships,
}

/// Concrete retained relation owner, independent of unrelated Unit revisions.
/// Absent vectors remain unknown. Binding to a particular Core/configuration
/// belongs to the containing LoadedState admission, not a global revision test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameDiplomaticRelations {
    player_count: i32,
    war: Option<Relationships>,
    alliances: Option<Relationships>,
    read_prefix: Option<ReadPrefixFields>,
}

pub struct DecodedPrefix<'a> {
    pub state: GameDiplomaticRelations,
    /// Relative to the attributed Game.DiplomaticRelations serializer start.
    pub consumed: usize,
    pub items_used: usize,
    /// Unclassified remaining serializer bytes, borrowed and never masked.
    pub tail: &'a [u8],
}

fn total(n: i32) -> Result<usize, Error> {
    let value = n.wrapping_sub(1).wrapping_mul(n) >> 1;
    if n < 0 || value < 0 {
        return Err(Error::InvalidStorage);
    }
    Ok(value as usize)
}

impl GameDiplomaticRelations {
    /// Admit complete first words from actual owner metadata. This does not
    /// initialize absent alliances or changed turns, or assert snapshot identity.
    pub fn from_war_values(player_count: i32, values: Vec<i32>) -> Result<Self, Error> {
        if values.len() != total(player_count)? {
            return Err(Error::InvalidStorage);
        }
        Ok(Self {
            player_count,
            war: Some(Relationships {
                values,
                changed_turns: None,
            }),
            alliances: None,
            read_prefix: None,
        })
    }
    /// Apply the original Read enum initialization using complete, admitted
    /// native definition inputs. WarTypes validates hashes; AllianceTypes maps
    /// a found hash to its actual GetCollectionIndex. Both use -1 on misses.
    /// This one-shot load step is not a gameplay relation setter.
    pub fn initialize_definitions(
        &mut self,
        war_hashes: &[u32],
        alliances: &[(u32, i32)],
    ) -> Result<(), Error> {
        if self.war.is_some() || self.alliances.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if war_hashes
            .iter()
            .enumerate()
            .any(|(i, h)| war_hashes[..i].contains(h))
            || alliances.iter().enumerate().any(|(i, (h, n))| {
                *n < 0
                    || alliances[..i]
                        .iter()
                        .any(|(other, index)| h == other || n == index)
            })
        {
            return Err(Error::DefinitionInputs);
        }
        let prefix = self.read_prefix.as_ref().ok_or(Error::DefinitionInputs)?;
        let mut war = prefix.saved_war.clone();
        for value in &mut war.values {
            if !war_hashes.contains(&(*value as u32)) {
                *value = -1;
            }
        }
        let mut alliance_values = prefix.saved_alliances.clone();
        for value in &mut alliance_values.values {
            *value = alliances
                .iter()
                .find(|(hash, _)| *hash == *value as u32)
                .map_or(-1, |(_, index)| *index);
        }
        self.war = Some(war);
        self.alliances = Some(alliance_values);
        Ok(())
    }
    pub fn player_count(&self) -> i32 {
        self.player_count
    }
    pub fn war(&self) -> Option<&Relationships> {
        self.war.as_ref()
    }
    pub fn alliances(&self) -> Option<&Relationships> {
        self.alliances.as_ref()
    }
    pub fn read_prefix(&self) -> Option<&ReadPrefixFields> {
        self.read_prefix.as_ref()
    }
    fn index(&self, a: i32, b: i32) -> Result<usize, Error> {
        for p in [a, b] {
            if p < 0 || p >= self.player_count {
                return Err(Error::InvalidPlayer(p));
            }
        }
        let n = self.player_count;
        let count = n.wrapping_sub(1).wrapping_mul(n) >> 1;
        let high = a.max(b);
        let triangle = high.wrapping_add(1).wrapping_mul(high) >> 1;
        usize::try_from(a.min(b).wrapping_sub(triangle).wrapping_add(count))
            .map_err(|_| Error::Index)
    }
    fn value(&self, values: &Relationships, a: i32, b: i32) -> Result<i32, Error> {
        if values.values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        values
            .values
            .get(self.index(a, b)?)
            .copied()
            .ok_or(Error::Index)
    }
    /// Native GetWarType and GetAllianceType return -1 for equal players before
    /// reading storage, even if that equal integer is not a valid player slot.
    pub fn war_type(&self, a: i32, b: i32) -> Result<i32, Error> {
        if a == b {
            return Ok(-1);
        }
        self.value(self.war.as_ref().ok_or(Error::MissingWar)?, a, b)
    }
    pub fn alliance_type(&self, a: i32, b: i32) -> Result<i32, Error> {
        if a == b {
            return Ok(-1);
        }
        self.value(
            self.alliances.as_ref().ok_or(Error::MissingAlliances)?,
            a,
            b,
        )
    }
    pub fn is_at_war(&self, a: i32, b: i32) -> Result<bool, Error> {
        Ok(self.war_type(a, b)? != -1)
    }
    pub fn is_allied(&self, a: i32, b: i32) -> Result<bool, Error> {
        Ok(self.alliance_type(a, b)? != -1)
    }
    /// Read the pinned current serializer in its original order, stopping just
    /// after the first AllianceTypes vector. Old-version initialization/remapping
    /// branches reject explicitly. No native callbacks are executed here.
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<DecodedPrefix<'_>, Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let version = r.i32()?;
        if version != VERSION {
            return Err(Error::Version(version));
        }
        let player_count = r.i32()?;
        let at_c8 = r.bools()?;
        let at_f0 = r.bools()?;
        let at_118 = r.bools()?;
        let at_140 = r.bools()?;
        let at_168 = r.relationships()?;
        let war = r.relationships()?;
        let at_1b8 = r.relationships()?;
        let at_1e0 = r.relationships()?;
        let at_320 = r.boolean()?;
        let alliances = r.relationships()?;
        let count = total(player_count)?;
        if war.values.len() != count || alliances.values.len() != count {
            return Err(Error::InvalidStorage);
        }
        Ok(DecodedPrefix {
            state: Self {
                player_count,
                war: None,
                alliances: None,
                read_prefix: Some(ReadPrefixFields {
                    at_c8,
                    at_f0,
                    at_118,
                    at_140,
                    at_168,
                    at_1b8,
                    at_1e0,
                    at_320,
                    saved_war: war,
                    saved_alliances: alliances,
                }),
            },
            consumed: r.pos,
            items_used: r.items,
            tail: &bytes[r.pos..],
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::NoncanonicalBool(x)),
        }
    }
    fn count(&mut self, width: usize) -> Result<usize, Error> {
        let count = self.i32()? as u32 as usize;
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        let bytes = count.checked_mul(width).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items
            || bytes > self.limits.max_bytes.saturating_sub(self.pos)
        {
            return Err(Error::ResourceLimit);
        }
        if bytes > self.bytes.len().saturating_sub(self.pos) {
            return Err(Error::Truncated);
        }
        Ok(count)
    }
    fn bools(&mut self) -> Result<Vec<BoolRelationship>, Error> {
        let count = self.count(5)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(BoolRelationship {
                value: self.boolean()?,
                changed_turn: self.i32()?,
            });
        }
        Ok(values)
    }
    fn relationships(&mut self) -> Result<Relationships, Error> {
        let count = self.count(8)?;
        let mut values = Vec::new();
        let mut turns = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        turns
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(self.i32()?);
            turns.push(self.i32()?);
        }
        Ok(Relationships {
            values,
            changed_turns: Some(turns),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        let put = |bytes: &mut Vec<u8>, x: i32| bytes.extend(x.to_le_bytes());
        put(&mut bytes, VERSION);
        put(&mut bytes, 3);
        for _ in 0..4 {
            put(&mut bytes, 0);
        }
        for values in [&[][..], &[-1, -2, 0][..], &[][..], &[][..]] {
            put(&mut bytes, values.len() as i32);
            for &v in values {
                put(&mut bytes, v);
                put(&mut bytes, i32::MIN);
            }
        }
        bytes.push(1);
        put(&mut bytes, 3);
        for v in [100, -1, i32::MIN] {
            put(&mut bytes, v);
            put(&mut bytes, i32::MAX);
        }
        bytes.extend([17, 23]);
        bytes
    }
    #[test]
    fn native_pair_order_sentinel_and_retained_turns() {
        let bytes = fixture();
        let decoded = GameDiplomaticRelations::decode_prefix(&bytes, Limits::default()).unwrap();
        assert_eq!(decoded.tail, [17, 23]);
        let mut s = decoded.state;
        assert_eq!(s.is_allied(0, 1), Err(Error::MissingAlliances));
        s.initialize_definitions(&[(-2i32) as u32, 0], &[(100, 7), (i32::MIN as u32, 3)])
            .unwrap();
        // Native reverse triangular order is (0,2), (1,2), (0,1).
        assert_eq!(s.alliance_type(0, 2), Ok(7));
        assert_eq!(s.alliance_type(2, 1), Ok(-1));
        assert_eq!(s.alliance_type(1, 0), Ok(3));
        assert_eq!(s.is_at_war(1, 2), Ok(true));
        assert_eq!(s.is_at_war(0, 2), Ok(false));
        assert_eq!(s.is_allied(-1, -1), Ok(false));
        assert_eq!(
            s.alliances().unwrap().changed_turns(),
            Some(&[i32::MAX; 3][..])
        );
        assert_eq!(s.alliance_type(-1, 0), Err(Error::InvalidPlayer(-1)));
    }
    #[test]
    fn definition_misses_and_duplicate_input_reject_without_partial_authority() {
        let bytes = fixture();
        let mut state = GameDiplomaticRelations::decode_prefix(&bytes, Limits::default())
            .unwrap()
            .state;
        assert_eq!(
            state.initialize_definitions(&[0, 0], &[]),
            Err(Error::DefinitionInputs)
        );
        assert!(state.war().is_none());
        assert!(state.alliances().is_none());
        state.initialize_definitions(&[], &[]).unwrap();
        assert_eq!(state.war_type(0, 1), Ok(-1));
        assert_eq!(state.alliance_type(0, 2), Ok(-1));
        assert_eq!(
            state.initialize_definitions(&[], &[]),
            Err(Error::AlreadyInitialized)
        );
        assert_eq!(
            state.read_prefix().unwrap().saved_alliances.values(),
            &[100, -1, i32::MIN]
        );
    }
    #[test]
    fn missing_alliance_never_becomes_peace_and_limits_precede_allocation() {
        let s = GameDiplomaticRelations::from_war_values(3, vec![-1; 3]).unwrap();
        assert_eq!(s.is_allied(0, 1), Err(Error::MissingAlliances));
        assert_eq!(s.is_allied(1, 1), Ok(false));
        assert_eq!(s.war().unwrap().changed_turns(), None);
        let bytes = fixture();
        for end in 0..bytes.len() - 2 {
            assert!(
                GameDiplomaticRelations::decode_prefix(&bytes[..end], Limits::default()).is_err()
            );
        }
        assert!(matches!(
            GameDiplomaticRelations::decode_prefix(
                &bytes,
                Limits {
                    max_bytes: bytes.len(),
                    max_items: 5
                }
            ),
            Err(Error::ResourceLimit)
        ));
    }
}
