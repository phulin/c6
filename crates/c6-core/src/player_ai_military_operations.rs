// Included by player_ai_military.rs. Current and bounded legacy Linux saved wire.
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeamRequirement {
    pub wire_version: u32,
    pub unit_tag_hash: i32,
    pub unit_requirement_hash: i32,
    pub min_units: i32,
    pub max_units: i32,
    pub min_percentage_bits: u32,
    pub max_percentage_bits: u32,
    pub reconsider_until_ready: BoolByte,
}
impl Default for TeamRequirement {
    fn default() -> Self {
        Self {
            wire_version: 2,
            unit_tag_hash: Default::default(),
            unit_requirement_hash: Default::default(),
            min_units: Default::default(),
            max_units: Default::default(),
            min_percentage_bits: Default::default(),
            max_percentage_bits: Default::default(),
            reconsider_until_ready: Default::default(),
        }
    }
}
impl Wire for TeamRequirement {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <BoolByte as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 1 | 2) {
            return Err(Error::Version {
                expected: 2,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            unit_tag_hash: <i32 as Wire>::read(r)?,
            unit_requirement_hash: if wire_version == 2 {
                <i32 as Wire>::read(r)?
            } else {
                -1
            },
            min_units: <i32 as Wire>::read(r)?,
            max_units: <i32 as Wire>::read(r)?,
            min_percentage_bits: <u32 as Wire>::read(r)?,
            max_percentage_bits: <u32 as Wire>::read(r)?,
            reconsider_until_ready: <BoolByte as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 2 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        2u32.write(w)?;
        self.unit_tag_hash.write(w)?;
        self.unit_requirement_hash.write(w)?;
        self.min_units.write(w)?;
        self.max_units.write(w)?;
        self.min_percentage_bits.write(w)?;
        self.max_percentage_bits.write(w)?;
        self.reconsider_until_ready.write(w)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeamDefinition {
    pub wire_version: u32,
    pub min_units: i32,
    pub max_units: i32,
    pub strength_advantage_bits: u32,
    pub ongoing_strength_advantage_bits: u32,
    pub safe_rally_point: BoolByte,
    pub condition_hash: u32,
    pub requirements: Vec<TeamRequirement>,
}
impl Default for TeamDefinition {
    fn default() -> Self {
        Self {
            wire_version: 2,
            min_units: Default::default(),
            max_units: Default::default(),
            strength_advantage_bits: Default::default(),
            ongoing_strength_advantage_bits: Default::default(),
            safe_rally_point: Default::default(),
            condition_hash: Default::default(),
            requirements: Default::default(),
        }
    }
}
impl Wire for TeamDefinition {
    const MIN: usize = 4
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <Vec<TeamRequirement> as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 1 | 2) {
            return Err(Error::Version {
                expected: 2,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            min_units: <i32 as Wire>::read(r)?,
            max_units: <i32 as Wire>::read(r)?,
            strength_advantage_bits: <u32 as Wire>::read(r)?,
            ongoing_strength_advantage_bits: <u32 as Wire>::read(r)?,
            safe_rally_point: <BoolByte as Wire>::read(r)?,
            condition_hash: if wire_version == 2 {
                <u32 as Wire>::read(r)?
            } else {
                u32::MAX
            },
            requirements: <Vec<TeamRequirement> as Wire>::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 2 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        2u32.write(w)?;
        self.min_units.write(w)?;
        self.max_units.write(w)?;
        self.strength_advantage_bits.write(w)?;
        self.ongoing_strength_advantage_bits.write(w)?;
        self.safe_rally_point.write(w)?;
        self.condition_hash.write(w)?;
        self.requirements.write(w)?;
        Ok(())
    }
}
/// Independently versioned saved OperationDef; no planning or native initialization runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationDef {
    pub wire_version: u32,
    pub op_name: SavedString,
    pub name_hash: i32,
    pub target_type: i32,
    pub target_parameter: i32,
    pub update_team_points: BoolByte,
    pub op_type: i32,
    pub behavior_tree_name: SavedString,
    pub priority: i32,
    pub teams: Vec<TeamDefinition>,
    pub max_dist_in_region: i32,
    pub max_dist_in_area: i32,
    pub max_dist_in_world: i32,
    pub max_target_strength: i32,
    pub max_target_defense: i32,
    pub min_odds_of_success_bits: u32,
    pub self_start: BoolByte,
    pub must_be_at_war: BoolByte,
    pub must_have_nukes: BoolByte,
    pub must_have_min_units: i32,
    pub enemy_type: SavedString,
    pub target_lua_script: SavedString,
    pub active_emergency: SavedString,
}
impl Default for OperationDef {
    fn default() -> Self {
        Self {
            wire_version: 7,
            op_name: Default::default(),
            name_hash: Default::default(),
            target_type: Default::default(),
            target_parameter: Default::default(),
            update_team_points: Default::default(),
            op_type: Default::default(),
            behavior_tree_name: Default::default(),
            priority: Default::default(),
            teams: Default::default(),
            max_dist_in_region: Default::default(),
            max_dist_in_area: Default::default(),
            max_dist_in_world: Default::default(),
            max_target_strength: Default::default(),
            max_target_defense: Default::default(),
            min_odds_of_success_bits: Default::default(),
            self_start: Default::default(),
            must_be_at_war: Default::default(),
            must_have_nukes: Default::default(),
            must_have_min_units: Default::default(),
            enemy_type: Default::default(),
            target_lua_script: Default::default(),
            active_emergency: Default::default(),
        }
    }
}
impl Wire for OperationDef {
    const MIN: usize = 4
        + <SavedString as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <i32 as Wire>::MIN
        + <SavedString as Wire>::MIN
        + <i32 as Wire>::MIN
        + <Vec<TeamDefinition> as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <i32 as Wire>::MIN
        + <u32 as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <BoolByte as Wire>::MIN
        + <i32 as Wire>::MIN
        + <SavedString as Wire>::MIN;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let wire_version = u32::read(r)?;
        if !matches!(wire_version, 5 | 7) {
            return Err(Error::Version {
                expected: 7,
                actual: wire_version,
            });
        }
        Ok(Self {
            wire_version,
            op_name: <SavedString as Wire>::read(r)?,
            name_hash: <i32 as Wire>::read(r)?,
            target_type: <i32 as Wire>::read(r)?,
            target_parameter: <i32 as Wire>::read(r)?,
            update_team_points: <BoolByte as Wire>::read(r)?,
            op_type: <i32 as Wire>::read(r)?,
            behavior_tree_name: <SavedString as Wire>::read(r)?,
            priority: <i32 as Wire>::read(r)?,
            teams: <Vec<TeamDefinition> as Wire>::read(r)?,
            max_dist_in_region: <i32 as Wire>::read(r)?,
            max_dist_in_area: <i32 as Wire>::read(r)?,
            max_dist_in_world: <i32 as Wire>::read(r)?,
            max_target_strength: <i32 as Wire>::read(r)?,
            max_target_defense: <i32 as Wire>::read(r)?,
            min_odds_of_success_bits: <u32 as Wire>::read(r)?,
            self_start: <BoolByte as Wire>::read(r)?,
            must_be_at_war: <BoolByte as Wire>::read(r)?,
            must_have_nukes: <BoolByte as Wire>::read(r)?,
            must_have_min_units: <i32 as Wire>::read(r)?,
            enemy_type: <SavedString as Wire>::read(r)?,
            target_lua_script: if wire_version == 7 {
                <SavedString as Wire>::read(r)?
            } else {
                SavedString(Vec::new())
            },
            active_emergency: if wire_version == 7 {
                <SavedString as Wire>::read(r)?
            } else {
                SavedString(Vec::new())
            },
        })
    }
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        if self.wire_version != 7 {
            return Err(Error::UnsupportedLegacyWrite);
        }
        7u32.write(w)?;
        self.op_name.write(w)?;
        self.name_hash.write(w)?;
        self.target_type.write(w)?;
        self.target_parameter.write(w)?;
        self.update_team_points.write(w)?;
        self.op_type.write(w)?;
        self.behavior_tree_name.write(w)?;
        self.priority.write(w)?;
        self.teams.write(w)?;
        self.max_dist_in_region.write(w)?;
        self.max_dist_in_area.write(w)?;
        self.max_dist_in_world.write(w)?;
        self.max_target_strength.write(w)?;
        self.max_target_defense.write(w)?;
        self.min_odds_of_success_bits.write(w)?;
        self.self_start.write(w)?;
        self.must_be_at_war.write(w)?;
        self.must_have_nukes.write(w)?;
        self.must_have_min_units.write(w)?;
        self.enemy_type.write(w)?;
        self.target_lua_script.write(w)?;
        self.active_emergency.write(w)?;
        Ok(())
    }
}
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

    fn legacy_definition5() -> Vec<u8> {
        fn words(raw: &mut Vec<u8>, xs: &[u32]) {
            for v in xs {
                raw.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mut raw = Vec::new();
        words(&mut raw, &[5, 3]);
        raw.extend_from_slice(&[0, 255, 1]);
        words(&mut raw, &[u32::MAX, i32::MIN as u32, i32::MAX as u32]);
        raw.push(197);
        words(&mut raw, &[u32::MAX, 2]);
        raw.extend_from_slice(&[255, 0]);
        words(
            &mut raw,
            &[
                i32::MIN as u32,
                0,
                i32::MIN as u32,
                u32::MAX,
                i32::MAX as u32,
                23,
                (-257i32) as u32,
                0x7fc00001,
            ],
        );
        raw.extend_from_slice(&[197, 2, 3]);
        words(&mut raw, &[u32::MAX, 1]);
        raw.push(255);
        raw
    }
    #[test]
    fn legacy_team_rows_defaults_and_occurrence_limits() {
        let mut raw = Vec::new();
        for v in [
            1u32,
            i32::MIN as u32,
            i32::MAX as u32,
            0x7fc00001,
            0xffc01234,
        ] {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        raw.push(197);
        raw.extend_from_slice(&2u32.to_le_bytes());
        for tag in [u32::MAX, 0x80000000] {
            for v in [
                1u32,
                tag,
                i32::MIN as u32,
                i32::MAX as u32,
                0x7fc00001,
                0xffc01234,
            ] {
                raw.extend_from_slice(&v.to_le_bytes());
            }
            raw.push(197);
        }
        assert_eq!(raw.len(), 75);
        let cap = Limits {
            max_bytes: 75,
            max_items: 2,
        };
        let mut r = Reader {
            context: None,
            data: &raw,
            pos: 0,
            items: 0,
            limits: cap,
        };
        let v = TeamDefinition::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (75, 2));
        assert_eq!(v.wire_version, 1);
        assert_eq!(v.condition_hash, u32::MAX);
        assert_eq!(v.strength_advantage_bits, 0x7fc00001);
        assert_eq!(v.ongoing_strength_advantage_bits, 0xffc01234);
        for row in &v.requirements {
            assert_eq!(row.wire_version, 1);
            assert_eq!(row.unit_requirement_hash, -1);
            assert_eq!(row.reconsider_until_ready, BoolByte(197));
        }
        assert_eq!(v.requirements[0].unit_tag_hash, -1);
        assert_eq!(v.requirements[1].unit_tag_hash, i32::MIN);
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits: cap,
            measuring: true,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        for n in 0..raw.len() {
            let mut r = Reader {
                context: None,
                data: &raw[..n],
                pos: 0,
                items: 0,
                limits: cap,
            };
            assert!(TeamDefinition::read(&mut r).is_err());
        }
        for limits in [
            Limits {
                max_bytes: 74,
                ..cap
            },
            Limits {
                max_items: 1,
                ..cap
            },
        ] {
            let mut r = Reader {
                context: None,
                data: &raw,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(TeamDefinition::read(&mut r), Err(Error::ResourceLimit));
        }
        let mut bad = raw.clone();
        bad[25..29].copy_from_slice(&3u32.to_le_bytes());
        let mut r = Reader {
            context: None,
            data: &bad,
            pos: 0,
            items: 0,
            limits: cap,
        };
        assert_eq!(
            TeamDefinition::read(&mut r),
            Err(Error::Version {
                expected: 2,
                actual: 3
            })
        );
        assert_eq!(
            (
                TeamDefinition::default().wire_version,
                TeamRequirement::default().wire_version
            ),
            (2, 2)
        );
    }
    #[test]
    fn operation_definition5_defaults_and_exact_budgets() {
        let raw = legacy_definition5();
        assert_eq!(raw.len(), 78);
        let cap = Limits {
            max_bytes: 78,
            max_items: 6,
        };
        let mut r = Reader {
            context: None,
            data: &raw,
            pos: 0,
            items: 0,
            limits: cap,
        };
        let v = OperationDef::read(&mut r).unwrap();
        assert_eq!((r.pos, r.items), (78, 6));
        assert_eq!(v.wire_version, 5);
        assert_eq!(v.op_name.0, [0, 255, 1]);
        assert_eq!(v.behavior_tree_name.0, [255, 0]);
        assert_eq!(v.enemy_type.0, [255]);
        assert_eq!(v.min_odds_of_success_bits, 0x7fc00001);
        assert_eq!(v.update_team_points, BoolByte(197));
        assert_eq!(v.must_have_min_units, -1);
        assert_eq!(v.target_lua_script, SavedString(Vec::new()));
        assert_eq!(v.active_emergency, SavedString(Vec::new()));
        let mut w = Writer {
            data: Vec::new(),
            bytes: 0,
            items: 0,
            limits: cap,
            measuring: true,
        };
        assert_eq!(v.write(&mut w), Err(Error::UnsupportedLegacyWrite));
        for n in 0..raw.len() {
            let mut r = Reader {
                context: None,
                data: &raw[..n],
                pos: 0,
                items: 0,
                limits: cap,
            };
            assert!(OperationDef::read(&mut r).is_err());
        }
        for limits in [
            Limits {
                max_bytes: 77,
                ..cap
            },
            Limits {
                max_items: 5,
                ..cap
            },
        ] {
            let mut r = Reader {
                context: None,
                data: &raw,
                pos: 0,
                items: 0,
                limits,
            };
            assert_eq!(OperationDef::read(&mut r), Err(Error::ResourceLimit));
        }
        let mut unsupported = raw.clone();
        unsupported[..4].copy_from_slice(&6u32.to_le_bytes());
        let mut r = Reader {
            context: None,
            data: &unsupported,
            pos: 0,
            items: 0,
            limits: cap,
        };
        assert_eq!(
            OperationDef::read(&mut r),
            Err(Error::Version {
                expected: 7,
                actual: 6
            })
        );
        assert_eq!(OperationDef::default().wire_version, 7);
    }
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
