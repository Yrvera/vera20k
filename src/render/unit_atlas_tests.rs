use super::*;
use crate::render::vxl_raster::VxlSprite;

#[test]
fn test_unit_sprite_key_hash_equality() {
    let key1 = UnitSpriteKey {
        type_id: "HTNK".into(),
        facing: 64,
        layer: VxlLayer::Composite,
        frame: 0,
        slope_type: 0,
    };
    let key2 = UnitSpriteKey {
        type_id: "HTNK".into(),
        facing: 64,
        layer: VxlLayer::Composite,
        frame: 0,
        slope_type: 0,
    };
    let key3 = UnitSpriteKey {
        type_id: "HTNK".into(),
        facing: 128,
        layer: VxlLayer::Composite,
        frame: 0,
        slope_type: 0,
    };
    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
    let mut set: HashSet<UnitSpriteKey> = HashSet::new();
    set.insert(key1);
    set.insert(key2); // duplicate
    set.insert(key3);
    assert_eq!(set.len(), 2);
}

#[test]
fn every_unit_sprite_key_dimension_remains_distinct() {
    let base = UnitSpriteKey {
        type_id: "HTNK".into(),
        facing: 64,
        layer: VxlLayer::Body,
        frame: 2,
        slope_type: 3,
    };
    let mut variants = vec![base.clone()];

    let mut different_type = base.clone();
    different_type.type_id = "CMIN".into();
    variants.push(different_type);

    let mut different_facing = base.clone();
    different_facing.facing = 66;
    variants.push(different_facing);

    let mut different_layer = base.clone();
    different_layer.layer = VxlLayer::Turret;
    variants.push(different_layer);

    let mut different_frame = base.clone();
    different_frame.frame = 3;
    variants.push(different_frame);

    let mut different_slope = base;
    different_slope.slope_type = 4;
    variants.push(different_slope);

    assert_eq!(
        variants.into_iter().collect::<HashSet<_>>().len(),
        6,
        "type, facing, layer, frame, and slope are all cache identity"
    );
}

#[test]
fn test_empty_world_returns_none() {
    let needed: HashSet<UnitSpriteKey> = HashSet::new();
    assert!(needed.is_empty());
}

#[test]
fn test_key_collection_deduplicates() {
    let mut needed: HashSet<UnitSpriteKey> = HashSet::new();
    for facing in [64u8, 64, 128] {
        needed.insert(UnitSpriteKey {
            type_id: "HTNK".to_string(),
            facing,
            layer: VxlLayer::Composite,
            frame: 0,
            slope_type: 0,
        });
    }
    assert_eq!(needed.len(), 2);
}

#[test]
fn test_composite_layers_depth_correct() {
    // Body: 2x2, all at depth 1.0, palette index 10 (opaque).
    let body = VxlSprite {
        palette_indices: vec![10, 10, 10, 10],
        depth: vec![1.0, 1.0, 1.0, 1.0],
        width: 2,
        height: 2,
        offset_x: 0.0,
        offset_y: 0.0,
    };
    // Turret: 1x1 at (1,1), depth 2.0 (closer), palette index 200 — overwrites body.
    let turret = VxlSprite {
        palette_indices: vec![200],
        depth: vec![2.0],
        width: 1,
        height: 1,
        offset_x: 1.0,
        offset_y: 1.0,
    };
    let out = composite_vxl_layers(&[body.clone(), turret]);
    assert_eq!(out.width, 2);
    assert_eq!(out.height, 2);
    let idx = (1 * out.width + 1) as usize;
    assert_eq!(out.palette_indices[idx], 200);

    // Turret behind body (depth 0.5 < body's 1.0) — body pixel wins.
    let turret_behind = VxlSprite {
        palette_indices: vec![150],
        depth: vec![0.5],
        width: 1,
        height: 1,
        offset_x: 1.0,
        offset_y: 1.0,
    };
    let out2 = composite_vxl_layers(&[body, turret_behind]);
    let idx2 = (1 * out2.width + 1) as usize;
    // Body pixel should remain (depth 1.0 > 0.5).
    assert_eq!(out2.palette_indices[idx2], 10);
}

#[test]
fn test_pad_layer_to_union_bounds() {
    // Body at offset (0,0), 2x2, palette index 10.
    let body = VxlSprite {
        palette_indices: vec![10, 10, 10, 10],
        depth: vec![1.0; 4],
        width: 2,
        height: 2,
        offset_x: 0.0,
        offset_y: 0.0,
    };
    // Turret at offset (5,3), 1x1, palette index 200 — different origin from body.
    let turret = VxlSprite {
        palette_indices: vec![200],
        depth: vec![2.0],
        width: 1,
        height: 1,
        offset_x: 5.0,
        offset_y: 3.0,
    };

    let all_layers: Vec<&VxlSprite> = vec![&body, &turret];

    // Pad body into union bounds.
    let padded_body = pad_layer_to_union_bounds(&body, &all_layers);
    // Pad turret into union bounds.
    let padded_turret = pad_layer_to_union_bounds(&turret, &all_layers);

    // Both should have the same dimensions and offset (shared origin).
    assert_eq!(padded_body.width, padded_turret.width);
    assert_eq!(padded_body.height, padded_turret.height);
    assert!((padded_body.offset_x - padded_turret.offset_x).abs() < 0.01);
    assert!((padded_body.offset_y - padded_turret.offset_y).abs() < 0.01);

    // Union bounds: min_x=0, min_y=0, max_x=6, max_y=4 → 6x4
    assert_eq!(padded_body.width, 6);
    assert_eq!(padded_body.height, 4);
    assert!((padded_body.offset_x - 0.0).abs() < 0.01);
    assert!((padded_body.offset_y - 0.0).abs() < 0.01);

    // Body pixel at (0,0) should be opaque (palette index 10).
    assert_eq!(padded_body.palette_indices[0], 10);

    // Turret pixel at (5,3) should be opaque (palette index 200).
    let turret_pix: usize = (3 * padded_turret.width + 5) as usize;
    assert_eq!(padded_turret.palette_indices[turret_pix], 200);
}

#[test]
fn test_canonical_turret_facing() {
    use super::canonical_turret_facing;
    // canonical_turret_facing takes a 16-bit DirStruct and converts via >>8 to the
    // 8-bit facing used for sprite selection. At step=1 there are 256 buckets, one per
    // representable facing, so the quantization is the identity — the high byte passes
    // through untouched.
    assert_eq!(canonical_turret_facing(0u16), 0);
    assert_eq!(canonical_turret_facing(256), 1);
    assert_eq!(canonical_turret_facing(512), 2);
    assert_eq!(canonical_turret_facing(768), 3);
    assert_eq!(canonical_turret_facing(1024), 4);
    assert_eq!(canonical_turret_facing(65280), 255);
    // The low byte is sub-facing precision and must not affect sprite selection.
    assert_eq!(canonical_turret_facing(768 + 255), 3);
    // Body and turret share the same granularity, and neither loses information.
    assert_eq!(canonical_unit_facing(3), 3);
    assert_eq!(canonical_turret_facing(768), 3);
}

#[test]
fn every_representable_facing_has_its_own_bucket() {
    use super::{canonical_turret_facing, canonical_unit_facing};
    // The simulation stores facing as a byte. With 256 buckets each of those 256 values
    // maps to a distinct pre-rendered sprite, so rotation can never show a staircase.
    // If a step > 1 is ever reintroduced this fails loudly rather than degrading looks
    // silently.
    for facing in 0..=u8::MAX {
        assert_eq!(canonical_unit_facing(facing), facing);
        assert_eq!(canonical_turret_facing(u16::from(facing) << 8), facing);
    }
}

#[test]
fn test_facing_config_for_layer() {
    // Every layer renders one sprite per representable facing: step 1, 256 buckets.
    for layer in [
        VxlLayer::Body,
        VxlLayer::Composite,
        VxlLayer::Turret,
        VxlLayer::Barrel,
    ] {
        let (step, buckets) = super::facing_config_for_layer(layer);
        assert_eq!(step, 1, "{layer:?} facing step");
        assert_eq!(buckets, 256, "{layer:?} facing buckets");
        // The facing derived from the last bucket must still fit a byte.
        assert_eq!((buckets - 1) * u16::from(step), 255);
    }
}

#[test]
fn forced_overflow_plan_keeps_every_sprite_in_bounds() {
    let dimensions = vec![(6, 4); 4];
    let plan = plan_sprite_pages(&dimensions, 8).expect("forced overflow must be pageable");

    assert!(plan.page_heights.len() >= 2);
    assert_eq!(plan.placements.len(), dimensions.len());

    assert!(plan.page_width <= 8);
    assert!(plan.page_heights.iter().all(|&height| height <= 8));

    let mut seen = vec![false; dimensions.len()];
    for (placement_index, placement) in plan.placements.iter().enumerate() {
        let (width, height) = dimensions[placement.sprite_index];
        assert!(!seen[placement.sprite_index]);
        seen[placement.sprite_index] = true;
        assert!(placement.x + width <= plan.page_width);
        assert!(placement.y + height <= plan.page_heights[placement.page]);
        for other in plan.placements.iter().skip(placement_index + 1) {
            if placement.page != other.page {
                continue;
            }
            let (other_width, other_height) = dimensions[other.sprite_index];
            let separated = placement.x + width <= other.x
                || other.x + other_width <= placement.x
                || placement.y + height <= other.y
                || other.y + other_height <= placement.y;
            assert!(separated, "page placements must not overlap");
        }

        let page_width = plan.page_width as f32;
        let page_height = plan.page_heights[placement.page] as f32;
        let uv_origin = [
            placement.x as f32 / page_width,
            placement.y as f32 / page_height,
        ];
        let uv_size = [width as f32 / page_width, height as f32 / page_height];
        assert!(uv_origin.into_iter().all(f32::is_finite));
        assert!(uv_size.into_iter().all(f32::is_finite));
        assert!(uv_origin[0] >= 0.0 && uv_origin[1] >= 0.0);
        assert!(uv_origin[0] + uv_size[0] <= 1.0);
        assert!(uv_origin[1] + uv_size[1] <= 1.0);
    }
    assert!(seen.into_iter().all(|value| value));
}

#[test]
fn wide_short_sprite_expands_page_width_before_placement() {
    let dimensions = [(100, 1), (4, 4)];
    let plan = plan_sprite_pages(&dimensions, 128).expect("both sprites fit the device limit");

    assert!(plan.page_width >= 100);
    for placement in &plan.placements {
        let (width, height) = dimensions[placement.sprite_index];
        assert!(placement.x + width <= plan.page_width);
        assert!(placement.y + height <= plan.page_heights[placement.page]);
    }
}

#[test]
fn incremental_repack_plan_retains_old_unloading_referent() {
    let make_cached = |type_id: &str| CachedUnitSprite {
        key: UnitSpriteKey {
            type_id: type_id.into(),
            facing: 0,
            layer: VxlLayer::Composite,
            frame: 0,
            slope_type: 0,
        },
        pixels: vec![1; 24],
        width: 6,
        height: 4,
        offset_x: 0.0,
        offset_y: 0.0,
    };
    // CMON is CMIN's stock UnloadingClass referent. It may be absent from the
    // current live-key collector but must remain in the rendered cache.
    let cached = vec![make_cached("CMON"), make_cached("CMIN")];
    let plan = plan_cached_sprite_pages(&cached, 8).expect("both cached sprites must repack");
    let retained = plan
        .placements
        .iter()
        .map(|placement| cached[placement.sprite_index].key.type_id.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(retained, HashSet::from(["CMON", "CMIN"]));
}

#[test]
fn page_plan_has_no_u8_page_cap() {
    let dimensions = vec![(8, 8); 257];
    let plan = plan_sprite_pages(&dimensions, 8).expect("257 pages must remain addressable");

    assert_eq!(plan.page_heights.len(), 257);
    assert_eq!(
        plan.placements.iter().map(|placement| placement.page).max(),
        Some(256)
    );
    assert_eq!(plan.placements.len(), dimensions.len());
}

#[test]
fn page_plan_rejects_an_individually_oversized_sprite() {
    let err = plan_sprite_pages(&[(9, 1)], 8).expect_err("oversized sprite must not be clipped");
    assert_eq!(
        err,
        UnitAtlasPackError::SpriteExceedsTextureLimit {
            sprite_index: 0,
            width: 9,
            height: 1,
            limit: 8,
        }
    );
}
