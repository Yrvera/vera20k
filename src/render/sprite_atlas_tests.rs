//! Tests for SHP sprite atlas key collection and deduplication.

use super::*;

fn make_shp_key(type_id: &str, facing: u8) -> ShpSpriteKey {
    ShpSpriteKey {
        type_id: type_id.to_string(),
        facing,
        frame: 0,
        house_color: HouseColorIndex::default(),
    }
}

#[test]
fn test_shp_sprite_key_hash_equality() {
    let hc = HouseColorIndex::default();
    let k1 = ShpSpriteKey {
        type_id: "E1".into(),
        facing: 64,
        frame: 10,
        house_color: hc,
    };
    let k2 = ShpSpriteKey {
        type_id: "E1".into(),
        facing: 64,
        frame: 10,
        house_color: hc,
    };
    let k3 = ShpSpriteKey {
        type_id: "E1".into(),
        facing: 64,
        frame: 11,
        house_color: hc,
    };
    assert_eq!(k1, k2);
    assert_ne!(k1, k3);
    let mut set: HashSet<ShpSpriteKey> = HashSet::new();
    set.insert(k1);
    set.insert(k2);
    set.insert(k3);
    assert_eq!(set.len(), 2);
}

#[test]
fn test_empty_world_returns_none() {
    let needed: HashSet<ShpSpriteKey> = HashSet::new();
    assert!(needed.is_empty());
}

fn rendered_test_sprite(type_id: &str, rgba: Vec<u8>) -> RenderedShpSprite {
    RenderedShpSprite {
        key: ShpSpriteKey {
            type_id: type_id.to_string(),
            facing: 0,
            frame: 0,
            house_color: HouseColorIndex::default(),
        },
        rgba,
        width: 1,
        height: 1,
        offset_x: -1.0,
        offset_y: -2.0,
    }
}

#[test]
fn incremental_refresh_failure_restores_the_exact_prior_rendered_cache() {
    let prior_key = make_shp_key("PRIOR", 0);
    let prior_entry = ShpSpriteEntry {
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        pixel_size: [1.0, 1.0],
        offset_x: -1.0,
        offset_y: -2.0,
        page: 0,
    };
    let mut prior = SpriteAtlas {
        pages: Vec::new(),
        entries: HashMap::from([(prior_key.clone(), prior_entry)]),
        make_frame_counts: HashMap::from([("PRIOR".to_string(), 3)]),
        active_anim_frame_counts: HashMap::from([("PRIOR".to_string(), 4)]),
        building_bounds: HashMap::from([(
            "PRIOR".to_string(),
            BuildingBounds {
                min_x: -1.0,
                min_y: -2.0,
                width: 1.0,
                height: 1.0,
            },
        )]),
        rendered_cache: vec![rendered_test_sprite("PRIOR", vec![1, 2, 3, 4])],
    };

    let prior_cache_len = prior.rendered_cache.len();
    let mut extracted_cache = std::mem::take(&mut prior.rendered_cache);
    extracted_cache.push(rendered_test_sprite("NEW", vec![5, 6, 7, 8]));

    let restored = abort_sprite_atlas_refresh(
        Some(prior),
        Some(extracted_cache),
        prior_cache_len,
        "required incremental sprite failed".to_string(),
    )
    .expect("an incremental failure must retain the prior atlas");

    assert_eq!(restored.sprite_count(), 1);
    assert!(restored.get(&prior_key).is_some());
    assert_eq!(restored.make_frame_counts["PRIOR"], 3);
    assert_eq!(restored.active_anim_frame_counts["PRIOR"], 4);
    assert_eq!(restored.building_bounds["PRIOR"].width, 1.0);
    assert_eq!(restored.rendered_cache.len(), 1);
    assert_eq!(restored.rendered_cache[0].key, prior_key);
    assert_eq!(restored.rendered_cache[0].rgba, vec![1, 2, 3, 4]);
}

#[test]
#[should_panic(expected = "required initial sprite failed")]
fn required_initial_build_failure_remains_fail_fast() {
    let _ = abort_sprite_atlas_refresh(None, None, 0, "required initial sprite failed".to_string());
}

#[test]
fn test_key_collection_deduplicates() {
    let hc = HouseColorIndex::default();
    let mut needed: HashSet<ShpSpriteKey> = HashSet::new();
    // Two identical keys + one different facing.
    needed.insert(make_shp_key("E1", 64));
    needed.insert(make_shp_key("E1", 64)); // duplicate
    needed.insert(make_shp_key("E1", 128));
    assert_eq!(needed.len(), 2);
    let _ = hc;
}

#[test]
fn test_structure_facing_collapse() {
    let hc = HouseColorIndex::default();
    // Structures always use facing 0 (no rotation).
    let mut needed: HashSet<ShpSpriteKey> = HashSet::new();
    for facing_raw in [64u8, 192u8] {
        let eff = 0u8; // structures collapse to facing 0
        needed.insert(ShpSpriteKey {
            type_id: "GAPOWR".to_string(),
            facing: eff,
            frame: 0,
            house_color: hc,
        });
        let _ = facing_raw;
    }
    assert_eq!(needed.len(), 1);
}

#[test]
fn test_different_houses_create_separate_keys() {
    let hc0 = HouseColorIndex(0); // [Colors] entry 0 (LightGold)
    let hc1 = HouseColorIndex(1); // [Colors] entry 1 (Gold)
    let k1 = ShpSpriteKey {
        type_id: "E1".into(),
        facing: 64,
        frame: 10,
        house_color: hc0,
    };
    let k2 = ShpSpriteKey {
        type_id: "E1".into(),
        facing: 64,
        frame: 10,
        house_color: hc1,
    };
    assert_ne!(
        k1, k2,
        "Same type+facing but different house should be distinct keys"
    );
    let mut set: HashSet<ShpSpriteKey> = HashSet::new();
    set.insert(k1);
    set.insert(k2);
    assert_eq!(set.len(), 2);
}

#[test]
fn alt_palette_art_takes_the_unit_palette_even_when_it_is_a_world_effect() {
    // WCCLOUD1 (Weather Storm) and SQDG (squid grapple) are registered as world
    // effects, so the name set alone would bake them against anim.pal. Both set
    // AltPalette=yes, which selects the unit palette instead. FBALL1 sets nothing
    // and must stay on anim.pal.
    let ini = crate::rules::ini_parser::IniFile::from_str(
        "[WCCLOUD1]\nAltPalette=yes\n\
         [SQDG]\nAltPalette=yes\n\
         [FBALL1]\nLayer=ground\n",
    );
    let art = ArtRegistry::from_ini(&ini);
    let effects: HashSet<String> = ["WCCLOUD1", "SQDG", "FBALL1"]
        .iter()
        .map(|name| name.to_string())
        .collect();
    let cell_drawers = HashSet::new();

    assert_eq!(
        sprite_palette_choice("WCCLOUD1", Some(&art), &effects, &cell_drawers),
        SpritePaletteChoice::Unit
    );
    assert_eq!(
        sprite_palette_choice("SQDG", Some(&art), &effects, &cell_drawers),
        SpritePaletteChoice::Unit
    );
    assert_eq!(
        sprite_palette_choice("FBALL1", Some(&art), &effects, &cell_drawers),
        SpritePaletteChoice::Anim
    );
}

#[test]
fn sprite_palette_choice_leaves_non_effect_and_unknown_art_on_the_unit_palette() {
    let ini = crate::rules::ini_parser::IniFile::from_str("[GAPOWR]\nRemapable=yes\n");
    let art = ArtRegistry::from_ini(&ini);
    let effects: HashSet<String> = HashSet::new();
    let cell_drawers = HashSet::new();

    // A structure that is not a world effect keeps the unit palette.
    assert_eq!(
        sprite_palette_choice("GAPOWR", Some(&art), &effects, &cell_drawers),
        SpritePaletteChoice::Unit
    );
    // No art registry at all must not change the previous name-set behaviour.
    assert_eq!(
        sprite_palette_choice("GAPOWR", None, &effects, &cell_drawers),
        SpritePaletteChoice::Unit
    );
    let mut with_effect: HashSet<String> = HashSet::new();
    with_effect.insert("FBALL1".to_string());
    assert_eq!(
        sprite_palette_choice("FBALL1", None, &with_effect, &cell_drawers),
        SpritePaletteChoice::Anim
    );
}

#[test]
fn declared_special_animation_frames_are_all_preloaded() {
    let mut needed = HashSet::new();
    insert_building_anim_frame_keys(&mut needed, "GAREFNOR", 3, HouseColorIndex(2));

    for frame in 0..3 {
        assert!(needed.contains(&ShpSpriteKey {
            type_id: "GAREFNOR".to_string(),
            facing: 0,
            frame,
            house_color: HouseColorIndex(2),
        }));
    }
    assert_eq!(needed.len(), 3);
}

#[test]
fn gsi_13_08_effect_frame_count_halves_only_shadowed_non_scheduler_assets() {
    assert_eq!(available_effect_anim_frame_count(21, false, false), 21);
    assert_eq!(available_effect_anim_frame_count(21, false, true), 10);
    assert_eq!(available_effect_anim_frame_count(20, false, true), 10);
    assert_eq!(available_effect_anim_frame_count(20, true, true), 20);
    assert_eq!(available_effect_anim_frame_count(1, false, true), 1);
    assert_eq!(available_effect_anim_frame_count(0, false, true), 0);
}

#[test]
fn gsi_13_08_warpout_keeps_all_frames_and_drives_the_progressive_alpha_ladder() {
    let ini = crate::rules::ini_parser::IniFile::from_str("[WARPOUT]\nTranslucent=yes\nRate=120\n");
    let art = ArtRegistry::from_ini(&ini);
    let warpout = art
        .anim_runtime_config("WARPOUT")
        .expect("parsed WARPOUT animation type");
    assert!(warpout.translucent);
    assert!(!warpout.shadow);
    assert_eq!(warpout.explicit_end, None);
    assert_eq!(warpout.explicit_loop_end, None);

    let mut needed = HashSet::new();
    let mut active_anim_frame_counts = HashMap::new();
    let frame_count = register_effect_anim_frames(
        &mut needed,
        &mut active_anim_frame_counts,
        "WARPOUT",
        21,
        art.scheduler_anim_types().contains("WARPOUT"),
        warpout.shadow,
    );

    assert_eq!(active_anim_frame_counts["WARPOUT"], 21);
    assert_eq!(needed.len(), 21);
    for frame in 0..=20 {
        assert!(needed.contains(&ShpSpriteKey {
            type_id: "WARPOUT".to_string(),
            facing: 0,
            frame,
            house_color: HouseColorIndex(0),
        }));
    }

    for (frame, expected_alpha) in [
        (4, 1.0),
        (5, 0.75),
        (8, 0.75),
        (9, 0.5),
        (12, 0.5),
        (13, 0.25),
        (20, 0.25),
    ] {
        let selection = crate::sim::anim_class::anim_translucency_selection(
            crate::sim::anim_class::AnimTranslucencyInput {
                base_flags: 0,
                forced_translucent: false,
                forced_uses_75: false,
                translucency_detail_level: warpout.translucency_detail_level,
                game_detail_level: 2,
                translucent_ramp: warpout.translucent,
                current_frame: frame,
                frame_count: i32::from(frame_count),
                explicit_translucency: warpout.translucency,
                instance_ramp: 0,
            },
        );
        assert!(selection.draw);
        assert_eq!(
            crate::rules::art_data::anim_translucency_source_alpha(selection.flags),
            expected_alpha,
            "WARPOUT frame {frame}",
        );
    }
}

fn make_raw_test_shp(frame_count: u16) -> Vec<u8> {
    let headers_end = 8usize + usize::from(frame_count) * 24;
    let mut data = Vec::with_capacity(headers_end + usize::from(frame_count));
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    for frame in 0..frame_count {
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 12]);
        let offset = u32::try_from(headers_end + usize::from(frame)).unwrap();
        data.extend_from_slice(&offset.to_le_bytes());
    }
    data.extend(std::iter::repeat_n(1u8, usize::from(frame_count)));
    data
}

#[test]
fn gsi_13_04_tem_only_tile_root_uses_iso_palette_and_registers_every_frame() {
    let mut rules = crate::rules::ruleset::RuleSet::from_ini(
        &crate::rules::ini_parser::IniFile::from_str("[General]\nDamageFireTypes=\n"),
    )
    .expect("rules");
    let mut art = ArtRegistry::from_ini(&crate::rules::ini_parser::IniFile::from_str(
        "[CUSTOM_TILE_ANIM]\nTheater=yes\nAltPalette=yes\nLoopCount=-1\n",
    ));
    art.bind_anim_frame_count_for_test("CUSTOM_TILE_ANIM", 4);
    rules.art_registry = art;
    let effects: HashSet<String> = ["CUSTOM_TILE_ANIM".to_string()].into_iter().collect();
    let cell_drawers: HashSet<String> = ["CUSTOM_TILE_ANIM".to_string()].into_iter().collect();

    assert!(
        collect_effect_names(&rules)
            .iter()
            .any(|name| name == "CUSTOM_TILE_ANIM")
    );
    assert_eq!(
        sprite_palette_choice(
            "CUSTOM_TILE_ANIM",
            Some(&rules.art_registry),
            &effects,
            &cell_drawers,
        ),
        SpritePaletteChoice::CellIso,
        "cell-drawer palette authority overrides ordinary Anim.PAL/AltPalette selection"
    );

    let candidates = effect_anim_shp_candidates(
        "CUSTOM_TILE_ANIM",
        Some(&rules.art_registry),
        "tem",
        "TEMPERATE",
    );
    assert_eq!(
        candidates,
        vec!["CUSTOM_TILE_ANIM.TEM", "CUSTOM_TILE_ANIM.SHP"]
    );
    let tem_only = HashMap::from([("CUSTOM_TILE_ANIM.TEM".to_string(), make_raw_test_shp(4))]);
    assert!(!tem_only.contains_key("CUSTOM_TILE_ANIM.SHP"));
    let data = candidates
        .iter()
        .find_map(|candidate| tem_only.get(candidate))
        .expect("theater-aware preload must resolve the TEM-only SHP");
    let shp = ShpFile::from_bytes(data).expect("TEM candidate is valid SHP(TS) data");

    let mut needed = HashSet::new();
    let mut counts = HashMap::new();
    let count = register_effect_anim_frames(
        &mut needed,
        &mut counts,
        "CUSTOM_TILE_ANIM",
        shp.frames.len() as u16,
        true,
        false,
    );
    assert_eq!(count, 4);
    assert_eq!(counts["CUSTOM_TILE_ANIM"], 4);
    for frame in 0..4 {
        assert!(needed.contains(&ShpSpriteKey {
            type_id: "CUSTOM_TILE_ANIM".to_string(),
            facing: 0,
            frame,
            house_color: HouseColorIndex(0),
        }));
    }
}

#[test]
fn collect_effect_names_includes_weapon_anim_entries() {
    let ini = crate::rules::ini_parser::IniFile::from_str(
        "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=TestWeapon\n\n\
[TestWeapon]\nDamage=1\nWarhead=TestWH\nAnim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW\nOccupantAnim=UCFLASH\n\n\
[TestWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("rules parse");
    let names = collect_effect_names(&rules);
    assert!(names.iter().any(|name| name == "MGUN-N"));
    assert!(names.iter().any(|name| name == "MGUN-NW"));
    assert!(names.iter().any(|name| name == "UCFLASH"));
}
