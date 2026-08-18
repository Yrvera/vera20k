# Slice 1 — INI Typed-Accessor Service — IMPLEMENTATION PLAN

**Status:** IMPLEMENTATION PLAN (per /write-plan). Doc work only — this plan CONTAINS the proposed Rust; it does NOT apply it. No `src/` file is created or modified by writing this plan.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Slice scope:** study-{S0,S1,S2} only — the service surface + its proof harness. **Zero consumer flips.** Consumer flips (study-S3..S7) are later, per-system slices and are out of scope here.

**Source-of-truth docs (read this run):**
- Design spec: `docs/plans/2026-06-04-ini-accessor-service-design.md` (incl. Design-review corrections C1–C4).
- Contract study: `docs/research/INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (P1–P21, GREEN).

**Every Rust file:line below was READ this run.** Anchors verified:
- `src/rules/ini_parser.rs` — `get` :61, `get_i32` :70, `get_f32` :77, `get_light_f32` :86, `get_percent` :100 (`strip_suffix('%')` :102), `get_bool` :114, `get_list` :128, `get_values` :142, `set` :52, `from_str` :197, `merge` :304, test mod attach `#[path = "ini_parser_tests.rs"]` :327-329.
- `src/rules/mod.rs` — `pub mod ini_parser;` :27 (alpha-sorted module list :18-44). No `ini_value`/`ini_enum` exists.
- `src/rules/foundation.rs` — `DEFAULT_FOUNDATION_ID = 0` :15, `FOUNDATION_TABLE: [FoundationDef;22]` :17, `foundation_def` :152 (`eq_ignore_ascii_case`, fallback id 0), `foundation_id` :160; test `foundation_id("3x3refinery")==9` :179.
- `src/util/fixed_math.rs` — `SimFixed = I16F16` :23 (16 frac bits), `sim_from_f32` :85, `sim_from_f64` :92 (both `SimFixed::from_num`), `sim_to_i32` :71 (`to_num::<i32>()`, doc says "rounds toward zero" — WRONG, it floors toward −∞).
- `src/rules/ini_parser_tests.rs` — only `test_get_percent` :208 etc.; **no** hex / first-char-bool / atoi-leniency test.
- Consumers (retire targets, NOT touched this slice): `object_type.rs:60 BuildCategory::from_ini`, `:87 PipScale::from_ini`, `:117 FactoryType::from_ini`, `:870 speed: get_i32("Speed")`; `weapon_type.rs:185 Range: get_f32`, `:189 speed: get_i32("Speed")`, `:199 MinimumRange: get_f32`; `superweapon_type.rs:142 range: get_f32`; `terrain_rules.rs:354 trim_end_matches('%')`; `warhead_type.rs:115 Verses→parse_verses`, `:118 CellSpread get_f32`, `:121 PercentAtMax get_f32`, `parse_verses` :197, `parse_prone_damage_basis_points` :212 (`strip_suffix('%')` :218).
- `src/sim/snapshot.rs:24 SNAPSHOT_VERSION = 17` — untouched this slice.
- Corpus precedent: `src/skirmish_modes.rs:10 const STOCK_MPMODESMD: &str = include_str!("../ini/mpmodesmd.ini");` — the deterministic in-binary corpus pattern S2 reuses. Confirmed `ini/rulesmd.ini` + `ini/artmd.ini` exist.

---

## 0. Hash-relevance + rollout summary

**Nothing in this slice flips hashed state.** The service is *added*; no consumer reads it. `state_hash`, `SNAPSHOT_VERSION` (=17), and the deterministic-replay parity harness (`Slice 8 T6`) are **untouched**. Therefore **no task here needs a `SNAPSHOT_VERSION` bump or a parity-harness re-baseline.** The version bump + parity re-baseline belong to the later consumer-flip slices (study-S3..S6); S2's divergence list is the precise input to those slices.

Rollout shape per task:
- **Tasks 1–9 are pure-additive / read-only** (new files + new tests). They cannot change any parsed value any consumer sees, because no consumer is repointed.
- **Task 10 (S2 corpus harness) is read-only** — it *observes* what the new accessors would resolve vs the old ones; it asserts equality-or-documented-divergence; it does not write game state.
- The only "rollback note" applicable is trivial (see §Rollback): revert the new files / `mod.rs` lines. There is no hash to restore.

Dependency order: **T1 → T2 → (T3..T9 parallel after T2) → T10 (needs T2..T9) ; T0 (S0 gate) runs before any percent-path test (T6) and before T10's SimFixed-comparison rows.**

---

## Task T0 — S0 BLOCKING GATE: pin ReadDouble→SimFixed quantization (test-only, in `fixed_math` tests)

**Why first:** the design's Correction 1 says the load-bearing fact is the *rounding mode of the f64→SimFixed quantization*, not "bit-identical to gamemd" (gamemd keeps the raw double; `SimFixed=I16F16` is coarser). `SimFixed::from_num` rounds **nearest-ties-even**. We must prove the f32-path and f64-path land on the same 16.16 value across the boundary table before any percent consumer (or T6 / T10 SimFixed rows) can be trusted.

**File to edit:** `src/util/fixed_math.rs` — add ONE test inside the existing `#[cfg(test)] mod tests` block (after `test_sim_from_f64`, currently :620-625). No production code changes; `sim_from_f32` / `sim_from_f64` already exist (:85, :92).

**Concrete test (proposed):**
```rust
    /// S0 GATE (design Correction 1): pin the ReadDouble->SimFixed quantization.
    /// gamemd computes `(double)(float)sscanf("%f") [×0.01 if '%']` and keeps the
    /// raw double. SimFixed (I16F16) is coarser (16 frac bits), so there is no
    /// "bit-identical to gamemd" target — the gate is the quantization rounding mode.
    /// `SimFixed::from_num` rounds NEAREST-TIES-EVEN (NOT truncate), so the f32-path
    /// and f64-path must land on the SAME 16.16 value over the stock boundary domain.
    #[test]
    fn test_read_double_precision_matches_gamemd() {
        // Each row: (raw INI string, has '%', the gamemd reference double).
        // reference = (f64)((f32) parsed) [×0.01 if has_percent].
        let rows: &[(&str, bool, f64)] = &[
            ("0", false, 0.0),
            ("1", false, 1.0),
            ("7", false, 7.0),
            ("0.5", false, 0.5),
            (".9", false, 0.9),
            ("0.016", false, 0.016),
            ("100%", true, 1.0),
            ("50%", true, 0.5),
            ("12.5%", true, 0.125),
            ("-50%", true, -0.5),   // NEGATIVE + percent (design §10 requires it)
            ("10%0", true, 0.1),    // FIXED (plan-review): "%f" reads 10 (stops at '%'),
                                    // strchr('%') matches ANYWHERE -> ×0.01 -> 0.1.
                                    // Verified: ReadDouble 0x005283D0 strchr(value,'%');
                                    // study INI_PARSING_HELPERS line 35/56/451. NOT 0.0.
        ];
        for (s, pct, _ref) in rows {
            // Reproduce gamemd's intermediate: sscanf "%f" reads the leading float
            // and STOPS at the first non-float char ('%'), then widens f32->f64,
            // then ×0.01 iff the *original string* contains '%' anywhere.
            let leading: f32 = parse_leading_f32(s);
            let widened: f64 = leading as f64;
            let reference: f64 = if *pct { widened * 0.01_f64 } else { widened };

            let from_f64 = sim_from_f64(reference);
            let from_f32 = sim_from_f32(reference as f32);
            // (a) f32-path vs f64-path agree after 16.16 quantization:
            assert_eq!(
                from_f64, from_f32,
                "path divergence for {s:?}: f64={from_f64} f32={from_f32}"
            );
            // (b) chosen path equals from_num(reference_double):
            assert_eq!(from_f64, SimFixed::from_num(reference), "row {s:?}");
        }
    }

    // Test helper mirroring sscanf "%f": leading optional-sign float, stop at first
    // non-float char (so "10%0" -> 10.0, "12.5%" -> 12.5). NOT production code.
    fn parse_leading_f32(s: &str) -> f32 {
        let t = s.trim();
        let bytes = t.as_bytes();
        let mut end = 0usize;
        let mut seen_dot = false;
        while end < bytes.len() {
            let c = bytes[end];
            let ok = c.is_ascii_digit()
                || (end == 0 && (c == b'-' || c == b'+'))
                || (c == b'.' && !seen_dot);
            if c == b'.' { seen_dot = true; }
            if !ok { break; }
            end += 1;
        }
        t[..end].parse::<f32>().unwrap_or(0.0)
    }
```
**Note for `"10%0"`:** gamemd's `sscanf "%f"` stops at `%`, so it parses `10`, then the `strchr('%')` test sees a `%` and multiplies by `0.01` → `0.1`. The reference row above is therefore `0.1`, not `0.0` — **the plan-review must confirm this single row's expected value** (the leading-float-stop + percent-multiply interaction). If review prefers, drop the `"10%0"` row from T0 and assert it only as a *value* (not SimFixed) row in T6/T10. Either way T0 must include at least one negative + one decimal-percent row.

**Verification:**
`cargo test -p vera20k test_read_double_precision_matches_gamemd`
Proves: the two conversion paths quantize identically over the boundary domain (gate green ⇒ later percent flips may proceed). **Dependency:** none. **Blocks:** T6 (percent test), T10 SimFixed rows.

---

## Task T1 — Register the two new modules in `mod.rs` (additive)

**File to edit:** `src/rules/mod.rs`. The module list is alpha-sorted (:18-44). Insert two declarations so they keep alpha order: `ini_enum` before `ini_parser`, `ini_value` after `ini_parser`.

**Edit anchor (current :26-28):**
```rust
pub mod infantry_sequence;
pub mod ini_parser;
pub mod jumpjet_params;
```
**Becomes:**
```rust
pub mod infantry_sequence;
pub mod ini_enum;
pub mod ini_parser;
pub mod ini_value;
pub mod jumpjet_params;
```
This will NOT compile until T2/T3 create the files — that is fine; T1 is a dependency edge, land it in the same commit as T2/T3.

**Verification:** compiles only after T2/T3 land; `cargo check -p vera20k`. **Dependency:** must land with T2 + T3.

---

## Task T2 — Create `src/rules/ini_value.rs`: the int/bool/double/string + atoi core (additive)

**File to create:** `src/rules/ini_value.rs`. Methods are an **inherent-impl split on `IniSection`** (design open-question 1, recommended: methods, not a `CcIni` newtype — minimizes flip-time diff). Rust allows a second `impl IniSection` block in another file of the same crate. `IniSection::get` (:61) is `pub`, so the new methods can call `self.get(key)`.

**Module header + the int/bool/double/string surface (proposed):**
```rust
//! Typed INI accessor service — the gamemd CCINIClass `ReadX` analog.
//!
//! Sits on top of the raw `IniSection` store (the "INIClass" analog). Reproduces
//! the gamemd parse CONTRACT bit-for-bit on the resolved value: $xx/xxh hex,
//! C-atoi leniency, first-char bool, '%'-anywhere ×0.01 double, strtrim ≤0x20.
//!
//! INVARIANT (P4/P18): "present" = key exists (even if value is empty). A present
//! key ALWAYS returns its parsed value (which for int may be atoi("")=0). `default`
//! is returned ONLY when section/key is ABSENT. This is NOT `unwrap_or(default)` —
//! it does not fall to default on parse failure.
//!
//! ## Dependency rules
//! - rules/ only: depends on `crate::rules::ini_parser`. No sim/render/ui/audio/net.
//! - Returns un-truncated f64 from `read_double`; the single f64->SimFixed
//!   conversion stays in `util::fixed_math`. No float enters sim/.

use crate::rules::ini_parser::IniSection;

/// 0x20 = ASCII space; gamemd `strtrim` strips bytes <= 0x20 (space + all ASCII
/// control) at BOTH ends — NOT Unicode whitespace.
const STRTRIM_MAX: u8 = 0x20;

impl IniSection {
    /// ReadInt (P1–P4, P18): `$xx`/`xxh` (case-insensitive `h`) hex, else C-atoi
    /// leniency. Default ONLY on absent key. Present-but-nonnumeric -> atoi (0).
    pub fn read_int(&self, key: &str, default: i32) -> i32 {
        match self.get(key) {
            None => default,
            Some(raw) => {
                // strtrim ≤0x20 both ends (P5), matching the value gamemd parses.
                let v = strtrim_ascii(raw);
                if let Some(rest) = v.strip_prefix('$') {
                    // "$%x": parse hex; junk after digits stops the C scan -> take
                    // the leading hex run (sscanf "$%x" stops at first non-hex).
                    parse_leading_hex(rest)
                } else if ends_with_h(v) {
                    // "%xh": leading hex run, ignore the trailing 'h'/'H'.
                    parse_leading_hex(&v[..v.len() - 1])
                } else {
                    atoi_lenient(v)
                }
            }
        }
    }

    /// ReadBool (P6, P18): `toupper(first char)` in {'1','T','Y'}=true,
    /// {'0','F','N'}=false, else default. `on`/`off` (first char 'o') -> default.
    /// Present-empty (first char '\0') -> default.
    pub fn read_bool(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                match v.bytes().next().map(|b| b.to_ascii_uppercase()) {
                    Some(b'1') | Some(b'T') | Some(b'Y') => true,
                    Some(b'0') | Some(b'F') | Some(b'N') => false,
                    _ => default, // present-empty or any other first char
                }
            }
        }
    }

    /// ReadDouble (P7): sscanf "%f" (leading float, single-precision) widened to
    /// f64, then ×0.01 iff the value string contains '%' ANYWHERE. Returns the
    /// gamemd double UN-truncated; the consumer truncates toward zero at ITS
    /// boundary (never `.round()` / never truncate here). Default ONLY on absent.
    /// [precision pinned by T0]
    pub fn read_double(&self, key: &str, default: f64) -> f64 {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                let leading: f32 = parse_leading_f32(v); // f32 first (mantissa narrow)
                let widened: f64 = leading as f64;
                if v.as_bytes().contains(&b'%') {
                    widened * 0.01_f64
                } else {
                    widened
                }
            }
        }
    }

    /// ReadString (P5, P18): strtrim ≤0x20 both ends; default on ABSENT key;
    /// present-empty -> "". No C buffer cap in Rust — debug_assert at the smallest
    /// gamemd per-accessor cap (32) to surface a corpus value that WOULD truncate
    /// (design open-question 6: debug-assert, do NOT silently truncate).
    pub fn read_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                debug_assert!(
                    v.len() <= 32,
                    "INI value for {key:?} is {} chars; gamemd smallest \
                     ReadString cap is 32 (enum/zone/action) — would truncate",
                    v.len()
                );
                v
            }
        }
    }
}

/// strtrim equivalent (P5): strip bytes <= 0x20 from BOTH ends. ASCII-only by
/// design (RA2 INI is ASCII); does NOT use `str::trim` (Unicode whitespace).
fn strtrim_ascii(s: &str) -> &str {
    let b = s.as_bytes();
    let mut start = 0usize;
    while start < b.len() && b[start] <= STRTRIM_MAX {
        start += 1;
    }
    let mut end = b.len();
    while end > start && b[end - 1] <= STRTRIM_MAX {
        end -= 1;
    }
    &s[start..end]
}

/// tolower(last char) == 'h' (P2). Case-insensitive via ASCII.
fn ends_with_h(s: &str) -> bool {
    s.as_bytes().last().map(|b| b.to_ascii_lowercase()) == Some(b'h')
}

/// Parse a leading run of hex digits (after `$` strip or before `h` strip).
/// sscanf "$%x"/"%xh" stop at the first non-hex char. No sign (hex is unsigned
/// in gamemd's `$`/`h` branches). Empty -> 0.
fn parse_leading_hex(s: &str) -> i32 {
    let mut acc: i64 = 0;
    let mut any = false;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => (c - b'0') as i64,
            b'a'..=b'f' => (c - b'a' + 10) as i64,
            b'A'..=b'F' => (c - b'A' + 10) as i64,
            _ => break,
        };
        any = true;
        acc = acc.wrapping_mul(16).wrapping_add(d);
    }
    if any { acc as i32 } else { 0 }
}

/// C-atoi-equivalent leading-numeric parse (P3): optional leading sign, then
/// leading decimal digits, stop at first non-digit. `5cells`->5, `abc`->0,
/// ``->0, `  7 `->7 (already strtrimmed), `-50`->-50, `+9`->9. NB `0x1A`->0
/// (atoi does NOT treat `0x` as hex; the `$`/`h` branches are separate).
pub(crate) fn atoi_lenient(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut neg = false;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    let mut any = false;
    while i < b.len() && b[i].is_ascii_digit() {
        any = true;
        acc = acc.wrapping_mul(10).wrapping_add((b[i] - b'0') as i64);
        i += 1;
    }
    if !any { return 0; }
    let v = if neg { -acc } else { acc };
    v as i32
}

/// sscanf "%f"-equivalent leading float (P7): optional sign, digits, single dot,
/// more digits; stop at first non-float char (so "12.5%"->12.5, "10%0"->10).
/// Empty/junk -> 0.0.
pub(crate) fn parse_leading_f32(s: &str) -> f32 {
    let b = s.as_bytes();
    let mut end = 0usize;
    let mut seen_dot = false;
    while end < b.len() {
        let c = b[end];
        let ok = c.is_ascii_digit()
            || (end == 0 && (c == b'-' || c == b'+'))
            || (c == b'.' && !seen_dot);
        if c == b'.' { seen_dot = true; }
        if !ok { break; }
        end += 1;
    }
    s[..end].parse::<f32>().unwrap_or(0.0)
}
```

**Notes carried from the design corrections:**
- `read_double` returns un-truncated f64 (ledger #6); **does NOT** touch `SimFixed` — the conversion stays in `util::fixed_math` at the call site (boundary discipline). No `sim/` dep.
- Scientific-notation (`1e3`) and exponent forms are NOT in `parse_leading_f32`; gamemd `"%f"` does accept `e`/`E` exponents. **Plan-review must confirm** whether any stock `rulesmd.ini`/`artmd.ini` double value uses exponent notation (the corpus scan in T10 surfaces it). If none, the simpler parser is correct for the stock domain; if any, add an `e`/`E` branch.

**Verification:** `cargo check -p vera20k` (compiles with T1). **Dependency:** T1 (module decl), `ini_parser.rs` (`get`, unchanged). Reuses T0's pinned conversion conceptually (T0 already green).

---

## Task T3 — Create `src/rules/ini_enum.rs`: the enum-by-name round-trip helper (additive)

**File to create:** `src/rules/ini_enum.rs`. One generic `enum_by_name` matching gamemd's enum helper (P10): whole-string case-insensitive compare against a static `{name,id}` table, table-default id on miss. This is the same shape `foundation.rs:152 foundation_def` already implements correctly — the helper generalizes it.

```rust
//! Generic enum-by-name table helper — the gamemd enum round-trip (P10).
//!
//! gamemd's enum readers (Foundation, MovementZone, SpeedType, Layer, Action)
//! ReadString into a fixed buffer (default = the default entry's NAME), then do a
//! WHOLE-STRING case-insensitive compare against a static `{name,id}` table and
//! return the matched id, else the table default id. A substring does NOT match.
//!
//! ## Dependency rules
//! - rules/ only; no other module dependency. Pure function over a static table.

/// One name->id row in an enum table.
#[derive(Debug, Clone, Copy)]
pub struct EnumByName {
    pub name: &'static str,
    pub id: i32,
}

/// Resolve `value` against `table` (whole-string, case-insensitive). Returns the
/// matched id, else `default_id`. Per-table defaults differ in gamemd
/// (Foundation -> 0 = "1x1"; MovementZone -> -1; Action -> 0) — the CALLER passes
/// the right default_id; this helper does not bake one in.
pub fn enum_by_name(value: &str, table: &[EnumByName], default_id: i32) -> i32 {
    let trimmed = value.trim_matches(|c: char| (c as u32) <= 0x20);
    table
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(trimmed))
        .map(|e| e.id)
        .unwrap_or(default_id)
}
```
**Note:** the trim uses the same ≤0x20 rule as `strtrim_ascii` (the gamemd enum readers run strtrim before the compare). `foundation.rs` currently uses `str::trim` (Unicode) at :153 — that is a pre-existing, stock-harmless difference; **do not change foundation.rs this slice** (it is a study-S3 flip target). This helper is added but not yet consumed.

**Verification:** `cargo check -p vera20k`. **Dependency:** T1 (module decl).

---

## Task T4 — Comma-tuple readers in `ini_value.rs`: `read_3int`, `read_minmax`, `read_point`, `read_rect` (additive)

**File to edit:** `src/rules/ini_value.rs` (append to the `impl IniSection` block from T2). All four are COMMA-delimited (P8/P9 — design Correction confirmed COMMA, not space). Each component parses via the atoi-lenient int rule (sscanf field-stop behavior). All-defaults copied on **absent key** (P8); a present-but-short value keeps the default for missing fields (ReadRect seeds `"0,0,0,0"` — i.e. missing trailing fields fall to 0/the default component).

```rust
impl IniSection {
    /// Read3Int (P8): comma "%d,%d,%d". All-defaults on ABSENT key. Each field
    /// atoi-lenient; missing trailing fields keep the corresponding default.
    pub fn read_3int(&self, key: &str, default: [i32; 3]) -> [i32; 3] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(3) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                out
            }
        }
    }

    /// ReadMinMax (P8): comma "%d,%d". All-defaults on ABSENT key.
    pub fn read_minmax(&self, key: &str, default: [i32; 2]) -> [i32; 2] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(2) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                out
            }
        }
    }

    /// ReadPoint/ReadSize (P9, COMMA): "%d,%d". All-defaults on ABSENT key.
    pub fn read_point(&self, key: &str, default: (i32, i32)) -> (i32, i32) {
        let [x, y] = self.read_minmax(key, [default.0, default.1]);
        (x, y)
    }

    /// ReadRect (P9, COMMA): "%d,%d,%d,%d". gamemd seeds "0,0,0,0" so missing
    /// fields keep the default component; all-defaults on ABSENT key.
    pub fn read_rect(&self, key: &str, default: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = [default.0, default.1, default.2, default.3];
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(4) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                (out[0], out[1], out[2], out[3])
            }
        }
    }
}
```
**Verification:** `cargo check -p vera20k`. **Dependency:** T2 (`atoi_lenient`, `strtrim_ascii`).

---

## Task T5 — `read_color_rgb` in `ini_value.rs` (additive)

**File to edit:** `src/rules/ini_value.rs`. P21: COMMA `"%d,%d,%d"` → `[u8;3]`; per-component is plain `%d` — **NOT atoi-lenient, NO `$`/`h` hex** (sscanf `%d` stops at first non-digit). Default RGB on miss/parse-fail. gamemd packs into a u8 (so a component is taken mod 256 by the C narrowing; we clamp/cast to u8 to mirror the byte pack).

```rust
impl IniSection {
    /// ReadColorRGB (P21): COMMA "%d,%d,%d" -> [u8;3]. Per-component plain %d
    /// (stops at first non-digit; NO atoi-leniency, NO hex). Default RGB on
    /// absent key or short value; component byte-narrowed to u8 (gamemd packs
    /// the sscanf int into a byte).
    pub fn read_color_rgb(&self, key: &str, default: [u8; 3]) -> [u8; 3] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(3) {
                    // plain %d: leading optional sign + digits, stop at non-digit.
                    out[i] = atoi_lenient(strtrim_ascii(tok)) as u8;
                }
                out
            }
        }
    }
}
```
**Note:** `atoi_lenient` and sscanf `%d` agree on the stock RGB domain (`"12,34,56"`); both read leading-sign + decimal digits and stop at non-digit. They differ only on a `$`/`h` value — which `%d` would NOT treat as hex, and neither does `atoi_lenient` (its `$`/`h` hex lives in `read_int`, not the bare `atoi_lenient` fn). So reusing `atoi_lenient` here is faithful to `%d`. **Plan-review confirm:** no stock `[Colors]`/tint triplet uses a `$`-prefixed component (T10 corpus scan surfaces it).

**Verification:** `cargo check -p vera20k`. **Dependency:** T2.

---

## Task T6 — Transform accessors `read_speed`, `read_range` in `ini_value.rs` (additive) — the C2 toward-zero trap

**File to edit:** `src/rules/ini_value.rs`. These two **transform** the value; the transform IS the contract.

- `read_speed` (P19): `read_int(-1)`; `-1` → default; else `min(v,100)`, `(v<<8)/100` round-toward-zero, `min(result,255)`. Rust `i32 /` already truncates toward zero, so the division is fine (ledger #18). `100→255`, `50→128`, `7→17`, `0→0`.
- `read_range` (P20): `read_double(-1.0)`; `== -1.0` → default; else **truncate toward ZERO** to i32. **MUST NOT use `util::sim_to_i32`** — `to_num::<i32>()` floors toward −∞ (`fixed_math.rs:71`, doc comment is wrong), diverging on negatives. Truncate explicitly via `f64 as i32` (Rust `as` on f64→i32 truncates toward zero, saturating, NaN→0). `5.9→5`.

```rust
impl IniSection {
    /// ReadSpeed (P19): read_int(-1) sentinel; -1 -> default; else clamp100,
    /// (v<<8)/100 truncate-toward-zero (Rust i32 / truncates toward 0), clamp255.
    pub fn read_speed(&self, key: &str, default: i32) -> i32 {
        let raw = self.read_int(key, -1);
        if raw == -1 {
            return default;
        }
        let capped = raw.min(100);
        let scaled = (capped << 8) / 100; // i32 / truncates toward zero (ledger #18)
        scaled.min(255)
    }

    /// ReadRange (P20): read_double(-1.0) sentinel; ==-1.0 -> default; else ftol
    /// TRUNCATE-TOWARD-ZERO. NOT util::sim_to_i32 (that floors toward −∞ — DRIFT
    /// on negatives, ledger #18). `f64 as i32` truncates toward zero.
    pub fn read_range(&self, key: &str, default: i32) -> i32 {
        let raw = self.read_double(key, -1.0);
        if raw == -1.0 {
            return default;
        }
        raw as i32 // truncate toward zero (gamemd ftol RC=11)
    }
}
```
**Note (design Correction 4 + OQ4):** a present-empty `Speed=` → `read_int("")` = atoi("") = 0 (NOT the -1 sentinel) → `read_speed` = `(0<<8)/100` = 0, NOT the call-site default. Same for `Range=` present-empty → `read_double` = 0.0 → not -1.0 → `read_range` = 0. This is correct per P4/P18 but means present-empty silently resolves to 0; **T10 must scan stock for present-empty `Speed=`/`Range=`/`MinimumRange=`.**

**Verification:** `cargo test -p vera20k` (transform tests land in T8). Requires T0 green (percent precision pinned) before `read_range`'s f64 path is trusted in T10's SimFixed rows. **Dependency:** T2 (`read_int`/`read_double`), T0.

---

## Task T7 — Unit tests for the int/bool/double/string/atoi core (additive, study-S1 list)

**File to edit:** `src/rules/ini_value.rs` — add a `#[cfg(test)] mod tests` block (the codebase mixes inline test mods and `#[path]` side files; inline is fine here, mirrors `foundation.rs`/`warhead_type.rs`). Build a tiny `IniFile` via `crate::rules::ini_parser::IniFile::from_str(...)` like `ini_parser_tests.rs` does.

```rust
#[cfg(test)]
mod tests {
    use crate::rules::ini_parser::IniFile;
    use super::{atoi_lenient, parse_leading_f32};

    fn sec(body: &str) -> crate::rules::ini_parser::IniFile { IniFile::from_str(body) }

    #[test] // P1/P2
    fn test_read_int_hex() {
        let ini = sec("[S]\nA=$1A\nB=1Ah\nC=0FFH\nD=$0\nE=$FF\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -9), 26);
        assert_eq!(s.read_int("B", -9), 26);
        assert_eq!(s.read_int("C", -9), 255);
        assert_eq!(s.read_int("D", -9), 0);
        assert_eq!(s.read_int("E", -9), 255);
    }

    #[test] // P3/P4/P18
    fn test_read_int_atoi_leniency() {
        let ini = sec("[S]\nA=5cells\nB=abc\nC=-50\nD=\nE=  7 \n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -9), 5);
        assert_eq!(s.read_int("B", -9), 0);   // present-nonnumeric -> 0, NOT default
        assert_eq!(s.read_int("C", -9), -50);
        assert_eq!(s.read_int("D", -9), 0);   // present-empty -> atoi("") = 0
        assert_eq!(s.read_int("E", -9), 7);
        assert_eq!(s.read_int("MISSING", -9), -9); // absent -> default
    }

    #[test] // OQ3: 0x is NOT hex via atoi fallback
    fn test_read_int_0x_prefix_is_zero() {
        let ini = sec("[S]\nA=0x1A\n");
        // atoi("0x1A") = 0 (stops at 'x'); $/h branches don't fire.
        assert_eq!(ini.section("S").unwrap().read_int("A", -9), 0);
    }

    #[test] // P6/P18
    fn test_read_bool_first_char() {
        let ini = sec("[S]\nA=yes\nB=Y\nC=T\nD=true\nE=1\nF=no\nG=N\nH=F\nI=false\nJ=0\nK=off\nL=xyz\nM=\n");
        let s = ini.section("S").unwrap();
        for k in ["A","B","C","D","E"] { assert!(s.read_bool(k, false), "{k}"); }
        for k in ["F","G","H","I","J"] { assert!(!s.read_bool(k, true), "{k}"); }
        assert!(s.read_bool("K", true));  // 'off' first char 'o' -> default
        assert!(s.read_bool("L", true));  // xyz -> default
        assert!(s.read_bool("M", true));  // present-empty -> default
        assert!(s.read_bool("MISSING", true)); // absent -> default
    }

    #[test] // P7 (after T0 pins precision)
    fn test_read_double_percent() {
        let ini = sec("[S]\nA=50%\nB=100%\nC=7\nD=0.5\nE=12.5%\n");
        let s = ini.section("S").unwrap();
        assert!((s.read_double("A", -1.0) - 0.5).abs() < 1e-6);
        assert!((s.read_double("B", -1.0) - 1.0).abs() < 1e-6);
        assert!((s.read_double("C", -1.0) - 7.0).abs() < 1e-6);
        assert!((s.read_double("D", -1.0) - 0.5).abs() < 1e-6);
        assert!((s.read_double("E", -1.0) - 0.125).abs() < 1e-6);
        assert!((s.read_double("MISSING", -42.0) + 42.0).abs() < 1e-9); // absent -> default
    }

    #[test] // P5/P18
    fn test_read_string_trim_default() {
        let ini = sec("[S]\nA=  hello  \nB=\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_string("A", "D"), "hello"); // trimmed
        assert_eq!(s.read_string("B", "D"), "");       // present-empty -> ""
        assert_eq!(s.read_string("MISSING", "D"), "D"); // absent -> default
    }

    #[test]
    fn test_atoi_and_leading_f32_helpers() {
        assert_eq!(atoi_lenient("5cells"), 5);
        assert_eq!(atoi_lenient("-50"), -50);
        assert_eq!(atoi_lenient("+9"), 9);
        assert_eq!(atoi_lenient(""), 0);
        assert!((parse_leading_f32("12.5%") - 12.5).abs() < 1e-6);
        assert!((parse_leading_f32(".9") - 0.9).abs() < 1e-6);
    }
}
```
**Note on `from_str` inline-comment stripping:** `IniFile::from_str` (:246) strips everything after `;` and `.trim()`s the value at load — so the *stored* value is already `;`-stripped and Unicode-trimmed. The `read_*` strtrim runs again on top (idempotent for ASCII). A value like `A=  hello  ` is stored as `"hello"` already (load trim), so `test_read_string_trim_default` still proves the read-side trim is at least not regressing. To exercise the read-side strtrim *independently*, the corpus test (T10) and a dedicated control-char row would be needed; **plan-review may add a direct `IniSection` fixture** (bypassing `from_str`) if read-side trim must be proven in isolation — `set` (:52) is `pub(crate)`, usable from a `rules`-internal test.

**Verification:** `cargo test -p vera20k --lib rules::ini_value`. **Dependency:** T2, T0 (for the percent test).

---

## Task T8 — Unit tests for tuples, color, transforms, enum (additive, study-S1 + S7 list)

**File(s) to edit:** `src/rules/ini_value.rs` tests mod (tuples/color/transforms) + `src/rules/ini_enum.rs` (enum test).

```rust
// in ini_value.rs tests mod:
    #[test] // P9 COMMA
    fn test_read_point_comma() {
        let ini = sec("[S]\nP=3,5\nR=1,2,3,4\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_point("P", (0, 0)), (3, 5));
        assert_eq!(s.read_rect("R", (0, 0, 0, 0)), (1, 2, 3, 4));
        assert_eq!(s.read_point("MISSING", (9, 9)), (9, 9)); // absent -> default
    }

    #[test] // P8 partial keeps default component
    fn test_read_3int_partial_keeps_default() {
        let ini = sec("[S]\nA=10,20\n"); // only 2 of 3 fields
        assert_eq!(ini.section("S").unwrap().read_3int("A", [1, 2, 3]), [10, 20, 3]);
    }

    #[test] // P21
    fn test_read_color_rgb() {
        let ini = sec("[S]\nC=12,34,56\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_color_rgb("C", [0, 0, 0]), [12, 34, 56]);
        assert_eq!(s.read_color_rgb("MISSING", [1, 2, 3]), [1, 2, 3]);
    }

    #[test] // P19
    fn test_read_speed_clamp() {
        let ini = sec("[S]\nA=100\nB=50\nC=7\nD=0\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_speed("A", -1), 255); // (100<<8)/100=256 -> clamp 255
        assert_eq!(s.read_speed("B", -1), 128); // (50<<8)/100=128
        assert_eq!(s.read_speed("C", -1), 17);  // (7<<8)/100=17 (trunc)
        assert_eq!(s.read_speed("D", -1), 0);
        assert_eq!(s.read_speed("MISSING", 42), 42); // absent -> default (sentinel -1)
    }

    #[test] // P20 truncate toward zero (ledger #18)
    fn test_read_range_truncates() {
        let ini = sec("[S]\nA=5.9\nB=5\nC=0.4\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_range("A", -1), 5); // 5.9 -> 5 (never rounds to 6)
        assert_eq!(s.read_range("B", -1), 5);
        assert_eq!(s.read_range("C", -1), 0);
        assert_eq!(s.read_range("MISSING", 7), 7); // absent -> default (sentinel -1.0)
    }
```
```rust
// in ini_enum.rs:
#[cfg(test)]
mod tests {
    use super::{enum_by_name, EnumByName};

    const FOUNDATION: &[EnumByName] = &[
        EnumByName { name: "1x1", id: 0 },
        EnumByName { name: "3x3refinery", id: 9 },
    ];

    #[test] // P10 whole-string, case-insensitive, table default
    fn test_enum_by_name() {
        assert_eq!(enum_by_name("3x3Refinery", FOUNDATION, 0), 9);
        assert_eq!(enum_by_name("1X1", FOUNDATION, 0), 0);
        assert_eq!(enum_by_name("unknown", FOUNDATION, 0), 0); // miss -> default
        assert_eq!(enum_by_name("3x3", FOUNDATION, 0), 0);     // substring NO match
    }
}
```
**Verification:** `cargo test -p vera20k ini_value`; `cargo test -p vera20k ini_enum`. **Dependency:** T4, T5, T6, T3, T0.

---

## Task T9 — `read_int`/`read_double` negative-int + sign tests (additive, completeness)

**File to edit:** `src/rules/ini_value.rs` tests mod. Locks the sign/edge behavior the corpus may exercise (negative ints, leading `+`, the `0x`-atoi case already in T7). Small but the design's open-question 3 needs a locked row.

```rust
    #[test]
    fn test_read_int_signs_and_edges() {
        let ini = sec("[S]\nA=+9\nB=-0\nC=$\nD=h\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -1), 9);
        assert_eq!(s.read_int("B", -1), 0);
        assert_eq!(s.read_int("C", -1), 0); // "$" with no hex digits -> 0
        // "h": ends_with_h true, leading hex of "" -> 0
        assert_eq!(s.read_int("D", -1), 0);
    }
```
**Note:** the `D=h` row depends on `ends_with_h("h")==true` then `parse_leading_hex("")==0`. **Plan-review confirm** this matches gamemd: a bare `h` value — gamemd `sscanf "%xh"` on `"h"` reads no hex digits → result undefined/0. If gamemd leaves the int uninitialized differently, drop this row (no stock key is `=h`). Surface in T10.

**Verification:** `cargo test -p vera20k ini_value`. **Dependency:** T2.

---

## Task T10 — S2 corpus equivalence harness "the shadow assert" (read-only, NOT hash-relevant)

**File to create:** `src/rules/ini_value.rs` — a dedicated `#[cfg(test)] mod corpus_tests` (or a `#[path]` side file `ini_value_corpus_tests.rs` if the body grows past ~150 lines; mirror `ini_parser.rs:327` `#[path]` pattern). Load the stock corpus deterministically via `include_str!`, mirroring `skirmish_modes.rs:10`:

```rust
#[cfg(test)]
mod corpus_tests {
    use crate::rules::ini_parser::IniFile;

    const STOCK_RULESMD: &str = include_str!("../../ini/rulesmd.ini");
    const STOCK_ARTMD: &str = include_str!("../../ini/artmd.ini");
```
**Path note:** `ini_value.rs` lives in `src/rules/`, so `../../ini/` from there is the repo `ini/`. (`skirmish_modes.rs` is in `src/`, hence its `../ini/`.) **Plan-review/implementer must confirm the relative depth at write time** — adjust to `../../ini/` for a `src/rules/` file.

**What the test asserts (study-S2 + design Corrections C4):**
1. **Equivalence-or-documented-divergence** over every section/key. For each key, compute the OLD accessor result (`get_i32`, `get_bool`, `get_percent`, `get_f32` as the consumers use them) and the NEW (`read_int`, `read_bool`, `read_double`). Where the OLD returns `Some(x)` and NEW returns the same numeric/bool → OK. Where they differ, the row MUST be in an explicit `DIVERGENCES` allowlist with the gamemd-correct expected value and a one-line cited reason (hex / first-char-bool / `%`-anywhere / atoi-leniency). **No silent diff.** A diff not in the allowlist fails the test.
2. **`0x`-prefix scan (OQ3):** assert no stock int key has a `0x`-prefixed value that any consumer reads as an int (would be atoi→0). Surface every occurrence.
3. **Present-empty `Speed=`/`Range=`/`MinimumRange=` scan (OQ4/C4):** collect every present-empty occurrence and assert the list matches a known (likely empty) allowlist; surface each so a later flip slice knows it will resolve to 0 vs the call-site default.
4. **P5 buffer-cap scan:** flag any value that an enum/zone/action accessor would read (cap 32) longer than 31 chars, and any 128-cap list value longer than 127 chars. Surface, do not silently pass.
5. **Exponent-notation scan (T2 note):** flag any double-valued key whose value contains `e`/`E` exponent so `parse_leading_f32` coverage can be confirmed.

**Skeleton (proposed):**
```rust
    /// Keys where the NEW accessor INTENTIONALLY diverges from the OLD one,
    /// each with the gamemd-correct expected value + reason. Populated from the
    /// first failing run, then audited row-by-row against the contract (P1–P21).
    /// Format: (section, key, reason). Empty until the first run enumerates them.
    const DIVERGENCES: &[(&str, &str, &str)] = &[
        // e.g. ("SomeUnit", "Strength", "hex $190 -> 400; old get_i32 None -> default"),
    ];

    #[test]
    fn test_ini_accessor_corpus_parity() {
        let mut ini = IniFile::from_str(STOCK_RULESMD);
        ini.merge(&IniFile::from_str(STOCK_ARTMD));

        let mut undocumented: Vec<String> = Vec::new();
        let mut zero_x: Vec<String> = Vec::new();
        let mut present_empty_transform: Vec<String> = Vec::new();
        let mut over_cap: Vec<String> = Vec::new();
        let mut exponent: Vec<String> = Vec::new();

        for name in ini.section_names() {
            let sec = ini.section(name).unwrap();
            for key in sec.keys() {
                let raw = sec.get(key).unwrap_or("");
                // 0x scan
                if raw.trim().starts_with("0x") || raw.trim().starts_with("0X") {
                    zero_x.push(format!("[{name}] {key}={raw}"));
                }
                // cap scan (smallest gamemd cap 32)
                let trimmed_len = raw.trim().len();
                if trimmed_len > 31 {
                    over_cap.push(format!("[{name}] {key} len={trimmed_len}"));
                }
                // exponent scan on plausibly-numeric values
                if raw.contains('e') || raw.contains('E') {
                    exponent.push(format!("[{name}] {key}={raw}"));
                }
                // int equivalence: old get_i32 vs new read_int (sentinel default)
                let old_i = sec.get_i32(key);
                let new_i = sec.read_int(key, i32::MIN);
                if let Some(o) = old_i {
                    if o != new_i && !is_documented(name, key) {
                        undocumented.push(format!("[{name}] {key}: old_i32={o} new_int={new_i} raw={raw}"));
                    }
                }
                // (bool / double equivalence checks: same shape, with the OLD
                //  accessor's parse rule; differences land in `undocumented`
                //  unless in DIVERGENCES.)
                // present-empty transform scan
                if raw.trim().is_empty()
                    && matches!(key.to_ascii_lowercase().as_str(), "speed" | "range" | "minimumrange")
                {
                    present_empty_transform.push(format!("[{name}] {key}="));
                }
            }
        }

        // Surface every category; fail on UNDOCUMENTED diffs only.
        assert!(
            undocumented.is_empty(),
            "UNDOCUMENTED parse divergences (each must be added to DIVERGENCES \
             with a gamemd-correct reason or proven a real fix):\n{}",
            undocumented.join("\n")
        );
        // The scans below are surfaced (eprintln!) for the later flip slices;
        // assert them empty only if review confirms the stock corpus has none.
        if !zero_x.is_empty() { eprintln!("0x-prefixed values:\n{}", zero_x.join("\n")); }
        if !present_empty_transform.is_empty() {
            eprintln!("present-empty Speed/Range/MinimumRange:\n{}", present_empty_transform.join("\n"));
        }
        if !over_cap.is_empty() { eprintln!(">31-char values (cap-32 scan):\n{}", over_cap.join("\n")); }
        if !exponent.is_empty() { eprintln!("exponent-notation values:\n{}", exponent.join("\n")); }
    }

    fn is_documented(section: &str, key: &str) -> bool {
        DIVERGENCES.iter().any(|(s, k, _)| s.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key))
    }
```
**Process for the implementer (study-S2 discipline):** run once, read the `undocumented` list, classify EACH row against P1–P21 (hex/first-char-bool/`%`-anywhere/atoi-leniency = a gamemd-correct FIX → add to `DIVERGENCES` with the cited reason; anything else = a NEW bug in the accessor → fix the accessor, do not paper it). Re-run until `undocumented` is empty and every `DIVERGENCES` row has a contract citation. The `eprintln!` scans are the input lists handed to the later flip slices' parity re-baseline. **No silent diffs leave this task.**

**Why this is read-only / not hash-relevant:** it builds an `IniFile` and *compares* accessor outputs; it never touches `World`, `state_hash`, `SNAPSHOT_VERSION`, or any sim state. It is a test, not a consumer flip.

**Verification:** `cargo test -p vera20k test_ini_accessor_corpus_parity` (green = every divergence documented + cited; scans surfaced). **Dependency:** T2, T4, T5, T6, T0 (for any SimFixed-quantized double comparison), and the corpus files (`ini/rulesmd.ini`, `ini/artmd.ini`).

---

## Acceptance tests (Slice 1 done = all green, deterministic, named)

| Test (named) | File | Proves | Gate |
|---|---|---|---|
| `test_read_double_precision_matches_gamemd` | `util/fixed_math.rs` | T0: f32-path == f64-path under 16.16 nearest-ties-even quantization, incl. negative + decimal-percent rows. The S0 BLOCKING gate. | T0 |
| `test_read_int_hex` | `ini_value.rs` | `$1A→26`, `1Ah→26`, `0FFH→255`, `$0→0`, `$FF→255` (P1/P2). | T7 |
| `test_read_int_atoi_leniency` | `ini_value.rs` | `5cells→5`, `abc→0`, `-50→-50`, present-empty→0, `  7 →7`, absent→default (P3/P4/P18). | T7 |
| `test_read_int_0x_prefix_is_zero` | `ini_value.rs` | `0x1A→0` (atoi fallback, NOT hex; OQ3). | T7 |
| `test_read_int_signs_and_edges` | `ini_value.rs` | `+9→9`, `-0→0`, `$→0`, `h→0` (edge — review one row, see T9). | T9 |
| `test_read_bool_first_char` | `ini_value.rs` | first-char T/Y/1 vs F/N/0; `off`→default; present-empty→default (P6/P18). | T7 |
| `test_read_double_percent` | `ini_value.rs` | `50%→0.5`, `100%→1.0`, `12.5%→0.125`, `7→7.0`, absent→default (P7). | T7 (needs T0) |
| `test_read_string_trim_default` | `ini_value.rs` | strtrim both ends; absent→default; present-empty→"" (P5/P18). | T7 |
| `test_read_point_comma` | `ini_value.rs` | `"3,5"→(3,5)`, `"1,2,3,4"→rect`, absent→default (P9 COMMA). | T8 |
| `test_read_3int_partial_keeps_default` | `ini_value.rs` | short value keeps default component (P8). | T8 |
| `test_read_color_rgb` | `ini_value.rs` | `"12,34,56"→[12,34,56]`, miss→default (P21). | T8 |
| `test_read_speed_clamp` | `ini_value.rs` | `100→255`, `50→128`, `7→17`, `0→0`, absent→default (P19). | T8 |
| `test_read_range_truncates` | `ini_value.rs` | `5.9→5` (truncate toward zero, never rounds; ledger #18), absent→default (P20). | T8 |
| `test_enum_by_name` | `ini_enum.rs` | `3x3Refinery→9`, case-insensitive, miss→default, substring NO match (P10). | T8 |
| `test_ini_accessor_corpus_parity` | `ini_value.rs` | S2: every divergence vs old accessors over stock `rulesmd.ini`+`artmd.ini` is zero or a cited gamemd-correct fix; 0x / present-empty-transform / cap-32 / exponent scans surfaced. | T10 |

**Full-suite gate:** `cargo test -p vera20k` green; **no consumer flipped, `SNAPSHOT_VERSION` unchanged (=17), runtime `state_hash` untouched** — additive/shadow by construction. Read the literal `test result:` line before reporting pass/fail.

---

## Rollback notes

**No task in this slice flips hashed state**, so there is no `state_hash` / `SNAPSHOT_VERSION` to restore and no parity-harness re-baseline to revert. Rollback for any task = delete the added code:
- T1: revert the two `mod.rs` lines.
- T2–T9: delete `src/rules/ini_value.rs` and `src/rules/ini_enum.rs` (and the T0 test in `fixed_math.rs`).
- T10: delete the `corpus_tests` mod.
Because no consumer reads the service, deleting it cannot change any player-observable output. The risk surface is confined to: (a) the new files failing to compile (caught by `cargo check`), and (b) a wrong test expectation (caught by the test itself before any consumer ever depends on the accessor).

The **hash-flipping rollback discipline applies to the LATER slices** (study-S3..S6) when consumers flip: each such flip that corrects a real drift will change a stock-skirmish replay `state_hash`, requiring a `SNAPSHOT_VERSION` bump (if a serialized stat layout changes) + a one-line cited parity-harness re-baseline per changed value. Those are NOT in this slice. T10's divergence list is the precise input set for them.

---

## Plan-review corrections (2026-06-04, /review-plan — verdict YELLOW→ready)

Reviewed against current `src/` (read-only) + the study doc (binary-cited). All Rust
file:line anchors re-read this run; all binary contract claims cross-checked against
`INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

**C-R1 (FIXED — the one real bug). T0 `"10%0"` row.** The drafted row was `("10%0", true, 0.0)`,
which would make `test_read_double_precision_matches_gamemd` assert a WRONG value and fail
(or, worse, lock in a wrong reference). gamemd `ReadDouble` (`0x005283D0`) does
`sscanf "%f"` → reads `10` (stops at `%`) → `strchr(value,'%')` matches `%` ANYWHERE → ×0.01
→ **0.1**. Corrected the row to `0.1` inline, with the Ghidra/study citation. (Study lines
35, 56, 451: "`strchr` truncates the int arg to a byte 0x25 = `'%'` — functionally contains-any-%".)
Plan assumption 1 RESOLVED: keep the row at 0.1; do NOT drop it.

**C-R2 (CONFIRMED, no change). Binary contract claims all match the study:**
- ReadInt `$xx`/`xxh` + atoi fallback — `0x005276D0` (study L33/54/395). `0x` → atoi stops at
  `x` → 0 (study L200). `test_read_int_0x_prefix_is_zero` correct.
- ReadBool first-char `{1,T,Y}`/`{0,F,N}` else default — `0x005295F0` (study L34/55).
- ReadDouble `%f` widened f32→f64, ×0.01 if `%` anywhere — `0x005283D0` (study L35/56).
- ReadSpeed `ReadInt(-1)`→clamp100→`(v<<8)/100` trunc→clamp255 — `0x00474810` (study L405/461).
  T6 arithmetic verified (100→255, 7→17). Rust `i32 /` truncates toward zero — correct.
- ReadRange `ReadDouble(-1.0)`→`Math__ftol` truncate-toward-zero — `0x00474620`/`0x007c5f00`
  RC=11 (study L406/407). C2 trap correctly flagged: `f64 as i32` (truncate-to-zero), NOT
  `util::sim_to_i32` (`fixed_math.rs:71` `to_num::<i32>()` floors toward −∞). VERIFIED `:71`.
- ReadColorRGB COMMA `%d,%d,%d`, NOT atoi-lenient, no `$`/`h` — `0x00474B50` (study L67/238).
- Enum-by-name whole-string `_stricmp`, default 0 — `FUN_00474DA0` (study L68).
- strtrim ≤0x20 both ends — `0x00727CF0` (study L36/81).

**C-R3 (CONFIRMED via corpus scan). Plan assumptions 3 + 5 RESOLVED:**
- Exponent-notation scan of `ini/rulesmd.ini`+`ini/artmd.ini`: **ZERO** double values use `e`/`E`
  exponent. `parse_leading_f32` needs NO exponent branch for the stock domain (T2 note resolved).
- `$`-prefixed hex values: **ZERO** in stock. `xxh`-suffix on numeric keys: **ZERO**. So no stock
  color triplet has a `$`/`h` component; `read_color_rgb` reusing `atoi_lenient` is faithful to
  `%d` over the entire stock domain (assumption 5 resolved). T10's scans remain as guards.
- The only `0x` hits are `;Foundation=0x0` (all COMMENTED OUT — `from_str` skips `;`-leading lines)
  and `0x0` is a Foundation enum-by-name string (table id 21), never an int read. No live `0x` int
  consumer exists (assumption/OQ3 resolved; T10 `0x` scan will surface zero rows).

**C-R4 (codebase anchors — all confirmed, two cosmetic off-by-ones, NON-BLOCKING):**
All struct fields / fn signatures / types the plan asserts exist AS STATED:
`IniSection::{get:61, get_i32:70, get_f32:77, get_light_f32:86, get_percent:100 (strip_suffix('%'):102),
get_bool:114, get_list:128, get_values:142, set:52 (pub(crate)), keys:165}`; `IniFile::{from_str:197,
merge:304, section:282, section_names:287}`; `mod.rs` alpha-sorted module list :18-44, no
`ini_value`/`ini_enum` present; `foundation.rs` `DEFAULT_FOUNDATION_ID=0:15`, `FOUNDATION_TABLE[22]:17`,
`foundation_def:152` (uses `str::trim` Unicode, fallback id 0), `foundation_id:160`, test
`3x3refinery==9:179`; `fixed_math.rs` `SimFixed=I16F16:23`, `sim_from_f32:85`, `sim_from_f64:92`,
`sim_to_i32:71` (doc "rounds toward zero" is WRONG — floors), `test_sim_from_f64:620-625`;
`snapshot.rs SNAPSHOT_VERSION=17:24` (assumption 7 confirmed at review time); `skirmish_modes.rs:10
include_str!("../ini/mpmodesmd.ini")`; consumers (read-only refs, NOT edited): `weapon_type.rs:185
Range,189 Speed,199 MinimumRange`, `warhead_type.rs:115 Verses,117-120 CellSpread,121-124 PercentAtMax`,
`object_type.rs:870 Speed`. Cosmetic off-by-ones (immaterial — these are read-only references, no edit
anchored on them): `object_type.rs BuildCategory::from_ini` is `:59` (plan says :60);
`ini_parser_tests.rs test_get_percent` is `:209` (plan says :208). No functional impact.

**C-R5 (NON-BLOCKING observation — T10 corpus shape).** Production keeps rules + art as SEPARATE
`IniFile`s (`ruleset.rs:1391` "Retained art.ini registry"); T10 merges them into one via `merge`.
Harmless for a per-key parse-EQUIVALENCE scan (old-accessor vs new-accessor see the same stored
string), but if a section name collides between rules and art, `merge` lets art override — note in
the test that it scans the MERGED view, not two separate views. No correctness impact on the contract.

**C-R6 (CONFIRMED, plan already self-flagged — assumptions 2 + 6).** `read_int` bare `$`/`h`
(`C=$`, `D=h`): the plan's `parse_leading_hex("")==0` path is the safe reading; the study does not
pin gamemd's exact behavior on a digit-less hex value (sscanf reads 0 fields → buffer untouched).
Since NO stock key uses `=$`/`=h` (corpus-confirmed: zero `$`/`h` values), these T9 rows test the
RUST helper's defined behavior, not a gamemd-observable. Acceptable as-is; if a future corpus row
appears, re-verify. Load-time-trim (assumption 6): `from_str` already `;`-strips + Unicode-`.trim()`s
the stored value, so T7's `A=  hello  ` is stored as `"hello"` — the read-side strtrim is exercised
only idempotently. To prove read-side strtrim on raw control chars in isolation, add a fixture via
`IniSection::set` (`pub(crate):52`) from a `rules`-internal test. OPTIONAL coverage, not a gate.

**Residual risk (all LOW):** (a) T0 is a CROSS-PATH-AGREEMENT gate (f32-path == f64-path under
16.16 nearest-ties-even), NOT a bit-identical-to-gamemd gate — correctly framed; the reference rows
use the study's `(double)(float)` intermediate. (b) The plan correctly marks the whole slice
NON-hash-relevant (no consumer flip, `SNAPSHOT_VERSION` stays 17) — verified no task touches
`state_hash`/`World`/`SNAPSHOT_VERSION`. (c) `merge`-view in T10 (C-R5) — cosmetic.

---

## Assumptions the plan-review MUST verify

1. ~~**T0 `"10%0"` reference value**~~ **RESOLVED (C-R1): = 0.1**, fixed inline. ReadDouble `0x005283D0` strchr-anywhere → ×0.01.
2. ~~**`read_int` bare `$`/`h` edge (T9 `D=h`, `C=$`)**~~ **RESOLVED (C-R6):** no stock key uses `=$`/`=h` (corpus-confirmed zero). T9 rows test the Rust helper's defined behavior (`parse_leading_hex("")==0`), acceptable as-is.
3. ~~**Exponent-notation doubles**~~ **RESOLVED (C-R3): ZERO** in `rulesmd.ini`+`artmd.ini`. `parse_leading_f32` needs NO exponent branch for the stock domain.
4. **`include_str!` relative path** — STILL CONFIRM AT WRITE TIME: `src/rules/ini_value.rs` reaches the repo corpus via `../../ini/rulesmd.ini` (two levels up), NOT `../ini/`. (`skirmish_modes.rs` is in `src/` hence its `../ini/`.) Not independently re-derived this review — verify when creating the file.
5. ~~**`atoi_lenient` for `read_color_rgb` vs sscanf `%d`**~~ **RESOLVED (C-R3): ZERO** stock `$`/`h` values exist; reusing `atoi_lenient` is faithful to `%d` over the whole stock domain. T10 scan kept as a guard.
6. **Inline-comment + load-time trim (T7 note)** — CONFIRMED (C-R6): `from_str` `;`-strips + Unicode-trims at load, so read-side strtrim is idempotent in tests. OPTIONAL extra coverage via `IniSection::set` (`pub(crate):52`) — not a gate.
7. ~~**`SNAPSHOT_VERSION` stays 17**~~ **CONFIRMED (C-R4): 17** at `snapshot.rs:24` this review. Re-check at execution time if a parallel slice may have bumped it.
