use super::compute_overlay_radar_colors;
use crate::assets::asset_manager::AssetManager;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static RADAR_TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct RadarTestDirectory(PathBuf);

impl RadarTestDirectory {
    fn new() -> Self {
        let sequence = RADAR_TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vera20k-overlay-radar-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create radar test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write_shp(&self, name: &str, colors: &[[u8; 3]]) {
        let mut data = Vec::with_capacity(8 + colors.len() * 24);
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(colors.len() as u16).to_le_bytes());
        for color in colors {
            data.extend_from_slice(&[0; 12]);
            data.extend_from_slice(color);
            data.push(0);
            data.extend_from_slice(&[0; 8]);
        }
        std::fs::write(self.0.join(name), data).expect("write radar SHP");
    }
}

impl Drop for RadarTestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn overlay_radar_loader_uses_stock_header_rgb_and_native_frame_selection() {
    let directory = RadarTestDirectory::new();
    let mut tib = vec![[0, 0, 0]; 12];
    tib[11] = [169, 155, 61]; // retail TIB01 frame 11
    directory.write_shp("TIB01.SHP", &tib);
    directory.write_shp("BRIDGE1.SHP", &[[0, 0, 6]]); // retail bridge.tem f0
    let mut low_bridge = vec![[0, 0, 0]; 8];
    low_bridge[1] = [7, 8, 9];
    low_bridge[7] = [70, 80, 90];
    directory.write_shp("LOBRDG01.SHP", &low_bridge);
    let assets = AssetManager::from_loose_root_for_test(directory.path());
    let registry = OverlayTypeRegistry::empty();
    let rules = IniFile::from_str("");
    let art = ArtRegistry::from_ini(&rules);
    let names = BTreeMap::from([
        (24, "BRIDGE1".to_string()),
        (74, "LOBRDG01".to_string()),
        (102, "TIB01".to_string()),
    ]);
    let colors =
        compute_overlay_radar_colors(&assets, &registry, &names, "tem", &rules, &art);

    assert_eq!(colors.get(&(102, 11)), Some(&[169, 155, 61]));
    assert_eq!(colors.get(&(24, 0)), Some(&[0, 0, 6]));
    assert_eq!(colors.get(&(74, 1)), Some(&[7, 8, 9]));
    assert_eq!(
        colors.get(&(74, 7)),
        Some(&[70, 80, 90]),
        "all runtime-addressable frames stay loaded for dirty visits",
    );
    assert!(!colors.keys().any(|&(id, _)| id == 100), "missing asset stays absent");
}
