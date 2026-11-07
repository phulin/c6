// Included in the existing membership fixture module. Every unneeded provider
// fails, ensuring report failure cannot silently execute later sight/rules work.
type NormalTestResult<T> = Result<T, &'static str>;
macro_rules! unavailable {
    ($name:ident($($arg:ident:$ty:ty),*) -> $out:ty) => {
        fn $name(&mut self,$($arg:$ty),*) -> NormalTestResult<$out> {
            $(let _=$arg;)* Err("unreached normal placement provider")
        }
    };
}
impl PlacementStateContext for Context {
    type Error = &'static str;
    fn unit(&self) -> NormalTestResult<&Unit> {
        Ok(&self.unit)
    }
    fn unit_mut(&mut self) -> NormalTestResult<&mut Unit> {
        Ok(&mut self.unit)
    }
    fn preflight_placement_report(&self, _: u32) -> NormalTestResult<()> {
        Ok(())
    }
    fn send_placement_report(&mut self, r: PlacementReport) -> NormalTestResult<()> {
        if r.event_hash == 0x9deb82e7 {
            assert_eq!(r.payload, PlacementPayload::Coordinate(NEW));
            self.calls.push("relocation coordinate report failure");
            return Err("relocation coordinate report failed");
        }
        assert_eq!(r.event_hash, 0xbe121408);
        assert_eq!(
            r.identity,
            ComponentId {
                owner: 0,
                component_type: 1,
                id: 9
            }
        );
        assert_eq!(
            r.payload,
            PlacementPayload::Moved {
                location: NEW,
                locally_visible: true,
                newly_locally_visible: true,
                embark_event: u32::MAX,
                parent: ComponentId {
                    owner: -1,
                    component_type: 0,
                    id: u32::MAX
                }
            }
        );
        self.calls.push("normal report failure");
        Err("normal report failed")
    }
}
impl crate::unit_healing::UnitHealingContext for Context {
    type Error = &'static str;
    fn unit(&self) -> NormalTestResult<&Unit> {
        Ok(&self.unit)
    }
    fn unit_mut(&mut self) -> NormalTestResult<&mut Unit> {
        Ok(&mut self.unit)
    }
    fn healing_world_state(&self) -> NormalTestResult<crate::unit_healing::HealingWorldState> {
        Err("unused")
    }
    fn healing_rules(&self) -> NormalTestResult<crate::unit_healing::HealingRules> {
        Err("unused")
    }
    fn preflight_healing_effects(&self) -> NormalTestResult<()> {
        Err("unused")
    }
    fn combat_max_hit_points(&self) -> NormalTestResult<i32> {
        Err("unused")
    }
    fn maximum_damage_for_report(&self) -> NormalTestResult<i32> {
        Err("unused")
    }
    unavailable!(send_damage_changed(r:crate::unit_healing::DamageChangedReport)->());
    unavailable!(clear_current_operations()->());
    unavailable!(dispatch_unit_healed(s:crate::unit_healing::UnitHealedSignal)->());
}
impl crate::unit_sight::SightContext for Context {
    type Error = &'static str;
    type Map = ();
    type VisibilityManager = ();
    type Visibility = ();
    type Player = ();
    type Plot = ();
    fn unit(&self) -> NormalTestResult<&Unit> {
        Ok(&self.unit)
    }
    fn unit_mut(&mut self) -> NormalTestResult<&mut Unit> {
        Ok(&mut self.unit)
    }
    fn preflight_sight(&self, _: bool) -> NormalTestResult<()> {
        Err("unreached sight")
    }
    unavailable!(sight_plots(o:i32,r:i32,p:PlotCoord,f:bool,t:bool)->Vec<crate::unit::SightPlotEntry>);
    unavailable!(globals_map()->());
    unavailable!(map_instance()->());
    unavailable!(visibility_manager()->());
    unavailable!(owner_visibility(m:(),o:i32)->());
    unavailable!(player(o:i32)->());
    unavailable!(domain()->i32);
    unavailable!(plot_index(m:(),p:PlotCoord)->i32);
    unavailable!(plot_xy(m:(),i:i32)->PlotCoord);
    unavailable!(plot(m:(),i:u32)->());
    unavailable!(water(p:())->bool);
    unavailable!(feature(p:())->i32);
    unavailable!(revealed_feature(p:(),f:i32)->bool);
    unavailable!(is_revealed(v:(),i:u32)->bool);
    unavailable!(change_visibility(m:(),c:crate::unit_sight::VisibilityChange)->i32);
    unavailable!(promotion_class()->Option<&str>);
    unavailable!(natural_wonder_experience()->i32);
    unavailable!(experience_modifier()->i32);
    unavailable!(change_experience(a:i32)->());
    unavailable!(plot_visibility_changed(p:(),u:(),s:ComponentId,o:i32,f:bool)->());
    unavailable!(signal_unit_plot(p:())->());
}
impl crate::improvement_movement::Context for Context {
    type Map = ();
    type Plot = ();
    type Diplomacy = ();
    type Definition = ();
    type AlivePlayers = ();
    fn preflight_movement_damage(&self) -> NormalTestResult<()> {
        Err("unreached damage")
    }
    unavailable!(barbarian_camp_improvement()->u32);
    unavailable!(unit_is_barbarian()->bool);
    unavailable!(clans_active()->bool);
    unavailable!(unit_plot()->Option<()>);
    unavailable!(improvement_type(p:())->u32);
    unavailable!(improvement_owner(p:())->i32);
    unavailable!(improvement_pillaged(p:())->bool);
    unavailable!(improvement_at(k:u32)->());
    unavailable!(improvement_find(k:u32)->Option<()>);
    unavailable!(remove_on_entry(d:())->bool);
    unavailable!(map_and_diplomacy()->((),()));
    unavailable!(plot_at(m:(),p:PlotCoord)->Option<()>);
    unavailable!(plot_location(p:())->PlotCoord);
    unavailable!(plot_index(p:())->i32);
    unavailable!(at_war(d:(),o:i32)->bool);
    unavailable!(property(d:(),k:u32)->Option<i32>);
    unavailable!(update_improvement_visibility(p:(),d:i32)->());
    unavailable!(kill(d:bool,a:i32)->());
    unavailable!(alive_players()->());
    unavailable!(alive_player_count(p:())->usize);
    unavailable!(alive_player(p:(),i:usize)->i32);
    unavailable!(signal_improvement_damage(o:i32,a:i32)->());
    unavailable!(set_dynamic_integer(k:u32,v:i32)->());
}
impl normal_move::Context for Context {
    type PlacementPlot = ();
    fn validate_location(
        &mut self,
        p: PlotCoord,
        f: u32,
    ) -> NormalTestResult<normal_move::Validation> {
        assert_eq!(p, NEW);
        assert_eq!(f, 0x108);
        Ok(normal_move::Validation::Exact)
    }
    fn preflight_normal_placement(&self, _: PlotCoord) -> NormalTestResult<()> {
        if self.admitted {
            Ok(())
        } else {
            Err("missing normal effects")
        }
    }
    fn local_player(&mut self) -> NormalTestResult<i32> {
        Ok(0)
    }
    fn plot_distance(&mut self, a: PlotCoord, b: PlotCoord) -> NormalTestResult<i32> {
        assert_eq!((a, b), (OLD, NEW));
        Ok(1)
    }
    unavailable!(outgoing_visibility_to(o:i32,v:i32)->bool);
    unavailable!(outgoing_visibility_blocked(o:i32,v:i32)->bool);
    unavailable!(local_revealed_state(p:PlotCoord)->i32);
    unavailable!(visibility_messages_disabled()->bool);
    unavailable!(embarked_state_change(t:bool,o:bool)->());
    unavailable!(placement_plot(p:PlotCoord)->Option<()>);
    unavailable!(plot_improvement(p:())->u32);
    unavailable!(improvement_fortification(k:u32)->i32);
    unavailable!(plot_improvement_pillaged(p:())->bool);
    unavailable!(buildings_at_plot(p:())->Vec<u32>);
    unavailable!(building_fortification(k:u32)->i32);
    unavailable!(signal_unit_moved(o:PlotCoord,n:PlotCoord)->());
    unavailable!(plot_is_city(p:())->bool);
    unavailable!(plot_district_type(p:())->u32);
    unavailable!(owning_city(p:())->ComponentId);
    unavailable!(plot_district(p:())->ComponentId);
    unavailable!(city_unit_changed(c:ComponentId)->());
    unavailable!(check_siege_near(p:PlotCoord,o:i32)->());
    unavailable!(award_post_move_bonuses(p:PlotCoord)->());
    unavailable!(notify_barbarian_approach(p:PlotCoord)->());
    unavailable!(diplomacy_unit_moved(p:PlotCoord)->());
    unavailable!(rules_unit_moved(p:PlotCoord)->());
}
#[test]
fn normal_placement_unknown_effects_reject_before_detaching() {
    let mut c = Context::new();
    c.admitted = false;
    let before = c.world.map().clone();
    let unit = c.unit.clone();
    assert_eq!(
        normal_move::set(&mut c, NEW, 0x108),
        Err(normal_move::Error::Context("missing normal effects"))
    );
    assert_eq!(c.world.map(), &before);
    assert_eq!(c.unit, unit);
    assert!(c.calls.is_empty());
}
#[test]
fn normal_movement_report_failure_retains_membership_and_completion_prefix() {
    let mut c = Context::new();
    c.unit.b_needs_movement_completion = false;
    assert_eq!(
        normal_move::set(&mut c, NEW, 0x108),
        Err(normal_move::Error::Context("normal report failed"))
    );
    assert_eq!(c.unit.k_location, NEW);
    assert_eq!(c.formation_location, NEW);
    assert!(c.unit.b_needs_movement_completion);
    assert_eq!(
        (
            c.world.map().plots[0].unit_count,
            c.world.map().plots[1].unit_count
        ),
        (0, 1)
    );
    assert_eq!(c.world.plot_units(0, 1).unwrap()[0].id, 9);
    assert_eq!(c.calls.last(), Some(&"normal report failure"));
}

#[test]
fn relocation_skips_validation_and_reports_coordinate_without_movement_animation() {
    let mut c = Context::new();
    c.unit.b_needs_movement_completion = false;
    // The validator above only accepts flags0x108: an unintended validation
    // call on this native flags0 path fails the test before membership changes.
    assert_eq!(
        normal_move::set_relocation(&mut c, NEW),
        Err(normal_move::Error::Context(
            "relocation coordinate report failed"
        ))
    );
    assert_eq!(c.unit.k_location, NEW);
    assert_eq!(c.formation_location, NEW);
    assert!(!c.unit.b_needs_movement_completion);
    assert_eq!(c.world.map().plots[0].unit_count, 0);
    assert_eq!(c.world.map().plots[1].unit_count, 1);
    assert_eq!(
        c.calls.last(),
        Some(&"relocation coordinate report failure")
    );
}
