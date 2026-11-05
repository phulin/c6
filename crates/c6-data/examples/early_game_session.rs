//! Typed embedding example. Run `cargo run -p c6-data --example early_game_session`
//! for its input contract. `cache_rules` and `play` are the embedding entry points:
//! the caller supplies an already initialized state and real stock turn services.
//! There is deliberately no fabricated game initializer or successful callback stub.
use c6_core::loaded_state::LoadedState;
use c6_data::{
    city_production_turn::{appeal, buildings, selection},
    loaded_actions::{
        choose_tech,
        session::{
            advance::{Request, TurnServices},
            initialized::{Initialized, Rejected},
            Session,
        },
        stock_notifications::{deactivation, research::CultureInputs, turn_loop},
    },
    research,
    resource_binding::ResourceAuthority,
    Rules,
};

/// Cache immutable definitions once from the caller's actual rules/configuration.
/// Current graphs, territory, visibility and modifier owners are not copied here.
pub struct CachedRules {
    pub research: research::Definitions,
    pub technologies: choose_tech::Definitions,
    pub buildings: buildings::Definitions,
    pub appeal: appeal::Definitions,
    pub selection: selection::Definitions,
}
pub fn cache_rules(
    state: &LoadedState,
    rules: &Rules,
    authority: &ResourceAuthority<'_>,
    start_era: u32,
    game_speed: u32,
) -> Result<CachedRules, String> {
    Ok(CachedRules {
        research: research::Definitions::from_rules(rules, authority, game_speed)
            .map_err(|e| format!("{e:?}"))?,
        technologies: choose_tech::Definitions::from_rules(state, rules, authority)
            .map_err(|e| e.to_string())?,
        buildings: buildings::Definitions::from_rules(
            state, rules, authority, start_era, game_speed,
        )
        .map_err(|e| e.to_string())?,
        appeal: appeal::Definitions::from_current_rules(state, rules).map_err(|e| e.to_string())?,
        selection: selection::Definitions::from_rules(state, rules).map_err(|e| e.to_string())?,
    })
}

/// Explicit policy inputs. The example never chooses a research index/building
/// for the player and never automatically ends another player's turn.
pub struct Choices {
    pub player: i32,
    pub city: u32,
    pub technology: i32,
    pub building_hash: u32,
}
pub struct Progress<'s, 'a> {
    pub session: Session<'s, 'a>,
    /// Any failure retains this Session, including its partially advanced state.
    pub result: Result<turn_loop::Outcome, String>,
}

/// Select research, select city production, then advance the SAME Session.
/// Supply a live active human City with a legal empty queue. Gameplay services
/// must be bound before construction (city queries, modifiers, scripts, etc.).
/// `TurnServices` must call the real public stock upkeep/control compositions;
/// missing services return their existing explicit error.
#[allow(clippy::too_many_arguments)]
pub fn play<'s, 'a, S: TurnServices<'a>>(
    state: &'s mut LoadedState,
    initialized: Initialized<'s, 'a>,
    rules: &CachedRules,
    choices: Choices,
    culture: Option<&CultureInputs<'_>>,
    selection_current: &selection::Current<'_>,
    selection_queries: &impl selection::Queries,
    turn_inputs: &deactivation::Inputs<'_, '_, '_>,
    services: &mut S,
    budget: &mut usize,
) -> Result<Progress<'s, 'a>, Box<Rejected<'s, 'a, c6_core::unit::OperationParameters>>> {
    let mut session = Session::from_initialized(state, initialized)?;
    let result = (|| {
        session
            .request_technology(
                choices.player,
                choices.technology,
                &rules.research,
                &rules.technologies,
                culture,
            )
            .map_err(|e| e.to_string())?;
        session
            .choose_building(
                choices.player,
                choices.city,
                choices.building_hash,
                &rules.buildings,
                &rules.appeal,
                &rules.selection,
                selection_current,
                selection_queries,
            )
            .map_err(|e| e.to_string())?;
        session
            .advance(
                Request::EndTurn {
                    player: choices.player,
                    reason: c6_core::turn_completion::NONE,
                },
                turn_inputs,
                services,
                budget,
            )
            .map_err(|e| e.to_string())
    })();
    // Inspect session.decisions() for the next policy/semantic decision. Resolve
    // it explicitly, then call advance(Resume) or advance_research as appropriate.
    // Continue using this session; do not reload state between requests.
    Ok(Progress { session, result })
}

fn main() {
    println!(
        "early_game_session: typed Rust embedding example\n\
        Supply initialized LoadedState, rules/resource authority, stock backend,\n\
        notification inputs and real TurnServices to cache_rules() and play().\n\
        play() selects research and building production, then advances one Session.\n\
        No private workspace, original engine, save loader or default game is used."
    );
}
