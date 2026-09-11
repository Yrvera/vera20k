// Included inside resolved_terrain::tests to share its real TMP/loader fixture.
// Native outputs: tools/spatial_oracle/terrain_recalc, original47D2B0 and callees.

#[test]
fn recalc_pristine_metadata_and_level_override_match_original_instructions() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../tools/spatial_oracle/terrain_recalc.json"
    ))
    .unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 10);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let flag = |key: &str| case[key].as_bool().unwrap();
        let number = |key: &str| case[key].as_i64().unwrap();
        let mut theater = synthetic_theater_from_ini(
            b"[TileSet0000]\nTilesInSet=1\nFileName=other\nSetName=Other\n\
              Morphable=no\nAllowTiberium=no\n\
              [TileSet0001]\nTilesInSet=1\nFileName=source\nSetName=Source\n\
              Morphable=yes\nAllowTiberium=yes\n",
        );
        let mut source = gsi_04_02_last_tiles_tmp_bytes(11, [0; 3], [0; 3]);
        let mut other = source.clone();
        if flag("lat") {
            other[61] = 15;
            other[62] = 4;
            other[12..16].copy_from_slice(&34u32.to_le_bytes());
            other.resize(72 + 8 * 34 / 2, 1);
            theater.rmg_tiles.ramp_base = Some(1);
        }
        if flag("sparse") {
            source[16..20].copy_from_slice(&0u32.to_le_bytes());
            other = source.clone();
        }
        let (_directory, assets) = gsi_04_02_asset_manager_with_loose_tmps(&[
            ("source01.tem", &source),
            ("other01.tem", &other),
        ]);
        let ini = IniFile::from_str(
            "[Clear]\nWheel=100%\n[Road]\nWheel=100%\n[Rock]\nWheel=0%\n\
             [Tiberium]\nWheel=0%\n\
             [OverlayTypes]\n0=EARLY\n1=ROAD\n2=RESOURCE\n\
             [EARLY]\nLand=Road\nNoUseTileLandType=yes\n\
             [ROAD]\nLand=Road\nNoUseTileLandType=no\n\
             [RESOURCE]\nLand=Tiberium\nTiberium=yes\nNoUseTileLandType=no\n",
        );
        let rules = TerrainRules::from_ini(&ini);
        let registry = OverlayTypeRegistry::from_ini(&ini, None);
        let mut map = make_map(Vec::new(), Vec::new(), Vec::new());
        map.header.width = 16;
        map.header.height = 16;
        map.header.local_left = 0;
        map.header.local_top = 0;
        map.header.local_width = 16;
        map.header.local_height = 16;
        let cells = (0..33)
            .flat_map(|y| (0..33).map(move |x| make_test_cell(x, y)))
            .collect();
        let mut grid = ResolvedTerrainGrid::from_cells(33, 33, cells);
        let index = 16 * 33 + 16;
        grid.cells[index].final_tile_index = if flag("invalid") {
            2
        } else {
            i32::from(flag("lat"))
        };
        grid.cells[index].final_sub_tile = number("input_subtile") as u8;
        grid.cells[index].level = 4;
        grid.cells[index].height_in_pixels = 88;
        let mut state = LoadCellRecalcState::for_authored_load(
            &map,
            &theater,
            &assets,
            &rules,
            &registry,
            flag("lat"),
            0,
            grid.cells.len(),
        );
        let overlay = if flag("early") {
            FinalizedOverlayCell::from_parts(0, 0)
        } else {
            match case["overlay_land"].as_i64() {
                Some(1) => FinalizedOverlayCell::from_parts(1, 0),
                Some(5) => FinalizedOverlayCell::from_parts(2, 0),
                None => FinalizedOverlayCell::default(),
                other => panic!("unexpected fixture overlay land {other:?}"),
            }
        };
        grid.recalc_cell_attributes(
            &mut state,
            index,
            overlay,
            number("level_override") as i32,
            &mut LoadRecalcTestEffects::default(),
        )
        .unwrap();
        let cell = &grid.cells[index];
        if flag("lat") {
            assert!(!cell.accepts_smudge, "{name}: final-tile Morphable query");
            assert!(
                !cell.allows_tiberium,
                "{name}: final-tile AllowTiberium query"
            );
        }
        for (field, actual) in [
            ("tile", i64::from(cell.final_tile_index)),
            ("subtile", i64::from(cell.final_sub_tile)),
            ("level", i64::from(cell.level)),
            ("slope", i64::from(cell.slope_type)),
            ("height", i64::from(cell.height_in_pixels)),
            ("land", i64::from(cell.yr_cell_land_type)),
            ("zone", i64::from(cell.zone_type)),
        ] {
            assert_eq!(actual, number(field), "{name}: {field}");
        }
    }
}
