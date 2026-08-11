//! Unit tests for the INI parser (IniFile + IniSection).

use super::*;

#[test]
fn test_basic_parse() {
    let ini: IniFile = IniFile::from_str("[General]\nName=Test\nCost=1000\n[Combat]\nDamage=50\n");

    assert_eq!(ini.section_count(), 2);

    let general: &IniSection = ini.section("General").expect("Should have General");
    assert_eq!(general.get("Name"), Some("Test"));
    assert_eq!(general.get("Cost"), Some("1000"));

    let combat: &IniSection = ini.section("Combat").expect("Should have Combat");
    assert_eq!(combat.get("Damage"), Some("50"));
}

#[test]
fn test_raw_lookup_is_case_sensitive() {
    let ini: IniFile = IniFile::from_str("[VIRUS]\nName=Infantry\n[Virus]\nName=Warhead\n");

    assert_eq!(ini.section("VIRUS").unwrap().get("Name"), Some("Infantry"));
    assert_eq!(ini.section("Virus").unwrap().get("Name"), Some("Warhead"));
    assert!(ini.section("virus").is_none());
    assert!(ini.section("VIRUS").unwrap().get("name").is_none());
}

#[test]
fn section_header_preserves_spaces_inside_brackets() {
    let ini = IniFile::from_str("  [ Name ]  \nKey=Value\n");

    assert!(ini.section("Name").is_none());
    assert_eq!(ini.section(" Name ").unwrap().get("Key"), Some("Value"));
}

#[test]
fn empty_section_name_is_retained() {
    let ini = IniFile::from_str("[]\nKey=Value\n");

    assert_eq!(ini.section("").unwrap().get("Key"), Some("Value"));
}

#[test]
fn entryless_physical_sections_are_discarded() {
    let ini = IniFile::from_str(
        "[HeaderOnly]\n[CommentOnly]\n; no accepted entries\n\
         [EmptyValueOnly]\nKey=\n[Kept]\nKey=Value\n",
    );

    assert!(ini.section("HeaderOnly").is_none());
    assert!(ini.section("CommentOnly").is_none());
    assert!(ini.section("EmptyValueOnly").is_none());
    assert_eq!(ini.section_names(), vec!["Kept"]);
}

#[test]
fn retail_parabomb_empty_body_does_not_hide_later_definition() {
    // ARTMD has an entryless PARABOMB occurrence before this populated body.
    let ini = IniFile::from_str(
        "[PARABOMB]\n\n[PARABOMB]\nRate=200\nLoopStart=7\nLoopCount=15\n",
    );

    let parabomb = ini.section("PARABOMB").expect("populated PARABOMB body");
    assert_eq!(parabomb.get("Rate"), Some("200"));
    assert_eq!(parabomb.get("LoopStart"), Some("7"));
    assert_eq!(parabomb.get("LoopCount"), Some("15"));
}

#[test]
fn malformed_header_falls_through_to_key_value_parsing() {
    let ini = IniFile::from_str("[S]\n[foo=bar\n");

    assert_eq!(ini.section("S").unwrap().get("[foo"), Some("bar"));
}

#[test]
fn test_comments_and_blank_lines() {
    let text: &str = "\
; This is a comment
# This is ordinary junk

[Section1]
Key1=Value1
#Key=Visible

; Another comment
Key2=Value2
";
    let ini: IniFile = IniFile::from_str(text);

    assert_eq!(ini.section_count(), 1);
    let section: &IniSection = ini.section("Section1").unwrap();
    assert_eq!(section.get("Key1"), Some("Value1"));
    assert_eq!(section.get("Key2"), Some("Value2"));
    assert_eq!(section.get("#Key"), Some("Visible"));
    assert_eq!(section.entry_count(), 3);
}

#[test]
fn test_inline_comments() {
    let ini: IniFile = IniFile::from_str("[Test]\nCost=1000 ; credits\nName=hello\n");

    let section: &IniSection = ini.section("Test").unwrap();
    assert_eq!(section.get("Cost"), Some("1000"));
    assert_eq!(section.get("Name"), Some("hello"));
}

#[test]
fn test_get_i32() {
    let ini: IniFile =
        IniFile::from_str("[Stats]\nCost=1000\nDamage=-50\nName=tank\nBadHex=$junk\n");

    let section: &IniSection = ini.section("Stats").unwrap();
    assert_eq!(section.get_i32("Cost"), Some(1000));
    assert_eq!(section.get_i32("Damage"), Some(-50));
    assert_eq!(section.get_i32("Name"), Some(0)); // C atoi prefix: no digits -> 0
    assert_eq!(section.get_i32("BadHex"), None); // `%x` converted nothing
    assert_eq!(section.get_i32("Missing"), None); // Key doesn't exist
}

#[test]
fn test_get_f32() {
    let ini: IniFile = IniFile::from_str("[Stats]\nSpeed=5.5\nROF=0.1\n");

    let section: &IniSection = ini.section("Stats").unwrap();
    let speed: f32 = section.get_f32("Speed").unwrap();
    assert!((speed - 5.5).abs() < f32::EPSILON);
    let rof: f32 = section.get_f32("ROF").unwrap();
    assert!((rof - 0.1).abs() < 0.001);
}

#[test]
fn test_get_light_f32_stops_before_comma() {
    let ini: IniFile =
        IniFile::from_str("[Light]\nGood=0.25\nCommaDecimal=0,01\nSigned=-0.5\nBad=abc\n");

    let section: &IniSection = ini.section("Light").unwrap();
    assert!((section.get_light_f32("Good").unwrap() - 0.25).abs() < 0.001);
    assert_eq!(section.get_light_f32("CommaDecimal"), Some(0.0));
    assert!((section.get_light_f32("Signed").unwrap() + 0.5).abs() < 0.001);
    assert_eq!(section.get_light_f32("Bad"), Some(0.0));
    assert_eq!(section.get_light_f32("Missing"), None);
}

#[test]
fn test_get_bool() {
    let ini: IniFile = IniFile::from_str(
        "[Flags]\nDoubleOwned=yes\nCloakable=no\nActive=true\nDebug=false\nBit=1\nOff=0\n",
    );

    let section: &IniSection = ini.section("Flags").unwrap();
    assert_eq!(section.get_bool("DoubleOwned"), Some(true));
    assert_eq!(section.get_bool("Cloakable"), Some(false));
    assert_eq!(section.get_bool("Active"), Some(true));
    assert_eq!(section.get_bool("Debug"), Some(false));
    assert_eq!(section.get_bool("Bit"), Some(true));
    assert_eq!(section.get_bool("Off"), Some(false));
}

#[test]
fn test_get_list() {
    let ini: IniFile = IniFile::from_str("[Build]\nPrereq=GAWEAP,RADAR,TECH\nEmpty=\n");

    let section: &IniSection = ini.section("Build").unwrap();

    let prereq: Vec<&str> = section.get_list("Prereq").unwrap();
    assert_eq!(prereq, vec!["GAWEAP", "RADAR", "TECH"]);

    assert!(section.get_list("Empty").is_none());
    assert!(section.get_list("Missing").is_none());
}

#[test]
fn duplicate_nonempty_section_bodies_are_retained_in_source_order() {
    let text: &str = "\
[General]
Key1=First
Key2=Original

[General]
Key2=Override
Key3=New
";
    let ini: IniFile = IniFile::from_str(text);

    assert_eq!(ini.section_count(), 2);

    assert_eq!(ini.section_names(), vec!["General", "General"]);
    assert_eq!(ini.sections[0].get("Key1"), Some("First"));
    assert_eq!(ini.sections[1].get("Key3"), Some("New"));
}

#[test]
fn duplicate_key_compatibility_lookup_keeps_first_definition() {
    let ini = IniFile::from_str("[General]\nBuildSpeed=.7\nBuildSpeed=.58\n");
    assert_eq!(
        ini.section("General").unwrap().get("BuildSpeed"),
        Some(".7")
    );
}

#[test]
fn semicolon_truncates_before_equals_and_empty_entries_are_omitted() {
    let ini = IniFile::from_str(
        "[S]\nIgnored;Key=Value\n;Comment=Value\n=NoKey\nNoValue=\nGood=Yes;Comment\n",
    );
    let section = ini.section("S").unwrap();
    assert_eq!(section.entry_count(), 1);
    assert_eq!(section.get("Good"), Some("Yes"));
}

#[test]
fn test_section_names_order() {
    let ini: IniFile =
        IniFile::from_str("[Zebra]\nKey=Z\n[Alpha]\nKey=A\n[Middle]\nKey=M\n");

    let names: Vec<&str> = ini.section_names();
    assert_eq!(names, vec!["Zebra", "Alpha", "Middle"]);
}

#[test]
fn test_from_bytes() {
    let data: &[u8] = b"[Test]\nLatin=\xE9\nControl=\x80\n";
    let ini: IniFile = IniFile::from_bytes(data).expect("all byte values are accepted");
    assert_eq!(ini.section("Test").unwrap().get("Latin"), Some("\u{e9}"));
    assert_eq!(ini.section("Test").unwrap().get("Control"), Some("\u{80}"));
}

#[test]
fn read_line_removes_embedded_carriage_returns() {
    let ini = IniFile::from_str("[S]\nK\re\ry=V\ra\rl\rue\n");

    assert_eq!(ini.section("S").unwrap().get("Key"), Some("Value"));
}

#[test]
fn nul_terminates_the_visible_line_but_not_the_physical_read() {
    let ini = IniFile::from_str("[S]\nGood=Yes\0Injected=No\nAfter=Seen\n");
    let section = ini.section("S").unwrap();

    assert_eq!(section.get("Good"), Some("Yes"));
    assert!(section.get("Injected").is_none());
    assert_eq!(section.get("After"), Some("Seen"));
}

#[test]
fn overlong_physical_line_discards_everything_after_511_bytes() {
    let text = format!("[S]\nA={}Injected=Yes\n", "x".repeat(509));
    let ini = IniFile::from_str(&text);
    let section = ini.section("S").unwrap();

    assert_eq!(section.get("A").unwrap().len(), 509);
    assert!(section.get("Injected").is_none());
}

#[test]
fn test_get_values_zero_indexed() {
    let ini: IniFile = IniFile::from_str("[Types]\n0=E1\n1=E2\n2=ENGINEER\n3=FLAKT\n");
    let section: &IniSection = ini.section("Types").unwrap();
    let values: Vec<&str> = section.get_values();
    assert_eq!(values, vec!["E1", "E2", "ENGINEER", "FLAKT"]);
}

#[test]
fn test_get_values_one_indexed() {
    // Active retail RULESMD uses 1-indexed type registries in this family.
    let ini: IniFile = IniFile::from_str("[InfantryTypes]\n1=E1\n2=E2\n3=SHK\n");
    let section: &IniSection = ini.section("InfantryTypes").unwrap();
    let values: Vec<&str> = section.get_values();
    assert_eq!(values, vec!["E1", "E2", "SHK"]);
}

#[test]
fn test_get_values_with_numeric_gaps() {
    let ini: IniFile =
        IniFile::from_str("[VehicleTypes]\n36=CMIN\n1=HTNK\n40=HARV\n2=MTNK\n5=SMIN\n");
    let section: &IniSection = ini.section("VehicleTypes").unwrap();
    let values: Vec<&str> = section.get_values();
    assert_eq!(values, vec!["CMIN", "HTNK", "HARV", "MTNK", "SMIN"]);
}

#[test]
fn test_get_values_reads_named_entries_too() {
    let ini: IniFile = IniFile::from_str("[Empty]\nName=Test\n");
    let section: &IniSection = ini.section("Empty").unwrap();
    let values: Vec<&str> = section.get_values();
    assert_eq!(values, vec!["Test"]);
}

#[test]
fn test_whitespace_handling() {
    let ini: IniFile = IniFile::from_str("[Test]\n  Key  =  Value  \n");

    let section: &IniSection = ini.section("Test").unwrap();
    assert_eq!(section.get("Key"), Some("Value"));
}

#[test]
fn test_get_percent() {
    let ini: IniFile = IniFile::from_str(
        "[AudioVisual]\nConditionRed=25%\nConditionYellow=50%\nBare=0.75\nBad=abc\n",
    );
    let section: &IniSection = ini.section("AudioVisual").unwrap();
    let red: f32 = section.get_percent("ConditionRed").unwrap();
    assert!((red - 0.25).abs() < f32::EPSILON);
    let yellow: f32 = section.get_percent("ConditionYellow").unwrap();
    assert!((yellow - 0.50).abs() < f32::EPSILON);
    // Bare float without % suffix works too.
    let bare: f32 = section.get_percent("Bare").unwrap();
    assert!((bare - 0.75).abs() < f32::EPSILON);
    // Non-numeric returns None.
    assert_eq!(section.get_percent("Bad"), Some(0.0));
    assert!(section.get_percent("Missing").is_none());
}

/// RC-1: map rules overrides merge only into sections the rules already
/// declare, with last-definition-wins per key; map-only sections never
/// allocate.
#[test]
fn map_overrides_merge_rules_but_ignore_unreferenced_map_sections() {
    let mut rules = IniFile::from_str(
        "[General]\nBuildSpeed=.7\nFlightLevel=1500\n[CombatDamage]\nC4Delay=.03\n",
    );
    let map = IniFile::from_str(
        "[General]\nBuildSpeed=2\n[CombatDamage]\nC4Delay=.06\n\
         [Basic]\nName=TestMap\n[Waypoints]\n0=45035\n",
    );
    let applied = rules.merge_rules_overrides(&map);
    assert_eq!(applied, 2);
    assert_eq!(
        rules.section("General").unwrap().get("BuildSpeed"),
        Some("2")
    );
    assert_eq!(
        rules.section("General").unwrap().get("FlightLevel"),
        Some("1500")
    );
    assert_eq!(
        rules.section("CombatDamage").unwrap().get("C4Delay"),
        Some(".06")
    );
    assert!(
        rules.section("Basic").is_none(),
        "map-only sections must not allocate"
    );
    assert!(rules.section("Waypoints").is_none());
}

/// Native type-registry passes find-or-allocate every listed value from each
/// later rules layer, preserving entry order independently of the key text.
#[test]
fn map_overrides_union_type_registries_by_value() {
    let mut rules = IniFile::from_str("[VehicleTypes]\n0=MTNK\n[Animations]\n0=RING1\n");
    let map = IniFile::from_str("[VehicleTypes]\n0=EVILTANK\n[Animations]\n0=EVILANIM\n");
    let applied = rules.merge_rules_overrides(&map);
    assert_eq!(applied, 2);
    assert_eq!(
        rules.section("VehicleTypes").unwrap().get_values(),
        vec!["MTNK", "EVILTANK"]
    );
    assert_eq!(
        rules.section("Animations").unwrap().get_values(),
        vec!["RING1", "EVILANIM"]
    );
}

fn process_rules_passes(root: &str, later: &str) -> ProcessedRulesLayers {
    let mut layers = RulesLayerStack::new(IniFile::from_str(root));
    layers.push(RulesLayerKind::Scenario, IniFile::from_str(later));
    layers.process()
}

#[test]
fn later_malformed_weapon_bool_preserves_current_field_default() {
    use crate::rules::weapon_type::WeaponType;

    let processed = process_rules_passes(
        "[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nRevealOnFire=yes\n",
        "[Gun]\nRevealOnFire=maybe\n",
    );
    let section = processed.ini().section("Gun").expect("allocated Gun body");

    assert_eq!(section.get("RevealOnFire"), Some("maybe"));
    assert!(WeaponType::from_ini_section("Gun", section).reveal_on_fire);
}

#[test]
fn later_allocated_type_does_not_read_earlier_orphan_body() {
    let processed = process_rules_passes(
        "[LATE]\nStrength=900\nCost=700\n",
        "[VehicleTypes]\n0=LATE\n",
    );
    let late = processed.ini().section("LATE").expect("allocated body");
    assert_eq!(late.get("Strength"), None);
    assert_eq!(late.get("Cost"), None);
}

#[test]
fn existing_type_keeps_prior_fields_and_applies_current_body() {
    let processed = process_rules_passes(
        "[VehicleTypes]\n0=EARLY\n[EARLY]\nStrength=100\nCost=700\n",
        "[EARLY]\nStrength=250\n",
    );
    let early = processed.ini().section("EARLY").expect("EARLY body");
    assert_eq!(early.get("Strength"), Some("250"));
    assert_eq!(early.get("Cost"), Some("700"));
}

#[test]
fn tiberium_pass_reuses_numeric_slot_and_ignores_replacement_identity() {
    let processed = process_rules_passes(
        "[Tiberiums]\n0=Riparius\n[Riparius]\nImage=1\nValue=25\n",
        "[Tiberiums]\n0=Cruentus\n[Riparius]\nImage=4\n[Cruentus]\nImage=2\n",
    );

    assert_eq!(
        processed.ini().section("Tiberiums").unwrap().get_values(),
        vec!["Riparius"]
    );
    let riparius = processed.ini().section("Riparius").unwrap();
    assert_eq!(riparius.get("Image"), Some("4"));
    assert_eq!(riparius.get("Value"), Some("25"));
    assert!(processed.ini().section("Cruentus").is_some());
}

#[test]
fn tiberium_out_of_range_slot_appends_one_live_type() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[Tiberiums]\n7=Riparius\n0=Cruentus\n[Riparius]\nImage=1\n[Cruentus]\nImage=2\n",
    ))
    .process();

    assert_eq!(
        processed.ini().section("Tiberiums").unwrap().get_values(),
        vec!["Riparius"]
    );
    assert_eq!(
        processed
            .ini()
            .section("Riparius")
            .and_then(|section| section.get("Image")),
        Some("1")
    );
}

#[test]
fn new_type_reads_its_same_pass_body() {
    let processed = process_rules_passes(
        "[VehicleTypes]\n0=EARLY\n[EARLY]\nStrength=100\n",
        "[VehicleTypes]\n0=LATE\n[LATE]\nStrength=250\n",
    );
    assert_eq!(
        processed
            .ini()
            .section("LATE")
            .and_then(|section| section.get("Strength")),
        Some("250")
    );
}

#[test]
fn ordered_pass_registry_union_is_case_insensitive_by_value() {
    let processed = process_rules_passes(
        "[VehicleTypes]\nFirst=MTNK\n",
        "[VehicleTypes]\n0=mtnk\nAgain=HTNK\n",
    );
    assert_eq!(
        processed
            .ini()
            .section("VehicleTypes")
            .unwrap()
            .get_values(),
        vec!["MTNK", "HTNK"]
    );
}

#[test]
fn later_pass_missing_scalar_key_preserves_live_value() {
    let processed = process_rules_passes(
        "[General]\nBuildSpeed=.7\nFlightLevel=1500\n",
        "[General]\nBuildSpeed=.58\n",
    );
    let general = processed.ini().section("General").unwrap();
    assert_eq!(general.get("BuildSpeed"), Some(".58"));
    assert_eq!(general.get("FlightLevel"), Some("1500"));
}

#[test]
fn later_general_section_without_damage_fire_types_preserves_live_list() {
    let processed = process_rules_passes(
        "[General]\nDamageFireTypes=FIRE01,FIRE02\nBuildSpeed=.7\n",
        "[General]\nBuildSpeed=.58\n",
    );
    let general = processed.ini().section("General").unwrap();
    assert_eq!(general.get("DamageFireTypes"), Some("FIRE01,FIRE02"));
    assert_eq!(general.get("BuildSpeed"), Some(".58"));
}

#[test]
fn general_prerequisite_groups_are_lookup_only() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[General]\nPrerequisitePower=MAPPOWR\n[MAPPOWR]\nStrength=750\n",
    ))
    .process();

    assert!(
        processed
            .ini()
            .section("BuildingTypes")
            .unwrap()
            .get_values()
            .is_empty()
    );
    assert_eq!(
        processed
            .ini()
            .section("General")
            .unwrap()
            .get("PrerequisitePower"),
        Some("")
    );
}

#[test]
fn general_prerequisite_groups_keep_only_registered_buildings() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[BuildingTypes]\n0=GAPOWR\n[General]\nPrerequisitePower=gapowr,MISSING\n",
    ))
    .process();

    assert_eq!(
        processed
            .ini()
            .section("General")
            .unwrap()
            .get("PrerequisitePower"),
        Some("GAPOWR")
    );
}

#[test]
fn prerequisite_proc_alternate_allocates_unit_before_same_pass_body_sweep() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[General]\nPrerequisiteProcAlternate=SMIN\n[SMIN]\nStrength=2000\n",
    ))
    .process();

    assert_eq!(
        processed
            .ini()
            .section("VehicleTypes")
            .unwrap()
            .get_values(),
        vec!["SMIN"]
    );
    assert_eq!(
        processed
            .ini()
            .section("SMIN")
            .and_then(|section| section.get("Strength")),
        Some("2000")
    );
}

#[test]
fn barrel_particle_allocates_particle_system_before_same_pass_body_sweep() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[General]\nBarrelParticle=BarrelSys\n[BarrelSys]\nHoldsWhat=SmokePart\n[SmokePart]\nDamage=5\n",
    ))
    .process();

    assert_eq!(
        processed
            .ini()
            .section("ParticleSystems")
            .unwrap()
            .get_values(),
        vec!["BarrelSys"]
    );
    assert_eq!(
        processed.ini().section("Particles").unwrap().get_values(),
        vec!["SmokePart"]
    );
    assert_eq!(
        processed
            .ini()
            .section("BarrelSys")
            .and_then(|section| section.get("HoldsWhat")),
        Some("SmokePart")
    );
    assert_eq!(
        processed
            .ini()
            .section("SmokePart")
            .and_then(|section| section.get("Damage")),
        None,
        "HoldsWhat allocates the particle after the particle body sweep"
    );
}

#[test]
fn special_weapons_created_warhead_reads_same_pass_body() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[SpecialWeapons]\nMutateWarhead=FreshMutate\n\
         [FreshMutate]\nCellSpread=3\nVerses=100%,80%\n",
    ))
    .process();

    assert_eq!(
        processed
            .ini()
            .section("FreshMutate")
            .and_then(|section| section.get("CellSpread")),
        Some("3")
    );
}

#[test]
fn special_weapons_created_projectile_waits_for_next_pass_body_sweep() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[SpecialWeapons]\nNukeProjectile=FreshNuke\n[FreshNuke]\nImage=NUKE\n",
    ))
    .process();

    assert_eq!(
        processed
            .ini()
            .section("FreshNuke")
            .and_then(|section| section.get("Image")),
        None
    );
}

#[test]
fn combat_damage_allocates_late_smudge_and_animation_references() {
    let processed = RulesLayerStack::new(IniFile::from_str(
        "[CombatDamage]\n\
         Scorches=BurnA,BurnB\n\
         Scorches1=BurnC\n\
         Scorches2=BurnD\n\
         Scorches3=BurnE\n\
         Scorches4=BurnF\n\
         SplashList=SplashA,SplashB\n\
         DrainAnimationType=DrainAnim\n\
         ControlledAnimationType=MindAnim\n\
         PermaControlledAnimationType=MindAnimR\n\
         [BurnA]\nWidth=3\n",
    ))
    .process();

    assert_eq!(
        processed.ini().section("SmudgeTypes").unwrap().get_values(),
        vec!["BurnA", "BurnB", "BurnC", "BurnD", "BurnE", "BurnF"]
    );
    assert_eq!(
        processed.ini().section("Animations").unwrap().get_values(),
        vec!["SplashA", "SplashB", "DrainAnim", "MindAnim", "MindAnimR"]
    );
    assert_eq!(
        processed
            .ini()
            .section("BurnA")
            .and_then(|section| section.get("Width")),
        None,
        "late CombatDamage allocations wait for the next type-data pass"
    );
}

#[test]
fn rules_hash_preserves_pass_boundaries() {
    let single = RulesLayerStack::new(IniFile::from_str("[General]\nBuildSpeed=.58\n"));
    let mut layered = RulesLayerStack::new(IniFile::from_str("[General]\nBuildSpeed=.7\n"));
    layered.push(
        RulesLayerKind::LangRule,
        IniFile::from_str("[General]\nBuildSpeed=.58\n"),
    );
    assert_eq!(
        single
            .process()
            .ini()
            .section("General")
            .unwrap()
            .get("BuildSpeed"),
        layered
            .process()
            .ini()
            .section("General")
            .unwrap()
            .get("BuildSpeed")
    );
    assert_ne!(single.content_hash(), layered.content_hash());
}

#[test]
fn projectiles_and_sides_are_not_explicit_type_registries() {
    let processed = process_rules_passes(
        "[Projectiles]\n0=KEEP\n[Sides]\nGDI=Americans\n",
        "[Projectiles]\n0=REPLACE\n[Sides]\nGDI=French\n",
    );
    assert_eq!(
        processed.ini().section("Projectiles").unwrap().get("0"),
        Some("REPLACE")
    );
    assert_eq!(
        processed.ini().section("Sides").unwrap().get("GDI"),
        Some("French")
    );
}

/// Sections not proven to be native registries keep ordinary overlay behavior.
#[test]
fn existing_unlisted_numbered_section_receives_ordinary_overlay() {
    let mut rules = IniFile::from_str("[FutureTypes]\n0=KEEP\n");
    let map = IniFile::from_str("[FutureTypes]\n0=EVIL\n");
    assert_eq!(rules.merge_rules_overrides(&map), 1);
    assert_eq!(rules.section("FutureTypes").unwrap().get("0"), Some("EVIL"));
}

/// RC-1: a map with no rules-shaped sections is a byte-level no-op.
#[test]
fn map_overrides_no_op_without_rules_shaped_sections() {
    let mut rules = IniFile::from_str("[General]\nBuildSpeed=.7\n");
    let map = IniFile::from_str("[Basic]\nName=Clean\n[IsoMapPack5]\n1=AAAA\n");
    assert_eq!(rules.merge_rules_overrides(&map), 0);
    assert_eq!(
        rules.section("General").unwrap().get("BuildSpeed"),
        Some(".7")
    );
}

/// Registry processing uses every entry value, including entries whose keys
/// are not numeric.
#[test]
fn map_registry_pass_uses_every_entry_in_source_order() {
    let mut rules =
        IniFile::from_str("[Particles]\n30=FireStream\n[ParticleSystems]\n10=GasCloudSys\n");
    let map = IniFile::from_str(
        "[Particles]\n30=EvilFire\nName=oops\n[ParticleSystems]\n10=EvilSys\nStray=1\n",
    );
    assert_eq!(rules.merge_rules_overrides(&map), 4);
    assert_eq!(
        rules.section("Particles").unwrap().get_values(),
        vec!["FireStream", "EvilFire", "oops"]
    );
    assert_eq!(
        rules.section("ParticleSystems").unwrap().get_values(),
        vec!["GasCloudSys", "EvilSys", "1"]
    );
}

/// Empty-value semantics: a map line `BuildSpeed=` with no value is
/// "key absent" in the original (its INI writer gates out empty/NULL values
/// and never stores an entry, so the readers keep the value already on the
/// field). It must NOT clobber the value from an earlier rules pass — clobbering it
/// with `""` would reset the field to the hardcoded Rust default at parse time.
#[test]
fn map_overrides_skip_empty_valued_keys() {
    let mut rules = IniFile::from_str(
        "[General]\nBuildSpeed=.7\nFlightLevel=1500\n[CombatDamage]\nC4Delay=.03\n",
    );
    let map = IniFile::from_str("[General]\nBuildSpeed=\n[CombatDamage]\nC4Delay=.06\n");
    let applied = rules.merge_rules_overrides(&map);
    assert_eq!(
        applied, 1,
        "the empty BuildSpeed= must not count as an override"
    );
    assert_eq!(
        rules.section("General").unwrap().get("BuildSpeed"),
        Some(".7"),
        "empty map value must leave the merged value intact"
    );
    assert_eq!(
        rules.section("CombatDamage").unwrap().get("C4Delay"),
        Some(".06")
    );
}

/// `[Colors]` is a find-or-create registry: an existing case-insensitive
/// identity keeps its first HSV value, while a new name allocates.
#[test]
fn map_colors_keep_existing_identity_and_allocate_new_name() {
    let mut rules = IniFile::from_str("[Colors]\nGold=42,252,252\nDarkRed=0,151,239\n");
    let map = IniFile::from_str("[Colors]\ngold=1,2,3\nNeonPink=12,200,255\n");
    let applied = rules.merge_rules_overrides(&map);
    assert_eq!(applied, 1);
    assert_eq!(
        rules.section("Colors").unwrap().get("Gold"),
        Some("42,252,252")
    );
    assert_eq!(
        rules.section("Colors").unwrap().get("NeonPink"),
        Some("12,200,255")
    );
}

/// `content_hash` is deterministic and sensitive to every value — a scalar
/// override changes it (the gap that left a registry-only rules hash blind to
/// map value overrides), while comment/whitespace-only differences do not.
#[test]
fn content_hash_is_deterministic_and_value_sensitive() {
    let a = IniFile::from_str("[General]\nBuildSpeed=.7\nFlightLevel=1500\n");
    // Same content parsed twice → identical hash (no HashMap-order drift).
    let a2 = IniFile::from_str("[General]\nBuildSpeed=.7\nFlightLevel=1500\n");
    assert_eq!(a.content_hash(), a2.content_hash());

    // One scalar value differs → hash differs.
    let b = IniFile::from_str("[General]\nBuildSpeed=.58\nFlightLevel=1500\n");
    assert_ne!(a.content_hash(), b.content_hash());

    // Comments and surrounding whitespace are stripped at parse → no effect.
    let c = IniFile::from_str("; header\n[General]\nBuildSpeed = .7   ; speed\nFlightLevel=1500\n");
    assert_eq!(a.content_hash(), c.content_hash());
}
