//! Owned Linux XP2 Game.DiplomaticRelations query state.
//!
//! V16 Read prefix and explicitly supplied current relation matrices. Missing
//! matrices remain unknown; setters do not execute their required callbacks.
//! Player.Diplomacy is a separate owner; no complete turn equivalence is implied.
use crate::unit::Limits;

pub mod grievances;

pub const VERSION: i32 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    ResourceLimit,
    Version(i32),
    NoncanonicalBool(u8),
    MissingWar,
    MissingAlliances,
    MissingPermanentAlliances,
    MissingDeclaredFriendships,
    MissingResearchAgreements,
    MissingHasMet,
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
    current_research_agreements: Option<Relationships>,
    current_contacts: Option<Vec<BoolRelationship>>,
    current_friendships: Option<Vec<BoolRelationship>>,
    current_permanent_alliances: Option<Vec<BoolRelationship>>,
    current_grievances: Option<(Relationships, Relationships)>,
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

/// The native sorted hash vector inserts at lower_bound, so equal hashes
/// resolve to the last insertion. Reverse search over native insertion order
/// gives that same result without a second allocation. Ordinals take priority.
fn native_definition_index<T>(values: &[T], key: u32, hash: impl Fn(&T) -> u32) -> Option<usize> {
    if (key as usize) < values.len() {
        Some(key as usize)
    } else {
        values.iter().rposition(|value| hash(value) == key)
    }
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
            current_research_agreements: None,
            current_contacts: None,
            current_friendships: None,
            current_permanent_alliances: None,
            current_grievances: None,
        })
    }
    /// Complete current native enum values, with each companion changed turn.
    /// This admits only the two supplied matrices. Other relation queries stay
    /// unavailable; this does not construct a default diplomatic world.
    pub fn from_current_research_relations(
        player_count: i32,
        alliances: Vec<(i32, i32)>,
        research_agreements: Vec<(i32, i32)>,
    ) -> Result<Self, Error> {
        let count = total(player_count)?;
        if alliances.len() != count || research_agreements.len() != count {
            return Err(Error::InvalidStorage);
        }
        fn matrix(pairs: Vec<(i32, i32)>) -> Relationships {
            let (values, turns) = pairs.into_iter().unzip();
            Relationships {
                values,
                changed_turns: Some(turns),
            }
        }
        Ok(Self {
            player_count,
            war: None,
            alliances: Some(matrix(alliances)),
            read_prefix: None,
            current_research_agreements: Some(matrix(research_agreements)),
            current_contacts: None,
            current_friendships: None,
            current_permanent_alliances: None,
            current_grievances: None,
        })
    }
    /// Apply the original Read enum initialization using complete, admitted
    /// native-order definition inputs. Find first treats keys below collection
    /// count as ordinal indexes; other keys resolve the last inserted matching
    /// unsigned hash. WarTypes returns the found definition's type hash;
    /// AllianceTypes returns its actual GetCollectionIndex. Misses return -1.
    /// This one-shot load step is not a gameplay relation setter.
    pub fn initialize_definitions(
        &mut self,
        war_hashes: &[u32],
        alliances: &[(u32, i32)],
    ) -> Result<(), Error> {
        if self.war.is_some() || self.alliances.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if u32::try_from(war_hashes.len()).is_err()
            || u32::try_from(alliances.len()).is_err()
            || alliances.iter().any(|(_, index)| *index < 0)
        {
            return Err(Error::DefinitionInputs);
        }
        let prefix = self.read_prefix.as_ref().ok_or(Error::DefinitionInputs)?;
        let mut war = prefix.saved_war.clone();
        for value in &mut war.values {
            *value = native_definition_index(war_hashes, *value as u32, |hash| *hash)
                .map_or(-1, |index| war_hashes[index] as i32);
        }
        let mut alliance_values = prefix.saved_alliances.clone();
        for value in &mut alliance_values.values {
            *value = native_definition_index(alliances, *value as u32, |(hash, _)| *hash)
                .map_or(-1, |index| alliances[index].1);
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
    /// Attach actual current contact values without inventing war/alliance or
    /// unrelated saved-prefix fields. Existing initialized contacts must agree.
    pub fn initialize_current_contacts(
        &mut self,
        player_count: i32,
        values: Vec<BoolRelationship>,
    ) -> Result<(), Error> {
        if player_count != self.player_count || values.len() != total(player_count)? {
            return Err(Error::InvalidStorage);
        }
        if let Some(existing) = self
            .current_contacts
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_c8))
        {
            return if existing == &values {
                Ok(())
            } else {
                Err(Error::AlreadyInitialized)
            };
        }
        self.current_contacts = Some(values);
        Ok(())
    }
    pub fn from_current_contacts(
        player_count: i32,
        values: Vec<BoolRelationship>,
    ) -> Result<Self, Error> {
        let mut value = Self {
            player_count,
            war: None,
            alliances: None,
            read_prefix: None,
            current_research_agreements: None,
            current_contacts: None,
            current_friendships: None,
            current_permanent_alliances: None,
            current_grievances: None,
        };
        value.initialize_current_contacts(player_count, values)?;
        Ok(value)
    }
    /// Native Game.DiplomaticRelations.HasMet: equal arguments return true
    /// before storage checks. Player.Diplomacy adds its own validity/self gate.
    pub fn has_met(&self, a: i32, b: i32) -> Result<bool, Error> {
        if a == b {
            return Ok(true);
        }
        let values = self
            .current_contacts
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_c8))
            .ok_or(Error::MissingHasMet)?;
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        values
            .get(self.index(a, b)?)
            .map(|cell| cell.value)
            .ok_or(Error::Index)
    }
    /// The actual symmetric contact payload write, including repeated turn
    /// assignment for distinct players. Equal-player writes are unsupported.
    /// The caller must then execute LogDiplomaticEvent in order;
    /// this storage operation does not pretend to deliver the native logger.
    pub fn set_has_met(&mut self, a: i32, b: i32, turn: i32) -> Result<(), Error> {
        if a == b {
            return Err(Error::Index);
        }
        let index = self.index(a, b)?;
        let count = total(self.player_count)?;
        let values = match &mut self.current_contacts {
            Some(values) => values,
            None => &mut self.read_prefix.as_mut().ok_or(Error::MissingHasMet)?.at_c8,
        };
        if values.len() != count {
            return Err(Error::InvalidStorage);
        }
        let cell = values.get_mut(index).ok_or(Error::Index)?;
        cell.value = true;
        cell.changed_turn = turn;
        Ok(())
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
    /// Native HasDeclaredFriendship uses the distinct symmetric bool vector.
    /// Equal identities return false before consulting the current storage.
    /// Current native friendship cells, independent of an optional saved Read
    /// prefix. Native values are already bools; no absent cell is inferred.
    pub fn initialize_current_friendships(
        &mut self,
        values: Vec<BoolRelationship>,
    ) -> Result<(), Error> {
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        if let Some(old) = self
            .current_friendships
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_f0))
        {
            if old != &values {
                return Err(Error::AlreadyInitialized);
            }
        }
        self.current_friendships = Some(values);
        Ok(())
    }
    /// Actual native permanent-alliance cells, retaining their changed turns.
    pub fn initialize_current_permanent_alliances(
        &mut self,
        values: Vec<BoolRelationship>,
    ) -> Result<(), Error> {
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        if let Some(old) = self
            .current_permanent_alliances
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_118))
        {
            if old != &values {
                return Err(Error::AlreadyInitialized);
            }
        }
        self.current_permanent_alliances = Some(values);
        Ok(())
    }
    /// Native current WarTypes indexes and changed turns; saved hashes require
    /// initialize_definitions instead. Preserve current storage on conflict.
    pub fn initialize_current_war(&mut self, values: Vec<(i32, i32)>) -> Result<(), Error> {
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        let rows = Relationships {
            values: values.iter().map(|p| p.0).collect(),
            changed_turns: Some(values.iter().map(|p| p.1).collect()),
        };
        if let Some(old) = &self.war {
            if old != &rows {
                return Err(Error::AlreadyInitialized);
            }
        }
        self.war = Some(rows);
        Ok(())
    }
    pub fn has_declared_friendship(&self, a: i32, b: i32) -> Result<bool, Error> {
        if a == b {
            return Ok(false);
        }
        let values = self
            .current_friendships
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_f0))
            .ok_or(Error::MissingDeclaredFriendships)?;
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        values
            .get(self.index(a, b)?)
            .map(|value| value.value)
            .ok_or(Error::Index)
    }
    /// The canonical Read prefix retains TechTypes hashes. Project the same
    /// native Read HashToIndex mapping using the current admitted collection.
    /// Equal identities return -1 before any storage/definition lookup.
    pub fn research_agreement_tech(
        &self,
        a: i32,
        b: i32,
        technology_hashes: &[u32],
    ) -> Result<i32, Error> {
        if a == b {
            return Ok(-1);
        }
        if let Some(current) = &self.current_research_agreements {
            return self.value(current, a, b);
        }
        let values = &self
            .read_prefix
            .as_ref()
            .ok_or(Error::MissingResearchAgreements)?
            .at_168;
        let hash = self.value(values, a, b)? as u32;
        Ok(
            native_definition_index(technology_hashes, hash, |value| *value)
                .map_or(-1, |index| index as i32),
        )
    }
    /// Current permanent-alliance bool relationship. Equal players return false
    /// before storage/range checks. Missing prefix/storage is not a false value.
    pub fn is_permanently_allied(&self, a: i32, b: i32) -> Result<bool, Error> {
        if a == b {
            return Ok(false);
        }
        let values = self
            .current_permanent_alliances
            .as_ref()
            .or_else(|| self.read_prefix.as_ref().map(|p| &p.at_118))
            .ok_or(Error::MissingPermanentAlliances)?;
        if values.len() != total(self.player_count)? {
            return Err(Error::InvalidStorage);
        }
        values
            .get(self.index(a, b)?)
            .map(|value| value.value)
            .ok_or(Error::Index)
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
                current_research_agreements: None,
                current_contacts: None,
                current_friendships: None,
                current_permanent_alliances: None,
                current_grievances: None,
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
mod current_contact_tests {
    use super::*;
    #[test]
    fn actual_contacts_keep_unknown_relations_and_preserve_contact_turn_writes() {
        let cells = vec![
            BoolRelationship {
                value: false,
                changed_turn: -1
            };
            3
        ];
        let mut owner = GameDiplomaticRelations::from_current_contacts(3, cells.clone()).unwrap();
        assert!(!owner.has_met(0, 2).unwrap());
        assert_eq!(owner.war_type(0, 2), Err(Error::MissingWar));
        assert_eq!(owner.alliance_type(0, 2), Err(Error::MissingAlliances));
        owner.set_has_met(2, 0, 7).unwrap();
        assert!(owner.has_met(0, 2).unwrap());
        let index = owner.index(0, 2).unwrap();
        assert_eq!(
            owner.current_contacts.as_ref().unwrap()[index].changed_turn,
            7
        );
        assert_eq!(
            owner.initialize_current_contacts(3, cells),
            Err(Error::AlreadyInitialized)
        );
        assert!(owner.has_met(-1, -1).unwrap());
    }
    #[test]
    fn contact_refinement_preserves_other_current_relation_matrices() {
        let mut owner = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, 4)],
            vec![(2, 5)],
        )
        .unwrap();
        owner
            .initialize_current_contacts(
                2,
                vec![BoolRelationship {
                    value: true,
                    changed_turn: 3,
                }],
            )
            .unwrap();
        assert!(owner.has_met(0, 1).unwrap());
        assert_eq!(owner.alliance_type(0, 1), Ok(-1));
        assert_eq!(
            owner.initialize_current_contacts(3, vec![]),
            Err(Error::InvalidStorage)
        );
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
    fn permanent_alliance_uses_own_bool_storage_and_symmetric_order() {
        let mut state = GameDiplomaticRelations::decode_prefix(&fixture(), Limits::default())
            .unwrap()
            .state;
        assert_eq!(state.is_permanently_allied(-1, -1), Ok(false));
        assert_eq!(
            state.is_permanently_allied(0, 1),
            Err(Error::InvalidStorage)
        );
        state.read_prefix.as_mut().unwrap().at_118 = vec![
            BoolRelationship {
                value: true,
                changed_turn: 91,
            },
            BoolRelationship {
                value: false,
                changed_turn: -7,
            },
            BoolRelationship {
                value: true,
                changed_turn: 17,
            },
        ];
        assert_eq!(state.is_permanently_allied(0, 2), Ok(true));
        assert_eq!(state.is_permanently_allied(2, 0), Ok(true));
        assert_eq!(state.is_permanently_allied(2, 1), Ok(false));
        assert_eq!(state.is_permanently_allied(0, 1), Ok(true));
        assert_eq!(
            state.is_permanently_allied(3, 0),
            Err(Error::InvalidPlayer(3))
        );
        assert_eq!(
            state.is_permanently_allied(-1, 0),
            Err(Error::InvalidPlayer(-1))
        );
        assert_eq!(state.read_prefix().unwrap().at_118[0].changed_turn, 91);
        let missing = GameDiplomaticRelations::from_war_values(3, vec![-1; 3]).unwrap();
        assert_eq!(
            missing.is_permanently_allied(0, 1),
            Err(Error::MissingPermanentAlliances)
        );
        assert_eq!(missing.is_permanently_allied(99, 99), Ok(false));
    }
    #[test]
    fn declared_friendship_has_distinct_symmetric_storage_and_unknown_is_error() {
        let mut state = GameDiplomaticRelations::decode_prefix(&fixture(), Limits::default())
            .unwrap()
            .state;
        state.read_prefix.as_mut().unwrap().at_f0 = vec![
            BoolRelationship {
                value: true,
                changed_turn: 1,
            },
            BoolRelationship {
                value: false,
                changed_turn: 2,
            },
            BoolRelationship {
                value: false,
                changed_turn: 3,
            },
        ];
        assert_eq!(state.has_declared_friendship(0, 2), Ok(true));
        assert_eq!(state.has_declared_friendship(2, 0), Ok(true));
        assert_eq!(state.has_declared_friendship(1, 2), Ok(false));
        state.read_prefix = None;
        assert_eq!(state.has_declared_friendship(-1, -1), Ok(false));
        assert_eq!(
            state.has_declared_friendship(0, 2),
            Err(Error::MissingDeclaredFriendships)
        );
    }
    #[test]
    fn research_agreement_remaps_saved_hashes_and_keeps_native_equal_sentinel() {
        let mut state = GameDiplomaticRelations::decode_prefix(&fixture(), Limits::default())
            .unwrap()
            .state;
        state.read_prefix.as_mut().unwrap().at_168.values = vec![0x80002000u32 as i32, 1, -1];
        let definitions = [0x80001000, 0x80002000];
        assert_eq!(state.research_agreement_tech(0, 2, &definitions), Ok(1));
        assert_eq!(state.research_agreement_tech(2, 0, &definitions), Ok(1));
        assert_eq!(state.research_agreement_tech(1, 2, &definitions), Ok(1));
        assert_eq!(state.research_agreement_tech(0, 1, &definitions), Ok(-1));
        state.read_prefix = None;
        assert_eq!(state.research_agreement_tech(-1, -1, &[]), Ok(-1));
        assert_eq!(
            state.research_agreement_tech(0, 1, &definitions),
            Err(Error::MissingResearchAgreements)
        );
    }
    #[test]
    fn definition_misses_and_invalid_input_reject_without_partial_authority() {
        let bytes = fixture();
        let mut state = GameDiplomaticRelations::decode_prefix(&bytes, Limits::default())
            .unwrap()
            .state;
        assert_eq!(
            state.initialize_definitions(&[0, 0], &[(100, -1)]),
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
    fn native_find_prioritizes_ordinal_then_last_duplicate_hash() {
        let bytes = fixture();
        let mut state = GameDiplomaticRelations::decode_prefix(&bytes, Limits::default())
            .unwrap()
            .state;
        let saved = state.read_prefix.as_mut().unwrap();
        saved.saved_war.values = vec![0, 77, 99];
        saved.saved_alliances.values = vec![0, 77, 99];
        let war = [0x7000_0001, 0, 77, 77];
        let alliances = [(88, 8), (0, 6), (77, 7), (77, 12)];
        state.initialize_definitions(&war, &alliances).unwrap();
        assert_eq!(state.war().unwrap().values(), &[0x7000_0001, 77, -1]);
        assert_eq!(state.alliances().unwrap().values(), &[8, 12, -1]);
        // This also checks equal hashes remain distinct native definitions.
        assert_eq!(native_definition_index(&war, 77, |h| *h), Some(3));
        assert_eq!(native_definition_index(&war, 4, |h| *h), None);
        assert_eq!(
            native_definition_index(&[u32::MAX, 0x8000_0000], u32::MAX, |h| *h),
            Some(0)
        );
        assert_eq!(
            state.read_prefix().unwrap().saved_war.values(),
            &[0, 77, 99]
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

#[cfg(test)]
mod current_research_tests {
    use super::*;
    #[test]
    fn current_native_matrices_keep_turns_and_do_not_remap_indices() {
        let state = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(4, 19)],
            vec![(7, 23)],
        )
        .unwrap();
        assert_eq!(state.alliance_type(0, 1), Ok(4));
        assert_eq!(
            state.alliances().unwrap().changed_turns(),
            Some([19].as_slice())
        );
        assert_eq!(state.research_agreement_tech(1, 0, &[]), Ok(7));
        assert_eq!(state.war_type(0, 1), Err(Error::MissingWar));
        assert!(
            GameDiplomaticRelations::from_current_research_relations(2, vec![], vec![(0, 0)])
                .is_err()
        );
    }
}

#[cfg(test)]
mod current_friendship_tests {
    use super::*;
    #[test]
    fn current_native_vectors_remain_distinct_from_missing_saved_prefix() {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, -1)],
            vec![(-1, -1)],
        )
        .unwrap();
        assert_eq!(
            r.has_declared_friendship(0, 1),
            Err(Error::MissingDeclaredFriendships)
        );
        r.initialize_current_friendships(vec![BoolRelationship {
            value: true,
            changed_turn: 7,
        }])
        .unwrap();
        r.initialize_current_war(vec![(3, 5)]).unwrap();
        assert_eq!(
            r.is_permanently_allied(0, 1),
            Err(Error::MissingPermanentAlliances)
        );
        r.initialize_current_permanent_alliances(vec![BoolRelationship {
            value: false,
            changed_turn: -1,
        }])
        .unwrap();
        assert_eq!(r.is_permanently_allied(0, 1), Ok(false));
        assert_eq!(r.has_declared_friendship(0, 1), Ok(true));
        assert_eq!(r.has_declared_friendship(0, 0), Ok(false));
        assert_eq!(r.war_type(1, 0), Ok(3));
        assert_eq!(r.war().unwrap().changed_turns(), Some(&[5][..]));
        assert!(r.read_prefix().is_none());
    }
    #[test]
    fn conflicting_current_vector_does_not_replace_prior_cells() {
        let mut r = GameDiplomaticRelations::from_current_research_relations(
            2,
            vec![(-1, -1)],
            vec![(-1, -1)],
        )
        .unwrap();
        r.initialize_current_friendships(vec![BoolRelationship {
            value: false,
            changed_turn: -1,
        }])
        .unwrap();
        assert_eq!(
            r.initialize_current_friendships(vec![BoolRelationship {
                value: true,
                changed_turn: 3
            }]),
            Err(Error::AlreadyInitialized)
        );
        assert_eq!(r.has_declared_friendship(0, 1), Ok(false));
        assert_eq!(r.initialize_current_war(vec![]), Err(Error::InvalidStorage));
        assert_eq!(r.war_type(0, 1), Err(Error::MissingWar));
    }
}
