//! Converts generator state into the engine's in-memory `MapFile`.
//!
//! Terrain, overlay and waypoint population lands with the phases; this
//! establishes the header geometry and the empty sections the rest of the
//! loader expects.

use std::collections::HashMap;

use crate::map::map_file::{MapFile, MapHeader};
use crate::rules::ini_parser::IniFile;

use super::options::RmgOptions;

/// The width/height options scale by a third before being turned into cell
/// counts, and every map type except the two island types caps that scale.
const DIMENSION_SCALE: f32 = 0.333_333_34;
const DIMENSION_SCALE_CAP: f32 = 1.2;

/// The full map is the generated interior plus a fixed margin, and the
/// playable area is inset from the top-left corner.
const SIZE_PAD_X: u32 = 4;
const SIZE_PAD_Y: u32 = 12;
const LOCAL_LEFT: u32 = 2;
const LOCAL_TOP: u32 = 5;

/// Map the theater option to the engine's theater name.
///
/// Out-of-range values fall back to temperate, which matches the original
/// leaving the option unclamped and indexing a fixed-stride name table.
pub fn theater_name(theater: i32) -> &'static str {
    match theater {
        1 => "SNOW",
        2 => "URBAN",
        3 => "DESERT",
        4 => "NEWURBAN",
        _ => "TEMPERATE",
    }
}

/// Scale factor applied to a width/height option.
///
/// Island map types are exempt from the cap, which is how they get to be
/// larger than any other type at the same option value.
pub fn dimension_scale(option: i32, map_type: i32) -> f32 {
    let scale = option as f32 * DIMENSION_SCALE;
    if !matches!(map_type, 3 | 4) && scale >= DIMENSION_SCALE_CAP {
        DIMENSION_SCALE_CAP
    } else {
        scale
    }
}

/// A `MapFile` with header and empty sections, ready for phases to fill.
///
/// `gen_w`/`gen_h` are the generated interior dimensions.
pub fn empty_map_file(options: &RmgOptions, gen_w: u32, gen_h: u32) -> MapFile {
    let header = MapHeader {
        theater: theater_name(options.theater).to_string(),
        width: gen_w + SIZE_PAD_X,
        height: gen_h + SIZE_PAD_Y,
        local_left: LOCAL_LEFT,
        local_top: LOCAL_TOP,
        local_width: gen_w,
        local_height: gen_h,
    };
    MapFile {
        header,
        basic: Default::default(),
        briefing: Default::default(),
        preview: Default::default(),
        cells: Vec::new(),
        entities: Vec::new(),
        overlays: Vec::new(),
        overlay_data: Default::default(),
        smudges: Vec::new(),
        terrain_objects: Vec::new(),
        waypoints: HashMap::new(),
        cell_tags: Default::default(),
        tags: Default::default(),
        triggers: Default::default(),
        events: Default::default(),
        actions: Default::default(),
        local_variables: Default::default(),
        trigger_graph: Default::default(),
        special_flags: Default::default(),
        explicit_tubes: Vec::new(),
        // `IniFile` has no `Default`; an empty document is the equivalent.
        ini: IniFile::from_str(""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theater_indices_map_to_engine_names() {
        assert_eq!(theater_name(0), "TEMPERATE");
        assert_eq!(theater_name(1), "SNOW");
        assert_eq!(theater_name(2), "URBAN");
        assert_eq!(theater_name(3), "DESERT");
        assert_eq!(theater_name(4), "NEWURBAN");
        assert_eq!(
            theater_name(99),
            "TEMPERATE",
            "the theater option is never clamped, so out-of-range must fall back"
        );
        assert_eq!(theater_name(-1), "TEMPERATE");
    }

    #[test]
    fn header_pads_size_and_insets_the_playable_area() {
        let map = empty_map_file(&RmgOptions::default(), 60, 60);
        assert_eq!(map.header.width, 64, "full width is interior + 4");
        assert_eq!(map.header.height, 72, "full height is interior + 12");
        assert_eq!(map.header.local_left, 2);
        assert_eq!(map.header.local_top, 5, "playable area starts at row 5");
        assert_eq!((map.header.local_width, map.header.local_height), (60, 60));
    }

    #[test]
    fn dimension_scale_caps_every_type_except_the_islands() {
        // option 3 -> exactly 1.0, under the cap for every type
        assert!((dimension_scale(3, 1) - 1.0).abs() < 1e-6);
        // island types are exempt
        assert!(dimension_scale(9, 3) > 1.2, "map type 3 is uncapped");
        assert!(dimension_scale(9, 4) > 1.2, "map type 4 is uncapped");
        // everything else saturates
        assert!((dimension_scale(9, 0) - 1.2).abs() < 1e-6);
        assert!((dimension_scale(9, 1) - 1.2).abs() < 1e-6);
        assert!((dimension_scale(9, 2) - 1.2).abs() < 1e-6);
    }

    #[test]
    fn generated_map_starts_with_no_content() {
        let map = empty_map_file(&RmgOptions::default(), 32, 32);
        assert!(map.cells.is_empty());
        assert!(map.overlays.is_empty());
        assert!(map.terrain_objects.is_empty());
        assert!(map.waypoints.is_empty());
    }

    /// Architecture invariant: the simulation must never depend on the
    /// generator. Generation is pre-play map construction.
    #[test]
    fn sim_does_not_reference_the_generator() {
        let mut offenders = Vec::new();
        for path in rust_files(std::path::Path::new("src/sim")) {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if text.contains("map::rmg") {
                offenders.push(path.display().to_string());
            }
        }
        assert!(
            offenders.is_empty(),
            "sim/ must not depend on the map generator: {offenders:?}"
        );
    }

    fn rust_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(root) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(rust_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
        out
    }
}
