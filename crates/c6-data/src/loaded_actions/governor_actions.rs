//! Explicit first ordinary governor appointment and assignment, followed by
//! normal-turn establishment. Other governors and promotions remain unsupported.
use crate::{
    game_parameters::sql_integer, registry::linux_type_hash,
    unit_initialization_definitions::collection, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding, governor_lifecycle as kernel,
    loaded_state::LoadedState, player_governors::CityId, unit::ComponentId,
};

pub mod stock;

pub const EDUCATOR: &[u8] = b"GOVERNOR_THE_EDUCATOR";
pub const LIBRARIAN: &[u8] = b"GOVERNOR_PROMOTION_EDUCATOR_LIBRARIAN";
#[derive(Debug)]
pub enum Error<E = String> {
    State(c6_core::loaded_state::Error),
    Unsupported(&'static str),
    Rules(String),
    Allocation(c6_core::city_initialization::Error),
    Effect(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "governor action: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
#[derive(Clone, Debug)]
pub struct YieldRecipe {
    pub id: Vec<u8>,
    pub yield_hash: u32,
    pub amount: i32,
}
pub struct Definitions {
    binding: WireBinding,
    pub governor_hash: u32,
    pub governor_index: i32,
    pub base_promotion_hash: u32,
    pub base_promotion_index: i32,
    pub name: Vec<u8>,
    pub identity_pressure: i32,
    pub turns_to_establish: i32,
    pub promotion_hashes: Vec<u32>,
    pub recipes: Vec<YieldRecipe>,
    maximum_appointments: i32,
    cannot_assign: bool,
    governor_hashes: Vec<u32>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let fail = |e: crate::unit_initialization_definitions::Error| Error::Rules(e.to_string());
        let registry = collection(rules, "Governors", "GovernorType").map_err(fail)?;
        let promotions =
            collection(rules, "GovernorPromotions", "GovernorPromotionType").map_err(fail)?;
        let db = rules.gameplay();
        let table = |name| {
            db.table(name)
                .ok_or(Error::Unsupported("governor rules table"))
        };
        if table("SecretSocieties")?.rows().next().is_some() {
            return Err(Error::Unsupported("secret society governor callbacks"));
        }
        if table("GovernorModifiers")?
            .rows()
            .any(|r| r.text("GovernorType").ok().map(str::as_bytes) == Some(EDUCATOR))
        {
            return Err(Error::Unsupported("additional Pingala governor modifiers"));
        }
        if table("GovernorReplaces")?.rows().any(|r| {
            r.text("UniqueGovernorType").ok().map(str::as_bytes) == Some(EDUCATOR)
                || r.text("ReplacesGovernorType").ok().map(str::as_bytes) == Some(EDUCATOR)
        }) {
            return Err(Error::Unsupported("Pingala replacement trait eligibility"));
        }
        let governor_hash = linux_type_hash(EDUCATOR);
        let base_promotion_hash = linux_type_hash(LIBRARIAN);
        let row = table("Governors")?
            .rows()
            .find(|r| r.text("GovernorType").ok().map(str::as_bytes) == Some(EDUCATOR))
            .ok_or(Error::Unsupported("Pingala definition"))?;
        if row
            .get("TraitType")
            .ok()
            .is_some_and(|v| !matches!(v, crate::SqlValue::Null))
            || row
                .integer("AssignCityState")
                .map_err(|e| Error::Rules(e.to_string()))?
                != 0
        {
            return Err(Error::Unsupported("modified Pingala eligibility"));
        }
        let number = |r: crate::Row<'_>, name| {
            r.get(name)
                .ok()
                .and_then(sql_integer)
                .ok_or(Error::Unsupported("governor integer rule"))
        };
        let parameter = |name| {
            table("GlobalParameters")?
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .ok_or(Error::Unsupported("governor global parameter"))
                .and_then(|r| number(r, "Value"))
        };
        if table("Governors_XP2")?
            .rows()
            .find(|r| r.text("GovernorType").ok().map(str::as_bytes) == Some(EDUCATOR))
            .and_then(|r| number(r, "AssignToMajor").ok())
            .is_some_and(|v| v != 0)
        {
            return Err(Error::Unsupported("Pingala assign to major civilization"));
        }
        let strength = number(row, "TransitionStrength")?;
        if strength == 0 {
            return Err(Error::Unsupported("zero governor transition strength"));
        }
        let turns_to_establish = parameter("GOVERNOR_BASE_TURNS_TO_ESTABLISH")?
            .wrapping_mul(100)
            .checked_div(strength)
            .ok_or(Error::Unsupported("governor establish division"))?;
        let sets = table("GovernorPromotionSets")?;
        let mut ordered = sets
            .rows()
            .filter(|r| r.text("GovernorType").ok().map(str::as_bytes) == Some(EDUCATOR))
            .map(|r| {
                r.text("GovernorPromotion")
                    .map(str::as_bytes)
                    .map(<[u8]>::to_vec)
                    .map_err(|e| Error::Rules(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // The reference collection is ordered by the promotion definition rowid.
        ordered.sort_by_key(|n| {
            promotions
                .find_index(linux_type_hash(n))
                .unwrap_or(usize::MAX)
        });
        let bases = ordered
            .iter()
            .filter(|n| {
                table("GovernorPromotions")
                    .ok()
                    .and_then(|t| {
                        t.rows().find(|r| {
                            r.text("GovernorPromotionType").ok().map(str::as_bytes)
                                == Some(n.as_slice())
                        })
                    })
                    .and_then(|r| number(r, "BaseAbility").ok())
                    == Some(1)
            })
            .collect::<Vec<_>>();
        if bases.len() != 1 || bases[0].as_slice() != LIBRARIAN {
            return Err(Error::Unsupported("Pingala base promotion set"));
        }
        let mut ids = table("GovernorPromotionModifiers")?
            .rows()
            .filter(|r| r.text("GovernorPromotionType").ok().map(str::as_bytes) == Some(LIBRARIAN))
            .map(|r| {
                Ok((
                    r.row_id()
                        .ok_or(Error::Unsupported("governor modifier intrinsic rowid"))?
                        .0,
                    r.text("ModifierId")
                        .map_err(|e| Error::Rules(e.to_string()))?
                        .as_bytes()
                        .to_vec(),
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        // GovernorPromotion's reference builder preserves intrinsic join-row
        // order. Native Librarian allocates science before culture; the
        // affected-owner vector separately sorts by modifier definition name.
        ids.sort_by_key(|r| r.0);
        let mut recipes = vec![];
        for (_, id) in ids {
            let m = table("Modifiers")?
                .rows()
                .find(|r| r.text("ModifierId").ok().map(str::as_bytes) == Some(id.as_slice()))
                .ok_or(Error::Unsupported("governor Modifier"))?;
            for flag in [
                "RunOnce",
                "NewOnly",
                "Permanent",
                "Repeatable",
                "OwnerStackLimit",
                "SubjectStackLimit",
            ] {
                if m.get(flag).ok().is_some_and(|v| {
                    !matches!(v, crate::SqlValue::Null) && sql_integer(v) != Some(0)
                }) {
                    return Err(Error::Unsupported("governor modifier flags"));
                }
            }
            for flag in ["OwnerRequirementSetId", "SubjectRequirementSetId"] {
                if m.text(flag).ok().is_some_and(|v| !v.is_empty()) {
                    return Err(Error::Unsupported("governor modifier requirements"));
                }
            }
            let dynamic = table("DynamicModifiers")?
                .rows()
                .find(|r| r.text("ModifierType").ok() == m.text("ModifierType").ok())
                .ok_or(Error::Unsupported("governor dynamic modifier"))?;
            if dynamic.text("CollectionType").ok() != Some("COLLECTION_OWNER")
                || dynamic.text("EffectType").ok() != Some("EFFECT_ADJUST_CITY_YIELD_MODIFIER")
            {
                return Err(Error::Unsupported("governor base effect"));
            }
            let mut yield_hash = None;
            let mut amount = 0;
            for a in table("ModifierArguments")?
                .rows()
                .filter(|r| r.text("ModifierId").ok().map(str::as_bytes) == Some(id.as_slice()))
            {
                if a.text("Type").ok() != Some("ARGTYPE_IDENTITY") {
                    return Err(Error::Unsupported("governor dynamic argument"));
                }
                match a.text("Name").ok() {
                    Some("YieldType") => {
                        yield_hash = Some(linux_type_hash(
                            a.text("Value")
                                .map_err(|e| Error::Rules(e.to_string()))?
                                .as_bytes(),
                        ))
                    }
                    Some("Amount") => amount = number(a, "Value")?,
                    _ => return Err(Error::Unsupported("governor effect argument")),
                }
            }
            recipes.push(YieldRecipe {
                id,
                yield_hash: yield_hash.ok_or(Error::Unsupported("governor yield type"))?,
                amount,
            });
        }
        if recipes.len() != 2 {
            return Err(Error::Unsupported("Pingala base modifier count"));
        }
        Ok(Self {
            binding: state.binding().clone(),
            governor_hash,
            governor_index: registry
                .find_index(governor_hash)
                .ok_or(Error::Unsupported("Pingala index"))? as i32,
            base_promotion_hash,
            base_promotion_index: promotions
                .find_index(base_promotion_hash)
                .ok_or(Error::Unsupported("Librarian index"))?
                as i32,
            name: row
                .text("Name")
                .map_err(|e| Error::Rules(e.to_string()))?
                .as_bytes()
                .to_vec(),
            identity_pressure: number(row, "IdentityPressure")?,
            turns_to_establish,
            promotion_hashes: promotions
                .entries()
                .iter()
                .map(|e| linux_type_hash(&e.type_name))
                .collect(),
            recipes,
            maximum_appointments: parameter("MAX_GOVERNOR_APPOINTMENTS")?,
            cannot_assign: table("GovernorsCannotAssign")?
                .rows()
                .filter(|r| r.text("GovernorType").ok().map(str::as_bytes) == Some(EDUCATOR))
                .try_fold(false, |value, r| {
                    Ok(value || number(r, "CannotAssign")? != 0)
                })?,
            governor_hashes: registry
                .entries()
                .iter()
                .map(|e| linux_type_hash(&e.type_name))
                .collect(),
        })
    }
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if state.binding() == &self.binding {
            Ok(())
        } else {
            Err(Error::Unsupported("governor rules binding"))
        }
    }
}
fn governors<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::player_governors::Governors, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .map(|p| &p.governors)
        .ok_or(Error::Unsupported("governor owner"))
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    /// The caller may appoint the supported Pingala or defer the title. The
    /// simulator does not claim that other native governor types are supported.
    FirstAppointment { owner: i32, supported_governor: u32 },
    Assignment {
        owner: i32,
        governor: u32,
        cities: Vec<ComponentId>,
    },
}
pub fn pending(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
) -> Result<Option<Decision>, Error> {
    defs.check(state)?;
    let g = governors(state, owner)?;
    if g.entries.is_empty()
        && !g.title_considered
        && can_appoint::<String>(state, defs, owner, defs.governor_hash)?
    {
        return Ok(Some(Decision::FirstAppointment {
            owner,
            supported_governor: defs.governor_hash,
        }));
    }
    if g.entries.iter().any(|s| {
        s.governor.definition_hash == defs.governor_hash && s.governor.assigned_city.owner == -1
    }) {
        let candidates = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .unwrap()
            .cities
            .records
            .entries
            .iter()
            .map(|s| ComponentId {
                owner: owner as i16,
                component_type: 2,
                id: s.value.header.id,
            })
            .collect::<Vec<_>>();
        let mut cities = Vec::new();
        for city in candidates {
            if can_assign::<String>(state, defs, owner, defs.governor_hash, city)? {
                cities.push(city);
            }
        }
        return Ok(Some(Decision::Assignment {
            owner,
            governor: defs.governor_hash,
            cities,
        }));
    }
    Ok(None)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Promoted {
        owner: i32,
        governor: i32,
        promotion: i32,
    },
    Appointed {
        owner: i32,
        governor: i32,
    },
    CityGovernorChanged {
        city: ComponentId,
    },
    Assigned {
        owner: i32,
        governor: i32,
        city: ComponentId,
    },
    Established {
        owner: i32,
        governor: i32,
        city: ComponentId,
    },
    Turn {
        owner: i32,
        governor: i32,
    },
}
pub trait Effects {
    type Error;
    fn preflight(
        &mut self,
        state: &LoadedState,
        defs: &Definitions,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn event(
        &mut self,
        state: &mut LoadedState,
        defs: &Definitions,
        event: Event,
    ) -> Result<(), Self::Error>;
}
pub fn can_appoint<E>(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    hash: u32,
) -> Result<bool, Error<E>> {
    defs.check(state)?;
    if hash != defs.governor_hash {
        return Err(Error::Unsupported("governor appointment type"));
    }
    let g = governors(state, owner)?;
    let owned = defs
        .governor_hashes
        .iter()
        .filter(|&&h| g.entries.iter().any(|s| s.governor.definition_hash == h))
        .count();
    Ok(hash == defs.governor_hash
        && g.governor_points > kernel::points_spent(g)
        && owned as i32 <= defs.maximum_appointments
        && owned < defs.governor_hashes.len()
        && !g.entries.iter().any(|s| s.governor.definition_hash == hash))
}
pub fn appoint<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    hash: u32,
    e: &mut E,
) -> Result<bool, Error<E::Error>> {
    if !can_appoint(state, defs, owner, hash)? {
        return Ok(false);
    }
    if !governors::<E::Error>(state, owner)?.entries.is_empty() {
        return Err(Error::Unsupported("appointment after first governor"));
    }
    if governors::<E::Error>(state, owner)?.favored_governor_hash == hash
        && governors::<E::Error>(state, owner)?.diplomatic_favor != 0
    {
        return Err(Error::Unsupported("target governor diplomatic favor"));
    }
    e.preflight(state, defs, owner).map_err(Error::Effect)?;
    let id = kernel::appoint(
        state.governors_mut(owner).map_err(Error::State)?,
        owner,
        &kernel::Appointment {
            definition_hash: hash,
            name: &defs.name,
            identity_pressure: defs.identity_pressure,
            govern_city_state: false,
            assign_to_major: false,
            promotions: &defs.promotion_hashes,
        },
    )
    .map_err(Error::Allocation)?;
    let Some(id) = id else {
        return Ok(false);
    };
    state
        .governors_mut(owner)
        .map_err(Error::State)?
        .entries
        .iter_mut()
        .find(|s| s.governor.id == id)
        .unwrap()
        .governor
        .promotions
        .iter_mut()
        .find(|p| p.definition_hash == defs.base_promotion_hash)
        .unwrap()
        .earned = true;
    e.event(
        state,
        defs,
        Event::Promoted {
            owner,
            governor: defs.governor_index,
            promotion: defs.base_promotion_index,
        },
    )
    .map_err(Error::Effect)?;
    e.event(
        state,
        defs,
        Event::Appointed {
            owner,
            governor: defs.governor_index,
        },
    )
    .map_err(Error::Effect)?;
    state
        .set_governor_title_considered(owner, false)
        .map_err(Error::State)?;
    Ok(true)
}
pub fn can_assign<E>(
    state: &LoadedState,
    defs: &Definitions,
    owner: i32,
    hash: u32,
    city: ComponentId,
) -> Result<bool, Error<E>> {
    defs.check(state)?;
    if hash != defs.governor_hash {
        return Err(Error::Unsupported("governor assignment type"));
    }
    if defs.cannot_assign || i32::from(city.owner) != owner || city.component_type != 2 {
        return Ok(false);
    }
    let Ok(current_city) = state.city(owner, city.id) else {
        return Ok(false);
    };
    let (plot, _) = c6_core::movement_world::plot_at(
        state.world().map(),
        c6_core::unit::PlotCoord {
            x: current_city.header.location_x,
            y: current_city.header.location_y,
        },
    )
    .map_err(|_| Error::Unsupported("governor destination plot"))?;
    match state
        .world()
        .visibility(owner)
        .and_then(|v| v.revealed.get(plot))
    {
        Some(0) => return Ok(false),
        Some(1) => {}
        _ => return Err(Error::Unsupported("current governor city reveal byte")),
    }
    let Some(g) = governors(state, owner)?
        .entries
        .iter()
        .find(|s| s.governor.definition_hash == hash)
        .map(|s| &s.governor)
    else {
        return Ok(false);
    };
    Ok(g.neutralized_turns < 1
        && !kernel::same_city(
            &g.assigned_city,
            &CityId {
                owner: city.owner,
                component_type: 2,
                id: city.id,
            },
        ))
}
pub fn assign<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    hash: u32,
    city: ComponentId,
    e: &mut E,
) -> Result<bool, Error<E::Error>> {
    if !can_assign(state, defs, owner, hash, city)? {
        return Ok(false);
    }
    if governors::<E::Error>(state, owner)?
        .entries
        .iter()
        .any(|s| s.governor.assigned_city.owner != -1)
    {
        return Err(Error::Unsupported("governor reassignment/removal"));
    }
    e.preflight(state, defs, owner).map_err(Error::Effect)?;
    let g = &mut state
        .governors_mut(owner)
        .map_err(Error::State)?
        .entries
        .iter_mut()
        .find(|s| s.governor.definition_hash == hash)
        .unwrap()
        .governor;
    g.assigned_city = CityId {
        owner: city.owner,
        component_type: 2,
        id: city.id,
    };
    g.established = false;
    if kernel::turns_on_site(g, &g.assigned_city) >= defs.turns_to_establish {
        g.established = true;
        e.event(
            state,
            defs,
            Event::Established {
                owner,
                governor: defs.governor_index,
                city,
            },
        )
        .map_err(Error::Effect)?;
    }
    e.event(state, defs, Event::CityGovernorChanged { city })
        .map_err(Error::Effect)?;
    e.event(
        state,
        defs,
        Event::Assigned {
            owner,
            governor: defs.governor_index,
            city,
        },
    )
    .map_err(Error::Effect)?;
    Ok(true)
}
pub fn do_turn<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    owner: i32,
    e: &mut E,
) -> Result<(), Error<E::Error>> {
    defs.check(state)?;
    let ids = governors::<E::Error>(state, owner)?
        .entries
        .iter()
        .map(|s| s.governor.id)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(());
    }
    e.preflight(state, defs, owner).map_err(Error::Effect)?;
    for id in ids {
        let g = governors::<E::Error>(state, owner)?
            .entries
            .iter()
            .find(|s| s.governor.id == id)
            .unwrap();
        if g.governor.definition_hash != defs.governor_hash
            || g.governor.neutralized_turns != 0
            || g.governor.neutralized_indefinitely != 0
        {
            return Err(Error::Unsupported("governor type or neutralization turn"));
        }
        let g = &mut state
            .governors_mut(owner)
            .map_err(Error::State)?
            .entries
            .iter_mut()
            .find(|s| s.governor.id == id)
            .unwrap()
            .governor;
        let city = g.assigned_city.clone();
        if kernel::edit_turns_on_site(g, city.clone(), 1, defs.turns_to_establish) {
            g.established = true;
            e.event(
                state,
                defs,
                Event::Established {
                    owner,
                    governor: defs.governor_index,
                    city: ComponentId {
                        owner: city.owner,
                        component_type: city.component_type,
                        id: city.id,
                    },
                },
            )
            .map_err(Error::Effect)?;
        }
        e.event(
            state,
            defs,
            Event::Turn {
                owner,
                governor: defs.governor_index,
            },
        )
        .map_err(Error::Effect)?;
    }
    Ok(())
}
