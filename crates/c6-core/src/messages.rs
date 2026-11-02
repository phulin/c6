//! Bounded UnitOperation body transport, without game legality or execution.
//!
//! Matches the pinned XP2 null-parameter writer. This is not a message envelope,
//! save format, or native reader implementation; decoding native bodies requires
//! prior knowledge that the registry's selected handler has no parameter object.

/// Lookup key, distinct from the registry's numeric wire ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationHash(pub u32);

impl OperationHash {
    pub const SKIP_TURN: Self = Self(0x2c68_1908);
}

/// Registry-resolved numeric ID, including the invalid sentinel -1.
/// No fixed SkipTurn ID is known; initialization determines the registry mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandlerId(pub i32);

/// Raw identity widths. Values are preserved without asserting a live unit exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitIdentifier {
    pub owner: u16,
    pub object_type: u16,
    pub id: u32,
}

/// Exact 24-byte body for a handler known independently to use null parameters.
/// Request-local booleans at message +0x40/+0x41 are absent from this body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NullParameterUnitOperationBody {
    pub unit: UnitIdentifier,
    pub handler: HandlerId,
    pub secondary: i32,
    pub flags: u32,
    pub mode: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BodyDecodeError {
    Length(usize),
}

impl NullParameterUnitOperationBody {
    pub const BYTE_LEN: usize = 24;

    pub fn encode(self) -> [u8; Self::BYTE_LEN] {
        let mut output = [0; Self::BYTE_LEN];
        output[0..2].copy_from_slice(&self.unit.owner.to_le_bytes());
        output[2..4].copy_from_slice(&self.unit.object_type.to_le_bytes());
        output[4..8].copy_from_slice(&self.unit.id.to_le_bytes());
        output[8..12].copy_from_slice(&self.handler.0.to_le_bytes());
        output[12..16].copy_from_slice(&self.secondary.to_le_bytes());
        output[16..20].copy_from_slice(&self.flags.to_le_bytes());
        output[20..24].copy_from_slice(&self.mode.to_le_bytes());
        output
    }

    /// Strict transport parser: rejects both truncation and trailing bytes.
    /// It does not resolve native handler IDs or deserialize parameter objects.
    pub fn decode(input: &[u8]) -> Result<Self, BodyDecodeError> {
        let input: &[u8; Self::BYTE_LEN] = input
            .try_into()
            .map_err(|_| BodyDecodeError::Length(input.len()))?;
        Ok(Self {
            unit: UnitIdentifier {
                owner: u16::from_le_bytes(input[0..2].try_into().unwrap()),
                object_type: u16::from_le_bytes(input[2..4].try_into().unwrap()),
                id: u32::from_le_bytes(input[4..8].try_into().unwrap()),
            },
            handler: HandlerId(i32::from_le_bytes(input[8..12].try_into().unwrap())),
            secondary: i32::from_le_bytes(input[12..16].try_into().unwrap()),
            flags: u32::from_le_bytes(input[16..20].try_into().unwrap()),
            mode: i32::from_le_bytes(input[20..24].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_fixture_preserves_widths_and_signed_sentinels() {
        let body = NullParameterUnitOperationBody {
            unit: UnitIdentifier {
                owner: 0xffff,
                object_type: 0x8000,
                id: 0x8765_4321,
            },
            handler: HandlerId(-1),
            secondary: i32::MIN,
            flags: 0xfedc_ba98,
            mode: i32::MAX,
        };
        let wire = [
            0xff, 0xff, 0, 0x80, 0x21, 0x43, 0x65, 0x87, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0x80,
            0x98, 0xba, 0xdc, 0xfe, 0xff, 0xff, 0xff, 0x7f,
        ];
        assert_eq!(body.encode(), wire);
        assert_eq!(NullParameterUnitOperationBody::decode(&wire), Ok(body));
    }

    #[test]
    fn transport_rejects_incomplete_and_extended_bodies() {
        for length in 0..=25 {
            if length != NullParameterUnitOperationBody::BYTE_LEN {
                assert_eq!(
                    NullParameterUnitOperationBody::decode(&vec![0; length]),
                    Err(BodyDecodeError::Length(length))
                );
            }
        }
    }
}
