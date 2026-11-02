//! Pinned Linux XP2 RequirementSystem v3 wire records, not live requirements.
//! See `docs/linux-requirements-codec.md`. IDs, reference order and raw state bytes
//! are preserved. Object lookup, requirement evaluation, temporary roots and native
//! Load reconstruction are outside this codec.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    /// Native length-prefixed string bytes; no UTF-8 conversion or terminator.
    pub requirement_id: Vec<u8>,
    pub context_id: i32,
    pub subject_id: i32,
    /// Native serialized RequirementState byte, without invented enum semantics.
    pub state_raw: u8,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RequirementSystem {
    /// Original wire sequence, including duplicate records and exact references.
    pub records: Vec<Requirement>,
}

/// Caller resource policy, not native engine limits. Allocation accounts for the
/// record array plus all name bytes; encoded output has a separate byte budget.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_records: usize,
    pub max_allocation: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_records: 1_000_000,
            max_allocation: 128 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    TrailingBytes,
    UnsupportedVersion(u8),
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::ResourceLimit)
}
fn word(bytes: &[u8], pos: usize) -> Result<u32, Error> {
    let end = add(pos, 4)?;
    Ok(u32::from_le_bytes(
        bytes
            .get(pos..end)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}
fn check_count(count: usize, limits: Limits) -> Result<usize, Error> {
    if count > limits.max_records || count > u32::MAX as usize {
        return Err(Error::ResourceLimit);
    }
    let allocation = count
        .checked_mul(std::mem::size_of::<Requirement>())
        .ok_or(Error::ResourceLimit)?;
    if allocation > limits.max_allocation {
        return Err(Error::ResourceLimit);
    }
    Ok(allocation)
}

impl RequirementSystem {
    /// Exact section decode. Preflight validates all lengths and aggregate resource
    /// budgets before allocating any records or names; trailing bytes are rejected.
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let version = *bytes.first().ok_or(Error::Truncated)?;
        if version != 3 {
            return Err(Error::UnsupportedVersion(version));
        }
        let count = word(bytes, 1)? as usize;
        let mut allocation = check_count(count, limits)?;
        if count > bytes.len().saturating_sub(5) / 13 {
            return Err(Error::Truncated);
        }
        let mut pos = 5;
        for _ in 0..count {
            let length = word(bytes, pos)? as usize;
            allocation = add(allocation, length)?;
            if allocation > limits.max_allocation {
                return Err(Error::ResourceLimit);
            }
            pos = add(add(pos, 13)?, length)?;
            if pos > bytes.len() {
                return Err(Error::Truncated);
            }
        }
        if pos != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        pos = 5;
        for _ in 0..count {
            let length = word(bytes, pos)? as usize;
            pos += 4;
            let mut requirement_id = Vec::new();
            requirement_id
                .try_reserve_exact(length)
                .map_err(|_| Error::ResourceLimit)?;
            requirement_id.extend_from_slice(&bytes[pos..pos + length]);
            pos += length;
            let context_id = word(bytes, pos)? as i32;
            let subject_id = word(bytes, pos + 4)? as i32;
            records.push(Requirement {
                requirement_id,
                context_id,
                subject_id,
                state_raw: bytes[pos + 8],
            });
            pos += 9;
        }
        Ok(Self { records })
    }

    /// Original current-version Write field order, with no normalization or ID
    /// assignment. Resource failures return no partially encoded output.
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let mut allocation = check_count(self.records.len(), limits)?;
        let mut size = 5;
        for record in &self.records {
            let length = record.requirement_id.len();
            if length > u32::MAX as usize {
                return Err(Error::ResourceLimit);
            }
            allocation = add(allocation, length)?;
            size = add(add(size, 13)?, length)?;
            if allocation > limits.max_allocation || size > limits.max_bytes {
                return Err(Error::ResourceLimit);
            }
        }
        if size > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::ResourceLimit)?;
        bytes.push(3);
        bytes.extend_from_slice(&(self.records.len() as u32).to_le_bytes());
        for record in &self.records {
            bytes.extend_from_slice(&(record.requirement_id.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&record.requirement_id);
            bytes.extend_from_slice(&record.context_id.to_le_bytes());
            bytes.extend_from_slice(&record.subject_id.to_le_bytes());
            bytes.push(record.state_raw);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> Requirement {
        Requirement {
            requirement_id: vec![0, 0xff, b'A'],
            context_id: i32::MIN,
            subject_id: -1,
            state_raw: 255,
        }
    }
    #[test]
    fn preserves_raw_fields_order_and_duplicates() {
        let a = record();
        let mut b = a.clone();
        b.context_id = i32::MAX;
        b.state_raw = 0;
        let system = RequirementSystem {
            records: vec![a.clone(), b, a],
        };
        let bytes = system.encode(Limits::default()).unwrap();
        assert_eq!(&bytes[..9], &[3, 3, 0, 0, 0, 3, 0, 0, 0]);
        assert_eq!(
            &bytes[9..21],
            &[0, 255, 65, 0, 0, 0, 128, 255, 255, 255, 255, 255]
        );
        assert_eq!(
            RequirementSystem::decode(&bytes, Limits::default()),
            Ok(system)
        );
    }
    #[test]
    fn all_raw_state_bytes_are_transportable() {
        for state_raw in 0..=u8::MAX {
            let system = RequirementSystem {
                records: vec![Requirement {
                    state_raw,
                    ..record()
                }],
            };
            let bytes = system.encode(Limits::default()).unwrap();
            assert_eq!(
                RequirementSystem::decode(&bytes, Limits::default()),
                Ok(system)
            );
        }
    }
    #[test]
    fn rejects_every_truncation_and_trailing_bytes() {
        let system = RequirementSystem {
            records: vec![record()],
        };
        let mut bytes = system.encode(Limits::default()).unwrap();
        for end in 0..bytes.len() {
            assert_eq!(
                RequirementSystem::decode(&bytes[..end], Limits::default()),
                Err(Error::Truncated)
            );
        }
        bytes.push(0);
        assert_eq!(
            RequirementSystem::decode(&bytes, Limits::default()),
            Err(Error::TrailingBytes)
        );
        for version in [0, 1, 2, 4, 255] {
            bytes[0] = version;
            assert_eq!(
                RequirementSystem::decode(&bytes, Limits::default()),
                Err(Error::UnsupportedVersion(version))
            );
        }
    }
    #[test]
    fn bounds_count_aggregate_names_and_output() {
        let system = RequirementSystem {
            records: vec![record(), record()],
        };
        let bytes = system.encode(Limits::default()).unwrap();
        for limits in [
            Limits {
                max_records: 1,
                ..Limits::default()
            },
            Limits {
                max_bytes: bytes.len() - 1,
                ..Limits::default()
            },
            Limits {
                max_allocation: 2 * std::mem::size_of::<Requirement>() + 5,
                ..Limits::default()
            },
        ] {
            assert_eq!(system.encode(limits), Err(Error::ResourceLimit));
            assert_eq!(
                RequirementSystem::decode(&bytes, limits),
                Err(Error::ResourceLimit)
            );
        }
        let mut hostile = vec![3];
        hostile.extend(u32::MAX.to_le_bytes());
        assert_eq!(
            RequirementSystem::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let mut hostile = vec![3, 1, 0, 0, 0];
        hostile.extend(u32::MAX.to_le_bytes());
        hostile.extend([0; 9]);
        assert_eq!(
            RequirementSystem::decode(&hostile, Limits::default()),
            Err(Error::ResourceLimit)
        );
        let empty = RequirementSystem::default();
        assert_eq!(empty.encode(Limits::default()).unwrap(), [3, 0, 0, 0, 0]);
        assert_eq!(
            empty.encode(Limits {
                max_bytes: 4,
                ..Limits::default()
            }),
            Err(Error::ResourceLimit)
        );
    }
}
