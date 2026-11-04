//! Bounded Linux World Congress resolution and option filtering callbacks.
//!
//! Callers supply the exact GameInfo iteration order, resolved resource lookups,
//! scalar types, and hash bit patterns for their rules configuration. No database
//! bridge, callback dispatcher, resolution execution, or initialized game is
//! provided. These source-backed predicates have synthetic regression coverage;
//! they do not establish native parity or complete state/action/turn support.
//! All twenty registrations are represented as filtering algorithms only. Game
//! providers, callback registration/dispatch, voting and resolution effects are
//! not implemented; external policies still choose every player decision.
//!
//! Typed rows/queries admit ordinary numeric, string/nil and boolean contracts
//! only. Bindings must preserve actual ordered rows, keyed lookups, return types
//! and reached failures; missing data must never become a default value. Errors
//! leave reached global cache/counter writes intact and prevent output assignment.
//! Such partial writes are callback diagnostics, not action acceptance.
//!
//! Each method's borrowed slice represents assignment of ResolutionOptions to
//! the corresponding global cached list. The source aliases that list rather
//! than copying it. Public cache fields permit explicit seeds and subsequent
//! mutation; callers tracking longer-lived aliases must refer back to that
//! same cache field. This API does not implement arbitrary Lua table aliasing.

/// Raw 32-bit definition identity, not a Lua numeric value.
///
/// Only rows with an admitted numeric 32-bit Hash are supported. Missing,
/// nonnumeric, and out-of-range Hash values are unsupported and cannot be
/// represented here; their Lua insertion behavior is outside this boundary.
/// Callers must preserve signed i32 Hash bit patterns by reinterpreting them
/// as u32 (for example, `u32::from_ne_bytes(hash_i32.to_ne_bytes())`). Do not
/// use a saturating float-to-u32 conversion: it destroys negative identities.
/// This module neither calculates hashes nor validates the input binding.
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

/// Script globals used by the bounded option callbacks.
///
/// None means not yet initialized; Some(empty) is an initialized empty list
/// (the source Lua empty table remains truthy).
/// Construct an explicit seed for an existing script environment. No Default
/// implementation infers cache state from a save or native snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldCongressOptionCaches {
    pub luxury_resources: Option<Vec<OptionHash>>,
    pub policy_types: Option<Vec<OptionHash>>,
    pub district_types: Option<Vec<OptionHash>>,
    pub players: Option<Vec<CongressPlayerId>>,
    pub great_people_classes: Option<Vec<OptionHash>>,
    /// None represents an absent global before its first luxury branch.
    /// Admitted resource-row counts are <= 2^53, preserving Lua integer steps.
    pub luxury_resources_count: Option<u64>,
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
            luxury_resources: None,
            policy_types: None,
            district_types: None,
            players: None,
            great_people_classes: None,
            luxury_resources_count: None,
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

/// Player identity is distinct from a rules definition hash.
pub type CongressPlayerId = i32;

/// Errors stop assignment of the callback output, but do not undo global writes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CongressQueryError<E> {
    Query(E),
    Missing(&'static str),
}
use CongressQueryError::{Missing, Query};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuxuryResourceRow<'a> {
    pub hash: OptionHash,
    pub resource_class_type: Option<&'a str>,
    pub frequency: f64,
    pub sea_frequency: f64,
}
impl LuxuryResourceRow<'_> {
    fn candidate(self) -> bool {
        self.resource_class_type == Some("RESOURCECLASS_LUXURY")
            && (self.frequency > 0.0 || self.sea_frequency > 0.0)
    }
}

/// Every method is required; unavailable queries must return an error.
/// Numeric inputs/results must retain the source Lua numbers, without defaults.
pub trait LuxuryWorld {
    type Error;
    fn resource_count(&self, hash: OptionHash) -> Result<f64, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionDefinition {
    WorldReligion,
    DiploVictory,
    Luxury,
}
pub trait ResolutionWorld: LuxuryWorld {
    fn resolution_hash(
        &self,
        definition: ResolutionDefinition,
    ) -> Result<Option<OptionHash>, Self::Error>;
    /// Resolve Game.GetReligion before querying its list; nil is an error.
    /// Return the actual Lua table length (#GetReligions()), including its
    /// traversal/length contract, rather than an inferred world religion count.
    fn religion_count(&self) -> Result<usize, Self::Error>;
    fn diplomatic_victory_enabled(&self) -> Result<bool, Self::Error>;
}

pub trait CultureQueries {
    type Error;
    fn is_policy_unlocked(&self, hash: OptionHash) -> Result<bool, Self::Error>;
    fn is_policy_obsolete(&self, hash: OptionHash) -> Result<bool, Self::Error>;
}
pub trait CultureWorld {
    type Error;
    /// Resolve Players[player] first; a missing player must be an error.
    /// None represents only a nil GetCulture result.
    fn culture(
        &self,
        player: CongressPlayerId,
    ) -> Result<Option<&dyn CultureQueries<Error = Self::Error>>, Self::Error>;
}
pub trait DiplomacyQueries {
    type Error;
    fn has_met(&self, player: CongressPlayerId) -> Result<bool, Self::Error>;
}
pub trait CongressPlayer {
    type Error;
    fn id(&self) -> Result<CongressPlayerId, Self::Error>;
}
pub type CongressPlayerEntry<'a, E> = Option<&'a dyn CongressPlayer<Error = E>>;

pub trait PlayerWorld {
    type Error;
    /// Ordered entries from GetAliveMajors, preserving duplicates. The slice
    /// contains numeric entries; traversal ends at the first None, as ipairs does.
    fn alive_majors(&self) -> Result<&[CongressPlayerEntry<'_, Self::Error>], Self::Error>;
    /// Resolve Players[player] and then GetDiplomacy. Missing player is an
    /// error; None represents a nil diplomacy handle (dereferenced in loop).
    fn diplomacy(
        &self,
        player: CongressPlayerId,
    ) -> Result<Option<&dyn DiplomacyQueries<Error = Self::Error>>, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreatPersonClassRow {
    pub hash: OptionHash,
    pub index: i32,
}
pub trait GreatPeopleQueries {
    type Error;
    fn is_class_available(&self, index: i32) -> Result<bool, Self::Error>;
}
pub trait GreatPeopleWorld {
    type Error;
    fn great_people(
        &self,
    ) -> Result<Option<&dyn GreatPeopleQueries<Error = Self::Error>>, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistrictRow<'a> {
    pub hash: OptionHash,
    pub district_type: Option<&'a str>,
    pub prereq_civic: Option<&'a str>,
    pub prereq_tech: Option<&'a str>,
}
pub trait UrbanWorld {
    type Error;
    /// Includes GetEras/GetCurrentEra and the keyed Eras lookup. Missing
    /// handles/rows must return errors rather than invented chronology values.
    fn current_era_chronology(&self) -> Result<f64, Self::Error>;
    fn district(&self, hash: OptionHash) -> Result<Option<DistrictRow<'_>>, Self::Error>;
    /// Resolve Civic/Technology, then its EraType, then Eras[EraType].
    /// A missing lookup at either step must return an error.
    fn civic_era_chronology(&self, civic: &str) -> Result<f64, Self::Error>;
    fn tech_era_chronology(&self, tech: &str) -> Result<f64, Self::Error>;
}

/// The empty GreatWorkObjects callback performs no options assignment.
pub const fn great_work_objects() {}

impl WorldCongressOptionCaches {
    pub fn can_use_resolutions<W: ResolutionWorld>(
        &mut self,
        resolutions: &mut Vec<OptionHash>,
        resources: &[LuxuryResourceRow<'_>],
        world: &W,
    ) -> Result<(), CongressQueryError<W::Error>> {
        let religion = world
            .resolution_hash(ResolutionDefinition::WorldReligion)
            .map_err(Query)?;
        let victory = world
            .resolution_hash(ResolutionDefinition::DiploVictory)
            .map_err(Query)?;
        let luxury = world
            .resolution_hash(ResolutionDefinition::Luxury)
            .map_err(Query)?;
        let mut allowed = Vec::new();
        for &hash in resolutions.iter() {
            let include = if Some(hash) == religion {
                world.religion_count().map_err(Query)? > 0
            } else if Some(hash) == victory {
                world.diplomatic_victory_enabled().map_err(Query)?
            } else if Some(hash) == luxury {
                self.luxury_resources_count = Some(0);
                for row in resources.iter().filter(|row| row.candidate()) {
                    if world.resource_count(row.hash).map_err(Query)? > 0.0 {
                        *self.luxury_resources_count.as_mut().unwrap() += 1;
                    }
                }
                self.luxury_resources_count.unwrap() > 0
            } else {
                true
            };
            if include {
                allowed.push(hash);
            }
        }
        *resolutions = allowed;
        Ok(())
    }

    pub fn luxury_ban<W: LuxuryWorld>(
        &mut self,
        rows: &[LuxuryResourceRow<'_>],
        world: &W,
    ) -> Result<&[OptionHash], CongressQueryError<W::Error>> {
        if self.luxury_resources.is_none() {
            self.luxury_resources = Some(Vec::new());
            for row in rows.iter().filter(|row| row.candidate()) {
                if world.resource_count(row.hash).map_err(Query)? > 0.0 {
                    self.luxury_resources.as_mut().unwrap().push(row.hash);
                }
            }
        }
        Ok(self.luxury_resources.as_deref().unwrap())
    }

    pub fn policy_treaty<W: CultureWorld>(
        &mut self,
        player: CongressPlayerId,
        rows: &[OptionHash],
        world: &W,
    ) -> Result<&[OptionHash], CongressQueryError<W::Error>> {
        self.policy_types = Some(Vec::new());
        if let Some(culture) = world.culture(player).map_err(Query)? {
            for &hash in rows {
                if culture.is_policy_unlocked(hash).map_err(Query)?
                    && !culture.is_policy_obsolete(hash).map_err(Query)?
                {
                    self.policy_types.as_mut().unwrap().push(hash);
                }
            }
        }
        Ok(self.policy_types.as_deref().unwrap())
    }

    pub fn urban_development<W: UrbanWorld>(
        &mut self,
        rows: &[DistrictRow<'_>],
        world: &W,
    ) -> Result<Vec<OptionHash>, CongressQueryError<W::Error>> {
        if self.district_types.is_none() {
            self.district_types = Some(Vec::new());
            for row in rows {
                if matches!(
                    row.district_type,
                    Some(
                        "DISTRICT_CITY_CENTER"
                            | "DISTRICT_HOLY_SITE"
                            | "DISTRICT_CAMPUS"
                            | "DISTRICT_ENCAMPMENT"
                            | "DISTRICT_HARBOR"
                            | "DISTRICT_AERODROME"
                            | "DISTRICT_COMMERCIAL_HUB"
                            | "DISTRICT_ENTERTAINMENT_COMPLEX"
                            | "DISTRICT_THEATER"
                            | "DISTRICT_INDUSTRIAL_ZONE"
                            | "DISTRICT_GOVERNMENT"
                            | "DISTRICT_WATER_ENTERTAINMENT_COMPLEX"
                            | "DISTRICT_DIPLOMATIC_QUARTER"
                            | "DISTRICT_PRESERVE"
                    )
                ) {
                    self.district_types.as_mut().unwrap().push(row.hash);
                }
            }
        }
        let current = world.current_era_chronology().map_err(Query)?;
        let mut output = Vec::new();
        for &hash in self.district_types.as_ref().unwrap() {
            let row = world
                .district(hash)
                .map_err(Query)?
                .ok_or(Missing("Districts[hash]"))?;
            let mut allowed = true;
            if let Some(civic) = row.prereq_civic.filter(|s| !s.is_empty()) {
                if world.civic_era_chronology(civic).map_err(Query)? > current {
                    allowed = false;
                }
            }
            if let Some(tech) = row.prereq_tech.filter(|s| !s.is_empty()) {
                if world.tech_era_chronology(tech).map_err(Query)? > current {
                    allowed = false;
                }
            }
            if allowed {
                output.push(hash);
            }
        }
        Ok(output)
    }

    fn player_options<W: PlayerWorld>(
        &mut self,
        player: CongressPlayerId,
        world: &W,
    ) -> Result<&[CongressPlayerId], CongressQueryError<W::Error>> {
        self.players = Some(Vec::new());
        let players = world.alive_majors().map_err(Query)?;
        let diplomacy = world.diplomacy(player).map_err(Query)?;
        for p in players.iter().take_while(|p| p.is_some()).flatten() {
            let id = p.id().map_err(Query)?;
            let met = diplomacy
                .ok_or(Missing("GetDiplomacy"))?
                .has_met(id)
                .map_err(Query)?;
            if met || id == player {
                self.players.as_mut().unwrap().push(id);
            }
        }
        Ok(self.players.as_deref().unwrap())
    }

    /// None means return without assigning ResolutionOptions. Some aliases the
    /// rebuilt global cache, including when the assigned list is empty.
    pub fn patronage<W: GreatPeopleWorld>(
        &mut self,
        rows: &[GreatPersonClassRow],
        world: &W,
    ) -> Result<Option<&[OptionHash]>, CongressQueryError<W::Error>> {
        self.great_people_classes = Some(Vec::new());
        let Some(people) = world.great_people().map_err(Query)? else {
            return Ok(None);
        };
        for row in rows {
            if people.is_class_available(row.index).map_err(Query)? {
                self.great_people_classes.as_mut().unwrap().push(row.hash);
            }
        }
        Ok(Some(self.great_people_classes.as_deref().unwrap()))
    }
}

macro_rules! player_callback {
    ($($name:ident),+) => { impl WorldCongressOptionCaches { $(
        pub fn $name<W: PlayerWorld>(&mut self, player: CongressPlayerId, world: &W) -> Result<&[CongressPlayerId], CongressQueryError<W::Error>> {
            self.player_options(player, world)
        }
    )+ } };
}
player_callback!(
    border_control,
    migration_treaty,
    arms_control_treaty,
    trade_treaty,
    diplo_victory
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct World {
        calls: RefCell<Vec<String>>,
        fail: Option<&'static str>,
        nil: bool,
    }
    impl World {
        fn call(&self, name: String) -> Result<(), &'static str> {
            self.calls.borrow_mut().push(name.clone());
            if self.fail == Some(name.as_str()) {
                Err("fixture failure")
            } else {
                Ok(())
            }
        }
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                fail: None,
                nil: false,
            }
        }
    }
    impl LuxuryWorld for World {
        type Error = &'static str;
        fn resource_count(&self, hash: u32) -> Result<f64, Self::Error> {
            self.call(format!("resource:{hash}"))?;
            Ok(if hash == 2 { 0.0 } else { 1.0 })
        }
    }
    impl ResolutionWorld for World {
        fn resolution_hash(&self, def: ResolutionDefinition) -> Result<Option<u32>, Self::Error> {
            self.call(format!("definition:{def:?}"))?;
            Ok(Some(match def {
                ResolutionDefinition::WorldReligion => 10,
                ResolutionDefinition::DiploVictory => 11,
                ResolutionDefinition::Luxury => 12,
            }))
        }
        fn religion_count(&self) -> Result<usize, Self::Error> {
            self.call("religion".into())?;
            Ok(0)
        }
        fn diplomatic_victory_enabled(&self) -> Result<bool, Self::Error> {
            self.call("victory".into())?;
            Ok(false)
        }
    }
    fn lux(hash: u32) -> LuxuryResourceRow<'static> {
        LuxuryResourceRow {
            hash,
            resource_class_type: Some("RESOURCECLASS_LUXURY"),
            frequency: 1.0,
            sea_frequency: 0.0,
        }
    }

    #[test]
    fn luxury_failure_retains_prefix_and_next_call_uses_it_without_queries() {
        let mut world = World::new();
        world.fail = Some("resource:3");
        let mut caches = WorldCongressOptionCaches::new_script();
        assert_eq!(
            caches.luxury_ban(&[lux(1), lux(2), lux(3)], &world),
            Err(Query("fixture failure"))
        );
        assert_eq!(caches.luxury_resources, Some(vec![1]));
        assert_eq!(caches.luxury_ban(&[lux(9)], &world).unwrap(), [1]);
        assert_eq!(
            *world.calls.borrow(),
            ["resource:1", "resource:2", "resource:3"]
        );
    }

    #[test]
    fn resolution_assignment_is_atomic_but_luxury_counter_is_not() {
        let mut world = World::new();
        world.fail = Some("victory");
        let mut caches = WorldCongressOptionCaches::new_script();
        let mut resolutions = vec![12, 11, 10, 99];
        assert_eq!(
            caches.can_use_resolutions(&mut resolutions, &[lux(1), lux(2), lux(1)], &world),
            Err(Query("fixture failure"))
        );
        assert_eq!(resolutions, [12, 11, 10, 99]);
        assert_eq!(caches.luxury_resources_count, Some(2));
        assert_eq!(
            *world.calls.borrow(),
            [
                "definition:WorldReligion",
                "definition:DiploVictory",
                "definition:Luxury",
                "resource:1",
                "resource:2",
                "resource:1",
                "victory"
            ]
        );
        world.calls.borrow_mut().clear();
        caches
            .can_use_resolutions(&mut vec![], &[], &world)
            .unwrap();
        assert_eq!(world.calls.borrow().len(), 3);
        world.fail = None;
        caches
            .can_use_resolutions(&mut resolutions, &[lux(1)], &world)
            .unwrap();
        assert_eq!(resolutions, [12, 99]);
    }
    impl CultureQueries for World {
        type Error = &'static str;
        fn is_policy_unlocked(&self, h: u32) -> Result<bool, Self::Error> {
            self.call(format!("unlocked:{h}"))?;
            Ok(h != 2)
        }
        fn is_policy_obsolete(&self, h: u32) -> Result<bool, Self::Error> {
            self.call(format!("obsolete:{h}"))?;
            Ok(h == 3)
        }
    }
    impl CultureWorld for World {
        type Error = &'static str;
        fn culture(
            &self,
            p: i32,
        ) -> Result<Option<&dyn CultureQueries<Error = Self::Error>>, Self::Error> {
            self.call(format!("culture:{p}"))?;
            Ok(if self.nil { None } else { Some(self) })
        }
    }
    impl GreatPeopleQueries for World {
        type Error = &'static str;
        fn is_class_available(&self, i: i32) -> Result<bool, Self::Error> {
            self.call(format!("class:{i}"))?;
            Ok(i != 2)
        }
    }
    impl GreatPeopleWorld for World {
        type Error = &'static str;
        fn great_people(
            &self,
        ) -> Result<Option<&dyn GreatPeopleQueries<Error = Self::Error>>, Self::Error> {
            self.call("great_people".into())?;
            Ok(if self.nil { None } else { Some(self) })
        }
    }
    #[test]
    fn policies_short_circuit_rebuild_and_nil_patronage_does_not_assign() {
        let mut caches = WorldCongressOptionCaches::new_script();
        let mut world = World::new();
        assert_eq!(
            caches.policy_treaty(4, &[1, 2, 3, 1], &world).unwrap(),
            [1, 1]
        );
        assert_eq!(
            *world.calls.borrow(),
            [
                "culture:4",
                "unlocked:1",
                "obsolete:1",
                "unlocked:2",
                "unlocked:3",
                "obsolete:3",
                "unlocked:1",
                "obsolete:1"
            ]
        );
        world.nil = true;
        assert!(caches.policy_treaty(4, &[1], &world).unwrap().is_empty());
        caches.great_people_classes = Some(vec![99]);
        assert_eq!(caches.patronage(&[], &world).unwrap(), None);
        assert_eq!(caches.great_people_classes, Some(vec![]));
        world.nil = false;
        assert_eq!(
            caches
                .patronage(
                    &[
                        GreatPersonClassRow { hash: 91, index: 1 },
                        GreatPersonClassRow { hash: 92, index: 2 }
                    ],
                    &world
                )
                .unwrap(),
            Some([91].as_slice())
        );
        world.fail = Some("obsolete:3");
        assert_eq!(
            caches.policy_treaty(4, &[1, 3], &world),
            Err(Query("fixture failure"))
        );
        assert_eq!(caches.policy_types, Some(vec![1]));
    }
    impl UrbanWorld for World {
        type Error = &'static str;
        fn current_era_chronology(&self) -> Result<f64, Self::Error> {
            self.call("era".into())?;
            Ok(2.0)
        }
        fn district(&self, h: u32) -> Result<Option<DistrictRow<'_>>, Self::Error> {
            self.call(format!("district:{h}"))?;
            Ok(if self.nil {
                None
            } else {
                Some(DistrictRow {
                    hash: h,
                    district_type: None,
                    prereq_civic: if h == 2 { Some("") } else { Some("C") },
                    prereq_tech: Some("T"),
                })
            })
        }
        fn civic_era_chronology(&self, c: &str) -> Result<f64, Self::Error> {
            self.call(format!("civic:{c}"))?;
            Ok(3.0)
        }
        fn tech_era_chronology(&self, t: &str) -> Result<f64, Self::Error> {
            self.call(format!("tech:{t}"))?;
            Ok(1.0)
        }
    }
    #[test]
    fn urban_checks_tech_after_future_civic_and_reuses_only_base_cache() {
        let mut caches = WorldCongressOptionCaches::new_script();
        let mut world = World::new();
        let rows = [1, 2].map(|h| DistrictRow {
            hash: h,
            district_type: Some("DISTRICT_CAMPUS"),
            prereq_civic: None,
            prereq_tech: None,
        });
        assert_eq!(caches.urban_development(&rows, &world).unwrap(), [2]);
        assert_eq!(
            *world.calls.borrow(),
            [
                "era",
                "district:1",
                "civic:C",
                "tech:T",
                "district:2",
                "tech:T"
            ]
        );
        world.calls.borrow_mut().clear();
        world.fail = Some("tech:T");
        assert_eq!(
            caches.urban_development(&[], &world),
            Err(Query("fixture failure"))
        );
        assert_eq!(caches.district_types, Some(vec![1, 2]));
        assert_eq!(
            *world.calls.borrow(),
            ["era", "district:1", "civic:C", "tech:T"]
        );
        world.fail = None;
        world.nil = true;
        assert_eq!(
            caches.urban_development(&[], &world),
            Err(Missing("Districts[hash]"))
        );
    }
    struct Player<'a> {
        id: i32,
        world: &'a World,
    }
    impl CongressPlayer for Player<'_> {
        type Error = &'static str;
        fn id(&self) -> Result<i32, Self::Error> {
            self.world.call(format!("id:{}", self.id))?;
            Ok(self.id)
        }
    }
    struct Players<'a> {
        world: &'a World,
        rows: Vec<Option<&'a dyn CongressPlayer<Error = &'static str>>>,
    }
    impl DiplomacyQueries for World {
        type Error = &'static str;
        fn has_met(&self, p: i32) -> Result<bool, Self::Error> {
            self.call(format!("met:{p}"))?;
            Ok(p == 8)
        }
    }
    impl PlayerWorld for Players<'_> {
        type Error = &'static str;
        fn alive_majors(&self) -> Result<&[CongressPlayerEntry<'_, Self::Error>], Self::Error> {
            self.world.call("majors".into())?;
            Ok(&self.rows)
        }
        fn diplomacy(
            &self,
            p: i32,
        ) -> Result<Option<&dyn DiplomacyQueries<Error = Self::Error>>, Self::Error> {
            self.world.call(format!("diplomacy:{p}"))?;
            Ok(if self.world.nil {
                None
            } else {
                Some(self.world)
            })
        }
    }
    #[test]
    fn player_callbacks_query_self_and_share_rebuilt_cache() {
        let mut caches = WorldCongressOptionCaches::new_script();
        let world = World::new();
        let own = Player {
            id: 4,
            world: &world,
        };
        let other = Player {
            id: 8,
            world: &world,
        };
        let players = Players {
            world: &world,
            rows: vec![Some(&own), Some(&other), Some(&other)],
        };
        assert_eq!(caches.border_control(4, &players).unwrap(), [4, 8, 8]);
        assert_eq!(
            *world.calls.borrow(),
            [
                "majors",
                "diplomacy:4",
                "id:4",
                "met:4",
                "id:8",
                "met:8",
                "id:8",
                "met:8"
            ]
        );
        let hole = Players {
            world: &world,
            rows: vec![None, Some(&other)],
        };
        world.calls.borrow_mut().clear();
        assert!(caches.border_control(4, &hole).unwrap().is_empty());
        assert_eq!(*world.calls.borrow(), ["majors", "diplomacy:4"]);
        let empty = Players {
            world: &world,
            rows: vec![],
        };
        assert!(caches.migration_treaty(4, &empty).unwrap().is_empty());
        assert!(caches.arms_control_treaty(4, &empty).unwrap().is_empty());
        assert!(caches.trade_treaty(4, &empty).unwrap().is_empty());
        assert!(caches.diplo_victory(4, &empty).unwrap().is_empty());
        assert_eq!(caches.players, Some(vec![]));
    }
    #[test]
    fn player_error_preserves_prefix_and_nil_diplomacy_is_lazy_error() {
        let mut caches = WorldCongressOptionCaches::new_script();
        let mut world = World::new();
        world.fail = Some("met:8");
        let own = Player {
            id: 4,
            world: &world,
        };
        let other = Player {
            id: 8,
            world: &world,
        };
        let players = Players {
            world: &world,
            rows: vec![Some(&own), Some(&other)],
        };
        assert_eq!(
            caches.trade_treaty(4, &players),
            Err(Query("fixture failure"))
        );
        assert_eq!(caches.players, Some(vec![4]));
        world.fail = None;
        world.nil = true;
        let empty = Players {
            world: &world,
            rows: vec![],
        };
        assert!(caches.border_control(4, &empty).unwrap().is_empty());
        let own = Player {
            id: 4,
            world: &world,
        };
        let players = Players {
            world: &world,
            rows: vec![Some(&own)],
        };
        assert_eq!(
            caches.border_control(4, &players),
            Err(Missing("GetDiplomacy"))
        );
        assert_eq!(caches.players, Some(vec![]));
    }

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
    fn signed_hash_bits_survive_filter_duplicates_and_cache_reuse() {
        let signed_hash = -1_234_567_i32;
        let hash = u32::from_ne_bytes(signed_hash.to_ne_bytes());
        let row = YieldRow {
            hash,
            yield_type: Some("YIELD_GOLD"),
        };
        let mut caches = WorldCongressOptionCaches::new_script();
        let options = caches.yield_ban(&[row, row]);
        assert_eq!(options, [0xffed_2979, 0xffed_2979]);
        assert_eq!(i32::from_ne_bytes(options[0].to_ne_bytes()), signed_hash);
        assert_eq!(caches.yield_ban(&[]), [hash, hash]);
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
