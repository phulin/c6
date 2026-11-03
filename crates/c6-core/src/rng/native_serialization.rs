//! Isolated GameCore::Random stream payload, not a game save or simulator snapshot.
//! XP2 Write RVA 0x5fa390 and normal Read RVA 0x5fa190; see
//! evidence/engine/rng-serialization.json. I/O callback failure semantics and
//! the legacy-vector branch are not modeled.
use super::RandomState;

pub const LENGTH: usize = 18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ShortInput {
        available: usize,
    },
    UnsupportedLegacy {
        flag: u8,
    },
    /// Native bytes can preserve values the logical Rust bool cannot represent.
    UnrepresentableSynchronous {
        byte: u8,
    },
}

/// Serialized fields only. Lock count, vptr and object padding are not serialized.
/// The raw synchronous byte is retained exactly, including noncanonical values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SerializedRandomRecord {
    pub seed: u32,
    pub calls: u32,
    pub resets: u32,
    pub synchronous_byte: u8,
}

impl From<RandomState> for SerializedRandomRecord {
    fn from(state: RandomState) -> Self {
        Self {
            seed: state.seed,
            calls: state.calls,
            resets: state.resets,
            synchronous_byte: state.synchronous as u8,
        }
    }
}

impl SerializedRandomRecord {
    /// Original Write always emits version 1 and an absent legacy vector flag.
    pub fn encode(self) -> [u8; LENGTH] {
        let mut bytes = [0; LENGTH];
        for (chunk, value) in bytes[..16].as_chunks_mut::<4>().0.iter_mut().zip([
            1,
            self.seed,
            self.calls,
            self.resets,
        ]) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        bytes[16] = self.synchronous_byte;
        bytes
    }

    /// Decode the supported normal prefix. Original Read consumes but ignores
    /// the version word. Additional bytes belong to the caller's stream.
    /// Legacy vector payloads are explicitly unsupported, regardless of length.
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize), Error> {
        if bytes.len() < LENGTH {
            return Err(Error::ShortInput {
                available: bytes.len(),
            });
        }
        if bytes[17] != 0 {
            return Err(Error::UnsupportedLegacy { flag: bytes[17] });
        }
        let field = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        Ok((
            Self {
                seed: field(4),
                calls: field(8),
                resets: field(12),
                synchronous_byte: bytes[16],
            },
            LENGTH,
        ))
    }

    /// Apply the represented logical fields, preserving the existing lock count.
    /// Rejection leaves state unchanged. A successful native read's preliminary
    /// reset-counter increment is overwritten by the serialized reset value.
    pub fn apply_to(self, state: &mut RandomState) -> Result<(), Error> {
        if self.synchronous_byte > 1 {
            return Err(Error::UnrepresentableSynchronous {
                byte: self.synchronous_byte,
            });
        }
        *state = RandomState {
            seed: self.seed,
            calls: self.calls,
            resets: self.resets,
            lock_count: state.lock_count,
            synchronous: self.synchronous_byte != 0,
        };
        Ok(())
    }
}

/// Decode and apply the supported prefix atomically; returns bytes consumed.
/// This API does not reproduce partial reads or errors of a native I/O stream.
pub fn read_into(state: &mut RandomState, bytes: &[u8]) -> Result<usize, Error> {
    let (record, consumed) = SerializedRandomRecord::decode_prefix(bytes)?;
    record.apply_to(state)?;
    Ok(consumed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_raw_bytes_preserve_records_and_enforce_logical_representability() {
        for byte in 0..=u8::MAX {
            let record = SerializedRandomRecord {
                seed: 37,
                calls: 8,
                resets: 19,
                synchronous_byte: byte,
            };
            assert_eq!(
                SerializedRandomRecord::decode_prefix(&record.encode()),
                Ok((record, LENGTH))
            );
            let mut state = RandomState {
                seed: 91,
                calls: 32,
                resets: 11,
                lock_count: 0xdeadbeef,
                synchronous: true,
            };
            let before = state;
            if byte <= 1 {
                assert_eq!(record.apply_to(&mut state), Ok(()));
                assert_eq!(
                    state,
                    RandomState {
                        seed: 37,
                        calls: 8,
                        resets: 19,
                        lock_count: before.lock_count,
                        synchronous: byte == 1
                    }
                );
            } else {
                assert_eq!(
                    record.apply_to(&mut state),
                    Err(Error::UnrepresentableSynchronous { byte })
                );
                assert_eq!(state, before);
            }
        }
    }
    #[test]
    fn ignored_version_lock_preservation_and_exact_consumption() {
        let record = SerializedRandomRecord {
            seed: 37,
            calls: 8,
            resets: 19,
            synchronous_byte: 1,
        };
        for version in [0u32, 1, 2, u32::MAX] {
            let mut bytes = record.encode().to_vec();
            bytes[..4].copy_from_slice(&version.to_le_bytes());
            bytes.extend([0xde, 0xad]);
            let mut state = RandomState {
                lock_count: 0xdeadbeef,
                resets: u32::MAX,
                ..RandomState::new(99)
            };
            assert_eq!(read_into(&mut state, &bytes), Ok(18));
            assert_eq!(
                state,
                RandomState {
                    seed: 37,
                    calls: 8,
                    resets: 19,
                    lock_count: 0xdeadbeef,
                    synchronous: true
                }
            );
        }
    }
    #[test]
    fn unsupported_inputs_leave_state_unchanged() {
        let bytes = SerializedRandomRecord::from(RandomState::new(5)).encode();
        let mut state = RandomState::new(99);
        let before = state;
        for n in 0..18 {
            assert_eq!(
                read_into(&mut state, &bytes[..n]),
                Err(Error::ShortInput { available: n })
            );
            assert_eq!(state, before);
        }
        for flag in [1, 255] {
            let mut bytes = bytes;
            bytes[17] = flag;
            assert_eq!(
                read_into(&mut state, &bytes),
                Err(Error::UnsupportedLegacy { flag })
            );
            assert_eq!(state, before);
        }
    }
}
