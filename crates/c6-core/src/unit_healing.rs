//! Linux XP2 per-unit end-turn healing amount calculation.
//!
//! This projects Rules::Units::Instance::HealUnit's current-state reads to its
//! positive ChangeDamage request. It does not change damage, deliver the native
//! healing signal, resolve world queries, or implement complete turn upkeep.
//! Inputs must come from one authoritative current state and configured rules;
//! unavailable provider values must not be replaced with defaults. Native
//! differential validation of this calculation remains outstanding.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingTerritory {
    Friendly,
    Neutral,
    Enemy,
    /// Any GetDiplomaticTerritory result other than 1, 2, or 3.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingDomain {
    /// Native GetDomain() == 0.
    Naval,
    /// Every other native domain value follows this branch.
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerritoryHealing {
    pub friendly: i32,
    pub neutral: i32,
    pub enemy: i32,
}

impl TerritoryHealing {
    fn at(self, territory: HealingTerritory) -> i32 {
        match territory {
            HealingTerritory::Friendly => self.friendly,
            HealingTerritory::Neutral => self.neutral,
            HealingTerritory::Enemy => self.enemy,
            HealingTerritory::Other => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingRules {
    /// COMBAT_HEAL_NAVAL_{FRIENDLY,NEUTRAL,ENEMY}.
    pub naval: TerritoryHealing,
    /// COMBAT_HEAL_LAND_{FRIENDLY,NEUTRAL,ENEMY}.
    pub land: TerritoryHealing,
    /// COMBAT_HEAL_CITY_GARRISON; replaces land friendly base on city plots.
    pub city_garrison: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealingResource {
    NoStrategicResource,
    Required {
        /// CanAccessResourceForUnitHealing(owner, definition resource index).
        accessible_for_healing: bool,
        /// GetResourceAmount(resource, ownership=0, final bool=true).
        resource_amount: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingImprovement {
    pub pillaged: bool,
    /// Actual improvement definition's ReligiousUnitHealRate.
    pub religious_heal_rate: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingGovernor {
    pub established: bool,
    pub extra_heal: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingCity {
    /// Religion::GetExtraReligiousHeal for the city purchasing this plot.
    pub extra_religious_heal: i32,
    /// Assigned governor of that city's owner, if any.
    pub assigned_governor: Option<HealingGovernor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HealingPlot {
    pub owned_by_unit_owner: bool,
    pub is_city: bool,
    pub improvement: Option<HealingImprovement>,
    pub purchased_by_city: Option<HealingCity>,
    /// Owner's Districts::GetReligionHealRate at this plot.
    pub district_religion_heal_rate: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitHealingState {
    pub eligible_to_heal: bool,
    pub delayed_death: bool,
    pub territory: HealingTerritory,
    pub no_foreign_heal: bool,
    pub no_neutral_heal: bool,
    pub resource: HealingResource,
    pub domain: HealingDomain,
    pub religious_strength: i32,
    pub embarked: bool,
    pub health: TerritoryHealing,
    pub religion_health: TerritoryHealing,
    pub plot: Option<HealingPlot>,
}

/// Requested healing, before damage clamping and synchronous native callbacks.
///
/// A positive result causes native ChangeDamage(-amount), followed by a signal
/// carrying both this requested amount and actual old_damage - new_damage.
/// Zero causes neither call. In particular, a full-health unit can still have
/// a positive request and therefore deliver callbacks. This function is pure
/// and consumes no RNG; it does not claim that those callbacks consume none.
pub fn healing_amount(unit: Option<&UnitHealingState>, rules: &HealingRules) -> i32 {
    let Some(unit) = unit else { return 0 };
    let Some(plot) = unit.plot else { return 0 };
    if !unit.eligible_to_heal || unit.delayed_death {
        return 0;
    }
    if let HealingResource::Required {
        accessible_for_healing: false,
        resource_amount,
    } = unit.resource
    {
        if resource_amount <= 0 {
            return 0;
        }
    }
    if (unit.territory == HealingTerritory::Enemy && unit.no_foreign_heal)
        || (unit.territory == HealingTerritory::Neutral && unit.no_neutral_heal)
    {
        return 0;
    }

    let amount = if unit.domain == HealingDomain::Naval {
        if unit.territory == HealingTerritory::Other {
            return 0;
        }
        rules
            .naval
            .at(unit.territory)
            .wrapping_add(unit.health.at(unit.territory))
    } else if unit.religious_strength > 0 {
        let improvement_rate = if plot.owned_by_unit_owner {
            plot.improvement
                .filter(|i| !i.pillaged)
                .map_or(0, |i| i.religious_heal_rate.max(0))
        } else {
            0
        };
        let base = plot.district_religion_heal_rate.max(improvement_rate);
        let amount = base.wrapping_add(unit.religion_health.at(unit.territory));
        if plot.owned_by_unit_owner {
            amount.wrapping_add(plot.purchased_by_city.map_or(0, |c| c.extra_religious_heal))
        } else {
            amount
        }
    } else {
        let base = match unit.territory {
            HealingTerritory::Friendly => {
                let base = if plot.is_city {
                    rules.city_garrison
                } else {
                    rules.land.friendly
                };
                base.wrapping_add(unit.health.friendly)
            }
            HealingTerritory::Enemy | HealingTerritory::Neutral if !unit.embarked => rules
                .land
                .at(unit.territory)
                .wrapping_add(unit.health.at(unit.territory)),
            _ => 0,
        };
        let governor = if plot.owned_by_unit_owner {
            plot.purchased_by_city
                .and_then(|c| c.assigned_governor)
                .filter(|g| g.established)
        } else {
            None
        };
        base.wrapping_add(governor.map_or(0, |g| g.extra_heal))
    };
    amount.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (UnitHealingState, HealingRules) {
        (
            UnitHealingState {
                eligible_to_heal: true,
                delayed_death: false,
                territory: HealingTerritory::Friendly,
                no_foreign_heal: false,
                no_neutral_heal: false,
                resource: HealingResource::NoStrategicResource,
                domain: HealingDomain::Other,
                religious_strength: 0,
                embarked: false,
                health: TerritoryHealing {
                    friendly: 3,
                    neutral: 5,
                    enemy: 7,
                },
                religion_health: TerritoryHealing {
                    friendly: 11,
                    neutral: 13,
                    enemy: 17,
                },
                plot: Some(HealingPlot {
                    owned_by_unit_owner: true,
                    is_city: false,
                    improvement: Some(HealingImprovement {
                        pillaged: false,
                        religious_heal_rate: 19,
                    }),
                    purchased_by_city: Some(HealingCity {
                        extra_religious_heal: 23,
                        assigned_governor: Some(HealingGovernor {
                            established: true,
                            extra_heal: 29,
                        }),
                    }),
                    district_religion_heal_rate: 31,
                }),
            },
            HealingRules {
                naval: TerritoryHealing {
                    friendly: 37,
                    neutral: 41,
                    enemy: 43,
                },
                land: TerritoryHealing {
                    friendly: 47,
                    neutral: 53,
                    enemy: 59,
                },
                city_garrison: 61,
            },
        )
    }

    #[test]
    fn naval_never_adds_city_governor_or_religion() {
        let (mut u, r) = fixture();
        u.domain = HealingDomain::Naval;
        u.religious_strength = 100;
        for (territory, expected) in [
            (HealingTerritory::Friendly, 40),
            (HealingTerritory::Neutral, 46),
            (HealingTerritory::Enemy, 50),
            (HealingTerritory::Other, 0),
        ] {
            u.territory = territory;
            assert_eq!(healing_amount(Some(&u), &r), expected);
        }
    }

    #[test]
    fn land_city_embark_and_governor_branches() {
        let (mut u, r) = fixture();
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.embarked = true;
        u.plot.as_mut().unwrap().is_city = true;
        assert_eq!(healing_amount(Some(&u), &r), 93);
        u.territory = HealingTerritory::Enemy;
        assert_eq!(healing_amount(Some(&u), &r), 29);
        u.plot.as_mut().unwrap().owned_by_unit_owner = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.embarked = false;
        assert_eq!(healing_amount(Some(&u), &r), 66);
    }

    #[test]
    fn religion_max_not_sum_and_independent_owned_city_bonus() {
        let (mut u, r) = fixture();
        u.religious_strength = 1;
        assert_eq!(healing_amount(Some(&u), &r), 65);
        u.plot.as_mut().unwrap().district_religion_heal_rate = -5;
        assert_eq!(healing_amount(Some(&u), &r), 53);
        u.plot
            .as_mut()
            .unwrap()
            .improvement
            .as_mut()
            .unwrap()
            .pillaged = true;
        assert_eq!(healing_amount(Some(&u), &r), 34);
        u.plot.as_mut().unwrap().owned_by_unit_owner = false;
        assert_eq!(healing_amount(Some(&u), &r), 11);
        u.territory = HealingTerritory::Other;
        assert_eq!(healing_amount(Some(&u), &r), 0);
    }

    #[test]
    fn gates_and_strategic_resource_fallback() {
        let (mut u, r) = fixture();
        assert_eq!(healing_amount(None, &r), 0);
        u.resource = HealingResource::Required {
            accessible_for_healing: false,
            resource_amount: 0,
        };
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.resource = HealingResource::Required {
            accessible_for_healing: true,
            resource_amount: -1,
        };
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.resource = HealingResource::Required {
            accessible_for_healing: false,
            resource_amount: 1,
        };
        assert_eq!(healing_amount(Some(&u), &r), 79);
        u.no_foreign_heal = true;
        u.territory = HealingTerritory::Enemy;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Neutral;
        u.no_neutral_heal = true;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Friendly;
        u.delayed_death = true;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.delayed_death = false;
        u.eligible_to_heal = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.eligible_to_heal = true;
        u.plot = None;
        assert_eq!(healing_amount(Some(&u), &r), 0);
    }

    #[test]
    fn religious_territories_and_governor_establishment() {
        let (mut u, r) = fixture();
        u.religious_strength = 1;
        for (territory, expected) in [
            (HealingTerritory::Friendly, 65),
            (HealingTerritory::Neutral, 67),
            (HealingTerritory::Enemy, 71),
            (HealingTerritory::Other, 54),
        ] {
            u.territory = territory;
            assert_eq!(healing_amount(Some(&u), &r), expected);
        }
        u.religious_strength = 0;
        // Unknown territory still reaches the owned-land governor branch.
        assert_eq!(healing_amount(Some(&u), &r), 29);
        u.plot
            .as_mut()
            .unwrap()
            .purchased_by_city
            .as_mut()
            .unwrap()
            .assigned_governor
            .as_mut()
            .unwrap()
            .established = false;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        u.territory = HealingTerritory::Neutral;
        assert_eq!(healing_amount(Some(&u), &r), 58);
        u.plot.as_mut().unwrap().purchased_by_city = None;
        assert_eq!(healing_amount(Some(&u), &r), 58);
    }

    #[test]
    fn wrapping_arithmetic_precedes_positive_request_gate() {
        let (mut u, mut r) = fixture();
        u.domain = HealingDomain::Naval;
        r.naval.friendly = i32::MAX;
        assert_eq!(healing_amount(Some(&u), &r), 0);
        r.naval.friendly = i32::MIN;
        u.health.friendly = -1;
        assert_eq!(healing_amount(Some(&u), &r), i32::MAX);
    }
}
