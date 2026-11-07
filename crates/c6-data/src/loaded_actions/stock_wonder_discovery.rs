//! Reached natural-wonder discovery subscribers in retained native order.
//! Current player type, era rates, feature flags, research and history owners
//! determine each branch. Unknown positive continuations remain explicit.
use super::{movement::Error, Runtime};
use crate::Rules;
use c6_core::{
    game_history::{Moment, MomentData, MOMENT_VERSION},
    loaded_state::LoadedState,
    unit::PlotCoord,
};

#[derive(Clone, Copy, Debug)]
pub enum Subscriber {
    SummaryMajorsOnly,
    Gossip,
    EraScore,
    History,
    Boost { row: i64 },
}
#[derive(Clone, Copy)]
enum Current {
    SummaryMajorsOnly,
    Gossip,
    EraScore,
    History,
    Boost { index: usize },
}
pub(super) struct Bindings {
    order: Vec<Current>,
    ever_alive_majors: Vec<Option<i32>>,
    pub(super) unit_wonder_empty: bool,
}
impl<P> Runtime<'_, P> {
    /// Complete actual GameSignals+678 order and its WasEverAlive-major pointer
    /// roster. These are current registrations/identities, never callback answers.
    pub fn bind_wonder_discovery(
        &mut self,
        state: &LoadedState,
        rules: &Rules,
        order: Vec<Subscriber>,
        ever_alive_majors: Vec<Option<i32>>,
        unit_wonder_empty: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("wonder discovery identity"))?;
        let boosts = rules
            .gameplay()
            .table("Boosts")
            .ok_or(Error::Missing("wonder boost definitions"))?;
        let order = order
            .into_iter()
            .map(|subscriber| {
                Ok(match subscriber {
                    Subscriber::SummaryMajorsOnly => Current::SummaryMajorsOnly,
                    Subscriber::Gossip => Current::Gossip,
                    Subscriber::EraScore => Current::EraScore,
                    Subscriber::History => Current::History,
                    Subscriber::Boost { row } => Current::Boost {
                        index: boosts
                            .rows()
                            .enumerate()
                            .find(|(_, r)| r.row_id().is_some_and(|id| id.0 == row))
                            .ok_or(Error::Missing("wonder boost row"))?
                            .0,
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let current = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("wonder sight owner"))?;
        if current.wonders.is_some() {
            return Err(Error::Missing("wonder discovery already bound"));
        }
        current.wonders = Some(Bindings {
            order,
            ever_alive_majors,
            unit_wonder_empty,
        });
        Ok(())
    }
    pub(super) fn wonder_discovered(
        &mut self,
        state: &mut LoadedState,
        player: i32,
        feature: i32,
    ) -> Result<(), Error> {
        let binding = self
            .resource_revelation
            .as_ref()
            .and_then(|r| r.wonders.as_ref())
            .ok_or(Error::Missing(
                "current natural wonder signal registrations",
            ))?;
        let order = binding.order.clone();
        let ever_alive = binding.ever_alive_majors.clone();
        for subscriber in order {
            match subscriber {
                Current::SummaryMajorsOnly | Current::Gossip => {
                    // GeneralPlayerDataProvider's actual include flags select
                    // majors only; NaturalWonderFound explicitly tests IsMajor.
                    let level = self
                        .movement()?
                        .civilization_levels
                        .iter()
                        .find(|r| r.0 == player)
                        .ok_or(Error::Missing("wonder current civilization level"))?
                        .1;
                    if level == 0x253718b0 {
                        return Err(Error::Missing(
                            "major wonder summary or gossip continuation",
                        ));
                    }
                }
                Current::EraScore => {
                    // Eras::$_11 has no major guard. Read its actual current
                    // +4c score rate even for a trivial/nonmajor player.
                    let rate = state
                        .game_era_discovery_rates(player)
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        .map_or(0, |r| r[1]);
                    if rate != 0 {
                        return Err(Error::Missing("positive wonder era score continuation"));
                    }
                }
                Current::History => {
                    let features = self.movement()?.definitions.features;
                    let mut other_found = false;
                    for other in ever_alive
                        .iter()
                        .flatten()
                        .copied()
                        .filter(|p| *p != player)
                    {
                        if crate::unit_sight_state::revealed_feature::get(
                            state, features, other, feature,
                        )
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        {
                            other_found = true;
                            break;
                        }
                    }
                    // OnFindWonder constructs this actual temporary Moment
                    // before HandleMoment applies its capability/major guard.
                    let moment = Moment {
                        wire_version: MOMENT_VERSION,
                        type_hash: if other_found { 0xb0ded3d6 } else { 0xbc50184b },
                        turn: -1,
                        action_plot: PlotCoord { x: -9999, y: -9999 },
                        acting_player_ordinal: player,
                        id: -1,
                        has_ever_been_commemorated_raw: 0,
                        game_era_hash: u32::MAX,
                        extra_data: vec![MomentData {
                            kind_raw: 0x536178f1,
                            value: feature,
                        }],
                        city_name_bytes: vec![],
                        unit_citizen_name_bytes: vec![],
                        other_city_name_bytes: vec![],
                        era_score: 0,
                    };
                    let history = self
                        .unit_creation
                        .as_ref()
                        .and_then(|u| u.history.as_ref())
                        .ok_or(Error::Missing("wonder canonical history owner"))?;
                    if history
                        .inputs
                        .civilization_levels
                        .iter()
                        .find(|r| r.0 == player)
                        .ok_or(Error::Missing("wonder history player level"))?
                        .1
                        == 0x253718b0
                    {
                        return Err(Error::Missing("major natural wonder history continuation"));
                    }
                    let mut effects =
                        crate::loaded_actions::government_actions::history::StockEffects {
                            recorded: history.recorded,
                            reports: Vec::new(),
                        };
                    crate::melee_attack::resolve::completion::history::handle_moment(
                        state,
                        history.definitions,
                        history.inputs,
                        moment,
                        &mut effects,
                    )
                    .map_err(|e| Error::Requirement(format!("wonder history: {e:?}")))?;
                }
                Current::Boost { index } => {
                    // Canonical TriggerBoost reads logging, then current scratch
                    // and barbarian state before any research/telemetry write.
                    self.action_research_boost(state, player, index)
                        .map_err(|e| Error::Requirement(format!("wonder boost: {e:?}")))?;
                }
            }
        }
        self.reached
            .push("stock natural wonder discovery subscribers");
        Ok(())
    }
}
