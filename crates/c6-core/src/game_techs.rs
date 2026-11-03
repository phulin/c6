//! Source-bounded Linux XP2 Game.Techs v3 transport and Read/Validate projection.
//! Callers must establish module/rules/configuration identity and locate the owner.
//! A nonempty projection follows the post-load branch that skips BuildTechs;
//! empty nodes or iteration require the unsupported native BuildTechs path.
//! No native differential, complete gameplay, action or turn parity is claimed.
use crate::movement_world::DefinitionLookup;
use crate::unit::Limits;

pub const VERSION: u32 = 3;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version(u32),
    InvalidBool(u8),
    DefinitionMapping,
    BuildTechsRequired,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux Game.Techs: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Wire enum values are definition hashes, not native indexes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireNode {
    pub type_hash: u32,
    pub active: bool,
    /// Original FixedPointT<8> raw signed value, with no host float conversion.
    pub cost_raw: i32,
    pub prerequisite_hashes: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Techs {
    pub nodes: Vec<WireNode>,
    /// Independent ordered TechTypes vector at owner +0xf0.
    pub iteration_hashes: Vec<u32>,
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
        let value = self.bytes.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn version(&mut self) -> Result<(), Error> {
        let value = self.u32()?;
        if value != VERSION {
            return Err(Error::Version(value));
        }
        Ok(())
    }
    fn count<T>(&mut self, minimum: usize) -> Result<Vec<T>, Error> {
        let n = self.u32()? as usize;
        self.items = self.items.checked_add(n).ok_or(Error::ResourceLimit)?;
        let storage = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?;
        let wire = n
            .checked_mul(minimum)
            .and_then(|n| n.checked_add(self.pos))
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items
            || storage > isize::MAX as usize
            || wire > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        if wire > self.bytes.len() {
            return Err(Error::Truncated);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(n)
            .map_err(|_| Error::ResourceLimit)?;
        // Capacity may exceed n; the count is retained separately by the caller.
        Ok(values)
    }
    fn hashes(&mut self) -> Result<Vec<u32>, Error> {
        let before = self.items;
        let mut values = self.count::<u32>(4)?;
        for _ in before..self.items {
            values.push(self.u32()?);
        }
        Ok(values)
    }
}
impl Techs {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            bytes,
            pos: 0,
            items: 0,
            limits,
        };
        r.version()?;
        let mut nodes = r.count::<WireNode>(17)?;
        let count = r.items;
        for _ in 0..count {
            r.version()?;
            let type_hash = r.u32()?;
            let active = match r.raw(1)?[0] {
                0 => false,
                1 => true,
                b => return Err(Error::InvalidBool(b)),
            };
            let cost_raw = r.u32()? as i32;
            let prerequisite_hashes = r.hashes()?;
            nodes.push(WireNode {
                type_hash,
                active,
                cost_raw,
                prerequisite_hashes,
            });
        }
        let iteration_hashes = r.hashes()?;
        Ok((
            Self {
                nodes,
                iteration_hashes,
            },
            r.pos,
            r.items,
        ))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if used != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    /// Reproduce hash remapping followed by stable ValidateTechs filtering.
    /// The supplied registry must be the actual admitted native-order registry.
    /// This result is not evidence that native initialization has executed.
    pub fn project_read(
        &self,
        definitions: &dyn DefinitionLookup,
    ) -> Result<ReadProjection, Error> {
        if definitions.count() > i32::MAX as usize {
            return Err(Error::DefinitionMapping);
        }
        fn remap(hash: u32, d: &dyn DefinitionLookup) -> Result<i32, Error> {
            match d.find_index(hash) {
                Some(index) if index < d.count() => Ok(index as i32),
                Some(_) => Err(Error::DefinitionMapping),
                None => Ok(-1),
            }
        }
        fn valid(index: i32, d: &dyn DefinitionLookup) -> Result<bool, Error> {
            match d.find_index(index as u32) {
                Some(found) if found < d.count() => Ok(true),
                Some(_) => Err(Error::DefinitionMapping),
                None => Ok(false),
            }
        }
        fn mapped(hashes: &[u32], d: &dyn DefinitionLookup) -> Result<Vec<i32>, Error> {
            let mut out = Vec::new();
            out.try_reserve_exact(hashes.len())
                .map_err(|_| Error::ResourceLimit)?;
            for hash in hashes {
                let index = remap(*hash, d)?;
                if valid(index, d)? {
                    out.push(index);
                }
            }
            Ok(out)
        }
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(self.nodes.len())
            .map_err(|_| Error::ResourceLimit)?;
        for node in &self.nodes {
            let native_type = remap(node.type_hash, definitions)?;
            if valid(native_type, definitions)? {
                nodes.push(Node {
                    native_type,
                    active: node.active,
                    cost_raw: node.cost_raw,
                    prerequisites: mapped(&node.prerequisite_hashes, definitions)?,
                });
            }
        }
        Ok(ReadProjection {
            nodes,
            iteration: mapped(&self.iteration_hashes, definitions)?,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub native_type: i32,
    pub active: bool,
    pub cost_raw: i32,
    pub prerequisites: Vec<i32>,
}
/// Typed state after this owner's Read/Validate, before post-load initialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadProjection {
    nodes: Vec<Node>,
    iteration: Vec<i32>,
}
impl ReadProjection {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn iteration(&self) -> &[i32] {
        &self.iteration
    }
    /// Exact bounded InitializePostLoad branch: native BuildTechs is needed if
    /// either vector is empty. No SQL-derived substitute graph is generated.
    pub fn post_load_without_rebuild(self) -> Result<Graph, Error> {
        if self.nodes.is_empty() || self.iteration.is_empty() {
            return Err(Error::BuildTechsRequired);
        }
        Ok(Graph(self))
    }
}
/// Owner-local source projection, admitted only for the no-rebuild branch.
/// External binding/provenance and differential acceptance remain caller duties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Graph(ReadProjection);
impl Graph {
    pub fn nodes(&self) -> &[Node] {
        self.0.nodes()
    }
    pub fn iteration(&self) -> &[i32] {
        self.0.iteration()
    }
    pub fn node(&self, tech: i32) -> Option<&Node> {
        if tech == -1 {
            return None;
        }
        if let Ok(index) = usize::try_from(tech) {
            if let Some(node) = self.nodes().get(index) {
                if node.native_type == tech {
                    return Some(node);
                }
            }
        }
        self.nodes().iter().find(|node| node.native_type == tech)
    }
    pub fn has_tech(&self, tech: i32) -> bool {
        self.node(tech).is_some_and(|node| node.active)
    }
    /// The constructed owner has an empty +0x110 fallback vector; the
    /// projected Clear/Read/Validate/no-rebuild path never modifies that field.
    /// This does not import an arbitrary already-mutated live owner.
    pub fn prerequisites(&self, tech: i32) -> &[i32] {
        self.node(tech).map_or(&[], |node| &node.prerequisites)
    }
}
