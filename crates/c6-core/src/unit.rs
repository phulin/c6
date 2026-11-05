//! Bounded Linux XP2 Unit/Formation wire records, not a Core Save loader or gameplay.
//!
//! Current operation queues support eleven verified handlers and their exact
//! parameter formats. Wire operation hashes are distinct from live numeric IDs.
//! Unit17/28 wire/default profiles retain independent nested-codec gates.
//! Legacy omitted runtime effects and missing prior scalars remain explicit.
//! Formation version 1 is decoded completely as serialized wire state.
//! See `docs/linux-unit-archive.md`; unsupported handlers cannot be skipped because
//! their bodies have no common length. No action transition is implemented here.

mod operation_nodes;
mod state;
pub mod variants;
pub use operation_nodes::{
    OperationNodeId, OperationNodes, OperationStorageError, OperationStorageHandle,
};
pub use state::*;

/// The original registered SkipTurn operation hash, never a numeric handler ID.
pub const SKIP_TURN_HASH: u32 = 0x2c681908;
pub const FOUND_CITY_HASH: u32 = 0x5733ac6c;
pub const MOVE_TO_HASH: u32 = 0x580f2f68;
pub const RANGE_ATTACK_HASH: u32 = 0x09d0292a;
pub const BUILD_IMPROVEMENT_HASH: u32 = 0x1d60e778;
pub const PILLAGE_HASH: u32 = 0x4885d724;
pub const MAKE_TRADE_ROUTE_HASH: u32 = 0x8374d954;
pub const ALERT_HASH: u32 = 0xe547bc17;
pub const FORTIFY_HASH: u32 = 0xd3a6f7e8;
pub const SPY_SIPHON_FUNDS_HASH: u32 = 0x223eea54;
pub const SPY_DISRUPT_ROCKETRY_HASH: u32 = 0xc31647e8;
pub const SPY_GAIN_SOURCES_HASH: u32 = 0xed72b77d;
pub const SPY_SABOTAGE_PRODUCTION_HASH: u32 = 0x09c1ecc1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationParameters {
    None,
    /// Native SharedParameters::SingleLocation, also used by RangeAttack.
    MoveTo {
        location: PlotCoord,
        modifiers: u32,
    },
    /// Linux base and XP2 BuildImprovement version-one parameters.
    BuildImprovement {
        modifiers: u32,
        improvement_hash: u32,
    },
    /// Linux Pillage ClassParameters version one.
    Pillage { modifiers: u32, improvement_hash: u32 },
    MakeTradeRoute {
        modifiers: u32,
        location0: PlotCoord,
        location1: PlotCoord,
        resource_hash: u32,
    },
}
fn parameter_kind(hash: u32) -> Result<u8, Error> {
    match hash {
        SKIP_TURN_HASH
        | FOUND_CITY_HASH
        | ALERT_HASH
        | FORTIFY_HASH
        | SPY_SIPHON_FUNDS_HASH
        | SPY_DISRUPT_ROCKETRY_HASH
        | SPY_GAIN_SOURCES_HASH
        | SPY_SABOTAGE_PRODUCTION_HASH => Ok(0),
        MOVE_TO_HASH | RANGE_ATTACK_HASH => Ok(1),
        MAKE_TRADE_ROUTE_HASH => Ok(2),
        BUILD_IMPROVEMENT_HASH => Ok(3),
        PILLAGE_HASH => Ok(4),
        _ => Err(Error::UnsupportedHandler(hash)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationNode {
    pub operation_hash: u32,
    pub directive: i32,
    pub flags: i32,
    pub push_turn: i32,
    pub previous_activity: i32,
    pub previous_automation: i32,
    pub parameters: OperationParameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationQueue {
    /// Native queue order, without sorting or handler-ID conversion.
    pub nodes: OperationNodes,
}

/// Codec resource policy; these are not native gameplay limits.
/// Items aggregate dynamic elements and string bytes; typed variants also have
/// a fixed maximum nesting depth of 64. Byte limits apply to the whole record.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_items: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_items: 1_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedLegacyWrite,
    MissingRetainedValue(&'static str),
    Truncated,
    TrailingBytes,
    ResourceLimit,
    Version { expected: i32, actual: i32 },
    UnsupportedHandler(u32),
    UnsupportedParameters(i32),
    ParameterHandlerMismatch(u32),
    NoncanonicalBool(u8),
    Variant(variants::VariantError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

impl OperationQueue {
    /// Decode an entire component. Only current queue/node/parameter versions are supported.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, limits)?;
        let value = Self::read(&mut r)?;
        r.finish()?;
        Ok(value)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        r.version(1)?;
        let count = r.count(32)?;
        let mut nodes = Vec::with_capacity(count);
        for _ in 0..count {
            let operation_hash = r.u32()?;
            let kind = parameter_kind(operation_hash)?;
            r.version(2)?;
            let directive = r.i32()?;
            let flags = r.i32()?;
            let push_turn = r.i32()?;
            let previous_activity = r.i32()?;
            let previous_automation = r.i32()?;
            let parameters = match kind {
                0 => {
                    let version = r.i32()?;
                    if version != 0 {
                        return Err(Error::UnsupportedParameters(version));
                    }
                    OperationParameters::None
                }
                1 => {
                    r.version(1)?;
                    OperationParameters::MoveTo {
                        location: r.coord()?,
                        modifiers: r.u32()?,
                    }
                }
                2 => {
                    r.version(1)?;
                    OperationParameters::MakeTradeRoute {
                        modifiers: r.u32()?,
                        location0: r.coord()?,
                        location1: r.coord()?,
                        resource_hash: r.u32()?,
                    }
                }
                3 => {
                    r.version(1)?;
                    OperationParameters::BuildImprovement {
                        modifiers: r.u32()?,
                        improvement_hash: r.u32()?,
                    }
                }
                4 => {
                    r.version(1)?;
                    OperationParameters::Pillage { modifiers:r.u32()?, improvement_hash:r.u32()? }
                }
                _ => unreachable!(),
            };
            nodes.push(OperationNode {
                operation_hash,
                directive,
                flags,
                push_turn,
                previous_activity,
                previous_automation,
                parameters,
            });
        }
        Ok(Self {
            nodes: nodes.into(),
        })
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        if self.nodes.len() > limits.max_items || self.nodes.len() > u32::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        let mut length = 8usize;
        for node in &self.nodes {
            let kind = parameter_kind(node.operation_hash)?;
            let (actual, size) = match &node.parameters {
                OperationParameters::None => (0, 32),
                OperationParameters::MoveTo { .. } => (1, 44),
                OperationParameters::MakeTradeRoute { .. } => (2, 56),
                OperationParameters::BuildImprovement { .. } => (3, 40),
                OperationParameters::Pillage { .. } => (4, 40),
            };
            if actual != kind {
                return Err(Error::ParameterHandlerMismatch(node.operation_hash));
            }
            length = length.checked_add(size).ok_or(Error::ResourceLimit)?;
        }
        if length > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::with_capacity(length);
        word(&mut bytes, 1);
        word(&mut bytes, self.nodes.len() as u32);
        for node in &self.nodes {
            for v in [
                node.operation_hash,
                2,
                node.directive as u32,
                node.flags as u32,
                node.push_turn as u32,
                node.previous_activity as u32,
                node.previous_automation as u32,
            ] {
                word(&mut bytes, v);
            }
            match &node.parameters {
                OperationParameters::None => word(&mut bytes, 0),
                OperationParameters::MoveTo {
                    location,
                    modifiers,
                } => {
                    for v in [1, location.x as u32, location.y as u32, *modifiers] {
                        word(&mut bytes, v);
                    }
                }
                OperationParameters::BuildImprovement { modifiers, improvement_hash }
                | OperationParameters::Pillage { modifiers, improvement_hash } => {
                    for v in [1, *modifiers, *improvement_hash] {
                        word(&mut bytes, v);
                    }
                }
                OperationParameters::MakeTradeRoute {
                    modifiers,
                    location0,
                    location1,
                    resource_hash,
                } => {
                    for v in [
                        1,
                        *modifiers,
                        location0.x as u32,
                        location0.y as u32,
                        location1.x as u32,
                        location1.y as u32,
                        *resource_hash,
                    ] {
                        word(&mut bytes, v);
                    }
                }
            }
        }
        Ok(bytes)
    }
}

/// Serialized PlotCoord stores signed 32-bit coordinates, including -9999 invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlotCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathNode {
    /// Original FixedPointT<8> raw representation.
    pub data1_raw: i32,
    pub data2: i32,
    pub flags: i32,
    pub x: i16,
    pub y: i16,
}

/// Full current Formation version-1 wire record. This is serialized state only:
/// live evaluating-operation, archive dirtiness and unit pointers are not encoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Formation {
    pub id: u32,
    pub location: PlotCoord,
    pub owner: i32,
    pub moves_remaining_raw: i32,
    pub domain: i32,
    pub is_embarked: bool,
    pub can_embark: bool,
    pub can_attack: bool,
    pub is_ranged_combat: bool,
    pub can_capture: bool,
    pub has_moved: bool,
    pub has_moved_into_zoc: bool,
    pub ignores_zoc: bool,
    pub last_path: Vec<PathNode>,
    pub last_path_cache_location: PlotCoord,
    pub unit_ids: Vec<u32>,
}
impl Formation {
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, limits)?;
        r.version(1)?;
        let id = r.u32()?;
        let location = PlotCoord {
            x: r.i32()?,
            y: r.i32()?,
        };
        let owner = r.i32()?;
        let moves_remaining_raw = r.i32()?;
        let domain = r.i32()?;
        let is_embarked = r.boolean()?;
        let can_embark = r.boolean()?;
        let can_attack = r.boolean()?;
        let is_ranged_combat = r.boolean()?;
        let can_capture = r.boolean()?;
        let has_moved = r.boolean()?;
        let has_moved_into_zoc = r.boolean()?;
        let ignores_zoc = r.boolean()?;
        let count = r.count(20)?;
        let mut last_path = Vec::with_capacity(count);
        for _ in 0..count {
            r.version(1)?;
            last_path.push(PathNode {
                data1_raw: r.i32()?,
                data2: r.i32()?,
                flags: r.i32()?,
                x: i16::from_le_bytes(r.take()?),
                y: i16::from_le_bytes(r.take()?),
            });
        }
        let last_path_cache_location = PlotCoord {
            x: r.i32()?,
            y: r.i32()?,
        };
        let count = r.count(4)?;
        let mut unit_ids = Vec::with_capacity(count);
        for _ in 0..count {
            unit_ids.push(r.u32()?);
        }
        r.finish()?;
        Ok(Self {
            id,
            location,
            owner,
            moves_remaining_raw,
            domain,
            is_embarked,
            can_embark,
            can_attack,
            is_ranged_combat,
            can_capture,
            has_moved,
            has_moved_into_zoc,
            ignores_zoc,
            last_path,
            last_path_cache_location,
            unit_ids,
        })
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let count = self
            .last_path
            .len()
            .checked_add(self.unit_ids.len())
            .ok_or(Error::ResourceLimit)?;
        let length = self
            .last_path
            .len()
            .checked_mul(20)
            .and_then(|p| {
                self.unit_ids
                    .len()
                    .checked_mul(4)
                    .and_then(|u| p.checked_add(u))
            })
            .and_then(|n| n.checked_add(52))
            .ok_or(Error::ResourceLimit)?;
        if count > limits.max_items
            || length > limits.max_bytes
            || self.last_path.len() > u32::MAX as usize
            || self.unit_ids.len() > u32::MAX as usize
        {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::with_capacity(length);
        for v in [
            1,
            self.id,
            self.location.x as u32,
            self.location.y as u32,
            self.owner as u32,
            self.moves_remaining_raw as u32,
            self.domain as u32,
        ] {
            word(&mut bytes, v);
        }
        for v in [
            self.is_embarked,
            self.can_embark,
            self.can_attack,
            self.is_ranged_combat,
            self.can_capture,
            self.has_moved,
            self.has_moved_into_zoc,
            self.ignores_zoc,
        ] {
            bytes.push(u8::from(v));
        }
        word(&mut bytes, self.last_path.len() as u32);
        for node in &self.last_path {
            for v in [
                1,
                node.data1_raw as u32,
                node.data2 as u32,
                node.flags as u32,
            ] {
                word(&mut bytes, v);
            }
            bytes.extend_from_slice(&node.x.to_le_bytes());
            bytes.extend_from_slice(&node.y.to_le_bytes());
        }
        word(&mut bytes, self.last_path_cache_location.x as u32);
        word(&mut bytes, self.last_path_cache_location.y as u32);
        word(&mut bytes, self.unit_ids.len() as u32);
        for id in &self.unit_ids {
            word(&mut bytes, *id);
        }
        Ok(bytes)
    }
}

fn word(bytes: &mut Vec<u8>, v: u32) {
    bytes.extend_from_slice(&v.to_le_bytes());
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    budget: usize,
    field: &'static str,
    field_start: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        Ok(Self {
            bytes,
            pos: 0,
            budget: limits.max_items,
            field: "Unit",
            field_start: 0,
        })
    }
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let slice = self
            .bytes
            .get(self.pos..)
            .and_then(|s| s.get(..N))
            .ok_or(Error::Truncated)?;
        self.pos += N;
        Ok(slice.try_into().expect("fixed-length slice"))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take()?))
    }
    fn version(&mut self, expected: i32) -> Result<(), Error> {
        let actual = self.i32()?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn count(&mut self, minimum: usize) -> Result<usize, Error> {
        let count = self.u32()? as usize;
        if count > self.budget {
            return Err(Error::ResourceLimit);
        }
        if count > (self.bytes.len() - self.pos) / minimum {
            return Err(Error::Truncated);
        }
        self.budget -= count;
        Ok(count)
    }
    fn coord(&mut self) -> Result<PlotCoord, Error> {
        Ok(PlotCoord {
            x: self.i32()?,
            y: self.i32()?,
        })
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take::<1>()?[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(Error::NoncanonicalBool(n)),
        }
    }
    fn finish(self) -> Result<(), Error> {
        if self.pos != self.bytes.len() {
            Err(Error::TrailingBytes)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn formation_wire() -> Vec<u8> {
        // Independently specified current writer order, including signed shorts
        // packed only here for convenient literal boundary coverage.
        let prefix: [u32; 7] = [
            1,
            u32::MAX,
            0x80000000,
            0x7fffffff,
            0xffffffff,
            0x80000000,
            0xdeadbeef,
        ];
        let mut bytes: Vec<u8> = prefix.into_iter().flat_map(u32::to_le_bytes).collect();
        bytes.extend_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);
        let rest: [u32; 13] = [
            1,
            1,
            0x80000000,
            0x7fffffff,
            0xffffffff,
            0x7fff8000,
            (-9999i32) as u32,
            (-9999i32) as u32,
            4,
            0xffffffff,
            0,
            0xffffffff,
            0x80000000,
        ];
        bytes.extend(rest.into_iter().flat_map(u32::to_le_bytes));
        bytes
    }
    #[test]
    fn formation_independent_wire_preserves_widths_order_and_duplicates() {
        let bytes = formation_wire();
        let state = Formation::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(state.id, u32::MAX);
        assert_eq!(
            state.location,
            PlotCoord {
                x: i32::MIN,
                y: i32::MAX
            }
        );
        assert_eq!(state.owner, -1);
        assert_eq!(state.moves_remaining_raw, i32::MIN);
        assert_eq!(state.domain as u32, 0xdeadbeef);
        assert_eq!(
            state.last_path,
            [PathNode {
                data1_raw: i32::MIN,
                data2: i32::MAX,
                flags: -1,
                x: i16::MIN,
                y: i16::MAX
            }]
        );
        assert_eq!(
            state.last_path_cache_location,
            PlotCoord { x: -9999, y: -9999 }
        );
        assert_eq!(state.unit_ids, [u32::MAX, 0, u32::MAX, 0x80000000]);
        assert_eq!(state.encode(Limits::default()).unwrap(), bytes);
        let mut reordered = state;
        reordered.unit_ids.reverse();
        assert_ne!(reordered.encode(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn formation_malformed_and_resource_boundaries() {
        let bytes = formation_wire();
        for n in 0..bytes.len() {
            assert!(
                Formation::decode(&bytes[..n], Limits::default()).is_err(),
                "prefix {n}"
            );
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert_eq!(
            Formation::decode(&bad, Limits::default()),
            Err(Error::TrailingBytes)
        );
        for offset in [0, 40] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&2u32.to_le_bytes());
            assert_eq!(
                Formation::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 1,
                    actual: 2
                })
            );
        }
        for offset in 28..36 {
            let mut bad = bytes.clone();
            bad[offset] = 2;
            assert_eq!(
                Formation::decode(&bad, Limits::default()),
                Err(Error::NoncanonicalBool(2))
            );
        }
        for offset in [36, 68] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                Formation::decode(&bad, Limits::default()),
                Err(Error::ResourceLimit)
            );
        }
        let state = Formation::decode(&bytes, Limits::default()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: 5,
        };
        assert_eq!(state.encode(exact).unwrap(), bytes);
        assert_eq!(Formation::decode(&bytes, exact).unwrap(), state);
        for limits in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: 4,
                ..exact
            },
        ] {
            assert_eq!(Formation::decode(&bytes, limits), Err(Error::ResourceLimit));
            assert_eq!(state.encode(limits), Err(Error::ResourceLimit));
        }
        let mut empty = state;
        empty.last_path.clear();
        empty.unit_ids.clear();
        let limits = Limits {
            max_bytes: 52,
            max_items: 0,
        };
        let bytes = empty.encode(limits).unwrap();
        assert_eq!(bytes.len(), 52);
        assert_eq!(Formation::decode(&bytes, limits).unwrap(), empty);
    }
    #[test]
    fn ranged_queue_preserves_single_location_and_operation_hash() {
        let bytes: Vec<u8> = [
            1u32,
            1,
            RANGE_ATTACK_HASH,
            2,
            u32::MAX,
            0,
            1,
            0x6b25012e,
            u32::MAX,
            1,
            18,
            13,
            0,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
        let queue = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(queue.nodes[0].operation_hash, RANGE_ATTACK_HASH);
        assert_eq!(
            queue.nodes[0].parameters,
            OperationParameters::MoveTo {
                location: PlotCoord { x: 18, y: 13 },
                modifiers: 0
            }
        );
        assert_eq!(queue.encode(Limits::default()).unwrap(), bytes);
    }
    #[test]
    fn build_improvement_queue_uses_modifiers_then_definition_hash() {
        // Linux XP2 ClassParameters::Write writes v1, modifiers, Improvement hash.
        let words = [1u32, 1, BUILD_IMPROVEMENT_HASH, 2, 0, 0, 7, 0, 0,
            1, 0x80000000, 0xdeadbeef];
        let bytes: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        let mut queue = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(queue.nodes[0].parameters, OperationParameters::BuildImprovement {
            modifiers: 0x80000000, improvement_hash: 0xdeadbeef,
        });
        assert_eq!(queue.encode(Limits::default()).unwrap(), bytes);
        for end in 0..bytes.len() {
            assert!(OperationQueue::decode(&bytes[..end], Limits::default()).is_err());
        }
        queue.nodes[0].operation_hash = MOVE_TO_HASH;
        assert_eq!(queue.encode(Limits::default()),
            Err(Error::ParameterHandlerMismatch(MOVE_TO_HASH)));
    }
    #[test]
    fn pillage_parameters_keep_their_distinct_handler_identity() {
        let words=[1u32,1,PILLAGE_HASH,2,u32::MAX,0,7,0,0,1,0x80000000,u32::MAX];
        let bytes:Vec<u8>=words.into_iter().flat_map(u32::to_le_bytes).collect();
        let mut queue=OperationQueue::decode(&bytes,Limits::default()).unwrap();
        assert_eq!(queue.nodes[0].parameters,OperationParameters::Pillage {
            modifiers:0x80000000,improvement_hash:u32::MAX,
        });
        assert_eq!(queue.encode(Limits::default()).unwrap(),bytes);
        queue.nodes[0].operation_hash=BUILD_IMPROVEMENT_HASH;
        assert_eq!(queue.encode(Limits::default()),Err(Error::ParameterHandlerMismatch(BUILD_IMPROVEMENT_HASH)));
    }
    #[test]
    fn queue_parameter_wire_is_handler_specific_and_preserves_hashes() {
        let words = [
            1u32,
            2,
            MOVE_TO_HASH,
            2,
            0,
            1,
            2,
            3,
            4,
            1,
            0x80000000,
            0x7fffffff,
            0xffffffff,
            MAKE_TRADE_ROUTE_HASH,
            2,
            5,
            6,
            7,
            8,
            9,
            1,
            0x80000000,
            0xffffffff,
            0,
            0x80000000,
            0x7fffffff,
            0xdeadbeef,
        ];
        let bytes: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        let state = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(
            state.nodes[0].parameters,
            OperationParameters::MoveTo {
                location: PlotCoord {
                    x: i32::MIN,
                    y: i32::MAX
                },
                modifiers: u32::MAX
            }
        );
        assert_eq!(
            state.nodes[1].parameters,
            OperationParameters::MakeTradeRoute {
                modifiers: 0x80000000,
                location0: PlotCoord { x: -1, y: 0 },
                location1: PlotCoord {
                    x: i32::MIN,
                    y: i32::MAX
                },
                resource_hash: 0xdeadbeef
            }
        );
        assert_eq!(state.encode(Limits::default()).unwrap(), bytes);
        for n in 0..bytes.len() {
            assert!(OperationQueue::decode(&bytes[..n], Limits::default()).is_err());
        }
        for offset in [36, 80] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&2u32.to_le_bytes());
            assert_eq!(
                OperationQueue::decode(&bad, Limits::default()),
                Err(Error::Version {
                    expected: 1,
                    actual: 2
                })
            );
        }
        let mut bad = state.clone();
        bad.nodes[0].parameters = OperationParameters::None;
        assert_eq!(
            bad.encode(Limits::default()),
            Err(Error::ParameterHandlerMismatch(MOVE_TO_HASH))
        );
        bad.nodes[0].operation_hash = 0;
        assert_eq!(
            bad.encode(Limits::default()),
            Err(Error::UnsupportedHandler(0))
        );
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: 2,
        };
        assert_eq!(state.encode(exact).unwrap(), bytes);
        assert_eq!(OperationQueue::decode(&bytes, exact).unwrap(), state);
        assert_eq!(
            state.encode(Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            }),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn found_city_queue_uses_exact_parameterless_node() {
        let words = [
            1u32,
            1,
            FOUND_CITY_HASH,
            2,
            u32::MAX,
            0,
            5,
            0x490cc8e1,
            u32::MAX,
            0,
        ];
        let bytes: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        let queue = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(queue.nodes[0].operation_hash, FOUND_CITY_HASH);
        assert_eq!(queue.nodes[0].parameters, OperationParameters::None);
        assert_eq!(queue.encode(Limits::default()).unwrap(), bytes);
        let mut bad = queue;
        bad.nodes[0].operation_hash = 0;
        assert_eq!(
            bad.encode(Limits::default()),
            Err(Error::UnsupportedHandler(0))
        );
    }
    #[test]
    fn queue_independent_wire_and_order() {
        let words = [
            1u32,
            2,
            SKIP_TURN_HASH,
            2,
            0x80000000,
            0xffffffff,
            0x7fffffff,
            0x490cc8e1,
            0xffffffff,
            0,
            SKIP_TURN_HASH,
            2,
            0,
            1,
            2,
            3,
            4,
            0,
        ];
        let bytes: Vec<u8> = words.into_iter().flat_map(u32::to_le_bytes).collect();
        let queue = OperationQueue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(queue.nodes[0].directive, i32::MIN);
        assert_eq!(queue.nodes[0].flags, -1);
        assert_eq!(queue.encode(Limits::default()).unwrap(), bytes);
        let mut reversed = queue.clone();
        reversed.nodes.reverse();
        assert_ne!(reversed.encode(Limits::default()).unwrap(), bytes);
        assert_eq!(
            OperationQueue::decode(
                &[1, 0, 0, 0, 0, 0, 0, 0],
                Limits {
                    max_bytes: 8,
                    max_items: 0
                }
            )
            .unwrap()
            .nodes
            .len(),
            0
        );
        for n in 0..bytes.len() {
            assert!(OperationQueue::decode(&bytes[..n], Limits::default()).is_err());
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert_eq!(
            OperationQueue::decode(&bad, Limits::default()),
            Err(Error::TrailingBytes)
        );
        for offset in [0, 12] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&99i32.to_le_bytes());
            assert!(matches!(
                OperationQueue::decode(&bad, Limits::default()),
                Err(Error::Version { .. })
            ));
        }
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            OperationQueue::decode(&bad, Limits::default()),
            Err(Error::UnsupportedHandler(0))
        );
        let mut bad = bytes.clone();
        bad[36..40].copy_from_slice(&1i32.to_le_bytes());
        assert_eq!(
            OperationQueue::decode(&bad, Limits::default()),
            Err(Error::UnsupportedParameters(1))
        );
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: 2,
        };
        assert_eq!(queue.encode(exact).unwrap(), bytes);
        for limits in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: 1,
                ..exact
            },
        ] {
            assert_eq!(queue.encode(limits), Err(Error::ResourceLimit));
            assert_eq!(
                OperationQueue::decode(&bytes, limits),
                Err(Error::ResourceLimit)
            );
        }
        let mut bad = bytes;
        bad[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            OperationQueue::decode(&bad, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
