//! Historical projections of the deterministic Rust hash stream.
//!
//! These policies preserve the existing provenance probes; they do not promise
//! snapshot loading compatibility or native parity for arbitrary versions.

#[derive(Clone, Copy)]
pub(super) enum HashSchema {
    Current,
    #[cfg(test)]
    Before(u16),
}

/// First hash schema containing each gated layout. A feature can select an
/// alternate encoding, not just append fields: retain both sides of its fold.
#[derive(Clone, Copy)]
#[repr(u16)]
pub(super) enum HashFeature {
    Lifecycle = 28,
    Mission = 29,
    MasterFrame = 43,
    EntityAnimation = 44,
    BuildingAnimOverlays = 45,
    TerminalScore = 46,
    PlayfieldAuthority = 47,
    TechnoPlayfield = 87,
    SensorDeposit = 88,
    RealCellBridgeFlags = 90,
    BaseDefenseResponse = 97,
    TechnoConstructor = 104,
    SparkDummyLevelSlope = 107,
    AlternateBaseCenter = 108,
    NavalBuildConst = 109,
    BasePlan = 110,
    BasePlanCenter = 111,
    HouseDeployLatches = 112,
    HouseUpdateActivation = 113,
    CrateAuthority = 114,
    WallRuntime = 115,
    DisguiseDetect = 117,
    HouseHarvesterNoOre = 132,
    HouseEva = 133,
    CreditIncome = 135,
    InfantryTerminal = 136,
    SustainedGapSight = 142,
    GapOperational = 144,
    BridgePublication = 145,
}

impl HashSchema {
    pub(super) const fn includes(self, _feature: HashFeature) -> bool {
        match self {
            Self::Current => true,
            #[cfg(test)]
            Self::Before(version) => (_feature as u16) < version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_policies_preserve_original_positional_masks() {
        // Frozen from the 25-argument calls before this refactor (main c1983f56).
        // Bit positions retain the original parameter order; expected masks
        // are the old call values, independent of the new version predicate.
        let features = [
            HashFeature::Lifecycle,
            HashFeature::Mission,
            HashFeature::MasterFrame,
            HashFeature::EntityAnimation,
            HashFeature::BuildingAnimOverlays,
            HashFeature::TerminalScore,
            HashFeature::PlayfieldAuthority,
            HashFeature::TechnoPlayfield,
            HashFeature::SensorDeposit,
            HashFeature::RealCellBridgeFlags,
            HashFeature::BaseDefenseResponse,
            HashFeature::TechnoConstructor,
            HashFeature::SparkDummyLevelSlope,
            HashFeature::AlternateBaseCenter,
            HashFeature::NavalBuildConst,
            HashFeature::BasePlan,
            HashFeature::BasePlanCenter,
            HashFeature::HouseDeployLatches,
            HashFeature::HouseUpdateActivation,
            HashFeature::CrateAuthority,
            HashFeature::WallRuntime,
            HashFeature::DisguiseDetect,
            HashFeature::HouseHarvesterNoOre,
            HashFeature::HouseEva,
            HashFeature::CreditIncome,
        ];
        let cases = [
            ("state_hash", HashSchema::Current, 0x01ffffffu32),
            (
                "state_hash_without_mission_v29",
                HashSchema::Before(29),
                0x00000001u32,
            ),
            (
                "state_hash_before_lifecycle_v28_and_mission_v29",
                HashSchema::Before(28),
                0x00000000u32,
            ),
            (
                "state_hash_without_spark_dummy_level_slope_v107",
                HashSchema::Before(107),
                0x00000fffu32,
            ),
            (
                "state_hash_without_naval_build_const_v109",
                HashSchema::Before(109),
                0x00003fffu32,
            ),
            (
                "state_hash_without_base_plan_v110",
                HashSchema::Before(110),
                0x00007fffu32,
            ),
            (
                "state_hash_without_base_plan_center_v111",
                HashSchema::Before(111),
                0x0000ffffu32,
            ),
            (
                "state_hash_without_house_deploy_latches_v112",
                HashSchema::Before(112),
                0x0001ffffu32,
            ),
            (
                "state_hash_without_house_update_activation_v113",
                HashSchema::Before(113),
                0x0003ffffu32,
            ),
            (
                "state_hash_without_crate_authority_v114",
                HashSchema::Before(114),
                0x0007ffffu32,
            ),
            (
                "state_hash_without_disguise_detect_v117",
                HashSchema::Before(117),
                0x001fffffu32,
            ),
            (
                "state_hash_without_house_harvester_no_ore_v132",
                HashSchema::Before(132),
                0x003fffffu32,
            ),
            (
                "state_hash_without_house_eva_v133",
                HashSchema::Before(133),
                0x007fffffu32,
            ),
            (
                "state_hash_without_credit_income_v135",
                HashSchema::Before(135),
                0x00ffffffu32,
            ),
            (
                "state_hash_without_wall_runtime_v115",
                HashSchema::Before(115),
                0x000fffffu32,
            ),
        ];
        for (name, schema, expected) in cases {
            let actual = features
                .iter()
                .enumerate()
                .fold(0u32, |mask, (bit, &feature)| {
                    mask | (u32::from(schema.includes(feature)) << bit)
                });
            assert_eq!(actual, expected, "{name}");
        }
    }
}
