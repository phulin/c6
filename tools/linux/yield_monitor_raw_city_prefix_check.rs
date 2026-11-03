//! Original raw-input probe, stopping after first Player.Cities before Units.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read(std::env::args_os().nth(1).ok_or("raw input")?)?;
    let map_limits = c6_core::map::Limits {
        max_bytes: 256 * 1024 * 1024,
        max_items: 64 * 1024 * 1024,
        ..Default::default()
    };
    let core = c6_core::core_stream::CorePrefix::decode(&raw, map_limits)?;
    let limits = c6_core::unit::Limits {
        max_bytes: 256 * 1024 * 1024,
        max_items: 64 * 1024 * 1024,
    };
    let result = c6_core::core_player::diagnostic_first_city_prefix(core.tail, limits);
    match result {
        Ok((players, owner, cities, bytes)) => {
            println!(
                "{}",
                serde_json::json!({
                    "schema":"c6-linux-original-first-city-prefix-v1",
                    "original_code_executed":false,
                    "player_count":players,
                    "first_player_owner":owner,
                    "first_player_cities":cities.len(),
                    "force_update_plot_appeal":cities.iter().filter(|city| city.2).count(),
                    "city_identities":cities.iter().map(|city|serde_json::json!([city.0,city.1])).collect::<Vec<_>>(),
                    "map_end":core.map_end,
                    "bounded_prefix_end":core.map_end+bytes,
                    "boundary":"first Player.Cities end, before Diplomacy and Units",
                    "all_player_city_inputs_supported":false,
                    "cache_freshness_proven":false
                })
            );
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({
                    "schema":"c6-linux-original-first-city-prefix-v1",
                    "original_code_executed":false,
                    "map_end":core.map_end,
                    "bounded_first_city_prefix_error":format!("{error:?}"),
                    "all_player_city_inputs_supported":false,
                    "cache_freshness_proven":false
                })
            );
        }
    }
    Ok(())
}
