// Included by player_ai_military.rs. Current pinned Linux saved wire only.
// Evidence and reproducible source checks: docs/linux-military-operations-contract.md.

/// Native narrow string storage: a u32 byte count followed by those exact bytes.
/// This is neither a UTF-8 assertion nor a nul-terminated string.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedString(pub Vec<u8>);
impl Wire for SavedString {
    const MIN: usize = 4;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)? as usize;
        let mut bytes = r.reserve::<u8>(n, 1)?;
        for _ in 0..n {
            bytes.push(r.take::<1>()?[0]);
        }
        Ok(Self(bytes))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        u32::try_from(self.0.len())
            .map_err(|_| Error::ResourceLimit)?
            .write(w)?;
        w.items = w
            .items
            .checked_add(self.0.len())
            .ok_or(Error::ResourceLimit)?;
        if w.items > w.limits.max_items {
            return Err(Error::ResourceLimit);
        }
        w.put(&self.0)
    }
}

record!(TeamRequirement, 2, {
    unit_tag_hash: i32, unit_requirement_hash: i32, min_units: i32,
    max_units: i32, min_percentage_bits: u32, max_percentage_bits: u32,
    reconsider_until_ready: BoolByte,
});
record!(TeamDefinition, 2, {
    min_units: i32, max_units: i32, strength_advantage_bits: u32,
    ongoing_strength_advantage_bits: u32, safe_rally_point: BoolByte,
    condition_hash: u32, requirements: Vec<TeamRequirement>,
});
record!(OperationDef, 7, {
    op_name: SavedString, name_hash: i32, target_type: i32,
    target_parameter: i32, update_team_points: BoolByte, op_type: i32,
    behavior_tree_name: SavedString, priority: i32, teams: Vec<TeamDefinition>,
    max_dist_in_region: i32, max_dist_in_area: i32, max_dist_in_world: i32,
    max_target_strength: i32, max_target_defense: i32,
    min_odds_of_success_bits: u32, self_start: BoolByte,
    must_be_at_war: BoolByte, must_have_nukes: BoolByte, must_have_min_units: i32,
    enemy_type: SavedString, target_lua_script: SavedString,
    active_emergency: SavedString,
});
record!(OperationTarget, 1, {
    enemy: i32, target_type: i32, location: PlotCoord, rally: PlotCoord,
    target_id: ComponentId,
});

/// WriteArray stores its count even though this current schema fixes it at six.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationPositions(pub [PlotCoord; 6]);
impl Wire for OperationPositions {
    const MIN: usize = 4 + 6 * <PlotCoord as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = u32::read(r)?;
        if n != 6 {
            return Err(Error::FixedCount(n));
        }
        let values = r.sequence::<PlotCoord>(6)?;
        Ok(Self(values.try_into().map_err(|_| Error::FixedCount(n))?))
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        6u32.write(w)?;
        w.sequence(&self.0)
    }
}
record!(OperationTeam, 3, {
    is_at_minimums: BoolByte, is_ready: BoolByte, is_full_strength: BoolByte,
    needed_unit_hash: u32, team_members: Vec<u32>, new_team_members: Vec<u32>,
    member_storage: Vec<Vec<u32>>, member_storage_location: Vec<PlotCoord>,
    endangered_members: Vec<u32>, has_enemies_near: BoolByte,
    op_positions: OperationPositions, local_enemy_strength: i32,
    local_team_strength: i32,
});
record!(Operation, 6, {
    player_id: i32, op_definition: OperationDef, teams: Vec<OperationTeam>,
    target: OperationTarget, rally_point: PlotCoord, blocked_turns: i32,
    tree_control_id: u32, tree_manager_id: u32, backup_target: OperationTarget,
    force_target_switch: BoolByte, op_strength: i32, target_strength: i32,
    current_combat_strength: i32, force_ready: BoolByte, war_ready: BoolByte,
    is_emergency: BoolByte, control_id: u32,
});

#[cfg(test)]
mod operation_wire_tests {
    use super::*;

    #[test]
    fn player_ai_military_operations_nested_storage_bits_and_limits() {
        // Synthetic codec boundary fixture; native corpus parity is audited separately.
        let operation = Operation {
            op_definition: OperationDef {
                op_name: SavedString(vec![0xff, 0, 0x80]),
                min_odds_of_success_bits: 0x7fc0_1234,
                teams: vec![TeamDefinition {
                    strength_advantage_bits: 0x8000_0000,
                    requirements: vec![TeamRequirement {
                        max_percentage_bits: 0xff80_0000,
                        reconsider_until_ready: BoolByte(0xfe),
                        ..TeamRequirement::default()
                    }],
                    ..TeamDefinition::default()
                }],
                ..OperationDef::default()
            },
            teams: vec![OperationTeam {
                needed_unit_hash: u32::MAX,
                member_storage: vec![vec![0, u32::MAX], vec![]],
                member_storage_location: vec![[i32::MIN, i32::MAX]],
                op_positions: OperationPositions([[i32::MIN, i32::MAX]; 6]),
                ..OperationTeam::default()
            }],
            ..Operation::default()
        };
        let value = Military {
            active_operations: RecycledOperations {
                entries: vec![OperationEntry {
                    index: 7,
                    operation,
                }],
                ..RecycledOperations::default()
            },
            ..Military::default()
        };
        let context = Context {
            operation_type_arrays_present: false,
        };
        let generous = Limits {
            max_bytes: 4096,
            max_items: 4096,
        };
        let (bytes, items) = value.encode_with_items(generous).unwrap();
        let exact = Limits {
            max_bytes: bytes.len(),
            max_items: items,
        };
        assert_eq!(
            Military::decode_in_context(&bytes, exact, context).unwrap(),
            value
        );
        for end in 0..bytes.len() {
            assert!(Military::decode_in_context(&bytes[..end], exact, context).is_err());
        }
        for short in [
            Limits {
                max_bytes: bytes.len() - 1,
                ..exact
            },
            Limits {
                max_items: items - 1,
                ..exact
            },
        ] {
            assert_eq!(
                Military::decode_in_context(&bytes, short, context),
                Err(Error::ResourceLimit)
            );
            assert_eq!(value.encode(short), Err(Error::ResourceLimit));
        }
    }

    #[test]
    fn player_ai_military_operations_reject_wrong_fixed_position_count() {
        for count in [0u32, 5, 7, u32::MAX] {
            let bytes = count.to_le_bytes();
            let mut reader = Reader {
                context: None,
                data: &bytes,
                pos: 0,
                items: 0,
                limits: Limits {
                    max_bytes: 4,
                    max_items: 6,
                },
            };
            assert_eq!(
                OperationPositions::read(&mut reader),
                Err(Error::FixedCount(count))
            );
        }
    }
}
