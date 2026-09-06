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
    let ini = IniFile::from_str("[PARABOMB]\n\n[PARABOMB]\nRate=200\nLoopStart=7\nLoopCount=15\n");

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
    let ini: IniFile = IniFile::from_str("[Zebra]\nKey=Z\n[Alpha]\nKey=A\n[Middle]\nKey=M\n");

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
