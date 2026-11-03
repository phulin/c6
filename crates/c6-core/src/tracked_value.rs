//! Bounded current Linux TrackedValue v1 wire transport, without native Read
//! application, constructors, callbacks, RNG or gameplay transitions.
use crate::map::LocalizedText;
use crate::unit::Limits;

pub const VERSION: u32 = 1;
/// Codec policy, not a native gameplay limit. Root depth is zero. Decoding
/// rejects deeper input before constructing it, keeping recursive drop bounded.
pub const MAX_DEPTH: usize = 64;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackedValue {
    pub value_q8: i32,
    pub include_steps: bool,
    pub min_q8: Option<i32>,
    pub max_q8: Option<i32>,
    /// Preserved in native order even when include_steps is false.
    pub steps: Vec<Step>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub priority: i32,
    pub id: u32,
    pub label: LocalizedText,
    pub value: TrackedValue,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    ResourceLimit,
    InvalidBool(u8),
    NegativeCount(i32),
    Version { expected: u32, actual: u32 },
    Localization(crate::map::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux TrackedValue wire: {self:?}")
    }
}
impl std::error::Error for Error {}
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    items: usize,
    limits: Limits,
}
fn charge(items: &mut usize, n: usize, limits: Limits) -> Result<(), Error> {
    *items = items.checked_add(n).ok_or(Error::ResourceLimit)?;
    if *items > limits.max_items {
        return Err(Error::ResourceLimit);
    }
    Ok(())
}
fn local_limits(bytes: usize, items: usize) -> Result<crate::map::Limits, Error> {
    // All localization allocations are backed by charged vector elements or
    // text bytes. 256 bytes per item conservatively covers its typed storage,
    // including a Text box; this is distinct from the serialized byte budget.
    Ok(crate::map::Limits {
        max_bytes: bytes,
        max_items: items,
        max_allocation: items.checked_mul(256).ok_or(Error::ResourceLimit)?,
        max_depth: MAX_DEPTH,
    })
}
impl Reader<'_> {
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let bytes = self.data.get(self.pos..end).ok_or(Error::Truncated)?;
        self.pos = end;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn boolean(&mut self) -> Result<bool, Error> {
        match self.raw(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(Error::InvalidBool(v)),
        }
    }
    fn optional(&mut self) -> Result<Option<i32>, Error> {
        Ok(if self.boolean()? {
            Some(self.u32()? as i32)
        } else {
            None
        })
    }
    fn value(&mut self, depth: usize) -> Result<TrackedValue, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::ResourceLimit);
        }
        charge(&mut self.items, 1, self.limits)?;
        let actual = self.u32()?;
        if actual != VERSION {
            return Err(Error::Version {
                expected: VERSION,
                actual,
            });
        }
        let value_q8 = self.u32()? as i32;
        let include_steps = self.boolean()?;
        let min_q8 = self.optional()?;
        let max_q8 = self.optional()?;
        let count = self.u32()? as i32;
        if count < 0 {
            return Err(Error::NegativeCount(count));
        }
        let count = count as usize;
        charge(&mut self.items, count, self.limits)?;
        // A step has two words, at least 11 localization bytes and 15 child
        // bytes. Preflight both the wire minimum and the typed allocation.
        let minimum = count.checked_mul(34).ok_or(Error::ResourceLimit)?;
        if minimum > self.limits.max_bytes - self.pos {
            return Err(Error::ResourceLimit);
        }
        if minimum > self.data.len().saturating_sub(self.pos) {
            return Err(Error::Truncated);
        }
        if count
            .checked_mul(std::mem::size_of::<Step>())
            .ok_or(Error::ResourceLimit)?
            > isize::MAX as usize
        {
            return Err(Error::ResourceLimit);
        }
        if count != 0 && depth == MAX_DEPTH {
            return Err(Error::ResourceLimit);
        }
        let mut steps = Vec::new();
        steps
            .try_reserve_exact(count)
            .map_err(|_| Error::ResourceLimit)?;
        for _ in 0..count {
            let priority = self.u32()? as i32;
            let id = self.u32()?;
            let limits = local_limits(
                self.limits.max_bytes - self.pos,
                self.limits.max_items - self.items,
            )?;
            let (label, used, items) =
                LocalizedText::decode_prefix_with_items(&self.data[self.pos..], limits)
                    .map_err(Error::Localization)?;
            self.raw(used)?;
            charge(&mut self.items, items, self.limits)?;
            let value = self.value(depth + 1)?;
            steps.push(Step {
                priority,
                id,
                label,
                value,
            });
        }
        Ok(TrackedValue {
            value_q8,
            include_steps,
            min_q8,
            max_q8,
            steps,
        })
    }
}
struct Writer {
    data: Vec<u8>,
    items: usize,
    limits: Limits,
}
impl Writer {
    fn raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let end = self
            .data
            .len()
            .checked_add(bytes.len())
            .ok_or(Error::ResourceLimit)?;
        if end > self.limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        self.data
            .try_reserve(bytes.len())
            .map_err(|_| Error::ResourceLimit)?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.raw(&v.to_le_bytes())
    }
    fn optional(&mut self, v: Option<i32>) -> Result<(), Error> {
        self.raw(&[u8::from(v.is_some())])?;
        if let Some(v) = v {
            self.u32(v as u32)?;
        }
        Ok(())
    }
    fn value(&mut self, v: &TrackedValue, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::ResourceLimit);
        }
        charge(&mut self.items, 1, self.limits)?;
        self.u32(VERSION)?;
        self.u32(v.value_q8 as u32)?;
        self.raw(&[u8::from(v.include_steps)])?;
        self.optional(v.min_q8)?;
        self.optional(v.max_q8)?;
        charge(&mut self.items, v.steps.len(), self.limits)?;
        self.u32(i32::try_from(v.steps.len()).map_err(|_| Error::ResourceLimit)? as u32)?;
        for step in &v.steps {
            self.u32(step.priority as u32)?;
            self.u32(step.id)?;
            let limits = local_limits(
                self.limits.max_bytes - self.data.len(),
                self.limits.max_items - self.items,
            )?;
            let (bytes, items) = step
                .label
                .encode_with_items(limits)
                .map_err(Error::Localization)?;
            charge(&mut self.items, items, self.limits)?;
            self.raw(&bytes)?;
            self.value(&step.value, depth + 1)?;
        }
        Ok(())
    }
}
impl TrackedValue {
    /// Items aggregate tracked nodes (including this root), steps, and the
    /// localization codec's dynamic elements/text bytes. Scalar fields are free.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let mut r = Reader {
            data: bytes,
            pos: 0,
            items: 0,
            limits,
        };
        let value = r.value(0)?;
        Ok((value, r.pos, r.items))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used, _) = Self::decode_prefix_with_items(bytes, limits)?;
        if used != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn encode_with_items(&self, limits: Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut w = Writer {
            data: Vec::new(),
            items: 0,
            limits,
        };
        w.value(self, 0)?;
        Ok((w.data, w.items))
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        Ok(self.encode_with_items(limits)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{LocalizedArgument as A, NamedArgument};
    fn leaf() -> TrackedValue {
        TrackedValue {
            value_q8: i32::MIN,
            include_steps: false,
            min_q8: Some(i32::MIN),
            max_q8: Some(i32::MAX),
            steps: vec![],
        }
    }
    fn label() -> LocalizedText {
        LocalizedText {
            base: vec![0, 255],
            arguments: vec![
                A::None,
                A::Int32(-1),
                A::Long64(i64::MIN),
                A::Float32Bits(0x7fc00042),
                A::Float64Bits(0x7ff8000000000042),
                A::Text(Box::new(LocalizedText {
                    base: vec![],
                    arguments: vec![],
                    named_arguments: vec![],
                })),
            ],
            named_arguments: vec![NamedArgument {
                argument_index: 5,
                name: vec![128],
            }],
        }
    }
    fn branch(v: TrackedValue) -> TrackedValue {
        TrackedValue {
            value_q8: i32::MAX,
            include_steps: false,
            min_q8: None,
            max_q8: None,
            steps: vec![Step {
                priority: i32::MIN,
                id: u32::MAX,
                label: label(),
                value: v,
            }],
        }
    }
    #[test]
    fn recursive_round_trip_and_exact_shared_budgets() {
        let mut value = branch(branch(leaf()));
        value.steps.push(value.steps[0].clone()); // duplicate ID/order preserved
        let (bytes, items) = value.encode_with_items(Limits::default()).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(value, TrackedValue::decode(&bytes, exact).unwrap());
        assert_eq!(
            (bytes.clone(), items),
            value.encode_with_items(exact).unwrap()
        );
        let mut tail = bytes.clone();
        tail.extend([7, 8]);
        assert_eq!(
            (value.clone(), bytes.len(), items),
            TrackedValue::decode_prefix_with_items(&tail, exact).unwrap()
        );
        assert_eq!(
            TrackedValue::decode(&tail, Limits::default()),
            Err(Error::Trailing)
        );
        for short in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert!(TrackedValue::decode(&bytes, short).is_err());
            assert!(value.encode(short).is_err());
        }
        for n in 0..bytes.len() {
            assert!(
                TrackedValue::decode(&bytes[..n], Limits::default()).is_err(),
                "prefix {n}"
            );
        }
    }
    #[test]
    fn typed_storage_is_distinct_from_wire_byte_budget() {
        let mut value = leaf();
        value.steps.push(Step {
            priority: 0,
            id: 0,
            label: LocalizedText {
                base: vec![],
                arguments: vec![],
                named_arguments: vec![],
            },
            value: TrackedValue {
                value_q8: 0,
                include_steps: false,
                min_q8: None,
                max_q8: None,
                steps: vec![],
            },
        });
        let (bytes, items) = value.encode_with_items(Limits::default()).unwrap();
        assert!(std::mem::size_of::<Step>() > bytes.len());
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(TrackedValue::decode(&bytes, exact).unwrap(), value);
        assert_eq!(value.encode(exact).unwrap(), bytes);
    }
    #[test]
    fn strict_headers_and_counts() {
        let bytes = leaf().encode(Limits::default()).unwrap();
        for at in [8, 9, 14] {
            let mut b = bytes.clone();
            b[at] = 2;
            assert_eq!(
                TrackedValue::decode(&b, Limits::default()),
                Err(Error::InvalidBool(2))
            );
        }
        let mut b = bytes.clone();
        b[0] = 0;
        assert!(matches!(
            TrackedValue::decode(&b, Limits::default()),
            Err(Error::Version { .. })
        ));
        let mut b = bytes;
        b[19..23].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(
            TrackedValue::decode(&b, Limits::default()),
            Err(Error::NegativeCount(-1))
        );
    }
    #[test]
    fn bounded_depth_rejects_before_building_deep_tree() {
        let empty_label = LocalizedText {
            base: vec![],
            arguments: vec![],
            named_arguments: vec![],
        };
        let label_bytes = empty_label
            .encode_with_items(crate::map::Limits::default())
            .unwrap()
            .0;
        let mut prefix = vec![];
        prefix.extend(VERSION.to_le_bytes());
        prefix.extend(0i32.to_le_bytes());
        prefix.extend([0, 0, 0]);
        prefix.extend(1i32.to_le_bytes());
        prefix.extend(0i32.to_le_bytes());
        prefix.extend(0u32.to_le_bytes());
        prefix.extend(label_bytes);
        let terminal = TrackedValue {
            value_q8: 0,
            include_steps: true,
            min_q8: None,
            max_q8: None,
            steps: vec![],
        }
        .encode(Limits::default())
        .unwrap();
        let mut bytes = prefix.repeat(MAX_DEPTH);
        bytes.extend(&terminal);
        let value = TrackedValue::decode(&bytes, Limits::default()).unwrap();
        assert_eq!(value.encode(Limits::default()).unwrap(), bytes);
        drop(value);
        let mut too_deep = prefix.repeat(MAX_DEPTH + 1);
        too_deep.extend(terminal);
        assert_eq!(
            TrackedValue::decode(&too_deep, Limits::default()),
            Err(Error::ResourceLimit)
        );
        // Construct one over-depth source value; encoding rejects it as well.
        let mut v = leaf();
        for _ in 0..=MAX_DEPTH {
            v = branch(v)
        }
        assert_eq!(v.encode(Limits::default()), Err(Error::ResourceLimit));
        drop(v);
    }
}
