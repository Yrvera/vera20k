//! Tests for art.ini parsing and art-resolution layering.

use super::*;

#[test]
fn test_apply_theater_letter() {
    assert_eq!(apply_theater_letter("GAPOWR", "TEMPERATE"), "GTPOWR");
    assert_eq!(apply_theater_letter("GAPOWR", "SNOW"), "GAPOWR");
    assert_eq!(apply_theater_letter("NAHAND", "TEMPERATE"), "NTHAND");
    assert_eq!(apply_theater_letter("X", "TEMPERATE"), "X");
    assert_eq!(apply_theater_letter("GAPOWR", "UNKNOWN"), "GAPOWR");
}

#[test]
fn test_hardcoded_new_theater_prefixes() {
    let reg: ArtRegistry = ArtRegistry::empty();
    assert!(reg.should_use_new_theater("GAPOWR"));
    assert!(reg.should_use_new_theater("GTPOWR"));
    assert!(reg.should_use_new_theater("NAHAND"));
    assert!(reg.should_use_new_theater("NTHAND"));
    assert!(reg.should_use_new_theater("CAOUTP"));
    assert!(reg.should_use_new_theater("CTOUTP"));
    assert!(!reg.should_use_new_theater("MTNK"));
    assert!(!reg.should_use_new_theater("E1"));
    assert!(!reg.should_use_new_theater("HTNK"));
}

#[test]
fn test_object_shp_candidates_no_new_theater() {
    let reg: ArtRegistry = ArtRegistry::empty();
    let candidates: Vec<String> = object_shp_candidates(Some(&reg), "E1", "tem", "TEMPERATE");
    assert_eq!(candidates, vec!["E1.SHP", "E1.TEM"]);
}

#[test]
fn test_object_shp_candidates_with_new_theater() {
    let reg: ArtRegistry = ArtRegistry::empty();
    let candidates: Vec<String> = object_shp_candidates(Some(&reg), "GAPOWR", "tem", "TEMPERATE");
    assert_eq!(
        candidates,
        vec![
            "GTPOWR.SHP",
            "GTPOWR.TEM",
            "GGPOWR.SHP",
            "GGPOWR.TEM",
            "GAPOWR.SHP",
            "GAPOWR.TEM",
        ]
    );
}

#[test]
fn test_object_shp_candidates_dedupe_when_substitution_matches_original() {
    let reg: ArtRegistry = ArtRegistry::empty();
    let candidates: Vec<String> = object_shp_candidates(Some(&reg), "GAPOWR", "sno", "SNOW");
    assert_eq!(
        candidates,
        vec!["GAPOWR.SHP", "GAPOWR.SNO", "GGPOWR.SHP", "GGPOWR.SNO",]
    );
}

#[test]
fn test_apply_generic_letter() {
    assert_eq!(apply_generic_letter("GAPOWR"), "GGPOWR");
    assert_eq!(apply_generic_letter("NAHAND"), "NGHAND");
    assert_eq!(apply_generic_letter("X"), "X");
}

#[test]
fn test_voxel_asset_names() {
    let (vxl, hva) = voxel_asset_names("HTNK");
    assert_eq!(vxl, "HTNK.VXL");
    assert_eq!(hva, "HTNK.HVA");
}

#[test]
fn test_from_ini_parses_entries() {
    let ini: IniFile = IniFile::from_str(
        "[GAPOWR]\nNewTheater=yes\nCameo=GAPICON\n\n[HTNK]\nVoxel=yes\nAltCameo=HTKALT\n\n[NACNST]\nImage=CIVNC\n\n[GI]\nCrawls=yes\nFireUp=2\nSecondaryFire=4\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    assert_eq!(reg.len(), 4);

    let gapowr: &ArtEntry = reg.get("GAPOWR").expect("GAPOWR exists");
    assert!(gapowr.new_theater);
    assert!(!gapowr.voxel);
    assert!(gapowr.image.is_none());
    assert_eq!(gapowr.cameo.as_deref(), Some("GAPICON"));

    let htnk: &ArtEntry = reg.get("HTNK").expect("HTNK exists");
    assert!(!htnk.new_theater);
    assert!(htnk.voxel);
    assert_eq!(htnk.alt_cameo.as_deref(), Some("HTKALT"));

    let nacnst: &ArtEntry = reg.get("NACNST").expect("NACNST exists");
    assert_eq!(nacnst.image.as_deref(), Some("CIVNC"));

    let gi: &ArtEntry = reg.get("GI").expect("GI exists");
    assert!(gi.crawls);
    assert_eq!(gi.fire_up, 2);
    assert_eq!(gi.fire_prone, 2);
    assert_eq!(gi.secondary_fire, 4);
    assert_eq!(gi.secondary_prone, 4);
}

#[test]
fn parses_hidden_occupancy_art_fields() {
    let ini: IniFile = IniFile::from_str(
        "[GAREFN]\nCanHideThings=no\nOccupyHeight=4\n\n[GAPOWR]\nHeight=3\n\n[NAPOWR]\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    assert!(!reg.can_hide_things("GAREFN"));
    assert_eq!(reg.occupy_height("GAREFN"), 4);
    assert!(reg.can_hide_things("GAPOWR"));
    assert_eq!(reg.occupy_height("GAPOWR"), 3);
    assert!(reg.can_hide_things("NAPOWR"));
    assert_eq!(reg.occupy_height("NAPOWR"), 0);
    assert!(reg.can_hide_things("MISSING"));
    assert_eq!(reg.occupy_height("MISSING"), 2);
}

#[test]
fn test_resolve_effective_image_id_chain() {
    let ini: IniFile = IniFile::from_str("[NACNST]\nImage=CIVNC\n\n[E1]\n\n[MTNK]\nImage=MTNK\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    assert_eq!(reg.resolve_effective_image_id("NACNST", "NACNST"), "CIVNC");
    assert_eq!(reg.resolve_effective_image_id("E1", "E1"), "E1");
    assert_eq!(reg.resolve_effective_image_id("MTNK", "MTNK"), "MTNK");
    assert_eq!(reg.resolve_effective_image_id("UNKNOWN", "FOO"), "FOO");
}

#[test]
fn test_resolve_object_art_exposes_exact_layers() {
    let ini: IniFile = IniFile::from_str("[NACNST]\nImage=CIVNC\nBibShape=NACNSTBB\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    let resolved: ResolvedObjectArt<'_> = reg.resolve_object_art("NACNST", "NACNST");
    assert_eq!(resolved.base_art_id, "NACNST");
    assert_eq!(resolved.image_id, "CIVNC");
    assert_eq!(resolved.metadata_section_id, "NACNST");
    assert_eq!(
        resolved.entry.and_then(|e| e.bib_shape.as_deref()),
        Some("NACNSTBB")
    );
}

#[test]
fn test_resolve_metadata_entry_prefers_rules_image_section() {
    let ini: IniFile = IniFile::from_str(
        "[GAAIRC]\nBibShape=GAAIRCBB\nActiveAnim=GAAIRC_A\n\n[GAAIRC_A]\nLoopStart=0\nLoopEnd=4\nLoopCount=-1\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    let entry: &ArtEntry = reg
        .resolve_metadata_entry("AMRADR", "GAAIRC")
        .expect("AMRADR should use GAAIRC art metadata");
    assert_eq!(entry.bib_shape.as_deref(), Some("GAAIRCBB"));
    assert_eq!(entry.building_anims.len(), 1);
    assert_eq!(entry.building_anims[0].anim_type, "GAAIRC_A");
}

#[test]
fn test_resolve_metadata_entry_keeps_type_section_when_it_owns_metadata() {
    let ini: IniFile = IniFile::from_str("[NACNST]\nImage=CIVNC\nBibShape=NACNSTBB\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    let entry: &ArtEntry = reg
        .resolve_metadata_entry("NACNST", "NACNST")
        .expect("NACNST metadata should stay on its own section");
    assert_eq!(entry.bib_shape.as_deref(), Some("NACNSTBB"));
    assert_eq!(entry.image.as_deref(), Some("CIVNC"));
}

#[test]
fn test_resolve_declared_cameo_id_prefers_art_data() {
    let ini: IniFile = IniFile::from_str(
        "[E1]\nCameo=E1CAMEO\n\n[MTNK]\nAltCameo=MTNKALT\n\n[NACNST]\nImage=CIVNC\n[CIVNC]\nCameo=CIVICON\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);

    assert_eq!(reg.resolve_declared_cameo_id("E1", "E1"), "E1CAMEO");
    assert_eq!(reg.resolve_declared_cameo_id("MTNK", "MTNK"), "MTNKALT");
    assert_eq!(reg.resolve_declared_cameo_id("NACNST", "NACNST"), "CIVICON");
    assert_eq!(reg.resolve_declared_cameo_id("UNKNOWN", "FOO"), "FOO");
}

#[test]
fn test_new_theater_from_ini_key() {
    let ini: IniFile = IniFile::from_str("[MYCIVBLD]\nNewTheater=yes\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    assert!(reg.should_use_new_theater("MYCIVBLD"));
}

#[test]
fn test_parse_building_anims() {
    let ini: IniFile = IniFile::from_str(
        "[CAOILD]\nActiveAnim=CAOILD_A\nActiveAnimYSort=362\nActiveAnimTwo=CAOILD_F\nActiveAnimTwoZAdjust=-50\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("CAOILD").expect("CAOILD exists");
    assert_eq!(entry.building_anims.len(), 2);

    assert_eq!(entry.building_anims[0].anim_type, "CAOILD_A");
    assert_eq!(entry.building_anims[0].y_sort, 362);
    assert_eq!(entry.building_anims[0].z_adjust, 0);

    assert_eq!(entry.building_anims[1].anim_type, "CAOILD_F");
    assert_eq!(entry.building_anims[1].y_sort, 0);
    assert_eq!(entry.building_anims[1].z_adjust, -50);
}

#[test]
fn test_no_building_anims_for_regular_entry() {
    let ini: IniFile = IniFile::from_str("[E1]\nVoxel=no\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("E1").expect("E1 exists");
    assert!(entry.building_anims.is_empty());
}

#[test]
fn test_parse_turret_offset() {
    let ini: IniFile = IniFile::from_str("[HTK]\nVoxel=yes\nTurretOffset=-80\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("HTK").expect("HTK exists");
    assert_eq!(entry.turret_offset, -80);
}

#[test]
fn test_resolve_declared_palette_id() {
    let ini: IniFile = IniFile::from_str("[TEST]\nImage=TESTART\n\n[TESTART]\nPalette=anim\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    assert_eq!(
        reg.resolve_declared_palette_id("TEST", "TEST"),
        Some("anim".to_string())
    );
}

#[test]
fn test_anim_candidates_use_anim_section_flags() {
    let ini: IniFile = IniFile::from_str("[CAOILD_A]\nImage=CAOILDX\nNewTheater=yes\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let image_id: String = reg.resolve_effective_image_id("CAOILD_A", "CAOILD_A");
    let candidates: Vec<String> =
        anim_shp_candidates(Some(&reg), "CAOILD_A", &image_id, "urb", "NEWURBAN");
    assert_eq!(
        candidates,
        vec![
            "CNOILDX.SHP",
            "CNOILDX.URB",
            "CGOILDX.SHP",
            "CAOILDX.SHP",
            "CAOILDX.URB",
        ]
    );
}

#[test]
fn test_make_candidates_use_deduped_uppercase_names() {
    let reg: ArtRegistry = ArtRegistry::empty();
    let candidates: Vec<String> = make_shp_candidates(Some(&reg), "GAPOWR", "sno", "SNOW");
    assert_eq!(
        candidates,
        vec![
            "GAPOWRMK.SHP",
            "GAPOWRMK.SNO",
            "GGPOWRMK.SHP",
            "GGPOWRMK.SNO",
        ]
    );
}

#[test]
fn test_resolve_overlay_image_id_and_candidates() {
    let art_ini: IniFile = IniFile::from_str("[LOBRDG27]\nImage=LOBRDGX\nTheater=yes\n");
    let rules_ini: IniFile = IniFile::from_str("[LOBRDG27]\nImage=LOBRDGY\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&art_ini);

    let image_id: String = reg.resolve_overlay_image_id("LOBRDG27", &rules_ini);
    assert_eq!(image_id, "LOBRDGY");

    let candidates: Vec<String> =
        overlay_shp_candidates(Some(&reg), "LOBRDG27", &image_id, "tem", "TEMPERATE");
    assert_eq!(candidates, vec!["LOBRDGY.TEM", "LOBRDGY.SHP"]);
    assert!(!candidates.iter().any(|name| name.contains("LOBRDG26")));
}

#[test]
fn parses_add_occupy_from_ini() {
    let ini: IniFile =
        IniFile::from_str("[GAREFN]\nAddOccupy1=-1,0\nAddOccupy2=-1,-1\nRemoveOccupy1=3,1\n");
    let registry: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = registry.get("GAREFN").expect("GAREFN");
    assert_eq!(entry.add_occupy, vec![(-1, 0), (-1, -1)]);
    assert_eq!(entry.remove_occupy, vec![(3, 1)]);
}

#[test]
fn add_remove_occupy_empty_when_no_keys() {
    let ini: IniFile = IniFile::from_str("[FOO]\nFoundation=2x2\n");
    let registry: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = registry.get("FOO").expect("FOO");
    assert!(entry.add_occupy.is_empty());
    assert!(entry.remove_occupy.is_empty());
}

#[test]
fn add_remove_occupy_scans_sparse_numbered_keys() {
    let ini: IniFile = IniFile::from_str(
        "[FOO]\n\
         AddOccupy1=-1,0\n\
         AddOccupy3=2,3\n\
         RemoveOccupy1=4,5\n\
         RemoveOccupy4=-2,-3\n",
    );
    let registry: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = registry.get("FOO").expect("FOO");
    assert_eq!(entry.add_occupy, vec![(-1, 0), (2, 3)]);
    assert_eq!(entry.remove_occupy, vec![(4, 5), (-2, -3)]);
}

#[test]
fn add_occupy_skips_malformed_entries() {
    let ini: IniFile =
        IniFile::from_str("[FOO]\nAddOccupy1=not_a_pair\nAddOccupy2=1,2\nAddOccupy4=3,4\n");
    let registry: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = registry.get("FOO").expect("FOO");
    assert_eq!(entry.add_occupy, vec![(1, 2), (3, 4)]);
}

#[test]
fn parses_anim_smudge_flags() {
    let ini = IniFile::from_bytes(
        b"[ANIMA]\n\
          Scorch=yes\n\
          \n\
          [ANIMB]\n\
          Crater=yes\n\
          ForceBigCraters=yes\n\
          \n\
          [ANIMC]\n",
    )
    .unwrap();
    let reg = ArtRegistry::from_ini(&ini);
    let a = reg.get("ANIMA").unwrap();
    assert!(a.scorch);
    assert!(!a.crater);
    let b = reg.get("ANIMB").unwrap();
    assert!(!b.scorch);
    assert!(b.crater);
    assert!(b.force_big_craters);
    let c = reg.get("ANIMC").unwrap();
    assert!(!c.scorch);
    assert!(!c.crater);
    assert!(!c.force_big_craters);
}

#[test]
fn parse_gaairc_four_pads() {
    let ini: IniFile = IniFile::from_str(
        "[GAAIRC]\n\
         DockingOffset0=0,-128,0\n\
         DockingOffset1=0,128,0\n\
         DockingOffset2=256,-128,0\n\
         DockingOffset3=256,128,0\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("GAAIRC").expect("GAAIRC entry");
    assert_eq!(entry.pads.len(), 4, "should parse all 4 offsets");
    assert_eq!(entry.pads[0].lepton_offset, (0, -128, 0));
    assert_eq!(entry.pads[1].lepton_offset, (0, 128, 0));
    assert_eq!(entry.pads[2].lepton_offset, (256, -128, 0));
    assert_eq!(entry.pads[3].lepton_offset, (256, 128, 0));
}

#[test]
fn parse_no_docking_offsets_yields_empty_vec() {
    let ini: IniFile = IniFile::from_str("[GAHPAD]\nHeight=1\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("GAHPAD").expect("GAHPAD entry");
    assert!(entry.pads.is_empty(), "no offsets → empty pads vec");
}

#[test]
fn parse_partial_offsets_collects_what_exists() {
    // art has only DockingOffset0 and DockingOffset2 (gap at 1).
    // Parser collects what's present. The art→rules merge handles sizing to NumberOfDocks.
    let ini: IniFile = IniFile::from_str(
        "[ODD]\n\
         DockingOffset0=64,0,0\n\
         DockingOffset2=192,0,0\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("ODD").expect("ODD entry");
    assert_eq!(entry.pads.len(), 2, "filter_map skips missing index 1");
    assert_eq!(entry.pads[0].lepton_offset, (64, 0, 0));
    assert_eq!(entry.pads[1].lepton_offset, (192, 0, 0));
}

#[test]
fn test_parses_guardian_gi_sequence_frames() {
    // GGI's GuardianGISequence has Deploy=300,15,0; Undeploy=180,2,2;
    // DeployedFire=323,6,6. ArtEntry should pull the middle integer of each.
    let ini: IniFile = IniFile::from_str(
        "[GGI]\n\
         Sequence=GuardianGISequence\n\
         \n\
         [GuardianGISequence]\n\
         Ready=0,1,1\n\
         Walk=8,6,6\n\
         Deploy=300,15,0\n\
         Undeploy=180,2,2\n\
         Deployed=315,1,1\n\
         DeployedFire=323,6,6\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("GGI").expect("GGI entry");
    assert_eq!(entry.sequence.as_deref(), Some("GuardianGISequence"));
    assert_eq!(entry.deploy_frames, Some(15));
    assert_eq!(entry.undeploy_frames, Some(2));
    assert_eq!(entry.deployed_fire_frames, Some(6));
}

#[test]
fn test_sequence_frames_default_none_when_sequence_missing() {
    // No Sequence= key -> no lookup -> None for all three.
    let ini: IniFile = IniFile::from_str("[CIVHOSP]\nCameo=HOSPICON\n");
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("CIVHOSP").expect("entry");
    assert_eq!(entry.deploy_frames, None);
    assert_eq!(entry.undeploy_frames, None);
    assert_eq!(entry.deployed_fire_frames, None);
}

#[test]
fn test_sequence_frames_partial_some_missing() {
    // Sequence section exists but only defines a subset of the three keys.
    let ini: IniFile = IniFile::from_str(
        "[E1]\n\
         Sequence=GISequence\n\
         \n\
         [GISequence]\n\
         Deploy=100,8,0\n",
    );
    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    let entry: &ArtEntry = reg.get("E1").expect("E1 entry");
    assert_eq!(entry.deploy_frames, Some(8));
    assert_eq!(entry.undeploy_frames, None);
    assert_eq!(entry.deployed_fire_frames, None);
}

#[test]
fn test_parse_sequence_frames_helper() {
    assert_eq!(parse_sequence_frames("300,15,0"), Some(15));
    assert_eq!(parse_sequence_frames(" 8 , 6 , 6 "), Some(6));
    assert_eq!(parse_sequence_frames("only-one"), None);
    assert_eq!(parse_sequence_frames("a,b,c"), None);
    assert_eq!(parse_sequence_frames(""), None);
}
