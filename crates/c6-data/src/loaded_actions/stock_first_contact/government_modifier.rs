//! Linux DiplomacyGovernments value preparation over live Player.Culture.
//! The canonical modifier owner performs the following SimpleEffect lifecycle.
use crate::{registry::linux_type_hash, Rules};
use c6_core::{loaded_state::LoadedState, player_culture::Culture};

#[derive(Debug)]
struct Government {
    hash: u32,
    intolerance: i32,
    /// Native reference count counts Government_SlotCounts rows, not NumSlots.
    slot_references: usize,
}
#[derive(Debug)]
pub struct Definitions {
    governments: Vec<Government>,
}
impl Definitions {
    /// Build from the caller's already bound gameplay rules.
    pub fn from_rules(rules: &Rules) -> Result<Self, &'static str> {
        let governments = rules
            .gameplay()
            .table("Governments")
            .ok_or("government diplomacy Governments table")?;
        let slots = rules
            .gameplay()
            .table("Government_SlotCounts")
            .ok_or("government diplomacy slot references")?;
        let registry = crate::unit_initialization_definitions::collection(
            rules,
            "Governments",
            "GovernmentType",
        )
        .map_err(|_| "government diplomacy registry")?;
        let mut result = Vec::new();
        for entry in registry.entries() {
            let row = governments
                .rows()
                .find(|row| {
                    row.text("GovernmentType")
                        .ok()
                        .is_some_and(|name| name.as_bytes() == entry.type_name)
                })
                .ok_or("government diplomacy definition")?;
            let intolerance = crate::game_parameters::sql_integer(
                row.get("OtherGovernmentIntolerance")
                    .map_err(|_| "government diplomacy intolerance field")?,
            )
            .ok_or("government diplomacy intolerance integer")?;
            let slot_references = slots
                .rows()
                .filter(|row| {
                    row.text("GovernmentType")
                        .ok()
                        .is_some_and(|name| name.as_bytes() == entry.type_name)
                })
                .count();
            result.push(Government {
                hash: linux_type_hash(&entry.type_name),
                intolerance,
                slot_references,
            });
        }
        Ok(Self {
            governments: result,
        })
    }
    pub fn value(
        &self,
        state: &LoadedState,
        owner: i32,
        subject: i32,
    ) -> Result<i32, &'static str> {
        let (_, owner) = state
            .research_components(owner)
            .map_err(|_| "government diplomacy owner Culture")?;
        let (_, subject) = state
            .research_components(subject)
            .map_err(|_| "government diplomacy subject Culture")?;
        Ok(self.culture_value(owner, subject))
    }
    fn culture_value(&self, owner: &Culture, subject: &Culture) -> i32 {
        let find = |culture: &Culture| {
            self.governments
                .iter()
                .find(|g| g.hash == culture.current_government_hash)
        };
        let (Some(a), Some(b)) = (find(owner), find(subject)) else {
            return 0;
        };
        pair_value(
            a,
            b,
            [
                owner.other_government_intolerance_multiplier_bits,
                subject.other_government_intolerance_multiplier_bits,
            ],
            [
                owner.other_government_intolerance_modifier,
                subject.other_government_intolerance_modifier,
            ],
        )
    }
}
fn pair_value(a: &Government, b: &Government, multipliers: [i32; 2], modifiers: [i32; 2]) -> i32 {
    let av = scaled(a.intolerance, multipliers[0]);
    let bv = scaled(b.intolerance, multipliers[1]);
    if a.hash == b.hash {
        let sum = av.wrapping_add(bv);
        // Native NEG/SHR/SUB/SAR implements truncation toward zero of -sum/2.
        let adjustment = (sum.wrapping_neg() as u32 >> 31) as i32;
        adjustment.wrapping_sub(sum) >> 1
    } else {
        let mut result = av.wrapping_add(bv);
        if a.slot_references == b.slot_references {
            result = result.wrapping_add(modifiers[0]).wrapping_add(modifiers[1]);
        }
        result
    }
}

fn scaled(value: i32, multiplier: i32) -> i32 {
    // FixedPoint<int,8>: constructor wraps before the signed widened product;
    // multiplication narrows before AsInt's arithmetic shift.
    (((i64::from(value.wrapping_shl(8)) * i64::from(multiplier)) >> 8) as i32) >> 8
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_government_negates_average_and_different_uses_reference_counts() {
        let a = Government {
            hash: 1,
            intolerance: 3,
            slot_references: 2,
        };
        let mut b = Government {
            hash: 1,
            intolerance: 3,
            slot_references: 2,
        };
        assert_eq!(pair_value(&a, &b, [256, 128], [99, 99]), -2);
        b.hash = 2;
        assert_eq!(pair_value(&a, &b, [256, 128], [-1, 2]), 5);
        b.slot_references = 3;
        assert_eq!(pair_value(&a, &b, [256, 128], [-1, 2]), 4);
        b.intolerance = -4;
        b.hash = 1;
        assert_eq!(pair_value(&a, &b, [256, 256], [99, 99]), 0);
    }
    #[test]
    fn q8_product_narrows_before_integer_conversion() {
        assert_eq!(scaled(3, 128), 1);
        assert_eq!(scaled(-3, 128), -2);
        assert_eq!(scaled(1 << 23, 256), -(1 << 23));
    }
}
