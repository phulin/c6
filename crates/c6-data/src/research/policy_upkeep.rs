//! Rule-driven Culture.ObsoletePolicies predicates. Age and GreatPeople
//! eligibility continuations remain explicit.
use super::*;
use c6_core::player_prerequisites::IndexedBools;

struct Obsolete {
    policy: u32,
    replacement: Option<u32>,
    great_person: bool,
}
pub(super) struct Definitions {
    obsolete: Vec<Obsolete>,
    restricted: Vec<u32>,
}
fn text_hash<E>(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error<E>> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(name) => Ok(Some(linux_type_hash(name))),
        _ => Err(Error::Missing(column)),
    }
}
impl Definitions {
    pub(super) fn from_rules(rules: &Rules) -> Result<Self, Error<std::convert::Infallible>> {
        let mut obsolete = Vec::new();
        for row in rules
            .gameplay()
            .table("ObsoletePolicies")
            .ok_or(Error::Missing("ObsoletePolicies"))?
            .rows()
        {
            obsolete.push(Obsolete {
                policy: text_hash(row, "PolicyType")?
                    .ok_or(Error::Missing("obsolete PolicyType"))?,
                replacement: text_hash(row, "ObsoletePolicy")?,
                great_person: text_hash(row, "RequiresAvailableGreatPersonClass")?.is_some(),
            });
        }
        let mut restricted = Vec::new();
        for row in rules
            .gameplay()
            .table("Policies_XP1")
            .ok_or(Error::Missing("Policies_XP1"))?
            .rows()
        {
            // ObsoletePolicies does not test MinimumGameEra. Its positive
            // age/maximum-era removal path needs the synchronous retract API.
            if scalar::<std::convert::Infallible>(row, "RequiresDarkAge")? != 0
                || scalar::<std::convert::Infallible>(row, "RequiresGoldenAge")? != 0
                || text_hash(row, "MaximumGameEra")?.is_some()
            {
                restricted
                    .push(text_hash(row, "PolicyType")?.ok_or(Error::Missing("age PolicyType"))?);
            }
        }
        Ok(Self {
            obsolete,
            restricted,
        })
    }
    pub(super) fn obsolete<E>(
        &self,
        policy: u32,
        active: bool,
        unlocked: &IndexedBools,
    ) -> Result<bool, Error<E>> {
        // IsPolicyObsolete scans ObsoletePolicies in definition order only
        // when IsPolicyActive succeeds. A replacement's *unlocked* flag is
        // used, rather than its active slot or prerequisite civic directly.
        if active {
            for row in self.obsolete.iter().filter(|row| row.policy == policy) {
                if let Some(replacement) = row.replacement {
                    let flag = unlocked
                        .pairs
                        .iter()
                        .rfind(|p| p.definition_hash == replacement)
                        .ok_or(Error::Missing("current replacement-policy unlocked flag"))?;
                    if flag.value {
                        return Ok(true);
                    }
                }
                if row.great_person {
                    return Err(Error::Missing("current GreatPeople policy obsolescence"));
                }
            }
        }
        Ok(false)
    }
    pub(super) fn check_age<E>(&self, policy: u32) -> Result<(), Error<E>> {
        // Native checks this even if the slot's policy is no longer active.
        if self.restricted.contains(&policy) {
            return Err(Error::Missing("current age-restricted policy upkeep"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::unit::HashValue;
    fn rules() -> Definitions {
        Definitions {
            obsolete: vec![Obsolete {
                policy: 1,
                replacement: Some(2),
                great_person: false,
            }],
            restricted: vec![],
        }
    }
    fn flags(values: &[(u32, bool)]) -> IndexedBools {
        IndexedBools {
            pairs: values
                .iter()
                .map(|&(definition_hash, value)| HashValue {
                    definition_hash,
                    value,
                })
                .collect(),
        }
    }
    #[test]
    fn replacement_unlock_requests_removal_without_selecting_a_card() {
        let rules = rules();
        assert!(!rules
            .obsolete::<()>(1, true, &flags(&[(2, false)]))
            .unwrap());
        assert!(matches!(
            rules.obsolete::<()>(1, true, &flags(&[(2, true)])),
            Ok(true)
        ));
        assert!(matches!(
            rules.obsolete::<()>(1, true, &flags(&[])),
            Err(Error::Missing("current replacement-policy unlocked flag"))
        ));
        assert!(rules.obsolete::<()>(1, false, &flags(&[])).is_ok());
    }
    #[test]
    fn remaining_checks_keep_native_order_and_explicit_boundaries() {
        let rules = Definitions {
            obsolete: vec![Obsolete {
                policy: 1,
                replacement: Some(2),
                great_person: true,
            }],
            restricted: vec![1],
        };
        assert!(matches!(
            rules.obsolete::<()>(1, true, &flags(&[(2, true)])),
            Ok(true)
        ));
        assert!(matches!(
            rules.obsolete::<()>(1, true, &flags(&[(2, false)])),
            Err(Error::Missing("current GreatPeople policy obsolescence"))
        ));
        assert!(matches!(
            rules.check_age::<()>(1),
            Err(Error::Missing("current age-restricted policy upkeep"))
        ));
    }
}
