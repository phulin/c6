//! Current initialized-context capability membership (Linux 0x2965700).
//! This models HasCapability only; allowances and fallback services are separate.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    hashes: Vec<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidOrder;
impl Membership {
    /// A complete current native collection, in unsigned comparator order.
    /// An unavailable/partial collection must remain None at the caller; an
    /// empty vector explicitly means the complete collection is empty.
    pub fn from_sorted(hashes: Vec<u32>) -> Result<Self, InvalidOrder> {
        if hashes.windows(2).any(|r| r[0] > r[1]) {
            return Err(InvalidOrder);
        }
        Ok(Self { hashes })
    }
    /// equal_range compares only the first unsigned32 hash of each16B record.
    /// Other record fields do not affect HasCapability, even when numeric0.
    pub fn has(&self, hash: u32) -> bool {
        let index = self.hashes.partition_point(|value| *value < hash);
        self.hashes.get(index) == Some(&hash)
    }
    pub fn hashes(&self) -> &[u32] {
        &self.hashes
    }
}
pub const BARBARIAN_CLANS: u32 = 0x6e29_3fcc;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsigned_presence_preserves_duplicate_and_extreme_hashes() {
        let m = Membership::from_sorted(vec![0, 0x7fff_ffff, 0x8000_0000, 0x8000_0000, u32::MAX])
            .unwrap();
        assert!(m.has(0));
        assert!(m.has(0x8000_0000));
        assert!(m.has(u32::MAX));
        assert!(!m.has(BARBARIAN_CLANS));
        assert_eq!(m.hashes().len(), 5);
    }
    #[test]
    fn empty_is_explicit_and_signed_order_is_rejected() {
        assert!(!Membership::from_sorted(vec![]).unwrap().has(0));
        assert_eq!(
            Membership::from_sorted(vec![u32::MAX, 0]),
            Err(InvalidOrder)
        );
    }
}
