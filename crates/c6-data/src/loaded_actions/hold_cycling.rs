//! Linux HoldCycling scans over admitted definitions and live registered handlers.
//! Full CanStart callbacks remain effects, including output/parameter cleanup;
//! these scans neither select a player action nor assume callbacks are pure.
#[path = "hold_cycling_commands.rs"]
pub mod commands;
#[path = "hold_cycling_snapshot_effects.rs"]
pub mod snapshot_effects;
use crate::{
    registry::LinuxDefinitionRegistry, resource_binding::ResourceAuthority,
    unit_initialization_definitions as definitions, Rules, SqlValue,
};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Command,
    Operation,
}

#[derive(Debug)]
pub enum Error<E> {
    Effects(E),
    NullCommandHandler(usize),
    Budget,
}

/// Actual default query at the Manager::CanStart boundary. The command's
/// parameters are empty; an operation additionally has directive=-1. Both pass
/// testVisible=false and a fresh nonnull, initially empty output map; operations
/// pass a null plot so the manager resolves the current Unit plot after scripts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefaultQuery {
    pub kind: Kind,
    pub runtime_id: i32,
}
impl DefaultQuery {
    pub fn test_visible(self) -> bool {
        false
    }
    pub fn directive(self) -> Option<i32> {
        (self.kind == Kind::Operation).then_some(-1)
    }
}

/// End and null are distinct: only the operation loop skips null slots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Slot<H> {
    End,
    Null,
    Handler(H),
}

/// Implementations read the current registered order, not SQL order. `slot`
/// rereads the end on every iteration, as the native loop does. Type is queried
/// twice for selected handlers; it must not be memoized across those calls.
/// An implementation must reject unsupported structural registry changes rather
/// than reinterpret invalidated native iterators as valid logical iteration.
pub trait Queries {
    type Handler: Copy;
    type Error;
    fn slot(&mut self, kind: Kind, index: usize) -> Result<Slot<Self::Handler>, Self::Error>;
    fn type_key(&mut self, kind: Kind, handler: Self::Handler) -> Result<u32, Self::Error>;
    fn runtime_id(&mut self, kind: Kind, key: u32) -> Result<i32, Self::Error>;
    /// Run full Manager::CanStart with DefaultQuery's exact arguments, then
    /// destroy the output map and empty parameters BEFORE returning its bool.
    /// Cleanup also runs on callback failure, matching the unwind path. Unknown
    /// subtype/script/destructor/RNG effects must return an error, never false.
    fn can_start_and_cleanup(&mut self, query: DefaultQuery) -> Result<bool, Self::Error>;
}

struct Collection {
    registry: LinuxDefinitionRegistry,
    hold: Vec<bool>,
}
impl Collection {
    fn holds(&self, key: u32) -> bool {
        self.registry.find_index(key).is_some_and(|i| self.hold[i])
    }
}

/// Definition lookup retains native direct-index priority and duplicate-hash
/// precedence. Registration order is independently supplied through Queries.
pub struct Definitions {
    binding: WireBinding,
    commands: Collection,
    operations: Collection,
}
#[derive(Debug)]
pub enum DefinitionError {
    Resources(definitions::Error),
    Rules(crate::Error),
    Unsupported(&'static str),
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, DefinitionError> {
        if state.binding() != bound.destination_binding() {
            return Err(DefinitionError::Unsupported(
                "HoldCycling resource/state binding",
            ));
        }
        definitions::Definitions::validate_resources(rules, bound)
            .map_err(DefinitionError::Resources)?;
        fn collection(
            rules: &Rules,
            table: &'static str,
            field: &'static str,
        ) -> Result<Collection, DefinitionError> {
            let registry =
                definitions::collection(rules, table, field).map_err(DefinitionError::Resources)?;
            let rows = rules
                .gameplay()
                .table(table)
                .ok_or(DefinitionError::Unsupported(table))?;
            let hold = registry
                .entries()
                .iter()
                .map(|entry| {
                    let row = rows
                        .row_by_id(entry.database_rowid)
                        .ok_or(DefinitionError::Unsupported(table))?;
                    match row.get("HoldCycling").map_err(DefinitionError::Rules)? {
                        SqlValue::Integer(0) => Ok(false),
                        SqlValue::Integer(1) => Ok(true),
                        _ => Err(DefinitionError::Unsupported("HoldCycling BOOLEAN")),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Collection { registry, hold })
        }
        Ok(Self {
            binding: state.binding().clone(),
            commands: collection(rules, "UnitCommands", "CommandType")?,
            operations: collection(rules, "UnitOperations", "OperationType")?,
        })
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    /// Command manager only; the caller acquires operations only on false.
    pub fn command_hold_cycling<Q: Queries>(
        &self,
        queries: &mut Q,
        budget: &mut usize,
    ) -> Result<bool, Error<Q::Error>> {
        scan(Kind::Command, &self.commands, queries, budget)
    }
    pub fn hold_cycling<Q: Queries>(
        &self,
        queries: &mut Q,
        budget: &mut usize,
    ) -> Result<bool, Error<Q::Error>> {
        // A true command skips the entire operation manager, including Get/Edit.
        if scan(Kind::Command, &self.commands, queries, budget)? {
            return Ok(true);
        }
        scan(Kind::Operation, &self.operations, queries, budget)
    }
}
fn scan<Q: Queries>(
    kind: Kind,
    definitions: &Collection,
    queries: &mut Q,
    budget: &mut usize,
) -> Result<bool, Error<Q::Error>> {
    let mut index = 0;
    loop {
        *budget = budget.checked_sub(1).ok_or(Error::Budget)?;
        let handler = match queries.slot(kind, index).map_err(Error::Effects)? {
            Slot::End => return Ok(false),
            Slot::Null if kind == Kind::Operation => {
                index += 1;
                continue;
            }
            Slot::Null => return Err(Error::NullCommandHandler(index)),
            Slot::Handler(handler) => handler,
        };
        let key = queries.type_key(kind, handler).map_err(Error::Effects)?;
        if definitions.holds(key) {
            let key = queries.type_key(kind, handler).map_err(Error::Effects)?;
            let runtime_id = queries.runtime_id(kind, key).map_err(Error::Effects)?;
            if queries
                .can_start_and_cleanup(DefaultQuery { kind, runtime_id })
                .map_err(Error::Effects)?
            {
                return Ok(true);
            }
        }
        index += 1;
    }
}

/// ExitFormation::CanStart (0x2d83260), after the manager's common guards.
/// `None` means a genuinely null Formation; unknown resolution is an error at
/// the provider boundary, not None. Count is the original unsigned u32 getter.
pub fn exit_formation_can_start(formation_count: Option<u32>) -> bool {
    formation_count.is_some_and(|count| count > 1)
}
/// Compute the stock ExitFormation leaf from current typed Unit/Formation
/// identity. This does not bypass the enclosing manager's phase/turn/queue/death
/// guards. An absent runtime record is unknown, whereas a known null Formation
/// is the source false branch.
pub fn exit_formation_current(
    state: &LoadedState,
    unit: &c6_core::unit::Unit,
) -> Result<bool, StockLeafError> {
    let runtime = state
        .unit_runtime(unit.e_owner, unit.id)
        .map_err(StockLeafError::State)?;
    let Some(reference) = runtime.formation else {
        return Ok(false);
    };
    state
        .formation_runtime(reference.owner, reference.requested_id)
        .map_err(StockLeafError::State)?;
    let formation = state
        .world()
        .player(reference.owner)
        .map_err(StockLeafError::World)?
        .formation(reference.requested_id)
        .map_err(StockLeafError::World)?
        .ok_or(StockLeafError::Unsupported(
            "ExitFormation current Formation absent",
        ))?;
    if formation.resolved_id != reference.resolved_id {
        return Err(StockLeafError::Unsupported(
            "ExitFormation current Formation identity",
        ));
    }
    let count = u32::try_from(formation.record.unit_ids.len())
        .map_err(|_| StockLeafError::Unsupported("Formation count exceeds native u32"))?;
    Ok(exit_formation_can_start(Some(count)))
}
#[derive(Debug)]
pub enum StockLeafError {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Unsupported(&'static str),
}

/// HarvestWonder::CanStart (0x2d83610) is literal xor eax,eax;ret. No callbacks,
/// writes or RNG occur in this leaf; manager guards must still run first.
pub fn harvest_wonder_can_start() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        registry::{linux_type_hash, DefinitionEntry},
        RulesRowId,
    };
    fn collection() -> Collection {
        Collection {
            registry: LinuxDefinitionRegistry::from_native_order(vec![
                DefinitionEntry {
                    database_rowid: RulesRowId(1),
                    type_name: b"skip".to_vec(),
                },
                DefinitionEntry {
                    database_rowid: RulesRowId(2),
                    type_name: b"hold".to_vec(),
                },
            ]),
            hold: vec![false, true],
        }
    }
    #[derive(Default)]
    struct Q {
        commands: Vec<Option<u32>>,
        operations: Vec<Option<u32>>,
        trace: Vec<String>,
        answers: Vec<Result<bool, &'static str>>,
        second_key: Option<u32>,
    }
    impl Queries for Q {
        type Handler = usize;
        type Error = &'static str;
        fn slot(&mut self, k: Kind, i: usize) -> Result<Slot<usize>, Self::Error> {
            self.trace.push(format!("slot:{k:?}:{i}"));
            Ok(
                match (if k == Kind::Command {
                    &self.commands
                } else {
                    &self.operations
                })
                .get(i)
                {
                    None => Slot::End,
                    Some(None) => Slot::Null,
                    Some(Some(_)) => Slot::Handler(i),
                },
            )
        }
        fn type_key(&mut self, k: Kind, h: usize) -> Result<u32, Self::Error> {
            self.trace.push(format!("type:{k:?}:{h}"));
            if self
                .trace
                .iter()
                .filter(|s| *s == &format!("type:{k:?}:{h}"))
                .count()
                == 2
            {
                if let Some(key) = self.second_key {
                    return Ok(key);
                }
            }
            Ok((if k == Kind::Command {
                &self.commands
            } else {
                &self.operations
            })[h]
                .unwrap())
        }
        fn runtime_id(&mut self, k: Kind, key: u32) -> Result<i32, Self::Error> {
            self.trace.push(format!("id:{k:?}:{key}"));
            Ok(key as i32)
        }
        fn can_start_and_cleanup(&mut self, q: DefaultQuery) -> Result<bool, Self::Error> {
            assert!(!q.test_visible());
            assert_eq!(q.directive(), (q.kind == Kind::Operation).then_some(-1));
            self.trace
                .push(format!("query:{:?}:{}", q.kind, q.runtime_id));
            let answer = self.answers.remove(0);
            self.trace.push("cleanup".into());
            answer
        }
    }
    fn scan_both(q: &mut Q) -> Result<bool, Error<&'static str>> {
        let d = collection();
        let mut budget = 100;
        if scan(Kind::Command, &d, q, &mut budget)? {
            return Ok(true);
        }
        scan(Kind::Operation, &d, q, &mut budget)
    }
    #[test]
    fn order_and_short_circuit_preserve_cleanup_and_effects() {
        let mut q = Q {
            commands: vec![Some(0), Some(1), Some(1)],
            operations: vec![Some(1)],
            answers: vec![Ok(false), Ok(true)],
            ..Q::default()
        };
        assert!(scan_both(&mut q).unwrap());
        assert_eq!(
            q.trace,
            vec![
                "slot:Command:0",
                "type:Command:0",
                "slot:Command:1",
                "type:Command:1",
                "type:Command:1",
                "id:Command:1",
                "query:Command:1",
                "cleanup",
                "slot:Command:2",
                "type:Command:2",
                "type:Command:2",
                "id:Command:1",
                "query:Command:1",
                "cleanup"
            ]
        );
    }
    #[test]
    fn operation_null_skips_command_null_refuses() {
        let mut q = Q {
            operations: vec![None, Some(1)],
            answers: vec![Ok(true)],
            ..Q::default()
        };
        assert!(scan_both(&mut q).unwrap());
        assert_eq!(q.trace[0], "slot:Command:0");
        assert!(q.trace.contains(&"slot:Operation:1".into()));
        let mut q = Q {
            commands: vec![None],
            ..Q::default()
        };
        assert!(matches!(
            scan_both(&mut q),
            Err(Error::NullCommandHandler(0))
        ));
    }
    #[test]
    fn second_type_is_not_first_lookup_and_errors_keep_prefix() {
        let mut q = Q {
            commands: vec![Some(linux_type_hash(b"hold"))],
            second_key: Some(77),
            answers: vec![Err("script RNG effect unresolved")],
            ..Q::default()
        };
        assert!(matches!(
            scan_both(&mut q),
            Err(Error::Effects("script RNG effect unresolved"))
        ));
        assert_eq!(
            &q.trace[q.trace.len() - 3..],
            ["id:Command:77", "query:Command:77", "cleanup"]
        );
    }
    #[test]
    fn missing_definition_skips_without_query_and_budget_stops() {
        let mut q = Q {
            commands: vec![Some(123)],
            ..Q::default()
        };
        assert!(!scan_both(&mut q).unwrap());
        assert!(!q.trace.iter().any(|s| s.starts_with("query")));
        assert!(matches!(
            scan(Kind::Command, &collection(), &mut q, &mut 0),
            Err(Error::Budget)
        ));
    }
    #[test]
    fn stock_leaves_keep_unsigned_count_boundary() {
        for (n, result) in [
            (None, false),
            (Some(0), false),
            (Some(1), false),
            (Some(2), true),
            (Some(u32::MAX), true),
        ] {
            assert_eq!(exit_formation_can_start(n), result);
        }
        assert!(!harvest_wonder_can_start());
    }
}
