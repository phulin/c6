//! Current Linux XP2 City::Citizens v2 wire values.
//!
//! Pinned target SHA-256: a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa.
//! Writer/reader: 0x286e7b0/0x286e890; current-version constant: 0x2f213e0.
//! Int arrays: WriteArray 0x2d50880 / ReadArray 0x2d504c0. Yield bool
//! hash arrays: 0x2870e30 / 0x2870ec0. Matching DWARF names each field.
//!
//! This codec preserves ordered wire values, including unknown/duplicate hashes.
//! It is not City.Read application or complete City/gameplay state. Native Read
//! neither resets nor resizes: int arrays overwrite the existing prefix up to
//! min(wire count, destination length), consuming excess; Yield bool pairs
//! overlay actual registry slots, consuming unknown hashes. Omitted slots and
//! the unsaved parent pointer require existing state. Initialize (0x286ea40)
//! separately zeros int arrays sized by CityHexspace::GetNumPlots and bool
//! arrays sized by the actual Yield collection; these lengths are dependencies,
//! not inferred from this record. No constructor/read side effects are emulated.
use crate::unit::{HashValue, Limits};

pub const VERSION: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Version { expected: i32, actual: i32 },
    InvalidBool,
    Trailing,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux City Citizens input: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Citizens {
    pub working_plot: Vec<i32>,
    pub forced_working_plot: Vec<i32>,
    pub favored_yields: Vec<HashValue<bool>>,
    pub disfavored_yields: Vec<HashValue<bool>>,
    pub num_unassigned_citizens: i32,
    pub num_citizens_working_plots: i32,
    pub force_update_plot_appeal: bool,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    allocated: usize,
    limits: Limits,
}
impl Reader<'_> {
    fn bytes(&mut self, count: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(count).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let result = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(result)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }
    fn bool(&mut self) -> Result<bool, Error> {
        match self.bytes(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidBool),
        }
    }
    fn list<T>(
        &mut self,
        wire_size: usize,
        mut read: impl FnMut(&mut Self) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        let count = self.u32()? as usize;
        // Original int WriteArray masks its byte count to 30 element bits.
        // Refuse that pathological wrap boundary rather than applying a plain
        // count*4 grammar to it when a caller raises the resource policy.
        if wire_size == 4 && count >= (1usize << 30) {
            return Err(Error::ResourceLimit);
        }
        self.items = self.items.checked_add(count).ok_or(Error::ResourceLimit)?;
        self.allocated = self
            .allocated
            .checked_add(
                count
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(Error::ResourceLimit)?,
            )
            .ok_or(Error::ResourceLimit)?;
        if self.items > self.limits.max_items || self.allocated > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let wire_bytes = count.checked_mul(wire_size).ok_or(Error::ResourceLimit)?;
        if self
            .pos
            .checked_add(wire_bytes)
            .ok_or(Error::ResourceLimit)?
            > self.limits.max_bytes
        {
            return Err(Error::ResourceLimit);
        }
        if wire_bytes > self.data.len() - self.pos {
            return Err(Error::Truncated);
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            result.push(read(self)?);
        }
        Ok(result)
    }
    fn yields(&mut self) -> Result<Vec<HashValue<bool>>, Error> {
        self.list(5, |r| {
            Ok(HashValue {
                definition_hash: r.u32()?,
                value: r.bool()?,
            })
        })
    }
}

impl Citizens {
    /// Decode at a structurally established boundary; never search for versions.
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            allocated: 0,
            limits,
        };
        let actual = r.i32()?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let result = Self {
            working_plot: r.list(4, |r| r.i32())?,
            forced_working_plot: r.list(4, |r| r.i32())?,
            favored_yields: r.yields()?,
            disfavored_yields: r.yields()?,
            num_unassigned_citizens: r.i32()?,
            num_citizens_working_plots: r.i32()?,
            force_update_plot_appeal: r.bool()?,
        };
        Ok((result, r.pos))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (result, consumed) = Self::decode_prefix(bytes, limits)?;
        if consumed != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(result)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let lengths = [
            self.working_plot.len(),
            self.forced_working_plot.len(),
            self.favored_yields.len(),
            self.disfavored_yields.len(),
        ];
        if lengths[0] >= (1usize << 30) || lengths[1] >= (1usize << 30) {
            return Err(Error::ResourceLimit);
        }
        let mut size = 29usize; // version, four counts, two ints, bool
        let mut items = 0usize;
        let mut allocated = 0usize;
        for ((count, width), memory_width) in lengths.into_iter().zip([4, 4, 5, 5]).zip([
            4,
            4,
            std::mem::size_of::<HashValue<bool>>(),
            std::mem::size_of::<HashValue<bool>>(),
        ]) {
            u32::try_from(count).map_err(|_| Error::ResourceLimit)?;
            items = items.checked_add(count).ok_or(Error::ResourceLimit)?;
            size = size
                .checked_add(count.checked_mul(width).ok_or(Error::ResourceLimit)?)
                .ok_or(Error::ResourceLimit)?;
            allocated = allocated
                .checked_add(
                    count
                        .checked_mul(memory_width)
                        .ok_or(Error::ResourceLimit)?,
                )
                .ok_or(Error::ResourceLimit)?;
        }
        if items > limits.max_items || size > limits.max_bytes || allocated > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(size)
            .map_err(|_| Error::ResourceLimit)?;
        out.extend_from_slice(&VERSION.to_le_bytes());
        for values in [&self.working_plot, &self.forced_working_plot] {
            out.extend_from_slice(&(values.len() as u32).to_le_bytes());
            for value in values {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for values in [&self.favored_yields, &self.disfavored_yields] {
            out.extend_from_slice(&(values.len() as u32).to_le_bytes());
            for pair in values {
                out.extend_from_slice(&pair.definition_hash.to_le_bytes());
                out.push(u8::from(pair.value));
            }
        }
        out.extend_from_slice(&self.num_unassigned_citizens.to_le_bytes());
        out.extend_from_slice(&self.num_citizens_working_plots.to_le_bytes());
        out.push(u8::from(self.force_update_plot_appeal));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Citizens {
        Citizens {
            working_plot: vec![i32::MIN, 0, i32::MAX],
            forced_working_plot: vec![-1, 9],
            favored_yields: vec![
                HashValue {
                    definition_hash: u32::MAX,
                    value: true,
                },
                HashValue {
                    definition_hash: u32::MAX,
                    value: false,
                },
            ],
            disfavored_yields: vec![HashValue {
                definition_hash: 0,
                value: true,
            }],
            num_unassigned_citizens: -7,
            num_citizens_working_plots: i32::MAX,
            force_update_plot_appeal: true,
        }
    }
    #[test]
    fn current_nonempty_wire_and_prefix() {
        let value = sample();
        let bytes = value.encode(Limits::default()).unwrap();
        assert_eq!(bytes.len(), 64);
        assert_eq!(&bytes[..8], &[2, 0, 0, 0, 3, 0, 0, 0]);
        assert_eq!(
            &bytes[36..46],
            &[255, 255, 255, 255, 1, 255, 255, 255, 255, 0]
        );
        assert_eq!(Citizens::decode(&bytes, Limits::default()).unwrap(), value);
        let mut followed = bytes.clone();
        followed.extend_from_slice(&[9, 8, 7]);
        assert_eq!(
            Citizens::decode_prefix(&followed, Limits::default()).unwrap(),
            (value, bytes.len())
        );
        assert_eq!(
            Citizens::decode(&followed, Limits::default()),
            Err(Error::Trailing)
        );
    }
    #[test]
    fn empty_arrays_and_exact_byte_budget() {
        let empty = Citizens {
            working_plot: vec![],
            forced_working_plot: vec![],
            favored_yields: vec![],
            disfavored_yields: vec![],
            num_unassigned_citizens: 0,
            num_citizens_working_plots: 0,
            force_update_plot_appeal: false,
        };
        let limits = Limits {
            max_items: 0,
            max_bytes: 29,
        };
        let bytes = empty.encode(limits).unwrap();
        assert_eq!(bytes.len(), 29);
        assert_eq!(Citizens::decode(&bytes, limits).unwrap(), empty);
        let value = sample();
        let limits = Limits {
            max_items: 8,
            max_bytes: 64,
        };
        assert_eq!(
            Citizens::decode(&value.encode(limits).unwrap(), limits).unwrap(),
            value
        );
    }
    #[test]
    fn rejects_truncation_versions_bools_and_limits() {
        let bytes = sample().encode(Limits::default()).unwrap();
        for end in 0..bytes.len() {
            assert_eq!(
                Citizens::decode(&bytes[..end], Limits::default()),
                Err(Error::Truncated),
                "prefix {end}"
            );
        }
        let mut changed = bytes.clone();
        changed[0] = 1;
        assert_eq!(
            Citizens::decode(&changed, Limits::default()),
            Err(Error::Version {
                expected: 2,
                actual: 1
            })
        );
        for offset in [40, 45, 54, 63] {
            let mut changed = bytes.clone();
            changed[offset] = 2;
            assert_eq!(
                Citizens::decode(&changed, Limits::default()),
                Err(Error::InvalidBool)
            );
        }
        for limits in [
            Limits {
                max_items: 7,
                ..Limits::default()
            },
            Limits {
                max_bytes: 63,
                ..Limits::default()
            },
        ] {
            assert_eq!(Citizens::decode(&bytes, limits), Err(Error::ResourceLimit));
            assert_eq!(sample().encode(limits), Err(Error::ResourceLimit));
        }
        let mut huge = bytes;
        huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Citizens::decode(&huge, Limits::default()),
            Err(Error::ResourceLimit)
        );
    }
}
