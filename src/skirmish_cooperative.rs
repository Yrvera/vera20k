//! Cooperative Skirmish campaign registry and front-end progress state.
//!
//! Retail data comes from `CoopCampMD.ini`. The app supplies the process-lifetime
//! Scenario RNG so campaign-map variant construction and country eligibility
//! retries advance the same stream as the surrounding Skirmish shell.

use crate::assets::asset_manager::AssetManager;
use crate::rules::ini_parser::{IniFile, IniSection};
use crate::sim::rng::SimRng;

pub const COOPERATIVE_CAMPAIGN_INI: &str = "CoopCampMD.ini";
const MAP_VARIANT_COUNT: usize = 3;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CooperativeError {
    #[error("CoopCampMD.ini was not found in the configured retail assets")]
    MissingCampaignIni,
    #[error("failed to parse CoopCampMD.ini from {source_name}: {message}")]
    InvalidCampaignIni {
        source_name: String,
        message: String,
    },
    #[error("campaign index {index} is outside the Cooperative registry (count {count})")]
    InvalidCampaignIndex { index: usize, count: usize },
    #[error(
        "Cooperative campaign '{campaign}' stage {stage} has {found} map variants; native progress construction requires three"
    )]
    InvalidMapVariantCount {
        campaign: String,
        stage: usize,
        found: usize,
    },
    #[error("the global multiplayer country roster is empty")]
    EmptyCountryRoster,
    #[error("the Cooperative {role} country list has no country present in the global roster")]
    NoEligibleCountries { role: &'static str },
    #[error("campaign index {index} has no preconstructed reserve progress record")]
    MissingReserveProgress { index: usize },
    #[error(
        "Cooperative progress selected map '{scenario}', but it is absent from the loadable Skirmish map registry"
    )]
    MissingScenarioMap { scenario: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooperativeAiDifficulty {
    Easy,
    Normal,
    Hard,
}

impl CooperativeAiDifficulty {
    fn from_ini(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("hard") => Self::Hard,
            Some("normal") => Self::Normal,
            Some("easy") | None => Self::Easy,
            // Native enum lookup retains the caller-provided Easy default when
            // the string is not a recognized difficulty name.
            Some(_) => Self::Easy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeStage {
    /// Exactly the first three source-order `MapN` tokens selected by
    /// `RandomRanged(0,2)`.
    pub map_variants: [String; MAP_VARIANT_COUNT],
    /// Raw `CampaignPlayerN` tokens in source order.
    pub player_countries: Vec<String>,
    /// Raw `CampaignEnemyN` tokens in source order.
    pub enemy_countries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeCampaign {
    /// Source section named by the corresponding `[Campaigns]` value.
    pub section: String,
    pub campaign_name: String,
    pub load_screen: String,
    /// Retail intentionally spells this key `Pallet`.
    pub load_screen_pallet: String,
    pub ai_difficulty: CooperativeAiDifficulty,
    pub stages: Vec<CooperativeStage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CooperativeRegistry {
    /// Campaigns in `[Campaigns]` source order. The index is CampaignType.
    pub campaigns: Vec<CooperativeCampaign>,
}

impl CooperativeRegistry {
    /// Parse the already loaded retail payload. A successful empty or missing
    /// `[Campaigns]` section yields a valid zero-campaign registry, matching the
    /// native lazy loader's successful-empty state.
    pub fn from_ini(ini: &IniFile) -> Result<Self, CooperativeError> {
        let Some(roster) = ini.section("Campaigns") else {
            return Ok(Self::default());
        };

        let mut campaigns = Vec::with_capacity(roster.entry_count());
        for roster_key in roster.keys() {
            let Some(section_name) = roster.get(roster_key) else {
                continue;
            };
            let section = ini.section(section_name);
            let stage_count = section
                .and_then(|section| section.get("NumberOfCampaignMaps"))
                .map(native_atoi_or_hex)
                .unwrap_or(0)
                .max(0) as usize;

            let mut stages = Vec::with_capacity(stage_count);
            for stage_index in 0..stage_count {
                let source_number = stage_index + 1;
                let variants = comma_tokens(
                    section.and_then(|section| section.get(&format!("Map{source_number}"))),
                );
                if variants.len() < MAP_VARIANT_COUNT {
                    return Err(CooperativeError::InvalidMapVariantCount {
                        campaign: section_name.to_string(),
                        stage: source_number,
                        found: variants.len(),
                    });
                }
                stages.push(CooperativeStage {
                    map_variants: [
                        variants[0].clone(),
                        variants[1].clone(),
                        variants[2].clone(),
                    ],
                    player_countries: comma_tokens(section.and_then(|section| {
                        section.get(&format!("CampaignPlayer{source_number}"))
                    })),
                    enemy_countries: comma_tokens(
                        section.and_then(|section| {
                            section.get(&format!("CampaignEnemy{source_number}"))
                        }),
                    ),
                });
            }

            campaigns.push(CooperativeCampaign {
                section: section_name.to_string(),
                campaign_name: read_string(section, "CampaignName"),
                load_screen: read_string(section, "CampaignLoadScreen"),
                load_screen_pallet: read_string(section, "CampaignLoadScreenPallet"),
                ai_difficulty: CooperativeAiDifficulty::from_ini(
                    section.and_then(|section| section.get("CampaignAI")),
                ),
                stages,
            });
        }

        Ok(Self { campaigns })
    }

    pub fn from_assets(assets: &AssetManager) -> Result<Self, CooperativeError> {
        let Some((bytes, source)) = assets.get_with_source_ref(COOPERATIVE_CAMPAIGN_INI) else {
            return Err(CooperativeError::MissingCampaignIni);
        };
        let ini =
            IniFile::from_bytes(bytes).map_err(|error| CooperativeError::InvalidCampaignIni {
                source_name: source.to_string(),
                message: error.to_string(),
            })?;
        Self::from_ini(&ini)
    }

    pub fn campaign(&self, campaign_type: usize) -> Option<&CooperativeCampaign> {
        self.campaigns.get(campaign_type)
    }

    /// Match any of the three source variants, preserving the registry's first
    /// source-order match. Scenario filenames are ASCII case-insensitive.
    pub fn campaign_for_map(&self, scenario: &str) -> Option<usize> {
        self.campaign_stage_for_map(scenario)
            .map(|(campaign, _)| campaign)
    }

    /// Resolve both the source-order campaign and stage for a concrete variant.
    /// The active progress record needs the stage index as its `CurrentMap`.
    pub fn campaign_stage_for_map(&self, scenario: &str) -> Option<(usize, usize)> {
        self.campaigns
            .iter()
            .enumerate()
            .find_map(|(campaign_index, campaign)| {
                campaign
                    .stages
                    .iter()
                    .position(|stage| {
                        stage
                            .map_variants
                            .iter()
                            .any(|variant| variant.eq_ignore_ascii_case(scenario))
                    })
                    .map(|stage_index| (campaign_index, stage_index))
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeProgressRecord {
    /// Native valid byte (`record+0x6C`).
    pub valid: bool,
    /// Native `CurrentMap`; fresh records start at -1, initialized at 0.
    pub current_map: i32,
    /// Native `CampaignType`; fresh records start at -1.
    pub campaign_type: i32,
    /// One RNG-selected filename per campaign stage.
    pub chosen_maps: Vec<String>,
}

impl Default for CooperativeProgressRecord {
    fn default() -> Self {
        Self {
            valid: false,
            current_map: -1,
            campaign_type: -1,
            chosen_maps: Vec::new(),
        }
    }
}

impl CooperativeProgressRecord {
    /// Initialize/reset a record for a source-order campaign index.
    ///
    /// A changed (including fresh `-1`) CampaignType makes exactly one logical
    /// inclusive `(0,2)` call per stage. Reinitializing the same CampaignType
    /// resets progress to map zero without rerolling or advancing the RNG.
    /// Returns whether the map vector was rebuilt.
    pub fn initialize_for_campaign(
        &mut self,
        registry: &CooperativeRegistry,
        campaign_index: usize,
        rng: &mut SimRng,
    ) -> Result<bool, CooperativeError> {
        let campaign =
            registry
                .campaign(campaign_index)
                .ok_or(CooperativeError::InvalidCampaignIndex {
                    index: campaign_index,
                    count: registry.campaigns.len(),
                })?;

        self.valid = true;
        self.current_map = 0;
        if self.campaign_type == campaign_index as i32 {
            return Ok(false);
        }

        let mut chosen_maps = Vec::with_capacity(campaign.stages.len());
        for stage in &campaign.stages {
            let variant = rng.next_range_u32_inclusive(0, 2) as usize;
            chosen_maps.push(stage.map_variants[variant].clone());
        }
        self.chosen_maps = chosen_maps;
        self.campaign_type = campaign_index as i32;
        Ok(true)
    }

    pub fn chosen_map(&self, map_index: usize) -> Option<&str> {
        self.chosen_maps.get(map_index).map(String::as_str)
    }

    pub fn current_chosen_map(&self) -> Option<&str> {
        usize::try_from(self.current_map)
            .ok()
            .and_then(|index| self.chosen_map(index))
    }
}

/// Cooperative mode object progress ownership relevant to shell RNG parity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CooperativeProgressState {
    active: Option<CooperativeProgressRecord>,
    reserves: Vec<Option<CooperativeProgressRecord>>,
}

impl CooperativeProgressState {
    /// Construct the per-campaign reserve vector after boot seeding. Stock data
    /// consumes ten logical `(0,2)` calls here even when Battle is later chosen.
    pub fn construct(
        registry: &CooperativeRegistry,
        rng: &mut SimRng,
    ) -> Result<Self, CooperativeError> {
        let mut reserves = Vec::with_capacity(registry.campaigns.len());
        for campaign_index in 0..registry.campaigns.len() {
            let mut record = CooperativeProgressRecord::default();
            record.initialize_for_campaign(registry, campaign_index, rng)?;
            reserves.push(Some(record));
        }
        Ok(Self {
            active: None,
            reserves,
        })
    }

    pub fn active(&self) -> Option<&CooperativeProgressRecord> {
        self.active.as_ref()
    }

    pub fn reserve(&self, campaign_index: usize) -> Option<&CooperativeProgressRecord> {
        self.reserves.get(campaign_index)?.as_ref()
    }

    /// Cooperative open creates a separately initialized active record only
    /// when the active pointer is null.
    pub fn ensure_active(
        &mut self,
        registry: &CooperativeRegistry,
        campaign_index: usize,
        rng: &mut SimRng,
    ) -> Result<&CooperativeProgressRecord, CooperativeError> {
        if self.active.is_none() {
            let mut record = CooperativeProgressRecord::default();
            record.initialize_for_campaign(registry, campaign_index, rng)?;
            self.active = Some(record);
        }
        Ok(self.active.as_ref().expect("active record was initialized"))
    }

    /// Accepted Choose Map campaign switching moves the selected preconstructed
    /// reserve into the active slot, then constructs a fresh replacement reserve
    /// for that campaign. The replacement consumes one `(0,2)` call per stage.
    pub fn accept_campaign_swap(
        &mut self,
        registry: &CooperativeRegistry,
        campaign_index: usize,
        rng: &mut SimRng,
    ) -> Result<&CooperativeProgressRecord, CooperativeError> {
        if campaign_index >= registry.campaigns.len() {
            return Err(CooperativeError::InvalidCampaignIndex {
                index: campaign_index,
                count: registry.campaigns.len(),
            });
        }
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.campaign_type == campaign_index as i32)
        {
            return Ok(self.active.as_ref().expect("selected campaign is active"));
        }
        let reserve_slot = self.reserves.get_mut(campaign_index).ok_or(
            CooperativeError::MissingReserveProgress {
                index: campaign_index,
            },
        )?;
        let selected = reserve_slot
            .take()
            .ok_or(CooperativeError::MissingReserveProgress {
                index: campaign_index,
            })?;
        self.active = Some(selected);

        let mut replacement = CooperativeProgressRecord::default();
        replacement.initialize_for_campaign(registry, campaign_index, rng)?;
        *reserve_slot = Some(replacement);
        Ok(self
            .active
            .as_ref()
            .expect("selected reserve became active"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeCountryRosterEntry {
    /// HouseType registry ID, such as `Americans`.
    pub id: String,
    /// Optional HouseType `Name=` alias, such as `America`.
    pub name: Option<String>,
}

impl CooperativeCountryRosterEntry {
    pub fn new(id: impl Into<String>, name: Option<&str>) -> Self {
        Self {
            id: id.into(),
            name: name.map(str::to_string),
        }
    }

    fn matches(&self, token: &str) -> bool {
        self.id.eq_ignore_ascii_case(token)
            || self
                .name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(token))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CooperativeCountryRole {
    Player,
    Enemy,
}

impl CooperativeCountryRole {
    fn label(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Enemy => "enemy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeCountryEligibility {
    accepted: Vec<bool>,
}

impl CooperativeCountryEligibility {
    /// Compile the token loop into the same predicate over every global roster
    /// index. `<random>` and unknown tokens never equal a concrete candidate.
    pub fn compile(
        tokens: &[String],
        roster: &[CooperativeCountryRosterEntry],
        role: CooperativeCountryRole,
    ) -> Result<Self, CooperativeError> {
        if roster.is_empty() {
            return Err(CooperativeError::EmptyCountryRoster);
        }
        let mut accepted = vec![false; roster.len()];
        for token in tokens {
            if let Some(index) = roster.iter().position(|entry| entry.matches(token)) {
                accepted[index] = true;
            }
        }
        if !accepted.iter().any(|accepted| *accepted) {
            return Err(CooperativeError::NoEligibleCountries { role: role.label() });
        }
        Ok(Self { accepted })
    }

    pub fn accepts(&self, global_country_index: usize) -> bool {
        self.accepted
            .get(global_country_index)
            .copied()
            .unwrap_or(false)
    }

    /// Draw against the full global roster bound until membership succeeds.
    /// Validation at construction makes the native unbounded retry safe without
    /// adding an attempt cap that would change the RNG transcript.
    pub fn draw(&self, rng: &mut SimRng) -> usize {
        loop {
            let candidate =
                rng.next_range_u32_inclusive(0, (self.accepted.len() - 1) as u32) as usize;
            if self.accepted[candidate] {
                return candidate;
            }
        }
    }
}

/// Cooperative human/AI country callback over the active progress record.
///
/// Invalid/missing campaign or map data returns country zero without consuming
/// RNG. A present but empty/all-invalid eligibility list returns a validation
/// error instead of entering the native infinite retry loop.
pub fn draw_country_for_progress(
    registry: &CooperativeRegistry,
    progress: &CooperativeProgressRecord,
    role: CooperativeCountryRole,
    roster: &[CooperativeCountryRosterEntry],
    rng: &mut SimRng,
) -> Result<usize, CooperativeError> {
    if !progress.valid {
        return Ok(0);
    }
    let Ok(campaign_index) = usize::try_from(progress.campaign_type) else {
        return Ok(0);
    };
    let Some(campaign) = registry.campaign(campaign_index) else {
        return Ok(0);
    };
    let Ok(map_index) = usize::try_from(progress.current_map) else {
        return Ok(0);
    };
    let Some(stage) = campaign.stages.get(map_index) else {
        return Ok(0);
    };
    let tokens = match role {
        CooperativeCountryRole::Player => &stage.player_countries,
        CooperativeCountryRole::Enemy => &stage.enemy_countries,
    };
    let eligibility = CooperativeCountryEligibility::compile(tokens, roster, role)?;
    Ok(eligibility.draw(rng))
}

fn read_string(section: Option<&IniSection>, key: &str) -> String {
    section
        .and_then(|section| section.get(key))
        .unwrap_or_default()
        .to_string()
}

fn comma_tokens(value: Option<&str>) -> Vec<String> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| value.split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

fn native_atoi_or_hex(value: &str) -> i32 {
    let value = value.trim_matches(|character: char| (character as u32) <= 0x20);
    if let Some(hex) = value.strip_prefix('$') {
        return parse_hex_prefix(hex).unwrap_or(0);
    }
    if value
        .as_bytes()
        .last()
        .is_some_and(|byte| byte.eq_ignore_ascii_case(&b'h'))
    {
        return parse_hex_prefix(&value[..value.len().saturating_sub(1)]).unwrap_or(0);
    }
    parse_decimal_prefix(value)
}

fn parse_hex_prefix(value: &str) -> Option<i32> {
    let mut parsed = 0u32;
    let mut found = false;
    for byte in value.bytes() {
        let digit = match byte {
            b'0'..=b'9' => u32::from(byte - b'0'),
            b'a'..=b'f' => u32::from(byte - b'a' + 10),
            b'A'..=b'F' => u32::from(byte - b'A' + 10),
            _ => break,
        };
        parsed = parsed.wrapping_mul(16).wrapping_add(digit);
        found = true;
    }
    found.then_some(parsed as i32)
}

fn parse_decimal_prefix(value: &str) -> i32 {
    let bytes = value.as_bytes();
    let mut cursor = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            cursor = 1;
            true
        }
        Some(b'+') => {
            cursor = 1;
            false
        }
        _ => false,
    };
    let mut parsed = 0u32;
    while let Some(&byte) = bytes.get(cursor) {
        if !byte.is_ascii_digit() {
            break;
        }
        parsed = parsed.wrapping_mul(10).wrapping_add(u32::from(byte - b'0'));
        cursor += 1;
    }
    let signed = parsed as i32;
    if negative {
        signed.wrapping_neg()
    } else {
        signed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CooperativeRegistry {
        CooperativeRegistry::from_ini(&IniFile::from_str(
            "[Campaigns]\nCampaign1=Allied\nCampaign2=World\n\
             [Allied]\nNumberOfCampaignMaps=2\nCampaignName=Name:All_Coop\n\
             CampaignPlayer1=Americans,British\nCampaignEnemy1=Russians,YuriCountry\n\
             CampaignPlayer2=Americans\nCampaignEnemy2=YuriCountry\nCampaignAI=Normal\n\
             Map1=A1,A1B,A1C\nMap2=A2,A2B,A2C\nCampaignLoadScreen=LSA.shp\n\
             CampaignLoadScreenPallet=LSA.pal\n\
             [World]\nNumberOfCampaignMaps=1\nCampaignPlayer1=America\n\
             CampaignEnemy1=YuriCountry\nMap1=W1,W1B,W1C\n",
        ))
        .unwrap()
    }

    fn roster() -> Vec<CooperativeCountryRosterEntry> {
        vec![
            CooperativeCountryRosterEntry::new("Americans", Some("America")),
            CooperativeCountryRosterEntry::new("British", Some("Great Britain")),
            CooperativeCountryRosterEntry::new("Russians", Some("Russia")),
            CooperativeCountryRosterEntry::new("YuriCountry", Some("YuriCountry")),
        ]
    }

    #[test]
    fn parses_campaigns_in_source_order_and_native_key_spelling() {
        let registry = registry();
        assert_eq!(registry.campaigns.len(), 2);
        assert_eq!(registry.campaigns[0].section, "Allied");
        assert_eq!(registry.campaigns[1].section, "World");
        assert_eq!(registry.campaigns[0].campaign_name, "Name:All_Coop");
        assert_eq!(registry.campaigns[0].load_screen, "LSA.shp");
        assert_eq!(registry.campaigns[0].load_screen_pallet, "LSA.pal");
        assert_eq!(
            registry.campaigns[0].ai_difficulty,
            CooperativeAiDifficulty::Normal
        );
        assert_eq!(
            registry.campaigns[1].ai_difficulty,
            CooperativeAiDifficulty::Easy
        );
        assert_eq!(registry.campaigns[0].stages[1].map_variants[2], "A2C");
        assert_eq!(registry.campaign_for_map("a2b"), Some(0));
    }

    #[test]
    fn rejects_stage_with_fewer_than_three_native_variants() {
        let ini = IniFile::from_str(
            "[Campaigns]\n1=Broken\n[Broken]\nNumberOfCampaignMaps=1\nMap1=One,Two\n",
        );
        assert_eq!(
            CooperativeRegistry::from_ini(&ini),
            Err(CooperativeError::InvalidMapVariantCount {
                campaign: "Broken".to_string(),
                stage: 1,
                found: 2,
            })
        );
    }

    #[test]
    fn fresh_progress_draws_once_per_stage_and_same_type_does_not_reroll() {
        let registry = registry();
        let mut rng = SimRng::new(0x1234);
        let mut reference = rng.clone();
        let expected: Vec<String> = registry.campaigns[0]
            .stages
            .iter()
            .map(|stage| {
                let variant = reference.next_range_u32_inclusive(0, 2) as usize;
                stage.map_variants[variant].clone()
            })
            .collect();

        let mut progress = CooperativeProgressRecord::default();
        assert!(
            progress
                .initialize_for_campaign(&registry, 0, &mut rng)
                .unwrap()
        );
        assert_eq!(progress.chosen_maps, expected);
        assert_eq!(rng.state(), reference.state());

        progress.current_map = 1;
        let before = rng.state();
        let chosen_before = progress.chosen_maps.clone();
        assert!(
            !progress
                .initialize_for_campaign(&registry, 0, &mut rng)
                .unwrap()
        );
        assert_eq!(progress.current_map, 0);
        assert_eq!(progress.chosen_maps, chosen_before);
        assert_eq!(rng.state(), before);
    }

    #[test]
    fn construction_initializes_every_reserve_and_swap_resamples_replacement() {
        let registry = registry();
        let mut rng = SimRng::new(77);
        let mut reference = rng.clone();
        for campaign in &registry.campaigns {
            for _ in &campaign.stages {
                reference.next_range_u32_inclusive(0, 2);
            }
        }
        let mut state = CooperativeProgressState::construct(&registry, &mut rng).unwrap();
        assert_eq!(rng.state(), reference.state());

        let selected_reserve = state.reserve(1).unwrap().clone();
        for _ in &registry.campaigns[1].stages {
            reference.next_range_u32_inclusive(0, 2);
        }
        let active = state.accept_campaign_swap(&registry, 1, &mut rng).unwrap();
        assert_eq!(*active, selected_reserve);
        assert!(state.reserve(1).is_some());
        assert_eq!(rng.state(), reference.state());
    }

    #[test]
    fn ensure_active_only_draws_when_pointer_is_null() {
        let registry = registry();
        let mut rng = SimRng::new(9);
        let mut state = CooperativeProgressState::construct(&registry, &mut rng).unwrap();
        state.ensure_active(&registry, 0, &mut rng).unwrap();
        let after_first = rng.state();
        state.ensure_active(&registry, 1, &mut rng).unwrap();
        assert_eq!(rng.state(), after_first);
        assert_eq!(state.active().unwrap().campaign_type, 0);
    }

    #[test]
    fn accepting_current_campaign_does_not_replace_or_resample() {
        let registry = registry();
        let mut rng = SimRng::new(19);
        let mut state = CooperativeProgressState::construct(&registry, &mut rng).unwrap();
        state.ensure_active(&registry, 0, &mut rng).unwrap();
        let before_rng = rng.state();
        let before_reserve = state.reserve(0).unwrap().clone();

        state.accept_campaign_swap(&registry, 0, &mut rng).unwrap();

        assert_eq!(rng.state(), before_rng);
        assert_eq!(*state.reserve(0).unwrap(), before_reserve);
    }

    #[test]
    fn country_draw_uses_global_bound_and_unbounded_retries() {
        let registry = registry();
        let mut progress = CooperativeProgressRecord::default();
        progress.valid = true;
        progress.campaign_type = 0;
        progress.current_map = 0;
        let roster = roster();

        let mut rng = SimRng::new(31);
        let mut reference = rng.clone();
        let expected = loop {
            let candidate = reference.next_range_u32_inclusive(0, 3) as usize;
            if candidate == 0 || candidate == 1 {
                break candidate;
            }
        };
        let actual = draw_country_for_progress(
            &registry,
            &progress,
            CooperativeCountryRole::Player,
            &roster,
            &mut rng,
        )
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(rng.state(), reference.state());
    }

    #[test]
    fn country_name_alias_resolves_to_global_roster_index() {
        let registry = registry();
        let mut progress = CooperativeProgressRecord::default();
        progress.valid = true;
        progress.campaign_type = 1;
        progress.current_map = 0;
        let mut rng = SimRng::new(1);
        assert_eq!(
            draw_country_for_progress(
                &registry,
                &progress,
                CooperativeCountryRole::Player,
                &roster(),
                &mut rng,
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn invalid_progress_returns_zero_without_rng_and_invalid_list_returns_error() {
        let registry = registry();
        let roster = roster();
        let mut rng = SimRng::new(5);
        let before = rng.state();
        assert_eq!(
            draw_country_for_progress(
                &registry,
                &CooperativeProgressRecord::default(),
                CooperativeCountryRole::Player,
                &roster,
                &mut rng,
            )
            .unwrap(),
            0
        );
        assert_eq!(rng.state(), before);

        let mut progress = CooperativeProgressRecord::default();
        progress.valid = true;
        progress.campaign_type = 0;
        progress.current_map = 1;
        let no_match_roster = vec![CooperativeCountryRosterEntry::new(
            "Russians",
            Some("Russia"),
        )];
        let before = rng.state();
        assert_eq!(
            draw_country_for_progress(
                &registry,
                &progress,
                CooperativeCountryRole::Player,
                &no_match_roster,
                &mut rng,
            ),
            Err(CooperativeError::NoEligibleCountries { role: "player" })
        );
        assert_eq!(rng.state(), before);
    }
}
