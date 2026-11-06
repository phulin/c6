//! Scalar payloads reached through Combat.Results' retained District pointer.
//! Placement destruction preserves these wrapper payloads. The view must be
//! taken after removal callbacks, immediately before the registry removes the
//! District. It expires if that owner's District storage is reconstructed.
use c6_core::{
    city::City,
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetiredDistrictScalars {
    pub district_id: u32,
    pub city_id: u32,
    pub location: PlotCoord,
    pub garrison_damage: i32,
    pub outer_defenses_damage: i32,
}

impl RetiredDistrictScalars {
    /// Read the current scalar values at the final destruction boundary. This
    /// does not retain vectors, a live District, or a replacement Member CID.
    pub fn before_registry_remove(state: &LoadedState, id: ComponentId) -> Result<Self, String> {
        let district = state
            .district(i32::from(id.owner), id.id)
            .map_err(|e| format!("retiring District: {e:?}"))?;
        Ok(Self {
            district_id: district.id,
            city_id: district.city_id,
            location: PlotCoord {
                x: district.location.0,
                y: district.location.1,
            },
            garrison_damage: district.garrison_damage,
            outer_defenses_damage: district.outer_defenses_damage,
        })
    }

    /// District.GetOwner follows GetPlot and the CURRENT plot ownership.
    pub fn current_owner(&self, state: &LoadedState) -> Result<i32, String> {
        let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), self.location)
            .map_err(|e| format!("retired District plot: {e:?}"))?;
        Ok(plot
            .city_attributes
            .as_ref()
            .map_or(-1, |attributes| i32::from(attributes.player)))
    }

    /// District.GetCity uses the stored numeric city ID against the CURRENT
    /// plot owner's Cities. Native Find uses its low16 slot and ignores the
    /// generation; ordinary LoadedState::city keeps its full-identity checks.
    pub fn current_city<'s>(&self, state: &'s LoadedState) -> Result<Option<&'s City>, String> {
        let owner = self.current_owner(state)?;
        if owner == -1 || self.city_id == u32::MAX {
            return Ok(None);
        }
        state
            .world()
            .player(owner)
            .map_err(|e| format!("retired District owner: {e:?}"))?;
        let player = state
            .retained()
            .players
            .iter()
            .find(|player| player.header.owner == owner)
            .ok_or_else(|| "retired District current Cities owner".to_owned())?;
        Ok(player
            .cities
            .records
            .entries
            .iter()
            .find(|entry| entry.index == (self.city_id & 0xffff) as i32)
            .map(|entry| &entry.value))
    }
}
