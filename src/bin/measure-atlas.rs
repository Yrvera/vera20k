//! Measure UnitAtlas memory at a saturated 30-player game state.
//!
//! Estimates the worst-case memory footprint of the GPU unit atlas by
//! counting the cardinality of all (type_id, facing, frame, slope, layer)
//! combinations a 30-player saturated game could produce, then multiplying
//! by an average tile size and the per-pixel byte cost (1 byte for the new
//! R8Uint atlas).
//!
//! Used to verify the GPU remap architecture's memory budget claim (atlas
//! stays under 200 MB at 30-player saturation). The R8Uint atlas is 4×
//! cheaper than the previous Rgba8Unorm atlas, AND drops the house
//! dimension from the cache key — a combined ~120× memory win at
//! saturation versus the pre-Phase-1 baseline.
//!
//! Run with: `cargo run --release --bin measure-atlas`.

/// Representative roster size per player at saturation.
///
/// Real RA2/YR rosters have roughly 30 buildable VXL types per faction
/// (vehicles + voxel-turret buildings). Three factions × overlapping
/// rosters → ~30 unique VXL types at most, even with all factions active.
const VXL_TYPES_PER_PLAYER: usize = 30;

/// Body/composite facing buckets (matches `unit_atlas::UNIT_FACING_BUCKETS`).
const BODY_FACING_BUCKETS: usize = 64;

/// Turret/barrel facing buckets (matches `unit_atlas::TURRET_FACING_BUCKETS`).
const TURRET_FACING_BUCKETS: usize = 128;

/// Slope variants pre-rendered for ground vehicles (slopes 0..=16, ramps
/// + corner tilts). Aircraft never tilt and use slope_type=0 only.
const GROUND_SLOPE_VARIANTS: usize = 17;

/// HVA frame count per layer for animated VXLs. Most VXLs are static
/// (1 frame); a handful animate (cargo lifts, tesla coil). Empirical
/// average across the YR roster is ~1.1 frames per layer.
const AVG_HVA_FRAMES: usize = 1;

// Empirical post-bounding-box tile sizes at scale 1.045 (rasterizer output).
// Body and composite layers are ~48-56 px; turret/barrel layers are smaller
// (~28-36 px) because the geometry footprint shrinks once the body is
// excluded. Modeling these separately is more accurate than a uniform avg.
const BODY_TILE_BYTES: usize = 56 * 56;        // ~3136 bytes
const TURRET_BARREL_TILE_BYTES: usize = 32 * 32; // ~1024 bytes
const COMPOSITE_TILE_BYTES: usize = 48 * 48;     // ~2304 bytes

/// R8Uint atlas: 1 byte per pixel.
const BYTES_PER_PIXEL: usize = 1;

fn main() {
    // Cardinality of unique sprite keys at 30-player saturation.
    //
    // The atlas is now house-neutral (Phase 1 architectural change), so
    // 30 players DON'T multiply key count. We count keys for the union of
    // all rosters — capped at VXL_TYPES_PER_PLAYER since real rosters
    // overlap heavily across factions.
    //
    // Roster mix (empirical YR baseline at saturation):
    //   - Ground turret-bearing (~17 types): body + turret + barrel layers,
    //     uses 17 slope variants (drives onto ramps).
    //   - Ground composite (~8 types): single composite layer, 17 slopes.
    //   - Aircraft (~5 types): single composite layer, 1 slope (no tilt).
    let ground_turret_types: usize = 17;
    let ground_composite_types: usize = 8;
    let aircraft_types: usize = 5;
    debug_assert_eq!(
        ground_turret_types + ground_composite_types + aircraft_types,
        VXL_TYPES_PER_PLAYER,
    );

    // Ground turret-bearing keys:
    //   Body × BODY_FACING × HVA_FRAMES × GROUND_SLOPE_VARIANTS
    //   + (Turret + Barrel) × TURRET_FACING × HVA_FRAMES × GROUND_SLOPE_VARIANTS
    let body_keys: usize = ground_turret_types
        * BODY_FACING_BUCKETS
        * AVG_HVA_FRAMES
        * GROUND_SLOPE_VARIANTS;
    let turret_barrel_keys: usize = ground_turret_types
        * 2
        * TURRET_FACING_BUCKETS
        * AVG_HVA_FRAMES
        * GROUND_SLOPE_VARIANTS;

    // Ground composite (non-turret): single composite layer, 17 slopes.
    let ground_composite_keys: usize = ground_composite_types
        * BODY_FACING_BUCKETS
        * AVG_HVA_FRAMES
        * GROUND_SLOPE_VARIANTS;

    // Aircraft: single composite layer, 1 slope (slope_type = 0 always).
    let aircraft_keys: usize =
        aircraft_types * BODY_FACING_BUCKETS * AVG_HVA_FRAMES;

    let composite_keys: usize = ground_composite_keys + aircraft_keys;
    let total_keys: usize = body_keys + turret_barrel_keys + composite_keys;

    // Memory broken down by layer type using realistic per-layer tile sizes.
    let body_bytes: usize = body_keys * BODY_TILE_BYTES * BYTES_PER_PIXEL;
    let turret_barrel_bytes: usize =
        turret_barrel_keys * TURRET_BARREL_TILE_BYTES * BYTES_PER_PIXEL;
    let composite_bytes: usize = composite_keys * COMPOSITE_TILE_BYTES * BYTES_PER_PIXEL;
    let total_bytes: usize = body_bytes + turret_barrel_bytes + composite_bytes;
    let total_mb: f64 = total_bytes as f64 / 1_048_576.0;

    println!("=== Saturated Unit Atlas Memory Estimate (Phase 1 R8Uint) ===");
    println!("VXL types per player:           {}", VXL_TYPES_PER_PLAYER);
    println!("  ground turret-bearing:        {}", ground_turret_types);
    println!("  ground composite:             {}", ground_composite_types);
    println!("  aircraft:                     {}", aircraft_types);
    println!("Body keys:                      {} ({:.1} MB)",
        body_keys, body_bytes as f64 / 1_048_576.0);
    println!("Turret + barrel keys:           {} ({:.1} MB)",
        turret_barrel_keys, turret_barrel_bytes as f64 / 1_048_576.0);
    println!("Composite keys (ground+air):    {} ({:.1} MB)",
        composite_keys, composite_bytes as f64 / 1_048_576.0);
    println!("Total atlas keys:               {}", total_keys);
    println!("Estimated atlas memory:         {} bytes ({:.1} MB)", total_bytes, total_mb);
    println!();

    // Compare against the pre-Phase-1 baseline:
    //  - 30 (house dimension multiplier, now removed) × 4 (RGBA, now R8Uint).
    let pre_phase1_bytes: usize = total_bytes * 30 * 4;
    let pre_phase1_mb: f64 = pre_phase1_bytes as f64 / 1_048_576.0;
    println!();
    println!(
        "Pre-Phase-1 baseline (30 houses × 4 bytes/pixel): {} bytes ({:.1} MB)",
        pre_phase1_bytes, pre_phase1_mb,
    );
    println!("Phase 1 reduction:                                {}× smaller",
        pre_phase1_bytes / total_bytes.max(1));

    // Verify the design's claimed budget (200 MB).
    const BUDGET_MB: f64 = 200.0;
    println!();
    if total_mb <= BUDGET_MB {
        println!("OK: under {:.0} MB budget ({:.1} MB headroom)", BUDGET_MB, BUDGET_MB - total_mb);
    } else {
        eprintln!("FAIL: exceeds {:.0} MB budget by {:.1} MB", BUDGET_MB, total_mb - BUDGET_MB);
        std::process::exit(1);
    }
}
