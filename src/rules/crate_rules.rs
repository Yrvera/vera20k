//! Scenario-start crate rule authority.
//!
//! `RulesClass__ReadCrateRules @ 0x0066B900` runs once per ordered rules pass
//! after late global references have been allocated. Missing sections and keys
//! retain the already-live fields; native no-type sentinels resolve to null.

use crate::rules::ini_parser::{IniFile, is_native_none_type_name};
use crate::util::native_x87::NativeF64Bits;

/// The six `[CrateRules]` fields consumed by scenario-start scatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateRules {
    pub minimum: i32,
    pub maximum: i32,
    pub regen: NativeF64Bits,
    pub wood_crate_img: Option<String>,
    pub crate_img: Option<String>,
    pub water_crate_img: Option<String>,
}

impl Default for CrateRules {
    fn default() -> Self {
        // RulesClass constructor values verified in active gamemd.exe. Image
        // pointers begin null; stock rulesmd.ini fills them during Process.
        Self {
            minimum: 1,
            maximum: 255,
            regen: NativeF64Bits::from_bits(10.0_f64.to_bits()),
            wood_crate_img: None,
            crate_img: None,
            water_crate_img: None,
        }
    }
}

/// Live fields retained across successive `RulesClass::Process` passes.
#[derive(Debug, Clone, Default)]
pub(crate) struct CrateRulesAccumulator(CrateRules);

impl CrateRulesAccumulator {
    pub(crate) fn apply_pass(&mut self, ini: &IniFile) {
        let Some(section) = ini.section("CrateRules") else {
            return;
        };

        if section.get("CrateMinimum").is_some() {
            self.0.minimum = section.read_int("CrateMinimum", self.0.minimum);
        }
        if section.get("CrateMaximum").is_some() {
            self.0.maximum = section.read_int("CrateMaximum", self.0.maximum);
        }
        if section.get("CrateRegen").is_some() {
            self.0.regen = NativeF64Bits::from_bits(
                section
                    .read_double("CrateRegen", f64::from_bits(self.0.regen.bits()))
                    .to_bits(),
            );
        }

        for (key, target) in [
            ("WoodCrateImg", &mut self.0.wood_crate_img),
            ("CrateImg", &mut self.0.crate_img),
            ("WaterCrateImg", &mut self.0.water_crate_img),
        ] {
            if section.get(key).is_none() {
                continue;
            }
            // `RulesClass__ReadCrateRules @ 0x0066B900` supplies capacity
            // 0x80 to all three ReadString calls. Truncation therefore owns
            // both the retained identity and the earlier late allocation.
            let value = section.read_string(key, "", 0x80);
            *target = (!is_native_none_type_name(&value)).then(|| value.to_ascii_uppercase());
        }
    }

    pub(crate) fn finish(self) -> CrateRules {
        self.0
    }
}
