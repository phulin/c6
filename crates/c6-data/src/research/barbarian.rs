//! Linux XP2 Barbarian.Manager::AcquireTechsAndCivics (RVA 0x282fa50).
//! Grants use the current acquisition dispatcher, including all synchronous
//! gameplay/RNG consumers. This automatic NPC update never selects research.
use super::{acquisition, collection, parameter, Context, Effects, Error, Kind};
use crate::{registry::linux_type_hash, Rules, SqlValue};

#[derive(Clone, Copy)]
struct Definition {
    index: i32,
    hash: u32,
    free: bool,
}

/// Immutable inputs in actual definition collection order, admitted once with
/// the containing research rules. Civics deliberately use the technology rate.
pub struct Definitions {
    percent: i32,
    technologies: Vec<Definition>,
    civics: Vec<Definition>,
}
impl Definitions {
    pub(super) fn from_rules(rules: &Rules) -> Result<Self, Error<std::convert::Infallible>> {
        let mut collections = Vec::new();
        for (table, key) in [("Technologies", "TechnologyType"), ("Civics", "CivicType")] {
            let registry = collection(rules, table, key).map_err(Error::Definitions)?;
            let rows = rules.gameplay().table(table).ok_or(Error::Missing(table))?;
            let mut definitions = Vec::new();
            for (index, entry) in registry.entries().iter().enumerate() {
                let row = rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("barbarian research definition row"))?;
                let free = match row.get("BarbarianFree").map_err(Error::Rules)? {
                    SqlValue::Integer(0) => false,
                    SqlValue::Integer(1) => true,
                    _ => return Err(Error::Missing("BarbarianFree bool")),
                };
                definitions.push(Definition {
                    index: i32::try_from(index)
                        .map_err(|_| Error::Missing("barbarian research index"))?,
                    hash: linux_type_hash(&entry.type_name),
                    free,
                });
            }
            collections.push(definitions);
        }
        Ok(Self {
            percent: parameter(rules, "BARBARIAN_TECH_PERCENT")
                .ok_or(Error::Missing("BARBARIAN_TECH_PERCENT"))?,
            technologies: collections.remove(0),
            civics: collections.remove(0),
        })
    }
}

fn threshold(alive_count: usize, percent: i32) -> i32 {
    ((alive_count as i32).wrapping_mul(percent).wrapping_add(50) / 100).max(1)
}

// Keep the source loop independently testable without substituting a research
// dispatcher in production. There is only one production implementation below.
trait Current {
    type Error;
    fn alive_majors(&self) -> Result<Vec<i32>, Self::Error>;
    fn has(&self, player: i32, kind: Kind, definition: Definition) -> Result<bool, Self::Error>;
    fn grant(&mut self, player: i32, kind: Kind, definition: Definition)
        -> Result<(), Self::Error>;
}
fn acquire<C: Current>(
    current: &mut C,
    definitions: &Definitions,
    player: i32,
) -> Result<(), C::Error> {
    let required = threshold(current.alive_majors()?.len(), definitions.percent);
    for (kind, entries) in [
        (Kind::Technology, &definitions.technologies),
        (Kind::Civic, &definitions.civics),
    ] {
        for &definition in entries {
            if current.has(player, kind, definition)? {
                continue;
            }
            let mut count = 0i32;
            for major in current.alive_majors()? {
                if current.has(major, kind, definition)? {
                    count = count.wrapping_add(1);
                }
            }
            if count >= required || definition.free {
                current.grant(player, kind, definition)?;
            }
        }
    }
    Ok(())
}
impl<E: Effects> Current for Context<'_, E> {
    type Error = acquisition::Error<Error<E::Error>>;
    fn alive_majors(&self) -> Result<Vec<i32>, Self::Error> {
        acquisition::Context::alive_majors(self).map_err(acquisition::Error::Context)
    }
    fn has(&self, player: i32, kind: Kind, definition: Definition) -> Result<bool, Self::Error> {
        let (techs, culture) = self
            .state
            .research_components(player)
            .map_err(Error::State)
            .map_err(acquisition::Error::Context)?;
        let flags = match kind {
            Kind::Technology => &techs.has_tech,
            Kind::Civic => &culture.has_civic,
        };
        flags
            .pairs
            .iter()
            .rev()
            .find(|pair| pair.definition_hash == definition.hash)
            .map(|pair| pair.value)
            .ok_or(acquisition::Error::MissingAcquiredFlag {
                player,
                hash: definition.hash,
            })
    }
    fn grant(
        &mut self,
        player: i32,
        kind: Kind,
        definition: Definition,
    ) -> Result<(), Self::Error> {
        acquisition::set(self, player, kind, definition.index, true)
    }
}
impl<E: Effects> Context<'_, E> {
    /// Execute one automatic barbarian research pass in native order. Every
    /// grant invokes real acquisition consumers. An error retains the completed
    /// mutation/callback prefix and must not be treated as a completed update or
    /// retried as a continuation (the failed acquisition may already be set).
    pub fn acquire_barbarian_research(
        &mut self,
        player: i32,
    ) -> Result<(), acquisition::Error<Error<E::Error>>> {
        self.check().map_err(acquisition::Error::Context)?;
        let definitions = &self.definitions.barbarian;
        acquire(self, definitions, player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fixture {
        flags: Vec<(i32, Kind, i32)>,
        grants: Vec<(Kind, i32)>,
        fail: Option<(Kind, i32)>,
    }
    impl Current for Fixture {
        type Error = &'static str;
        fn alive_majors(&self) -> Result<Vec<i32>, Self::Error> {
            Ok(if self.grants.is_empty() {
                vec![1, 2]
            } else {
                vec![1]
            })
        }
        fn has(
            &self,
            player: i32,
            kind: Kind,
            definition: Definition,
        ) -> Result<bool, Self::Error> {
            Ok(self.flags.contains(&(player, kind, definition.index)))
        }
        fn grant(
            &mut self,
            player: i32,
            kind: Kind,
            definition: Definition,
        ) -> Result<(), Self::Error> {
            self.flags.push((player, kind, definition.index));
            self.grants.push((kind, definition.index));
            // Model a synchronous consumer changing a later major's research.
            if definition.index == 2 && kind == Kind::Technology {
                self.flags.push((1, Kind::Technology, 0));
            }
            if self.fail == Some((kind, definition.index)) {
                return Err("consumer stopped");
            }
            Ok(())
        }
    }
    fn definition(index: i32, free: bool) -> Definition {
        Definition {
            index,
            hash: index as u32,
            free,
        }
    }
    fn rules() -> Definitions {
        Definitions {
            percent: 50,
            technologies: vec![
                definition(2, true),
                definition(0, false),
                definition(1, false),
            ],
            civics: vec![definition(1, false), definition(0, true)],
        }
    }
    #[test]
    fn rounded_wrapping_threshold() {
        for (n, percent, expected) in [
            (0, 100, 1),
            (3, 50, 2),
            (4, 50, 2),
            (5, 50, 3),
            (100, -51, 1),
            (2, i32::MAX, 1),
            (1, i32::MAX, 1),
        ] {
            assert_eq!(threshold(n, percent), expected);
        }
    }
    #[test]
    fn collection_order_live_callbacks_and_shared_threshold() {
        let mut current = Fixture::default();
        current.flags.push((2, Kind::Technology, 1));
        current.flags.push((1, Kind::Civic, 1));
        acquire(&mut current, &rules(), 63).unwrap();
        assert_eq!(
            current.grants,
            vec![
                (Kind::Technology, 2),
                (Kind::Technology, 0),
                (Kind::Civic, 1),
                (Kind::Civic, 0),
            ]
        );
    }
    #[test]
    fn initial_threshold_survives_membership_changes_and_applies_to_civics() {
        let mut definitions = rules();
        definitions.percent = 100;
        let mut current = Fixture::default();
        current.flags.push((1, Kind::Civic, 1));
        acquire(&mut current, &definitions, 63).unwrap();
        assert_eq!(
            current.grants,
            vec![(Kind::Technology, 2), (Kind::Civic, 0)]
        );
    }
    #[test]
    fn callback_failure_retains_prefix_and_stops_later_grants() {
        let mut current = Fixture {
            fail: Some((Kind::Technology, 0)),
            ..Fixture::default()
        };
        assert_eq!(acquire(&mut current, &rules(), 63), Err("consumer stopped"));
        assert_eq!(
            current.grants,
            vec![(Kind::Technology, 2), (Kind::Technology, 0)]
        );
        assert!(current.flags.contains(&(63, Kind::Technology, 0)));
        assert!(!current
            .flags
            .iter()
            .any(|&(p, k, _)| p == 63 && k == Kind::Civic));
    }
    #[test]
    fn already_acquired_free_definition_does_not_replay() {
        let mut current = Fixture::default();
        current.flags.push((63, Kind::Technology, 2));
        acquire(&mut current, &rules(), 63).unwrap();
        assert_eq!(current.grants, vec![(Kind::Civic, 0)]);
    }
}
