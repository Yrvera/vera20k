//! Physical INI representation and scalar readers for Westwood data.
//!
//! Active `gamemd.exe` treats raw section and key names as case-sensitive.
//! A fresh load retains duplicate nonempty section bodies. Empty keys, values,
//! and physical section bodies are not inserted. Arbitrary duplicate-name
//! lookup remains a native CRC/qsort exactification residual.
//! Native registry allocation and ordered RulesClass passes live in
//! `rules::native_processing`; typed gameplay projection lives in `ruleset`.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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
    pub(super) fn new(name: String) -> Self {
        Self {
            name,
            entries: HashMap::new(),
            key_order: Vec::new(),
            projected_values: HashMap::new(),
        }
    }

    pub(super) fn overlay_rules_pass(&mut self, patch: &IniSection) {
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
    pub(super) fn empty() -> Self {
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
    pub(super) fn merge_rules_projection(&mut self, patch: &IniFile) {
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

    /// Obtain the first compatibility section, creating it at the current tail.
    /// Native processing owns which values enter it; INI owns storage/order.
    pub(super) fn projection_section_mut(&mut self, name: &str) -> &mut IniSection {
        let index = if let Some(&index) = self.first_section.get(name) {
            index
        } else {
            let index = self.sections.len();
            self.sections.push(IniSection::new(name.to_string()));
            self.first_section.insert(name.to_string(), index);
            index
        };
        &mut self.sections[index]
    }

    pub(super) fn replace_first_section(&mut self, section: IniSection) {
        if let Some(index) = self.first_section.get(&section.name).copied() {
            self.sections[index] = section;
        } else {
            let index = self.sections.len();
            self.first_section.insert(section.name.clone(), index);
            self.sections.push(section);
        }
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

#[cfg(test)]
#[path = "ini_parser_tests.rs"]
mod tests;
