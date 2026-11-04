//! Exact Linux UnitContainer AddUnit/RemoveUnit logical membership changes.
//!
//! Uses the existing typed layered grid, preserving per-cell identity order.
//! Does not move a Unit, change Plot counters, update sight or run placement
//! callbacks. Those are ordered responsibilities of Placement::Set.
use crate::map::{UnitCell, UnitContainer, UnitLayer, UnitReference};
use crate::unit::{ComponentId, PlotCoord};

pub const ALL_LAYERS: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupancyError {
    DuplicateCell { layer: usize, plot_index: u32 },
    ResourceLimit,
    InvalidWorldGrid,
}
fn index(container: &UnitContainer, p: PlotCoord) -> u32 {
    (p.y as u32)
        .wrapping_mul(container.width)
        .wrapping_add(p.x as u32)
}
fn cell(
    layer: &UnitLayer,
    plot_index: u32,
    layer_index: usize,
) -> Result<Option<usize>, OccupancyError> {
    let mut found = None;
    for (i, c) in layer.cells.iter().enumerate() {
        if c.plot_index == plot_index && found.replace(i).is_some() {
            return Err(OccupancyError::DuplicateCell {
                layer: layer_index,
                plot_index,
            });
        }
    }
    Ok(found)
}

/// AddUnit (0x2a4e7f0): find the first matching layer, or append a newly
/// constructed layer even when the subsequent grid-index check returns false.
/// Existing identity returns true without reordering or duplication. New identity
/// appends to the cell. Coordinates use native wrapping linear-index arithmetic;
/// no independent x/y bounds or terrain normalization is invented here.
pub fn add_unit(
    container: &mut UnitContainer,
    id: ComponentId,
    p: PlotCoord,
    layer_id: u32,
) -> Result<bool, OccupancyError> {
    let plot = index(container, p);
    let identity = UnitReference::from_component_id(id);
    let found_layer = container.layers.iter().position(|l| l.layer_id == layer_id);
    let maximum = found_layer
        .map(|i| container.layers[i].maximum)
        .unwrap_or_else(|| container.width.wrapping_mul(container.height));
    let found_cell = if let Some(i) = found_layer {
        if plot < maximum {
            cell(&container.layers[i], plot, i)?
        } else {
            None
        }
    } else {
        None
    };
    // Reserve every needed allocation before changing the logical grid.
    if let Some(i) = found_layer {
        if plot < maximum {
            if let Some(j) = found_cell {
                if container.layers[i].cells[j].units.contains(&identity) {
                    return Ok(true);
                }
                container.layers[i].cells[j]
                    .units
                    .try_reserve(1)
                    .map_err(|_| OccupancyError::ResourceLimit)?;
            } else {
                container.layers[i]
                    .cells
                    .try_reserve(1)
                    .map_err(|_| OccupancyError::ResourceLimit)?;
            }
        }
    } else {
        container
            .layers
            .try_reserve(1)
            .map_err(|_| OccupancyError::ResourceLimit)?;
    }
    let mut new_units = Vec::new();
    let mut new_cells = Vec::new();
    if plot < maximum && found_cell.is_none() {
        new_units
            .try_reserve(1)
            .map_err(|_| OccupancyError::ResourceLimit)?;
        new_units.push(identity);
        if found_layer.is_none() {
            new_cells
                .try_reserve(1)
                .map_err(|_| OccupancyError::ResourceLimit)?;
            new_cells.push(UnitCell {
                plot_index: plot,
                units: std::mem::take(&mut new_units),
            });
        }
    }
    let layer = if let Some(i) = found_layer {
        i
    } else {
        container.layers.push(UnitLayer {
            layer_id,
            maximum,
            cells: new_cells,
        });
        return Ok(plot < maximum);
    };
    if plot >= maximum {
        return Ok(false);
    }
    if let Some(j) = found_cell {
        container.layers[layer].cells[j].units.push(identity);
    } else {
        // Native sparse grids enumerate cells by ascending slot index.
        let insertion = container.layers[layer]
            .cells
            .iter()
            .position(|c| c.plot_index > plot)
            .unwrap_or(container.layers[layer].cells.len());
        container.layers[layer].cells.insert(
            insertion,
            UnitCell {
                plot_index: plot,
                units: new_units,
            },
        );
    }
    Ok(true)
}

/// RemoveUnit (0x2a4e820): a specific layer removes the first equal ComponentID.
/// ALL_LAYERS visits every current layer in native order, removes once per layer,
/// and returns the LAST successful layer ID. No match returns UINTMAX. Empty
/// cells disappear; layers remain. The remaining identities keep their order.
pub fn remove_unit(
    container: &mut UnitContainer,
    id: ComponentId,
    p: PlotCoord,
    layer_id: u32,
) -> Result<u32, OccupancyError> {
    let plot = index(container, p);
    let identity = UnitReference::from_component_id(id);
    let specific = if layer_id == ALL_LAYERS {
        None
    } else {
        container.layers.iter().position(|l| l.layer_id == layer_id)
    };
    if layer_id != ALL_LAYERS && specific.is_none() {
        return Ok(ALL_LAYERS);
    }
    // Check all reached cells before any removal; malformed snapshots are not
    // silently interpreted as two native grid slots at the same linear index.
    for (i, l) in container.layers.iter().enumerate() {
        if (layer_id == ALL_LAYERS || specific == Some(i)) && plot < l.maximum {
            cell(l, plot, i)?;
        }
    }
    let mut removed = ALL_LAYERS;
    for (i, l) in container.layers.iter_mut().enumerate() {
        if (layer_id != ALL_LAYERS && specific != Some(i)) || plot >= l.maximum {
            continue;
        }
        if let Some(j) = cell(l, plot, i)? {
            if let Some(k) = l.cells[j].units.iter().position(|u| *u == identity) {
                l.cells[j].units.remove(k);
                if l.cells[j].units.is_empty() {
                    l.cells.remove(j);
                }
                removed = l.layer_id;
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(owner: i16, n: u32) -> ComponentId {
        ComponentId {
            owner,
            component_type: 1,
            id: n,
        }
    }
    fn grid() -> UnitContainer {
        UnitContainer {
            width: 4,
            height: 3,
            layers: vec![],
        }
    }
    const P: PlotCoord = PlotCoord { x: 2, y: 1 };
    #[test]
    fn add_preserves_order_and_suppresses_only_exact_component_identity() {
        let mut c = grid();
        for u in [id(1, 7), id(2, 7), id(1, 8), id(1, 7)] {
            assert_eq!(add_unit(&mut c, u, P, 4), Ok(true));
        }
        assert_eq!(c.layers[0].maximum, 12);
        assert_eq!(
            c.layers[0].cells[0]
                .units
                .iter()
                .map(|u| u.component_id())
                .collect::<Vec<_>>(),
            [id(1, 7), id(2, 7), id(1, 8)]
        );
        assert_eq!(remove_unit(&mut c, id(2, 7), P, 4), Ok(4));
        assert_eq!(
            c.layers[0].cells[0]
                .units
                .iter()
                .map(|u| u.component_id())
                .collect::<Vec<_>>(),
            [id(1, 7), id(1, 8)]
        );
    }
    #[test]
    fn all_layers_removes_from_every_layer_and_returns_last_success() {
        let mut c = grid();
        for l in [3, 0, 9] {
            add_unit(&mut c, id(0, 1), P, l).unwrap();
        }
        assert_eq!(remove_unit(&mut c, id(0, 1), P, ALL_LAYERS), Ok(9));
        assert!(c.layers.iter().all(|l| l.cells.is_empty()));
        assert_eq!(
            c.layers.iter().map(|l| l.layer_id).collect::<Vec<_>>(),
            [3, 0, 9]
        );
    }
    #[test]
    fn removal_keeps_first_duplicate_semantics_and_empty_cell_lifetime() {
        let mut c = grid();
        add_unit(&mut c, id(0, 1), P, 0).unwrap();
        c.layers[0].cells[0]
            .units
            .push(UnitReference::from_component_id(id(0, 1)));
        assert_eq!(remove_unit(&mut c, id(0, 1), P, 0), Ok(0));
        assert_eq!(c.layers[0].cells[0].units.len(), 1);
        remove_unit(&mut c, id(0, 1), P, 0).unwrap();
        assert!(c.layers[0].cells.is_empty());
    }
    #[test]
    fn native_linear_index_aliasing_and_failed_new_layer_are_preserved() {
        let mut c = grid();
        assert_eq!(
            add_unit(&mut c, id(0, 1), PlotCoord { x: 5, y: 0 }, 7),
            Ok(true)
        );
        assert_eq!(c.layers[0].cells[0].plot_index, 5);
        assert_eq!(
            remove_unit(&mut c, id(0, 1), PlotCoord { x: 1, y: 1 }, 7),
            Ok(7)
        );
        assert_eq!(
            add_unit(&mut c, id(0, 1), PlotCoord { x: -1, y: 0 }, 8),
            Ok(false)
        );
        assert_eq!(c.layers.len(), 2);
        assert!(c.layers[1].cells.is_empty());
    }
    #[test]
    fn malformed_reached_cell_refuses_before_all_layer_removal() {
        let mut c = grid();
        for l in [0, 1] {
            add_unit(&mut c, id(0, 1), P, l).unwrap();
        }
        let duplicate = c.layers[1].cells[0].clone();
        c.layers[1].cells.push(duplicate);
        let before = c.clone();
        assert!(matches!(
            remove_unit(&mut c, id(0, 1), P, ALL_LAYERS),
            Err(OccupancyError::DuplicateCell { .. })
        ));
        assert_eq!(c, before);
    }
}
