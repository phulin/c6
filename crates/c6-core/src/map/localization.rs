use super::{Error, Reader, Writer};

/// Current writer's recursive localization value, preserving bytes and float bits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalizedText {
    pub base: Vec<u8>,
    pub arguments: Vec<LocalizedArgument>,
    pub named_arguments: Vec<NamedArgument>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalizedArgument {
    None,
    Int32(i32),
    Long64(i64),
    Float32Bits(u32),
    Float64Bits(u64),
    Text(Box<LocalizedText>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedArgument {
    pub argument_index: u32,
    pub name: Vec<u8>,
}
impl LocalizedText {
    /// Decode one current localization prefix using the existing Map codec.
    /// Trailing caller bytes are neither read nor charged. Report the aggregate
    /// item charge so an enclosing component can preserve its own shared budget.
    pub fn decode_prefix_with_items(
        bytes: &[u8],
        limits: super::Limits,
    ) -> Result<(Self, usize, usize), Error> {
        let clipped = bytes.len() > limits.max_bytes;
        let mut reader = Reader::new(&bytes[..bytes.len().min(limits.max_bytes)], limits)?;
        let value = Self::read(&mut reader, 0).map_err(|error| {
            if clipped && error == Error::Truncated {
                Error::ResourceLimit
            } else {
                error
            }
        })?;
        Ok((value, reader.pos, reader.items))
    }

    /// Encode with the same recursive byte/item/allocation/depth enforcement
    /// as Map. The returned item charge belongs to the enclosing record too.
    pub fn encode_with_items(&self, limits: super::Limits) -> Result<(Vec<u8>, usize), Error> {
        let mut writer = Writer::new(limits);
        self.write(&mut writer, 0)?;
        Ok((writer.bytes, writer.items))
    }

    pub(super) fn read(r: &mut Reader<'_>, depth: usize) -> Result<Self, Error> {
        if depth > r.limits.max_depth {
            return Err(Error::ResourceLimit);
        }
        if r.u32()? != u32::MAX || r.u8()? != 1 {
            return Err(Error::UnsupportedLocalizationFormat);
        }
        let base = r.list(1, Reader::u8)?;
        let count = r.u8()? as usize;
        let mut arguments = r.vector::<LocalizedArgument>(count, 1)?;
        for _ in 0..count {
            arguments.push(match r.u8()? {
                0 => LocalizedArgument::None,
                1 => LocalizedArgument::Int32(r.i32()?),
                2 => LocalizedArgument::Long64(r.u64()? as i64),
                3 => LocalizedArgument::Float32Bits(r.u32()?),
                4 => LocalizedArgument::Float64Bits(r.u64()?),
                5 => {
                    r.charge(0, std::mem::size_of::<LocalizedText>())?;
                    LocalizedArgument::Text(Box::new(Self::read(r, depth + 1)?))
                }
                tag => return Err(Error::UnsupportedLocalizationArgument(tag)),
            });
        }
        let count = r.u8()? as usize;
        let mut named_arguments = r.vector::<NamedArgument>(count, 9)?;
        for _ in 0..count {
            let argument_index = r.u32()?;
            if r.u8()? != 1 {
                return Err(Error::UnsupportedLocalizationNameCount);
            }
            let name = r.list(1, Reader::u8)?;
            named_arguments.push(NamedArgument {
                argument_index,
                name,
            });
        }
        Ok(Self {
            base,
            arguments,
            named_arguments,
        })
    }
    pub(super) fn write(&self, w: &mut Writer, depth: usize) -> Result<(), Error> {
        if depth > w.limits.max_depth {
            return Err(Error::ResourceLimit);
        }
        w.u32(u32::MAX)?;
        w.u8(1)?;
        w.list(&self.base, |w, v| w.u8(*v))?;
        w.charge::<LocalizedArgument>(self.arguments.len())?;
        w.u8(u8::try_from(self.arguments.len()).map_err(|_| Error::ResourceLimit)?)?;
        for arg in &self.arguments {
            match arg {
                LocalizedArgument::None => w.u8(0)?,
                LocalizedArgument::Int32(v) => {
                    w.u8(1)?;
                    w.i32(*v)?;
                }
                LocalizedArgument::Long64(v) => {
                    w.u8(2)?;
                    w.u64(*v as u64)?;
                }
                LocalizedArgument::Float32Bits(v) => {
                    w.u8(3)?;
                    w.u32(*v)?;
                }
                LocalizedArgument::Float64Bits(v) => {
                    w.u8(4)?;
                    w.u64(*v)?;
                }
                LocalizedArgument::Text(v) => {
                    w.u8(5)?;
                    w.allocation = w
                        .allocation
                        .checked_add(std::mem::size_of::<LocalizedText>())
                        .ok_or(Error::ResourceLimit)?;
                    if w.allocation > w.limits.max_allocation {
                        return Err(Error::ResourceLimit);
                    }
                    v.write(w, depth + 1)?;
                }
            }
        }
        w.charge::<NamedArgument>(self.named_arguments.len())?;
        w.u8(u8::try_from(self.named_arguments.len()).map_err(|_| Error::ResourceLimit)?)?;
        for arg in &self.named_arguments {
            w.u32(arg.argument_index)?;
            w.u8(1)?;
            w.list(&arg.name, |w, v| w.u8(*v))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod prefix_tests {
    use super::*;
    #[test]
    fn recursive_prefix_preserves_tail_and_reports_shared_item_charge() {
        let value = LocalizedText {
            base: b"base".to_vec(),
            arguments: vec![
                LocalizedArgument::Long64(i64::MIN),
                LocalizedArgument::Text(Box::new(LocalizedText {
                    base: b"nested".to_vec(),
                    arguments: vec![LocalizedArgument::Float64Bits(0x7ff8000000000042)],
                    named_arguments: vec![],
                })),
            ],
            named_arguments: vec![NamedArgument {
                argument_index: 1,
                name: b"N".to_vec(),
            }],
        };
        let limits = crate::map::Limits::default();
        let (wire, items) = value.encode_with_items(limits).unwrap();
        let mut carrier = wire.clone();
        carrier.extend_from_slice(b"untouched-tail");
        assert_eq!(
            LocalizedText::decode_prefix_with_items(&carrier, limits).unwrap(),
            (value.clone(), wire.len(), items)
        );
        // A prefix budget equal to the record length accepts a longer carrier.
        let exact = crate::map::Limits {
            max_bytes: wire.len(),
            ..limits
        };
        assert_eq!(
            LocalizedText::decode_prefix_with_items(&carrier, exact).unwrap(),
            (value.clone(), wire.len(), items)
        );
        // The enclosing record subtracts charges before reading a later value.
        let exhausted = crate::map::Limits {
            max_items: items - 1,
            ..limits
        };
        assert_eq!(
            LocalizedText::decode_prefix_with_items(&wire, exhausted),
            Err(Error::ResourceLimit)
        );
        assert_eq!(
            value.encode_with_items(exhausted),
            Err(Error::ResourceLimit)
        );
        for end in 0..wire.len() {
            assert!(LocalizedText::decode_prefix_with_items(&wire[..end], limits).is_err());
        }
        let too_short = crate::map::Limits {
            max_bytes: wire.len() - 1,
            ..limits
        };
        assert_eq!(
            LocalizedText::decode_prefix_with_items(&carrier, too_short),
            Err(Error::ResourceLimit)
        );
    }
}
