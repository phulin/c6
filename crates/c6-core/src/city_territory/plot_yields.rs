//! Current PlotYields objects/IDs and exact object-created ordering.
use super::CityId;
use crate::unit_moved_signal::{ListenerRing, RegistryError};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Object {
    pub id: u32,
    pub plot: u32,
    pub city: Option<CityId>,
    pub improvement_override: i32,
    /// Native ordered tuples (YieldTypes,int), initially empty, not base yields.
    pub yields: Vec<(i32, i32)>,
    pub owned_modifiers: Option<Vec<i32>>,
    pub affected_modifiers: Option<crate::unit_moved_signal::modifiers::AffectedModifiers>,
}
impl Object {
    pub fn new(plot: u32, city: Option<CityId>, improvement_override: i32) -> Self {
        Self {
            id: 0,
            plot,
            city,
            improvement_override,
            yields: vec![],
            owned_modifiers: Some(vec![]),
            affected_modifiers: Some(
                crate::unit_moved_signal::modifiers::AffectedModifiers::from_sorted(vec![])
                    .expect("new empty affected list"),
            ),
        }
    }
    /// Captured yield tuples do not establish ownership/affected-modifier
    /// state. Required modifier operations must fail before writing it.
    pub fn owned_modifiers_mut(&mut self) -> Result<&mut Vec<i32>, &'static str> {
        self.owned_modifiers
            .as_mut()
            .ok_or("current PlotYields owned modifiers")
    }
    pub fn affected_modifiers_mut(
        &mut self,
    ) -> Result<&mut crate::unit_moved_signal::modifiers::AffectedModifiers, &'static str> {
        self.affected_modifiers
            .as_mut()
            .ok_or("current PlotYields affected modifiers")
    }
    pub const fn is_temporary(&self) -> bool {
        true
    }
    pub fn change_yield(&mut self, kind: i32, delta: i32) {
        if let Some(row) = self.yields.iter_mut().find(|r| r.0 == kind) {
            row.1 = row.1.wrapping_add(delta);
        } else {
            self.yields.push((kind, delta));
        }
    }
    pub fn yield_value(&self, kind: i32) -> i32 {
        self.yields.iter().find(|r| r.0 == kind).map_or(0, |r| r.1)
    }
}
#[derive(Debug)]
pub struct Runtime {
    pub counter: u32,
    pub objects: Vec<Object>,
    pub created: Option<ListenerRing<u64>>,
}
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Ring(RegistryError),
    MissingSignal,
}
pub trait Context {
    type Error;
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
    fn created(&mut self, receiver: u64, object: u32) -> Result<(), Self::Error>;
    /// Full current ModifierSystem.ObjectDestroyed, after removal from vectors.
    fn destroyed(&mut self, object: Object) -> Result<(), Self::Error>;
}
pub fn add<C: Context>(c: &mut C, mut object: Object) -> Result<u32, Error<C::Error>> {
    let runtime = c.runtime_mut();
    runtime.counter = runtime.counter.wrapping_add(1);
    let id = runtime.counter;
    object.id = id;
    runtime.objects.push(object);
    let mut cursor = c
        .runtime_mut()
        .created
        .as_mut()
        .ok_or(Error::MissingSignal)?
        .cursor_start()
        .map_err(Error::Ring)?;
    loop {
        let route = c
            .runtime()
            .created
            .as_ref()
            .ok_or(Error::MissingSignal)?
            .cursor_route(cursor)
            .map_err(Error::Ring)?;
        if let Some(route) = route {
            if let Err(error) = c.created(route, id) {
                c.runtime_mut()
                    .created
                    .as_mut()
                    .ok_or(Error::MissingSignal)?
                    .cursor_release(cursor)
                    .map_err(Error::Ring)?;
                return Err(Error::Context(error));
            }
        }
        match c
            .runtime_mut()
            .created
            .as_mut()
            .ok_or(Error::MissingSignal)?
            .cursor_next(cursor)
            .map_err(Error::Ring)?
        {
            Some(next) => cursor = next,
            None => return Ok(id),
        }
    }
}
pub fn remove<C: Context>(c: &mut C, id: u32) -> Result<(), Error<C::Error>> {
    let objects = &c.runtime().objects;
    let at = objects.partition_point(|v| v.id < id);
    if objects.get(at).is_some_and(|v| v.id == id) {
        let object = c.runtime_mut().objects.remove(at);
        c.destroyed(object).map_err(Error::Context)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        runtime: Runtime,
        called: Vec<u64>,
        destroyed: Vec<u32>,
    }
    impl Context for C {
        type Error = ();
        fn runtime(&self) -> &Runtime {
            &self.runtime
        }
        fn runtime_mut(&mut self) -> &mut Runtime {
            &mut self.runtime
        }
        fn created(&mut self, r: u64, id: u32) -> Result<(), ()> {
            assert_eq!(self.runtime.objects.last().unwrap().id, id);
            self.called.push(r);
            if r == 1 {
                self.runtime.created.as_mut().unwrap().subscribe(3).unwrap();
            }
            Ok(())
        }
        fn destroyed(&mut self, o: Object) -> Result<(), ()> {
            assert!(!self.runtime.objects.iter().any(|v| v.id == o.id));
            self.destroyed.push(o.id);
            Ok(())
        }
    }
    #[test]
    fn add_dispatches_reentrant_appends_and_remove_erases_before_callback() {
        let mut c = C {
            runtime: Runtime {
                counter: 0,
                objects: vec![],
                created: Some(ListenerRing::from_ordered(&[1, 2]).unwrap()),
            },
            called: vec![],
            destroyed: vec![],
        };
        let id = add(&mut c, Object::new(7, Some(CityId { owner: 4, id: 8 }), -1)).unwrap();
        assert_eq!(id, 1);
        assert_eq!(c.called, [1, 2, 3]);
        remove(&mut c, 9).unwrap();
        assert!(c.destroyed.is_empty());
        remove(&mut c, 1).unwrap();
        assert_eq!(c.destroyed, [1]);
    }
    #[test]
    fn tuple_updates_preserve_native_first_match_order_and_wrapping() {
        let mut o = Object::new(0, None, -1);
        assert!(o.is_temporary());
        assert_eq!(o.yield_value(4), 0);
        o.change_yield(4, 0);
        o.change_yield(2, i32::MAX);
        o.change_yield(2, 1);
        assert_eq!(o.yields, [(4, 0), (2, i32::MIN)]);
        o.yields.push((2, 9));
        o.change_yield(2, 1);
        assert_eq!(o.yields[2], (2, 9));
        assert_eq!(o.yield_value(2), i32::MIN + 1);
    }
    #[test]
    fn query_only_import_keeps_modifier_ownership_unknown() {
        let mut object = Object::new(7, None, -1);
        assert!(object.owned_modifiers_mut().unwrap().is_empty());
        assert!(object.affected_modifiers_mut().is_ok());
        object.owned_modifiers = None;
        object.affected_modifiers = None;
        object.yields = vec![(2, 5)];
        assert_eq!(object.yield_value(2), 5);
        assert!(object.owned_modifiers_mut().is_err());
        assert!(object.affected_modifiers_mut().is_err());
        assert_eq!(object.yields, [(2, 5)]);
    }
    #[test]
    fn absent_signal_retains_native_allocated_prefix() {
        let mut c = C {
            runtime: Runtime {
                counter: u32::MAX,
                objects: vec![],
                created: None,
            },
            called: vec![],
            destroyed: vec![],
        };
        assert!(matches!(
            add(&mut c, Object::new(0, None, -1)),
            Err(Error::MissingSignal)
        ));
        assert_eq!(c.runtime.counter, 0);
        assert_eq!(c.runtime.objects[0].id, 0);
    }
}
