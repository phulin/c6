//! Bounded Linux World Congress definition-driven option callbacks.
//!
//! Callers supply the exact GameInfo iteration order, resolved resource lookups,
//! scalar types, and hash bit patterns for their rules configuration. No database
//! bridge, callback dispatcher, resolution execution, or initialized game is
//! provided. These source-backed predicates have synthetic regression coverage;
//! they do not establish native parity or complete state/action/turn support.
//!
//! Each method's borrowed slice represents assignment of ResolutionOptions to
//! the corresponding global cached list. The source aliases that list rather
//! than copying it. Public cache fields permit explicit seeds and subsequent
//! mutation; callers tracking longer-lived aliases must refer back to that
//! same cache field. This API does not implement arbitrary Lua table aliasing.

/// Hash bits supplied by GameInfo; this module does not calculate hashes.
pub type OptionHash = u32;

/// Scalar values at this boundary retain Lua type and truth semantics.
///
/// Only nil and false are false in Lua: numeric zero and empty text are true.
/// Tables, functions, userdata, and arbitrary script execution are unsupported.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LuaScalar<'a> {
    Nil,
    Boolean(bool),
    Number(f64),
    String(&'a str),
}

impl LuaScalar<'_> {
    fn truthy(self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }

    fn equals_true(self) -> bool {
        matches!(self, Self::Boolean(true))
    }

    fn equals_zero(self) -> bool {
        matches!(self, Self::Number(n) if n == 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceRow<'a> {
    pub hash: OptionHash,
    pub power_provided: LuaScalar<'a>,
}

/// One Resource_Consumption row and its already-resolved Resources lookup.
/// Missing keyed resources must be represented by None, never a default row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceConsumptionRow<'a> {
    pub resource: Option<ResourceRow<'a>>,
}

macro_rules! named_row {
    ($name:ident, $field:ident) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name<'a> {
            pub hash: OptionHash,
            pub $field: Option<&'a str>,
        }
    };
}
named_row!(YieldRow, yield_type);
named_row!(BuildingRow, building_type);
named_row!(GovernorRow, trait_type);
named_row!(GovernmentRow, government_type);
named_row!(FeatureRow, feature_type);
named_row!(PromotionClassRow, promotion_class_type);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GovernmentSlotRow<'a> {
    pub government_type: Option<&'a str>,
    pub slot_type: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectRow<'a> {
    pub hash: OptionHash,
    pub space_race: LuaScalar<'a>,
    pub wmd: LuaScalar<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperationRow<'a> {
    pub hash: OptionHash,
    pub category_in_ui: Option<&'a str>,
    /// Caller supplies a numeric BaseProbability; malformed/non-numeric rows
    /// cannot be evaluated by this bounded API.
    pub base_probability: f64,
}

/// Independent lazy globals for nine definition-driven callbacks.
///
/// None means not yet initialized; Some(empty) is an initialized empty list.
/// Construct an explicit seed for an existing script environment. No Default
/// implementation infers cache state from a save or native snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldCongressOptionCaches {
    pub power_resources: Option<Vec<OptionHash>>,
    pub unit_yields: Option<Vec<OptionHash>>,
    pub power_buildings: Option<Vec<OptionHash>>,
    pub governors: Option<Vec<OptionHash>>,
    pub wildcard_governments: Option<Vec<OptionHash>>,
    pub public_works_projects: Option<Vec<OptionHash>>,
    pub deforestation_features: Option<Vec<OptionHash>>,
    pub espionage_operations: Option<Vec<OptionHash>>,
    pub military_promotions: Option<Vec<OptionHash>>,
}

impl WorldCongressOptionCaches {
    /// Use only for a newly initialized script with no preexisting cache globals.
    pub const fn new_script() -> Self {
        Self {
            power_resources: None,
            unit_yields: None,
            power_buildings: None,
            governors: None,
            wildcard_governments: None,
            public_works_projects: None,
            deforestation_features: None,
            espionage_operations: None,
            military_promotions: None,
        }
    }

    /// WC_Validate_PowerResourceBan. Consumption duplicates remain duplicates.
    pub fn power_resource_ban(&mut self, rows: &[ResourceConsumptionRow<'_>]) -> &[OptionHash] {
        self.power_resources.get_or_insert_with(|| {
            rows.iter()
                .filter_map(|row| row.resource)
                .filter(|resource| !resource.power_provided.equals_zero())
                .map(|resource| resource.hash)
                .collect()
        })
    }

    /// WC_Validate_YieldBan; preserves row order rather than listed type order.
    pub fn yield_ban(&mut self, rows: &[YieldRow<'_>]) -> &[OptionHash] {
        self.unit_yields.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    matches!(
                        row.yield_type,
                        Some("YIELD_PRODUCTION" | "YIELD_GOLD" | "YIELD_FAITH")
                    )
                })
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_PowerBuilding.
    pub fn power_building(&mut self, rows: &[BuildingRow<'_>]) -> &[OptionHash] {
        self.power_buildings.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    matches!(
                        row.building_type,
                        Some(
                            "BUILDING_COAL_POWER_PLANT"
                                | "BUILDING_FOSSIL_FUEL_POWER_PLANT"
                                | "BUILDING_POWER_PLANT"
                        )
                    )
                })
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_ValidateGovernanceDoctrine. Empty trait text is not nil.
    pub fn governance_doctrine(&mut self, rows: &[GovernorRow<'_>]) -> &[OptionHash] {
        self.governors.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| row.trait_type.is_none())
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_WorldIdeology. Any matching wildcard row qualifies; no slot
    /// count is read by this callback.
    pub fn world_ideology(
        &mut self,
        rows: &[GovernmentRow<'_>],
        slots: &[GovernmentSlotRow<'_>],
    ) -> &[OptionHash] {
        self.wildcard_governments.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    slots.iter().any(|slot| {
                        slot.government_type == row.government_type
                            && slot.slot_type == Some("SLOT_WILDCARD")
                    })
                })
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_PublicWorks: SpaceRace must be boolean true, whereas WMD
    /// uses Lua truthiness (including numeric zero).
    pub fn public_works(&mut self, rows: &[ProjectRow<'_>]) -> &[OptionHash] {
        self.public_works_projects.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| row.space_race.equals_true() || row.wmd.truthy())
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_DeforestationFeature.
    pub fn deforestation_feature(&mut self, rows: &[FeatureRow<'_>]) -> &[OptionHash] {
        self.deforestation_features.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    matches!(
                        row.feature_type,
                        Some("FEATURE_MARSH" | "FEATURE_FOREST" | "FEATURE_JUNGLE")
                    )
                })
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_ESPIONAGE_PACT.
    pub fn espionage_pact(&mut self, rows: &[OperationRow<'_>]) -> &[OptionHash] {
        self.espionage_operations.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    row.category_in_ui == Some("OFFENSIVESPY") && row.base_probability > 0.0
                })
                .map(|row| row.hash)
                .collect()
        })
    }

    /// WC_Validate_MILITARY_ADVISORY.
    pub fn military_advisory(&mut self, rows: &[PromotionClassRow<'_>]) -> &[OptionHash] {
        self.military_promotions.get_or_insert_with(|| {
            rows.iter()
                .filter(|row| {
                    matches!(
                        row.promotion_class_type,
                        Some(
                            "PROMOTION_CLASS_RECON"
                                | "PROMOTION_CLASS_MELEE"
                                | "PROMOTION_CLASS_RANGED"
                                | "PROMOTION_CLASS_ANTI_CAVALRY"
                                | "PROMOTION_CLASS_LIGHT_CAVALRY"
                                | "PROMOTION_CLASS_HEAVY_CAVALRY"
                                | "PROMOTION_CLASS_SIEGE"
                        )
                    )
                })
                .map(|row| row.hash)
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_lookup_duplicates_and_non_numeric_zero_distinction() {
        let powered = ResourceConsumptionRow {
            resource: Some(ResourceRow {
                hash: 8,
                power_provided: LuaScalar::Number(2.0),
            }),
        };
        let rows = [
            powered,
            ResourceConsumptionRow { resource: None },
            powered,
            ResourceConsumptionRow {
                resource: Some(ResourceRow {
                    hash: 9,
                    power_provided: LuaScalar::Number(-0.0),
                }),
            },
            ResourceConsumptionRow {
                resource: Some(ResourceRow {
                    hash: 10,
                    power_provided: LuaScalar::Boolean(false),
                }),
            },
        ];
        assert_eq!(
            WorldCongressOptionCaches::new_script().power_resource_ban(&rows),
            [8, 8, 10]
        );
    }

    #[test]
    fn project_boolean_equality_differs_from_lua_truthiness() {
        let pairs = [
            (LuaScalar::Number(1.0), LuaScalar::Nil),
            (LuaScalar::Boolean(true), LuaScalar::Boolean(false)),
            (LuaScalar::Boolean(false), LuaScalar::Number(0.0)),
            (LuaScalar::Nil, LuaScalar::String("")),
            (LuaScalar::Nil, LuaScalar::Boolean(false)),
        ];
        let rows: Vec<_> = pairs
            .into_iter()
            .enumerate()
            .map(|(i, (space_race, wmd))| ProjectRow {
                hash: i as u32,
                space_race,
                wmd,
            })
            .collect();
        assert_eq!(
            WorldCongressOptionCaches::new_script().public_works(&rows),
            [1, 2, 3]
        );
    }

    #[test]
    fn caches_retain_first_call_and_explicit_seed_mutation() {
        let mut caches = WorldCongressOptionCaches::new_script();
        assert!(caches.yield_ban(&[]).is_empty());
        let yields = [YieldRow {
            hash: 3,
            yield_type: Some("YIELD_FAITH"),
        }];
        assert!(caches.yield_ban(&yields).is_empty());
        caches.unit_yields = Some(vec![7, 7]);
        assert_eq!(caches.yield_ban(&yields), [7, 7]);
        caches.unit_yields.as_mut().unwrap().push(9);
        assert_eq!(caches.yield_ban(&[]), [7, 7, 9]);
        assert_eq!(
            caches.governance_doctrine(&[
                GovernorRow {
                    hash: 2,
                    trait_type: Some("")
                },
                GovernorRow {
                    hash: 1,
                    trait_type: None
                },
            ]),
            [1]
        );
        assert_eq!(caches.governance_doctrine(&[]), [1]);
    }

    #[test]
    fn wildcard_matching_is_existential_and_preserves_government_order() {
        let governments = [
            GovernmentRow {
                hash: 3,
                government_type: Some("C"),
            },
            GovernmentRow {
                hash: 2,
                government_type: Some("B"),
            },
            GovernmentRow {
                hash: 1,
                government_type: Some("A"),
            },
        ];
        let slots = [
            GovernmentSlotRow {
                government_type: Some("A"),
                slot_type: Some("SLOT_WILDCARD"),
            },
            GovernmentSlotRow {
                government_type: Some("B"),
                slot_type: Some("SLOT_MILITARY"),
            },
            GovernmentSlotRow {
                government_type: Some("C"),
                slot_type: Some("SLOT_WILDCARD"),
            },
            GovernmentSlotRow {
                government_type: Some("C"),
                slot_type: Some("SLOT_WILDCARD"),
            },
        ];
        let mut caches = WorldCongressOptionCaches::new_script();
        assert_eq!(caches.world_ideology(&governments, &slots), [3, 1]);
        assert_eq!(caches.world_ideology(&[], &[]), [3, 1]);
    }

    #[test]
    fn named_filters_preserve_input_order_and_reject_near_matches() {
        let mut caches = WorldCongressOptionCaches::new_script();
        assert_eq!(
            caches.yield_ban(&[
                YieldRow {
                    hash: 5,
                    yield_type: Some("YIELD_FAITH")
                },
                YieldRow {
                    hash: 9,
                    yield_type: Some("YIELD_SCIENCE")
                },
                YieldRow {
                    hash: 4,
                    yield_type: Some("YIELD_GOLD")
                },
                YieldRow {
                    hash: 3,
                    yield_type: Some("YIELD_PRODUCTION")
                },
                YieldRow {
                    hash: 5,
                    yield_type: Some("YIELD_FAITH")
                },
            ]),
            [5, 4, 3, 5]
        );
        assert_eq!(
            caches.power_building(&[
                BuildingRow {
                    hash: 4,
                    building_type: Some("BUILDING_POWER_PLANT")
                },
                BuildingRow {
                    hash: 2,
                    building_type: Some("BUILDING_COAL_POWER_PLANT")
                },
                BuildingRow {
                    hash: 8,
                    building_type: Some("BUILDING_POWER_PLANT_EXTRA")
                },
                BuildingRow {
                    hash: 3,
                    building_type: Some("BUILDING_FOSSIL_FUEL_POWER_PLANT")
                },
            ]),
            [4, 2, 3]
        );
        assert_eq!(
            caches.deforestation_feature(&[
                FeatureRow {
                    hash: 3,
                    feature_type: Some("FEATURE_JUNGLE")
                },
                FeatureRow {
                    hash: 9,
                    feature_type: None
                },
                FeatureRow {
                    hash: 1,
                    feature_type: Some("FEATURE_MARSH")
                },
                FeatureRow {
                    hash: 2,
                    feature_type: Some("FEATURE_FOREST")
                },
            ]),
            [3, 1, 2]
        );
        assert_eq!(
            caches.military_advisory(&[
                PromotionClassRow {
                    hash: 2,
                    promotion_class_type: Some("PROMOTION_CLASS_SIEGE")
                },
                PromotionClassRow {
                    hash: 8,
                    promotion_class_type: Some("PROMOTION_CLASS_NAVAL_MELEE")
                },
                PromotionClassRow {
                    hash: 1,
                    promotion_class_type: Some("PROMOTION_CLASS_RECON")
                },
            ]),
            [2, 1]
        );
    }

    #[test]
    fn espionage_requires_both_category_and_positive_probability() {
        let rows = [
            OperationRow {
                hash: 1,
                category_in_ui: Some("OFFENSIVESPY"),
                base_probability: 1.0,
            },
            OperationRow {
                hash: 2,
                category_in_ui: Some("OFFENSIVESPY"),
                base_probability: 0.0,
            },
            OperationRow {
                hash: 3,
                category_in_ui: Some("DEFENSIVESPY"),
                base_probability: 50.0,
            },
            OperationRow {
                hash: 4,
                category_in_ui: Some("OFFENSIVESPY"),
                base_probability: -1.0,
            },
        ];
        assert_eq!(
            WorldCongressOptionCaches::new_script().espionage_pact(&rows),
            [1]
        );
    }
}
