//! Current Linux XP2 Control.Manager wire state. No native Read initialization,
//! callbacks, global lookups, policy execution or gameplay/RNG are reproduced.
use crate::unit::Limits;

pub const VERSION: u32 = 2;
pub const RECYCLED_VERSION: u32 = 0x10001;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    Version { expected: u32, actual: u32 },
    BlockSize(u32),
    ContractType(i32),
    Bool(u8),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux AI.Control.Manager wire: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub index: i32,
    pub id: u32,
    pub priority: i32,
    pub recruit_priority: i32,
    /// Unsaved live ShouldDestroy bit; wire-only imports leave it unknown.
    pub should_destroy: Option<bool>,
    /// Unsaved callbacks-pointer validity, queried by BeginTurn without invoking it.
    pub is_valid: Option<bool>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecycledInstances {
    pub capacity: u32,
    pub instances: Vec<Instance>,
    pub current_id_bits: u32,
    pub first_free: u32,
    pub last_free: u32,
    pub free_indices: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractBase {
    pub status: i32,
    pub player_id: i32,
    pub priority: i32,
    pub time_limit: i32,
    pub time_estimate: i32,
    pub id: u32,
    pub owner: u32,
    /// Raw FixedPointT<8> storage, without conversion through floating point.
    pub extra_present_value: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractData {
    UnitRequest {
        unit_hash: u32,
        building_hash: u32,
        district_hash: u32,
        location: [i32; 2],
        required_city: u32,
        keep_control: bool,
    },
    TechResearch {
        technology_hashes: [u32; 2],
        best_controller: u32,
    },
    CivicResearch {
        civic_hashes: [u32; 2],
    },
    PurchasePlot {
        location: [i32; 2],
    },
    DiplomaticAction {
        player_id: i32,
        action_hash: u32,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contract {
    pub base: ContractBase,
    pub data: ContractData,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlManager {
    pub player_id: i32,
    pub priority: i32,
    pub controls: RecycledInstances,
    pub has_out_of_turn_processing: bool,
    pub running_as_human: bool,
    /// Null slots are serialized as signed -1 and remain in their saved order.
    pub contracts: Vec<Option<Contract>>,
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
        let bytes = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes.try_into().unwrap())
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take()?))
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.take::<1>()?[0] {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::Bool(x)),
        }
    }
    fn version(&mut self, expected: u32) -> Result<(), Error> {
        let actual = self.u32()?;
        if actual != expected {
            return Err(Error::Version { expected, actual });
        }
        Ok(())
    }
    fn vector<T>(&mut self, count: usize, min_bytes: usize) -> Result<Vec<T>, Error> {
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        let end = self
            .pos
            .checked_add(count.checked_mul(min_bytes).ok_or(Error::ResourceLimit)?)
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        if end > self.data.len() {
            return Err(Error::Truncated);
        }
        count
            .checked_mul(std::mem::size_of::<T>())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(Error::ResourceLimit)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        Ok(result)
    }
    fn location(&mut self) -> Result<[i32; 2], Error> {
        Ok([self.i32()?, self.i32()?])
    }
    fn contract(&mut self, key: i32) -> Result<Contract, Error> {
        if !(0..=4).contains(&key) {
            return Err(Error::ContractType(key));
        }
        self.version(3)?;
        let base = ContractBase {
            status: self.i32()?,
            player_id: self.i32()?,
            priority: self.i32()?,
            time_limit: self.i32()?,
            time_estimate: self.i32()?,
            id: self.u32()?,
            owner: self.u32()?,
            extra_present_value: self.i32()?,
        };
        self.version(if key == 1 || key == 4 { 2 } else { 1 })?;
        let data = match key {
            0 => ContractData::UnitRequest {
                unit_hash: self.u32()?,
                building_hash: self.u32()?,
                district_hash: self.u32()?,
                location: self.location()?,
                required_city: self.u32()?,
                keep_control: self.boolean()?,
            },
            1 => ContractData::TechResearch {
                technology_hashes: [self.u32()?, self.u32()?],
                best_controller: self.u32()?,
            },
            2 => ContractData::CivicResearch {
                civic_hashes: [self.u32()?, self.u32()?],
            },
            3 => ContractData::PurchasePlot {
                location: self.location()?,
            },
            4 => ContractData::DiplomaticAction {
                player_id: self.i32()?,
                action_hash: self.u32()?,
            },
            _ => unreachable!(),
        };
        Ok(Contract { base, data })
    }
}
fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn location(out: &mut Vec<u8>, value: [i32; 2]) {
    for v in value {
        word(out, v as u32);
    }
}
impl Contract {
    fn wire_bytes(&self) -> usize {
        44 + match self.data {
            ContractData::UnitRequest { .. } => 25,
            ContractData::TechResearch { .. } => 12,
            _ => 8,
        }
    }
    fn write(&self, out: &mut Vec<u8>) {
        let key = match self.data {
            ContractData::UnitRequest { .. } => 0,
            ContractData::TechResearch { .. } => 1,
            ContractData::CivicResearch { .. } => 2,
            ContractData::PurchasePlot { .. } => 3,
            ContractData::DiplomaticAction { .. } => 4,
        };
        word(out, key);
        word(out, 3);
        let b = &self.base;
        for v in [
            b.status as u32,
            b.player_id as u32,
            b.priority as u32,
            b.time_limit as u32,
            b.time_estimate as u32,
            b.id,
            b.owner,
            b.extra_present_value as u32,
        ] {
            word(out, v);
        }
        word(out, if key == 1 || key == 4 { 2 } else { 1 });
        match self.data {
            ContractData::UnitRequest {
                unit_hash,
                building_hash,
                district_hash,
                location: loc,
                required_city,
                keep_control,
            } => {
                for v in [unit_hash, building_hash, district_hash] {
                    word(out, v);
                }
                location(out, loc);
                word(out, required_city);
                out.push(u8::from(keep_control));
            }
            ContractData::TechResearch {
                technology_hashes,
                best_controller,
            } => {
                for v in technology_hashes {
                    word(out, v);
                }
                word(out, best_controller);
            }
            ContractData::CivicResearch { civic_hashes } => {
                for v in civic_hashes {
                    word(out, v);
                }
            }
            ContractData::PurchasePlot { location: loc } => location(out, loc),
            ContractData::DiplomaticAction {
                player_id,
                action_hash,
            } => {
                word(out, player_id as u32);
                word(out, action_hash);
            }
        }
    }
}
impl ControlManager {
    pub fn decode(data: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, consumed) = Self::decode_prefix(data, limits)?;
        if consumed != data.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
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
        r.version(VERSION)?;
        let player_id = r.i32()?;
        let priority = r.i32()?;
        r.version(RECYCLED_VERSION)?;
        let block = r.u32()?;
        if block != 16 {
            return Err(Error::BlockSize(block));
        }
        let capacity = r.u32()?;
        let count = r.u32()? as usize;
        let mut instances = r.vector(count, 20)?;
        for _ in 0..count {
            let index = r.i32()?;
            r.version(2)?;
            instances.push(Instance {
                index,
                id: r.u32()?,
                priority: r.i32()?,
                recruit_priority: r.i32()?,
                should_destroy: None,
                is_valid: None,
            });
        }
        let current_id_bits = r.u32()?;
        let count = r.u32()? as usize;
        let first_free = r.u32()?;
        let last_free = r.u32()?;
        let mut free_indices = r.vector(count, 4)?;
        for _ in 0..count {
            free_indices.push(r.u32()?);
        }
        let has_out_of_turn_processing = r.boolean()?;
        let running_as_human = r.boolean()?;
        let count = r.u32()? as usize;
        let mut contracts = r.vector(count, 4)?;
        for _ in 0..count {
            let key = r.i32()?;
            contracts.push(if key == -1 {
                None
            } else {
                Some(r.contract(key)?)
            });
        }
        Ok((
            Self {
                player_id,
                priority,
                controls: RecycledInstances {
                    capacity,
                    instances,
                    current_id_bits,
                    first_free,
                    last_free,
                    free_indices,
                },
                has_out_of_turn_processing,
                running_as_human,
                contracts,
            },
            r.pos,
            r.items,
        ))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.encode_with_items(limits).map(|(v, _)| v)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let c = &self.controls;
        let mut bytes = 50usize;
        let mut items = 0usize;
        for (count, width) in [
            (c.instances.len(), 20usize),
            (c.free_indices.len(), 4),
            (self.contracts.len(), 4),
        ] {
            u32::try_from(count).map_err(|_| Error::ResourceLimit)?;
            items = items.checked_add(count).ok_or(Error::ResourceLimit)?;
            bytes = bytes
                .checked_add(count.checked_mul(width).ok_or(Error::ResourceLimit)?)
                .ok_or(Error::ResourceLimit)?;
        }
        for contract in self.contracts.iter().flatten() {
            bytes = bytes
                .checked_add(contract.wire_bytes() - 4)
                .ok_or(Error::ResourceLimit)?;
        }
        if items > limits.max_items || bytes > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| Error::ResourceLimit)?;
        for v in [
            VERSION,
            self.player_id as u32,
            self.priority as u32,
            RECYCLED_VERSION,
            16,
            c.capacity,
            c.instances.len() as u32,
        ] {
            word(&mut out, v);
        }
        for i in &c.instances {
            for v in [
                i.index as u32,
                2,
                i.id,
                i.priority as u32,
                i.recruit_priority as u32,
            ] {
                word(&mut out, v);
            }
        }
        for v in [
            c.current_id_bits,
            c.free_indices.len() as u32,
            c.first_free,
            c.last_free,
        ] {
            word(&mut out, v);
        }
        for &v in &c.free_indices {
            word(&mut out, v);
        }
        out.extend_from_slice(&[
            u8::from(self.has_out_of_turn_processing),
            u8::from(self.running_as_human),
        ]);
        word(&mut out, self.contracts.len() as u32);
        for c in &self.contracts {
            match c {
                Some(c) => c.write(&mut out),
                None => word(&mut out, u32::MAX),
            }
        }
        debug_assert_eq!(out.len(), bytes);
        Ok((out, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_contract_forms_and_boundaries() {
        let base = ContractBase {
            status: i32::MIN,
            player_id: -1,
            priority: i32::MAX,
            time_limit: -1,
            time_estimate: 0,
            id: u32::MAX,
            owner: 17,
            extra_present_value: -256,
        };
        let forms = [
            ContractData::UnitRequest {
                unit_hash: u32::MAX,
                building_hash: 4,
                district_hash: 9,
                location: [i32::MIN, i32::MAX],
                required_city: u32::MAX,
                keep_control: true,
            },
            ContractData::TechResearch {
                technology_hashes: [0, u32::MAX],
                best_controller: 21,
            },
            ContractData::CivicResearch {
                civic_hashes: [1, 2],
            },
            ContractData::PurchasePlot { location: [-1, 34] },
            ContractData::DiplomaticAction {
                player_id: -1,
                action_hash: u32::MAX,
            },
        ];
        let value = ControlManager {
            player_id: -1,
            priority: -1,
            controls: RecycledInstances {
                capacity: 16,
                instances: vec![Instance {
                    index: 3,
                    id: u32::MAX,
                    priority: -1,
                    recruit_priority: 4,
                    should_destroy: None,
                    is_valid: None,
                }],
                current_id_bits: 7,
                first_free: 2,
                last_free: 1,
                free_indices: vec![2, 1],
            },
            has_out_of_turn_processing: false,
            running_as_human: true,
            contracts: forms
                .into_iter()
                .map(|data| {
                    Some(Contract {
                        base: base.clone(),
                        data,
                    })
                })
                .chain([None])
                .collect(),
        };
        let large = Limits {
            max_bytes: 4096,
            max_items: 9,
        };
        let (wire, items) = value.encode_with_items(large).unwrap();
        assert_eq!(items, 9);
        let exact = Limits {
            max_bytes: wire.len(),
            max_items: items,
        };
        assert_eq!(ControlManager::decode(&wire, exact), Ok(value.clone()));
        for end in 0..wire.len() {
            assert!(ControlManager::decode(&wire[..end], exact).is_err());
        }
        for cap in [
            Limits {
                max_bytes: wire.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(value.encode(cap), Err(Error::ResourceLimit));
            assert_eq!(
                ControlManager::decode(&wire, cap),
                Err(Error::ResourceLimit)
            );
        }
        let mut suffix = wire.clone();
        suffix.push(42);
        assert_eq!(ControlManager::decode(&suffix, exact), Err(Error::Trailing));
        assert_eq!(
            ControlManager::decode_prefix_with_items(&suffix, exact),
            Ok((value, wire.len(), items))
        );
        let key_pos = 50 + 20 + 8;
        let mut hostile = wire;
        hostile[key_pos..key_pos + 4].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(
            ControlManager::decode(&hostile, exact),
            Err(Error::ContractType(5))
        );
    }
    #[test]
    fn hostile_counts_and_versions() {
        let value = ControlManager {
            player_id: 0,
            priority: 0,
            controls: RecycledInstances {
                capacity: 0,
                instances: vec![],
                current_id_bits: 0,
                first_free: u32::MAX,
                last_free: u32::MAX,
                free_indices: vec![],
            },
            has_out_of_turn_processing: false,
            running_as_human: false,
            contracts: vec![],
        };
        let limits = Limits {
            max_bytes: 50,
            max_items: 0,
        };
        let wire = value.encode(limits).unwrap();
        assert_eq!(ControlManager::decode(&wire, limits), Ok(value));
        for offset in [24, 32, 46] {
            let mut bad = wire.clone();
            bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert_eq!(
                ControlManager::decode(&bad, limits),
                Err(Error::ResourceLimit)
            );
        }
        for (offset, expected) in [(0, 2), (12, RECYCLED_VERSION)] {
            let mut bad = wire.clone();
            bad[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
            assert_eq!(
                ControlManager::decode(&bad, limits),
                Err(Error::Version {
                    expected,
                    actual: 0
                })
            );
        }
    }
}
