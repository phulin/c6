//! Additional immutable factory definitions for the existing mutable set owner.
use super::Runtime;
impl Runtime {
    /// Join definition metadata admitted from the same target/rules inputs.
    /// Current child states, subscriptions, labels and allocation counters stay
    /// in this owner; no mutable records are imported or duplicated here.
    pub fn install_factory_definitions(&mut self, source: &Self) -> Result<(), &'static str> {
        for (id, value) in &source.set_definitions {
            if let Some((_, previous)) = self.set_definitions.iter().find(|d| d.0 == *id) {
                if !match (previous, value) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.id == b.id && a.requirements == b.requirements,
                    _ => false,
                } {
                    return Err("current requirement set definition mismatch");
                }
            }
        }
        for value in &source.requirement_definitions {
            if let Some(previous) = self
                .requirement_definitions
                .iter()
                .find(|d| d.id == value.id)
            {
                if previous.predicate != value.predicate
                    || previous.inverse != value.inverse
                    || previous.persistent != value.persistent
                {
                    return Err("current requirement definition mismatch");
                }
            }
        }
        self.set_definitions
            .try_reserve(source.set_definitions.len())
            .map_err(|_| "factory definition allocation")?;
        self.requirement_definitions
            .try_reserve(source.requirement_definitions.len())
            .map_err(|_| "factory definition allocation")?;
        for (id, value) in &source.set_definitions {
            if !self.set_definitions.iter().any(|d| d.0 == *id) {
                self.set_definitions.push((id.clone(), value.clone()));
            }
        }
        for value in &source.requirement_definitions {
            if !self
                .requirement_definitions
                .iter()
                .any(|d| d.id == value.id)
            {
                self.requirement_definitions.push(value.clone());
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::super::{Predicate, RequirementDefinition, SetDefinition};
    use super::*;
    #[test]
    fn factory_metadata_keeps_live_counter_and_rejects_conflicts() {
        let mut current = Runtime::from_initial(900, Vec::new(), Vec::new()).unwrap();
        let definition = RequirementDefinition {
            id: b"unit-tag".to_vec(),
            predicate: Predicate::Unsupported(42),
            inverse: false,
            persistent: false,
        };
        let mut source = Runtime::from_initial(
            100,
            vec![(
                b"unit-set".to_vec(),
                Some(SetDefinition {
                    id: b"unit-set".to_vec(),
                    requirements: vec![definition.id.clone()],
                }),
            )],
            vec![definition],
        )
        .unwrap();
        current.install_factory_definitions(&source).unwrap();
        assert_eq!(current.next_native_id, 900);
        assert_eq!(current.next_label, 1 << 63);
        assert_eq!(current.requirement_definitions.len(), 1);
        current.install_factory_definitions(&source).unwrap();
        assert_eq!(current.requirement_definitions.len(), 1);
        source.requirement_definitions[0].inverse = true;
        assert!(current.install_factory_definitions(&source).is_err());
        assert!(!current.requirement_definitions[0].inverse);
    }
}
