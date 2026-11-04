//! Unit.AddInitialAbilities2c27260: native iteration and synchronous change signal.
/// Current tag repository inputs. Hashes are native Tag IDs here, rather than
/// Unit/ability type hashes. A missing repository is a caller error.
pub fn can_have(
    all_units: bool,
    all_combat_units: bool,
    combat: i32,
    ranged: i32,
    bombard: i32,
    unit_tags: &[u32],
    ability_tags: &[u32],
) -> bool {
    all_units
        || (all_combat_units && (combat > 0 || ranged > 0 || bombard > 0))
        || unit_tags.iter().any(|tag| ability_tags.contains(tag))
}
pub trait Context {
    type Error;
    fn ability_count(&self) -> Result<usize, Self::Error>;
    /// Definition.At(loop_index).GetCollectionIndex(), resolved again by At.
    fn collection_index(&self, loop_index: usize) -> Result<usize, Self::Error>;
    fn can_have(&self, index: usize) -> Result<bool, Self::Error>;
    fn inactive(&self, index: usize) -> Result<bool, Self::Error>;
    fn count(&self, index: usize) -> Result<i32, Self::Error>;
    fn store_count(&mut self, index: usize, count: i32) -> Result<(), Self::Error>;
    /// Globals.Game null means no signal; otherwise dispatch GameSignals+1560.
    fn changed(&mut self, index: usize) -> Result<(), Self::Error>;
}
pub fn add_initial<C: Context>(context: &mut C) -> Result<(), C::Error> {
    let count = context.ability_count()?;
    for loop_index in 0..count {
        let index = context.collection_index(loop_index)?;
        if !context.can_have(index)? || context.inactive(index)? {
            continue;
        }
        let previous = context.count(index)?;
        context.store_count(index, previous.wrapping_add(1))?;
        let current = context.count(index)?;
        if (previous > 0) != (current > 0) {
            context.changed(index)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Test {
        counts: [i32; 3],
        allowed: [bool; 3],
        signals: Vec<usize>,
    }
    impl Context for Test {
        type Error = ();
        fn ability_count(&self) -> Result<usize, ()> {
            Ok(3)
        }
        fn collection_index(&self, index: usize) -> Result<usize, ()> {
            Ok(index)
        }
        fn can_have(&self, index: usize) -> Result<bool, ()> {
            Ok(self.allowed[index])
        }
        fn inactive(&self, _: usize) -> Result<bool, ()> {
            Ok(false)
        }
        fn count(&self, index: usize) -> Result<i32, ()> {
            Ok(self.counts[index])
        }
        fn store_count(&mut self, index: usize, value: i32) -> Result<(), ()> {
            self.counts[index] = value;
            Ok(())
        }
        fn changed(&mut self, index: usize) -> Result<(), ()> {
            self.signals.push(index);
            if index == 0 {
                self.allowed[1] = false;
            }
            Ok(())
        }
    }
    #[test]
    fn callbacks_change_later_predicates_and_existing_counts_still_increment() {
        let mut context = Test {
            counts: [0, 0, 2],
            allowed: [true; 3],
            signals: vec![],
        };
        add_initial(&mut context).unwrap();
        assert_eq!(context.counts, [1, 0, 3]);
        assert_eq!(context.signals, [0]);
    }
    #[test]
    fn signed_presence_can_change_on_wrapping_increment() {
        let mut context = Test {
            counts: [i32::MAX, 0, -1],
            allowed: [true, false, true],
            signals: vec![],
        };
        add_initial(&mut context).unwrap();
        assert_eq!(context.counts, [i32::MIN, 0, 0]);
        assert_eq!(context.signals, [0]);
        assert!(can_have(false, true, 0, 1, 0, &[], &[]));
        assert!(can_have(false, false, 0, 0, 0, &[3, 1], &[1]));
        assert!(!can_have(false, true, 0, 0, 0, &[3], &[1]));
    }
}
