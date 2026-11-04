//! Player.Cities.Create2a58ef0: real base construction followed by naming,
//! placement and synchronous creation/capital continuations on the same state.
use crate::{
    city_initialization as base,
    unit::{ComponentId, PlotCoord},
};

#[derive(Debug)]
pub enum Error<E> {
    Base(base::CreationError<E>),
    State(crate::loaded_state::Error),
    Effect(E),
    Naming(super::names::Error),
    ResourceLimit,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Placed city creation: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// Mandatory reached continuations. They observe/mutate the very state used by
/// base creation; none has a successful default or a precomputed native answer.
pub trait Context: base::Context {
    /// Includes the real NameManager mutation/RNG before returning a name.
    fn generate_name(
        &mut self,
        city: ComponentId,
        first_city: bool,
        location: PlotCoord,
    ) -> Result<Vec<u8>, Self::Error>;
    fn renamed_signal(&mut self, city: ComponentId) -> Result<(), Self::Error>;
    fn report(&mut self, event_hash: u32, city: ComponentId) -> Result<(), Self::Error>;
    fn place(
        &mut self,
        city: ComponentId,
        location: PlotCoord,
        initializing: bool,
    ) -> Result<i32, Self::Error>;
    fn destroy_failed_placement(&mut self, city: ComponentId) -> Result<(), Self::Error>;
    fn city_created(&mut self, city: ComponentId, initializing: bool) -> Result<(), Self::Error>;
    fn current_continent(&mut self, city: ComponentId) -> Result<u32, Self::Error>;
    fn settled_on_continent(
        &mut self,
        city: ComponentId,
        continent: u32,
    ) -> Result<(), Self::Error>;
    fn is_free_cities(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn capital_changed(&mut self, city: ComponentId) -> Result<(), Self::Error>;
    fn add_palace(&mut self, city: ComponentId) -> Result<(), Self::Error>;
    fn cultural_identity_created(&mut self, city: ComponentId) -> Result<(), Self::Error>;
    fn check_production(&mut self, city: ComponentId) -> Result<(), Self::Error>;
}
fn owner(city: ComponentId) -> i32 {
    i32::from(city.owner)
}
fn cstr(value: &[u8]) -> &[u8] {
    value.split(|&v| v == 0).next().unwrap_or(&[])
}
/// SetName2894b50: assign first, dispatch rename only for a nonempty changed
/// previous string, then reread location before the optional CID report.
pub fn set_name<C: Context>(
    context: &mut C,
    city: ComponentId,
    name: &[u8],
) -> Result<(), Error<C::Error>> {
    let value = context
        .state()
        .city(owner(city), city.id)
        .map_err(Error::State)?;
    let incoming = cstr(name);
    let renamed = !value.header.name.is_empty() && cstr(&value.header.name) != incoming;
    let mut next = Vec::new();
    next.try_reserve_exact(incoming.len())
        .map_err(|_| Error::ResourceLimit)?;
    next.extend_from_slice(incoming);
    context
        .state_mut()
        .city_mut(owner(city), city.id)
        .map_err(Error::State)?
        .header
        .name = next;
    if renamed {
        context.renamed_signal(city).map_err(Error::Effect)?;
    }
    let current = context
        .state()
        .city(owner(city), city.id)
        .map_err(Error::State)?;
    // PlotCoord.IsValid1fe2340 is a sentinel check, not map bounds.
    if current.header.location_x > -9999 && current.header.location_y > -9999 {
        context.report(0xd482_0ab3, city).map_err(Error::Effect)?;
    }
    Ok(())
}
/// SetAsCapital2894da0 writes before synchronous listeners and report. A
/// listener may reverse the flag; the caller must query it again afterward.
pub fn set_as_capital<C: Context>(
    context: &mut C,
    city: ComponentId,
    value: bool,
) -> Result<(), Error<C::Error>> {
    if context
        .state()
        .city(owner(city), city.id)
        .map_err(Error::State)?
        .header
        .capital
        == value
    {
        return Ok(());
    }
    context
        .state_mut()
        .city_mut(owner(city), city.id)
        .map_err(Error::State)?
        .header
        .capital = value;
    context.capital_changed(city).map_err(Error::Effect)?;
    context.report(0x7bbb_e6e0, city).map_err(Error::Effect)
}
/// FoundCity's empty-name creation call. `first_city` controls name/capital;
/// `parameters.original_capital` is a distinct initialization field.
pub fn create<C: Context>(
    context: &mut C,
    definitions: &base::Definitions<'_>,
    parameters: base::Parameters,
    creation: super::Creation,
    initializing: bool,
) -> Result<Option<ComponentId>, Error<C::Error>> {
    let Some(city) =
        base::create(context, definitions, creation.owner, parameters).map_err(Error::Base)?
    else {
        return Ok(None);
    };
    let name = context
        .generate_name(city, creation.first_city, creation.location)
        .map_err(Error::Effect)?;
    set_name(context, city, &name)?;
    if context
        .place(city, creation.location, initializing)
        .map_err(Error::Effect)?
        != 1
    {
        context
            .destroy_failed_placement(city)
            .map_err(Error::Effect)?;
        return Ok(None);
    }
    context
        .city_created(city, initializing)
        .map_err(Error::Effect)?;
    let continent = context.current_continent(city).map_err(Error::Effect)?;
    let cities = context
        .state_mut()
        .cities_mut(creation.owner)
        .map_err(Error::State)?;
    if cities.records.entries.len() == 1 {
        cities.home_continent_hash = continent;
    }
    if !cities.settled_continent_hashes.contains(&continent) {
        cities
            .settled_continent_hashes
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        cities.settled_continent_hashes.push(continent);
        if !initializing {
            context
                .settled_on_continent(city, continent)
                .map_err(Error::Effect)?;
        }
    }
    if creation.first_city
        && !context
            .is_free_cities(creation.owner)
            .map_err(Error::Effect)?
    {
        set_as_capital(context, city, true)?;
        if context
            .state()
            .city(owner(city), city.id)
            .map_err(Error::State)?
            .header
            .capital
        {
            context.add_palace(city).map_err(Error::Effect)?;
        }
    }
    context
        .cultural_identity_created(city)
        .map_err(Error::Effect)?;
    context.check_production(city).map_err(Error::Effect)?;
    Ok(Some(city))
}
