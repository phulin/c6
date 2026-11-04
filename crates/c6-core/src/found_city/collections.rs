//! The two concrete GameEffects CityCreated collection fanouts. New cities are
//! actual City subjects; the shared signal never substitutes a combat handle.
use crate::{loaded_state::LoadedState, melee_attack::proposals::collections, unit::ComponentId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Collection {
    PlayerCities { receiver: u64, owner: i32 },
    AllCities { receiver: u64 },
}
#[derive(Debug)]
pub enum Error<E> {
    State(crate::loaded_state::Error),
    Context(E),
    Collection(collections::Error<E>),
    Identity,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "City collection insertion: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
pub trait Context: collections::Context<u64> {
    fn state(&self) -> &LoadedState;
    /// Source MakeObject(City*) must resolve/create the actual canonical City
    /// modifier wrapper, including its object-created subscriptions/lifetime.
    fn make_city_object(&mut self, city: ComponentId) -> Result<u64, Self::Error>;
    /// Release the temporary shared reference after AddItem. An Unsupported
    /// callback leaves an explicit incomplete prefix, not a completed action.
    fn release_city_object(&mut self, object: u64) -> Result<(), Self::Error>;
}
/// PlayerCities lambda240d970 and AllCities lambda240ab80. `initializing` from
/// GameSignals.CityCreated is unused by BOTH native collection callbacks.
pub fn city_created<C: Context>(
    context: &mut C,
    collection: Collection,
    city: ComponentId,
) -> Result<(), Error<C::Error>> {
    if city.component_type != 2 {
        return Err(Error::Identity);
    }
    let receiver = match collection {
        Collection::PlayerCities { receiver, owner } => {
            let current = context
                .state()
                .city(i32::from(city.owner), city.id)
                .map_err(Error::State)?;
            if current.header.owner != owner {
                return Ok(());
            }
            if current.header.delayed_death {
                return Ok(());
            }
            receiver
        }
        // Unlike PlayerCities, AllCities performs no delayed-death filter.
        Collection::AllCities { receiver } => receiver,
    };
    let object = context.make_city_object(city).map_err(Error::Context)?;
    collections::add_typed_item(context, receiver, object).map_err(Error::Collection)?;
    context.release_city_object(object).map_err(Error::Context)
}
