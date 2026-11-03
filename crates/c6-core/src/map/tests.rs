use super::*;
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn plot_wire() -> Vec<u8> {
    let mut b = words(&[0xffffffff, 0x80000000, 0x12345678, 0xdeadbeef, 0]);
    b.extend(0x9876u16.to_le_bytes());
    b.extend(0xffffffffu32.to_le_bytes());
    b.push(0xff);
    b.extend(0xabcdef01u32.to_le_bytes());
    b.extend(u16::MAX.to_le_bytes());
    b.extend(0x80000001u32.to_le_bytes());
    b.extend([0xfe, 0xfd, 0xfc]);
    b.extend(i16::MIN.to_le_bytes());
    b.extend([0x80, 0xff, 0x7f, 0x90, 0xef, 0xa5]);
    b.extend([0xff, 0xff, 1]);
    b.extend(0u32.to_le_bytes());
    b.extend(words(&[u32::MAX, 0x80000000, 0x12345678]));
    b.push(0x80);
    b.extend(0xfedcba98u32.to_le_bytes());
    b
}
#[test]
fn plot_literal_widths_order_flags_city_and_all_truncated_prefixes() {
    let bytes = plot_wire();
    assert_eq!(bytes.len(), 72);
    let p = Plot::decode(&bytes, 6, Limits::default()).unwrap();
    assert_eq!(
        (
            p.area_id,
            p.landmass_id,
            p.feature_layout,
            p.resource_count,
            p.unit_count,
            p.appeal
        ),
        (-1, i32::MIN, 0x9876, u16::MAX, -1, i16::MIN)
    );
    assert_eq!(
        (p.river_crossing_mask, p.cliff_crossing_mask, p.flags.bits()),
        (0xef, 0xa5, 0x1ffff)
    );
    assert_eq!(p.city_attributes.as_ref().unwrap().player, -128);
    assert_eq!(p.encode(6, Limits::default()).unwrap(), bytes);
    for n in 0..bytes.len() {
        assert!(
            Plot::decode(&bytes[..n], 6, Limits::default()).is_err(),
            "prefix{n}"
        );
    }
    let mut bad = bytes.clone();
    bad[50] |= 2;
    assert!(matches!(
        Plot::decode(&bad, 6, Limits::default()),
        Err(Error::UnsupportedPlotFlags(_))
    ));
    let mut p = p;
    p.flags.set(PlotFlag::HasCityAttributes, false);
    assert_eq!(
        p.encode(6, Limits::default()),
        Err(Error::CityAttributesFlagMismatch)
    );
    assert!(matches!(
        Plot::decode(&bytes, 4, Limits::default()),
        Err(Error::UnsupportedVersion { .. })
    ));
}
#[test]
fn localized_literal_all_argument_types_long_width_and_named_count() {
    let mut b = words(&[u32::MAX]);
    b.push(1);
    b.extend(words(&[2]));
    b.extend([0xff, 0]);
    b.push(6);
    b.push(0);
    b.push(1);
    b.extend(i32::MIN.to_le_bytes());
    b.push(2);
    b.extend(i64::MIN.to_le_bytes());
    b.push(3);
    b.extend(0x7fc01234u32.to_le_bytes());
    b.push(4);
    b.extend(0x8000000000000000u64.to_le_bytes());
    b.push(5);
    b.extend(words(&[u32::MAX]));
    b.push(1);
    b.extend(words(&[1]));
    b.push(b'x');
    b.extend([0, 0]);
    b.push(1);
    b.extend(words(&[5]));
    b.push(1);
    b.extend(words(&[2]));
    b.extend(*b"ab");
    let mut r = Reader::new(&b, Limits::default()).unwrap();
    let text = LocalizedText::read(&mut r, 0).unwrap();
    r.finish().unwrap();
    assert_eq!(text.arguments[2], LocalizedArgument::Long64(i64::MIN));
    assert_eq!(
        text.arguments[3],
        LocalizedArgument::Float32Bits(0x7fc01234)
    );
    let mut w = Writer::new(Limits::default());
    text.write(&mut w, 0).unwrap();
    assert_eq!(w.bytes, b);
    for end in 0..b.len() {
        let mut r = Reader::new(&b[..end], Limits::default()).unwrap();
        assert!(LocalizedText::read(&mut r, 0).is_err());
    }
    let mut r = Reader::new(
        &b,
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(LocalizedText::read(&mut r, 0), Err(Error::ResourceLimit));
    let mut bad = b.clone();
    let index = bad.len() - 7;
    bad[index] = 2;
    let mut r = Reader::new(&bad, Limits::default()).unwrap();
    assert_eq!(
        LocalizedText::read(&mut r, 0),
        Err(Error::UnsupportedLocalizationNameCount)
    );
}
#[test]
fn route_shared_portal_version_occurs_once_before_all_records() {
    let b = words(&[1, 1, 9, 8, 2, 2, 10, 2, 11, 12, 1, 13, 20, 1, 21, 2, 22, 23]);
    let mut r = Reader::new(&b, Limits::default()).unwrap();
    let v = Component::read(&mut r, b"Route").unwrap();
    r.finish().unwrap();
    let Component::Route(route) = &v else {
        panic!()
    };
    assert_eq!(route.portals[1].entrance_plot_index, 20);
    assert_eq!(route.portals[1].source_plot_indices, [22, 23]);
    let mut w = Writer::new(Limits::default());
    v.write(&mut w).unwrap();
    assert_eq!(w.bytes, b);
    for end in [0, 4, 19, 23, b.len() - 1] {
        let mut r = Reader::new(&b[..end], Limits::default()).unwrap();
        assert!(Component::read(&mut r, b"Route").is_err());
    }
}
#[test]
fn terrain_nonempty_all_six_lists_preserve_types_and_signed_territory_ids() {
    let mut b = words(&[6, 1, 9, 0xaabbccdd, 5, 0x12345678]);
    b.extend([0x80, 0xff, 0, 1, 2, 3, 4, 0xef]);
    b.extend([0, 1, 0, 1, 0, 1, 1, 0, 1]);
    for _ in 0..2 {
        b.extend(words(&[1, 6, 0xdeadbeef, 0x87654321, u32::MAX, 2]));
        b.extend([0, 0xff]);
    }
    for _ in 0..3 {
        b.extend(words(&[1, 1, 0x80000000, 0x87654321, 0xfffffffe, 1]));
        b.push(b'x');
    }
    let mut r = Reader::new(&b, Limits::default()).unwrap();
    let v = Component::read(&mut r, b"Terrain").unwrap();
    r.finish().unwrap();
    let Component::Terrain(t) = &v else { panic!() };
    assert_eq!(t.coastal_lowlands[0].river_crossing_mask, 0xef);
    assert_eq!(t.deserts[0].territory_id, -1);
    assert_eq!(t.named_seas[0].kind, i32::MIN);
    let mut w = Writer::new(Limits::default());
    v.write(&mut w).unwrap();
    assert_eq!(w.bytes, b);
}
#[test]
fn territory_general_record_name_and_recycled_generation_are_distinct() {
    let mut b = words(&[
        1, 65537, 16, 1, 1, 0, 2, 65536, 0xabcdef01, 1, 2, 3, 4, 5, 6,
    ]);
    b.extend([1, 0]);
    b.extend(words(&[10, 20, 30, 40, 50, 60, 1, 0xdeadbeef, 7, u32::MAX]));
    b.push(1);
    b.extend(words(&[1]));
    b.push(b'N');
    b.extend([0, 0]);
    b.extend(words(&[
        0x56780000,
        0,
        u32::MAX,
        u32::MAX,
        2,
        65536,
        u32::MAX,
    ]));
    let mut r = Reader::new(&b, Limits::default()).unwrap();
    let v = Component::read(&mut r, b"Territories").unwrap();
    r.finish().unwrap();
    let Component::Territories {
        container,
        plot_territory_ids,
    } = &v
    else {
        panic!()
    };
    assert_eq!(container.entries[0].index, 0);
    assert_eq!(container.entries[0].value.id, 65536);
    assert_eq!(container.current_id_bits, 0x56780000);
    assert_eq!(plot_territory_ids, &[65536, u32::MAX]);
    let mut w = Writer::new(Limits::default());
    v.write(&mut w).unwrap();
    assert_eq!(w.bytes, b);
}
#[test]
fn sparse_get_defaults_and_invalid_value_reference_are_not_silent_fallbacks() {
    let s = SparsePlotData {
        default: 99,
        indices: vec![-1, 0],
        values: vec![37],
    };
    assert_eq!(*s.get(-1).unwrap(), 99);
    assert_eq!(*s.get(0).unwrap(), 99);
    assert_eq!(*s.get(1).unwrap(), 37);
    assert_eq!(*s.get(2).unwrap(), 99);
    let s = SparsePlotData {
        default: 99,
        indices: vec![1],
        values: vec![37],
    };
    assert_eq!(s.get(0), Err(Error::InvalidSparseIndex));
}
#[test]
fn plot_resource_limits_are_checked_and_unsupported_data_never_changes_input() {
    let b = plot_wire();
    let original = b.clone();
    assert_eq!(
        Plot::decode(
            &b,
            6,
            Limits {
                max_bytes: b.len() - 1,
                ..Limits::default()
            }
        ),
        Err(Error::ResourceLimit)
    );
    let p = Plot::decode(
        &b,
        6,
        Limits {
            max_bytes: b.len(),
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        p.encode(
            6,
            Limits {
                max_bytes: b.len(),
                ..Limits::default()
            }
        )
        .unwrap(),
        b
    );
    assert_eq!(b, original);
}

#[test]
fn plot_v5_shares_v6_wire_and_applies_native_volatile_clear() {
    let mut bytes = plot_wire();
    let cleared = Plot::decode(&bytes, 5, Limits::default()).unwrap();
    assert_eq!(cleared.flags.bits(), 0xffff);
    assert_eq!(bytes[50], 1); // Keep the ignored raw flag witness unchanged.
    assert!(Plot::decode(&bytes, 6, Limits::default())
        .unwrap()
        .flags
        .get(PlotFlag::FeatureVolatile));
    bytes[50] = 0;
    let old = Plot::decode(&bytes, 5, Limits::default()).unwrap();
    assert_eq!(old, cleared);
    assert_eq!(old, Plot::decode(&bytes, 6, Limits::default()).unwrap());
    assert!(!old.flags.get(PlotFlag::FeatureVolatile));
    assert!(old.city_attributes.is_some());
    assert_eq!(old.encode(6, Limits::default()).unwrap(), bytes);
    assert!(matches!(
        old.encode(5, Limits::default()),
        Err(Error::UnsupportedVersion { .. })
    ));
    for n in 0..bytes.len() {
        assert!(
            Plot::decode(&bytes[..n], 5, Limits::default()).is_err(),
            "prefix {n}"
        );
    }
    assert_eq!(
        Plot::decode(
            &bytes,
            5,
            Limits {
                max_bytes: bytes.len() - 1,
                ..Limits::default()
            }
        ),
        Err(Error::ResourceLimit)
    );
    bytes[49] &= !0x40; // No CityAttributes: record ends after the property count.
    bytes.truncate(55);
    assert!(Plot::decode(&bytes, 5, Limits::default())
        .unwrap()
        .city_attributes
        .is_none());
    bytes[50] = 2;
    assert!(matches!(
        Plot::decode(&bytes, 5, Limits::default()),
        Err(Error::UnsupportedPlotFlags(_))
    ));
    for version in [-1, 0, 1, 2, 3, 4, 7, i32::MAX] {
        assert!(
            matches!(Plot::decode(&bytes, version, Limits::default()), Err(Error::UnsupportedVersion { actual, .. }) if actual == version)
        );
    }
}
