//! Linux XP2 Formation path-cache control flow.
//!
//! This is the cache/replanning boundary, not the movement search algorithm.
//! The provider must execute the real two-layer search and its callbacks.
use crate::pathfinder::{PathCallbacks, TwoLayer};
use crate::unit::{Formation, PathNode, PlotCoord};

/// Additional Formation::GeneratePath inputs. Domain membership must retain
/// actual unit lookup semantics; visibility must use the formation owner.
pub trait GenerationContext: PathCallbacks {
    /// The same retained Formation; no detached copy and later copyback.
    fn formation(&self) -> &Formation;
    fn formation_mut(&mut self) -> Result<&mut Formation, Self::Error>;
    fn includes_unit_of_domain(&mut self, domain: i32) -> Result<bool, Self::Error>;
    fn is_plot(&mut self, location: PlotCoord) -> Result<bool, Self::Error>;
    fn is_visible(&mut self, location: PlotCoord) -> Result<bool, Self::Error>;
    /// TwoLayer::GeneratePath's engine-utility gate (0x2a54245). This is a
    /// resolved live service input, not an assumed default.
    fn can_generate(&mut self) -> Result<bool, Self::Error>;
}

/// Formation::Movement::GeneratePath (0x2948d20). The caller retains the same
/// acquired search instance throughout this call. Pool locking is outside this
/// standalone function. Cache writes and metadata occur even for native false.
pub fn generate_formation_path<C: GenerationContext>(
    search: &mut TwoLayer,
    context: &mut C,
    destination: PlotCoord,
    mut flags: u32,
    reuse: bool,
    turns: Option<&mut i32>,
) -> Result<bool, C::Error> {
    if !context.formation().has_moved {
        flags |= 0x100;
    }
    if context.formation().domain == 0
        && context.formation().unit_ids.len() >= 2
        && (context.includes_unit_of_domain(2)? || context.includes_unit_of_domain(4)?)
    {
        flags |= 0x2000;
    }
    let found = if context.can_generate()? {
        let source = context.formation().location;
        search.generate(context, source, destination, flags, reuse)?
    } else {
        false
    };
    context.formation_mut()?.last_path_cache_location = destination;
    context.formation_mut()?.last_path = search.copy_path();
    let count = context.formation().last_path.len();
    for index in (0..count).rev() {
        let at = location(&context.formation().last_path[index]);
        if context.is_plot(at)? && !context.is_visible(at)? {
            context.formation_mut()?.last_path[index].flags |= 1;
            if index + 2 < count {
                context.formation_mut()?.last_path[index + 1].flags |= 2;
            }
            let at = location(&context.formation().last_path[0]);
            if context.is_plot(at)? && !context.is_visible(at)? {
                context.formation_mut()?.last_path[0].flags |= 1;
            }
            break;
        }
    }
    if let Some(turns) = turns {
        *turns = if found {
            context
                .formation()
                .last_path
                .first()
                .map_or(i32::MAX, |node| node.data2)
        } else {
            i32::MAX
        };
    }
    Ok(found)
}

/// Live inputs omitted from the Formation wire record must be supplied explicitly.
pub trait PathCacheContext {
    type Error;
    fn formation(&self) -> &Formation;
    fn is_evaluating_operation(&self) -> Result<bool, Self::Error>;
    /// Map lookup followed by owner visibility. Null plot returns false.
    fn plot_is_visible(&mut self, location: PlotCoord) -> Result<bool, Self::Error>;
    fn is_plot(&mut self, location: PlotCoord) -> Result<bool, Self::Error>;
    /// Formation::Movement::GeneratePath, including cache writes on failure.
    fn generate_path(&mut self, destination: PlotCoord, flags: u32) -> Result<bool, Self::Error>;
}

fn location(node: &PathNode) -> PlotCoord {
    PlotCoord {
        x: i32::from(node.x),
        y: i32::from(node.y),
    }
}

/// ResultNodeArray::GetTurnDest (0x2a51d40). Native paths are destination-first.
/// Scan from the origin; retain the last node in the requested turn before a
/// strictly greater turn. Do not assume monotonically increasing turn fields.
pub fn turn_destination(path: &[PathNode], turn: i32) -> Option<&PathNode> {
    let mut nodes = path.iter().rev();
    let mut current = nodes.next()?;
    let mut matched = current.data2 == turn;
    for node in nodes {
        if matched && node.data2 > turn {
            return Some(current);
        }
        current = node;
        matched = node.data2 == turn;
    }
    matched.then_some(current)
}

/// UpdatePathCache (0x2948820). Evaluating operations may retain an existing
/// path, replan to a newly visible former endpoint, or use turn one as a
/// temporary target. Every failed intermediate replan retains its native cache
/// mutation before a distinct requested target is tried.
pub fn update_path_cache<C: PathCacheContext>(
    c: &mut C,
    destination: PlotCoord,
    flags: u32,
) -> Result<bool, C::Error> {
    if !c.is_evaluating_operation()? {
        return c.generate_path(destination, flags);
    }
    let path = &c.formation().last_path;
    if path.len() < 2 {
        return if c.formation().last_path_cache_location == destination {
            Ok(false)
        } else {
            c.generate_path(destination, flags)
        };
    }
    let first = path[0];
    if first.flags & 1 != 0 && c.plot_is_visible(location(&first))? {
        let intermediate = location(&first);
        if c.generate_path(intermediate, flags)? {
            return Ok(true);
        }
        return if intermediate == destination {
            Ok(false)
        } else {
            c.generate_path(destination, flags)
        };
    }
    let path = &c.formation().last_path;
    if path[path.len() - 2].flags & 1 == 0 {
        return Ok(true);
    }
    let intermediate = turn_destination(path, 1).map(location);
    let Some(intermediate) = intermediate else {
        return c.generate_path(destination, flags);
    };
    if !c.is_plot(intermediate)? {
        return Ok(false);
    }
    if c.generate_path(intermediate, flags)? {
        return Ok(true);
    }
    if intermediate == destination {
        Ok(false)
    } else {
        c.generate_path(destination, flags)
    }
}

/// AddPathMetaData (0x2948e60). Mark the first hidden plot found from the
/// origin, and the following stored node only with the native strict bound.
/// Existing flags are retained. The destination is checked again independently.
pub fn add_path_metadata<E>(
    path: &mut [PathNode],
    mut is_plot: impl FnMut(PlotCoord) -> Result<bool, E>,
    mut is_visible: impl FnMut(PlotCoord) -> Result<bool, E>,
) -> Result<(), E> {
    for index in (0..path.len()).rev() {
        let at = location(&path[index]);
        if is_plot(at)? && !is_visible(at)? {
            path[index].flags |= 1;
            if index + 2 < path.len() {
                path[index + 1].flags |= 2;
            }
            let at = location(&path[0]);
            if is_plot(at)? && !is_visible(at)? {
                path[0].flags |= 1;
            }
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_preserves_strict_following_node_bound() {
        let node = |x| PathNode {
            data1_raw: 0,
            data2: 1,
            flags: 8,
            x,
            y: 0,
        };
        let mut path = vec![node(0), node(1), node(2), node(3)];
        let mut reads = Vec::new();
        add_path_metadata(
            &mut path,
            |_| Ok::<_, ()>(true),
            |p| {
                reads.push(p.x);
                Ok(p.x != 2 && p.x != 0)
            },
        )
        .unwrap();
        assert_eq!(
            path.iter().map(|n| n.flags).collect::<Vec<_>>(),
            vec![9, 8, 9, 8]
        );
        assert_eq!(reads, vec![3, 2, 0]);
        let mut path = vec![node(0), node(1), node(2), node(3)];
        add_path_metadata(&mut path, |_| Ok::<_, ()>(true), |p| Ok(p.x != 1)).unwrap();
        assert_eq!(
            path.iter().map(|n| n.flags).collect::<Vec<_>>(),
            vec![8, 9, 10, 8]
        );
    }
    #[test]
    fn turn_destination_uses_original_order_and_signed_turns() {
        let node = |turn| PathNode {
            data1_raw: 0,
            data2: turn,
            flags: 0,
            x: turn as i16,
            y: 0,
        };
        let path = vec![node(3), node(2), node(1), node(1), node(0)];
        assert_eq!(turn_destination(&path, 1).unwrap().x, 1);
        assert!(std::ptr::eq(turn_destination(&path, 1).unwrap(), &path[2]));
        assert!(turn_destination(&path, 8).is_none());
    }
}
