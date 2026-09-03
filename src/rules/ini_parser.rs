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
use crate::rules::powerups::{PowerupTable, PowerupsAccumulator};
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

    /// Value lookup that ignores key case. **VERA-internal — gamemd has no
    /// equivalent, and no new caller should be added.**
    ///
    /// gamemd's `INIClass` is case-SENSITIVE on both section names and entry
    /// names. It hashes the raw bytes on the store side
    /// (`INIClass::LoadFromStraw @ 0x00525A60`, raw-pointer CRC call at
    /// `0x005260D4`, whose only text transform is `strtrim @ 0x00727CF0` —
    /// bytes `<= 0x20` off both ends) and on the lookup side
    /// (`CCINIClass::ReadInt @ 0x005276D0`, CRC call at `0x00527727`), through
    /// the standard reflected CRC-32 in `CRCEngine::AddData @ 0x004A1DE0`
    /// (table at `0x0081F7B4`, poly `0xEDB88320`), and then compares 32-bit
    /// integers only — `INIClass::FindEntry_BinarySearch @ 0x0052B4F0` and
    /// `FindSection_BinarySearch @ 0x0052B620` never call `strcmp`. There is
    /// no folding instruction anywhere on the path and no string fallback.
    ///
    /// An earlier version of this comment claimed the opposite and cited
    /// `MaxDebris=` as the reason this helper exists. That was backwards: the
    /// 17 stock `[VehicleTypes]` spelling `Maxdebris=3` are invisible to
    /// gamemd and keep the constructor default of 0, so reading them was the
    /// divergence. Those call sites are now case-exact.
    ///
    /// Across the whole stock INI corpus exactly 6 authored key spellings
    /// disagree in case with gamemd's own literal: `Maxdebris` (17 sections),
    /// `JumpJetAccel` (8), `JumpJetTurnRate` (8), `Vshift` (9), `Fshift` (3)
    /// and `volume` (2) — 47 (section, key) pairs over 38 distinct sections.
    /// 0 section names disagree. The survivors here are `sound_ini.rs`'s 18
    /// call sites, reached by the three soundmd mis-spellings above: 14 of
    /// those 47 pairs, over 13 distinct sound events, because
    /// `[GrinderGrinding]` carries both `Fshift` and `Vshift`. They belong to
    /// the audio lane; this helper is deleted once those are converted.
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

    /// Apply the verified registry-allocation and per-pass body timing with an
    /// intentionally empty fixed-Art source.
    ///
    /// The returned INI is a compatibility projection for existing typed Rust
    /// readers. It contains the final live scalar values, unioned registries,
    /// and only the per-type keys read at or after that type was allocated.
    ///
    /// Production active-YR loading must call [`Self::process_with_fixed_art`]
    /// with the one selected ARTMD.INI snapshot. This convenience exists for
    /// synthetic Rules-only fixtures whose fixed-Art source is genuinely empty.
    pub fn process(&self) -> Result<ProcessedRulesLayers, RulesError> {
        self.process_with_fixed_art(&IniFile::empty())
    }

    /// Apply every Rules pass against the same fixed ARTMD.INI snapshot.
    ///
    /// Retail provenance: `Load_Game_Rules @ 0x0052CD70` selects ARTMD before
    /// the first `RulesClass::Process @ 0x00668BF0`; `ReadTypeData @ 0x00679A10`
    /// then reuses global `g_ArtINI @ 0x00887180` on every later pass.
    pub fn process_with_fixed_art(
        &self,
        fixed_art: &IniFile,
    ) -> Result<ProcessedRulesLayers, RulesError> {
        self.process_with_fixed_art_and_registry_state(
            fixed_art,
            NativeRulesRegistryState::default(),
        )
    }

    /// Continue Process calls against an already-live process registry.
    ///
    /// Shell preview and gameplay Full_Init share native Type registries even
    /// though their numeric-ID cursors have distinct reset rules. This move-only
    /// input prevents a second Rust registry authority from being synthesized.
    pub(crate) fn process_with_fixed_art_and_registry_state(
        &self,
        fixed_art: &IniFile,
        registry_state: NativeRulesRegistryState,
    ) -> Result<ProcessedRulesLayers, RulesError> {
        self.process_with_fixed_art_and_registry_state_recovering(fixed_art, registry_state)
            .map_err(NativeRulesProcessingFailure::into_error)
    }

    /// Continue Process calls while retaining the sole native registry owner
    /// when a later pass fails.
    ///
    /// Native `RulesClass::Process @ 0x00668BF0` is not transactional. A
    /// Tiberium failure can occur after earlier constructors in the same pass
    /// have already mutated the live registries. Process-lifetime callers must
    /// therefore recover that partial state instead of rolling back to the
    /// pre-call owner or synthesizing a fresh registry from the compatibility
    /// projection.
    pub(crate) fn process_with_fixed_art_and_registry_state_recovering(
        &self,
        fixed_art: &IniFile,
        registry_state: NativeRulesRegistryState,
    ) -> Result<ProcessedRulesLayers, NativeRulesProcessingFailure> {
        let mut processor = RulesPassProcessor::with_registry_state(registry_state);
        for (_, ini) in self.iter_passes() {
            if let Err(error) = processor.apply_pass(ini, fixed_art) {
                let (_, partial_trace, _, _) = processor.finish();
                return Err(NativeRulesProcessingFailure {
                    error,
                    partial_trace,
                });
            }
        }
        let (ini, native_type_construction_trace, crate_rules, powerups) = processor.finish();
        Ok(ProcessedRulesLayers {
            ini,
            crate_rules,
            powerups,
            content_hash: self.content_hash(),
            native_type_construction_trace,
        })
    }
}

/// A failed native Process call plus the state already mutated before failure.
#[derive(Debug)]
pub(crate) struct NativeRulesProcessingFailure {
    error: RulesError,
    partial_trace: NativeTypeConstructionTrace,
}

impl NativeRulesProcessingFailure {
    pub(crate) fn into_parts(self) -> (RulesError, NativeTypeConstructionTrace) {
        (self.error, self.partial_trace)
    }

    fn into_error(self) -> RulesError {
        self.error
    }
}

/// Reproduce the Type-constructor portion of active YR's cold rules startup.
///
/// This is deliberately not a `RulesClass::Process` call for the selected
/// RULESMD root. `Load_Game_Rules @ 0x0052CD70` first runs only
/// `ReadAudioVisual(root)`, may then run one full Process for optional
/// `LANGRULE.INI`, and `Init_Game @ 0x0052BA60` follows with the root Anim and
/// Building master/body sweeps. Both body loops reload their live family count.
///
/// The input state makes the native direct-repeat behavior explicit. The real
/// cold call supplies [`NativeRulesRegistryState::default`]; a repeat without a
/// destructive reset continues the retained registries and emits only new
/// successful constructor events.
pub(crate) fn process_native_rules_cold_start(
    registry_state: NativeRulesRegistryState,
    selected_rules_root: &IniFile,
    fixed_art: &IniFile,
    langrule: Option<&IniFile>,
) -> Result<NativeTypeConstructionTrace, RulesError> {
    process_native_rules_cold_start_inner(
        registry_state,
        selected_rules_root,
        fixed_art,
        langrule,
    )
    .map(|(trace, _phase_event_counts)| trace)
}

/// Shared production/test implementation. Counts are cumulative boundaries
/// after AudioVisual, Anim master, Anim bodies, Building master, and Building
/// bodies respectively; retaining them here keeps the stock oracle tied to the
/// exact production sequence instead of duplicating that sequence in a test.
fn process_native_rules_cold_start_inner(
    registry_state: NativeRulesRegistryState,
    selected_rules_root: &IniFile,
    fixed_art: &IniFile,
    langrule: Option<&IniFile>,
) -> Result<(NativeTypeConstructionTrace, [usize; 5]), RulesError> {
    let mut processor = RulesPassProcessor::with_registry_state(registry_state);
    processor.allocate_audio_visual_references(selected_rules_root);
    let after_audio_visual = processor.native_type_construction_events.len();
    if let Some(langrule) = langrule {
        processor.apply_pass(langrule, fixed_art)?;
    }
    processor.allocate_explicit_family(
        selected_rules_root,
        "Animations",
        RulesTypeFamily::Animation,
    );
    let after_animation_master = processor.native_type_construction_events.len();
    processor.process_anim_family(fixed_art);
    let after_animation_bodies = processor.native_type_construction_events.len();
    processor.allocate_explicit_family(
        selected_rules_root,
        "BuildingTypes",
        RulesTypeFamily::Building,
    );
    let after_building_master = processor.native_type_construction_events.len();
    processor.process_techno_family(
        RulesTypeFamily::Building,
        selected_rules_root,
        fixed_art,
    );
    let after_building_bodies = processor.native_type_construction_events.len();
    let (_, trace, _, _) = processor.finish();
    Ok((
        trace,
        [
            after_audio_visual,
            after_animation_master,
            after_animation_bodies,
            after_building_master,
            after_building_bodies,
        ],
    ))
}

/// Run the constructor-capable noncampaign prefix before Full_Init's rules
/// destruction boundary.
///
/// Active YR `ScenarioClass::Full_Init @ 0x00686B20` performs the root
/// Countries master, root General references, then a live HouseType body loop
/// against the process-retained startup registries. The returned event vector
/// is therefore only `E_multi`; the caller must already have drained the older
/// cold-start events while retaining their registry state.
pub(crate) fn process_native_noncampaign_rules_prepass(
    registry_state: NativeRulesRegistryState,
    selected_rules_root: &IniFile,
) -> NativeTypeConstructionTrace {
    process_native_noncampaign_rules_prepass_inner(registry_state, selected_rules_root).0
}

/// Shared production/test implementation. Counts are cumulative `E_multi`
/// boundaries after Countries, General, and the live HouseType body loop.
fn process_native_noncampaign_rules_prepass_inner(
    registry_state: NativeRulesRegistryState,
    selected_rules_root: &IniFile,
) -> (NativeTypeConstructionTrace, [usize; 3]) {
    let mut processor = RulesPassProcessor::with_registry_state(registry_state);
    processor.allocate_explicit_family(
        selected_rules_root,
        "Countries",
        RulesTypeFamily::Country,
    );
    let after_countries = processor.native_type_construction_events.len();
    processor.allocate_general_references(selected_rules_root);
    let after_general = processor.native_type_construction_events.len();
    processor.process_house_family(selected_rules_root);
    let after_house_bodies = processor.native_type_construction_events.len();
    let (_, trace, _, _) = processor.finish();
    (
        trace,
        [after_countries, after_general, after_house_bodies],
    )
}

/// Result of applying an ordered rules stack.
#[derive(Debug)]
pub struct ProcessedRulesLayers {
    ini: IniFile,
    crate_rules: CrateRules,
    powerups: PowerupTable,
    content_hash: u64,
    native_type_construction_trace: NativeTypeConstructionTrace,
}

impl ProcessedRulesLayers {
    pub fn ini(&self) -> &IniFile {
        &self.ini
    }

    /// Consume only the typed-reader compatibility projection and deliberately
    /// discard the native constructor/registry receipt.
    ///
    /// Gameplay-equivalent fresh loads must use
    /// [`Self::into_ini_and_native_type_construction_trace`] instead.
    pub(crate) fn into_projection_discarding_native_receipt(self) -> IniFile {
        self.ini
    }

    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    pub fn crate_rules(&self) -> &CrateRules {
        &self.crate_rules
    }

    pub fn powerups(&self) -> &PowerupTable {
        &self.powerups
    }

    pub(crate) fn native_type_construction_trace(&self) -> &NativeTypeConstructionTrace {
        &self.native_type_construction_trace
    }

    pub(crate) fn into_ini_and_native_type_construction_trace(
        self,
    ) -> (IniFile, NativeTypeConstructionTrace) {
        (self.ini, self.native_type_construction_trace)
    }
}

/// One active-YR Type constructor family whose constructor calls
/// `AbstractClass::AssignUniqueID @ 0x00410230`.
///
/// `ParticleTypeClass` is deliberately absent: its constructor has no Assign
/// call. Script/Team/TaskForce/Trigger/Tag/Tiberium types are absent for the
/// same reason. The family label records the native constructor, not the INI
/// section spelling (`Countries` constructs `HouseType`, for example).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeTypeConstructorFamily {
    HouseType,
    Side,
    OverlayType,
    SuperWeaponType,
    WarheadType,
    SmudgeType,
    TerrainType,
    BuildingType,
    UnitType,
    AircraftType,
    InfantryType,
    AnimType,
    VoxelAnimType,
    ParticleSystemType,
    WeaponType,
    BulletType,
}

/// One successful Type construction in native process order.
///
/// This is normally a first-new-name event, but inputs longer than the native
/// 24-byte stored ID can repeatedly miss lookup and emit duplicate stored IDs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NativeTypeConstructionEvent {
    family: NativeTypeConstructorFamily,
    native_stored_id: String,
}

impl NativeTypeConstructionEvent {
    pub(crate) fn family(&self) -> NativeTypeConstructorFamily {
        self.family
    }

    pub(crate) fn native_stored_id(&self) -> &str {
        &self.native_stored_id
    }
}

/// Move-only receipt for the exact Type-ID prefix emitted by an ordered Rules
/// stack, plus the allocated SuperWeaponType count needed by later House
/// constructor blocks.
///
/// This is chronology, not a final-registry recount. It must therefore travel
/// with the processed Rules result and may not be cloned into a second prefix
/// authority.
#[derive(Debug)]
pub(crate) struct NativeTypeConstructionTrace {
    events: Vec<NativeTypeConstructionEvent>,
    allocated_super_weapon_type_count: usize,
    registry_state: NativeRulesRegistryState,
}

impl NativeTypeConstructionTrace {
    pub(crate) fn events(&self) -> &[NativeTypeConstructionEvent] {
        &self.events
    }

    pub(crate) fn event_count(&self) -> usize {
        self.events.len()
    }

    pub(crate) fn allocated_super_weapon_type_count(&self) -> usize {
        self.allocated_super_weapon_type_count
    }

    pub(crate) fn registry_state(&self) -> &NativeRulesRegistryState {
        &self.registry_state
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<NativeTypeConstructionEvent>,
        usize,
        NativeRulesRegistryState,
    ) {
        (
            self.events,
            self.allocated_super_weapon_type_count,
            self.registry_state,
        )
    }

    /// Drain constructor history at a native numeric-ID reset while retaining
    /// the one live Type-registry authority for the next lookup pass.
    ///
    /// Cold startup events predate Full_Init's `Clear_Scene` reset. They affect
    /// `E_multi` duplicate suppression but must not be charged to the fresh
    /// Scenario cursor.
    pub(crate) fn into_registry_state_discarding_events(self) -> NativeRulesRegistryState {
        self.registry_state
    }
}

/// Process-resident live Type registries after one or more Rules passes.
///
/// The vectors are ordered native stored IDs plus the bodies that have actually
/// been read so far. The receipt is deliberately move-only: preview, Start, and
/// fresh Full_Init must hand off one authority instead of recounting a merged
/// INI. Tiberium slots are included even though their constructors spend no ID.
#[derive(Debug, Default)]
pub(crate) struct NativeRulesRegistryState {
    families: HashMap<RulesTypeFamily, Vec<ProcessedType>>,
    tiberiums: Vec<ProcessedType>,
}

impl NativeRulesRegistryState {
    pub(crate) fn family_len(&self, family: NativeTypeConstructorFamily) -> usize {
        self.families
            .iter()
            .find_map(|(rules_family, members)| {
                (rules_family.native_constructor_family() == Some(family)).then_some(members.len())
            })
            .unwrap_or(0)
    }

    pub(crate) fn tiberium_slot_count(&self) -> usize {
        self.tiberiums.len()
    }

    /// Consume the pre-reset registry owner at Full_Init's destructive Rules
    /// reset and return a genuinely empty post-reset owner.
    ///
    /// Numeric-ID history is intentionally not represented here and therefore
    /// cannot be rewound by this operation.
    pub(crate) fn destructive_reset(self) -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RulesTypeFamily {
    Country,
    Side,
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

/// Explicit registry order in `RulesClass::Process @ 0x00668BF0`.
///
/// `Sides` is special: `FUN_00672440` passes the entry name to
/// `SideClass::Constructor @ 0x006A4550`; all other rows read a 32-byte value.
const EXPLICIT_RULE_TYPE_FAMILIES: &[(&str, RulesTypeFamily)] = &[
    ("Countries", RulesTypeFamily::Country),
    ("Sides", RulesTypeFamily::Side),
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

/// Families rebuilt into the compatibility INI projection. `Sides` remains
/// an ordinary section there because its values are country membership lists,
/// not an index-to-Type registry.
const PROJECTED_RULE_TYPE_FAMILIES: &[(&str, RulesTypeFamily)] = &[
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

impl RulesTypeFamily {
    fn native_constructor_family(self) -> Option<NativeTypeConstructorFamily> {
        Some(match self {
            Self::Country => NativeTypeConstructorFamily::HouseType,
            Self::Side => NativeTypeConstructorFamily::Side,
            Self::Overlay => NativeTypeConstructorFamily::OverlayType,
            Self::SuperWeapon => NativeTypeConstructorFamily::SuperWeaponType,
            Self::Warhead => NativeTypeConstructorFamily::WarheadType,
            Self::Smudge => NativeTypeConstructorFamily::SmudgeType,
            Self::Terrain => NativeTypeConstructorFamily::TerrainType,
            Self::Building => NativeTypeConstructorFamily::BuildingType,
            Self::Vehicle => NativeTypeConstructorFamily::UnitType,
            Self::Aircraft => NativeTypeConstructorFamily::AircraftType,
            Self::Infantry => NativeTypeConstructorFamily::InfantryType,
            Self::Animation => NativeTypeConstructorFamily::AnimType,
            Self::VoxelAnimation => NativeTypeConstructorFamily::VoxelAnimType,
            Self::Particle => return None,
            Self::ParticleSystem => NativeTypeConstructorFamily::ParticleSystemType,
            Self::Weapon => NativeTypeConstructorFamily::WeaponType,
            Self::Projectile => NativeTypeConstructorFamily::BulletType,
        })
    }
}

#[derive(Debug, Clone)]
struct ProcessedType {
    native_stored_id: String,
    body: IniSection,
}

impl ProcessedType {
    fn new(native_stored_id: String) -> Self {
        Self {
            body: IniSection::new(native_stored_id.clone()),
            native_stored_id,
        }
    }
}

#[derive(Debug, Default)]
struct RulesPassProcessor {
    ordinary: Option<IniFile>,
    crate_rules: CrateRulesAccumulator,
    powerups: PowerupsAccumulator,
    families: HashMap<RulesTypeFamily, Vec<ProcessedType>>,
    native_type_construction_events: Vec<NativeTypeConstructionEvent>,
    tiberiums: Vec<ProcessedType>,
    colors: Vec<(String, String)>,
    prerequisite_groups: HashMap<&'static str, Vec<String>>,
}

impl RulesPassProcessor {
    fn with_registry_state(registry_state: NativeRulesRegistryState) -> Self {
        Self {
            families: registry_state.families,
            tiberiums: registry_state.tiberiums,
            ..Self::default()
        }
    }

    fn apply_pass(&mut self, pass: &IniFile, fixed_art: &IniFile) -> Result<(), RulesError> {
        if let Some(ordinary) = self.ordinary.as_mut() {
            ordinary.merge_rules_projection(pass);
        } else {
            let mut ordinary = IniFile::empty();
            ordinary.merge_rules_projection(pass);
            self.ordinary = Some(ordinary);
        }

        // Exact constructor-capable order from `RulesClass::Process @
        // 0x00668BF0`. Colors precede every Type registry but spend no Type ID.
        self.allocate_colors(pass);
        for &(registry, family) in EXPLICIT_RULE_TYPE_FAMILIES {
            self.allocate_explicit_family(pass, registry, family);
        }

        // JumpjetControls and MultiplayerSettings contain no Type factory.
        self.allocate_ai_references(pass);
        self.read_prerequisite_groups(pass);
        self.allocate_general_references(pass);
        self.process_type_data(pass, fixed_art);

        // Difficulty readers contain no Type factories.
        self.allocate_crate_references(pass);
        // ReadCrateRules @ 0x0066B900 reads the semantic crate values in the
        // same Process pass; it allocates no Type and spends no ID.
        self.crate_rules.apply_pass(pass);
        self.powerups.apply_pass(pass);
        self.allocate_combat_references(pass);
        self.allocate_radiation_references(pass);
        // Elevation and Wall contain no Type factories.
        self.allocate_audio_visual_references(pass);
        self.process_special_weapons(pass);
        self.process_tiberiums(pass)?;
        // AdvancedCommandBar contains no Type factory.
        Ok(())
    }

    fn allocate_explicit_family(
        &mut self,
        pass: &IniFile,
        registry: &str,
        family: RulesTypeFamily,
    ) {
        let Some(section) = pass.section(registry) else {
            return;
        };
        for key in section.keys() {
            let identity = if family == RulesTypeFamily::Side {
                key.to_string()
            } else {
                section.read_string(key, "", 32)
            };
            if !identity.is_empty() {
                self.find_or_allocate(family, &identity);
            }
        }
    }

    fn family_mut(&mut self, family: RulesTypeFamily) -> &mut Vec<ProcessedType> {
        self.families.entry(family).or_default()
    }

    fn find_or_allocate(&mut self, family: RulesTypeFamily, incoming: &str) -> Option<usize> {
        // Caller-specific ReadString buffers trim the whole scalar before this
        // boundary. List tokens deliberately arrive untrimmed. The factory
        // must therefore inspect the exact incoming string, not trim again.
        if incoming.is_empty()
            || (family != RulesTypeFamily::Side && is_exact_native_none_type_name(incoming))
        {
            return None;
        }
        let members = self.family_mut(family);
        if let Some(index) = members
            .iter()
            .position(|member| member.native_stored_id.eq_ignore_ascii_case(incoming))
        {
            return Some(index);
        }
        // AbstractTypeClass::Constructor @ 0x00410800 stores only 0x18 bytes.
        // Lookup above compares that stored ID against the full input, so a
        // repeated >24-byte spelling can construct another equal stored ID.
        let native_stored_id = incoming.chars().take(0x18).collect::<String>();
        let index = members.len();
        members.push(ProcessedType::new(native_stored_id.clone()));
        if let Some(family) = family.native_constructor_family() {
            self.native_type_construction_events
                .push(NativeTypeConstructionEvent {
                    family,
                    native_stored_id,
                });
        }
        Some(index)
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

    fn allocate_scalar_from(
        &mut self,
        section: &IniSection,
        key: &str,
        family: RulesTypeFamily,
        capacity: usize,
    ) {
        if section.get(key).is_none() {
            return;
        }
        let incoming = section.read_string(key, "", capacity);
        if !incoming.is_empty() {
            self.find_or_allocate(family, &incoming);
        }
    }

    fn allocate_list_from(
        &mut self,
        section: &IniSection,
        key: &str,
        family: RulesTypeFamily,
        capacity: usize,
    ) {
        if section.get(key).is_none() {
            return;
        }
        let incoming = section.read_string(key, "", capacity);
        for token in native_strtok_comma_tokens(&incoming) {
            self.find_or_allocate(family, token);
        }
    }

    fn lookup_existing(&self, family: RulesTypeFamily, incoming: &str) -> Option<String> {
        if incoming.is_empty()
            || (family != RulesTypeFamily::Side && is_exact_native_none_type_name(incoming))
        {
            return None;
        }
        self.families.get(&family)?.iter().find_map(|member| {
            member
                .native_stored_id
                .eq_ignore_ascii_case(incoming)
                .then(|| member.native_stored_id.clone())
        })
    }

    fn allocate_ai_references(&mut self, pass: &IniFile) {
        const BUILDING_LISTS: &[&str] = &[
            "BuildConst",
            "BuildPower",
            "BuildRefinery",
            "BuildBarracks",
            "BuildTech",
            "BuildWeapons",
            "AlliedBaseDefenses",
            "SovietBaseDefenses",
            "ThirdBaseDefenses",
            "BuildDefense",
            "BuildPDefense",
            "BuildAA",
            "BuildHelipad",
            "BuildRadar",
            "ConcreteWalls",
            "NSGates",
            "EWGates",
            "BuildNavalYard",
            "BuildDummy",
            "NeutralTechBuildings",
        ];

        let Some(section) = pass.section("AI") else {
            return;
        };
        for key in BUILDING_LISTS {
            self.allocate_list_from(section, key, RulesTypeFamily::Building, 0x80);
        }
    }

    fn allocate_general_references(&mut self, pass: &IniFile) {
        const SITES: &[(&str, RulesTypeFamily, bool)] = &[
            ("DamageFireTypes", RulesTypeFamily::Animation, true),
            ("OreTwinkle", RulesTypeFamily::Animation, false),
            ("BarrelExplode", RulesTypeFamily::Animation, false),
            ("BarrelDebris", RulesTypeFamily::VoxelAnimation, true),
            ("BarrelParticle", RulesTypeFamily::ParticleSystem, false),
            ("NukeTakeOff", RulesTypeFamily::Animation, false),
            ("Wake", RulesTypeFamily::Animation, false),
            ("DropPod", RulesTypeFamily::Animation, true),
            ("DeadBodies", RulesTypeFamily::Animation, true),
            ("MetallicDebris", RulesTypeFamily::Animation, true),
            ("BridgeExplosions", RulesTypeFamily::Animation, true),
            ("IonBlast", RulesTypeFamily::Animation, false),
            ("IonBeam", RulesTypeFamily::Animation, false),
            ("WeatherConClouds", RulesTypeFamily::Animation, true),
            ("WeatherConBolts", RulesTypeFamily::Animation, true),
            (
                "WeatherConBoltExplosion",
                RulesTypeFamily::Animation,
                false,
            ),
            ("DominatorWarhead", RulesTypeFamily::Warhead, false),
            ("DominatorFirstAnim", RulesTypeFamily::Animation, false),
            ("DominatorSecondAnim", RulesTypeFamily::Animation, false),
            ("ChronoPlacement", RulesTypeFamily::Animation, false),
            ("ChronoBeam", RulesTypeFamily::Animation, false),
            ("ChronoBlast", RulesTypeFamily::Animation, false),
            ("ChronoBlastDest", RulesTypeFamily::Animation, false),
            ("WarpIn", RulesTypeFamily::Animation, false),
            ("WarpOut", RulesTypeFamily::Animation, false),
            ("WarpAway", RulesTypeFamily::Animation, false),
            (
                "IronCurtainInvokeAnim",
                RulesTypeFamily::Animation,
                false,
            ),
            (
                "ForceShieldInvokeAnim",
                RulesTypeFamily::Animation,
                false,
            ),
            ("WeaponNullifyAnim", RulesTypeFamily::Animation, false),
            ("ChronoSparkle1", RulesTypeFamily::Animation, false),
            ("InfantryExplode", RulesTypeFamily::Animation, false),
            ("FlamingInfantry", RulesTypeFamily::Animation, false),
            ("InfantryHeadPop", RulesTypeFamily::Animation, false),
            ("InfantryNuked", RulesTypeFamily::Animation, false),
            ("InfantryVirus", RulesTypeFamily::Animation, false),
            ("InfantryBrute", RulesTypeFamily::Animation, false),
            ("InfantryMutate", RulesTypeFamily::Animation, false),
            ("Behind", RulesTypeFamily::Animation, false),
            ("MoveFlash", RulesTypeFamily::Animation, false),
            ("Parachute", RulesTypeFamily::Animation, false),
            ("BombParachute", RulesTypeFamily::Animation, false),
            ("DropZoneAnim", RulesTypeFamily::Animation, false),
            ("EMPulseSparkles", RulesTypeFamily::Animation, false),
            ("LargeVisceroid", RulesTypeFamily::Vehicle, false),
            ("SmallVisceroid", RulesTypeFamily::Vehicle, false),
            ("DropPodWeapon", RulesTypeFamily::Weapon, false),
            (
                "ExplosiveVoxelDebris",
                RulesTypeFamily::VoxelAnimation,
                true,
            ),
            ("TireVoxelDebris", RulesTypeFamily::VoxelAnimation, false),
            ("ScrapVoxelDebris", RulesTypeFamily::VoxelAnimation, false),
            ("RepairBay", RulesTypeFamily::Building, true),
            ("GDIGateOne", RulesTypeFamily::Building, false),
            ("GDIGateTwo", RulesTypeFamily::Building, false),
            ("NodGateOne", RulesTypeFamily::Building, false),
            ("NodGateTwo", RulesTypeFamily::Building, false),
            ("WallTower", RulesTypeFamily::Building, false),
            ("Shipyard", RulesTypeFamily::Building, true),
            ("GDIPowerPlant", RulesTypeFamily::Building, false),
            ("NodRegularPower", RulesTypeFamily::Building, false),
            ("NodAdvancedPower", RulesTypeFamily::Building, false),
            ("ThirdPowerPlant", RulesTypeFamily::Building, false),
            (
                "PrerequisiteProcAlternate",
                RulesTypeFamily::Vehicle,
                false,
            ),
            ("BaseUnit", RulesTypeFamily::Vehicle, true),
            ("HarvesterUnit", RulesTypeFamily::Vehicle, true),
            ("PadAircraft", RulesTypeFamily::Aircraft, true),
            ("Paratrooper", RulesTypeFamily::Infantry, false),
            ("SecretInfantry", RulesTypeFamily::Infantry, true),
            ("SecretUnits", RulesTypeFamily::Vehicle, true),
            ("SecretBuildings", RulesTypeFamily::Building, true),
            ("AlliedDisguise", RulesTypeFamily::Infantry, false),
            ("SovietDisguise", RulesTypeFamily::Infantry, false),
            ("ThirdDisguise", RulesTypeFamily::Infantry, false),
            ("Engineer", RulesTypeFamily::Infantry, false),
            ("Technician", RulesTypeFamily::Infantry, false),
            ("Pilot", RulesTypeFamily::Infantry, false),
            ("AlliedCrew", RulesTypeFamily::Infantry, false),
            ("SovietCrew", RulesTypeFamily::Infantry, false),
            ("ThirdCrew", RulesTypeFamily::Infantry, false),
            ("AmerParaDropInf", RulesTypeFamily::Infantry, true),
            ("AllyParaDropInf", RulesTypeFamily::Infantry, true),
            ("SovParaDropInf", RulesTypeFamily::Infantry, true),
            ("YuriParaDropInf", RulesTypeFamily::Infantry, true),
            ("AnimToInfantry", RulesTypeFamily::Infantry, true),
            ("LightningWarhead", RulesTypeFamily::Warhead, false),
            ("PrismType", RulesTypeFamily::Building, false),
            ("V3RocketType", RulesTypeFamily::Aircraft, false),
            ("DMislType", RulesTypeFamily::Aircraft, false),
            ("CMislType", RulesTypeFamily::Aircraft, false),
            ("VeinholeTypeClass", RulesTypeFamily::Terrain, false),
            (
                "DefaultMirageDisguises",
                RulesTypeFamily::Terrain,
                true,
            ),
        ];

        let Some(section) = pass.section("General") else {
            return;
        };
        for &(key, family, is_list) in SITES {
            if is_list {
                self.allocate_list_from(section, key, family, 0x80);
            } else {
                self.allocate_scalar_from(section, key, family, 0x80);
            }
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
            if general.get(key).is_none() {
                continue;
            }
            let raw = general.read_string(key, "", 0x80);
            let resolved = native_strtok_comma_tokens(&raw)
                .filter_map(|identity| self.lookup_existing(RulesTypeFamily::Building, identity))
                .collect();
            self.prerequisite_groups.insert(key, resolved);
        }
    }

    fn family_len(&self, family: RulesTypeFamily) -> usize {
        self.families.get(&family).map_or(0, Vec::len)
    }

    fn begin_rules_member_read(
        &mut self,
        family: RulesTypeFamily,
        index: usize,
        pass: &IniFile,
    ) -> Option<(String, IniSection, IniSection)> {
        let native_stored_id = self
            .families
            .get(&family)?
            .get(index)?
            .native_stored_id
            .clone();
        let raw = pass.section(&native_stored_id)?.clone();
        let member = self.families.get_mut(&family)?.get_mut(index)?;
        member.body.overlay_rules_pass(&raw);
        Some((native_stored_id, raw, member.body.clone()))
    }

    fn process_type_data(&mut self, pass: &IniFile, fixed_art: &IniFile) {
        self.process_house_family(pass);
        self.process_super_weapon_family(pass);
        self.process_anim_family(fixed_art);
        self.process_techno_family(RulesTypeFamily::Building, pass, fixed_art);
        self.process_techno_family(RulesTypeFamily::Aircraft, pass, fixed_art);
        self.process_techno_family(RulesTypeFamily::Vehicle, pass, fixed_art);
        self.process_techno_family(RulesTypeFamily::Infantry, pass, fixed_art);
        self.process_weapon_family(pass);
        self.process_bullet_family(pass, fixed_art);
        self.process_warhead_family(pass);
        // Weapon post and Building post add no Type references.
        self.process_plain_family(RulesTypeFamily::Terrain, pass);
        self.process_plain_family(RulesTypeFamily::Smudge, pass);
        self.process_plain_family(RulesTypeFamily::Overlay, pass);
        self.process_particle_family(pass);
        self.process_particle_system_family(pass);
        self.process_voxel_anim_family(pass);
        // MissionControl adds no Type references.
    }

    fn process_house_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Country) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::Country, index, pass)
            {
                self.allocate_list_from(
                    &raw,
                    "VeteranInfantry",
                    RulesTypeFamily::Infantry,
                    0x80,
                );
                self.allocate_list_from(
                    &raw,
                    "VeteranUnits",
                    RulesTypeFamily::Vehicle,
                    0x80,
                );
                self.allocate_list_from(
                    &raw,
                    "VeteranAircraft",
                    RulesTypeFamily::Aircraft,
                    0x80,
                );
                self.allocate_scalar_from(&raw, "Side", RulesTypeFamily::Side, 0x80);
            }
            index += 1;
        }
    }

    fn process_super_weapon_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::SuperWeapon) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::SuperWeapon, index, pass)
            {
                self.allocate_scalar_from(&raw, "WeaponType", RulesTypeFamily::Weapon, 0x80);
                self.allocate_scalar_from(&raw, "AuxBuilding", RulesTypeFamily::Building, 0x80);
            }
            index += 1;
        }
    }

    fn process_anim_family(&mut self, fixed_art: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Animation) {
            let native_stored_id = self
                .families
                .get(&RulesTypeFamily::Animation)
                .and_then(|members| members.get(index))
                .map(|member| member.native_stored_id.clone());
            if let Some(section) = native_stored_id
                .as_deref()
                .and_then(|identity| fixed_art.section(identity))
            {
                for key in ["Next", "Spawns"] {
                    self.allocate_scalar_from(section, key, RulesTypeFamily::Animation, 0x80);
                }
                self.allocate_scalar_from(
                    section,
                    "TiberiumSpawnType",
                    RulesTypeFamily::Overlay,
                    0x80,
                );
                for key in ["BounceAnim", "ExpireAnim", "TrailerAnim"] {
                    self.allocate_scalar_from(section, key, RulesTypeFamily::Animation, 0x80);
                }
                self.allocate_scalar_from(section, "Warhead", RulesTypeFamily::Warhead, 0x80);
                self.allocate_scalar_from(
                    section,
                    "SpawnsParticle",
                    RulesTypeFamily::Particle,
                    0x20,
                );
            }
            index += 1;
        }
    }

    fn process_techno_family(
        &mut self,
        family: RulesTypeFamily,
        pass: &IniFile,
        fixed_art: &IniFile,
    ) {
        let mut index = 0;
        while index < self.family_len(family) {
            if let Some((native_stored_id, raw, effective)) =
                self.begin_rules_member_read(family, index, pass)
            {
                self.process_techno_base(&raw, &effective);
                match family {
                    RulesTypeFamily::Building => {
                        self.allocate_scalar_from(
                            &raw,
                            "FreeUnit",
                            RulesTypeFamily::Vehicle,
                            0x80,
                        );
                        self.allocate_scalar_from(
                            &raw,
                            "SecretInfantry",
                            RulesTypeFamily::Infantry,
                            0x80,
                        );
                        self.allocate_scalar_from(
                            &raw,
                            "SecretUnit",
                            RulesTypeFamily::Vehicle,
                            0x80,
                        );
                        self.allocate_scalar_from(
                            &raw,
                            "SecretBuilding",
                            RulesTypeFamily::Building,
                            0x80,
                        );
                        self.allocate_fixed_art_techno_reference(
                            fixed_art,
                            &native_stored_id,
                            &effective,
                            "ToOverlay",
                            RulesTypeFamily::Overlay,
                        );
                    }
                    RulesTypeFamily::Aircraft => {
                        self.allocate_fixed_art_techno_reference(
                            fixed_art,
                            &native_stored_id,
                            &effective,
                            "Trailer",
                            RulesTypeFamily::Animation,
                        );
                    }
                    RulesTypeFamily::Vehicle => {}
                    RulesTypeFamily::Infantry => {
                        self.allocate_scalar_from(
                            &raw,
                            "OccupyWeapon",
                            RulesTypeFamily::Weapon,
                            0x80,
                        );
                        self.allocate_scalar_from(
                            &raw,
                            "EliteOccupyWeapon",
                            RulesTypeFamily::Weapon,
                            0x80,
                        );
                        self.allocate_list_from(
                            &raw,
                            "DeadBodies",
                            RulesTypeFamily::Animation,
                            0x80,
                        );
                        self.allocate_list_from(
                            &raw,
                            "DeathAnims",
                            RulesTypeFamily::Animation,
                            0x80,
                        );
                    }
                    _ => unreachable!("only Techno families enter the Techno reader"),
                }
            }
            index += 1;
        }
    }

    fn process_techno_base(&mut self, raw: &IniSection, effective: &IniSection) {
        self.allocate_scalar_from(raw, "DeathWeapon", RulesTypeFamily::Weapon, 0x80);
        self.allocate_list_from(raw, "DebrisTypes", RulesTypeFamily::VoxelAnimation, 0x80);
        self.allocate_list_from(raw, "DebrisAnims", RulesTypeFamily::Animation, 0x80);

        let turret_count = effective.read_int("TurretCount", 0);
        let weapon_count = effective.read_int("WeaponCount", 0);
        let clear_all_weapons = effective.read_bool("ClearAllWeapons", false);
        if turret_count >= 1 && weapon_count > 0 {
            for slot in 1..=weapon_count {
                self.allocate_scalar_from(
                    raw,
                    &format!("Weapon{slot}"),
                    RulesTypeFamily::Weapon,
                    0x80,
                );
                self.allocate_scalar_from(
                    raw,
                    &format!("EliteWeapon{slot}"),
                    RulesTypeFamily::Weapon,
                    0x80,
                );
            }
        } else if turret_count < 1 && !clear_all_weapons {
            for key in ["Primary", "Secondary", "ElitePrimary", "EliteSecondary"] {
                self.allocate_scalar_from(raw, key, RulesTypeFamily::Weapon, 0x80);
            }
        }

        self.allocate_list_from(raw, "Dock", RulesTypeFamily::Building, 0x80);
        self.allocate_scalar_from(raw, "DeploysInto", RulesTypeFamily::Building, 0x80);
        self.allocate_scalar_from(raw, "UndeploysInto", RulesTypeFamily::Vehicle, 0x80);
        self.allocate_scalar_from(raw, "PowersUnit", RulesTypeFamily::Vehicle, 0x80);
        self.allocate_list_from(raw, "Explosion", RulesTypeFamily::Animation, 0x80);
        self.allocate_list_from(raw, "DestroyAnim", RulesTypeFamily::Animation, 0x80);
        self.allocate_scalar_from(
            raw,
            "NaturalParticleSystem",
            RulesTypeFamily::ParticleSystem,
            0x80,
        );
        self.allocate_scalar_from(
            raw,
            "RefinerySmokeParticleSystem",
            RulesTypeFamily::ParticleSystem,
            0x80,
        );
        self.allocate_list_from(
            raw,
            "DamageParticleSystems",
            RulesTypeFamily::ParticleSystem,
            0x80,
        );
        self.allocate_list_from(
            raw,
            "DestroyParticleSystems",
            RulesTypeFamily::ParticleSystem,
            0x80,
        );
        self.allocate_scalar_from(raw, "AirstrikeTeamType", RulesTypeFamily::Aircraft, 0x80);
        self.allocate_scalar_from(
            raw,
            "EliteAirstrikeTeamType",
            RulesTypeFamily::Aircraft,
            0x80,
        );
        self.allocate_scalar_from(raw, "UnloadingClass", RulesTypeFamily::Vehicle, 0x80);
        self.allocate_scalar_from(raw, "DeployingAnim", RulesTypeFamily::Animation, 0x80);
        self.allocate_scalar_from(raw, "Enslaves", RulesTypeFamily::Infantry, 0x80);
        self.allocate_scalar_from(raw, "Spawns", RulesTypeFamily::Aircraft, 0x80);
    }

    fn allocate_fixed_art_techno_reference(
        &mut self,
        fixed_art: &IniFile,
        native_stored_id: &str,
        effective: &IniSection,
        key: &str,
        family: RulesTypeFamily,
    ) {
        let image = effective.read_string("Image", native_stored_id, 0x80);
        if image.is_empty() {
            return;
        }
        if let Some(section) = fixed_art.section(&image) {
            self.allocate_scalar_from(section, key, family, 0x80);
        }
    }

    fn process_weapon_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Weapon) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::Weapon, index, pass)
            {
                self.allocate_list_from(&raw, "Anim", RulesTypeFamily::Animation, 0x80);
                for key in ["AssaultAnim", "OccupantAnim", "OpenToppedAnim"] {
                    self.allocate_scalar_from(&raw, key, RulesTypeFamily::Animation, 0x80);
                }
                self.allocate_scalar_from(
                    &raw,
                    "AttachedParticleSystem",
                    RulesTypeFamily::ParticleSystem,
                    0x14,
                );
                self.allocate_scalar_from(&raw, "Warhead", RulesTypeFamily::Warhead, 0x80);
                self.allocate_scalar_from(&raw, "Projectile", RulesTypeFamily::Projectile, 0x80);
            }
            index += 1;
        }
    }

    fn process_bullet_family(&mut self, pass: &IniFile, fixed_art: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Projectile) {
            if let Some((_id, raw, effective)) =
                self.begin_rules_member_read(RulesTypeFamily::Projectile, index, pass)
            {
                let image = effective.read_string("Image", "", 0x19);
                if !image.is_empty()
                    && let Some(section) = fixed_art.section(&image)
                {
                    self.allocate_scalar_from(
                        section,
                        "Trailer",
                        RulesTypeFamily::Animation,
                        0x80,
                    );
                }
                self.allocate_scalar_from(
                    &raw,
                    "AirburstWeapon",
                    RulesTypeFamily::Weapon,
                    0x80,
                );
                self.allocate_scalar_from(
                    &raw,
                    "ShrapnelWeapon",
                    RulesTypeFamily::Weapon,
                    0x80,
                );
            }
            index += 1;
        }
    }

    fn process_warhead_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Warhead) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::Warhead, index, pass)
            {
                self.allocate_scalar_from(
                    &raw,
                    "Particle",
                    RulesTypeFamily::ParticleSystem,
                    0x80,
                );
                self.allocate_list_from(&raw, "AnimList", RulesTypeFamily::Animation, 0x80);
                self.allocate_list_from(
                    &raw,
                    "DebrisTypes",
                    RulesTypeFamily::VoxelAnimation,
                    0x80,
                );
            }
            index += 1;
        }
    }

    fn process_plain_family(&mut self, family: RulesTypeFamily, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(family) {
            self.begin_rules_member_read(family, index, pass);
            index += 1;
        }
    }

    fn process_particle_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::Particle) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::Particle, index, pass)
            {
                self.allocate_scalar_from(&raw, "Warhead", RulesTypeFamily::Warhead, 0x80);
            }
            index += 1;
        }
    }

    fn process_particle_system_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::ParticleSystem) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::ParticleSystem, index, pass)
            {
                let holds_what = raw.read_string("HoldsWhat", "undefined", 0x40);
                self.find_or_allocate(RulesTypeFamily::Particle, &holds_what);
            }
            index += 1;
        }
    }

    fn process_voxel_anim_family(&mut self, pass: &IniFile) {
        let mut index = 0;
        while index < self.family_len(RulesTypeFamily::VoxelAnimation) {
            if let Some((_id, raw, _effective)) =
                self.begin_rules_member_read(RulesTypeFamily::VoxelAnimation, index, pass)
            {
                for key in ["BounceAnim", "ExpireAnim", "TrailerAnim"] {
                    self.allocate_scalar_from(&raw, key, RulesTypeFamily::Animation, 0x80);
                }
                self.allocate_scalar_from(&raw, "Warhead", RulesTypeFamily::Warhead, 0x80);
                self.allocate_scalar_from(
                    &raw,
                    "AttachedSystem",
                    RulesTypeFamily::ParticleSystem,
                    0x80,
                );
            }
            index += 1;
        }
    }

    fn allocate_crate_references(&mut self, pass: &IniFile) {
        let Some(section) = pass.section("CrateRules") else {
            return;
        };
        for key in ["WoodCrateImg", "CrateImg", "WaterCrateImg"] {
            self.allocate_scalar_from(section, key, RulesTypeFamily::Overlay, 0x80);
        }
        self.allocate_scalar_from(section, "UnitCrateType", RulesTypeFamily::Vehicle, 0x80);
    }

    fn allocate_combat_references(&mut self, pass: &IniFile) {
        let Some(section) = pass.section("CombatDamage") else {
            return;
        };
        for key in ["Scorches", "Scorches1", "Scorches2", "Scorches3", "Scorches4"] {
            self.allocate_list_from(section, key, RulesTypeFamily::Smudge, 0x80);
        }
        self.allocate_list_from(section, "SplashList", RulesTypeFamily::Animation, 0x80);
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
        ] {
            self.allocate_scalar_from(section, key, RulesTypeFamily::Warhead, 0x80);
        }
        self.allocate_scalar_from(section, "DeathWeapon", RulesTypeFamily::Weapon, 0x80);
        for key in [
            "DrainAnimationType",
            "ControlledAnimationType",
            "PermaControlledAnimationType",
        ] {
            self.allocate_scalar_from(section, key, RulesTypeFamily::Animation, 0x80);
        }
        self.allocate_scalar_from(section, "IonCannonWarhead", RulesTypeFamily::Warhead, 0x80);
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
            self.allocate_scalar_from(section, key, RulesTypeFamily::ParticleSystem, 0x80);
        }
    }

    fn allocate_radiation_references(&mut self, pass: &IniFile) {
        if let Some(section) = pass.section("Radiation") {
            self.allocate_scalar_from(section, "RadSiteWarhead", RulesTypeFamily::Warhead, 0x80);
        }
    }

    fn allocate_audio_visual_references(&mut self, pass: &IniFile) {
        let Some(section) = pass.section("AudioVisual") else {
            return;
        };
        for key in ["DropPodPuff", "VeinAttack", "Dig", "AtmosphereEntry"] {
            self.allocate_scalar_from(section, key, RulesTypeFamily::Animation, 0x80);
        }
        for key in ["TreeFire", "OnFire"] {
            self.allocate_list_from(section, key, RulesTypeFamily::Animation, 0x80);
        }
        self.allocate_scalar_from(section, "Smoke", RulesTypeFamily::Animation, 0x80);
        self.allocate_scalar_from(section, "Smoke", RulesTypeFamily::Animation, 0x80);
        for key in ["SmallFire", "LargeFire"] {
            self.allocate_scalar_from(section, key, RulesTypeFamily::Animation, 0x80);
        }
    }

    fn process_special_weapons(&mut self, pass: &IniFile) {
        let Some(section) = pass.section("SpecialWeapons") else {
            return;
        };
        for (key, family) in [
            ("NukeWarhead", RulesTypeFamily::Warhead),
            ("NukeProjectile", RulesTypeFamily::Projectile),
            ("NukeDown", RulesTypeFamily::Projectile),
            ("MutateWarhead", RulesTypeFamily::Warhead),
            ("MutateExplosionWarhead", RulesTypeFamily::Warhead),
            ("EMPulseWarhead", RulesTypeFamily::Warhead),
            ("EMPulseProjectile", RulesTypeFamily::Projectile),
        ] {
            self.allocate_scalar_from(section, key, family, 0x80);
        }
        self.process_warhead_family(pass);
    }

    fn process_tiberiums(&mut self, pass: &IniFile) -> Result<(), RulesError> {
        let Some(registry) = pass.section("Tiberiums") else {
            return Ok(());
        };

        for key in registry.keys() {
            let slot = crate::rules::ini_value::atoi_lenient(key);
            let identity = registry.read_string(key, "", 0x18);
            if identity.is_empty() {
                continue;
            }

            if slot < 0 {
                return Err(RulesError::InvalidValue {
                    section: "Tiberiums".to_string(),
                    key: key.to_string(),
                    expected: "a nonnegative native Tiberium slot".to_string(),
                    value: key.to_string(),
                });
            }

            let index = if slot < self.tiberiums.len() as i32 {
                slot as usize
            } else {
                self.tiberiums.push(ProcessedType::new(identity));
                self.tiberiums.len() - 1
            };
            let native_stored_id = self.tiberiums[index].native_stored_id.clone();
            if let Some(section) = pass.section(&native_stored_id).cloned() {
                self.tiberiums[index].body.overlay_rules_pass(&section);
                self.allocate_list_from(
                    &section,
                    "Debris",
                    RulesTypeFamily::Animation,
                    0x80,
                );
            }
        }
        Ok(())
    }

    fn finish(mut self) -> (IniFile, NativeTypeConstructionTrace, CrateRules, PowerupTable) {
        let allocated_super_weapon_type_count = self
            .families
            .get(&RulesTypeFamily::SuperWeapon)
            .map_or(0, Vec::len);
        let mut ini = self.ordinary.take().unwrap_or_else(IniFile::empty);

        for &(registry, family) in PROJECTED_RULE_TYPE_FAMILIES {
            let mut section = IniSection::new(registry.to_string());
            if let Some(members) = self.families.get(&family) {
                for (index, member) in members.iter().enumerate() {
                    section.set(&index.to_string(), &member.native_stored_id);
                }
            }
            ini.replace_first_section(section);
        }
        let mut tiberiums = IniSection::new("Tiberiums".to_string());
        for (index, member) in self.tiberiums.iter().enumerate() {
            tiberiums.set(&index.to_string(), &member.native_stored_id);
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
        for &(_, family) in PROJECTED_RULE_TYPE_FAMILIES {
            if let Some(members) = self.families.get(&family) {
                for member in members {
                    ini.replace_first_section(member.body.clone());
                }
            }
        }
        for family in [RulesTypeFamily::Weapon, RulesTypeFamily::Projectile] {
            if let Some(members) = self.families.get(&family) {
                for member in members {
                    ini.replace_first_section(member.body.clone());
                }
            }
        }
        for member in &self.tiberiums {
            ini.replace_first_section(member.body.clone());
        }

        (
            ini,
            NativeTypeConstructionTrace {
                events: self.native_type_construction_events,
                allocated_super_weapon_type_count,
                registry_state: NativeRulesRegistryState {
                    families: self.families,
                    tiberiums: self.tiberiums,
                },
            },
            self.crate_rules.finish(),
            self.powerups.finish(),
        )
    }
}

pub(crate) fn trim_ascii_controls(value: &str) -> &str {
    value.trim_matches(|character| u32::from(character) <= 0x20)
}

/// Native `strtok(buffer, ",")` tokenization used by Type-reference vectors.
/// Whole-string trimming and caller truncation have already happened in
/// `ReadString`; empty fields collapse and individual tokens remain untrimmed.
fn native_strtok_comma_tokens(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').filter(|token| !token.is_empty())
}

fn is_exact_native_none_type_name(value: &str) -> bool {
    value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("<none>")
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
