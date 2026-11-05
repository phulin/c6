//! Short read loans of the retained territory owner between gameplay actions.
use crate::{city_territory::Runtime, city_turn::Error};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::Deref,
};

/// A city query can use an exclusively retained runtime or the same shared
/// runtime that founding and territory actions borrow mutably between queries.
pub trait Territory {
    fn read(&self) -> Result<Read<'_>, Error>;
    /// Mutations require the retained shared owner; an immutable runtime view
    /// cannot provide a mutable loan.
    fn write(&self) -> Result<RefMut<'_, Runtime>, Error> {
        Err(Error::Unsupported("current territory has no mutable owner"))
    }
}

pub enum Read<'a> {
    Borrowed(&'a Runtime),
    Shared(Ref<'a, Runtime>),
}
impl Deref for Read<'_> {
    type Target = Runtime;
    fn deref(&self) -> &Runtime {
        match self {
            Self::Borrowed(runtime) => runtime,
            Self::Shared(runtime) => runtime,
        }
    }
}
impl Territory for Runtime {
    fn read(&self) -> Result<Read<'_>, Error> {
        Ok(Read::Borrowed(self))
    }
}
impl Territory for RefCell<Runtime> {
    fn read(&self) -> Result<Read<'_>, Error> {
        self.try_borrow().map(Read::Shared).map_err(|_| {
            Error::Unsupported("current territory is mutably borrowed during a city query")
        })
    }
    fn write(&self) -> Result<RefMut<'_, Runtime>, Error> {
        self.try_borrow_mut().map_err(|_| {
            Error::Unsupported("current territory is already borrowed during an update")
        })
    }
}
