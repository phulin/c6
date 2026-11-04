//! Linux FoundCity handler2d9e1a0/2d9e2b0. This is the ordered handler kernel,
//! not complete city construction or an admitted action. Mandatory callbacks
//! must edit the same canonical world, including real City creation/placement,
//! subscribers/RNG and delayed Unit destruction; no default city is synthesized.
use crate::{
    skip_turn::StartData,
    unit::{ComponentId, PlotCoord},
};
pub mod collections;
pub mod creation;
pub mod feature_production;
pub mod names;
pub mod placement;

pub const OPERATION_HASH: u32 = 0x5733_ac6c;
pub const FOUNDING_TRANSFER_HASH: u32 = 0x6172_3064;
pub const NO_TYPE: i32 = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Feature {
    pub removable: bool,
}
/// Source gate inputs come from current actor, definitions and map. Outer
/// Manager/script legality is separate and must run before handler dispatch.
pub trait CanStartContext {
    type Error;
    type Plot: Copy;
    fn settling_disabled(&mut self) -> Result<bool, Self::Error>;
    fn unit_definition_found_city(&mut self) -> Result<Option<bool>, Self::Error>;
    fn unit_moves_remaining_q8(&mut self) -> Result<i32, Self::Error>;
    fn plot(&mut self) -> Result<Option<Self::Plot>, Self::Error>;
    fn owner(&mut self) -> Result<i32, Self::Error>;
    fn valid_found_location(&mut self, owner: i32, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn feature_type(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    /// None means native Find actually has no definition, not missing inputs.
    fn feature_definition(&mut self, kind: i32) -> Result<Option<Feature>, Self::Error>;
    /// Optional native out-map projection947248c1. It does not gate legality.
    fn removable_feature_output(&mut self, kind: i32) -> Result<(), Self::Error>;
}
pub fn can_start<C: CanStartContext>(
    c: &mut C,
    test_visible: bool,
    has_output: bool,
) -> Result<bool, C::Error> {
    if c.settling_disabled()? {
        return Ok(false);
    }
    if c.unit_definition_found_city()? != Some(true) {
        return Ok(false);
    }
    // Native CanMove is evaluated even when the supplied plot is null.
    let can_move = c.unit_moves_remaining_q8()? > 0;
    let Some(plot) = c.plot()? else {
        return Ok(false);
    };
    if !can_move {
        return Ok(false);
    }
    if !test_visible {
        let owner = c.owner()?;
        if !c.valid_found_location(owner, plot)? {
            return Ok(false);
        }
    }
    let feature = c.feature_type(plot)?;
    if feature != NO_TYPE {
        if let Some(definition) = c.feature_definition(feature)? {
            if has_output && definition.removable {
                c.removable_feature_output(feature)?;
            }
        }
    }
    Ok(true)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Creation {
    pub owner: i32,
    pub location: PlotCoord,
    pub transfer_type: u32,
    pub first_city: bool,
    pub original_capital: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    RemoveFeature {
        location: PlotCoord,
        value: i32,
    },
    ObserverCoordinates {
        x: i32,
        y: i32,
    },
    OperationStarted {
        identity: ComponentId,
        operation_hash: u32,
        value: i32,
    },
    Founded {
        identity: ComponentId,
        location: PlotCoord,
        visible_to_local_observer: bool,
    },
    Consumed {
        identity: ComponentId,
        value: i32,
    },
}
impl Report {
    pub const fn event_hash(&self) -> u32 {
        match self {
            Self::RemoveFeature { .. } => 0x878c_d252,
            Self::ObserverCoordinates { .. } => 0xb6b4_8d9c,
            Self::OperationStarted { .. } => 0x3d40_06f2,
            Self::Founded { .. } => 0x981a_9186,
            Self::Consumed { .. } => 0xc130_0ee7,
        }
    }
    pub const fn subtype_hash(&self) -> u32 {
        match self {
            Self::Founded { .. } => 0x9308_ffc2,
            Self::OperationStarted { operation_hash, .. } => *operation_hash,
            _ => 0,
        }
    }
}
/// All mutation and subscriber hooks are mandatory. In particular create_city
/// must run the real canonical constructor, naming/RNG, placement, CityCreated,
/// capital/palace and buildqueue sequence; a borrowed/default DTO is invalid.
pub trait StartContext {
    type Error;
    type Plot: Copy;
    type City: Copy;
    fn unit_plot(&mut self) -> Result<Self::Plot, Self::Error>;
    fn feature_type(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn feature_definition(&mut self, kind: i32) -> Result<Option<Feature>, Self::Error>;
    fn owner(&mut self) -> Result<i32, Self::Error>;
    fn location(&mut self) -> Result<PlotCoord, Self::Error>;
    fn identity(&mut self) -> Result<ComponentId, Self::Error>;
    fn report(&mut self, report: Report) -> Result<(), Self::Error>;
    /// Reacquire current Unit location/map plot AFTER RemoveFeature reporting.
    fn remove_feature_at_current_location(&mut self) -> Result<(), Self::Error>;
    fn observer_can_see_player(&mut self, owner: i32) -> Result<bool, Self::Error>;
    fn operation_hash(&mut self) -> Result<u32, Self::Error>;
    fn majority_religion(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn city_count(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn can_set_original_capital(&mut self, owner: i32) -> Result<bool, Self::Error>;
    /// Source constructor defaults: first i32-1, empty name, final word-1;
    /// bool argument to Player.Cities.Create is false. None is actual creation
    /// failure; native Start still reports and consumes the Unit in that case.
    fn create_city(&mut self, creation: Creation) -> Result<Option<Self::City>, Self::Error>;
    fn religion_auto_spreads(&mut self, religion: i32) -> Result<Option<bool>, Self::Error>;
    fn pressure_needed_for_majority(
        &mut self,
        city: Self::City,
        religion: i32,
    ) -> Result<i32, Self::Error>;
    fn add_religious_pressure(
        &mut self,
        city: Self::City,
        religion: i32,
        amount_q8: i32,
        reason: i32,
        source_player: i32,
    ) -> Result<(), Self::Error>;
    fn local_observer_sees_current_location(&mut self) -> Result<bool, Self::Error>;
    fn delayed_destroy_unit(&mut self) -> Result<(), Self::Error>;
}
/// Start does not re-run CanStart. Prefix mutations remain on a later hook
/// error; action becomes true only AFTER successful delayed destruction.
pub fn start<C: StartContext>(c: &mut C, data: &mut StartData) -> Result<(), C::Error> {
    let plot = c.unit_plot()?;
    let feature = c.feature_type(plot)?;
    if feature != NO_TYPE {
        if let Some(definition) = c.feature_definition(feature)? {
            if definition.removable {
                let location = c.location()?;
                c.report(Report::RemoveFeature { location, value: 0 })?;
                c.remove_feature_at_current_location()?;
            }
        }
    }
    let owner = c.owner()?;
    if c.observer_can_see_player(owner)? {
        let location = c.location()?;
        c.report(Report::ObserverCoordinates {
            x: location.x,
            y: location.y,
        })?;
    }
    let operation_hash = c.operation_hash()?;
    let identity = c.identity()?;
    c.report(Report::OperationStarted {
        identity,
        operation_hash,
        value: 0,
    })?;
    let owner = c.owner()?;
    let religion = c.majority_religion(owner)?;
    let location = c.location()?;
    let creation_owner = c.owner()?;
    let first = c.city_count(owner)? == 0;
    let original_capital = first && c.can_set_original_capital(owner)?;
    let city = c.create_city(Creation {
        owner: creation_owner,
        location,
        transfer_type: FOUNDING_TRANSFER_HASH,
        first_city: first,
        original_capital,
    })?;
    if religion != NO_TYPE {
        let auto = c.religion_auto_spreads(religion)?;
        if let (Some(city), Some(true)) = (city, auto) {
            let pressure = c.pressure_needed_for_majority(city, religion)?;
            c.add_religious_pressure(city, religion, pressure.wrapping_mul(2), 10, -1)?;
        }
    }
    let identity = c.identity()?;
    let location = c.location()?;
    let visible = c.local_observer_sees_current_location()?;
    c.report(Report::Founded {
        identity,
        location,
        visible_to_local_observer: visible,
    })?;
    let identity = c.identity()?;
    c.report(Report::Consumed { identity, value: 0 })?;
    c.delayed_destroy_unit()?;
    data.action = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World {
        log: Vec<&'static str>,
        reports: Vec<Report>,
        disabled: bool,
        definition: Option<bool>,
        moves: i32,
        plot: Option<u8>,
        feature: i32,
        removable: bool,
        legal: bool,
        location: PlotCoord,
        city: Option<u32>,
        count: i32,
        religion: i32,
        pressure: i32,
        creation: Option<Creation>,
        pressure_added: Option<i32>,
        fail_destroy: bool,
    }
    impl Default for World {
        fn default() -> Self {
            Self {
                log: vec![],
                reports: vec![],
                disabled: false,
                definition: Some(true),
                moves: 256,
                plot: Some(1),
                feature: NO_TYPE,
                removable: false,
                legal: true,
                location: PlotCoord { x: 8, y: 9 },
                city: Some(7),
                count: 0,
                religion: NO_TYPE,
                pressure: 0,
                creation: None,
                pressure_added: None,
                fail_destroy: false,
            }
        }
    }
    impl CanStartContext for World {
        type Error = &'static str;
        type Plot = u8;
        fn settling_disabled(&mut self) -> Result<bool, Self::Error> {
            self.log.push("disabled");
            Ok(self.disabled)
        }
        fn unit_definition_found_city(&mut self) -> Result<Option<bool>, Self::Error> {
            self.log.push("definition");
            Ok(self.definition)
        }
        fn unit_moves_remaining_q8(&mut self) -> Result<i32, Self::Error> {
            self.log.push("moves");
            Ok(self.moves)
        }
        fn plot(&mut self) -> Result<Option<u8>, Self::Error> {
            self.log.push("plot");
            Ok(self.plot)
        }
        fn owner(&mut self) -> Result<i32, Self::Error> {
            Ok(2)
        }
        fn valid_found_location(&mut self, owner: i32, _: u8) -> Result<bool, Self::Error> {
            assert_eq!(owner, 2);
            self.log.push("location_gate");
            Ok(self.legal)
        }
        fn feature_type(&mut self, _: u8) -> Result<i32, Self::Error> {
            Ok(self.feature)
        }
        fn feature_definition(&mut self, _: i32) -> Result<Option<Feature>, Self::Error> {
            Ok(Some(Feature {
                removable: self.removable,
            }))
        }
        fn removable_feature_output(&mut self, _: i32) -> Result<(), Self::Error> {
            self.log.push("feature_output");
            Ok(())
        }
    }
    impl StartContext for World {
        type Error = &'static str;
        type Plot = u8;
        type City = u32;
        fn unit_plot(&mut self) -> Result<u8, Self::Error> {
            self.plot.ok_or("plot")
        }
        fn feature_type(&mut self, _: u8) -> Result<i32, Self::Error> {
            Ok(self.feature)
        }
        fn feature_definition(&mut self, _: i32) -> Result<Option<Feature>, Self::Error> {
            Ok(Some(Feature {
                removable: self.removable,
            }))
        }
        fn owner(&mut self) -> Result<i32, Self::Error> {
            Ok(2)
        }
        fn location(&mut self) -> Result<PlotCoord, Self::Error> {
            Ok(self.location)
        }
        fn identity(&mut self) -> Result<ComponentId, Self::Error> {
            Ok(ComponentId {
                owner: 2,
                component_type: 1,
                id: 42,
            })
        }
        fn report(&mut self, report: Report) -> Result<(), Self::Error> {
            self.log.push("report");
            // A real synchronous subscriber can move the actor before map edit.
            if matches!(report, Report::RemoveFeature { .. }) {
                self.location.x = 10;
            }
            self.reports.push(report);
            Ok(())
        }
        fn remove_feature_at_current_location(&mut self) -> Result<(), Self::Error> {
            assert_eq!(self.location.x, 10);
            self.log.push("clear_feature");
            self.feature = NO_TYPE;
            Ok(())
        }
        fn observer_can_see_player(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn operation_hash(&mut self) -> Result<u32, Self::Error> {
            Ok(OPERATION_HASH)
        }
        fn majority_religion(&mut self, _: i32) -> Result<i32, Self::Error> {
            self.log.push("majority");
            Ok(self.religion)
        }
        fn city_count(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(self.count)
        }
        fn can_set_original_capital(&mut self, _: i32) -> Result<bool, Self::Error> {
            self.log.push("capital_gate");
            Ok(true)
        }
        fn create_city(&mut self, creation: Creation) -> Result<Option<u32>, Self::Error> {
            self.log.push("create");
            self.creation = Some(creation);
            Ok(self.city)
        }
        fn religion_auto_spreads(&mut self, _: i32) -> Result<Option<bool>, Self::Error> {
            self.log.push("auto_spread");
            Ok(Some(true))
        }
        fn pressure_needed_for_majority(&mut self, _: u32, _: i32) -> Result<i32, Self::Error> {
            self.log.push("pressure");
            Ok(self.pressure)
        }
        fn add_religious_pressure(
            &mut self,
            city: u32,
            religion: i32,
            amount: i32,
            reason: i32,
            source: i32,
        ) -> Result<(), Self::Error> {
            assert_eq!((city, religion, reason, source), (7, self.religion, 10, -1));
            self.log.push("add_pressure");
            self.pressure_added = Some(amount);
            Ok(())
        }
        fn local_observer_sees_current_location(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn delayed_destroy_unit(&mut self) -> Result<(), Self::Error> {
            self.log.push("destroy");
            if self.fail_destroy {
                Err("destroy")
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn gating_is_lazy_and_visible_mode_skips_only_location_legality() {
        let mut w = World {
            disabled: true,
            ..World::default()
        };
        assert_eq!(can_start(&mut w, false, false), Ok(false));
        assert_eq!(w.log, ["disabled"]);
        let mut w = World {
            legal: false,
            feature: 7,
            ..World::default()
        };
        assert_eq!(can_start(&mut w, true, false), Ok(true));
        assert!(!w.log.contains(&"location_gate"));
        assert_eq!(can_start(&mut w, false, false), Ok(false));
    }
    #[test]
    fn movement_precedes_null_plot_and_unremovable_feature_does_not_reject() {
        let mut w = World {
            plot: None,
            ..World::default()
        };
        assert_eq!(can_start(&mut w, false, false), Ok(false));
        assert_eq!(w.log, ["disabled", "definition", "moves", "plot"]);
        let mut w = World {
            feature: 7,
            ..World::default()
        };
        assert_eq!(can_start(&mut w, false, true), Ok(true));
        assert!(!w.log.contains(&"feature_output"));
        w.removable = true;
        assert_eq!(can_start(&mut w, false, true), Ok(true));
        assert!(w.log.contains(&"feature_output"));
        w.moves = 0;
        assert_eq!(can_start(&mut w, true, true), Ok(false));
    }
    #[test]
    fn feature_report_precedes_clear_and_creation_uses_current_location() {
        let mut w = World {
            feature: 7,
            removable: true,
            ..World::default()
        };
        let mut data = StartData {
            delete: false,
            action: false,
        };
        assert_eq!(start(&mut w, &mut data), Ok(()));
        assert_eq!(&w.log[..2], ["report", "clear_feature"]);
        assert_eq!(
            w.creation,
            Some(Creation {
                owner: 2,
                location: PlotCoord { x: 10, y: 9 },
                transfer_type: FOUNDING_TRANSFER_HASH,
                first_city: true,
                original_capital: true
            })
        );
        assert_eq!(
            w.reports.iter().map(Report::event_hash).collect::<Vec<_>>(),
            [0x878cd252, 0xb6b48d9c, 0x3d4006f2, 0x981a9186, 0xc1300ee7]
        );
        assert_eq!(w.reports[2].subtype_hash(), OPERATION_HASH);
        assert_eq!(
            data,
            StartData {
                delete: false,
                action: true
            }
        );
    }
    #[test]
    fn failed_city_creation_still_queries_religion_reports_and_consumes() {
        let mut w = World {
            city: None,
            religion: 6,
            count: 1,
            ..World::default()
        };
        let mut data = StartData {
            delete: true,
            action: false,
        };
        start(&mut w, &mut data).unwrap();
        assert!(w.log.contains(&"auto_spread"));
        assert!(!w.log.contains(&"pressure"));
        assert!(!w.log.contains(&"capital_gate"));
        assert_eq!(w.reports.last().unwrap().event_hash(), 0xc1300ee7);
        assert_eq!(w.log.last(), Some(&"destroy"));
        assert_eq!(
            data,
            StartData {
                delete: true,
                action: true
            }
        );
    }
    #[test]
    fn pressure_uses_precreation_religion_and_wrapping_q8_product() {
        let mut w = World {
            religion: 6,
            pressure: i32::MAX,
            ..World::default()
        };
        start(
            &mut w,
            &mut StartData {
                delete: false,
                action: false,
            },
        )
        .unwrap();
        assert_eq!(w.pressure_added, Some(-2));
        let a = w.log.iter().position(|v| *v == "majority").unwrap();
        let b = w.log.iter().position(|v| *v == "create").unwrap();
        let c = w.log.iter().position(|v| *v == "auto_spread").unwrap();
        assert!(a < b && b < c);
    }
    #[test]
    fn failed_destruction_retains_prefix_without_claiming_action_completion() {
        let mut w = World {
            fail_destroy: true,
            ..World::default()
        };
        let mut data = StartData {
            delete: false,
            action: false,
        };
        assert_eq!(start(&mut w, &mut data), Err("destroy"));
        assert!(w.creation.is_some());
        assert_eq!(w.reports.len(), 4);
        assert!(!data.action);
    }
}
