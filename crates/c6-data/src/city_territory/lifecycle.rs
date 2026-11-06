//! Bounded PlotYieldData destruction with an authenticated empty signal.
use c6_core::city_territory::plot_yields::Object;
/// The shared pool has already removed this object. Destruction has no further
/// gameplay work only when its actual signal and both modifier domains are
/// empty. Positive subscribers or modifiers require their real continuations.
pub fn destroy_unsubscribed(
    object: &Object,
    destroyed_subscribers: usize,
) -> Result<(), &'static str> {
    if destroyed_subscribers != 0 {
        return Err("PlotYields destruction subscribers");
    }
    if object
        .owned_modifiers
        .as_ref()
        .is_none_or(|v| !v.is_empty())
    {
        return Err("PlotYields owned modifier destruction");
    }
    if object
        .affected_modifiers
        .as_ref()
        .is_none_or(|v| !v.entries().is_empty())
    {
        return Err("PlotYields affected modifier destruction");
    }
    Ok(())
}
