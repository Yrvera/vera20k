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
fn overlay_radar_loader_separates_cell_anim_from_overlay_own_shp() {
    let directory = RadarTestDirectory::new();
    let mut own_tib = vec![[0, 0, 0]; 12];
    own_tib[11] = [169, 155, 61]; // retail TIB01 frame 11: ignored without CellAnim
    directory.write_shp("TIB01.SHP", &own_tib);
    own_tib[11] = [201, 202, 203];
    directory.write_shp("TIBANIM.SHP", &own_tib);
    let mut twinkle = vec![[0, 0, 0]; 12];
    twinkle[11] = [31, 41, 59];
    directory.write_shp("TWNK1.SHP", &twinkle);
    directory.write_shp("PLAIN.SHP", &[[8, 9, 10]]);
    directory.write_shp("PLAINFALLBACK.SHP", &[[21, 22, 23]]);
    directory.write_shp("BRIDGE1.SHP", &[[0, 0, 6]]); // retail bridge.tem f0
    let assets = AssetManager::from_loose_root_for_test(directory.path());
    let rules = IniFile::from_str(
        "[OverlayTypes]\n\
         0=TIB01\n1=TIBANIM\n2=TIBMISSING\n3=PLAIN\n4=PLAINFALLBACKOVL\n5=TIBUNKNOWN\n\
         [Animations]\n0=TWNK1\n1=MISSINGANIM\n2=PLAINFALLBACK\n\
         [TIB01]\nTiberium=yes\n\
         [TIBANIM]\nTiberium=yes\nCellAnim=TWNK1\n\
         [TIBMISSING]\nTiberium=yes\nCellAnim=MISSINGANIM\n\
         [PLAINFALLBACKOVL]\nCellAnim=PLAINFALLBACK\n\
         [TIBUNKNOWN]\nTiberium=yes\nCellAnim=NOTREGISTERED\n",
    );
    let registry = OverlayTypeRegistry::from_ini(&rules, None);
    let art = ArtRegistry::from_ini(&rules);
    let names = BTreeMap::from([(24, "BRIDGE1".to_string())]);
    let colors = compute_overlay_radar_colors(
        &assets,
        &registry,
        &names,
        "tem",
        "TEMPERATE",
        &rules,
        &art,
    );

    assert_eq!(registry.flags(0).and_then(|flags| flags.cell_anim.as_deref()), None);
    assert_eq!(
        registry.flags(1).and_then(|flags| flags.cell_anim.as_deref()),
        Some("TWNK1"),
    );
    assert_eq!(
        registry.flags(5).and_then(|flags| flags.cell_anim.as_deref()),
        None,
        "FindByName failure leaves the native pointer null",
    );
    assert!(!colors.contains_key(&(0, 11)), "stock TIB01 own SHP is ignored");
    assert_eq!(
        colors.get(&(1, 11)),
        Some(&[31, 41, 59]),
        "tiberium reads the referenced CellAnim SHP, not TIBANIM.SHP",
    );
    assert!(!colors.contains_key(&(2, 11)), "missing CellAnim SHP stays absent");
    assert_eq!(colors.get(&(3, 0)), Some(&[8, 9, 10]));
    assert!(
        !colors.contains_key(&(4, 0)),
        "non-tiberium radar sourcing remains on the overlay's own missing SHP",
    );
    assert_eq!(colors.get(&(24, 0)), Some(&[0, 0, 6]));
}
