# House-Color Ramp Parity (D9) Implementation Plan — v2 (post-review rework)

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> **v2 changes (from `/review-plan`):** corrected the data-flow model — `HouseColorIndex`
> becomes the runtime `[Colors]` entry index and stays the render/GPU key; both producers
> (skirmish lobby slot, map `Color=`) resolve to it; the per-house ramp is a runtime table built
> from `[Colors]`; the GPU ramp build is switched from position-indexed to value-indexed (fixes a
> pre-existing latent bug); added the `app_loading` player-scheme consumers and the
> `generate_ramp_from_base` (tiberium) check.

**Goal:** Make per-house unit/building color ramps (palette indices 16..31) and the radar/target-line
color gamemd-exact by replacing the invented `SCHEME_BASES`+`generate_ramp` with gamemd's fixed-hue
trig Saturation/Value sweep over the real `[Colors]` H,S,V, and unifying every house-color producer
onto the `[Colors]`+priority data the loading path already parses correctly.

**Architecture:** Render/rules-side **data-parity + API-consolidation** (second lookup-table substrate
slice; cell-spread shipped first on `cell-spread-substrate`). **No `sim/` changes, no lockstep/
state-hash risk** — house color is cosmetic, per-client. The 8-bit palette-band architecture
(`pal_file` indices [16,32), the GPU/shader universal palette→display conversion) is structurally
correct and matches gamemd — **keep it**; only the *contents* of indices 16..31 and how a house
selects its scheme change.

**Design Doc:** `docs/research/HOUSE_COLOR_REMAP_PIPELINE_GHIDRA_REPORT.md` (verified 2026-06-04;
extends `docs/research/substrate/tables/REMAP_PALETTE_SOUND_SUBSTRATE_STUDY.md`).

---

## Grounding Summary

- **docs/research/** — `HOUSE_COLOR_REMAP_PIPELINE_GHIDRA_REPORT.md` (HIGH): the 16-shade ramp is built
  by `ColorScheme__BuildRampPalette @ 0x0068C3B0` into a scheme's palette indices 16..31 via a
  fixed-hue trig S/V sweep over the `[Colors]` H,S,V; the radar dot samples scheme palette index 16.
- **Ghidra (verified):** ramp loop = 16 iters; shade `i`: `cosAngle=50°+i·(40°/15)`,
  `sinAngle=20°+i·(70°/15)` (i==0 → `11.25°=π/16`); `modV=ftol(cos(cosAngle)·V)`,
  `modS=ftol(sin(sinAngle)·S)`; `(r,g,b)=HSV_to_RGB(H, modS, modV)` (6-sextant integer, identical to
  `color_scheme.rs::hsv_to_rgb`); written to index `i+16`. `ftol`=truncate-toward-zero. Priority LUT
  `{3,11,21,29,13,25,17,15,5}` (`0x0083ED14`); runtime scheme `R` ↔ `[Colors]` entry `R/2` (doubling).
- **Repo reality (verified during review):** `HouseColorIndex(u8)` is the per-house **render identity
  and GPU ramp-row key** — `house_color_to_remap_row(hc)=hc.0+1` (`units.rs:733`). Two producers set
  it: skirmish `HouseColorIndex(slot.color_index)` (`app_skirmish.rs:157`) and map
  `color_index_for_name(Color=)` (`houses.rs:139`). The per-house ramp comes from
  `house_color_ramp(idx)` → static invented `SCHEMES` (`house_colors.rs:75`). `color_scheme.rs` already
  parses `[Colors]` + owns the priority LUT + `/2` doubling + the exact `hsv_to_rgb`, but only feeds the
  loading bar / lobby tint.
- **Latent bug to fix:** `build_house_ramp_bytes` writes row by **array position** (`slot+1`,
  `palette_textures.rs:276`) while units sample by **value** (`hc.0+1`). Equal only while indices are
  dense `0..N`. The rework makes indices `[Colors]`-entry-based (sparse), so the GPU build must become
  value-indexed.
- **INI:** `[Colors]` in `ini/rulesmd.ini` (21 entries, `Name=H,S,V`); map `[<House>] Color=<name>`;
  MP lobby color slot. No new keys.
- **Unknown → flagged (Task 3):** whether the Rust lobby presents colors in gamemd's priority ORDER
  (Gold, DarkRed, DarkBlue, DarkGreen, Orange, DarkSky, Purple, Magenta for priorities 0..7), and
  exactly what `slot.color_index` ranges over. **Deferred:** bit-exact ramp triples (emulate, Task 9).

## Key Technical Decisions

- **`HouseColorIndex` = runtime `[Colors]` entry index (0..N), and stays the render/GPU key.** Both
  producers resolve to it; the GPU row is `hc.0+1`; the ramp content for index N = the trig ramp of
  `[Colors]` entry N. This *unifies* the two disjoint producers instead of "changing the meaning"
  ad hoc. **Confidence:** high (model) / medium (the producer rewrites) — **Source:** report §3/§5 +
  `app_skirmish.rs:157` / `houses.rs:139` / `units.rs:733`.
- **Per-house ramps live in a runtime `HouseColorRamps` table built at load from parsed `[Colors]`,
  replacing the static `SCHEMES`.** `ramp(idx) = build_scheme_ramp(schemes[idx].hsv)`. Owned in
  `rules/house_colors.rs` (rules builds data; render consumes). Consumers borrow `&HouseColorRamps`.
  **Confidence:** high — **Source:** report §1 (ramp is runtime `[Colors]`-derived).
- **Skirmish slot → `[Colors]` entry via the priority LUT + `/2`** (`scheme_entry_for_priority(p) =
  PRIORITY_TO_SCHEME_INDEX[p]/2`; p0..7 → entries {1,5,10,14,6,12,8,7} = Gold/DarkRed/DarkBlue/
  DarkGreen/Orange/DarkSky/Purple/Magenta; random(-2) → 2 = LightGrey). Map `Color=name` → `[Colors]`
  entry by case-insensitive name. **Confidence:** high — `slot.color_index` IS the gamemd priority in
  priority order (confirmed in review via `house_color_tint` `controls.rs:233-256`); no remap needed.
  **Source:** report §4 + `controls.rs:233-256`.
- **Switch `build_house_ramp_bytes` to value-indexed rows** (`row = hc.0+1`), fixing the pre-existing
  position-vs-value inconsistency. **Confidence:** high — **Source:** `units.rs:733` vs
  `palette_textures.rs:276`.
- **`f64` `cos`/`sin` in the builder is fine** (render/rules, not sim; not lockstep). Bit-exact is a
  separate emulate-gated assertion (Task 9). **Confidence:** high — **Source:** CLAUDE.md.
- **Reuse `color_scheme.rs::hsv_to_rgb`** (byte-identical to the report's `HSV_to_RGB`). **Confidence:**
  high — **Source:** report §3.1 vs `color_scheme.rs:94`.
- **Keep `ramp[0]` for radar/target-line** — after the fix, `ramp[0]` = scheme index 16 = gamemd's
  radar color. **Confidence:** high — **Source:** report §3.3/§10.
- **Do NOT wire the ComputeRemap bright triple (+0x56FC) into units/radar** — separate UI color, out of
  scope. **Confidence:** high — **Source:** report §3.4/§10.

## Open Questions

### Resolved During Planning / Review
- *Ramp source?* trig S/V sweep over `[Colors]` HSV (report §3.1).
- *Unit color display-format-dependent?* No — universal 8-bit→display conversion (report §5). Keep RGBA8.
- *Radar dot source?* scheme palette index 16 = corrected `ramp[0]` (report §3.3).
- *Is `HouseColorIndex` the render key?* Yes — GPU ramp row `hc.0+1` (`units.rs:733`); keep it.
- *Who produces `HouseColorIndex`?* skirmish lobby slot (`app_skirmish.rs:157`) + map `Color=`
  (`houses.rs:139`). Both must be re-pointed (Task 3).
- *Is `slot.color_index` the gamemd priority, in priority order?* **YES (confirmed in review):**
  `NormalizedSkirmishSlot.color_index` (`app_skirmish.rs:140`) — `house_color_tint`
  (`controls.rs:233-256`) already maps it via `scheme_for_priority` (LUT + `/2`) for the lobby swatch,
  matching the loading bar. So the Task-3 producer fix `scheme_entry_for_priority(slot.color_index)`
  needs **no slot→priority remap**. (Aside, out of D9 scope: the lobby swatch uses raw
  `hsv_to_rgb(scheme.hsv)` while the unit brightest shade is the trig `ramp[0]` — a minor swatch-vs-unit
  difference; flag only.)

### Deferred to Implementation
- **Exact per-shade RGB** — emulate-gated (Task 9); algorithm-replication tasks ship without it.
- **Default/fallback scheme** for an unknown `Color=` or missing `[Colors]` — Task 3 picks the gamemd
  default (`InitColor` forces ColorSchemeIndex 5 → runtime scheme 5 → `[Colors]` entry 2 = LightGrey);
  confirm at implementation.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/color_scheme.rs` | Add `scheme_entry_for_priority`/`scheme_entry_by_name` (→ `[Colors]` entry index) + `scheme_hsv_by_entry` |
| Modify | `src/rules/house_colors.rs` | Add `build_scheme_ramp(hsv)`; add runtime `HouseColorRamps` (built from `[Colors]`); retire `SCHEME_BASES`/`generate_ramp`/`SCHEMES`/invented `color_index_for_name`; `HouseColorIndex` = `[Colors]` entry index |
| Modify | `src/map/houses.rs:139` | Resolve `Color=name` → `[Colors]` entry index (thread parsed schemes in); default → entry 2 |
| Modify | `src/app_skirmish.rs:157` | Resolve `slot.color_index` → `[Colors]` entry index via `scheme_entry_for_priority` (after Task-3 lobby-order check) |
| Modify | `src/render/palette_textures.rs:263` | `build_house_ramp_bytes` value-indexed (`row=hc.0+1`) from `HouseColorRamps`; `PaletteSet::new` takes `&HouseColorRamps` |
| Modify | `src/render/minimap_helpers.rs:277` | `owner_dot_color` → `HouseColorRamps::ramp(hc)[0]` |
| Modify | `src/app_target_lines.rs:282` | `rally_tint_for_owner` → `HouseColorRamps::ramp(hc)[0]` |
| Modify | `src/app_loading.rs:788-823` | `player_scheme_shade_rgb`/`_bar_rgb`/`_fallback_backing_rgb` → `HouseColorRamps` (loading-screen player color) |
| Modify | `src/app_init_helpers.rs:563` | Build `HouseColorRamps` at load; pass to `PaletteSet::new` (the `active` dedup/sort stays) |
| Verify | `src/assets/pal_file.rs:154`, `src/render/sprite_atlas.rs:1224`, `src/app_instances/*` | Keep `[16,32)` band + `house_color_to_remap_row`; confirm they consume corrected ramps |

## Interface Changes

- **`HouseColorIndex(u8)` semantics change** from "0..8 invented scheme" to "**`[Colors]` entry index**
  (0..N)". It remains the GPU ramp-row key (`house_color_to_remap_row` unchanged). Every producer and
  consumer must agree on this. **Producers:** `app_skirmish.rs:157`, `houses.rs:139`. **Consumers**
  (grep `house_color_ramp`/`HouseColorIndex`/`house_color_to_remap_row`): `render/palette_textures.rs`,
  `render/minimap_helpers.rs`, `render/minimap.rs`, `app_target_lines.rs`, `app_loading.rs`,
  `app_instances/{units,shp,overlays,particles}.rs`, `app_render/build_instances.rs`,
  `render/sprite_atlas.rs`, `render/selection_overlay.rs`, `assets/pal_file.rs`, `bin/mix-browser.rs`.
- **`house_colors.rs`:** REMOVE `SCHEME_BASES`, `SCHEMES`, `generate_ramp`, and the invented branch of
  `color_index_for_name`. ADD `pub fn build_scheme_ramp(hsv:[u8;3])->[Color;16]` and
  `pub struct HouseColorRamps { ramps: Vec<[Color;16]> }` with `fn from_schemes(&[ColorSchemeEntry])`
  and `fn ramp(&self, idx: HouseColorIndex) -> &[Color;16]` (NO_REMAP/out-of-range → default scheme).
  `color_index_for_name` is replaced by `color_scheme::scheme_entry_by_name` (used by `houses.rs`).
- **`color_scheme.rs`:** ADD `scheme_entry_for_priority(i32)->usize` (`= scheme_index_for_priority/2`),
  `scheme_entry_by_name(&[ColorSchemeEntry],&str)->Option<usize>`, `scheme_hsv_by_entry(&[..],usize)->Option<[u8;3]>`.
- **`palette_textures.rs`:** `PaletteSet::new(gpu, palette, ramps: &HouseColorRamps, houses:&[HouseColorIndex])`;
  `build_house_ramp_bytes` writes `row = hc.0+1` per distinct house (value-indexed).
- **`generate_ramp_from_base`** is (per its doc) the **tiberium** ramp — Task 7 greps its callers and
  keeps a tiberium-only path if live; do NOT delete blindly.

## Sim Checklist

Not applicable — **no `sim/` files touched** (confirm in Task 8). `f64` trig allowed (render/rules).

## Risk Areas

- **Blast radius ~21 files** (the `HouseColorIndex` semantic + the `HouseColorRamps` threading). The
  index-space change (0..8 → `[Colors]` entries 0..20) plus the value-indexed GPU rows is the core risk
  — re-point every producer/consumer and run the full suite.
- **Latent position-vs-value GPU bug** is fixed here; verify the value-indexed rows land correctly for
  sparse indices (a 4-player skirmish using `[Colors]` entries {1,5,10,14}).
- **Lobby color order** (Task 3) — if `slot.color_index` isn't gamemd priority order, colors map wrong.
- **Tiberium ramp** (`generate_ramp_from_base`) must not break.
- **Visual regression:** unit/radar colors change to the correct gamemd values — that's the fix; verify
  in-game (Task 10). Existing color golden tests will need updating to the corrected values.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 2 | Trig S/V sweep (cos→V 50→90°, sin→S 20→90°, i0 sin=π/16, `ftol`=trunc, `hsv_to_rgb`) | Wrong curve → every unit's team-color shading wrong, every frame | Structure/monotonicity now; exact-RGB via emulate (Task 9) |
| 3 | Skirmish slot → priority LUT `{3,11,21,29,13,25,17,15,5}` `/2`; map `Color=`→entry by name | Wrong scheme → entirely wrong color | priority 0..7+random → correct `[Colors]` names; map `Color=DarkBlue`→DarkBlue entry |
| 4 | Value-indexed GPU rows (`row=hc.0+1`) | Sparse `[Colors]` indices must land on the right GPU row | 4-player sparse-index skirmish renders correct per-house colors |
| 5 | Radar dot/target line = `ramp(hc)[0]` (= scheme index 16) | Dot must match unit team-color brightest shade | dot == `ramp(hc)[0]` == scheme index 16 |
| 9 | Bit-exact 16 ramp triples (emulate-gated) | Final ±1 RGB parity | `emulate_function 0x0068C3B0` reference |

---

## Tasks

### Task 1: Add `[Colors]` entry-index resolvers to `color_scheme.rs`

**Why:** Both house-color producers must resolve to a `[Colors]` entry index; `color_scheme.rs` owns
the parsed `[Colors]` + the priority LUT + `/2` doubling. Expose entry-index + HSV lookups.

**Files:** Modify `src/rules/color_scheme.rs`.

**Pattern:** mirrors existing `scheme_index_for_priority` / `scheme_for_priority`.

**Step 1: Add the resolvers.**
```rust
/// `[Colors]` entry index for a color priority (the runtime scheme index, un-doubled): random(-2)
/// and 0..=8 go through the LUT then `/2`; higher passthrough/2. (Runtime scheme R ↔ entry R/2.)
pub fn scheme_entry_for_priority(priority: i32) -> usize {
    scheme_index_for_priority(priority) / 2
}

/// `[Colors]` entry index for a map `Color=<name>` (case-insensitive). None if no entry matches.
pub fn scheme_entry_by_name(schemes: &[ColorSchemeEntry], name: &str) -> Option<usize> {
    let want = name.trim();
    schemes.iter().position(|s| s.name.eq_ignore_ascii_case(want))
}

/// H,S,V of a `[Colors]` entry by index.
pub fn scheme_hsv_by_entry(schemes: &[ColorSchemeEntry], entry: usize) -> Option<[u8; 3]> {
    schemes.get(entry).map(|s| s.hsv)
}
```

**Step 2: Tests** (extend `mod tests`, reuse the `schemes()` fixture).
```rust
#[test]
fn scheme_entry_for_priority_matches_lut_div2() {
    // p0→3/2=1 Gold, p1→11/2=5 DarkRed, p2→21/2=10 DarkBlue, p3→29/2=14 DarkGreen, random→5/2=2
    assert_eq!(scheme_entry_for_priority(0), 1);
    assert_eq!(scheme_entry_for_priority(1), 5);
    assert_eq!(scheme_entry_for_priority(2), 10);
    assert_eq!(scheme_entry_for_priority(3), 14);
    assert_eq!(scheme_entry_for_priority(-2), 2);
}

#[test]
fn scheme_entry_by_name_is_case_insensitive() {
    let s = schemes();
    assert_eq!(scheme_entry_by_name(&s, "darkred"), Some(5));
    assert_eq!(scheme_entry_by_name(&s, "Nope"), None);
}
```

**Step 3:** `cargo test -p vera20k -- color_scheme` → PASS. **Step 4:** Commit
(`house-color: add [Colors] entry-index resolvers (priority + name)`).

---

### Task 2: Add `build_scheme_ramp` + `HouseColorRamps` to `house_colors.rs`

**Why:** The core parity fix — gamemd's exact 16-shade ramp from an HSV triple — and a runtime table
holding one ramp per `[Colors]` entry, replacing the static invented `SCHEMES`.

**Files:** Modify `src/rules/house_colors.rs`.

**Step 1: Add the builder** (`f64` trig OK — rules/render, not sim; `ftol`=trunc → `.trunc()` + clamp).
```rust
use crate::rules::color_scheme::{hsv_to_rgb, ColorSchemeEntry};

/// gamemd's per-scheme 16-shade team band (palette indices 16..31): fixed hue H; V rides a cosine
/// 50°→90°, S rides a sine 20°→90° (shade 0 sine = π/16); each (modS,modV) through the 6-sextant
/// integer HSV→RGB. Shade 0 = brightest (the radar/UI/target-line color). `f64` trig matches gamemd's
/// table/x87 to ±1/channel (bit-exact values come from the emulation reference).
pub fn build_scheme_ramp(hsv: [u8; 3]) -> [Color; 16] {
    use std::f64::consts::PI;
    let h = hsv[0];
    let s = hsv[1] as f64;
    let v = hsv[2] as f64;
    let cos_base = 50.0_f64.to_radians();
    let cos_step = (40.0_f64 / 15.0).to_radians();
    let sin_base = 20.0_f64.to_radians();
    let sin_step = (70.0_f64 / 15.0).to_radians();
    let mut ramp = [Color { r: 0, g: 0, b: 0, a: 255 }; 16];
    for (i, slot) in ramp.iter_mut().enumerate() {
        let cos_angle = cos_base + (i as f64) * cos_step;
        let sin_angle = if i == 0 { PI / 16.0 } else { sin_base + (i as f64) * sin_step };
        let mod_v = (cos_angle.cos() * v).trunc().clamp(0.0, 255.0) as u8;
        let mod_s = (sin_angle.sin() * s).trunc().clamp(0.0, 255.0) as u8;
        let [r, g, b] = hsv_to_rgb([h, mod_s, mod_v]);
        *slot = Color { r, g, b, a: 255 };
    }
    ramp
}

/// Runtime per-`[Colors]`-entry ramp table (replaces the static invented `SCHEMES`). Index =
/// `[Colors]` entry index = `HouseColorIndex.0`.
pub struct HouseColorRamps {
    ramps: Vec<[Color; 16]>,
}

/// `[Colors]` entry index used when a house has no resolvable color (gamemd `InitColor` forces
/// ColorSchemeIndex 5 → runtime scheme 5 → entry 2 = LightGrey). Confirm during Task 3.
const DEFAULT_SCHEME_ENTRY: usize = 2;

impl HouseColorRamps {
    pub fn from_schemes(schemes: &[ColorSchemeEntry]) -> Self {
        let ramps = schemes.iter().map(|s| build_scheme_ramp(s.hsv)).collect();
        Self { ramps }
    }
    /// Ramp for a house. NO_REMAP / out-of-range → default scheme (or a flat fallback if empty).
    pub fn ramp(&self, index: HouseColorIndex) -> &[Color; 16] {
        if index != NO_REMAP {
            if let Some(r) = self.ramps.get(index.0 as usize) {
                return r;
            }
        }
        self.ramps.get(DEFAULT_SCHEME_ENTRY).unwrap_or(&FALLBACK_RAMP)
    }
}

/// Flat fallback when `[Colors]` is empty (should not happen in stock play).
static FALLBACK_RAMP: [Color; 16] = [Color { r: 180, g: 180, b: 180, a: 255 }; 16];
```

**Step 2: Tests.**
```rust
#[cfg(test)]
mod ramp_tests {
    use super::*;
    use crate::rules::color_scheme::ColorSchemeEntry;

    fn cs(name: &str, hsv: [u8;3]) -> ColorSchemeEntry { ColorSchemeEntry { name: name.into(), hsv } }

    #[test]
    fn ramp_is_16_opaque_brightest_first() {
        let r = build_scheme_ramp([153, 214, 212]); // DarkBlue
        assert_eq!(r.len(), 16);
        assert!(r.iter().all(|c| c.a == 255));
        let lum = |c: &Color| c.r as u32 + c.g as u32 + c.b as u32;
        assert!(lum(&r[0]) > lum(&r[15]), "shade0 brighter than shade15");
        assert!(r[0].b >= r[0].r && r[0].b >= r[0].g, "blue hue preserved: {:?}", r[0]);
    }

    #[test]
    fn ramps_table_indexes_by_entry_and_falls_back() {
        let schemes = vec![cs("A",[0,230,255]), cs("B",[153,214,212]), cs("C",[0,0,240])];
        let t = HouseColorRamps::from_schemes(&schemes);
        assert_eq!(t.ramp(HouseColorIndex(1)), &build_scheme_ramp([153,214,212]));
        assert_eq!(t.ramp(NO_REMAP), t.ramp(HouseColorIndex(DEFAULT_SCHEME_ENTRY as u8)));
    }
}
```

**Step 3:** `cargo test -p vera20k -- house_colors` (the new `ramp_tests`) → PASS (crate won't fully
build until Task 7 retires `SCHEMES` consumers; run the whole crate after Task 7). **Step 4:** Commit
(`house-color: gamemd trig ramp builder + runtime HouseColorRamps table`).

---

### Task 3: Re-point both producers to `[Colors]` entry indices

**Why:** Replace the disjoint producers (skirmish `slot.color_index`, map `color_index_for_name`) with
resolution to a `[Colors]` entry index, so a `HouseColorIndex` consistently selects the right scheme.

**Files:** Modify `src/app_skirmish.rs` (color-map builder ~L150-159), `src/map/houses.rs:137-140`;
read the lobby color-assignment first.

**Step 1: (DONE in review — slot index = priority, no remap needed.)** Confirmed:
`NormalizedSkirmishSlot.color_index` (`app_skirmish.rs:140`) is the gamemd color priority in priority
order — `house_color_tint` (`controls.rs:233-256`) already resolves it via `scheme_for_priority`
(LUT + `/2`) for the lobby swatch, matching the loading bar. So Step 2 uses
`scheme_entry_for_priority(slot.color_index)` directly with no slot→priority remap. (If a future lobby
reorders its colors, revisit.)

**Step 2: Skirmish producer.** Thread the parsed `[Colors]` schemes (from the ruleset) into the
color-map builder and change `app_skirmish.rs:157` to
`colors.insert(slot.owner_name, HouseColorIndex(scheme_entry_for_priority(slot.color_index as i32) as u8))`.

**Step 3: Map producer.** Thread the schemes into `parse_house_roster` (or resolve in a follow-up pass
that has them), and change `houses.rs:137-140` to resolve `Color=name` via
`scheme_entry_by_name(schemes, name).unwrap_or(DEFAULT_SCHEME_ENTRY)` → `HouseColorIndex`. Drop the
`color_index_for_name` call.

**Step 4: Tests.** Skirmish: slot color 0..7 → entry indices {1,5,10,14,6,12,8,7}; map `Color=DarkBlue`
→ the `[Colors]` "DarkBlue" entry index; missing/unknown → `DEFAULT_SCHEME_ENTRY`. Update the existing
`houses.rs`/`app_skirmish.rs` color tests (they assert old `HouseColorIndex(1)/(2)` values — replace
with the entry-index expectations, using a `[Colors]` fixture).

**Step 5:** `cargo test -p vera20k -- houses skirmish` → PASS. **Step 6:** Commit
(`house-color: producers resolve to [Colors] entry index (skirmish priority LUT + map by-name)`).

---

### Task 4: Build `HouseColorRamps` at load + value-indexed GPU ramp texture

**Why:** Feed the GPU ramp texture (and other consumers) from the `[Colors]`-derived ramps, and fix the
position-vs-value row bug so sparse `[Colors]` indices land on the right rows.

**Files:** Modify `src/render/palette_textures.rs`, `src/app_init_helpers.rs:563-570`.

**Step 1: Build the table at load.** In `app_init_helpers.rs` (where `PaletteSet` is built), construct
`HouseColorRamps::from_schemes(&ruleset.color_schemes)` once and pass `&HouseColorRamps` into
`PaletteSet::new`. (Confirm the ruleset exposes the parsed `[Colors]`; if not, parse via
`parse_color_schemes` at rules load and store it — small add.) Keep the existing `active`
dedup/sort-by-`.0`.

**Step 2: Value-index the GPU rows.** Change `PaletteSet::new` signature to take `ramps:
&HouseColorRamps`, and rewrite `build_house_ramp_bytes(palette, ramps, houses)` so each house writes
its row by **value**:
```rust
// Row 0 = theater palette [16,32) (no-remap fallback) — unchanged.
// For each distinct house index hc, row hc.0+1 = ramps.ramp(hc).
for &hc in houses {
    let row = (hc.0 as usize + 1).min(MAX_HOUSES as usize - 1);
    let ramp = ramps.ramp(hc);
    // write ramp into out[row*row_bytes ..]
}
```
(Replaces the `enumerate()`/`slot+1` position indexing.) Keep `MAX_HOUSES`/`RAMP_SIZE` and the
`rebuild_house_ramps` path (update it the same way).

**Step 3: Tests.** Update `palette_textures` tests: row `hc.0+1` == `ramps.ramp(hc)`; row 0 == theater
`[16,32)`; a sparse-index set (e.g. {1,5,10,14}) writes the correct rows (1-based by value).

**Step 4:** `cargo test -p vera20k -- palette_textures` → PASS. **Step 5:** Commit
(`house-color: value-indexed GPU ramp texture from HouseColorRamps`).

---

### Task 5: Re-point radar dot + target lines

**Why:** `owner_dot_color` and `rally_tint_for_owner` use `house_color_ramp(idx)[0]` (the invented
table). Re-point to `HouseColorRamps::ramp(hc)[0]` — after the fix, `ramp[0]` = scheme index 16 =
gamemd's radar color.

**Files:** Modify `src/render/minimap_helpers.rs:277`, `src/app_target_lines.rs:282`.

**Step 1:** Thread `&HouseColorRamps` to both call sites and replace `house_colors::house_color_ramp(
index)` with `ramps.ramp(index)`. Keep `[0]`. Fix the stale `minimap_helpers.rs:275` doc comment
("middle shade index 8" → "shade 0 = scheme index 16, gamemd radar color").

**Step 2: Test.** `owner_dot_color` for a known house == `build_scheme_ramp(its_hsv)[0]`.

**Step 3:** `cargo test -p vera20k -- minimap` → PASS. **Step 4:** Commit
(`house-color: radar dot + target lines from corrected ramp[0] (scheme index 16)`).

---

### Task 6: Re-point the loading-screen player colors (`app_loading.rs`)

**Why:** `player_scheme_shade_rgb` / `player_scheme_bar_rgb` / `player_scheme_fallback_backing_rgb`
(~`app_loading.rs:788-823`) take `HouseColorIndex` and feed the loading screen — a consumer the v1 plan
missed. They must read the corrected ramps too (or the existing `color_scheme` HSV path if they already
do — verify which).

**Files:** Modify `src/app_loading.rs`.

**Step 1:** Read `app_loading.rs:788-823`. If `player_scheme_*` read the invented `house_color_ramp`/
`SCHEMES`, re-point them to `HouseColorRamps::ramp(idx)` (thread `&HouseColorRamps` to the loading
init). If they already use `color_scheme.rs` HSV (the load bar at `:238` does), confirm the
`color_index`→scheme mapping is consistent with the new entry-index meaning and leave them.

**Step 2: Test/verify.** Loading-screen player color for color index N matches `ramps.ramp(N)` shade.

**Step 3:** `cargo test -p vera20k -- app_loading` (or build) → PASS. **Step 4:** Commit
(`house-color: loading-screen player colors from corrected ramps`).

---

### Task 7: Retire invented data + sweep remaining consumers + tiberium check

**Why:** Remove `SCHEME_BASES`/`generate_ramp`/`SCHEMES`/invented `color_index_for_name`; confirm the
tiberium ramp survives; re-point any remaining `house_color_ramp`/`HouseColorIndex` consumers.

**Files:** Modify `src/rules/house_colors.rs`; sweep the 21-file consumer list.

**Step 1: Tiberium check.** Grep callers of `generate_ramp_from_base` across `src/`. If a tiberium path
uses it, keep a minimal tiberium-only helper (or move it to the tiberium module); if it has no caller,
delete it. Do NOT break tiberium color.

**Step 2: Delete** `SCHEME_BASES`, `SCHEMES`, `generate_ramp`, and the invented branch of
`color_index_for_name` (remove the function if `houses.rs` no longer calls it). Update/replace the
`house_colors.rs` tests that asserted the invented ramps (`test_*_match`,
`test_ramp_brightest_to_darkest`, `test_house_color_ramp_valid`, `test_out_of_range_returns_gold`) with
`HouseColorRamps`-based equivalents.

**Step 3: Sweep.** Grep `house_color_ramp`, `HouseColorIndex`, `house_color_to_remap_row`,
`generate_ramp_from_base` across `src/`; re-point every remaining reference (`app_instances/*`,
`app_render/build_instances.rs`, `render/sprite_atlas.rs`, `render/selection_overlay.rs`,
`render/minimap.rs`, `assets/pal_file.rs`, `bin/mix-browser.rs`, and the `house_color_tint` fallback at
`app_skirmish_shell_render/controls.rs:249`) to the runtime table / new semantics.

**Step 4:** `cargo build -p vera20k` (no dangling refs) + `cargo test -p vera20k` (full). **Step 5:**
Commit (`house-color: retire invented SCHEME_BASES/generate_ramp; unify on HouseColorRamps`).

---

### Task 8: Full regression + clippy + scope check

**Why:** ~21 files changed; confirm nothing else broke and the change is render/rules-only.

**Step 1:** `cargo test -p vera20k` (full) — read the literal `test result:` line; update remaining
color golden assertions to the corrected gamemd values (do not revert the fix).
**Step 2:** `cargo clippy -p vera20k` — no new warnings in touched files.
**Step 3:** `git diff --name-only` — confirm **zero** files under `src/sim/`.
**Step 4:** Commit any golden-value updates (`house-color: update color golden values to gamemd schemes`).

---

### Task 9: Bit-exact ramp reference via emulation (gated, separate pass)

**Why:** `f64` trig can drift ±1/channel from gamemd's table/x87. Generate the exact reference to assert
per-shade RGB. **Separate bounded Ghidra pass — do NOT bury `emulate_function` in other work.**

**Files:** add an exact-RGB test to `src/rules/house_colors.rs`.

**Step 1:** For 2–3 stock schemes (Gold `[43,239,255]`, DarkRed `[0,230,255]`, DarkBlue
`[153,214,212]`), run `emulate_function 0x0068C3B0` (set the HSV input + dest palette buffer) to dump
the 16 output triples gamemd produces.

**Step 2:** Add `build_scheme_ramp_matches_gamemd_emulated`: assert `build_scheme_ramp(hsv)` equals the
emulated reference per channel. If `f64` drifts >0, decide with the user between a ±1 tolerance and
porting gamemd's trig tables; document the choice.

**Step 3:** `cargo test -p vera20k -- house_colors` → PASS (or documented ±1 tolerance). **Step 4:**
Commit (`house-color: exact-RGB ramp parity vs emulated 0x0068C3B0`).

---

### Task 10: gamemd fidelity check (in-game)

**Verify:** Load a skirmish; compare unit team colors + radar dots per player color against gamemd.exe
side-by-side (or `/fidelity-check`). Expected: each color matches the corresponding `[Colors]` scheme
band; the radar dot matches the unit's brightest shade. Record any residual delta (the global RGBA8-vs-
RGB565 precision difference is expected and uniform).

---

## Sources & References

- **Design doc:** `docs/research/HOUSE_COLOR_REMAP_PIPELINE_GHIDRA_REPORT.md` (verified 2026-06-04).
- **gamemd.exe (kept here, not in code comments):** `ColorScheme__BuildRampPalette 0x0068C3B0`,
  `HSV_to_RGB 0x00517440`, `PriorityToColorScheme 0x0069A310` (LUT `0x0083ED14`), `InitColor 0x50B840`
  (scheme `+0x330`=16), `RadarClass__RenderCellPixel 0x00655C50`, angle consts `0x007F0E80`.
- **INI:** `ini/rulesmd.ini` `[Colors] <Name>=H,S,V`; map `[<House>] Color=<name>`.
- **Repo:** `src/rules/house_colors.rs`, `src/rules/color_scheme.rs`, `src/map/houses.rs:137`,
  `src/app_skirmish.rs:157`, `src/render/palette_textures.rs:263`, `src/render/minimap_helpers.rs:277`,
  `src/app_target_lines.rs:282`, `src/app_loading.rs:788`, `src/app_init_helpers.rs:563`,
  `src/app_instances/units.rs:733`.
- **Prior slice:** cell-spread (`cell-spread-substrate`, commit `1266b61e`).
- **Review:** v2 incorporates `/review-plan` findings (HouseColorIndex render-key role, skirmish
  producer, value-indexed GPU rows, `app_loading` consumers, `generate_ramp_from_base` tiberium check).
- **Doc-staleness already corrected in the report:** ComputeRemap multiplier is **240.0**, not 255.
