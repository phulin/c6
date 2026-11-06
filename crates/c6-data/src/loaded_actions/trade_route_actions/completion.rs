//! Linux XP2 Trade.Manager.ExpireRoute2c06870, over the canonical current graph.
//! Ordered subscribers and the manager's cleanup tail execute on these same
//! owners. An error retains the graph/post writes already reached by native.
use super::{ComponentId, Error};
use c6_core::{loaded_state::LoadedState, trade::Route};

pub trait Effects {
    type Error;
    fn trading_post_to_be_created(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn trading_post_changed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn trading_post_notification(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn cleanup_removed_route(
        &mut self,
        state: &mut LoadedState,
        route: &Route,
        origin: ComponentId,
        destination: ComponentId,
    ) -> Result<(), Self::Error>;
}
fn construct_post<E: Effects>(
    state: &mut LoadedState,
    city: ComponentId,
    owner: i32,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    // HasInactiveTradingPost is exactly the stored bit when HasActive is false.
    // Their disjunction therefore equals the constructed bit, including bounds.
    let posts = &state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)?
        .trade
        .constructed_trading_posts;
    if usize::try_from(owner)
        .ok()
        .and_then(|i| posts.get(i))
        .copied()
        .unwrap_or(false)
    {
        return Ok(());
    }
    effects
        .trading_post_to_be_created(state, city, owner)
        .map_err(Error::Effect)?;
    // Reacquire after synchronous subscribers: the original setter also reads
    // current storage and dispatches Changed only if its stored bit changes.
    if let Ok(index) = usize::try_from(owner) {
        let record = state
            .city_mut(i32::from(city.owner), city.id)
            .map_err(Error::State)?;
        let changed = if let Some(bit) = record.trade.constructed_trading_posts.get_mut(index) {
            let changed = !*bit;
            *bit = true;
            changed
        } else {
            false
        };
        if changed {
            effects
                .trading_post_changed(state, city, owner)
                .map_err(Error::Effect)?;
        }
    }
    effects
        .trading_post_notification(state, city, owner)
        .map_err(Error::Effect)
}
/// Remove the existing route, construct absent trading posts in origin then
/// destination order, and execute the actual cleanup tail. Missing cities keep
/// the removal prefix and skip the tail, matching original CManager lookups.
pub fn expire<E: Effects>(
    state: &mut LoadedState,
    origin: ComponentId,
    destination: ComponentId,
    effects: &mut E,
) -> Result<(), Error<E::Error>> {
    let route = state
        .trade_graph_mut()
        .map_err(Error::State)?
        .remove_route_by_city_pair(origin, destination);
    let Some(route) = route else {
        return Ok(());
    };
    let exists = |city: ComponentId| -> Result<bool, Error<E::Error>> {
        match state.city(i32::from(city.owner), city.id) {
            Ok(_) => Ok(true),
            Err(c6_core::loaded_state::Error::Missing(
                c6_core::loaded_state::MissingInput::City { .. },
            )) => Ok(false),
            Err(error) => Err(Error::State(error)),
        }
    };
    let origin_exists = exists(origin)?;
    let destination_exists = exists(destination)?;
    if !origin_exists || !destination_exists {
        return Ok(());
    }
    construct_post(state, origin, i32::from(origin.owner), effects)?;
    construct_post(state, destination, i32::from(origin.owner), effects)?;
    effects
        .cleanup_removed_route(state, &route, origin, destination)
        .map_err(Error::Effect)
}
