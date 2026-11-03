//! Bounded Linux host dirty-component fanout, not a complete reporting sender.
//!
//! Source contracts: host SetDirty 0x32b5f90, Add 0x32b5950 and equality
//! 0x2fb7350. Native registrations and differential acceptance remain missing.
//! This state must not be substituted for Core unit dirty-index history.

/// Native ComponentID's three fields; all eight bytes participate in equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentId {
    pub owner: u16,
    pub component_type: u16,
    pub id: u32,
}

/// One real host DirtyComponents object's ordered ID list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyComponents {
    pub components: Vec<ComponentId>,
}

/// Registration knowledge is supplied explicitly; unknown never means empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventRegistration {
    Unknown,
    Absent,
    /// Indices into the supplied receiver collection, in native pointer order.
    Targets(Vec<usize>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FanoutError {
    UnknownRegistration,
    MissingReceiver(usize),
}

/// Apply one original ChangesObject-qualified identity parameter's fanout.
///
/// Callers must preserve original typed-map parameter order and decode actual
/// ParameterUsageDefinition bit zero. This does not decode SendEvent, run a
/// translator, drain host events, reproduce allocator state, or admit SkipTurn.
/// Unsupported registration/receiver inputs reject before any mutation.
pub fn mark_component_dirty(
    receivers: &mut [DirtyComponents],
    registration: &EventRegistration,
    changes_object: bool,
    component: ComponentId,
) -> Result<(), FanoutError> {
    if !changes_object {
        return Ok(());
    }
    let targets = match registration {
        EventRegistration::Unknown => return Err(FanoutError::UnknownRegistration),
        EventRegistration::Absent => return Ok(()),
        EventRegistration::Targets(targets) => targets,
    };
    for &target in targets {
        if target >= receivers.len() {
            return Err(FanoutError::MissingReceiver(target));
        }
    }
    for &target in targets {
        let components = &mut receivers[target].components;
        if !components.contains(&component) {
            components.push(component);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_and_receiver_order_preserve_existing_entries() {
        let original = ComponentId {
            owner: 0,
            component_type: 1,
            id: 7,
        };
        let variants = [
            ComponentId {
                owner: 1,
                ..original
            },
            ComponentId {
                component_type: 2,
                ..original
            },
            ComponentId { id: 8, ..original },
        ];
        let mut receivers = vec![
            DirtyComponents {
                components: vec![original],
            },
            DirtyComponents::default(),
        ];
        let registration = EventRegistration::Targets(vec![1, 0, 1]);
        for component in variants.into_iter().chain([original, original]) {
            mark_component_dirty(&mut receivers, &registration, true, component).unwrap();
        }
        assert_eq!(
            receivers[0].components,
            [original, variants[0], variants[1], variants[2]]
        );
        assert_eq!(
            receivers[1].components,
            [variants[0], variants[1], variants[2], original]
        );
    }

    #[test]
    fn unsupported_registration_is_atomic_and_no_change_flag_skips_it() {
        let id = ComponentId {
            owner: u16::MAX,
            component_type: 0,
            id: u32::MAX,
        };
        let mut receivers = vec![DirtyComponents::default()];
        assert_eq!(
            mark_component_dirty(
                &mut receivers,
                &EventRegistration::Targets(vec![0, 1]),
                true,
                id
            ),
            Err(FanoutError::MissingReceiver(1))
        );
        assert!(receivers[0].components.is_empty());
        assert_eq!(
            mark_component_dirty(&mut receivers, &EventRegistration::Unknown, true, id),
            Err(FanoutError::UnknownRegistration)
        );
        mark_component_dirty(&mut receivers, &EventRegistration::Unknown, false, id).unwrap();
        mark_component_dirty(&mut receivers, &EventRegistration::Absent, true, id).unwrap();
        assert!(receivers[0].components.is_empty());
    }
}
