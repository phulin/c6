//! Short read loans of the retained territory owner between gameplay actions.
use crate::{city_territory::Runtime, city_turn::Error};
use std::{
    cell::{Ref, RefCell},
    ops::Deref,
};

/// A city query can use an exclusively retained runtime or the same shared
/// runtime that founding and territory actions borrow mutably between queries.
pub trait Territory {
    fn read(&self) -> Result<Read<'_>, Error>;
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
}
