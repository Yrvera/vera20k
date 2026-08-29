//! Native crate and `[Powerups]` rules authority.
//!
//! Crate rules are live typed state across each `RulesClass::Process` pass.
//! Powerups are a separate executable-global table whose mixed missing-key
//! fallback cannot be reconstructed from a flattened INI projection.

use crate::rules::ini_parser::{IniFile, IniSection};
use crate::rules::ini_value::{atoi_lenient, parse_read_double};
use crate::util::native_x87::NativeF64Bits;

pub const POWERUP_COUNT: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CrateEffect {
    Money = 0,
    Unit = 1,
    HealBase = 2,
    Cloak = 3,
    Explosion = 4,
    Napalm = 5,
    Squad = 6,
    Darkness = 7,
    Reveal = 8,
    Armor = 9,
    Speed = 10,
    Firepower = 11,
    Icbm = 12,
    Invulnerability = 13,
    Veteran = 14,
    IonStorm = 15,
    Gas = 16,
    Tiberium = 17,
    Pod = 18,
}

impl CrateEffect {
    pub const ALL: [Self; POWERUP_COUNT] = [
        Self::Money,
        Self::Unit,
        Self::HealBase,
        Self::Cloak,
        Self::Explosion,
        Self::Napalm,
        Self::Squad,
        Self::Darkness,
        Self::Reveal,
        Self::Armor,
        Self::Speed,
        Self::Firepower,
        Self::Icbm,
        Self::Invulnerability,
        Self::Veteran,
        Self::IonStorm,
        Self::Gas,
        Self::Tiberium,
        Self::Pod,
    ];

    pub const NAMES: [&'static str; POWERUP_COUNT] = [
        "Money",
        "Unit",
        "HealBase",
        "Cloak",
        "Explosion",
        "Napalm",
        "Squad",
        "Darkness",
        "Reveal",
        "Armor",
        "Speed",
        "Firepower",
        "ICBM",
        "Invulnerability",
        "Veteran",
        "IonStorm",
        "Gas",
        "Tiberium",
        "Pod",
    ];

    pub fn from_name(value: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .position(|name| name.eq_ignore_ascii_case(value.trim()))
            .map(|index| Self::ALL[index])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PowerupEntry {
    pub weight: i32,
    pub animation: Option<String>,
    pub water_allowed: bool,
    pub data: NativeF64Bits,
}

impl PowerupEntry {
    fn baseline(index: usize) -> Self {
        const WEIGHTS: [i32; POWERUP_COUNT] = [
            50, 20, 1, 3, 5, 5, 20, 1, 1, 10, 10, 10, 1, 3, 1, 1, 1, 1, 1,
        ];
        Self {
            weight: WEIGHTS[index],
            animation: None,
            water_allowed: false,
            data: NativeF64Bits::POSITIVE_ZERO,
        }
    }
}

pub fn executable_powerup_baseline() -> [PowerupEntry; POWERUP_COUNT] {
    std::array::from_fn(PowerupEntry::baseline)
}

/// Advance the executable-global Powerups table through one original INI pass.
///
/// gamemd-derived: `RulesClass__ReadPowerups @ 0x00673E80`. A live section
/// visits all nineteen names with `"0,NONE,0"`; comma tokenization is CRT
/// `strtok`, so empty fields collapse and shift later values left.
pub(crate) fn apply_powerups_pass(
    table: &mut [PowerupEntry; POWERUP_COUNT],
    pass: &IniFile,
    live_animations: &[String],
) {
    let Some(section) = pass.section("Powerups") else {
        return;
    };
    for (index, name) in CrateEffect::NAMES.iter().enumerate() {
        let raw = section.read_string(name, "0,NONE,0", 128);
        let tokens: Vec<&str> = raw
            .split(',')
            .map(trim_ascii_controls)
            .filter(|token| !token.is_empty())
            .collect();
        let entry = &mut table[index];
        if let Some(token) = tokens.first() {
            entry.weight = atoi_lenient(token);
        }
        if let Some(token) = tokens.get(1) {
            entry.animation = live_animations
                .iter()
                .find(|candidate| candidate.eq_ignore_ascii_case(token))
                .cloned();
        }
        if let Some(token) = tokens.get(2) {
            if token.eq_ignore_ascii_case("yes") {
                entry.water_allowed = true;
            } else if token.eq_ignore_ascii_case("no") {
                entry.water_allowed = false;
            }
        }
        if let Some(token) = tokens.get(3) {
            entry.data = NativeF64Bits::from_bits(parse_direct_atof(token).to_bits());
        }
    }
}

/// Direct CRT-atof shape used only by ReadPowerups. This deliberately differs
/// from CCINI ReadDouble's `%f`-to-f32 widening path.
fn parse_direct_atof(raw: &str) -> f64 {
    let trimmed = trim_ascii_controls(raw);
    let percent = trimmed.as_bytes().contains(&b'%');
    let bytes = trimmed.as_bytes();
    let mut end = 0usize;
    if bytes
        .get(end)
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        end += 1;
    }
    let mut digits = 0usize;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
        digits += 1;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if bytes
        .get(end)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        let exponent_start = end;
        end += 1;
        if bytes
            .get(end)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            end += 1;
        }
        let exponent_digits = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if exponent_digits == end {
            end = exponent_start;
        }
    }
    let parsed = trimmed[..end].parse::<f64>().unwrap_or(0.0);
    if percent { parsed * 0.01_f64 } else { parsed }
}

fn trim_ascii_controls(value: &str) -> &str {
    value.trim_matches(|character: char| character.is_ascii() && character <= ' ')
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrateRules {
    pub free_mcv: bool,
    pub wood_crate_img: Option<String>,
    pub crate_img: Option<String>,
    pub water_crate_img: Option<String>,
    pub heal_crate_sound: Option<String>,
    pub solo_money: i32,
    pub unit_crate_type: Option<String>,
    pub silver_crate: CrateEffect,
    pub wood_crate: CrateEffect,
    pub water_crate: CrateEffect,
    pub minimum: i32,
    pub maximum: i32,
    pub regen: NativeF64Bits,
    pub radius_leptons: i32,
    pub powerups: [PowerupEntry; POWERUP_COUNT],
    pub crate_money_sound: Option<String>,
    pub crate_reveal_sound: Option<String>,
    pub crate_fire_sound: Option<String>,
    pub crate_armour_sound: Option<String>,
    pub crate_speed_sound: Option<String>,
    pub crate_unit_sound: Option<String>,
    pub crate_promote_sound: Option<String>,
}

impl Default for CrateRules {
    fn default() -> Self {
        Self {
            free_mcv: false,
            wood_crate_img: None,
            crate_img: None,
            water_crate_img: None,
            heal_crate_sound: None,
            solo_money: 2000,
            unit_crate_type: None,
            silver_crate: CrateEffect::HealBase,
            wood_crate: CrateEffect::Money,
            water_crate: CrateEffect::Money,
            minimum: 1,
            maximum: 255,
            regen: NativeF64Bits::from_bits(0x4024_0000_0000_0000),
            radius_leptons: 640,
            powerups: executable_powerup_baseline(),
            crate_money_sound: None,
            crate_reveal_sound: None,
            crate_fire_sound: None,
            crate_armour_sound: None,
            crate_speed_sound: None,
            crate_unit_sound: None,
            crate_promote_sound: None,
        }
    }
}

impl CrateRules {
    pub(crate) fn apply_pass(&mut self, pass: &IniFile) {
        if let Some(section) = pass.section("CrateRules") {
            self.free_mcv = section.read_bool("FreeMCV", self.free_mcv);
            self.wood_crate_img = read_type_name(section, "WoodCrateImg", &self.wood_crate_img);
            self.crate_img = read_type_name(section, "CrateImg", &self.crate_img);
            self.water_crate_img = read_type_name(section, "WaterCrateImg", &self.water_crate_img);
            self.heal_crate_sound =
                read_sound_name(section, "HealCrateSound", &self.heal_crate_sound);
            self.minimum = section.read_int("CrateMinimum", self.minimum);
            self.maximum = section.read_int("CrateMaximum", self.maximum);
            self.radius_leptons = read_crate_range(section, "CrateRadius", self.radius_leptons);
            self.regen = NativeF64Bits::from_bits(
                section
                    .read_double("CrateRegen", f64::from_bits(self.regen.bits()))
                    .to_bits(),
            );
            self.unit_crate_type = read_type_name(section, "UnitCrateType", &self.unit_crate_type);
            self.solo_money = section.read_int("SoloCrateMoney", self.solo_money);
            self.silver_crate = read_effect(section, "SilverCrate", self.silver_crate);
            self.wood_crate = read_effect(section, "WoodCrate", self.wood_crate);
            self.water_crate = read_effect(section, "WaterCrate", self.water_crate);
        }
        if let Some(section) = pass.section("AudioVisual") {
            self.crate_promote_sound =
                read_sound_name(section, "CratePromoteSound", &self.crate_promote_sound);
            self.crate_money_sound =
                read_sound_name(section, "CrateMoneySound", &self.crate_money_sound);
            self.crate_reveal_sound =
                read_sound_name(section, "CrateRevealSound", &self.crate_reveal_sound);
            self.crate_fire_sound =
                read_sound_name(section, "CrateFireSound", &self.crate_fire_sound);
            self.crate_armour_sound =
                read_sound_name(section, "CrateArmourSound", &self.crate_armour_sound);
            self.crate_speed_sound =
                read_sound_name(section, "CrateSpeedSound", &self.crate_speed_sound);
            self.crate_unit_sound =
                read_sound_name(section, "CrateUnitSound", &self.crate_unit_sound);
        }
    }
}

fn read_type_name(section: &IniSection, key: &str, current: &Option<String>) -> Option<String> {
    let Some(_) = section.get(key) else {
        return current.clone();
    };
    let value = section.read_string(key, "", 128);
    if value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("<none>") {
        None
    } else if value.is_empty() {
        current.clone()
    } else {
        Some(value.to_ascii_uppercase())
    }
}

fn read_sound_name(section: &IniSection, key: &str, current: &Option<String>) -> Option<String> {
    let Some(_) = section.get(key) else {
        return current.clone();
    };
    let value = section.read_string(key, "", 128);
    if value.is_empty()
        || value.eq_ignore_ascii_case("none")
        || value.eq_ignore_ascii_case("<none>")
    {
        None
    } else {
        Some(value)
    }
}

fn read_effect(section: &IniSection, key: &str, current: CrateEffect) -> CrateEffect {
    let Some(_) = section.get(key) else {
        return current;
    };
    CrateEffect::from_name(&section.read_string(key, "", 128)).unwrap_or(CrateEffect::Money)
}

/// `CCINIClass__ReadRange @ 0x00474620`, including the low-dword result of the
/// x87 integer-indefinite value for nonfinite/out-of-i64 conversion.
fn read_crate_range(section: &IniSection, key: &str, current: i32) -> i32 {
    let Some(raw) = section.get(key) else {
        return current;
    };
    let trimmed = trim_ascii_controls(raw);
    if trimmed
        .trim_start_matches(['+', '-'])
        .to_ascii_lowercase()
        .starts_with("nan")
    {
        return current;
    }
    let value = parse_read_double(raw);
    if value == -1.0 || value.is_nan() {
        return current;
    }
    let scaled = value * 256.0;
    if !scaled.is_finite()
        || scaled < -9_223_372_036_854_775_808.0
        || scaled >= 9_223_372_036_854_775_808.0
    {
        0
    } else {
        scaled.trunc() as i64 as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::{RulesLayerKind, RulesLayerStack};
    use crate::rules::overlay_types::OverlayTypeRegistry;
    use crate::rules::ruleset::RuleSet;

    #[test]
    fn constructor_matches_rulesclass_and_executable_powerup_globals() {
        let rules = CrateRules::default();
        assert!(!rules.free_mcv);
        assert_eq!(rules.minimum, 1);
        assert_eq!(rules.maximum, 255);
        assert_eq!(rules.solo_money, 2000);
        assert_eq!(rules.regen.bits(), 0x4024_0000_0000_0000);
        assert_eq!(rules.radius_leptons, 640);
        assert_eq!(rules.silver_crate, CrateEffect::HealBase);
        assert_eq!(
            rules.powerups.map(|entry| entry.weight),
            [
                50, 20, 1, 3, 5, 5, 20, 1, 1, 10, 10, 10, 1, 3, 1, 1, 1, 1, 1
            ]
        );
    }

    #[test]
    fn crate_rules_apply_signed_and_native_range_semantics() {
        let mut rules = CrateRules::default();
        rules.apply_pass(&IniFile::from_str(
            "[CrateRules]\nCrateMinimum=-9\nCrateMaximum=-3\nCrateRadius=-.5\nCrateRegen=3\nSoloCrateMoney=5000\nFreeMCV=yes\nCrateImg=CRATE\n",
        ));
        assert_eq!((rules.minimum, rules.maximum), (-9, -3));
        assert_eq!(rules.radius_leptons, -128);
        assert_eq!(rules.regen.bits(), 3.0_f64.to_bits());
        assert_eq!(rules.crate_img.as_deref(), Some("CRATE"));
        rules.apply_pass(&IniFile::from_str("[CrateRules]\nCrateRadius=-1\n"));
        assert_eq!(rules.radius_leptons, -128, "-1 sentinel retains prior");
        rules.apply_pass(&IniFile::from_str("[CrateRules]\nCrateRadius=nan\n"));
        assert_eq!(rules.radius_leptons, -128, "unordered NaN retains prior");
        rules.apply_pass(&IniFile::from_str("[CrateRules]\nCrateRadius=1e100\n"));
        assert_eq!(
            rules.radius_leptons, 0,
            "integer-indefinite low dword is zero"
        );
    }

    #[test]
    fn powerups_absent_preserves_and_present_uses_mixed_missing_key_fallback() {
        let mut table = executable_powerup_baseline();
        apply_powerups_pass(&mut table, &IniFile::from_str("[General]\nX=1\n"), &[]);
        assert_eq!(table[0].weight, 50);
        table[0].water_allowed = true;
        table[0].data = NativeF64Bits::from_bits(7.0_f64.to_bits());
        apply_powerups_pass(
            &mut table,
            &IniFile::from_str("[Powerups]\nUnknown=keeps-section-live\n"),
            &["NONE".to_string()],
        );
        assert_eq!(table[0].weight, 0);
        assert_eq!(table[0].animation.as_deref(), Some("NONE"));
        assert!(table[0].water_allowed);
        assert_eq!(table[0].data.bits(), 7.0_f64.to_bits());
    }

    #[test]
    fn powerup_strtok_shift_and_direct_f64_percent_are_exact() {
        let mut table = executable_powerup_baseline();
        apply_powerups_pass(
            &mut table,
            &IniFile::from_str("[Powerups]\nMoney=20,,yes,1.2%\n"),
            &["YES".to_string()],
        );
        assert_eq!(table[0].animation.as_deref(), Some("YES"));
        assert_eq!(
            table[0].data.bits(),
            0,
            "collapsed comma shifts the data away"
        );
        apply_powerups_pass(
            &mut table,
            &IniFile::from_str("[Powerups]\nMoney=20,MONEY,yes,1.2%\n"),
            &["MONEY".to_string()],
        );
        assert_eq!(table[0].data.bits(), (1.2_f64 * 0.01_f64).to_bits());
    }

    #[test]
    fn complete_retail_shaped_powerups_resolve_exact_arrays() {
        let ini = IniFile::from_str(
            "[Animations]\n0=MONEY\n1=HEALALL\n2=CLOAK\n3=SHROUDX\n4=REVEAL\n5=ARMOR\n6=SPEED\n7=FIREPOWR\n8=CHEMISLE\n9=VETERAN\n\
             [Powerups]\nArmor=10,ARMOR,yes,1.5\nFirepower=10,FIREPOWR,yes,2.0\nHealBase=10,HEALALL,yes\nMoney=20,MONEY,yes,2000\nReveal=10,REVEAL,yes\nSpeed=10,SPEED,yes,1.2\nVeteran=20,VETERAN,yes,1\nUnit=20,<none>,no\nInvulnerability=0,ARMOR,yes,1.0\nIonStorm=0,<none>,yes\nGas=0,<none>,yes,100\nTiberium=0,<none>,no\nPod=0,<none>,no\nCloak=0,CLOAK,yes\nDarkness=0,SHROUDX,yes\nExplosion=0,<none>,yes,500\nICBM=0,CHEMISLE,yes\nNapalm=0,<none>,no,600\nSquad=0,<none>,no\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("retail-shaped powerups");
        let powerups = &rules.crate_rules.powerups;
        assert_eq!(
            powerups.clone().map(|entry| entry.weight),
            [
                20, 20, 10, 0, 0, 0, 0, 0, 10, 10, 10, 10, 0, 0, 20, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            powerups.clone().map(|entry| entry.water_allowed),
            [
                true, false, true, true, true, false, false, true, true, true, true, true, true,
                true, true, true, true, false, false
            ]
        );
        assert_eq!(powerups[0].data.bits(), 2000.0_f64.to_bits());
        assert_eq!(powerups[9].data.bits(), 1.5_f64.to_bits());
        assert_eq!(powerups[10].data.bits(), 1.2_f64.to_bits());
        assert_eq!(powerups[11].data.bits(), 2.0_f64.to_bits());
        assert_eq!(powerups[0].animation.as_deref(), Some("MONEY"));
        assert_eq!(powerups[1].animation, None);
    }

    #[test]
    fn powerups_advance_per_original_pass_not_flattened_projection() {
        let mut layers = RulesLayerStack::new(IniFile::from_str(
            "[Animations]\n0=MONEY\n1=ARMOR\n[Powerups]\nMoney=20,MONEY,yes,2000\n",
        ));
        layers.push(
            RulesLayerKind::Scenario,
            IniFile::from_str("[Powerups]\nArmor=7,ARMOR,no,1.75\n"),
        );
        let rules = RuleSet::from_rules_layers(&layers).expect("typed passes");
        assert_eq!(rules.crate_rules.powerups[0].weight, 0);
        assert!(rules.crate_rules.powerups[0].water_allowed);
        assert_eq!(
            rules.crate_rules.powerups[0].data.bits(),
            2000.0_f64.to_bits()
        );
        assert_eq!(rules.crate_rules.powerups[9].weight, 7);
        assert_eq!(
            rules.crate_rules.powerups[9].data.bits(),
            1.75_f64.to_bits()
        );
    }

    #[test]
    fn crate_type_flags_share_existing_rule_owners() {
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=CRATE\n[CRATE]\nCrate=yes\nCrateTrigger=yes\n\
             [VehicleTypes]\n0=GOOD\n[GOOD]\nCrateGoodie=yes\nCarriesCrate=yes\n\
             [BuildingTypes]\n0=PROP\n[PROP]\nFoundation=1x1\nCrateBeneath=yes\nCrateBeneathIsMoney=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("object flags");
        let good = rules.object("GOOD").expect("unit");
        assert!(good.crate_goodie && good.carries_crate);
        let prop = rules.object("PROP").expect("building");
        assert!(prop.crate_beneath && prop.crate_beneath_is_money);
        let overlays = OverlayTypeRegistry::from_ini(&ini, None);
        let flags = overlays.flags_by_name("CRATE").expect("crate overlay");
        assert!(flags.crate_type && flags.crate_trigger);
    }
}
