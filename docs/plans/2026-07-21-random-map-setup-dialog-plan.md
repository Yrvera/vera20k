# Random-Map Setup Dialog (0x105) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
>
> **Revision 2 (2026-07-21)** — all 18 findings from
> `2026-07-21-random-map-setup-dialog-plan-REVIEW.md` are applied. Signatures below were
> read out of the current source; do not "correct" them from memory.

**Goal:** Implement the "Create Random Map" settings dialog (gamemd dialog `0x105`, command
`0x583`) pixel-matched onto the Choose Map (`0x6B`) modal frame, wiring its OK path to write
`RandMap.Sed`, upsert the sentinel, and commit the selection so the already-working `.SED`
launch generator runs.

**Architecture:** `RandomMapSetupControl` + `RandomMapSetupLayout` live in
`ui/skirmish_shell/layout.rs` (beside `ChooseMapModalButton`); the state model lives in
`ui/skirmish_shell/state/random_map_setup.rs`; drawing is a new sprite-batch pass in
`app_skirmish_shell_render/modals.rs`; input rides the existing press==release gesture in
`app.rs`. The generator backend is untouched.

**Design Doc:** `docs/plans/2026-07-21-random-map-setup-dialog-design.md`
**Review:** `docs/plans/2026-07-21-random-map-setup-dialog-plan-REVIEW.md`

---

## Grounding Summary

**Docs.** Four `docs/research/skirmish-ui/` reports: `…0X105_LAYOUT_GEOMETRY…` (exact DLU
rects for all 25 controls from the PE resource), `…SETUP_DIALOG_CONTROLS_OPTIONS…`
(defaults, clamps, state machine, Randomize draw list), `…0X583_IMPLEMENTATION_CONTRACT…`
(result-1 gate, `RandMap.Sed`, sentinel, commit), `…RANDMAP_SED_WRITER_FULL_LAYOUT…`
(`Description` encoding).

**Binary (verified this session, recorded here because the Ghidra bridge may be down later).**
- `Random__RandomRanged @ 0x0065c7e0` is **inclusive on both ends**: `span = max-min`, the
  rejection loop accepts `value <= span`, returns `min + value`. It also swaps when
  `max < min`, and short-circuits to `min` when `min == max`.
- `FUN_00597260` derived-field draw order + the ten range tables (values in Task 2).
- `get_xrefs_to 0x00abed18` / `0x00abed40` → only the read inside `FUN_00597260`; **no
  writer**, so accessibility-min and urban-min are constant `0`.
- `FUN_00596300` case `0x621`: seed is drawn **twice**; order is theater, map type,
  **time**, **resources**, size, derive, description, seed.
- `FUN_00596300` case `0x497`: OK + Save start disabled; Load/Delete from availability; seed
  randomized when `-1`.
- `FUN_00596300` case `0x620`: disables all 13 controls **including Cancel**, then re-enables
  all 13 — **including Load/Delete unconditionally**, bypassing the availability check.
- `FUN_0069a980`: `param_5 → +0x17C` (official), `param_7 → +0x180`, `param_8 → +0x184`;
  `FUN_005e8590` calls it `(…, 1, 0, 2, 4)`.
- `DAT_00817f70` = `0x2c` (`,`); `FUN_00528E00` appends it **after every** code unit → the
  encoded description ends with a trailing comma.
- **`0x6B` and `0x105` do NOT share right-column x coordinates** — see Task 9.

**Repo pattern.** `ChooseMapModalState` (`state/choose_map.rs:21`),
`compute_choose_map_modal_layout` (`layout.rs:559`), `push_choose_map_modal_instances`
(`modals.rs:102`), the `ChooseMapModalButton` match (`app.rs:1330`), and the `{ra2_dir}`
write in `app_options_persist.rs:58-61`.

**INI.** `RMGMD.INI [General]` supplies `RMGVegetationMinimums`/`RMGVegetationMaximums`
(`ini/rmgmd.ini:15-16`), already parsed into `RmgSettings` (`settings.rs:29-30`). **No INI
key exists** for the water/ruggedness/urban/accessibility/region ranges.

## Key Technical Decisions

- **`RandMap.Sed` → `{ra2_dir}`, launch path reused unchanged.** `app_init.rs:353` reads
  `std::fs::read(ra2_dir.join(seed_name))`. **Confidence:** high.
- **A failed write must block the commit.** `app_init.rs:358-361` treats a missing seed file
  as non-fatal and falls back to `RmgOptions::default()`, so a silent failure would launch a
  *default* map. **Confidence:** high.
- **`Description` must join `RmgOptions` and the `.SED` codec** (native writes 17 keys,
  Description first; we write 16). **Confidence:** high — verified in the binary.
- **`RandomRanged` is inclusive.** **Confidence:** high — verified from the function body.
- **The UI RNG instance is deliberately not matched** (`DialogRng`, Task 11). The terrain is
  decided by the seed via the already-exact generator RNG, so this only affects *which*
  random configuration is offered; a separate stream also cannot perturb gameplay randomness.
  **Confidence:** accepted divergence, explicitly named — not "verified equivalent".
- **`0x105` gets its own rect table for BOTH columns.** Inheriting `0x6B`'s right column
  would misplace it 3–5 px. **Confidence:** high — both templates extracted.

## Open Questions

### Resolved During Planning

- Where `RandMap.Sed` goes → `{ra2_dir}` (`app_init.rs:353`).
- The `0x00597260` ranges → extracted, Task 2.
- Accessibility/urban minimums → constant `0`, proven by absence of any writer.
- Are the ranges INI-configurable → only vegetation.
- Does our `.SED` codec match → no, `Description` missing (Task 1).
- Is `RandomRanged` inclusive → yes.
- Do `0x6B` and `0x105` share right-column coordinates → **no** (Task 9).

### Deferred to Implementation

- **Seed edit `0x3FB` visibility.** Style `0x48002000` lacks `WS_VISIBLE`, yet display-sync
  sends it text. Render it disabled/read-only, and confirm against a gamemd screenshot in
  Task 15.
- **Cancel's x coordinate.** `0x105` puts `0x5C0` at x=423 while `0x6B` uses 425, but the
  port's choose-map derives Cancel from `back_rect(screen_w, panel)`, which **ignores the DLU
  input entirely**. Task 9 gives the decision rule.
- **CSF caption text** for the ~14 `GUI:*` keys — resolve through the existing shell CSF
  loader at draw time.

### Resolved After Planning (2026-07-21 Ghidra pass)

- **Sentinel min/max — RESOLVED, keep the repo's `2..8`; change nothing.** Native never
  reads `+0x180`/`+0x184` in any path that decides a player count.
  `MPGameOptions__GetScenarioPlayerCount` (`0x005E653F`) instead counts `[Waypoints]` 0..7
  in the selected file and, finding none (the `.SED` case), reads **`[RandomMap] NumPlayers`**,
  defaulting to `8`. `MPGameOptions__SelectScenario` (`0x005E7C2B`) reads only `+0x58`,
  `+0x15C` and `+0x17C` (official). `FUN_005ed5a0` / `FUN_005ed370` touch neither field.
  So a random map's effective player count **is the dialog's own trackbar value (2..8)**,
  and commit `04029220` was correct. Both the plan's original "2..4" framing *and* the
  "`+0x184` == `player_capacity`" hypothesis were wrong — the `4` is simply not consulted.
- **Map-type gate — RESOLVED, `build.rs:170` is CORRECT.** `RandomMapGenerator__Generate`
  (`0x00598960`) gates the region/bridge block on exactly `map_type == 3 || map_type == 4`,
  matching the port. The apparent conflict was a **naming** problem: `Stage::IslandPasses`
  is a misnomer — 3/4 are *inland* and *mountainous* (the block calls
  `MapClass__MarkBridgesForRepair_Low`), while archipelago is map_type **0** and gets its
  75–100% water from the *normal* water path. Renaming the stage is cosmetic but prevents
  the next reader repeating the mistake.

### Out of Scope — Flag Only (do NOT fix here)

- **`player_capacity` may be wrong.** The port hardcodes
  `player_capacity = RANDOM_MAP_GENERATED_START_QUOTA = 4` (`skirmish_scenarios.rs:23`),
  but native derives a random map's capacity from `[RandomMap] NumPlayers`. Choosing 8
  players may therefore leave the row advertising 4. Separate investigation.
- **Three generator details worth checking against the port** (backend, not this plan):
  1. **Water gate.** Native: `if (map_type == 3 || map_type == 4) { if (water_amount != 0)
     special_water(); } else { normal_water(); }`, then a common pass. The `!= 0` guard
     exists **only** on the 3/4 branch — if the port lacks it, that is drift.
  2. **Tech buildings** run only when `map_type != 0` — archipelago gets none.
  3. **Rocks** are gated on **theater** (`+0x38 == 0`), not map type. The port already
     matches this; recorded so it is not "fixed" by mistake.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/map/rmg/options.rs` | `description` field + comma-hex UTF-16 `.SED` codec |
| Create | `src/map/rmg/randomize.rs` | `RandomRanged`, range tables, `derive_from_map_type`, `randomize` |
| Modify | `src/map/rmg/mod.rs` | `pub mod randomize;` (module root — **never rustfmt**) |
| Modify | `src/ui/skirmish_shell/layout.rs` | stale comment; `RandomMapSetupControl`; `0x105` constants; layout + hit-test |
| Create | `src/ui/skirmish_shell/state/random_map_setup.rs` | state model + state machine |
| Modify | `src/ui/skirmish_shell/state.rs` | `mod` + `pub use` (module root — **never rustfmt**) |
| Modify | `src/ui/skirmish_shell/state/player_name.rs` | `SkirmishShellState.random_map_setup_modal` field + default |
| Modify | `src/ui/skirmish_shell/mod.rs` | add new items to the `pub use` allow-lists (module root — **never rustfmt**) |
| Modify | `src/app_skirmish_shell_render/modals.rs` | background entry + draw pass |
| Modify | `src/app_skirmish_shell_render.rs` | draw + text dispatch (module root — **never rustfmt**) |
| Modify | `src/app.rs` | `0x583` opens modal; handlers; accept/cancel |
| Modify | `src/ui/skirmish_shell/state/tests.rs` | sentinel + cancel tests |

**Module roots — do NOT `rustfmt` (they recurse into submodules):** `src/map/rmg/mod.rs`,
`src/ui/skirmish_shell/mod.rs`, `src/ui/skirmish_shell/state.rs`,
`src/app_skirmish_shell_render.rs`. Format only leaf files you edited.

## Interface Changes

- `RmgOptions` gains `pub description: String`. **Exactly two** struct literals break, both
  exhaustive test literals in `src/map/rmg/options.rs` (`:175`, `:262`); every other
  construction already uses `..Default::default()`, and `app_init.rs:352` is `::default()`.
- New items — **all must be added to the `pub use` allow-lists in
  `src/ui/skirmish_shell/mod.rs`** or they are unreachable from `app.rs` and the render
  layer: `layout::{RandomMapSetupControl, RandomMapSetupLayout, compute_random_map_setup_layout,
  random_map_setup_control_at}`, `state::{RandomMapSetupModalState, AcceptOutcome, SetupCombo}`.
- No existing signature changes. No `sim/` involvement.

## Sim Checklist

Not applicable — no task touches `sim/`.

## Risk Areas

| Risk | Mitigation |
|---|---|
| Inheriting `0x6B`'s right column misplaces it 3–5 px | Task 9 defines `0x105`'s own constants; test asserts they differ from choose-map |
| Adding `description` breaks struct literals | Task 1 Step 4 names the exact two sites |
| Silent `.SED` write failure → default map at launch | Task 12 keeps the modal open on error |
| New modal steals input from choose-map | Task 11 inserts the gate between the validation and choose-map gates |
| `modal` borrow vs `state` reborrow (E0499) | Task 11 copies the selection out inside the match arm (`ChooseMapSelection` is `Copy`) |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | `Description` first, comma-hex UTF-16 **with trailing comma** | Native writes 17 keys; the description is the sentinel's row name | Test vs `52,61,6e,64,6f,6d,20,4d,61,70,` |
| 2 | The ten derived range tables | Decide water/ruggedness/urban/accessibility/region for every randomized map | Values read from `0x0082b080`; zero-mins proven by absent writer |
| 2 | RNG draw **order** | Any reorder desynchronizes the whole downstream stream | Scripted-RNG test asserts the exact sequence |
| 3 | Seed drawn **twice** | Dropping the inner draw shifts every later value | Scripted-RNG test counts draws |
| 3 | `theater = RandomRanged(0,100) > 0x31` → only 0 or 1 | Randomize must never pick urban/desert. Visible in the combo every click | Test across the full input range |
| 6 | OK starts **disabled**; seed randomized on open | First thing the player sees; seed 0 would make every fresh dialog identical | State test |
| 7 | Any option change re-disables OK | Visible gating on every edit | State test |
| 8 | Generate disables **every** control incl. Cancel | Native blocks all input during the synchronous run | State test |
| 8 | Generate enables Load/Delete **unconditionally** | Native quirk: bypasses the availability check | State test |
| 9 | `0x105`'s own right-column x (422/423/430), NOT `0x6B`'s (425/428) | 3–5 px visible drift | Test asserts they differ from the choose-map layout |
| 9 | Label heights `[14,14,12,12,12,14]` | Three rows would be 2 DLU too tall | Per-row rect test |
| 9 | Players is a **trackbar** | Wrong widget class is immediately visible | Draw review + Task 15 |
| 12 | Sentinel: `RandMap.Sed`, official, exactly one row | A duplicate row is visible in the map list | Test asserts a single row |
| 13 | Cancel preserves the previous selection | Cancel must have zero side effects | Test asserts selection + no write |

---

## Tasks

### Task 1: Add `Description` to `RmgOptions` and the `.SED` codec

**Why:** Native emits 17 keys with `Description` first; we emit 16. The description becomes
the sentinel's displayed row name. Foundation for Tasks 3 and 12.

**Files:** Modify `src/map/rmg/options.rs`

**Step 1: Add the field.** In `pub struct RmgOptions` after `seed`:
```rust
    /// Display description. Stored in `.SED` as comma-separated hex UTF-16 code
    /// units, and used as the random-map row's displayed name.
    pub description: String,
```
In `impl Default for RmgOptions` add `description: String::new(),`.
(`RmgOptions` derives `Debug, Clone, PartialEq, Eq` — `String` satisfies all three.)

**Step 2: Add the codec.** Above `impl RmgOptions`:
```rust
/// Encode a description as the original's comma-separated hex UTF-16 code units.
/// Each code unit is lowercase hex followed by a comma, including a trailing one.
fn encode_description(text: &str) -> String {
    let mut out = String::new();
    for unit in text.encode_utf16() {
        out.push_str(&format!("{unit:x}"));
        out.push(',');
    }
    out
}

/// Decode the comma-separated hex UTF-16 form. Unparsable tokens are skipped,
/// matching the original's tolerant tokenizer.
fn decode_description(raw: &str) -> String {
    let units: Vec<u16> = raw
        .split(',')
        .filter_map(|token| {
            let token = token.trim();
            (!token.is_empty())
                .then(|| u16::from_str_radix(token, 16).ok())
                .flatten()
        })
        .collect();
    String::from_utf16_lossy(&units)
}
```

**Step 3: Wire read and write.** In `apply_sed`, after the `let mut read = …` closure and
before `read("Width", …)`:
```rust
        if let Some(raw) = section.get("Description") {
            self.description = decode_description(raw);
        }
```
In `to_sed_bytes`, make `Description` the **first** entry of `values`:
```rust
            ("Description", encode_description(&self.description)),
```
The remaining 16 entries keep their current order.

**Step 4: Fix the two broken literals.** `cargo check -p vera20k`. Exactly two exhaustive
struct literals break — `src/map/rmg/options.rs:175` and `src/map/rmg/options.rs:262`, both
in tests. Add `description: String::new(),` to each (or `..Default::default()`). Nothing
else needs changing: every other `RmgOptions { … }` already ends with `..Default::default()`,
and `app_init.rs:352` is `RmgOptions::default()`, not a literal.

**Step 5: Add tests.** Append to `mod tests`:
```rust
    #[test]
    fn description_encodes_to_the_native_comma_hex_form() {
        assert_eq!(
            encode_description("Random Map"),
            "52,61,6e,64,6f,6d,20,4d,61,70,"
        );
    }

    #[test]
    fn description_decodes_the_native_form() {
        assert_eq!(
            decode_description("52,61,6e,64,6f,6d,20,4d,61,70,"),
            "Random Map"
        );
    }

    #[test]
    fn description_round_trips_through_sed() {
        let mut original = RmgOptions {
            description: "Random Map".to_string(),
            seed: 1234,
            ..Default::default()
        };
        original.normalize();
        let bytes = original.to_sed_bytes();
        let mut parsed = RmgOptions::default();
        parsed.apply_sed(&IniFile::from_bytes(&bytes).unwrap());
        parsed.normalize();
        assert_eq!(parsed.description, "Random Map");
        assert_eq!(parsed, original);
    }

    #[test]
    fn description_is_the_first_emitted_key() {
        let bytes = RmgOptions::default().to_sed_bytes();
        let text = String::from_utf8(bytes).expect("ini is utf-8");
        let body = text.split("[RandomMap]").nth(1).expect("section present");
        let first = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        assert!(first.starts_with("Description"), "got {first:?}");
    }

    #[test]
    fn malformed_description_tokens_are_skipped() {
        assert_eq!(decode_description("52,zz,61,"), "Ra");
    }
```

**Step 6: Verify.** `cargo test -p vera20k description -- --nocapture` → 5 PASS. Read the
literal `test result:` line.

**Step 7:** `rustfmt --edition 2024 src/map/rmg/options.rs`; commit
`rmg/sed: carry Description through the options model and .SED codec`

---

### Task 2: Derived-range tables and `derive_from_map_type`

**Why:** Randomize fills nine fields from per-map-type ranges. These tables and their draw
order are the parity core.

**Files:** Create `src/map/rmg/randomize.rs`; modify `src/map/rmg/mod.rs` (add
`pub mod randomize;` between `pipeline` and `rng` in the alphabetical list at `mod.rs:7-18`).

**Step 1: Header, RNG trait, tables**
```rust
//! The dialog-time randomizer: the `Surprise Me` option draws plus the
//! per-map-type derived-field ranges.
//!
//! Separate from the generator's seeded RNG — this runs while the player is
//! still editing options and only decides which configuration appears.

use super::options::RmgOptions;
use super::settings::RmgSettings;

/// Inclusive uniform draw, matching the original's range helper on both ends.
pub trait RandomRanged {
    /// Returns a value in `[min, max]` inclusive.
    fn ranged(&mut self, min: i32, max: i32) -> i32;
}

/// Map-type buckets: archipelago, continent, team continent, inland, mountainous.
const MAP_TYPES: usize = 5;

// Per-map-type derived ranges. Vegetation is absent: it comes from `RMGMD.INI`
// via `RmgSettings`. Urban and accessibility minimums are zero-initialised
// storage that nothing ever writes.
const WATER_MIN: [i32; MAP_TYPES] = [75, 0, 50, 0, 0];
const WATER_MAX: [i32; MAP_TYPES] = [100, 25, 100, 100, 100];
const RUGGEDNESS_MIN: [i32; MAP_TYPES] = [20, 20, 20, 20, 20];
const RUGGEDNESS_MAX: [i32; MAP_TYPES] = [100, 100, 100, 100, 100];
const URBAN_MIN: [i32; MAP_TYPES] = [0, 0, 0, 0, 0];
const URBAN_MAX: [i32; MAP_TYPES] = [50, 100, 100, 100, 0];
const ACCESSIBILITY_MIN: [i32; MAP_TYPES] = [0, 0, 0, 0, 0];
const ACCESSIBILITY_MAX: [i32; MAP_TYPES] = [100, 100, 100, 100, 20];
const REGION_SIZE_MIN: [i32; MAP_TYPES] = [50, 0, 35, 0, 0];
const REGION_SIZE_MAX: [i32; MAP_TYPES] = [100, 100, 100, 100, 50];

/// The resource option is scaled by this to produce the tiberium amount.
const TIBERIUM_PER_RESOURCE_STEP: i32 = 0x14;
```

**Step 2: `derive_from_map_type`.** The draw order is load-bearing — do not reorder.
```rust
/// Fill the derived fields from the map type, exactly in the original's order.
///
/// Consumes eight RNG draws: water, ruggedness, urban, accessibility, region
/// size, tiberium layout, vegetation, seed. Tiberium is computed, not drawn.
pub fn derive_from_map_type(
    options: &mut RmgOptions,
    settings: &RmgSettings,
    rng: &mut impl RandomRanged,
) {
    let bucket = options.map_type.clamp(0, MAP_TYPES as i32 - 1) as usize;

    options.water_amount = rng.ranged(WATER_MIN[bucket], WATER_MAX[bucket]);
    options.ruggedness = rng.ranged(RUGGEDNESS_MIN[bucket], RUGGEDNESS_MAX[bucket]);
    options.urban_presence = rng.ranged(URBAN_MIN[bucket], URBAN_MAX[bucket]);
    options.accessibility = rng.ranged(ACCESSIBILITY_MIN[bucket], ACCESSIBILITY_MAX[bucket]);
    options.region_size = rng.ranged(REGION_SIZE_MIN[bucket], REGION_SIZE_MAX[bucket]);

    options.tiberium = options.resources * TIBERIUM_PER_RESOURCE_STEP;
    options.tiberium_layout = rng.ranged(0, 100);

    // Vegetation bounds are INI-driven and individually clamped; an inverted
    // pair collapses the minimum onto the maximum rather than erroring.
    let mut min = settings.vegetation_min[bucket].clamp(0, 100);
    let max = settings.vegetation_max[bucket].clamp(0, 100);
    if max < min {
        min = max;
    }
    options.vegetation = rng.ranged(min, max);

    options.seed = rng.ranged(0, 0xFFFF);
}
```

**Step 3: Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Records every draw and returns scripted values in order.
    struct ScriptedRng {
        values: Vec<i32>,
        calls: Vec<(i32, i32)>,
    }

    impl ScriptedRng {
        fn new(values: Vec<i32>) -> Self {
            Self { values, calls: Vec::new() }
        }
    }

    impl RandomRanged for ScriptedRng {
        fn ranged(&mut self, min: i32, max: i32) -> i32 {
            self.calls.push((min, max));
            let index = self.calls.len() - 1;
            *self.values.get(index).unwrap_or(&min)
        }
    }

    #[test]
    fn derive_draws_in_the_original_order_with_the_right_ranges() {
        let mut options = RmgOptions { map_type: 4, resources: 3, ..Default::default() };
        let mut rng = ScriptedRng::new(vec![0; 8]);
        derive_from_map_type(&mut options, &RmgSettings::default(), &mut rng);

        // Mountainous (bucket 4) ranges, in draw order.
        assert_eq!(
            rng.calls,
            vec![
                (0, 100),   // water
                (20, 100),  // ruggedness
                (0, 0),     // urban
                (0, 20),    // accessibility
                (0, 50),    // region size
                (0, 100),   // tiberium layout
                (0, 0),     // vegetation (default settings are 0/0)
                (0, 0xFFFF) // seed
            ]
        );
    }

    #[test]
    fn tiberium_is_resources_times_twenty_and_is_not_drawn() {
        let mut options = RmgOptions { map_type: 1, resources: 3, ..Default::default() };
        let mut rng = ScriptedRng::new(vec![0; 8]);
        derive_from_map_type(&mut options, &RmgSettings::default(), &mut rng);
        assert_eq!(options.tiberium, 60);
        assert_eq!(rng.calls.len(), 8, "tiberium must not consume a draw");
    }

    #[test]
    fn archipelago_is_water_heavy_and_continent_is_not() {
        assert_eq!((WATER_MIN[0], WATER_MAX[0]), (75, 100));
        assert_eq!((WATER_MIN[1], WATER_MAX[1]), (0, 25));
    }

    #[test]
    fn inverted_vegetation_bounds_collapse_onto_the_maximum() {
        let settings = RmgSettings {
            vegetation_min: [80; 5],
            vegetation_max: [30; 5],
            ..Default::default()
        };
        let mut options = RmgOptions { map_type: 0, ..Default::default() };
        let mut rng = ScriptedRng::new(vec![0; 8]);
        derive_from_map_type(&mut options, &settings, &mut rng);
        // Vegetation is draw #7 (index 6): min collapsed from 80 down to 30.
        assert_eq!(rng.calls[6], (30, 30));
    }

    #[test]
    fn out_of_range_map_type_clamps_into_the_table() {
        let mut options = RmgOptions { map_type: 99, ..Default::default() };
        let mut rng = ScriptedRng::new(vec![0; 8]);
        derive_from_map_type(&mut options, &RmgSettings::default(), &mut rng);
        assert_eq!(rng.calls[0], (WATER_MIN[4], WATER_MAX[4]));
    }
}
```

**Step 4: Verify.** `cargo test -p vera20k derive -- --nocapture` → 5 PASS.

**Step 5:** `rustfmt --edition 2024 src/map/rmg/randomize.rs` (**not** `mod.rs`); commit
`rmg/randomize: derived-field ranges and draw order from the binary tables`

---

### Task 3: `randomize()` — the Surprise Me subset

**Why:** The button's own draws sit on top of Task 2's derived fields.

**Files:** Modify `src/map/rmg/randomize.rs`

**Step 1: Add**
```rust
/// Theater draw threshold: values above this select the second theater, so the
/// randomizer only ever produces the first two theaters.
const THEATER_THRESHOLD: i32 = 0x31;

/// Randomize the option set the way the dialog's Surprise Me button does.
///
/// Draw order: theater, map type, time, resources, size, then the derived
/// fields, then a second seed draw that supersedes the derived one.
pub fn randomize(
    options: &mut RmgOptions,
    settings: &RmgSettings,
    rng: &mut impl RandomRanged,
    description: &str,
) {
    options.theater = i32::from(rng.ranged(0, 100) > THEATER_THRESHOLD);
    options.map_type = rng.ranged(1, 4);
    options.time = rng.ranged(0, 3);
    options.resources = rng.ranged(0, 3);
    // One draw drives both size axes.
    let size = rng.ranged(0, 3);
    options.width = size;
    options.height = size;

    derive_from_map_type(options, settings, rng);

    options.description = description.to_string();
    // The derived pass already wrote a seed; the button draws again over it.
    options.seed = rng.ranged(0, 0xFFFF);

    options.normalize();
}
```

**Step 2: Tests** (append inside `mod tests`)
```rust
    #[test]
    fn randomize_only_ever_picks_the_first_two_theaters() {
        for value in 0..=100 {
            let mut options = RmgOptions::default();
            let mut rng = ScriptedRng::new(vec![value, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            randomize(&mut options, &RmgSettings::default(), &mut rng, "");
            assert!(
                options.theater == 0 || options.theater == 1,
                "theater {} out of range for draw {value}",
                options.theater
            );
        }
    }

    #[test]
    fn randomize_theater_flips_above_the_threshold() {
        let script = |first: i32| {
            let mut options = RmgOptions::default();
            let mut rng = ScriptedRng::new(vec![first, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            randomize(&mut options, &RmgSettings::default(), &mut rng, "");
            options.theater
        };
        assert_eq!(script(0x31), 0, "at the threshold stays on theater 0");
        assert_eq!(script(0x32), 1, "just above the threshold flips");
    }

    #[test]
    fn randomize_writes_both_size_axes_from_one_draw() {
        let mut options = RmgOptions::default();
        let mut rng = ScriptedRng::new(vec![0, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        randomize(&mut options, &RmgSettings::default(), &mut rng, "");
        assert_eq!((options.width, options.height), (2, 2));
        assert_eq!(rng.calls[4], (0, 3), "size is a single 0..3 draw");
    }

    #[test]
    fn randomize_draws_the_seed_twice() {
        let mut options = RmgOptions::default();
        let mut rng = ScriptedRng::new(vec![0; 16]);
        randomize(&mut options, &RmgSettings::default(), &mut rng, "");
        let seed_draws = rng.calls.iter().filter(|c| **c == (0, 0xFFFF)).count();
        assert_eq!(seed_draws, 2, "derived pass and the button each draw a seed");
        // 5 button draws + 8 derived draws + 1 final seed draw.
        assert_eq!(rng.calls.len(), 14);
    }

    #[test]
    fn randomize_draws_time_before_resources() {
        let mut options = RmgOptions::default();
        // theater, map type, time=2, resources=1, size
        let mut rng = ScriptedRng::new(vec![0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        randomize(&mut options, &RmgSettings::default(), &mut rng, "");
        assert_eq!(options.time, 2);
        assert_eq!(options.resources, 1);
    }

    #[test]
    fn randomize_sets_the_description_and_normalizes() {
        let mut options = RmgOptions::default();
        let mut rng = ScriptedRng::new(vec![0; 16]);
        randomize(&mut options, &RmgSettings::default(), &mut rng, "Random Map");
        assert_eq!(options.description, "Random Map");
        assert!(options.tiberium >= 1, "normalize floors tiberium at 1");
    }
```

**Step 3: Verify.** `cargo test -p vera20k randomize -- --nocapture` → 6 PASS.

**Step 4:** `rustfmt --edition 2024 src/map/rmg/randomize.rs`; commit
`rmg/randomize: Surprise Me draw order incl. the double seed draw`

---

### Task 4: Fix the stale DLU comment

**Why:** Independent and isolated. Done **before** Task 9 so its line anchor is still valid
(Task 9 inserts ~35 lines above it).

**Files:** Modify `src/ui/skirmish_shell/layout.rs:41-44`

**Step 1:** Replace the `VALIDATION_MODAL_W/H` comment (currently claiming a
"300x200-DLU template") with:
```rust
// Validation popup child pixel size, using the 6x13 dialog-unit base. The
// parent chooser resource is a 533x369-DLU template; the exact post-creation
// client size for this child popup has not been captured, so treat the derived
// size as unconfirmed pending a native GetClientRect/screenshot.
```

**Step 2:** `cargo check -p vera20k` → compiles (comment-only).

**Step 3:** Commit `ui/skirmish: correct the stale 300x200-DLU note (template is 533x369)`

---

### Task 5: Control enum and state types

**Why:** Types before behavior.

**Files:**
- Modify `src/ui/skirmish_shell/layout.rs` (the **enum**, beside `ChooseMapModalButton` at
  `layout.rs:159-164`)
- Create `src/ui/skirmish_shell/state/random_map_setup.rs`
- Modify `src/ui/skirmish_shell/state.rs`

**Pattern:** `ChooseMapModalButton` lives in `layout.rs` and `state.rs` imports it *from*
layout (`state.rs:53`). Follow that direction — the enum goes in `layout.rs`, **not** in
`state/`.

**Step 1: The enum, in `layout.rs`** beside `ChooseMapModalButton`:
```rust
/// Every interactive control in the random-map setup dialog `0x105`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomMapSetupControl {
    MapType0x405,
    Time0x3ea,
    Theater0x407,
    Size0x406,
    Resources0x408,
    Players0x3eb,
    Randomize0x621,
    Generate0x620,
    Ok0x6c5,
    Load0x6c2,
    Save0x6c3,
    Delete0x6c4,
    Cancel0x5c0,
}
```

**Step 2: The state module** — `src/ui/skirmish_shell/state/random_map_setup.rs`:
```rust
//! Random-map setup modal state (the Create Random Map dialog).
//!
//! Render-agnostic: owns the working option set, the enable/disable state
//! machine, and the accept/cancel outcome. Depends on the rmg options model
//! and the layout control enum only — no assets, no wgpu.

use crate::map::rmg::options::RmgOptions;
use crate::map::rmg::randomize::{RandomRanged, randomize};
use crate::map::rmg::settings::RmgSettings;

use super::super::layout::RandomMapSetupControl;
use super::choose_map::ChooseMapSelection;

/// Which combo is currently dropped open, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCombo {
    MapType,
    Time,
    Theater,
    Size,
    Resources,
}

/// What closing the dialog should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Accepted: commit these options.
    Commit(Box<RmgOptions>),
    /// Rejected because nothing has been generated yet.
    NeedsGenerate,
}

/// The Create Random Map dialog.
#[derive(Debug, Clone)]
pub struct RandomMapSetupModalState {
    /// The working option set the controls edit.
    pub options: RmgOptions,
    /// True once Generate has run and no option has changed since.
    pub generated: bool,
    /// True while the synchronous generate block owns the dialog.
    pub generating: bool,
    /// Load/Delete enablement. Starts at saved-seed availability, but the
    /// generate action turns it on unconditionally, as the original does.
    pub saved_seed_buttons_enabled: bool,
    pub open_combo: Option<SetupCombo>,
    pub pressed_control: Option<RandomMapSetupControl>,
    /// Restored verbatim if the player cancels.
    pub previous_selection: Option<ChooseMapSelection>,
}
```

**Step 3: Register it.** `state.rs` declares submodules as **private `mod`** and re-exports
selectively (`state.rs:3-8`, `:27`). Add `mod random_map_setup;` to the list and:
```rust
pub use random_map_setup::{AcceptOutcome, RandomMapSetupModalState, SetupCombo};
```

**Step 4: Export from the shell.** `src/ui/skirmish_shell/mod.rs` uses explicit `pub use`
allow-lists (`:12-31` for layout, `:32+` for state). Add `RandomMapSetupControl` to the
layout list and `RandomMapSetupModalState, AcceptOutcome, SetupCombo` to the state list —
without this they are unreachable from `app.rs`.

**Step 5: Verify.** `cargo check -p vera20k` → compiles (dead-code warnings are fine).

**Step 6:** `rustfmt --edition 2024 src/ui/skirmish_shell/state/random_map_setup.rs` and
`src/ui/skirmish_shell/layout.rs` (**not** `state.rs` or `mod.rs` — module roots). Commit
`ui/skirmish: random-map setup control enum and state types`

---

### Task 6: `open()` and the enable rules

**Why:** The opening state is directly player-visible (OK greyed, seed populated).

**Files:** Modify `src/ui/skirmish_shell/state/random_map_setup.rs`

**Step 1: Implement**
```rust
/// Sentinel meaning "no seed chosen yet"; replaced with a random one on open.
const UNSET_SEED: i32 = -1;

impl RandomMapSetupModalState {
    /// Open the dialog over the current selection.
    ///
    /// An unset seed is replaced with a fresh random one, matching the
    /// original's init. OK starts disabled: the player must generate first.
    pub fn open(
        mut options: RmgOptions,
        previous_selection: Option<ChooseMapSelection>,
        saved_seeds_available: bool,
        rng: &mut impl RandomRanged,
    ) -> Self {
        if options.seed == UNSET_SEED {
            options.seed = rng.ranged(0, 0xFFFF);
        }
        options.normalize();
        Self {
            options,
            generated: false,
            generating: false,
            saved_seed_buttons_enabled: saved_seeds_available,
            open_combo: None,
            pressed_control: None,
            previous_selection,
        }
    }

    /// Whether a control is currently interactive. Every control is inert
    /// during the synchronous generate block, including Cancel.
    pub fn is_enabled(&self, control: RandomMapSetupControl) -> bool {
        use RandomMapSetupControl as C;
        if self.generating {
            return false;
        }
        match control {
            // Accept and save both require a generated result.
            C::Ok0x6c5 | C::Save0x6c3 => self.generated,
            C::Load0x6c2 | C::Delete0x6c4 => self.saved_seed_buttons_enabled,
            _ => true,
        }
    }
}
```

**Step 2: Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Always returns `max`, so draws are identifiable.
    struct MaxRng;
    impl RandomRanged for MaxRng {
        fn ranged(&mut self, _min: i32, max: i32) -> i32 {
            max
        }
    }

    fn opened() -> RandomMapSetupModalState {
        RandomMapSetupModalState::open(RmgOptions::default(), None, false, &mut MaxRng)
    }

    #[test]
    fn open_replaces_the_unset_seed() {
        assert_eq!(RmgOptions::default().seed, -1);
        assert_eq!(opened().options.seed, 0xFFFF, "unset seed is randomized");
    }

    #[test]
    fn open_keeps_an_existing_seed() {
        let options = RmgOptions { seed: 4321, ..Default::default() };
        let state = RandomMapSetupModalState::open(options, None, false, &mut MaxRng);
        assert_eq!(state.options.seed, 4321);
    }

    #[test]
    fn ok_and_save_start_disabled() {
        let state = opened();
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert!(!state.is_enabled(RandomMapSetupControl::Save0x6c3));
    }

    #[test]
    fn load_and_delete_follow_saved_seed_availability_at_open() {
        let none = opened();
        assert!(!none.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(!none.is_enabled(RandomMapSetupControl::Delete0x6c4));

        let some =
            RandomMapSetupModalState::open(RmgOptions::default(), None, true, &mut MaxRng);
        assert!(some.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(some.is_enabled(RandomMapSetupControl::Delete0x6c4));
    }

    #[test]
    fn generate_and_cancel_start_enabled() {
        let state = opened();
        assert!(state.is_enabled(RandomMapSetupControl::Generate0x620));
        assert!(state.is_enabled(RandomMapSetupControl::Cancel0x5c0));
    }
}
```

**Step 3: Verify.** `cargo test -p vera20k random_map_setup -- --nocapture` → 5 PASS.

**Step 4:** `rustfmt --edition 2024 src/ui/skirmish_shell/state/random_map_setup.rs`; commit
`ui/skirmish: random-map setup init state and enable rules`

---

### Task 7: Option mutators and the dirty gate

**Why:** Every edit must re-disable OK — visible on every control the player touches.

**Files:** Modify `src/ui/skirmish_shell/state/random_map_setup.rs`

**Step 1:** Add to `impl RandomMapSetupModalState`:
```rust
    /// Apply an option edit. Any change invalidates the generated result, so
    /// accept is disabled until the next generate.
    pub fn set_map_type(&mut self, value: i32) {
        self.options.map_type = value;
        self.on_option_changed();
    }

    pub fn set_time(&mut self, value: i32) {
        self.options.time = value;
        self.on_option_changed();
    }

    pub fn set_theater(&mut self, value: i32) {
        self.options.theater = value;
        self.on_option_changed();
    }

    /// One size selection drives both axes.
    pub fn set_size(&mut self, value: i32) {
        self.options.width = value;
        self.options.height = value;
        self.on_option_changed();
    }

    pub fn set_resources(&mut self, value: i32) {
        self.options.resources = value;
        self.on_option_changed();
    }

    pub fn set_num_players(&mut self, value: i32) {
        self.options.num_players = value;
        self.on_option_changed();
    }

    fn on_option_changed(&mut self) {
        self.options.normalize();
        self.generated = false;
    }

    /// Surprise Me: randomize the option subset and invalidate the result.
    pub fn randomize_options(
        &mut self,
        settings: &RmgSettings,
        rng: &mut impl RandomRanged,
        description: &str,
    ) {
        randomize(&mut self.options, settings, rng, description);
        self.generated = false;
        self.open_combo = None;
    }
```

**Step 2: Tests**
```rust
    #[test]
    fn changing_an_option_disables_accept_again() {
        let mut state = opened();
        state.generated = true;
        state.set_resources(2);
        assert!(!state.generated, "an edit invalidates the generated result");
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
    }

    #[test]
    fn size_writes_both_axes() {
        let mut state = opened();
        state.set_size(3);
        assert_eq!((state.options.width, state.options.height), (3, 3));
    }

    #[test]
    fn mutators_clamp_through_normalize() {
        let mut state = opened();
        state.set_num_players(99);
        assert_eq!(state.options.num_players, 8);
        state.set_num_players(0);
        assert_eq!(state.options.num_players, 2);
    }

    #[test]
    fn randomize_invalidates_the_generated_result() {
        let mut state = opened();
        state.generated = true;
        state.randomize_options(&RmgSettings::default(), &mut MaxRng, "Random Map");
        assert!(!state.generated);
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert_eq!(state.options.description, "Random Map");
    }
```

**Step 3: Verify.** `cargo test -p vera20k random_map_setup -- --nocapture` → 9 PASS.

**Step 4:** `rustfmt --edition 2024 src/ui/skirmish_shell/state/random_map_setup.rs`; commit
`ui/skirmish: random-map setup option mutators and dirty gate`

---

### Task 8: Generate, accept, and cancel

**Why:** Completes the state machine, including the Load/Delete quirk.

**Files:** Modify `src/ui/skirmish_shell/state/random_map_setup.rs`

**Step 1:** Add to `impl RandomMapSetupModalState`:
```rust
    /// Begin the synchronous generate block: every control goes inert.
    pub fn begin_generate(&mut self) {
        self.generating = true;
        self.open_combo = None;
    }

    /// End the generate block, marking a result available so accept unlocks.
    ///
    /// This also switches Load/Delete on unconditionally: the original
    /// re-enables the whole control set afterwards without re-testing whether
    /// any saved seed actually exists.
    pub fn finish_generate(&mut self) {
        self.generating = false;
        self.generated = true;
        self.saved_seed_buttons_enabled = true;
    }

    /// Accept. The original generates first when nothing has been generated
    /// yet, so a caller receiving `NeedsGenerate` must generate then retry.
    pub fn accept(&self) -> AcceptOutcome {
        if !self.generated {
            return AcceptOutcome::NeedsGenerate;
        }
        let mut committed = self.options.clone();
        committed.normalize();
        AcceptOutcome::Commit(Box::new(committed))
    }

    /// Cancel. Returns the selection to restore; the caller performs no other
    /// side effects. `ChooseMapSelection` is `Copy`.
    pub const fn cancel(&self) -> Option<ChooseMapSelection> {
        self.previous_selection
    }
```

**Step 2: Tests**
```rust
    #[test]
    fn every_control_is_inert_during_generate_including_cancel() {
        let mut state = opened();
        state.begin_generate();
        for control in [
            RandomMapSetupControl::MapType0x405,
            RandomMapSetupControl::Theater0x407,
            RandomMapSetupControl::Players0x3eb,
            RandomMapSetupControl::Randomize0x621,
            RandomMapSetupControl::Generate0x620,
            RandomMapSetupControl::Ok0x6c5,
            RandomMapSetupControl::Cancel0x5c0,
        ] {
            assert!(!state.is_enabled(control), "{control:?} must be inert");
        }
    }

    #[test]
    fn finishing_generate_unlocks_accept() {
        let mut state = opened();
        state.begin_generate();
        state.finish_generate();
        assert!(state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert!(state.is_enabled(RandomMapSetupControl::Cancel0x5c0));
    }

    #[test]
    fn generate_enables_load_and_delete_even_with_no_saved_seeds() {
        // The original re-enables the whole control set after generating
        // without re-testing saved-seed availability.
        let mut state = opened();
        assert!(!state.is_enabled(RandomMapSetupControl::Load0x6c2));
        state.begin_generate();
        state.finish_generate();
        assert!(state.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(state.is_enabled(RandomMapSetupControl::Delete0x6c4));
    }

    #[test]
    fn accept_before_generate_asks_for_generation() {
        assert_eq!(opened().accept(), AcceptOutcome::NeedsGenerate);
    }

    #[test]
    fn accept_after_generate_commits_normalized_options() {
        let mut state = opened();
        state.finish_generate();
        match state.accept() {
            AcceptOutcome::Commit(options) => {
                assert!(options.tiberium >= 1, "committed options are normalized");
                assert_eq!(options.seed, state.options.seed);
            }
            other => panic!("expected a commit, got {other:?}"),
        }
    }
```

**Step 3: Verify.** `cargo test -p vera20k random_map_setup -- --nocapture` → 14 PASS.

**Step 4:** `rustfmt --edition 2024 src/ui/skirmish_shell/state/random_map_setup.rs`; commit
`ui/skirmish: random-map setup generate/accept/cancel semantics`

---

### Task 9: `0x105` layout — its own rect table

**Why:** Geometry before drawing. **`0x105` does NOT share `0x6B`'s right-column x
coordinates** — inheriting them misplaces the column 3–5 px.

**Files:** Modify `src/ui/skirmish_shell/layout.rs`

**Verified signatures — use exactly these:**
```rust
pub fn compute_choose_map_modal_layout(screen_w: u32, screen_h: u32) -> ChooseMapModalLayout
pub fn choose_map_modal_button_at(layout: &ChooseMapModalLayout, x: i32, y: i32) -> Option<ChooseMapModalButton>
pub fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx                                   // ui::shell::geom
pub fn right_panel_rects(screen_w: i32, screen_h: i32) -> RightPanelRects                   // ui::shell::geom
pub fn snap_button_biased_truncate(screen_w: i32, screen_h: i32, source: RectPx, panel: RightPanelRects, cell_w: i32) -> RectPx
fn right_anchor(screen_w: i32, screen_h: i32, original: RectPx) -> RectPx                   // private to layout.rs
fn back_rect(screen_w: i32, panel: RightPanelRects) -> RectPx                               // private to layout.rs
pub const SDBTNANM_W: i32 = 156;
```
`ChooseMapModalLayout` fields (`layout.rs:172-187`): `screen`, `dialog`, `mode_list`,
`map_list`, `use_map_button`, `cancel_button`, `create_random_map_button`, `title`,
`select_engagement`, `game_type_heading`, `game_map_heading`, `status_help`, `preview`.
**There is no `frame` field** — the full-screen rect is `screen`, the modal body is `dialog`.

**Step 1: Constants** (beside the `CHOOSE_MAP_*` block at `layout.rs:36-46`)
```rust
// Random-map setup dialog `0x105`. Same 533x369-DLU frame, font and background
// as choose-map, but its own control rects in BOTH columns: the right column
// sits 2-3 DLU left of choose-map's, and the preview 2 DLU right.
const SETUP_LABEL_X: i32 = 74;
const SETUP_LABEL_W: i32 = 93;
const SETUP_CONTROL_X: i32 = 179;
const SETUP_CONTROL_W: i32 = 150;
/// Row tops for map type, time, theater, size, resources, players.
const SETUP_ROW_Y: [i32; 6] = [41, 65, 90, 114, 138, 163];
/// Label tops; the taller rows sit one DLU above their control.
const SETUP_LABEL_Y: [i32; 6] = [40, 64, 90, 114, 138, 162];
/// Label heights are NOT uniform: rows 0/1/5 are 14 DLU, rows 2/3/4 are 12.
const SETUP_LABEL_H: [i32; 6] = [14, 14, 12, 12, 12, 14];
const SETUP_COMBO_H: i32 = 103;
/// The time combo's dropdown is two DLU shorter than the others.
const SETUP_TIME_COMBO_H: i32 = 101;
const SETUP_TRACKBAR_H: i32 = 13;
/// Randomize / Generate row.
const SETUP_ACTION_Y: i32 = 257;
const SETUP_ACTION_W: i32 = 83;
const SETUP_ACTION_H: i32 = 15;
const SETUP_RANDOMIZE_X: i32 = 74;
const SETUP_GENERATE_X: i32 = 246;
/// Display-only seed field.
const SETUP_SEED_RECT: (i32, i32, i32, i32) = (279, 287, 50, 12);
// Right column - `0x105` values, NOT choose-map's.
const SETUP_RIGHT_X: i32 = 422;
const SETUP_RIGHT_W: i32 = 108;
const SETUP_RIGHT_H: i32 = 23;
const SETUP_TITLE_RECT: (i32, i32, i32, i32) = (422, 1, 108, 10);
const SETUP_PREVIEW_RECT: (i32, i32, i32, i32) = (430, 23, 96, 69);
const SETUP_USE_MAP_Y: i32 = 122;
const SETUP_LOAD_Y: i32 = 149;
const SETUP_SAVE_Y: i32 = 176;
const SETUP_DELETE_Y: i32 = 203;
/// Cancel is one DLU right of the other right-column buttons.
const SETUP_CANCEL_RECT: (i32, i32, i32, i32) = (423, 346, 108, 23);
/// Bottom status line, the one control that matches choose-map exactly.
const SETUP_BLANK_RECT: (i32, i32, i32, i32) = (2, 355, 303, 12);
/// Hidden-until-generating progress widgets.
const SETUP_PROGRESS_TEXT_RECT: (i32, i32, i32, i32) = (74, 219, 150, 11);
const SETUP_PROGRESS_BAR_RECT: (i32, i32, i32, i32) = (229, 217, 100, 21);
```

**Step 2: Layout struct and builder**
```rust
/// Pixel rects for the random-map setup dialog `0x105`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomMapSetupLayout {
    pub screen: RectPx,
    pub dialog: RectPx,
    pub label_rects: [RectPx; 6],
    pub control_rects: [RectPx; 6],
    pub randomize: RectPx,
    pub generate: RectPx,
    pub seed_field: RectPx,
    pub title: RectPx,
    pub preview: RectPx,
    pub use_map: RectPx,
    pub load: RectPx,
    pub save: RectPx,
    pub delete: RectPx,
    pub cancel: RectPx,
    pub blank: RectPx,
    pub progress_text: RectPx,
    pub progress_bar: RectPx,
}

pub fn compute_random_map_setup_layout(screen_w: u32, screen_h: u32) -> RandomMapSetupLayout
```
Mirror `compute_choose_map_modal_layout` (`layout.rs:559-595`) for structure — `screen` and
`dialog` are both `RectPx::new(0, 0, screen_w, screen_h)`; use `right_panel_rects` +
`right_anchor` + `snap_button_biased_truncate` exactly as it does — but feed them the
**`0x105`** base rects above, not choose-map's. Combo heights: rows 0/2/3/4 use
`SETUP_COMBO_H`, row 1 uses `SETUP_TIME_COMBO_H`, row 5 uses `SETUP_TRACKBAR_H`. Labels use
the per-row `SETUP_LABEL_H`.

**Cancel decision rule:** choose-map builds Cancel with `back_rect(screen_w, panel)`, which
**ignores its DLU input**. Compute both `back_rect(screen_w, panel)` and
`right_anchor(screen_w, screen_h, dlu_rect(423, 346, 108, 23))`. If they are equal, use
`back_rect` (shared shell Back geometry). If they differ, use the anchored `0x105` rect and
record the difference in the commit message for Task 15 to check in the side-by-side.

**Step 3: Hit-test** — note the repo takes **two separate `i32`s**, not a tuple:
```rust
/// Topmost interactive control at `(x, y)`, or `None`.
pub fn random_map_setup_control_at(
    layout: &RandomMapSetupLayout,
    x: i32,
    y: i32,
) -> Option<RandomMapSetupControl>
```
Test the control rows first, then the action row, then the right column — the z-order
`choose_map_modal_button_at` (`layout.rs:686-701`) uses.

**Step 4: Tests**
```rust
    #[test]
    fn setup_shares_the_choose_map_frame_but_not_its_right_column() {
        let choose = compute_choose_map_modal_layout(800, 600);
        let setup = compute_random_map_setup_layout(800, 600);
        // The frame is genuinely shared.
        assert_eq!(setup.screen, choose.screen);
        assert_eq!(setup.dialog, choose.dialog);
        // The right column is NOT: 0x105 sits 2-3 DLU left of 0x6B, and the
        // preview 2 DLU right. Inheriting choose-map's rects is the bug this
        // test exists to catch.
        assert_ne!(setup.use_map, choose.use_map_button);
        assert_ne!(setup.title, choose.title);
        assert_ne!(setup.preview, choose.preview);
    }

    #[test]
    fn setup_control_rects_match_800x600_resource_geometry() {
        let setup = compute_random_map_setup_layout(800, 600);
        assert_eq!(setup.control_rects[0], dlu_rect(179, 41, 150, 103));
        assert_eq!(setup.control_rects[1], dlu_rect(179, 65, 150, 101));
        assert_eq!(setup.control_rects[5], dlu_rect(179, 163, 150, 13));
        assert_eq!(setup.randomize, dlu_rect(74, 257, 83, 15));
        assert_eq!(setup.generate, dlu_rect(246, 257, 83, 15));
        assert_eq!(setup.seed_field, dlu_rect(279, 287, 50, 12));
        assert_eq!(setup.blank, dlu_rect(2, 355, 303, 12));
    }

    #[test]
    fn setup_label_heights_are_not_uniform() {
        let setup = compute_random_map_setup_layout(800, 600);
        assert_eq!(setup.label_rects[0], dlu_rect(74, 40, 93, 14));
        assert_eq!(setup.label_rects[2], dlu_rect(74, 90, 93, 12));
        assert_eq!(setup.label_rects[4], dlu_rect(74, 138, 93, 12));
        assert_eq!(setup.label_rects[5], dlu_rect(74, 162, 93, 14));
    }
```

**Step 5: Verify.** `cargo test -p vera20k setup_ -- --nocapture` → 3 PASS.

**Step 6:** `rustfmt --edition 2024 src/ui/skirmish_shell/layout.rs`; commit
`ui/skirmish: 0x105 layout with its own right-column rects`

---

### Task 10: Shell state field and the draw pass

**Why:** The state field is created **here**, before the dispatch that reads it — Task 11
would otherwise be a forward reference.

**Files:**
- Modify `src/ui/skirmish_shell/state/player_name.rs` (this is where `SkirmishShellState` is
  declared — **not** `state.rs`)
- Modify `src/app_skirmish_shell_render/modals.rs`
- Modify `src/app_skirmish_shell_render.rs`

**Step 1: The state field.** `SkirmishShellState` is declared at
`state/player_name.rs:221-267`, with `choose_map_modal: Option<ChooseMapModalState>` at
`:256` and its `Default` init `choose_map_modal: None` at `:302`. Add beside each:
```rust
    pub random_map_setup_modal: Option<RandomMapSetupModalState>,
```
```rust
            random_map_setup_modal: None,
```

**Step 2: Background entry.** `choose_map_background_entry` (`modals.rs:92-100`) is typed to
`&ChooseMapModalLayout`, so it cannot be called with the new layout. Add a sibling that
mirrors its body (same asset, same 800-only gate):
```rust
/// Background for the random-map setup modal. Same asset and same 800-wide-only
/// gate as the choose-map modal - the two dialogs share one background surface.
pub(super) fn random_map_setup_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &RandomMapSetupLayout,
) -> Option<SkirmishShellChromeEntry>
```

**Step 3: The draw pass.** Match the repo's parameter order and visibility — `out` first,
`atlas` second, and **no** `viewport` parameter (screen width comes from `layout.screen.w`):
```rust
pub(super) fn push_random_map_setup_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &RandomMapSetupLayout,
    state: &RandomMapSetupModalState,
)
```
Draw order, mirroring `push_choose_map_modal_instances` (`modals.rs:102-185`):
1. background via `push_entry_native(out, entry, x, y, depth)` (`chrome.rs:173`)
2. full-screen `SHELL_MODAL_BG_RGB` fill + dark bevel outline
3. the five combos and the players trackbar — call
   `paint_control(out, chrome, ControlPaint::Combo { rect, swatch: None, open, disabled })`
   and `ControlPaint::Trackbar { rect, thumb_px }` (`controls.rs:277`, variants at
   `controls.rs:242-270`), with `chrome = atlas.control_chrome()` and `thumb_px` from
   `trackbar_pixel_offset(value, min, max, step, rect)` (`layout.rs:326`).
   **Do not** call `push_combo_instances` / `push_trackbar_instances` — those are hard-wired
   to `SkirmishShellLayout` + `SkirmishShellState` and cannot serve a new dialog.
4. the disabled seed field at `layout.seed_field`
5. Randomize and Generate
6. right column: Use Map, Load, Save, Delete, Cancel — via
   `push_right_panel_button_shp(out, atlas, rect, pressed, disabled, depth)`
   (`chrome.rs:321`), which already resolves SDBTNANM frame 4 pressed / 2 otherwise
7. preview-window outline at `layout.preview` (empty — v1 draws no terrain)
8. **if `state.generating`**, the progress text/bar

Use `state.is_enabled(control)` for `disabled` and
`state.pressed_control == Some(control)` for `pressed`.

**Step 4: Dispatch.** In `app_skirmish_shell_render.rs:203-206` the choose-map branch
**returns immediately**:
```rust
if let Some(choose_map_layout) = choose_map_layout {
    push_choose_map_modal_instances(&mut instances, atlas, choose_map_layout, shell, modes);
    return instances;
}
```
So the setup draw must go **inside that block, before the `return`** — a sibling `if` after
it is dead code. Draw the setup modal after the choose-map instances so it stacks on top.
Add the text pass beside `push_choose_map_modal_text_draws` at
`app_skirmish_shell_render.rs:726-727`: title (`GUI:GenerateMap`), the six labels, button
captions, the seed value, and `GUI:WorkingPleaseWait` while generating.

**Step 5: Verify.** `cargo check -p vera20k` → compiles. Visual check is Task 15.

**Step 6:** `rustfmt --edition 2024 src/app_skirmish_shell_render/modals.rs` and
`src/ui/skirmish_shell/state/player_name.rs` only. **Do NOT rustfmt
`src/app_skirmish_shell_render.rs` — it is a module root** and would reformat `modals.rs`,
`controls.rs`, `chrome.rs`, `preview.rs`. Commit
`ui/skirmish: draw the random-map setup modal`

---

### Task 11: Open the modal from command `0x583`

**Why:** Replaces the log-only stub. The OK/Cancel arms are **stubs here** and are filled in
Tasks 12–13.

**Files:** Modify `src/app.rs`

**Step 1: `DialogRng`.** Define it **before** first use (add to
`src/ui/skirmish_shell/state/random_map_setup.rs` and export it):
```rust
/// Dialog-time RNG. Deliberately separate from the generator's seeded RNG: this
/// only decides which configuration the player is offered, never the terrain,
/// and a separate stream cannot perturb gameplay randomness.
pub struct DialogRng(u32);

impl DialogRng {
    pub fn from_entropy() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0x1234_5678)
                | 1,
        )
    }
}

impl RandomRanged for DialogRng {
    fn ranged(&mut self, min: i32, max: i32) -> i32 {
        // The original swaps an inverted pair; no caller passes one, but match
        // it so the helper is total.
        let (min, max) = if max < min { (max, min) } else { (min, max) };
        if min == max {
            return min;
        }
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        let span = (max - min + 1) as u32;
        min + (self.0 % span) as i32
    }
}
```

**Step 2: Replace the stub.** At `app.rs:1337`, inside the match in
`handle_choose_map_modal_mouse_up`. **Borrow discipline:** `modal` is a live `&mut` borrow of
`state` for the whole match, so anything needing `state` must happen after it ends. Copy the
selection out **inside the arm** (`ChooseMapSelection` is `Copy`):
```rust
                ChooseMapModalButton::CreateRandomMap0x583 => {
                    open_random_map_setup = Some(modal.cancel_selection());
                }
```
Declare `let mut open_random_map_setup: Option<ChooseMapSelection> = None;` beside
`selection_to_commit`, then after the existing `if let Some(selection) = selection_to_commit`
block (where `modal` is no longer used):
```rust
    if let Some(previous) = open_random_map_setup {
        state.skirmish_shell_state.random_map_setup_modal =
            Some(RandomMapSetupModalState::open(
                RmgOptions::default(),
                Some(previous),
                false, // saved-seed browsing is not implemented yet
                &mut DialogRng::from_entropy(),
            ));
    }
```
Note the field path is **`state.skirmish_shell_state`** (not `state.skirmish_shell`).
`cancel_selection` (`choose_map.rs:145`) is `pub const fn … -> ChooseMapSelection` and
returns the selection saved on open — exactly the value to restore on cancel.

**Step 3: Handlers.** Add `handle_random_map_setup_mouse_down` / `_up` mirroring
`handle_choose_map_modal_mouse_down` (`app.rs:1284-1312`) and `_up` (`:1314-1350`): down
records `pressed_control` **only if `is_enabled`**; up fires only when press == release. Get
the layout from a new accessor mirroring `skirmish_choose_map_layout` (`app.rs:589-596`):
```rust
fn skirmish_random_map_setup_layout(state: &AppState) -> RandomMapSetupLayout {
    compute_random_map_setup_layout(state.render_width(), state.render_height())
}
```
Arms:
- `Randomize0x621` → `randomize_options(&settings, &mut DialogRng::from_entropy(), &description)`
- `Generate0x620` → `begin_generate()` then `finish_generate()` (v1 runs no generator)
- combos / trackbar → the Task 7 mutators
- `Ok0x6c5` → **stub: `{}` for now** (Task 12)
- `Cancel0x5c0` → **stub: `{}` for now** (Task 13)
- `Load0x6c2 | Save0x6c3 | Delete0x6c4` → `{}` (saved-seed UX is deferred)

**Step 4: Gate input.** In `handle_skirmish_shell_mouse_down` (`app.rs:1559`, choose-map gate
at `:1563-1565`) and `handle_skirmish_shell_mouse_up` (`:1610`, gate at `:1614-1617`), insert
the setup-modal gate **between** the validation-modal gate and the choose-map gate, so the
setup dialog owns input while open.

**Step 5: Verify.** `cargo check -p vera20k`, then
`cargo test -p vera20k choose_map -- --nocapture` → existing tests still PASS.

**Step 6:** `rustfmt --edition 2024 src/app.rs` and
`src/ui/skirmish_shell/state/random_map_setup.rs`; inspect the diff and revert unrelated
hunks. Commit `ui/skirmish: command 0x583 opens the random-map setup modal`

---

### Task 12: Accept — write `.SED`, upsert sentinel, commit

**Why:** Makes Create Random Map produce a playable map.

**Files:** Modify `src/app.rs`; modify `src/ui/skirmish_shell/state/tests.rs`

**Reuse, don't reimplement.** `ChooseMapModalState::create_random_map`
(`choose_map.rs:149-163`) already does the sentinel upsert **plus** the
`mode.random_maps_allowed` gate and the `refresh_records` call that makes the row appear:
```rust
pub fn create_random_map(
    &mut self,
    records: &mut Vec<SkirmishScenarioRecord>,
    modes: &[SkirmishGameMode],
    display_name: impl Into<String>,
) -> Option<usize>
```
Hand-rolling the upsert drops both — use this.

**Step 1: The commit helper.** Both `commit_choose_map_selection`
(`app.rs:1208`, returns **`bool`**) and `close_choose_map_modal` (`app.rs:1199`) live in
`impl App` (from `app.rs:523`), so `Self::` is right.
```rust
/// The random-map seed file name the launch path recognises.
const RANDMAP_SED_FILE: &str = "RandMap.Sed";

/// Commit accepted random-map setup: persist the seed file, refresh the
/// sentinel record, and select it so launch generates from it.
///
/// A failed write is fatal to the commit: the launch path treats a missing
/// seed file as "use defaults", which would silently start a different map.
fn commit_random_map_setup(state: &mut AppState, options: &RmgOptions) -> anyhow::Result<()> {
    let ra2_dir = state
        .game_config
        .as_ref()
        .map(|config| config.paths.ra2_dir.clone())
        .ok_or_else(|| anyhow::anyhow!("no game config; cannot locate the RA2 directory"))?;
    std::fs::write(ra2_dir.join(RANDMAP_SED_FILE), options.to_sed_bytes())?;

    let display = if options.description.is_empty() {
        "Random Map"
    } else {
        options.description.as_str()
    };
    // Reuse the modal helper: it upserts the sentinel, honours
    // `random_maps_allowed`, and refreshes the filtered record list.
    let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
        return Ok(());
    };
    let index = modal.create_random_map(
        &mut state.skirmish_scenario_records,
        &state.skirmish_game_modes,
        display,
    );
    if let Some(index) = index {
        let mode_id = modal.selected_mode_id;
        // `record_index` is Option<usize>.
        let selection = ChooseMapSelection { mode_id, record_index: Some(index) };
        let _ = Self::commit_choose_map_selection(state, selection);
    }
    Ok(())
}
```
Resolve the two borrows the same way Task 11 does — end the `modal` borrow (copy `index` and
`mode_id` out) before calling `Self::commit_choose_map_selection(state, …)`. Match the actual
field names for the records/modes on `AppState`.

**Step 2: The OK arm**
```rust
    RandomMapSetupControl::Ok0x6c5 => {
        // The original generates first when nothing has been generated yet.
        if matches!(setup.accept(), AcceptOutcome::NeedsGenerate) {
            setup.begin_generate();
            setup.finish_generate();
        }
        if let AcceptOutcome::Commit(options) = setup.accept() {
            commit_options = Some(options);
        }
    }
```
Then, after the `setup` borrow ends:
```rust
    if let Some(options) = commit_options {
        match Self::commit_random_map_setup(state, &options) {
            Ok(()) => {
                state.skirmish_shell_state.random_map_setup_modal = None;
                Self::close_choose_map_modal(state);
            }
            Err(err) => {
                log::error!("random map: could not write {RANDMAP_SED_FILE}: {err}");
                // Stay open: committing now would launch a default map.
            }
        }
    }
```

**Step 3: Sentinel test — assert CURRENT behavior.** The fields are `Option<u8>`
(`skirmish_scenarios.rs:41-57`), and the repo's constants are
`RANDOM_MAP_MIN_PLAYERS = 2` / `RANDOM_MAP_MAX_PLAYERS = 8` (`:17-18`). Assert what the code
does today:
```rust
    #[test]
    fn upserting_the_random_map_sentinel_keeps_exactly_one_row() {
        let mut records = Vec::new();
        let first = upsert_random_map_sentinel(&mut records, "Random Map");
        let second = upsert_random_map_sentinel(&mut records, "Random Map");
        assert_eq!(first, second, "the sentinel is updated, never duplicated");
        let sentinels = records
            .iter()
            .filter(|r| matches!(r.kind, SkirmishScenarioKind::RandomMapSentinel))
            .count();
        assert_eq!(sentinels, 1);
        let sentinel = &records[first];
        assert_eq!(sentinel.file_name, RANDMAP_SED);
        assert!(sentinel.official);
        assert_eq!(
            (sentinel.min_players, sentinel.max_players),
            (Some(2), Some(8)),
            "current repo behaviour; see the open question below"
        );
    }
```

> **RESOLVED 2026-07-21 — the repo's `2..8` is correct. Change nothing in
> `skirmish_scenarios.rs`.**
> Native does construct the record with min `2` / max `4`
> (`ChooseMap__AcceptRandomMapSetup` → `FUN_0069A980(…, 1, 0, 2, 4)`), but **nothing reads
> those two fields when deciding a player count.**
> `MPGameOptions__GetScenarioPlayerCount` (`0x005E653F`) counts `[Waypoints]` 0..7 in the
> selected file and, finding none — which is always the case for a `.SED` — reads
> **`[RandomMap] NumPlayers`**, defaulting to `8` when absent or `0`.
> `MPGameOptions__SelectScenario` (`0x005E7C2B`) reads only `+0x58`, `+0x15C`, `+0x17C`.
> So the player count for a random map is the value this dialog's trackbar writes (2..8),
> which is exactly what the port models. Commit `04029220` was right; the assertion above
> is correct as written.
> *(Verified via `decompile_function` on `0x005E8590`, `0x0069A980`, `0x005E653F`,
> `0x005E7C2B`, `0x005ED5A0`, `0x005ED370`.)*

**Step 4: Verify.** `cargo test -p vera20k sentinel -- --nocapture` → PASS.

**Step 5:** `rustfmt --edition 2024 src/app.rs src/ui/skirmish_shell/state/tests.rs`; commit
`ui/skirmish: accepted random-map setup writes RandMap.Sed and commits it`

---

### Task 13: Cancel

**Why:** Cancel must have zero side effects.

**Files:** Modify `src/app.rs`; modify `src/ui/skirmish_shell/state/tests.rs`

**Step 1: The arm**
```rust
    RandomMapSetupControl::Cancel0x5c0 => {
        // Result 2 in the original: no seed file, no sentinel, no selection
        // change. The choose-map modal underneath is left untouched, so the
        // previous selection survives by construction.
        close_setup_modal = true;
    }
```
and after the borrow ends: `if close_setup_modal { state.skirmish_shell_state.random_map_setup_modal = None; }`

**Step 2: Test**
```rust
    #[test]
    fn cancelling_setup_returns_the_previous_selection_untouched() {
        let previous = ChooseMapSelection { mode_id: 1, record_index: Some(3) };
        let state = RandomMapSetupModalState::open(
            RmgOptions::default(),
            Some(previous),
            false,
            &mut MaxRng,
        );
        assert_eq!(state.cancel(), Some(previous));
    }
```

**Step 3: Verify.** `cargo test -p vera20k random_map_setup -- --nocapture` → PASS.

**Step 4:** `rustfmt --edition 2024 src/app.rs`; commit
`ui/skirmish: random-map setup cancel leaves the selection untouched`

---

### Task 14: Full regression pass

**Step 1:** `cargo test -p vera20k` — read the literal `test result:` line; all pass.
**Step 2:** `cargo clippy -p vera20k` — no new warnings from touched files.
**Step 3:** `git diff --stat` — confirm only File Map files changed and that no formatting
churn landed in untouched regions or in any module root.
**Step 4:** Commit fixes as `ui/skirmish: regression fixes for the random-map dialog`.

---

### Task 15: Verify against gamemd.exe

**In the port:**
1. Skirmish → Choose Map → **Create Random Map**.
2. Dialog opens on the choose-map frame; option controls where the listboxes were; preview
   box empty.
3. **OK is greyed.** The seed field shows a non-zero value.
4. Change any combo → OK stays greyed. **Generate** → OK becomes available.
5. **Randomize** → theater/map-type/time/resources/size change; theater is only ever the
   first two entries; OK greys out again.
6. **OK** → modal closes; the map list shows a single random-map row.
7. **Start** → a generated map loads and is playable.
8. `{ra2_dir}/RandMap.Sed` exists; its first key is `Description`.
9. Re-enter, accept again → still exactly one row.
10. Re-enter, **Cancel** → the previously selected map is still selected.
11. With no saved seeds, press Generate → **Load and Delete become enabled** (native quirk).

**Against gamemd.exe — side-by-side:** control positions and sizes (especially the right
column, which must sit ~5 px left of the choose-map column, and the preview ~3 px right);
players is a slider; button labels; which controls start greyed; the greying behaviour as
options change; and **whether Cancel sits where the port puts it** (the Task 9 decision).

**Expected divergence (accepted):** the preview box is empty; the original shows a small
terrain image with start markers.

**Report** the result honestly, including any geometry drift, and file the preview follow-up.

---

## Sources & References

- **Design:** `docs/plans/2026-07-21-random-map-setup-dialog-design.md`
- **Review:** `docs/plans/2026-07-21-random-map-setup-dialog-plan-REVIEW.md`
- **Ghidra reports:** `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_DIALOG_0X105_LAYOUT_GEOMETRY_GHIDRA_REPORT.md`,
  `…/SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS_GHIDRA_REPORT.md`,
  `…/SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`,
  `…/SKIRMISH_RANDMAP_SED_WRITER_FULL_LAYOUT_GHIDRA_REPORT.md`
- **gamemd.exe addresses** (kept here, never in Rust comments): dialog `0x105`; WndProc
  `0x00596300`; modal pump `0x00595BC0`; command entry `0x005E8590`; record constructor
  `0x0069A980`; seed constructor `0x00595680`; normalizer `0x005975E0`; control sync
  `0x00596C70`; display sync `0x00596E50`; derived-field helper `0x00597260`;
  `RandomRanged` `0x0065C7E0`; `.SED` writer `0x00597730` with description codec
  `0x00528E00` / `0x00528F00`; delimiter `0x00817F70`; range tables `0x0082B080` (region
  min), `0x0082B094` (region max), `0x0082B0A8` (water min), `0x0082B0BC` (water max),
  `0x0082B0D0` (accessibility max), `0x0082B0F8` (urban max), `0x0082B10C` (ruggedness min),
  `0x0082B120` (ruggedness max), `0x00ABED18` (accessibility min, zero),
  `0x00ABED40` (urban min, zero)
- **Verification calls made while planning:** `decompile_function` on `0x00597260`,
  `0x0065C7E0`, `0x00528E00`, `0x005E8590`, `0x0069A980`, `0x00596300`;
  `read_memory 0x0082B080` (180 bytes), `0x00ABED18` (72 bytes), `0x00817F70` (8 bytes);
  `get_xrefs_to 0x00ABED18` / `0x00ABED40`; `get_function_callees 0x00597260`;
  `search_functions RandomRanged`; PE `RT_DIALOG` extraction of `0x6B` and `0x105`
- **INI:** `RMGMD.INI [General]` `RMGVegetationMinimums`, `RMGVegetationMaximums`
  (`ini/rmgmd.ini:15-16`) — already parsed by `src/map/rmg/settings.rs:29-30`
- **Related code:** `src/ui/skirmish_shell/layout.rs:36,41,159,172,326,559,686`,
  `src/ui/skirmish_shell/state.rs:3,27,53`,
  `src/ui/skirmish_shell/state/choose_map.rs:15,21,138,145,149`,
  `src/ui/skirmish_shell/state/player_name.rs:221,256,302`,
  `src/ui/skirmish_shell/mod.rs:12,32`, `src/app_skirmish_shell_render/modals.rs:92,102`,
  `src/app_skirmish_shell_render/chrome.rs:173,321`,
  `src/app_skirmish_shell_render/controls.rs:242,277`,
  `src/app_skirmish_shell_render.rs:203,726`,
  `src/app.rs:120,523,589,1199,1208,1284,1314,1330,1337,1559,1610`,
  `src/skirmish_scenarios.rs:14,17,23,41,115,221`, `src/app_init.rs:343,353,358`,
  `src/app_options_persist.rs:58`, `src/map/rmg/options.rs:14,17,57,97,125,175,262`,
  `src/map/rmg/settings.rs:29`, `src/util/config.rs:21`
- **Prior commits:** `f6213c7f`, `cacc073f`, `2faf3601`, `848c6d38`, `04029220` (sentinel
  widening — see Task 12's open question)
