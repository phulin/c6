//! Linux Sight::Manager::UpdateMap and its ordered DoUpdateMap effects.
//!
//! Computes visibility-change requests and maintains the real Unit sight list.
//! Raycasting, visibility sharing/count writes, contact/discovery, experience and
//! synchronous signals are required live providers. No no-op or RNG substitute
//! is supplied, and this is not complete Placement::Set or MoveTo.
use crate::unit::{ComponentId, PlotCoord, SightPlotEntry, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibilityChange {
    pub owner: i32,
    pub plot_index: u32,
    pub delta: i32,
    pub hidden_type: i32,
    /// Native first bool argument is always true for Unit sight; the stock
    /// visibility manager does not read it.
    pub source_flag: bool,
    /// Native second bool argument is the signed ring comparison <2.
    pub near_origin: bool,
    pub origin_index: i32,
    pub sight_range: i32,
    pub source: ComponentId,
}

/// Handles retain the actual receiver objects across synchronous callbacks.
/// Required effects must be supported before update_map starts. All player
/// choices reached by contact/discovery yield to the arena's external policy.
pub trait SightContext {
    type Error;
    type Map: Copy;
    type VisibilityManager: Copy;
    type Visibility: Copy;
    type Player: Copy;
    type Plot: Copy;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    fn preflight_sight(&self, add: bool) -> Result<(), Self::Error>;
    /// GetSightPlots: exact native raycasting/order, not all tiles in a radius.
    fn sight_plots(
        &mut self,
        owner: i32,
        range: i32,
        origin: PlotCoord,
        through_features: bool,
        through_terrain: bool,
    ) -> Result<Vec<SightPlotEntry>, Self::Error>;
    fn globals_map(&mut self) -> Result<Self::Map, Self::Error>;
    fn map_instance(&mut self) -> Result<Self::Map, Self::Error>;
    fn visibility_manager(&mut self) -> Result<Self::VisibilityManager, Self::Error>;
    fn owner_visibility(
        &mut self,
        manager: Self::VisibilityManager,
        owner: i32,
    ) -> Result<Self::Visibility, Self::Error>;
    fn player(&mut self, owner: i32) -> Result<Self::Player, Self::Error>;
    fn domain(&mut self) -> Result<i32, Self::Error>;
    /// Actual runtime HiddenVisibilityTypes enum. The saved SightModifiers
    /// field is definition-hash serialized and must be resolved, not cast.
    fn hidden_type(&mut self) -> Result<i32, Self::Error>;
    fn plot_index(&mut self, map: Self::Map, origin: PlotCoord) -> Result<i32, Self::Error>;
    fn plot_xy(&mut self, map: Self::Map, index: i32) -> Result<PlotCoord, Self::Error>;
    fn plot(&mut self, map: Self::Map, index: u32) -> Result<Self::Plot, Self::Error>;
    fn water(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn revealed_feature(&mut self, player: Self::Player, feature: i32)
        -> Result<bool, Self::Error>;
    fn is_revealed(
        &mut self,
        visibility: Self::Visibility,
        index: u32,
    ) -> Result<bool, Self::Error>;
    /// Complete native Manager::ChangeVisibilityCount, including sharing,
    /// revealed state, resources, ViewPlotContents and propagated callbacks.
    /// Return its real integer result; 2 selects the following discovery tail.
    fn change_visibility(
        &mut self,
        manager: Self::VisibilityManager,
        change: VisibilityChange,
    ) -> Result<i32, Self::Error>;
    fn promotion_class(&mut self) -> Result<Option<&str>, Self::Error>;
    fn natural_wonder_experience(&mut self) -> Result<i32, Self::Error>;
    /// Actual GetExperienceModifier(false), signed Q8; no assumed 1.0 modifier.
    fn experience_modifier(&mut self) -> Result<i32, Self::Error>;
    /// Full ChangeExperience, including promotion/notification/callback effects.
    fn change_experience(&mut self, amount: i32) -> Result<(), Self::Error>;
    /// Rules.Players.OnPlotVisibilityChanged(plot, retained player, source,
    /// NO_PLAYER, !previously_revealed), including gameplay/RNG/contact effects.
    fn plot_visibility_changed(
        &mut self,
        plot: Self::Plot,
        player: Self::Player,
        source: ComponentId,
        other_player: i32,
        first_reveal: bool,
    ) -> Result<(), Self::Error>;
    /// Game Signals+0x10c8 Unit/Plot dispatch, after the Rules continuation.
    fn signal_unit_plot(&mut self, plot: Self::Plot) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SightError<E> {
    EffectsUnavailable(E),
    Context(E),
    ResourceLimit,
    InvalidatedSightCursor { index: usize },
}

enum Entries<'a> {
    New(&'a [SightPlotEntry]),
    Current,
}
fn entry<C: SightContext>(
    c: &C,
    entries: &Entries<'_>,
    index: usize,
) -> Result<Option<SightPlotEntry>, SightError<C::Error>> {
    let entries = match entries {
        Entries::New(v) => *v,
        Entries::Current => &c.unit().map_err(SightError::Context)?.k_sight_plots,
    };
    if index == entries.len() {
        return Ok(None);
    }
    entries
        .get(index)
        .cloned()
        .map(Some)
        .ok_or(SightError::InvalidatedSightCursor { index })
}

/// FixedPointT<8> multiplication and AsInt: signed i64 product, logical shift,
/// then native low32 storage and a second arithmetic shift for whole experience.
fn experience_amount(base: i32, modifier: i32) -> i32 {
    (((i64::from(base.wrapping_shl(8)) * i64::from(modifier)) as u64) >> 8) as i32 >> 8
}

fn apply<C: SightContext>(
    c: &mut C,
    origin: PlotCoord,
    entries: Entries<'_>,
    add: bool,
) -> Result<(), SightError<C::Error>> {
    use SightError::Context;
    let range = c.unit().map_err(Context)?.k_sight_modifiers.i_sight_range;
    let map = c.globals_map().map_err(Context)?;
    let manager = c.visibility_manager().map_err(Context)?;
    let hidden = c.hidden_type().map_err(Context)?;
    let owner = c.unit().map_err(Context)?.e_owner;
    let origin_index = c.plot_index(map, origin).map_err(Context)?;
    let visibility = c.owner_visibility(manager, owner).map_err(Context)?;
    let player = c.player(owner).map_err(Context)?;
    let domain = c.domain().map_err(Context)?;
    let u = c.unit().map_err(Context)?;
    let source = ComponentId {
        owner: u.e_owner as i16,
        component_type: 1,
        id: u.id,
    };
    let water_hidden = if domain == 0 { hidden } else { -1 };
    let land_hidden = if domain == 2 { hidden } else { -1 };
    let mut index = 0;
    while let Some(entry) = entry(c, &entries, index)? {
        let plot = c.plot(map, entry.ui_index).map_err(Context)?;
        let hidden_type = if c.water(plot).map_err(Context)? {
            water_hidden
        } else {
            land_hidden
        };
        let feature = c.feature(plot).map_err(Context)?;
        let old_feature = c.revealed_feature(player, feature).map_err(Context)?;
        let old_revealed = c.is_revealed(visibility, entry.ui_index).map_err(Context)?;
        let result = c
            .change_visibility(
                manager,
                VisibilityChange {
                    owner,
                    plot_index: entry.ui_index,
                    delta: if add { 1 } else { -1 },
                    hidden_type,
                    source_flag: true,
                    near_origin: entry.i_ring < 2,
                    origin_index,
                    sight_range: range,
                    source,
                },
            )
            .map_err(Context)?;
        if result == 2 {
            let feature = c.feature(plot).map_err(Context)?;
            let new_feature = c.revealed_feature(player, feature).map_err(Context)?;
            if !old_feature
                && !old_revealed
                && new_feature
                && c.promotion_class().map_err(Context)? == Some("PROMOTION_CLASS_RECON")
            {
                let base = c.natural_wonder_experience().map_err(Context)?;
                let modifier = c.experience_modifier().map_err(Context)?;
                c.change_experience(experience_amount(base, modifier))
                    .map_err(Context)?;
            }
            c.plot_visibility_changed(plot, player, source, -1, !old_revealed)
                .map_err(Context)?;
            c.signal_unit_plot(plot).map_err(Context)?;
        }
        index += 1;
    }
    Ok(())
}

/// Sight.Manager.UpdateMap (0x2c45a30). Add new visibility BEFORE subtracting old
/// sight; then replace the authoritative Unit sight vector. Range<=0 still
/// removes old sight. Removal derives origin from the first old entry, ignoring
/// the supplied origin. Existing sight cursor storage must remain valid during
/// its callbacks; invalidating reallocation/erase is an unsupported context.
/// Errors retain already executed visibility/contact/RNG effects and writes.
pub fn update_map<C: SightContext>(
    c: &mut C,
    origin: PlotCoord,
    add: bool,
) -> Result<(), SightError<C::Error>> {
    use SightError::Context;
    c.preflight_sight(add)
        .map_err(SightError::EffectsUnavailable)?;
    let mut new = Vec::new();
    if add {
        let range = c.unit().map_err(Context)?.k_sight_modifiers.i_sight_range;
        if range > 0 {
            let owner = c.unit().map_err(Context)?.e_owner;
            let through_features = c.unit().map_err(Context)?.b_can_see_through_features;
            let through_terrain = c.unit().map_err(Context)?.b_can_see_through_terrain;
            new = c
                .sight_plots(owner, range, origin, through_features, through_terrain)
                .map_err(Context)?;
            apply(c, origin, Entries::New(&new), true)?;
        }
    }
    if let Some(first) = c.unit().map_err(Context)?.k_sight_plots.first() {
        let first_index = first.ui_index as i32;
        let map = c.map_instance().map_err(Context)?;
        let old_origin = c.plot_xy(map, first_index).map_err(Context)?;
        apply(c, old_origin, Entries::Current, false)?;
        c.unit_mut().map_err(Context)?.k_sight_plots.clear();
    } else if add {
        // Native add path edits/clears the vector even when already empty.
        c.unit_mut().map_err(Context)?.k_sight_plots.clear();
    }
    if add && !new.is_empty() {
        let sight = &mut c.unit_mut().map_err(Context)?.k_sight_plots;
        sight
            .try_reserve(new.len())
            .map_err(|_| SightError::ResourceLimit)?;
        sight.extend(new);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plot(index: u32, ring: i32) -> SightPlotEntry {
        SightPlotEntry {
            ui_index: index,
            i_ring: ring,
        }
    }
    struct Context {
        unit: Unit,
        new: Vec<SightPlotEntry>,
        counts: Vec<i32>,
        revealed: Vec<bool>,
        features: Vec<bool>,
        changes: Vec<VisibilityChange>,
        calls: Vec<String>,
        admitted: bool,
        domain: i32,
        mutate_owner: bool,
        fail_rules: bool,
        experience: Vec<i32>,
        rng_effects: u32,
    }
    impl Context {
        fn new() -> Self {
            let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
            unit.id = 9;
            unit.e_owner = 0;
            unit.k_sight_modifiers.i_sight_range = 2;
            unit.k_sight_modifiers.e_can_see_hidden_type = 7;
            unit.k_sight_plots = vec![plot(1, 0), plot(4, 1)];
            let mut counts = vec![0; 8];
            counts[1] = 1;
            counts[4] = 1;
            Self {
                unit,
                new: vec![plot(4, 0), plot(5, 2)],
                counts,
                revealed: vec![true; 8],
                features: vec![true; 8],
                changes: vec![],
                calls: vec![],
                admitted: true,
                domain: 2,
                mutate_owner: false,
                fail_rules: false,
                experience: vec![],
                rng_effects: 0,
            }
        }
    }
    impl SightContext for Context {
        type Error = &'static str;
        type Map = u32;
        type VisibilityManager = u32;
        type Visibility = i32;
        type Player = i32;
        type Plot = u32;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn preflight_sight(&self, _: bool) -> Result<(), Self::Error> {
            if self.admitted {
                Ok(())
            } else {
                Err("unknown sight effects")
            }
        }
        fn sight_plots(
            &mut self,
            owner: i32,
            range: i32,
            origin: PlotCoord,
            _: bool,
            _: bool,
        ) -> Result<Vec<SightPlotEntry>, Self::Error> {
            self.calls
                .push(format!("raycast:{owner}:{range}:{}", origin.x));
            Ok(self.new.clone())
        }
        fn globals_map(&mut self) -> Result<u32, Self::Error> {
            Ok(1)
        }
        fn map_instance(&mut self) -> Result<u32, Self::Error> {
            self.calls.push("map instance".into());
            Ok(2)
        }
        fn visibility_manager(&mut self) -> Result<u32, Self::Error> {
            Ok(3)
        }
        fn owner_visibility(&mut self, _: u32, owner: i32) -> Result<i32, Self::Error> {
            Ok(owner)
        }
        fn player(&mut self, owner: i32) -> Result<i32, Self::Error> {
            Ok(owner)
        }
        fn domain(&mut self) -> Result<i32, Self::Error> {
            Ok(self.domain)
        }
        fn hidden_type(&mut self) -> Result<i32, Self::Error> {
            Ok(7)
        }
        fn plot_index(&mut self, map: u32, p: PlotCoord) -> Result<i32, Self::Error> {
            assert_eq!(map, 1);
            Ok(p.x)
        }
        fn plot_xy(&mut self, map: u32, index: i32) -> Result<PlotCoord, Self::Error> {
            assert_eq!(map, 2);
            self.calls.push(format!("old origin:{index}"));
            Ok(PlotCoord { x: index, y: 0 })
        }
        fn plot(&mut self, map: u32, index: u32) -> Result<u32, Self::Error> {
            assert_eq!(map, 1);
            Ok(index)
        }
        fn water(&mut self, p: u32) -> Result<bool, Self::Error> {
            Ok(p == 5)
        }
        fn feature(&mut self, p: u32) -> Result<i32, Self::Error> {
            Ok(p as i32)
        }
        fn revealed_feature(&mut self, _: i32, f: i32) -> Result<bool, Self::Error> {
            Ok(self.features[f as usize])
        }
        fn is_revealed(&mut self, _: i32, i: u32) -> Result<bool, Self::Error> {
            Ok(self.revealed[i as usize])
        }
        fn change_visibility(&mut self, _: u32, v: VisibilityChange) -> Result<i32, Self::Error> {
            self.calls
                .push(format!("count:{}:{}", v.plot_index, v.delta));
            self.changes.push(v);
            let i = v.plot_index as usize;
            self.counts[i] += v.delta;
            if self.mutate_owner {
                self.mutate_owner = false;
                self.unit.e_owner = 7;
            }
            if self.counts[i] == 1 && v.delta == 1 {
                self.revealed[i] = true;
                self.features[i] = true;
                Ok(2)
            } else {
                Ok(0)
            }
        }
        fn promotion_class(&mut self) -> Result<Option<&str>, Self::Error> {
            Ok(Some("PROMOTION_CLASS_RECON"))
        }
        fn natural_wonder_experience(&mut self) -> Result<i32, Self::Error> {
            Ok(5)
        }
        fn experience_modifier(&mut self) -> Result<i32, Self::Error> {
            Ok(384)
        }
        fn change_experience(&mut self, amount: i32) -> Result<(), Self::Error> {
            self.calls.push(format!("experience:{amount}"));
            self.experience.push(amount);
            Ok(())
        }
        fn plot_visibility_changed(
            &mut self,
            p: u32,
            player: i32,
            source: ComponentId,
            other: i32,
            first: bool,
        ) -> Result<(), Self::Error> {
            assert_eq!(other, -1);
            self.calls
                .push(format!("rules:{p}:{player}:{}:{first}", source.owner));
            self.rng_effects += 1;
            if self.fail_rules {
                Err("contact failed")
            } else {
                Ok(())
            }
        }
        fn signal_unit_plot(&mut self, p: u32) -> Result<(), Self::Error> {
            self.calls.push(format!("signal:{p}"));
            Ok(())
        }
    }
    const NEW: PlotCoord = PlotCoord { x: 4, y: 0 };
    #[test]
    fn adds_new_before_removing_old_and_replaces_actual_cached_entries() {
        let mut c = Context::new();
        update_map(&mut c, NEW, true).unwrap();
        assert_eq!(
            c.changes
                .iter()
                .map(|v| (v.plot_index, v.delta))
                .collect::<Vec<_>>(),
            [(4, 1), (5, 1), (1, -1), (4, -1)]
        );
        assert_eq!(c.counts[4], 1);
        assert_eq!(c.counts[5], 1);
        assert_eq!(c.counts[1], 0);
        assert_eq!(c.unit.k_sight_plots, c.new);
        assert_eq!(
            c.changes.iter().map(|v| v.origin_index).collect::<Vec<_>>(),
            [4, 4, 1, 1]
        );
        assert_eq!(
            c.changes.iter().map(|v| v.hidden_type).collect::<Vec<_>>(),
            [7, -1, 7, 7]
        );
        assert!(!c.changes[1].near_origin);
        assert!(c.changes[0].near_origin);
        assert!(c.changes.iter().all(|v| v.source_flag));
    }
    #[test]
    fn first_natural_wonder_experience_precedes_contact_and_signal() {
        let mut c = Context::new();
        c.revealed[5] = false;
        c.features[5] = false;
        update_map(&mut c, NEW, true).unwrap();
        assert_eq!(c.experience, [7]);
        let count = c.calls.iter().position(|v| v == "count:5:1").unwrap();
        assert_eq!(
            &c.calls[count + 1..count + 4],
            ["experience:7", "rules:5:0:0:true", "signal:5"]
        );
        assert_eq!(c.rng_effects, 1);
    }
    #[test]
    fn old_removal_recaptures_owner_after_new_callbacks() {
        let mut c = Context::new();
        c.mutate_owner = true;
        update_map(&mut c, NEW, true).unwrap();
        assert_eq!(
            c.changes.iter().map(|v| v.owner).collect::<Vec<_>>(),
            [0, 0, 7, 7]
        );
        assert_eq!(
            c.changes.iter().map(|v| v.source.owner).collect::<Vec<_>>(),
            [0, 0, 7, 7]
        );
    }
    #[test]
    fn nonpositive_range_still_subtracts_old_and_removal_ignores_supplied_origin() {
        let mut c = Context::new();
        c.unit.k_sight_modifiers.i_sight_range = 0;
        update_map(&mut c, NEW, true).unwrap();
        assert_eq!(
            c.changes
                .iter()
                .map(|v| (v.plot_index, v.delta))
                .collect::<Vec<_>>(),
            [(1, -1), (4, -1)]
        );
        assert!(c.unit.k_sight_plots.is_empty());
        assert!(!c.calls.iter().any(|v| v.starts_with("raycast")));
        let mut c = Context::new();
        update_map(&mut c, PlotCoord { x: -123, y: -123 }, false).unwrap();
        assert!(c.changes.iter().all(|v| v.origin_index == 1));
        assert!(c.unit.k_sight_plots.is_empty());
    }
    #[test]
    fn unavailable_effects_are_inert_and_contact_failure_retains_visibility_prefix() {
        let mut c = Context::new();
        c.admitted = false;
        let before = c.unit.k_sight_plots.clone();
        assert!(matches!(
            update_map(&mut c, NEW, true),
            Err(SightError::EffectsUnavailable(_))
        ));
        assert_eq!(c.unit.k_sight_plots, before);
        assert!(c.changes.is_empty());
        c.admitted = true;
        c.fail_rules = true;
        assert_eq!(
            update_map(&mut c, NEW, true),
            Err(SightError::Context("contact failed"))
        );
        assert_eq!(c.counts[4], 2);
        assert_eq!(c.counts[5], 1);
        assert_eq!(c.unit.k_sight_plots, before);
        assert_eq!(c.rng_effects, 1);
        assert!(!c.calls.iter().any(|v| v == "signal:5"));
    }
    #[test]
    fn fixed_experience_keeps_wrapping_shift_and_signed_floor() {
        assert_eq!(experience_amount(-1, 128), -1);
        assert_eq!(experience_amount(5, 384), 7);
        assert_eq!(experience_amount(1 << 24, 256), 0);
    }
}
