# Skirmish Lobby Color Swatches — Priority-Order [Colors] Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make the Skirmish lobby color picker draw its 8 swatches from gamemd's real
`[Colors]` HSV schemes in priority order (via `rules.color_schemes` →
`color_scheme::scheme_for_priority` → `hsv_to_rgb`), so a lobby swatch matches the
loading-screen progress-bar backing for the same slot.

**Architecture:** Pure presentation change in the skirmish-shell renderer
(`render`/`app` layer, above `sim/`). The data path (`color_scheme.rs`) already exists and
is unit-tested — it currently powers only the loading screen. This plan re-points the
lobby swatch lookup onto that same path and threads the parsed `[Colors]` slice from
`state.rules` down into the swatch draw. `sim/` is untouched.

**Design Doc:** None — design is fully specified in the user's task brief (no `/brainstorm`
needed for a 4-site UI render fix). This plan stands in for the design spec.

---

## Grounding Summary

- **What the repo already proves:** `src/rules/color_scheme.rs` already parses `[Colors]`,
  applies the SessionClass priority LUT `[3,11,21,29,13,25,17,15,5]` with scheme-doubling
  (`scheme_for_priority`), and runs gamemd's 6-sextant integer `hsv_to_rgb`. Its tests
  (`priority_table_selects_the_eight_multiplayer_colors`,
  `backing_rgb_resolves_player_priority_to_scheme_color`) already prove priority 0..7 →
  Gold, DarkRed, DarkBlue, DarkGreen, Orange, DarkSky, Purple, Magenta.
- **What's wrong today:** `house_color_tint` (`src/app_skirmish_shell_render/controls.rs:232`)
  looks the slot up in `house_colors::SCHEME_BASES` order (gold, **darkblue**, **darkred**, …),
  so slot 1 renders DarkBlue and slot 2 DarkRed — the reverse of priority order.
- **Confirming the slot↔priority mapping:** the lobby's own tooltips
  (`src/ui/skirmish_shell/state/hit_test.rs:263-271`) already declare `Color(0)`=Gold,
  `Color(1)`=Red, `Color(2)`=Blue, `Color(3)`=Green, `Color(4)`=Orange, `Color(5)`=SkyBlue,
  `Color(6)`=Purple, `Color(7)`=Pink — i.e. **slot index == color priority**. Only the swatch
  draw disagrees; everything else (tooltips, launch handoff) is already priority-ordered.
- **No Ghidra needed:** this is a Rust-internal render correction onto an already-verified
  data path; the binary behavior is captured by `color_scheme.rs` and its tests. No new
  binary claims are introduced.
- **INI:** `[Colors]` in `ini/rulesmd.ini` (already parsed into `RuleSet.color_schemes` at
  `src/rules/ruleset.rs:1314,1405,1591`). No new INI keys.
- **Repo pattern mirrored:** the loading screen's
  `NativeLoadingScreen::resolve_backing_color(&rules.color_schemes)`
  (`src/app_loading.rs:238,327`) — same slice, same priority resolution. The lobby will read
  the same `state.rules.color_schemes`.
- **Still unknown after grounding:** none material. The only defensive choice is the
  empty-`[Colors]` fallback (legacy ramp), which cannot occur in a normal lobby because
  `state.rules` is always populated by then.

## Key Technical Decisions

- **`color_index` (0..=7) is passed straight through as the color priority** to
  `scheme_for_priority(schemes, index as i32)`. — **Confidence:** high
  - **Source:** repo pattern `src/ui/skirmish_shell/state/hit_test.rs:263-271` (tooltips) +
    `src/rules/color_scheme.rs` tests; `HOUSE_COLOR_COUNT = 8` (`src/skirmish_launch.rs:13`).
- **Thread `color_schemes: &[ColorSchemeEntry]` down the render call chain** rather than
  resolving colors earlier or stashing them on the shell state. — **Confidence:** high
  - **Source:** mirrors how `maps: &[MapMenuEntry]` is already threaded into
    `build_skirmish_shell_instances`; keeps the renderer reading `state.rules` directly like
    the loading screen does.
- **Empty-`[Colors]` fallback keeps the legacy synthesized ramp** so a swatch never renders
  black if rules somehow aren't loaded. — **Confidence:** high (defensive-only path)
  - **Source:** inferred; `house_colors::house_color_ramp` already exists for this.

## Open Questions

### Resolved During Planning

- *Is slot index the same as color priority?* — Yes. Confirmed by the existing tooltip table
  and launch handoff, both priority-ordered; only the swatch draw was off.
- *Are there other callers of the functions whose signatures change?* — No. `house_color_tint`
  is used only at `controls.rs` (combo face + dropdown); `build_skirmish_shell_instances` has a
  single call site (`app_skirmish_shell_render.rs:500`); the test module in that file does not
  call it.

### Deferred to Implementation

- None. (In-game side-by-side confirmation of the swatch colors is the final verification step,
  Task 6, but it does not block the code.)

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_skirmish_shell_render/controls.rs` | Re-point `house_color_tint` onto the `[Colors]` priority path; thread `color_schemes` through `push_combo_face` / `push_combo_instances` / `push_dropdown_instances`; add swatch tests |
| Modify | `src/app_skirmish_shell_render.rs` | Add `color_schemes` param to `build_skirmish_shell_instances`; forward it; supply it from `state.rules` at the call site |

## Interface Changes

Internal (`pub(super)` / crate-local) signature changes only — no public API, no schema, no
sim contract:

- `house_color_tint(index)` → `house_color_tint(color_schemes: &[ColorSchemeEntry], index)`
- `push_combo_face(...)` gains a leading-data `color_schemes: &[ColorSchemeEntry]` param
- `push_combo_instances(...)` and `push_dropdown_instances(...)` each gain `color_schemes`
- `build_skirmish_shell_instances(...)` gains `color_schemes: &[ColorSchemeEntry]` (added
  right after `shell`)

Only dependent: the single call site at `app_skirmish_shell_render.rs:500`, updated in Task 5.

## Sim Checklist

Not applicable — no `sim/` files touched, no game-logic math, no state hash, no tick ordering.
All values are render-side `f32` tints (rendering math, allowed outside `sim/`).

## Risk Areas

- **Blast radius is tiny:** two files, one call site. Worst case is a wrong swatch color, caught
  immediately on screen and by Task 4 tests.
- **Regression guard:** Task 4 asserts `house_color_tint(schemes, slot)` equals the loading
  screen's `backing_rgb_for_priority(schemes, slot)` for every slot 0..=7 — this pins the lobby
  swatch and the loading backing together so they can't drift apart again.
- **Scope fence:** `house_colors::SCHEME_BASES` and `house_color_ramp` stay in place and keep
  driving unit/building/radar colors — do NOT touch them. Confirm with the user before any
  unit-color change (explicitly out of scope per the task brief).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Swatch RGB = `hsv_to_rgb(scheme_for_priority(schemes, slot).hsv)` | The player sees the lobby swatch and (after launch) the loading backing for their slot; they must be the same color. Visible every skirmish setup. | Task 4 unit test (swatch == loading backing for all 8 slots) + Task 6 in-game side-by-side vs gamemd lobby |
| Task 1 | Slot→priority order (slot 1 = Red, slot 2 = Blue, …) | Matches the lobby tooltips and gamemd's MP color order; current code shows the wrong color per slot every match. | Task 4 unit test (slot 1 red-dominant, slot 2 blue-dominant) + Task 6 |

---

## Tasks

### Task 1: Re-point `house_color_tint` onto the `[Colors]` priority path

**Why:** This is the actual fix — map a lobby slot to its gamemd `[Colors]` color instead of the
synthesized `SCHEME_BASES` ramp. Done first because everything else just feeds it the slice.

**Files:**
- Modify: `src/app_skirmish_shell_render/controls.rs:10` (imports)
- Modify: `src/app_skirmish_shell_render/controls.rs:232-240` (`house_color_tint`)

**Pattern:** Mirrors `src/app_loading.rs:238` (`resolve_backing_color`) — same `color_schemes`
slice, same `scheme_for_priority` + `hsv_to_rgb` resolution.

**Step 1: Update imports**

Replace the existing house-colors import line:

```rust
use crate::rules::house_colors::{HouseColorIndex, house_color_ramp};
```

with both imports (color_scheme path + the retained legacy ramp for the fallback):

```rust
use crate::rules::color_scheme::{ColorSchemeEntry, hsv_to_rgb, scheme_for_priority};
use crate::rules::house_colors::{HouseColorIndex, house_color_ramp};
```

**Step 2: Rewrite `house_color_tint`**

Replace the current function body (lines 232-240):

```rust
pub(super) fn house_color_tint(index: usize) -> [f32; 3] {
    let ramp = house_color_ramp(HouseColorIndex(index.min(7) as u8));
    let color = ramp[0];
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ]
}
```

with:

```rust
/// Resolve a lobby color slot (0..=7) to its swatch RGB.
///
/// The 8 lobby color slots present the `[Colors]` schemes in priority order: the
/// slot index IS the color priority. `scheme_for_priority` applies the priority
/// LUT + scheme-doubling, then `hsv_to_rgb` runs the same 6-sextant integer
/// conversion the loading-screen backing uses — so a lobby swatch and the loading
/// backing match for a given slot.
///
/// Falls back to the legacy synthesized ramp only when the `[Colors]` list is empty
/// (rules not yet loaded), so the swatch still renders rather than going black; in a
/// normal skirmish lobby the scheme list is always populated.
pub(super) fn house_color_tint(color_schemes: &[ColorSchemeEntry], index: usize) -> [f32; 3] {
    if let Some(scheme) = scheme_for_priority(color_schemes, index as i32) {
        let [r, g, b] = hsv_to_rgb(scheme.hsv);
        return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    }
    let ramp = house_color_ramp(HouseColorIndex(index.min(7) as u8));
    let color = ramp[0];
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ]
}
```

**Step 3: Verify (compile only — callers fixed in later tasks)**

This task alone will leave the two `house_color_tint` call sites failing to compile (wrong arg
count). That is expected and fixed in Tasks 2 and 3. Do not `cargo check` until Task 3 is done.

**Step 4: Commit** — `git add -p` the two hunks; commit
`feat(skirmish): draw lobby color swatches from [Colors] priority schemes` AFTER Task 5 compiles.
(Hold the commit; the change isn't compilable in isolation.)

---

### Task 2: Thread `color_schemes` into `push_combo_face`

**Why:** The closed combo face draws the selected slot's swatch via `house_color_tint`, so it must
receive the scheme slice. Ordered before its caller so the new param exists when wired up.

**Files:**
- Modify: `src/app_skirmish_shell_render/controls.rs:242-273` (`push_combo_face`)

**Pattern:** Mirrors how `atlas` is already passed by reference into this function.

**Step 1: Add the parameter**

Change the signature (line 242-250). Current:

```rust
pub(super) fn push_combo_face(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    color_index: Option<usize>,
    open: bool,
    disabled: bool,
    depth: f32,
) {
```

New (add `color_schemes` right after `atlas`):

```rust
pub(super) fn push_combo_face(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    rect: RectPx,
    color_index: Option<usize>,
    open: bool,
    disabled: bool,
    depth: f32,
) {
```

**Step 2: Pass the slice into `house_color_tint`**

In the swatch block (around lines 254-262), change:

```rust
            house_color_tint(color_index),
```

to:

```rust
            house_color_tint(color_schemes, color_index),
```

**Step 3: Verify** — defer `cargo check` to Task 5 (callers still mismatch until then).

**Step 4: Commit** — fold into the single commit at Task 5.

---

### Task 3: Thread `color_schemes` into `push_combo_instances` and `push_dropdown_instances`

**Why:** These two are the callers that own a swatch draw — `push_combo_instances` calls
`push_combo_face` (8 call sites), `push_dropdown_instances` calls `house_color_tint` directly for
each open-dropdown row. Both must accept and forward the slice.

**Files:**
- Modify: `src/app_skirmish_shell_render/controls.rs:358-454` (`push_combo_instances`)
- Modify: `src/app_skirmish_shell_render/controls.rs:456-521` (`push_dropdown_instances`)

**Pattern:** Same param-threading as `maps: &[MapMenuEntry]` already in `push_dropdown_instances`.

**Step 1: `push_combo_instances` signature**

Current (lines 358-363):

```rust
pub(super) fn push_combo_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
```

New:

```rust
pub(super) fn push_combo_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) {
```

**Step 2: Pass `color_schemes` into every `push_combo_face` call**

There are 9 `push_combo_face(...)` calls in this function (lines ~365, 374, 383, 392, 407, 417,
426, 435, 444). Each currently starts:

```rust
    push_combo_face(
        out,
        atlas,
        <rect>,
        ...
```

Insert `color_schemes,` after `atlas,` in **every** one:

```rust
    push_combo_face(
        out,
        atlas,
        color_schemes,
        <rect>,
        ...
```

(Mechanical: the only `push_combo_face` callers are in this function — confirm with
`Grep "push_combo_face("` that all matches inside `push_combo_instances` are updated.)

**Step 3: `push_dropdown_instances` signature**

Current (lines 456-462):

```rust
pub(super) fn push_dropdown_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) {
```

New:

```rust
pub(super) fn push_dropdown_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    color_schemes: &[ColorSchemeEntry],
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
) {
```

**Step 4: Pass the slice into the dropdown-row `house_color_tint` call**

In the dropdown row loop (around line 499-505), change:

```rust
                house_color_tint(color_index),
```

to:

```rust
                house_color_tint(color_schemes, color_index),
```

**Step 5: Verify** — defer `cargo check` to Task 5.

**Step 6: Commit** — fold into the single commit at Task 5.

---

### Task 4: Add swatch parity tests in `controls.rs`

**Why:** Pin the lobby swatch to the loading-screen backing and to priority order so the fix
can't silently regress. Pure-logic test — no rendering needed.

**Files:**
- Modify: `src/app_skirmish_shell_render/controls.rs` (append a `#[cfg(test)] mod tests` at end
  of file — none exists today)

**Pattern:** Mirrors the scheme-list fixture in `src/rules/color_scheme.rs` tests.

**Step 1: Append the test module**

Add at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::color_scheme::backing_rgb_for_priority;

    /// The reachable entries of the retail rulesmd `[Colors]` list, in order — same
    /// fixture rationale as the color_scheme.rs tests. Slot/priority indices land on
    /// the scattered scheme entries via the priority LUT + doubling.
    fn retail_schemes() -> Vec<ColorSchemeEntry> {
        let raw: &[(&str, [u8; 3])] = &[
            ("LightGold", [25, 255, 255]),
            ("Gold", [43, 239, 255]),
            ("LightGrey", [0, 0, 240]),
            ("Grey", [0, 0, 131]),
            ("Red", [20, 255, 184]),
            ("DarkRed", [0, 230, 255]),
            ("Orange", [25, 230, 255]),
            ("Magenta", [221, 102, 255]),
            ("Purple", [201, 201, 189]),
            ("LightBlue", [119, 143, 255]),
            ("DarkBlue", [153, 214, 212]),
            ("NeonBlue", [185, 156, 238]),
            ("DarkSky", [131, 200, 230]),
            ("Green", [104, 241, 195]),
            ("DarkGreen", [81, 200, 210]),
        ];
        raw.iter()
            .map(|(name, hsv)| ColorSchemeEntry {
                name: name.to_string(),
                hsv: *hsv,
            })
            .collect()
    }

    #[test]
    fn swatch_matches_loading_backing_for_every_slot() {
        // The lobby swatch and the loading-screen progress-bar backing must agree
        // color-for-color for each of the 8 slots — this is the parity that broke.
        let schemes = retail_schemes();
        for slot in 0..8usize {
            let rgb = backing_rgb_for_priority(&schemes, slot as i32).unwrap();
            let expected = [
                rgb[0] as f32 / 255.0,
                rgb[1] as f32 / 255.0,
                rgb[2] as f32 / 255.0,
            ];
            assert_eq!(house_color_tint(&schemes, slot), expected, "slot {slot}");
        }
    }

    #[test]
    fn slot_one_is_red_slot_two_is_blue() {
        // Priority order: slot 1 = DarkRed (red-dominant), slot 2 = DarkBlue
        // (blue-dominant) — the reverse of the old SCHEME_BASES ordering.
        let schemes = retail_schemes();
        let red = house_color_tint(&schemes, 1);
        assert!(red[0] > red[1] && red[0] > red[2], "slot 1 red-dominant: {red:?}");
        let blue = house_color_tint(&schemes, 2);
        assert!(blue[2] > blue[0] && blue[2] > blue[1], "slot 2 blue-dominant: {blue:?}");
    }

    #[test]
    fn empty_schemes_fall_back_to_legacy_ramp() {
        // Defensive path: with no [Colors] loaded the swatch still renders a color.
        let tint = house_color_tint(&[], 0);
        assert!(tint.iter().any(|&channel| channel > 0.0));
    }
}
```

**Step 2: Verify** — defer running until Task 5 compiles the crate.

**Step 3: Commit** — fold into the single commit at Task 5.

---

### Task 5: Add `color_schemes` to `build_skirmish_shell_instances` and supply it at the call site

**Why:** Wires the parsed `[Colors]` slice from `state.rules` into the render entry point and down
to the two swatch-drawing functions. This is the task that makes the crate compile again.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs:182-192` (signature)
- Modify: `src/app_skirmish_shell_render.rs:333` (`push_combo_instances` call)
- Modify: `src/app_skirmish_shell_render.rs:362` (`push_dropdown_instances` call)
- Modify: `src/app_skirmish_shell_render.rs:500-510` (call site)

**Pattern:** Mirrors the existing `maps: &[MapMenuEntry]` param and how `app_loading.rs:327` reads
`&rules.color_schemes`.

**Step 1: Confirm the import is available**

`ColorSchemeEntry` must be nameable in `app_skirmish_shell_render.rs`. Check existing imports; if
`crate::rules::color_scheme::ColorSchemeEntry` is not already imported, add:

```rust
use crate::rules::color_scheme::ColorSchemeEntry;
```

near the other `use crate::rules::...` lines at the top of the file.

**Step 2: Add the parameter to `build_skirmish_shell_instances`**

Current (lines 182-192):

```rust
pub fn build_skirmish_shell_instances(
    atlas: &SkirmishShellChromeAtlas,
    font: &BitFont,
    layout: &SkirmishShellLayout,
    choose_map_layout: Option<&ChooseMapModalLayout>,
    validation_layout: Option<&ValidationModalLayout>,
    shell: &SkirmishShellState,
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
    wave: Option<&ShellFrameWave>,
) -> Vec<SpriteInstance> {
```

New (add `color_schemes` after `shell`):

```rust
pub fn build_skirmish_shell_instances(
    atlas: &SkirmishShellChromeAtlas,
    font: &BitFont,
    layout: &SkirmishShellLayout,
    choose_map_layout: Option<&ChooseMapModalLayout>,
    validation_layout: Option<&ValidationModalLayout>,
    shell: &SkirmishShellState,
    color_schemes: &[ColorSchemeEntry],
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
    wave: Option<&ShellFrameWave>,
) -> Vec<SpriteInstance> {
```

**Step 3: Forward to the two swatch functions**

Line 333, change:

```rust
    push_combo_instances(&mut instances, atlas, layout, shell);
```

to:

```rust
    push_combo_instances(&mut instances, atlas, color_schemes, layout, shell);
```

Line 362, change:

```rust
    push_dropdown_instances(&mut instances, atlas, layout, shell, maps);
```

to:

```rust
    push_dropdown_instances(&mut instances, atlas, color_schemes, layout, shell, maps);
```

**Step 4: Supply the slice at the call site (line 500)**

Current:

```rust
    let instances = build_skirmish_shell_instances(
        atlas,
        &state.bit_font,
        &layout,
        choose_map_layout.as_ref(),
        validation_layout.as_ref(),
        &state.skirmish_shell_state,
        &state.skirmish_shell_maps,
        &state.skirmish_modes,
        wave,
    );
```

New (insert the schemes arg after the shell-state arg; empty slice when rules aren't loaded —
the defensive fallback in `house_color_tint` then applies):

```rust
    let color_schemes = state
        .rules
        .as_ref()
        .map(|rules| rules.color_schemes.as_slice())
        .unwrap_or(&[]);
    let instances = build_skirmish_shell_instances(
        atlas,
        &state.bit_font,
        &layout,
        choose_map_layout.as_ref(),
        validation_layout.as_ref(),
        &state.skirmish_shell_state,
        color_schemes,
        &state.skirmish_shell_maps,
        &state.skirmish_modes,
        wave,
    );
```

> Note: `state.rules` is borrowed immutably here; confirm no conflicting `&mut state` borrow is
> live across this statement. If the borrow checker complains, bind `color_schemes` to an owned
> `Vec<ColorSchemeEntry>` clone before the `build_*` call as a fallback (cheap — runs once per
> shell repaint, not per tick), but prefer the slice form.

**Step 5: Verify compile**

Run: `cargo check -p vera20k`
Expected: clean (no errors). If "package ID did not match" / exit 101, re-confirm the `-p` name is
`vera20k`.

**Step 6: Run the tests added in Task 4**

Run: `cargo test -p vera20k house_color_tint swatch_matches slot_one empty_schemes`
(or `cargo test -p vera20k -- controls` to scope to the module).
Read the literal `test result:` line — expected `ok`, 3 passed. Also run the existing color
tests to confirm no regression: `cargo test -p vera20k -- color_scheme`.

**Step 7: Commit**

Now that the full change set compiles and tests pass, stage all hunks from Tasks 1-5 and commit:

```
feat(skirmish): draw lobby color swatches from [Colors] priority schemes

Lobby color swatches now resolve through rules.color_schemes +
color_scheme::scheme_for_priority + hsv_to_rgb, matching the loading-screen
backing for the same slot. Replaces the SCHEME_BASES-ordered house_color_ramp
lookup in the lobby; unit/radar SCHEME_BASES colors are intentionally left
unchanged.
```

(Commit to `dev` per the project git workflow — no branch, no push.)

---

### Task 6: Verify against gamemd lobby in-game

**Why:** Confirm the player-visible result matches the original — swatches in correct priority
order, and the swatch == the loading backing for the chosen slot.

**Verify:**
- Launch the engine (`/run` or the project's run path), open Skirmish, open a color combo.
- Expected order top-to-bottom of selectable colors: Gold/Yellow, Red, Blue, Green, Orange,
  Periwinkle/SkyBlue, Purple, Pink — matching gamemd's lobby and the tooltips.
- Pick slot 1 (Red) for the player; start the game; confirm the loading-screen progress-bar
  backing is the **same** red, not a different hue. Repeat for slot 2 (Blue).
- Side-by-side against retail gamemd.exe lobby if any swatch hue looks off.
- This task is observation only — no code. If a hue mismatches, the regression is in the data
  path (color_scheme.rs), not this change; re-open before declaring done.

---

## Sources & References

- **Design doc:** none — user task brief (this plan stands in).
- **Repo — data path (already implemented + tested):** `src/rules/color_scheme.rs`
  (`parse_color_schemes`, `scheme_for_priority`, `hsv_to_rgb`, `backing_rgb_for_priority`,
  priority LUT `[3,11,21,29,13,25,17,15,5]`).
- **Repo — slot↔priority proof:** `src/ui/skirmish_shell/state/hit_test.rs:263-271` (tooltips),
  `src/ui/skirmish_shell/state/combos.rs:282-293` (color combo items), `src/skirmish_launch.rs:13`
  (`HOUSE_COLOR_COUNT = 8`).
- **Repo — the code being changed:** `src/app_skirmish_shell_render/controls.rs:232-240` (current
  `house_color_tint`), `:242-273` (`push_combo_face`), `:358-454` (`push_combo_instances`),
  `:456-521` (`push_dropdown_instances`); `src/app_skirmish_shell_render.rs:182-192,333,362,500`.
- **Repo — mirrored pattern (loading screen):** `src/app_loading.rs:238,327`
  (`resolve_backing_color(&rules.color_schemes)`).
- **Rules struct:** `src/rules/ruleset.rs:1314,1405,1591` (`color_schemes: Vec<ColorSchemeEntry>`).
- **INI:** `ini/rulesmd.ini` `[Colors]` (`Name=H,S,V`) — already parsed.
- **Out of scope (do NOT touch):** `src/rules/house_colors.rs` `SCHEME_BASES` / `house_color_ramp`
  (unit/building/radar colors) — separate decision, confirm with user first.
