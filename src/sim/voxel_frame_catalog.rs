//! GPU-independent HVA frame-count catalog (F09).
//!
//! Voxel animation frame counts are authoritative sim metadata — the
//! `VoxelAnimation` component advances by them — but were historically read
//! back from the renderer's unit atlas after texture building. This module
//! owns the assets/rules half with no GPU involvement: it resolves each voxel
//! type's model variants and layers exactly as the atlas seeding does and
//! parses `.hva` frame counts directly. The GPU-free construction path (app
//! and headless) and the renderer's atlas seeding consume the same functions,
//! so presentation and simulation cannot disagree on a frame count.
//!
//! Depends on `assets/`, `rules/`, and sim component/store types only.

use std::collections::BTreeMap;

use crate::assets::asset_manager::AssetManager;
use crate::assets::hva_file::HvaFile;
use crate::rules::art_data::{self, ArtRegistry};
use crate::rules::ruleset::RuleSet;
use crate::sim::components::VxlLayer;

pub(crate) const NO_SPAWN_ALT_SUFFIX: &str = "WO";

/// One voxel model that an entity can select at presentation time.
///
/// Initial atlas construction and incremental coverage checks must enumerate
/// the same set. Otherwise an already-valid base model can hide a missing
/// UnloadingClass or `%sWO` auxiliary model until the draw lookup fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnitAtlasVariant {
    pub(crate) type_id: String,
    pub(crate) has_turret: bool,
}

pub(crate) fn unit_atlas_variants(type_id: &str, rules: Option<&RuleSet>) -> Vec<UnitAtlasVariant> {
    let object = rules.and_then(|rules| rules.object(type_id));
    let mut variants = vec![UnitAtlasVariant {
        type_id: type_id.to_string(),
        has_turret: object.is_some_and(|object| object.has_turret),
    }];

    if let Some((unloading_type, unloading_object)) = object
        .and_then(|object| object.unloading_class.as_deref())
        .and_then(|unloading_type| {
            rules
                .and_then(|rules| rules.object(unloading_type))
                .map(|object| (unloading_type, object))
        })
    {
        variants.push(UnitAtlasVariant {
            type_id: unloading_type.to_string(),
            has_turret: unloading_object.has_turret,
        });
    }

    if object.is_some_and(|object| object.no_spawn_alt) {
        variants.push(UnitAtlasVariant {
            type_id: format!("{type_id}{NO_SPAWN_ALT_SUFFIX}"),
            // Native stores the `%sWO` pair in the same AuxVoxel slot used by
            // turrets. Stock NoSpawnAlt types therefore render it as one
            // composite body and cannot also own a turret.
            has_turret: false,
        });
    }

    variants
}

/// The layer set to seed atlas keys for, given a type's turret flag.
///
/// A turreted type gets separate Body/Turret layers, and a Barrel layer **only
/// when a barrel voxel actually exists**. Most turreted units model the gun as
/// part of the turret and ship no `…BARL.VXL`/`…BARREL.VXL` — the Soviet War
/// Miner is one. Seeding a Barrel key for those produced a key that could never
/// be satisfied: the Barrel branch of the renderer rebuilds the body and turret
/// sprites, finds no barrel, and returns `None`, so nothing is cached and the
/// whole attempt repeats on the next frame, forever. A single such unit on
/// screen logged ~135k render failures in four minutes of play and paid for two
/// discarded voxel rasterisations every frame.
pub(crate) fn seed_layers_for(
    asset_manager: &AssetManager,
    type_id: &str,
    has_turret: bool,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
) -> &'static [VxlLayer] {
    if !has_turret {
        return &[VxlLayer::Composite];
    }
    if has_barrel_voxel(asset_manager, type_id, rules, art) {
        &[VxlLayer::Body, VxlLayer::Turret, VxlLayer::Barrel]
    } else {
        &[VxlLayer::Body, VxlLayer::Turret]
    }
}

/// Whether this type ships a separate barrel voxel under either suffix the
/// renderer accepts. Resolves the image id exactly as the render path does, so
/// the seeding decision and the lookup can never disagree.
fn has_barrel_voxel(
    asset_manager: &AssetManager,
    type_id: &str,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
) -> bool {
    let rules_image: String = rules
        .and_then(|r| r.object(type_id))
        .map(|o| o.image.clone())
        .unwrap_or_else(|| type_id.to_string());
    let image: String = art
        .map(|a| a.resolve_effective_image_id(type_id, &rules_image))
        .unwrap_or_else(|| rules_image.to_uppercase());
    asset_manager
        .get_ref(&format!("{image}BARL.VXL"))
        .or_else(|| asset_manager.get_ref(&format!("{image}BARREL.VXL")))
        .is_some()
}

/// Detect the HVA animation frame count for a given (type_id, layer) combo.
///
/// Loads the HVA file from the asset manager and returns `frame_count`.
/// Returns 1 if no HVA is found or if parsing fails (single-frame default).
pub(crate) fn detect_hva_frame_count(
    asset_manager: &AssetManager,
    type_id: &str,
    layer: VxlLayer,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
) -> u32 {
    let rules_image: String = rules
        .and_then(|r| r.object(type_id))
        .map(|o| o.image.clone())
        .unwrap_or_else(|| type_id.to_string());
    let image: String = art
        .map(|a| a.resolve_effective_image_id(type_id, &rules_image))
        .unwrap_or_else(|| rules_image.to_uppercase());

    let hva_name: String = match layer {
        VxlLayer::Composite | VxlLayer::Body => art_data::voxel_asset_names(&image).1,
        VxlLayer::Turret => format!("{}TUR.HVA", image),
        VxlLayer::Barrel => format!("{}BARL.HVA", image),
    };

    let frame_count: u32 = asset_manager
        .get_ref(&hva_name)
        .and_then(|data| HvaFile::from_bytes(data).ok())
        .map(|h| h.frame_count)
        .unwrap_or(1);

    // Also try BARREL suffix if BARL had no HVA.
    if layer == VxlLayer::Barrel && frame_count <= 1 {
        let alt_name: String = format!("{}BARREL.HVA", image);
        let alt_count: u32 = asset_manager
            .get_ref(&alt_name)
            .and_then(|data| HvaFile::from_bytes(data).ok())
            .map(|h| h.frame_count)
            .unwrap_or(1);
        if alt_count > 1 {
            return alt_count;
        }
    }

    frame_count.max(1)
}

/// Build the frame-count catalog for every voxel entity in the store, keyed
/// by `(type_id, layer)` — the same enumeration (variants, then layers) the
/// unit-atlas seeding walks, minus the sprite keys.
pub(crate) fn build_voxel_frame_catalog(
    entities: &crate::sim::entity_store::EntityStore,
    interner: &crate::sim::intern::StringInterner,
    asset_manager: &AssetManager,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
) -> BTreeMap<(String, VxlLayer), u32> {
    let mut frame_counts: BTreeMap<(String, VxlLayer), u32> = BTreeMap::new();
    for entity in entities.values() {
        if !entity.is_voxel {
            continue;
        }
        let type_str = interner.resolve(entity.type_ref);
        for variant in unit_atlas_variants(type_str, rules) {
            for &layer in seed_layers_for(
                asset_manager,
                &variant.type_id,
                variant.has_turret,
                rules,
                art,
            ) {
                frame_counts
                    .entry((variant.type_id.clone(), layer))
                    .or_insert_with(|| {
                        detect_hva_frame_count(asset_manager, &variant.type_id, layer, rules, art)
                    });
            }
        }
    }
    frame_counts
}
