//! Canonical little-endian snapshot envelope; never serializes native padding.
use crate::rng::RandomState;
use crate::{Identity, Snapshot, SNAPSHOT_VERSION};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    Truncated,
    Magic,
    Version,
    Invalid,
    Trailing,
}
impl Snapshot {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"C6SN");
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.identity.dll_sha256);
        out.extend_from_slice(&self.identity.rules_sha256);
        for n in [
            self.random.seed,
            self.random.calls,
            self.random.resets,
            self.random.lock_count,
        ] {
            out.extend_from_slice(&n.to_le_bytes());
        }
        out.push(self.random.synchronous as u8);
        out.extend_from_slice(&(self.sections.len() as u64).to_le_bytes());
        for (name, data) in &self.sections {
            out.extend_from_slice(&(name.len() as u64).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&(data.len() as u64).to_le_bytes());
            out.extend_from_slice(data);
        }
        out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        struct Reader<'a>(&'a [u8]);
        impl<'a> Reader<'a> {
            fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
                if n > self.0.len() {
                    return Err(DecodeError::Truncated);
                }
                let (a, b) = self.0.split_at(n);
                self.0 = b;
                Ok(a)
            }
            fn u32(&mut self) -> Result<u32, DecodeError> {
                Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
            }
            fn len(&mut self) -> Result<usize, DecodeError> {
                usize::try_from(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
                    .map_err(|_| DecodeError::Invalid)
            }
        }
        let mut r = Reader(bytes);
        if r.take(4)? != b"C6SN" {
            return Err(DecodeError::Magic);
        }
        let version = r.u32()?;
        if version != SNAPSHOT_VERSION {
            return Err(DecodeError::Version);
        }
        let identity = Identity {
            dll_sha256: r.take(32)?.try_into().unwrap(),
            rules_sha256: r.take(32)?.try_into().unwrap(),
        };
        let mut random = RandomState {
            seed: r.u32()?,
            calls: r.u32()?,
            resets: r.u32()?,
            lock_count: r.u32()?,
            synchronous: false,
        };
        random.synchronous = match r.take(1)?[0] {
            0 => false,
            1 => true,
            _ => return Err(DecodeError::Invalid),
        };
        let count = r.len()?;
        if count > r.0.len() / 16 {
            return Err(DecodeError::Truncated);
        }
        let mut sections = BTreeMap::new();
        for _ in 0..count {
            let n = r.len()?;
            let name = std::str::from_utf8(r.take(n)?)
                .map_err(|_| DecodeError::Invalid)?
                .to_owned();
            let n = r.len()?;
            let data = r.take(n)?.to_vec();
            if sections.insert(name, data).is_some() {
                return Err(DecodeError::Invalid);
            }
        }
        if !r.0.is_empty() {
            return Err(DecodeError::Trailing);
        }
        Ok(Self {
            version,
            identity,
            random,
            sections,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_all_truncations() {
        let mut s = Snapshot::new(RandomState::new(u32::MAX));
        s.identity.dll_sha256 = [17; 32];
        s.sections.insert("opaque".into(), vec![1, 0, 255]);
        let bytes = s.encode();
        assert_eq!(Snapshot::decode(&bytes), Ok(s));
        for n in 0..bytes.len() {
            assert!(Snapshot::decode(&bytes[..n]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(Snapshot::decode(&trailing), Err(DecodeError::Trailing));
    }
}
