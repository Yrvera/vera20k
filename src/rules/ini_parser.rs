//! Parser and layer composer for Westwood INI data.
//!
//! Active `gamemd.exe` treats raw section and key names as case-sensitive.
//! A fresh load retains duplicate nonempty section bodies. Empty keys, values,
//! and physical section bodies are not inserted. Arbitrary duplicate-name
//! lookup remains a native CRC/qsort exactification residual.
//! Rules layers are processed separately: ordinary values update live fields,
//! while numbered type registries find-or-allocate by their value.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::rules::crate_rules::{CrateRules, CrateRulesAccumulator};
use crate::rules::error::RulesError;

const READ_LINE_PAYLOAD: usize = 511;

/// One physical section occurrence in an INI file.
#[derive(Debug, Clone)]
pub struct IniSection {
    /// Exact section spelling from the file.
    pub name: String,
    /// Exact keys and their values. Initial duplicate keys are first-wins.
    entries: HashMap<String, String>,
    /// Exact keys in their first insertion order.
    key_order: Vec<String>,
    /// Values presented by successive `RulesClass::Process` passes.
    ///
    /// Raw INIs leave this empty. The compatibility projection retains it so
    /// typed readers can use the current live field as the next pass default.
    projected_values: HashMap<String, Vec<String>>,
}

impl IniSection {
    fn new(name: String) -> Self {
        Self {
            name,
            entries: HashMap::new(),
            key_order: Vec::new(),
            projected_values: HashMap::new(),
        }
    }

    fn overlay_rules_pass(&mut self, patch: &IniSection) {
        for key in patch.keys() {
            if let Some(value) = patch.get(key) {
                self.set_projected(key, value);
            }
        }
    }

    fn set_projected(&mut self, key: &str, value: &str) {
        if !self.entries.contains_key(key) {
            self.key_order.push(key.to_string());
        }
        self.entries.insert(key.to_string(), value.to_string());
        self.projected_values
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }

    /// Insert during a fresh file load. The Rust compatibility lookup keeps
    /// the first exact duplicate; native multi-duplicate CRC/qsort selection
    /// is intentionally outside the ordinary-retail contract.
    fn insert_initial(&mut self, key: &str, value: &str) {
        if self.entries.contains_key(key) {
            return;
        }
        self.key_order.push(key.to_string());
        self.entries.insert(key.to_string(), value.to_string());
    }

    /// Apply a later load/rules pass. Existing exact keys are replaced and new
    /// keys retain layer source order.
    pub(crate) fn set(&mut self, key: &str, value: &str) {
        if !self.entries.contains_key(key) {
            self.key_order.push(key.to_string());
        }
        self.entries.insert(key.to_string(), value.to_string());
        self.projected_values.remove(key);
    }

    /// Get an exact-case key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Integer reader with native `$FF`, `FFh`, and C `atoi` prefix behavior.
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        if let Some(values) = self.projected_values.get(key) {
            let mut resolved = None;
            for value in values {
                if let Some(parsed) = crate::rules::ini_value::parse_read_int_value(value) {
                    resolved = Some(parsed);
                }
            }
            resolved
        } else {
            crate::rules::ini_value::parse_read_int_value(self.get(key)?)
        }
    }

    /// Native float read: parse as `f32`, then return that value.
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key)?;
        Some(self.read_double(key, 0.0) as f32)
    }

    /// Native double read widens the parsed `f32`; it does not parse an f64
    /// mantissa directly.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key)?;
        Some(self.read_double(key, 0.0))
    }

    /// Westwood numeric reads stop before comma-separated trailing text.
    pub fn get_light_f32(&self, key: &str) -> Option<f32> {
        let val = self.get(key)?;
        let number = val.split_once(',').map_or(val, |(head, _)| head);
        Some(crate::rules::ini_value::parse_leading_f32(number))
    }

    /// A percent sign anywhere in the value scales the parsed f32 by 0.01.
    pub fn get_percent(&self, key: &str) -> Option<f32> {
        self.get(key)?;
        Some(self.read_double(key, 0.0) as f32)
    }

    /// Native boolean reads inspect only the first trimmed character.
    ///
    /// Retail provenance: current-field default — `WeaponTypeClass__ReadINI` @
    /// `0x00772080`, calling `CCINIClass__ReadBool` @ `0x005295F0`.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let mut resolved = None;
        if let Some(values) = self.projected_values.get(key) {
            for value in values {
                if let Some(parsed) = parse_bool_value(value) {
                    resolved = Some(parsed);
                }
            }
            resolved
        } else {
            parse_bool_value(self.get(key)?)
        }
    }

    pub(crate) fn projected_values(&self, key: &str) -> Option<&[String]> {
        self.projected_values.get(key).map(Vec::as_slice)
    }

    pub fn get_list(&self, key: &str) -> Option<Vec<&str>> {
        let val = self.get(key)?;
        Some(val.split(',').map(trim_ascii_controls).collect())
    }

    /// Values of every entry in source order. Native registry loops use
    /// GetEntryCount/GetEntryName-by-index and do not inspect the key spelling.
    pub fn get_values(&self) -> Vec<&str> {
        self.key_order
            .iter()
            .filter_map(|key| self.entries.get(key).map(String::as_str))
            .collect()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.key_order.iter().map(String::as_str)
    }

    fn contains_value_ignore_ascii_case(&self, value: &str) -> bool {
        self.key_order.iter().any(|key| {
            self.entries
                .get(key)
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(value))
        })
    }

    fn append_numbered_value(&mut self, value: &str) {
        let mut next = self
            .key_order
            .iter()
            .filter_map(|key| key.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        if self.key_order.iter().any(|key| key == "0") {
            next = next.saturating_add(1);
        }
        while self.entries.contains_key(&next.to_string()) {
            next = next.saturating_add(1);
        }
        self.set(&next.to_string(), value);
    }

    /// Value lookup that ignores key case.
    ///
    /// The ordinary readers match keys exactly, which is right for almost
    /// everything because retail spells keys consistently. `MaxDebris=` is the
    /// exception this exists for: 17 of the 456 stock sections that author it
    /// spell it `Maxdebris=`, and gamemd's `INIClass` compares keys
    /// case-insensitively, so those 17 do take effect in retail. Reach for this
    /// only where a retail spelling inconsistency is known and counted — making
    /// every reader case-insensitive is an INI-layer change with its own row.
    pub fn get_ignoring_case(&self, key: &str) -> Option<&str> {
        self.key_ignore_ascii_case(key)
            .and_then(|exact| self.entries.get(exact))
            .map(String::as_str)
    }

    /// [`Self::get_i32`]'s case-insensitive sibling; see
    /// [`Self::get_ignoring_case`] for when to use it.
    pub fn get_i32_ignoring_case(&self, key: &str) -> Option<i32> {
        let exact = self.key_ignore_ascii_case(key)?.to_string();
        self.get_i32(&exact)
    }

    fn key_ignore_ascii_case(&self, key: &str) -> Option<&str> {
        self.key_order
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(key))
            .map(String::as_str)
    }
}

/// Parsed section occurrences plus Rust's deterministic exact-name lookup index
/// for the supported ordinary INI contract.
#[derive(Debug, Clone)]
pub struct IniFile {
    sections: Vec<IniSection>,
    first_section: HashMap<String, usize>,
}

impl IniFile {
    fn empty() -> Self {
        Self {
            sections: Vec::new(),
            first_section: HashMap::new(),
        }
    }

    /// Parse arbitrary bytes by zero-extending each byte to a Unicode scalar.
    /// This mirrors gamemd's ordinary byte-to-wide helper; INI data is not
    /// interpreted as UTF-8, CP1252, or the Windows active code page.
    pub fn from_bytes(data: &[u8]) -> Result<Self, RulesError> {
        let text = crate::util::native_string::widen_bytes(data);
        Ok(Self::from_str(&text))
    }

    pub fn from_str(text: &str) -> Self {
        let mut ini = Self::empty();
        let mut current_section = None;

        for physical_line in text.split('\n') {
            // Straw::ReadLine removes every CR byte while consuming the line.
            let without_carriage_returns: String = physical_line
                .chars()
                .filter(|character| *character != '\r')
                .collect();
            if without_carriage_returns.is_empty() {
                continue;
            }

            // Straw::ReadLine stores at most 511 payload bytes and consumes the
            // remainder through LF. The discarded tail is never a second line.
            let chunk_end = without_carriage_returns
                .char_indices()
                .nth(READ_LINE_PAYLOAD)
                .map_or(without_carriage_returns.len(), |(index, _)| index);
            let buffered = &without_carriage_returns[..chunk_end];
            // NUL occupies a buffer byte, but subsequent C-string operations
            // make the rest of that physical line invisible to the loader.
            let visible = buffered.split_once('\0').map_or(buffered, |(head, _)| head);
            Self::parse_line(&mut ini, &mut current_section, visible);
        }

        // Retail provenance: INI lexical loading — `INIClass__LoadFromStraw` @ `0x00525A60`.
        // Active read mode destroys a candidate section unless at least one
        // accepted nonempty entry was linked into it.
        ini.discard_entryless_sections();
        ini
    }

    fn discard_entryless_sections(&mut self) {
        self.sections.retain(|section| section.entry_count() != 0);
        self.first_section.clear();
        for (index, section) in self.sections.iter().enumerate() {
            self.first_section
                .entry(section.name.clone())
                .or_insert(index);
        }
    }

    fn parse_line(ini: &mut Self, current_section: &mut Option<usize>, raw_line: &str) {
        let line = trim_ascii_controls(raw_line);
        if line.is_empty() {
            return;
        }

        if line.starts_with('[')
            && let Some(end) = line.find(']')
        {
            let name = &line[1..end];
            let index = ini.sections.len();
            ini.sections.push(IniSection::new(name.to_string()));
            ini.first_section.entry(name.to_string()).or_insert(index);
            *current_section = Some(index);
            return;
        }

        // Semicolon truncation happens before the first-equals split. `#` has
        // no comment meaning in the active parser.
        let payload = trim_ascii_controls(line.split_once(';').map_or(line, |(head, _)| head));
        let Some((key, value)) = payload.split_once('=') else {
            return;
        };
        let key = trim_ascii_controls(key);
        let value = trim_ascii_controls(value);
        if key.is_empty() || value.is_empty() {
            return;
        }
        if let Some(index) = *current_section {
            ini.sections[index].insert_initial(key, value);
        }
    }

    /// Exact raw INI lookup.
    pub fn section(&self, name: &str) -> Option<&IniSection> {
        self.first_section
            .get(name)
            .and_then(|index| self.sections.get(*index))
    }

    pub fn section_names(&self) -> Vec<&str> {
        self.sections
            .iter()
            .map(|section| section.name.as_str())
            .collect()
    }

    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Load another INI into an already populated INI object. This models the
    /// native PutString path: later nonempty exact keys replace earlier ones.
    pub fn merge(&mut self, patch: &IniFile) {
        for (patch_index, patch_section) in patch.sections.iter().enumerate() {
            if patch.first_section.get(&patch_section.name) != Some(&patch_index) {
                continue;
            }
            self.overlay_section(patch_section);
        }
    }

    /// Build the typed-reader compatibility view for one ordered rules pass.
    ///
    /// Retail provenance: sequential typed defaults — `RulesClass__Process` @ `0x00668BF0`.
    fn merge_rules_projection(&mut self, patch: &IniFile) {
        for (patch_index, patch_section) in patch.sections.iter().enumerate() {
            if patch.first_section.get(&patch_section.name) != Some(&patch_index) {
                continue;
            }
            if let Some(index) = self.first_section.get(&patch_section.name).copied() {
                self.sections[index].overlay_rules_pass(patch_section);
            } else {
                let mut section = IniSection::new(patch_section.name.clone());
                section.overlay_rules_pass(patch_section);
                let index = self.sections.len();
                self.first_section.insert(section.name.clone(), index);
                self.sections.push(section);
            }
        }
    }

    /// Apply a later RulesClass processing pass (LANGRULE/mode/map).
    ///
    /// Ordinary sections update exact keys and may introduce referenced type
    /// sections. Type registries append new case-insensitive type
    /// identities in layer source order, regardless of repeated numeric keys.
    /// `[Colors]` similarly find-or-allocates by color name.
    pub fn merge_rules_layer(&mut self, patch: &IniFile) -> usize {
        let mut applied = 0;
        for (patch_index, patch_section) in patch.sections.iter().enumerate() {
            if patch.first_section.get(&patch_section.name) != Some(&patch_index) {
                continue;
            }
            if RULE_TYPE_REGISTRIES.contains(&patch_section.name.as_str()) {
                applied += self.merge_type_registry(patch_section);
            } else if patch_section.name == "Colors" {
                applied += self.merge_named_registry(patch_section);
            } else if self.section(&patch_section.name).is_some()
                || self.layer_references_section(patch, &patch_section.name)
            {
                applied += self.overlay_section(patch_section);
            }
        }
        applied
    }

    /// Compatibility name for map callers; maps use the same later
    /// RulesClass pass as mode INIs and may allocate new types and colors.
    pub fn merge_rules_overrides(&mut self, patch: &IniFile) -> usize {
        self.merge_rules_layer(patch)
    }

    fn overlay_section(&mut self, patch_section: &IniSection) -> usize {
        if let Some(index) = self.first_section.get(&patch_section.name).copied() {
            let target = &mut self.sections[index];
            for key in patch_section.keys() {
                if let Some(value) = patch_section.get(key) {
                    target.set(key, value);
                }
            }
        } else {
            let index = self.sections.len();
            self.sections.push(patch_section.clone());
            self.first_section.insert(patch_section.name.clone(), index);
        }
        patch_section.entry_count()
    }

    fn replace_first_section(&mut self, section: IniSection) {
        if let Some(index) = self.first_section.get(&section.name).copied() {
            self.sections[index] = section;
        } else {
            let index = self.sections.len();
            self.first_section.insert(section.name.clone(), index);
            self.sections.push(section);
        }
    }

    fn merge_type_registry(&mut self, patch_section: &IniSection) -> usize {
        let target_index = if let Some(index) = self.first_section.get(&patch_section.name) {
            *index
        } else {
            let index = self.sections.len();
            self.sections
                .push(IniSection::new(patch_section.name.clone()));
            self.first_section.insert(patch_section.name.clone(), index);
            index
        };
        let target = &mut self.sections[target_index];
        let mut applied = 0;
        for value in patch_section.get_values() {
            if !target.contains_value_ignore_ascii_case(value) {
                target.append_numbered_value(value);
                applied += 1;
            }
        }
        applied
    }

    fn merge_named_registry(&mut self, patch_section: &IniSection) -> usize {
        let target_index = if let Some(index) = self.first_section.get(&patch_section.name) {
            *index
        } else {
            let index = self.sections.len();
            self.sections
                .push(IniSection::new(patch_section.name.clone()));
            self.first_section.insert(patch_section.name.clone(), index);
            index
        };
        let target = &mut self.sections[target_index];
        let mut applied = 0;
        for key in patch_section.keys() {
            let Some(value) = patch_section.get(key) else {
                continue;
            };
            if target.key_ignore_ascii_case(key).is_some() {
                continue;
            }
            target.set(key, value);
            applied += 1;
        }
        applied
    }

    fn layer_references_section(&self, patch: &IniFile, name: &str) -> bool {
        self.sections
            .iter()
            .chain(patch.sections.iter())
            .flat_map(|section| {
                section
                    .keys()
                    .filter_map(|key| section.get(key))
                    .flat_map(|value| value.split(','))
            })
            .any(|value| trim_ascii_controls(value).eq_ignore_ascii_case(name))
    }

    /// Deterministic hash over native section occurrence and entry order.
    pub fn content_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for section in &self.sections {
            section.name.hash(&mut hasher);
            for key in section.keys() {
                key.hash(&mut hasher);
                if let Some(value) = section.get(key) {
                    value.hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }
}

/// One native `RulesClass::Process` source in its runtime position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulesLayerKind {
    RulesMd,
    LangRule,
    GameMode,
    Scenario,
}

#[derive(Debug, Clone)]
struct RulesLayer {
    kind: RulesLayerKind,
    ini: IniFile,
}

/// Ordered active-YR rules sources.
///
/// Each member remains an independent INI because `RulesClass::Process`
/// applies it to live type state. Flattening the text first is observably
/// wrong: a type allocated by a later map must not read an orphan body that
/// appeared in an earlier source.
#[derive(Debug, Clone)]
pub struct RulesLayerStack {
    layers: Vec<RulesLayer>,
}

impl RulesLayerStack {
    pub fn new(rulesmd: IniFile) -> Self {
        Self {
            layers: vec![RulesLayer {
                kind: RulesLayerKind::RulesMd,
                ini: rulesmd,
            }],
        }
    }

    pub fn push(&mut self, kind: RulesLayerKind, ini: IniFile) {
        self.layers.push(RulesLayer { kind, ini });
    }

    pub fn iter_passes(&self) -> impl Iterator<Item = (RulesLayerKind, &IniFile)> {
        self.layers.iter().map(|layer| (layer.kind, &layer.ini))
    }

    /// Hash both source contents and pass boundaries. Two stacks that flatten
    /// to the same key/value view can still produce different live type state.
    pub fn content_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        "VERA20K_RULES_LAYER_STACK_V1".hash(&mut hasher);
        self.layers.len().hash(&mut hasher);
        for layer in &self.layers {
            layer.kind.hash(&mut hasher);
            layer.ini.content_hash().hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Apply the verified registry-allocation and per-pass body timing.
    ///
    /// The returned INI is a compatibility projection for existing typed Rust
    /// readers. It contains the final live scalar values, unioned registries,
    /// and only the per-type keys read at or after that type was allocated.
    pub fn process(&self) -> ProcessedRulesLayers {
        let mut processor = RulesPassProcessor::default();
        for (_, ini) in self.iter_passes() {
            processor.apply_pass(ini);
        }
        let (ini, crate_rules) = processor.finish();
        ProcessedRulesLayers {
            ini,
            crate_rules,
            content_hash: self.content_hash(),
        }
    }
}

/// Result of applying an ordered rules stack.
#[derive(Debug, Clone)]
pub struct ProcessedRulesLayers {
    ini: IniFile,
    crate_rules: CrateRules,
    content_hash: u64,
}

impl ProcessedRulesLayers {
    pub fn ini(&self) -> &IniFile {
        &self.ini
    }

    pub fn into_ini(self) -> IniFile {
        self.ini
    }

    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    pub fn crate_rules(&self) -> &CrateRules {
        &self.crate_rules
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RulesTypeFamily {
    Country,
    Overlay,
    SuperWeapon,
    Warhead,
    Smudge,
    Terrain,
    Building,
    Vehicle,
    Aircraft,
    Infantry,
    Animation,
    VoxelAnimation,
    Particle,
    ParticleSystem,
    Weapon,
    Projectile,
}

const RULE_TYPE_FAMILIES: &[(&str, RulesTypeFamily)] = &[
    ("Countries", RulesTypeFamily::Country),
    ("OverlayTypes", RulesTypeFamily::Overlay),
    ("SuperWeaponTypes", RulesTypeFamily::SuperWeapon),
    ("Warheads", RulesTypeFamily::Warhead),
    ("SmudgeTypes", RulesTypeFamily::Smudge),
    ("TerrainTypes", RulesTypeFamily::Terrain),
    ("BuildingTypes", RulesTypeFamily::Building),
    ("VehicleTypes", RulesTypeFamily::Vehicle),
    ("AircraftTypes", RulesTypeFamily::Aircraft),
    ("InfantryTypes", RulesTypeFamily::Infantry),
    ("Animations", RulesTypeFamily::Animation),
    ("VoxelAnims", RulesTypeFamily::VoxelAnimation),
    ("Particles", RulesTypeFamily::Particle),
    ("ParticleSystems", RulesTypeFamily::ParticleSystem),
];

#[derive(Debug, Clone)]
struct ProcessedType {
    canonical_name: String,
    body: IniSection,
}

impl ProcessedType {
    fn new(canonical_name: String) -> Self {
        Self {
            body: IniSection::new(canonical_name.clone()),
            canonical_name,
        }
    }
}

#[derive(Debug, Default)]
struct RulesPassProcessor {
    ordinary: Option<IniFile>,
    crate_rules: CrateRulesAccumulator,
    families: HashMap<RulesTypeFamily, Vec<ProcessedType>>,
    tiberiums: Vec<ProcessedType>,
    colors: Vec<(String, String)>,
    prerequisite_groups: HashMap<&'static str, Vec<String>>,
}

impl RulesPassProcessor {
    fn apply_pass(&mut self, pass: &IniFile) {
        if let Some(ordinary) = self.ordinary.as_mut() {
            ordinary.merge_rules_projection(pass);
        } else {
            let mut ordinary = IniFile::empty();
            ordinary.merge_rules_projection(pass);
            self.ordinary = Some(ordinary);
        }
        // RulesClass::Process allocates every explicit registry before the
        // TypeClass Read_INI sweep. Registry entry names are arbitrary; their
        // values are consumed in source order through a 32-byte local buffer.
        for &(registry, family) in RULE_TYPE_FAMILIES {
            let Some(section) = pass.section(registry) else {
                continue;
            };
            for key in section.keys() {
                let identity = section.read_string(key, "", 32);
                if !identity.is_empty() {
                    self.find_or_allocate(family, &identity);
                }
            }
        }
        self.allocate_colors(pass);
        self.read_prerequisite_groups(pass);
        self.allocate_pre_type_references(pass);

        // RulesClass::ReadTypeData @ 0x00679A10. Animation bodies are the one
        // deliberate omission here: their Read_INI receives fixed g_ArtINI,
        // never the current rules pass.
        self.apply_family(RulesTypeFamily::Country, pass);
        let super_weapon_refs =
            self.apply_family_and_collect(RulesTypeFamily::SuperWeapon, pass, &["WeaponType"]);
        self.allocate_many(RulesTypeFamily::Weapon, super_weapon_refs);

        for family in [
            RulesTypeFamily::Building,
            RulesTypeFamily::Aircraft,
            RulesTypeFamily::Vehicle,
            RulesTypeFamily::Infantry,
        ] {
            let mut particle_system_refs = self.collect_current_values(
                family,
                pass,
                &["NaturalParticleSystem", "RefinerySmokeParticleSystem"],
            );
            particle_system_refs.extend(self.collect_current_list_values(
                family,
                pass,
                &["DamageParticleSystems", "DestroyParticleSystems"],
            ));
            let weapon_refs = self.apply_family_and_collect(
                family,
                pass,
                &[
                    "Primary",
                    "Secondary",
                    "ElitePrimary",
                    "EliteSecondary",
                    "DeathWeapon",
                    "OccupyWeapon",
                    "EliteOccupyWeapon",
                    "Weapon1",
                    "Weapon2",
                    "Weapon3",
                    "Weapon4",
                    "Weapon5",
                    "Weapon6",
                    "Weapon7",
                    "Weapon8",
                    "Weapon9",
                    "Weapon10",
                    "Weapon11",
                    "Weapon12",
                    "Weapon13",
                    "Weapon14",
                    "Weapon15",
                    "Weapon16",
                    "Weapon17",
                ],
            );
            self.allocate_many(RulesTypeFamily::Weapon, weapon_refs);
            self.allocate_many(RulesTypeFamily::ParticleSystem, particle_system_refs);
        }

        let projectile_refs =
            self.apply_family_and_collect(RulesTypeFamily::Weapon, pass, &["Projectile"]);
        let warhead_refs = self.collect_current_values(RulesTypeFamily::Weapon, pass, &["Warhead"]);
        let attached_particle_systems =
            self.collect_current_values(RulesTypeFamily::Weapon, pass, &["AttachedParticleSystem"]);
        self.allocate_many(RulesTypeFamily::Projectile, projectile_refs);
        self.allocate_many(RulesTypeFamily::Warhead, warhead_refs);
        self.allocate_many(RulesTypeFamily::ParticleSystem, attached_particle_systems);

        // Bullet readers run after the weapon loop. Airburst/shrapnel weapons
        // allocated here therefore start reading bodies on the next rules pass.
        let late_weapon_refs = self.apply_family_and_collect(
            RulesTypeFamily::Projectile,
            pass,
            &["AirburstWeapon", "ShrapnelWeapon"],
        );
        self.allocate_many(RulesTypeFamily::Weapon, late_weapon_refs);
        self.apply_family(RulesTypeFamily::Warhead, pass);

        for family in [
            RulesTypeFamily::Terrain,
            RulesTypeFamily::Smudge,
            RulesTypeFamily::Overlay,
        ] {
            self.apply_family(family, pass);
        }

        let late_particle_warheads =
            self.apply_family_and_collect(RulesTypeFamily::Particle, pass, &["Warhead"]);
        self.allocate_many(RulesTypeFamily::Warhead, late_particle_warheads);
        let late_particle_refs =
            self.apply_family_and_collect(RulesTypeFamily::ParticleSystem, pass, &["HoldsWhat"]);
        self.allocate_many(RulesTypeFamily::Particle, late_particle_refs);
        self.apply_family(RulesTypeFamily::VoxelAnimation, pass);

        // These RulesClass readers execute after ReadTypeData. Projectile
        // objects first allocated here wait for a subsequent Process pass.
        // ReadSpecialWeapons then explicitly re-runs every Warhead ReadINI, so
        // late-created warheads do consume their body in this same pass.
        self.allocate_late_global_references(pass);
        self.crate_rules.apply_pass(pass);
        if pass.section("SpecialWeapons").is_some() {
            self.apply_family(RulesTypeFamily::Warhead, pass);
        }

        // TiberiumClass::ReadINI_All is the final type reader in Process. Its
        // registry keys are numeric slot selectors, not ordinary type IDs.
        self.process_tiberiums(pass);
    }

    fn family_mut(&mut self, family: RulesTypeFamily) -> &mut Vec<ProcessedType> {
        self.families.entry(family).or_default()
    }

    fn find_or_allocate(&mut self, family: RulesTypeFamily, identity: &str) {
        let identity = trim_ascii_controls(identity);
        // UnitTypeClass::FindOrAllocate @ 0x007480D0 rejects both native
        // no-type sentinels before the per-family case-insensitive lookup.
        if is_native_none_type_name(identity) {
            return;
        }
        let members = self.family_mut(family);
        if members
            .iter()
            .any(|member| member.canonical_name.eq_ignore_ascii_case(identity))
        {
            return;
        }
        members.push(ProcessedType::new(identity.to_string()));
    }

    fn allocate_many(&mut self, family: RulesTypeFamily, identities: Vec<String>) {
        for identity in identities {
            self.find_or_allocate(family, &identity);
        }
    }

    fn allocate_colors(&mut self, pass: &IniFile) {
        let Some(section) = pass.section("Colors") else {
            return;
        };
        for key in section.keys() {
            let Some(value) = section.get(key) else {
                continue;
            };
            if self
                .colors
                .iter()
                .any(|(existing, _)| existing.eq_ignore_ascii_case(key))
            {
                continue;
            }
            self.colors.push((key.to_string(), value.to_string()));
        }
    }

    fn allocate_pre_type_references(&mut self, pass: &IniFile) {
        let Some(general) = pass.section("General") else {
            return;
        };

        if let Some(value) = general.get("ParaDropPlane") {
            self.find_or_allocate(RulesTypeFamily::Aircraft, value);
        }
        for key in [
            "AmerParaDropInf",
            "AllyParaDropInf",
            "SovParaDropInf",
            "YuriParaDropInf",
        ] {
            if let Some(values) = general.get_list(key) {
                for value in values {
                    self.find_or_allocate(RulesTypeFamily::Infantry, value);
                }
            }
        }
        for key in ["BaseUnit"] {
            if let Some(values) = general.get_list(key) {
                for value in values {
                    self.find_or_allocate(RulesTypeFamily::Vehicle, value);
                }
            }
        }
        if let Some(value) = general.get("BarrelParticle") {
            self.find_or_allocate(RulesTypeFamily::ParticleSystem, value);
        }
        if let Some(value) = general.get("LightningWarhead") {
            self.find_or_allocate(RulesTypeFamily::Warhead, value);
        }
        for key in [
            "WarpIn",
            "WarpOut",
            "WarpAway",
            "ChronoSparkle1",
            "Wake",
            "MoveFlash",
            "Parachute",
            "IronCurtainInvokeAnim",
            "ForceShieldInvokeAnim",
        ] {
            if let Some(value) = general.get(key) {
                self.find_or_allocate(RulesTypeFamily::Animation, value);
            }
        }
        for key in ["DamageFireTypes", "MetallicDebris", "BridgeExplosions"] {
            if let Some(values) = general.get_list(key) {
                for value in values {
                    self.find_or_allocate(RulesTypeFamily::Animation, value);
                }
            }
        }
        if let Some(values) = general.get_list("ConcreteWalls") {
            for value in values {
                self.find_or_allocate(RulesTypeFamily::Building, value);
            }
        }
        // The six PrerequisiteXxx vectors are lookup-only and never allocate
        // their named buildings. ProcAlternate is a separate scalar UnitType
        // reference (the stock Slave Miner special case).
        if let Some(value) = general.get("PrerequisiteProcAlternate") {
            self.find_or_allocate(RulesTypeFamily::Vehicle, value);
        }
    }

    fn read_prerequisite_groups(&mut self, pass: &IniFile) {
        const KEYS: &[&str] = &[
            "PrerequisitePower",
            "PrerequisiteProc",
            "PrerequisiteRadar",
            "PrerequisiteTech",
            "PrerequisiteBarracks",
            "PrerequisiteFactory",
        ];

        let Some(general) = pass.section("General") else {
            return;
        };
        for &key in KEYS {
            let Some(values) = general.get_list(key) else {
                continue;
            };
            let resolved = values
                .into_iter()
                .filter_map(|identity| {
                    self.families
                        .get(&RulesTypeFamily::Building)?
                        .iter()
                        .find(|member| member.canonical_name.eq_ignore_ascii_case(identity))
                        .map(|member| member.canonical_name.clone())
                })
                .collect();
            self.prerequisite_groups.insert(key, resolved);
        }
    }

    fn process_tiberiums(&mut self, pass: &IniFile) {
        let Some(registry) = pass.section("Tiberiums") else {
            return;
        };

        for key in registry.keys() {
            let slot = crate::rules::ini_value::atoi_lenient(key);
            let identity = registry.read_string(key, "", 0x18);
            if identity.is_empty() {
                continue;
            }

            let index = if slot < self.tiberiums.len() as i32 {
                // Retail performs the same signed comparison and then indexes
                // directly. A malformed negative slot therefore faults; the
                // Rust bounds check intentionally rejects it instead of
                // silently turning it into an append.
                slot as usize
            } else {
                self.tiberiums.push(ProcessedType::new(identity));
                self.tiberiums.len() - 1
            };
            let canonical_name = self.tiberiums[index].canonical_name.clone();
            if let Some(section) = pass.section(&canonical_name) {
                self.tiberiums[index].body.overlay_rules_pass(section);
            }
        }
    }

    fn apply_family(&mut self, family: RulesTypeFamily, pass: &IniFile) {
        let Some(members) = self.families.get_mut(&family) else {
            return;
        };
        for member in members {
            if let Some(section) = pass.section(&member.canonical_name) {
                member.body.overlay_rules_pass(section);
            }
        }
    }

    fn apply_family_and_collect(
        &mut self,
        family: RulesTypeFamily,
        pass: &IniFile,
        keys: &[&str],
    ) -> Vec<String> {
        let values = self.collect_current_values(family, pass, keys);
        self.apply_family(family, pass);
        values
    }

    fn collect_current_values(
        &self,
        family: RulesTypeFamily,
        pass: &IniFile,
        keys: &[&str],
    ) -> Vec<String> {
        let Some(members) = self.families.get(&family) else {
            return Vec::new();
        };
        let mut values = Vec::new();
        for member in members {
            let Some(section) = pass.section(&member.canonical_name) else {
                continue;
            };
            for key in keys {
                if let Some(value) = section.get(key) {
                    let value = trim_ascii_controls(value);
                    if !value.is_empty() {
                        values.push(value.to_string());
                    }
                }
            }
        }
        values
    }

    fn collect_current_list_values(
        &self,
        family: RulesTypeFamily,
        pass: &IniFile,
        keys: &[&str],
    ) -> Vec<String> {
        let Some(members) = self.families.get(&family) else {
            return Vec::new();
        };
        let mut values = Vec::new();
        for member in members {
            let Some(section) = pass.section(&member.canonical_name) else {
                continue;
            };
            for key in keys {
                if let Some(items) = section.get_list(key) {
                    values.extend(
                        items
                            .into_iter()
                            .filter(|item| !item.is_empty())
                            .map(str::to_string),
                    );
                }
            }
        }
        values
    }

    fn allocate_late_global_references(&mut self, pass: &IniFile) {
        if let Some(section) = pass.section("CombatDamage") {
            for key in ["DeathWeapon"] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::Weapon, value);
                }
            }
            for key in [
                "FlameDamage",
                "FlameDamage2",
                "C4Warhead",
                "CrushWarhead",
                "V3Warhead",
                "DMislWarhead",
                "V3EliteWarhead",
                "DMislEliteWarhead",
                "CMislWarhead",
                "CMislEliteWarhead",
                "IvanWarhead",
                "IonCannonWarhead",
            ] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::Warhead, value);
                }
            }
            for key in [
                "DefaultLargeGreySmokeSystem",
                "DefaultSmallGreySmokeSystem",
                "DefaultSparkSystem",
                "DefaultLargeRedSmokeSystem",
                "DefaultSmallRedSmokeSystem",
                "DefaultDebrisSmokeSystem",
                "DefaultFireStreamSystem",
                "DefaultTestParticleSystem",
                "DefaultRepairParticleSystem",
            ] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::ParticleSystem, value);
                }
            }
            for key in [
                "Scorches",
                "Scorches1",
                "Scorches2",
                "Scorches3",
                "Scorches4",
            ] {
                if let Some(values) = section.get_list(key) {
                    for value in values {
                        self.find_or_allocate(RulesTypeFamily::Smudge, value);
                    }
                }
            }
            if let Some(values) = section.get_list("SplashList") {
                for value in values {
                    self.find_or_allocate(RulesTypeFamily::Animation, value);
                }
            }
            for key in [
                "DrainAnimationType",
                "ControlledAnimationType",
                "PermaControlledAnimationType",
            ] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::Animation, value);
                }
            }
        }
        if let Some(section) = pass.section("Radiation")
            && let Some(value) = section.get("RadSiteWarhead")
        {
            self.find_or_allocate(RulesTypeFamily::Warhead, value);
        }
        if let Some(section) = pass.section("SpecialWeapons") {
            for key in [
                "NukeWarhead",
                "MutateWarhead",
                "MutateExplosionWarhead",
                "EMPulseWarhead",
            ] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::Warhead, value);
                }
            }
            for key in ["NukeProjectile", "NukeDown", "EMPulseProjectile"] {
                if let Some(value) = section.get(key) {
                    self.find_or_allocate(RulesTypeFamily::Projectile, value);
                }
            }
        }
        if let Some(section) = pass.section("CrateRules") {
            if let Some(value) = section.get("UnitCrateType") {
                self.find_or_allocate(RulesTypeFamily::Vehicle, value);
            }
            for key in ["WoodCrateImg", "CrateImg", "WaterCrateImg"] {
                if section.get(key).is_some() {
                    // Same caller capacity as the semantic CrateRules pass:
                    // ReadCrateRules @ 0x0066B956/0x0066B989/0x0066B9C7.
                    let value = section.read_string(key, "", 0x80);
                    self.find_or_allocate(RulesTypeFamily::Overlay, &value);
                }
            }
        }
    }

    fn finish(mut self) -> (IniFile, CrateRules) {
        let mut ini = self.ordinary.take().unwrap_or_else(IniFile::empty);

        for &(registry, family) in RULE_TYPE_FAMILIES {
            let mut section = IniSection::new(registry.to_string());
            if let Some(members) = self.families.get(&family) {
                for (index, member) in members.iter().enumerate() {
                    section.set(&index.to_string(), &member.canonical_name);
                }
            }
            ini.replace_first_section(section);
        }
        let mut tiberiums = IniSection::new("Tiberiums".to_string());
        for (index, member) in self.tiberiums.iter().enumerate() {
            tiberiums.set(&index.to_string(), &member.canonical_name);
        }
        ini.replace_first_section(tiberiums);
        if !self.prerequisite_groups.is_empty() {
            let general_index = if let Some(index) = ini.first_section.get("General").copied() {
                index
            } else {
                let index = ini.sections.len();
                ini.sections.push(IniSection::new("General".to_string()));
                ini.first_section.insert("General".to_string(), index);
                index
            };
            let general = &mut ini.sections[general_index];
            for (key, values) in self.prerequisite_groups {
                general.set(key, &values.join(","));
            }
        }

        let mut colors = IniSection::new("Colors".to_string());
        for (name, value) in self.colors {
            colors.set(&name, &value);
        }
        ini.replace_first_section(colors);

        // Replace every allocated type's ordinary text body with the keys that
        // its live object actually read after allocation.
        for &(_, family) in RULE_TYPE_FAMILIES {
            if let Some(members) = self.families.remove(&family) {
                for member in members {
                    ini.replace_first_section(member.body);
                }
            }
        }
        for family in [RulesTypeFamily::Weapon, RulesTypeFamily::Projectile] {
            if let Some(members) = self.families.remove(&family) {
                for member in members {
                    ini.replace_first_section(member.body);
                }
            }
        }
        for member in self.tiberiums {
            ini.replace_first_section(member.body);
        }

        (ini, self.crate_rules.finish())
    }
}

pub(crate) fn trim_ascii_controls(value: &str) -> &str {
    value.trim_matches(|character| u32::from(character) <= 0x20)
}

/// Whether a type-name reader resolves the input to native null.
///
/// `UnitTypeClass__FindOrAllocate @ 0x007480D0`, reached for
/// `UndeploysInto=` by `TechnoTypeClass__ReadINI @ 0x00712170` at
/// `0x0071329D..0x007132E4`, rejects these names before lookup/allocation.
pub(crate) fn is_native_none_type_name(value: &str) -> bool {
    let value = trim_ascii_controls(value);
    value.is_empty() || value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("<none>")
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match trim_ascii_controls(value)
        .bytes()
        .next()?
        .to_ascii_uppercase()
    {
        b'1' | b'T' | b'Y' => Some(true),
        b'0' | b'F' | b'N' => Some(false),
        _ => None,
    }
}

const RULE_TYPE_REGISTRIES: &[&str] = &[
    "InfantryTypes",
    "VehicleTypes",
    "AircraftTypes",
    "BuildingTypes",
    "TerrainTypes",
    "SmudgeTypes",
    "OverlayTypes",
    "Tiberiums",
    "SuperWeaponTypes",
    "Countries",
    "Animations",
    "VoxelAnims",
    "Warheads",
    "Particles",
    "ParticleSystems",
];

#[cfg(test)]
#[path = "ini_parser_tests.rs"]
mod tests;
