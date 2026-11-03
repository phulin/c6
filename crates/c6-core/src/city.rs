//! Linux City v44/v51 wire record assembled from verified component codecs.
//! Version44 omits fields with explicit zero-reset or unknown-retained effects;
//! nested component versions remain independently checked.
//!
//! Wire completeness is not complete live City state, constructor/read fixups,
//! gameplay/RNG equivalence, City execution or a complete Player/Core importer.
use crate::unit::{variants, HashValue, Limits};
use crate::{city_citizens, city_economy, city_production, city_services, city_wire};

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Header(city_wire::Error),
    Citizens(city_citizens::Error),
    Economy(city_economy::Error),
    Production(city_production::Error),
    Services(city_services::Error),
    Properties(variants::VariantError),
    Truncated,
    Trailing,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City wire input: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Eq, PartialEq)]
pub struct City {
    pub header: city_wire::Header,
    pub districts: city_wire::Districts,
    pub citizens: city_citizens::Citizens,
    pub culture: city_economy::Culture,
    pub gold: city_economy::Gold,
    pub growth: city_economy::Growth,
    pub religion: city_services::Religion,
    pub resources: city_services::Resources,
    pub build_queue: city_production::BuildQueue,
    pub buildings: city_production::Buildings,
    pub combat: city_services::Combat,
    pub trade: city_services::Trade,
    pub cultural_identity: city_services::CulturalIdentity,
    pub power: city_services::Power,
    /// Original inherited m_kProperties, TypedVariantMap virtual+0x1f8, flag1.
    pub properties: variants::TypedVariantMap,
    /// Preserved without inferring a planning-only exclusion from its name.
    pub ai_controller: u32,
    pub turn_city_founded: i32,
    pub allowed_improvement_hashes: Vec<u32>,
    pub population_change_on_creation: Vec<i32>,
    /// See header.powered_yield_effect(): absent44 means definition-sized zero
    /// reset, not an empty effective native array.
    pub yield_change_bonus_with_power_modifier: Vec<HashValue<i32>>,
}
struct Budget {
    limits: Limits,
    pos: usize,
    items: usize,
}
impl Budget {
    fn powered_yields(
        &mut self,
        bytes: &[u8],
        city_version: i32,
    ) -> Result<Vec<HashValue<i32>>, Error> {
        if city_version == 44 {
            return Ok(Vec::new());
        }
        self.list(bytes, 8, |r, data| {
            Ok(HashValue {
                definition_hash: r.scalar(data)?,
                value: r.scalar(data)? as i32,
            })
        })
    }
    fn remaining(&self) -> Limits {
        Limits {
            max_bytes: self.limits.max_bytes.saturating_sub(self.pos),
            max_items: self.limits.max_items.saturating_sub(self.items),
        }
    }
    fn charge(&mut self, bytes: usize, items: usize) -> Result<(), Error> {
        self.pos = self.pos.checked_add(bytes).ok_or(Error::ResourceLimit)?;
        self.items = self.items.checked_add(items).ok_or(Error::ResourceLimit)?;
        if self.pos > self.limits.max_bytes || self.items > self.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        Ok(())
    }
    fn read<T, E>(
        &mut self,
        bytes: &[u8],
        function: impl FnOnce(&[u8], Limits) -> Result<(T, usize, usize), E>,
        error: impl FnOnce(E) -> Error,
    ) -> Result<T, Error> {
        let tail = bytes.get(self.pos..).ok_or(Error::Truncated)?;
        let (value, consumed, items) = function(tail, self.remaining()).map_err(error)?;
        if consumed > tail.len() {
            return Err(Error::Truncated);
        }
        self.charge(consumed, items)?;
        Ok(value)
    }
    fn scalar(&mut self, bytes: &[u8]) -> Result<u32, Error> {
        let end = self.pos.checked_add(4).ok_or(Error::ResourceLimit)?;
        let value = u32::from_le_bytes(
            bytes
                .get(self.pos..end)
                .ok_or(Error::Truncated)?
                .try_into()
                .unwrap(),
        );
        self.charge(4, 0)?;
        Ok(value)
    }
    fn list<T>(
        &mut self,
        bytes: &[u8],
        size: usize,
        mut read: impl FnMut(&mut Self, &[u8]) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let count = self.scalar(bytes)? as usize;
        self.charge(0, count)?;
        if count > bytes.len().saturating_sub(self.pos) / size {
            return Err(Error::Truncated);
        }
        if count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::ResourceLimit)?
            > self.remaining().max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            values.push(read(self, bytes)?);
        }
        Ok(values)
    }
    fn write(&mut self, out: &mut Vec<u8>, bytes: Vec<u8>, items: usize) -> Result<(), Error> {
        self.charge(bytes.len(), items)?;
        out.try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&bytes);
        Ok(())
    }
    fn put(&mut self, out: &mut Vec<u8>, value: u32) -> Result<(), Error> {
        self.charge(4, 0)?;
        out.try_reserve(4).map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }
    fn count(&mut self, out: &mut Vec<u8>, n: usize) -> Result<(), Error> {
        self.charge(0, n)?;
        self.put(out, u32::try_from(n).map_err(|_| Error::ResourceLimit)?)
    }
}
impl City {
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut b = Budget {
            limits,
            pos: 0,
            items: 0,
        };
        macro_rules! read {
            ($ty:ty,$error:ident) => {
                b.read(bytes, <$ty>::decode_prefix_with_items, Error::$error)?
            };
        }
        let header = read!(city_wire::Header, Header);
        let districts = read!(city_wire::Districts, Header);
        let citizens = read!(city_citizens::Citizens, Citizens);
        let culture = read!(city_economy::Culture, Economy);
        let gold = read!(city_economy::Gold, Economy);
        let growth = read!(city_economy::Growth, Economy);
        let religion = read!(city_services::Religion, Services);
        let resources = read!(city_services::Resources, Services);
        let build_queue = read!(city_production::BuildQueue, Production);
        let buildings = read!(city_production::Buildings, Production);
        let combat = read!(city_services::Combat, Services);
        let trade = read!(city_services::Trade, Services);
        let cultural_identity = read!(city_services::CulturalIdentity, Services);
        let power = read!(city_services::Power, Services);
        let remaining = b.remaining();
        let mut budget = variants::DecodeBudget::new(variants::Limits {
            max_bytes: remaining.max_bytes,
            max_items: remaining.max_items,
            max_depth: 64,
        });
        let (properties, size) =
            variants::TypedVariantMap::decode_prefix(&bytes[b.pos..], &mut budget)
                .map_err(Error::Properties)?;
        b.charge(size, budget.items_used())?;
        let ai_controller = b.scalar(bytes)?;
        let turn_city_founded = b.scalar(bytes)? as i32;
        let allowed_improvement_hashes = b.list(bytes, 4, |r, data| r.scalar(data))?;
        let population_change_on_creation =
            b.list(bytes, 4, |r, data| Ok(r.scalar(data)? as i32))?;
        let yield_change_bonus_with_power_modifier =
            b.powered_yields(bytes, header.wire_version)?;
        Ok((
            Self {
                header,
                districts,
                citizens,
                culture,
                gold,
                growth,
                religion,
                resources,
                build_queue,
                buildings,
                combat,
                trade,
                cultural_identity,
                power,
                properties,
                ai_controller,
                turn_city_founded,
                allowed_improvement_hashes,
                population_change_on_creation,
                yield_change_bonus_with_power_modifier,
            },
            b.pos,
            b.items,
        ))
    }
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let (value, size, _) = Self::decode_prefix_with_items(bytes, limits)?;
        Ok((value, size))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, size) = Self::decode_prefix(bytes, limits)?;
        if size != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut b = Budget {
            limits,
            pos: 0,
            items: 0,
        };
        let mut out = Vec::new();
        macro_rules! write {
            ($field:ident,$error:ident) => {{
                let (bytes, items) = self
                    .$field
                    .encode_with_items(b.remaining())
                    .map_err(Error::$error)?;
                b.write(&mut out, bytes, items)?;
            }};
        }
        write!(header, Header);
        write!(districts, Header);
        write!(citizens, Citizens);
        write!(culture, Economy);
        write!(gold, Economy);
        write!(growth, Economy);
        write!(religion, Services);
        write!(resources, Services);
        write!(build_queue, Production);
        write!(buildings, Production);
        write!(combat, Services);
        write!(trade, Services);
        write!(cultural_identity, Services);
        write!(power, Services);
        let remaining = b.remaining();
        let (bytes, items) = self
            .properties
            .encode(variants::Limits {
                max_bytes: remaining.max_bytes,
                max_items: remaining.max_items,
                max_depth: 64,
            })
            .map_err(Error::Properties)?;
        b.write(&mut out, bytes, items)?;
        b.put(&mut out, self.ai_controller)?;
        b.put(&mut out, self.turn_city_founded as u32)?;
        b.count(&mut out, self.allowed_improvement_hashes.len())?;
        for &v in &self.allowed_improvement_hashes {
            b.put(&mut out, v)?;
        }
        b.count(&mut out, self.population_change_on_creation.len())?;
        for &v in &self.population_change_on_creation {
            b.put(&mut out, v as u32)?;
        }
        b.count(&mut out, self.yield_change_bonus_with_power_modifier.len())?;
        for v in &self.yield_change_bonus_with_power_modifier {
            b.put(&mut out, v.definition_hash)?;
            b.put(&mut out, v.value as u32)?;
        }
        Ok((out, b.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn powered_tail44_consumes_nothing_51_preserves_pairs_and_budget() {
        let bytes = [
            1u32.to_le_bytes(),
            u32::MAX.to_le_bytes(),
            i32::MIN.to_le_bytes(),
            99u32.to_le_bytes(),
        ]
        .concat();
        let mut b = Budget {
            limits: Limits {
                max_bytes: 12,
                max_items: 1,
            },
            pos: 0,
            items: 0,
        };
        assert!(b.powered_yields(&bytes, 44).unwrap().is_empty());
        assert_eq!((b.pos, b.items), (0, 0));
        assert_eq!(
            b.powered_yields(&bytes, 51).unwrap(),
            [HashValue {
                definition_hash: u32::MAX,
                value: i32::MIN
            }]
        );
        assert_eq!((b.pos, b.items), (12, 1));
        let mut b = Budget {
            limits: Limits {
                max_bytes: 12,
                max_items: 0,
            },
            pos: 0,
            items: 0,
        };
        assert_eq!(b.powered_yields(&bytes, 51), Err(Error::ResourceLimit));
        for n in 0..12 {
            let mut b = Budget {
                limits: Limits::default(),
                pos: 0,
                items: 0,
            };
            assert!(b.powered_yields(&bytes[..n], 51).is_err());
        }
    }
    #[test]
    fn shared_component_budget_rejects_cumulative_overflow_and_preserves_tail() {
        // Independent source-only two-component carrier. Actual City record
        // exhaustion/roundtrips and component cuts are checked by native-byte
        // fixtures in the offline differential audit, not synthetic game state.
        let bytes = [
            4u32.to_le_bytes(),
            1u32.to_le_bytes(),
            9u32.to_le_bytes(),
            (-3i32).to_le_bytes(),
            99u32.to_le_bytes(),
        ]
        .concat();
        let limits = Limits {
            max_bytes: 16,
            max_items: 1,
        };
        let mut b = Budget {
            limits,
            pos: 0,
            items: 0,
        };
        let districts = b
            .read(
                &bytes,
                city_wire::Districts::decode_prefix_with_items,
                Error::Header,
            )
            .unwrap();
        assert_eq!(districts.district_ids, [9]);
        assert_eq!((b.pos, b.items), (16, 1));
        assert_eq!(b.scalar(&bytes), Err(Error::ResourceLimit));
        let mut b = Budget {
            limits: Limits {
                max_bytes: 32,
                max_items: 1,
            },
            pos: 0,
            items: 0,
        };
        assert!(b
            .read(
                &bytes,
                city_wire::Districts::decode_prefix_with_items,
                Error::Header
            )
            .is_ok());
        let second = [
            4u32.to_le_bytes(),
            1u32.to_le_bytes(),
            8u32.to_le_bytes(),
            0u32.to_le_bytes(),
        ]
        .concat();
        // No remaining item credit: the same child codec cannot spend it again.
        assert!(b
            .read(
                &[bytes[..16].to_vec(), second].concat(),
                city_wire::Districts::decode_prefix_with_items,
                Error::Header
            )
            .is_err());
    }
}
