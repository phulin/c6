//! Source-backed fresh Unit creation components. The ordered factory must still
//! run initial abilities, promotions, Formation attachment and Placement effects.
pub mod abilities;
pub mod constructor;
pub mod default_ai;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ResourceLimit,
}
pub struct Definitions<'a> {
    pub abilities: &'a [u32],
    pub terrains: &'a [u32],
    pub features: &'a [u32],
    pub great_person_classes: &'a [u32],
    pub promotions: &'a [u32],
    pub districts: &'a [u32],
    pub improvements: &'a [u32],
    pub yields: &'a [u32],
    pub operations: &'a [(u32, bool)],
    pub max_array_items: usize,
}
impl Definitions<'_> {
    fn validate(&self) -> Result<(), Error> {
        let count = self
            .abilities
            .len()
            .checked_add(
                self.terrains
                    .len()
                    .checked_mul(2)
                    .ok_or(Error::ResourceLimit)?,
            )
            .and_then(|n| n.checked_add(self.features.len()))
            .and_then(|n| n.checked_add(self.great_person_classes.len().checked_mul(2)?))
            .and_then(|n| n.checked_add(self.promotions.len()))
            .and_then(|n| n.checked_add(self.districts.len().checked_mul(2)?))
            .and_then(|n| n.checked_add(self.improvements.len().checked_mul(2)?))
            .and_then(|n| n.checked_add(self.yields.len()))
            .and_then(|n| n.checked_add(self.operations.len()))
            .ok_or(Error::ResourceLimit)?;
        if count > self.max_array_items {
            Err(Error::ResourceLimit)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Properties {
    pub type_hash: u32,
    pub combat: i32,
    pub sight: i32,
    pub range: i32,
    pub build_charges: i32,
    pub park_charges: i32,
    pub disaster_charges: i32,
    pub action_charges: i32,
    pub religious_strength: i32,
    pub spread_charges: i32,
    pub religious_heal_charges: i32,
    pub religion_evict_percent: i32,
    pub zone_of_control: bool,
}
