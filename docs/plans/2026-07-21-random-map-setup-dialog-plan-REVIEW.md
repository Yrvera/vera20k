# Plan Review: `2026-07-21-random-map-setup-dialog-plan.md`

**Verdict: FIX FIRST** — 18 confirmed issues. The approach is sound and the extracted
*data* is correct; the defects are a wrong geometry-inheritance assumption, wrong Rust
signatures, and task-ordering breaks. No rework of the design is needed.

---

## 1. Headline finding (new — not on the author's list)

### `0x6B` and `0x105` do NOT share right-column coordinates. Task 8's inheritance strategy is wrong.

Fresh extraction of `RT_DIALOG 0x6B` (PE template at file `0x4F26D8`, 11 controls,
533×369 DLU) versus the `0x105` table:

| Control | `0x6B` x | `0x105` x | Δ DLU | Δ px @800 |
|---|---:|---:|---:|---:|
| Title `0x694` | 425 | 422 | −3 | −5 |
| UseMap `0x6C5` | 425 | 422 | −3 | −5 |
| Cancel `0x5C0` | 425 | 423 | −2 | −3 |
| Preview `0x468` | 428 | 430 | +2 | +3 |
| Blank `0x695` | 2 | 2 | 0 | 0 |

Only `0x695` genuinely matches. The port's own constants
(`use_map_base = dlu_rect(425,122,108,23)`, `preview_base = dlu_rect(428,23,96,69)`,
`title_base = dlu_rect(425,1,108,10)` at `layout.rs:562-594`) match `0x6B` **exactly** —
so the existing choose-map dialog is correct and must not be touched. The bug is purely in
the plan's instruction to reuse those expressions for `0x105`.

> ### ⚠ CORRECTION (2026-07-21, during execution of Task 9)
>
> **The pixel-drift claim below was WRONG.** I predicted a 3–5 px misplacement. It cannot
> happen, because every right-column helper the port uses **discards the source rect's x**:
> - `right_anchor` (`layout.rs:399`) computes `screen_w - offset_x - original.w - inset` —
>   it reads only `y`, `w`, `h`.
> - `snap_button_biased_truncate` (`geom.rs:177`) computes `screen_w - offset_x - cell_w`
>   and uses `source.y` only, for the tile index.
> - `back_rect` (`layout.rs:423`) ignores the source rect entirely.
>
> Since the shared controls' `y`/`w`/`h` are **identical** between `0x6B` and `0x105`, the
> 2–3 DLU x difference produces **byte-identical output**. The resource difference is real
> and worth recording; the rendering consequence is nil.
>
> The implementation still passes `0x105`'s own values (correct data, same result), and
> `setup_shares_the_choose_map_frame_and_right_column` now pins the equality empirically —
> the opposite assertion to the one this review originally prescribed.
>
> **Lesson:** I compared two resources and inferred a pixel consequence without reading the
> transform in between. Verify the whole path, not just the inputs.

**Original (superseded) scenario:** execute Task 8 as written at 800×600 and the setup
dialog's Use Map button lands at x=638px instead of x=633px, with the preview box 3px off
in the opposite direction.

**Fix as landed:** `0x105` carries its own right-column DLU constants (422/423/430) for
data fidelity, and the frame/background/`0x695` are shared. Output is identical either way.

**Upstream doc correction (my own error):** `SKIRMISH_RANDOM_MAP_DIALOG_0X105_LAYOUT_GEOMETRY_GHIDRA_REPORT.md`
§3 final bullet claims these controls are "all at the same coordinates as `0x6B`". That is
**WRONG** — only `0x695` is. The §3 *table* is correct; only the summary bullet is wrong.
Correct it so the next reader is not misled.

---

## 2. Binary fidelity — all author-flagged items resolved

Every uncertain item is now settled **from the binary**, not from docs.

| Claim | Verdict | Evidence |
|---|---|---|
| `RandomRanged` inclusive on both ends | **CONFIRMED** | `decompile_function 0x0065c7e0`: `span = max-min`; rejection loop accepts `value <= span`; returns `min + value`. Also swaps when `max<min`, and short-circuits to `min` when `min==max`. **Tasks 2–3 stand as written.** |
| Sentinel min=2 / max=4 | **CONFIRMED** | `decompile_function 0x0069a980`: `param_7 → +0x180`, `param_8 → +0x184`, `param_5 → +0x17C`. `FUN_005e8590` calls `(…, 1, 0, 2, 4)`. Verified param→field mapping, not doc-sourced. |
| Description trailing comma | **CONFIRMED** | `read_memory 0x00817f70` = `0x2c` (`,`); `FUN_00528E00` appends the delimiter *inside* the per-code-unit loop, so the last unit gets one. Lowercase, unpadded, radix `0x10`. Plan's `"52,61,…,70,"` is right. |
| Randomize draws seed **twice** | **CONFIRMED** | `0x00596300` case `0x621`: `FUN_00597260` ends with a seed draw, then `DAT_00abe04c = RandomRanged(0,0xffff)` again. |
| Randomize draw **order** | **CONFIRMED** | `0x621`: theater(`+0x38`), maptype(`+0x3C`), **time(`+0x48`)**, **resources(`+0x40`)**, size(`+0x64`=`+0x68`), derive, description, seed. Time before resources — as planned. |
| `0x00597260` order + table mapping | **CONFIRMED** | Decompile operand addresses match the plan's assignment; `get_function_callees 0x00597260` returns only `Random__RandomRanged`. |
| Accessibility/urban min = 0 | **CONFIRMED** | `get_xrefs_to 0x00abed18` / `0x00abed40` → only the read inside `FUN_00597260`. No writer. |
| Init: OK + Save disabled, Load/Delete from availability, seed randomized if −1 | **CONFIRMED** | `0x00596300` case `0x497`. |
| Generate disables all 13 controls incl. Cancel | **CONFIRMED** | case `0x620` disables `0x405,0x3ea,0x407,0x406,0x408,0x3eb,0x621,0x620,0x6c2,0x6c3,0x6c4,0x6c5,0x5c0`. |
| Players is a trackbar | **CONFIRMED** | handled under `WM_HSCROLL` (`0x114`) with `GetDlgCtrlID == 0x3eb`. |
| OK always succeeds in skirmish | **CONFIRMED** | case `0x6c5`: generates first if no preview, then result 1. The `RandMap.Map` / `Save_Scenario_Map_File` path is **map-editor only** — correctly excluded from the plan. |
| Cancel → result 2 | **CONFIRMED** | case `0x5c0`: `*puVar5 = 2`. |

### New binary-derived issue

**Generate re-enables Load/Delete unconditionally.** Case `0x620` re-enables `0x6C2` and
`0x6C4` with a literal `1`, bypassing the `FUN_00559C20` saved-seed availability check that
gates them at init. The plan's `is_enabled` returns `saved_seeds_available` in *all* states.
**Scenario:** fresh dialog, no saved seeds → both greyed; press Generate → native shows them
enabled, the port keeps them greyed. *(Save `0x6C3` gating on `generated` is correct — it
matches init/Generate/Randomize/option-change in every state.)*

---

## 3. The contested sentinel — both sides, no side picked

- **Binary (verified):** the record gets min `2` at `+0x180`, max `4` at `+0x184`.
- **Repo (verified):** `RANDOM_MAP_MIN_PLAYERS = 2`, `RANDOM_MAP_MAX_PLAYERS = 8`
  (`skirmish_scenarios.rs:17-18`), **plus** `RANDOM_MAP_GENERATED_START_QUOTA = 4` (`:23`)
  used as `player_capacity`.
- **History:** the doc comment at `skirmish_scenarios.rs:19-23` explicitly records that
  `RANDOM_MAP_MAX_PLAYERS` used to alias the quota, capping the sentinel at 4 and making
  5–8-player random maps unselectable — which is why commit `04029220` widened it.

> **RESOLVED 2026-07-21 (post-review Ghidra pass). Keep `2..8`; change nothing.**
> Neither hypothesis was right. **Nothing reads `+0x180`/`+0x184` when deciding a player
> count.** `MPGameOptions__GetScenarioPlayerCount` (`0x005E653F`) counts `[Waypoints]` 0..7
> in the selected file and, finding none — always the case for a `.SED` — reads
> **`[RandomMap] NumPlayers`**, defaulting to `8`. `MPGameOptions__SelectScenario`
> (`0x005E7C2B`) reads only `+0x58`, `+0x15C`, `+0x17C`; `FUN_005ED5A0` / `FUN_005ED370`
> touch neither field. So a random map's player count is the dialog trackbar value (2..8),
> commit `04029220` was correct, and the `+0x184 == player_capacity` guess was also wrong —
> the `4` is simply never consulted.
> **New flag:** the port hardcodes `player_capacity = 4` while native derives capacity from
> `NumPlayers` — separate investigation.

**Meanwhile:** the plan's Task 12 test is wrong on **both type and value** — the fields are
`Option<u8>`, so the current-code assertion is `(Some(2), Some(8))`. And Task 12 Step 4's
"Expected: PASS (or a clear report of the discrepancy)" is self-contradictory; a test either
asserts current behavior or is not written yet.

---

## 4. Geometry — values correct, two omissions

Every DLU constant in Task 8 matches the `0x105` resource exactly, including the
2-DLU-shorter time combo (`101` vs `103`) and the full `SETUP_ROW_Y` / `SETUP_LABEL_Y`
ordering. Two gaps:

1. **Label heights are not uniform.** Rows are `[14, 14, 12, 12, 12, 14]` (map type, time,
   theater, size, resources, players). The plan declares `SETUP_LABEL_W` but **no
   `SETUP_LABEL_H`**, and its only test asserts height `14` — a single-height implementation
   makes three rows 2 DLU too tall. Needs an array.
2. **`0x695` bottom blank is uncovered** — absent from the constants *and* from the
   `RandomMapSetupLayout` field list. (`ChooseMapModalLayout` has no blank field either; its
   closest is `status_help`.)

Minor: the `0x405` label is `GUI:Environment`, not "map type" — and the plan's deferred-CSF
list omits ~12 keys that appear in §3.

---

## 5. Codebase accuracy — line numbers excellent, signatures wrong

Line citations are unusually good (nearly all exact, none off by >3). The failures are
concentrated exactly where the plan admitted it was guessing. **16 compile-blockers:**

| # | Issue | Correct form |
|---|---|---|
| 1 | `state.skirmish_shell` (3 sites, Tasks 11–13) | **`state.skirmish_shell_state`** |
| 2 | `compute_choose_map_modal_layout(viewport)` | **two `u32` args** `(screen_w, screen_h)` |
| 3 | `random_map_setup_control_at(layout, point)` | **`(layout, x: i32, y: i32)`** |
| 4 | Draw fn signature/order/visibility | **`pub(super) fn(out, atlas, layout, shell, modes)`**, no `viewport` |
| 5 | `choose.frame` | **`choose.dialog`** (no `frame` field exists) |
| 6 | `record_index: index` | **`Some(index)`** — it's `Option<usize>` |
| 7 | Sentinel assert type/value | **`(Some(2), Some(8))`** |
| 8 | `src/ui/skirmish_shell/mod.rs` missing from File Map | new public items are **unreachable** without adding to its `pub use` allow-lists |
| 9 | `pub mod random_map_setup;` | **`mod` + `pub use`** — `state.rs` uses private `mod` |
| 10 | Draw dispatch placed *after* the choose-map block | that block **early-returns**; must go *inside* it |
| 11 | "reuse combo/trackbar helpers" | those are hard-wired to the main shell; use **`paint_control` + `ControlPaint::{Combo,Trackbar}`** |
| 12 | `choose_map_background_entry` "unchanged" | it is typed to `&ChooseMapModalLayout` — impossible from a `&RandomMapSetupLayout` |
| 13 | Task 11 Step 2 borrow | `modal` is used after `state` is reborrowed → **E0499**; copy the selection out inside the match arm |
| 14 | Task 7 `.clone()` | `ChooseMapSelection` **is `Copy`** — drop it |
| 15 | Task 10 Step 4 rustfmt | `app_skirmish_shell_render.rs` is a **module root** — violates the plan's own rule |
| 16 | `Self::shell_rng(state)` (Task 11 Step 2) | never defined; Step 3 defines `DialogRng` |

**Also:** Task 12 hand-rolls the sentinel upsert while
`ChooseMapModalState::create_random_map` (`choose_map.rs:149`) already does it — and the
hand-rolled version **drops the `random_maps_allowed` gate and the `refresh_records` call**
that make the new row appear. Reuse it.

**Architectural inversion:** `ChooseMapModalButton` lives in `layout.rs`, and `state.rs`
imports *from* layout. Putting `RandomMapSetupControl` in `state/` while `layout.rs`'s
hit-test returns it inverts that direction. Define it in `layout.rs`.

**Blast radius (Task 1 Step 4)** — functionally right, three details wrong: exactly **two**
sites break (`options.rs:175`, `options.rs:262`, both exhaustive test literals);
`build.rs` already has `..Default::default()` (nothing to do); `app_init.rs:352` is
`::default()`, not a literal; and `generate_map` takes `&RmgOptions` — it does not construct one.

---

## 6. Task ordering

- **BLOCKING:** Task 10 dispatches on `random_map_setup_modal`, created in **Task 11** — its
  "Expected: compiles" is impossible. Hoist the field into Task 10. (The field lives in
  `SkirmishShellState` at `state/player_name.rs:256/302`, *not* in `state.rs`.)
- Task 11 forward-references Tasks 12/13 with no stub code → non-exhaustive match.
- Task 9's line anchor (`layout.rs:41-44`) is invalidated by Task 8 inserting ~28 lines
  above it. Anchor on the text, or run Task 9 first.
- Tasks 1 and 12 touch files absent from their own Files lists and the File Map, so Task 14
  Step 3's "only File Map files changed" check yields a false failure.

---

## 7. False positives caught

- **Geometry values** — I expected drift given the volume; every numeric constant matched
  the resource. The problem was inheritance, not extraction.
- **Randomize order** — the counter-intuitive time-before-resources ordering is correct.
- **`RandomRanged` inclusivity** — the author's headline worry was unfounded; Tasks 2–3 need
  no change.
- **Save `0x6C3` gating on `generated`** — looked like an over-simplification, but matches
  native in all four transitions.

## 8. Theoretical (no action)

- `DialogRng::ranged` returns `min` when `max <= min`; native *swaps* when `max < min`. No
  callsite passes an inverted range (vegetation is pre-collapsed), so unreachable.
- **UI RNG instance** unresolved — would need disassembly of the per-callsite `ECX` setup.
  Acceptable: the terrain is decided by the seed via the already-exact generator RNG, so the
  choice only affects *which* random config is offered. A separate `DialogRng` is in fact
  **strictly safer** than native, since it cannot advance a stream shared with gameplay.
  Record as a named, accepted divergence rather than "verified equivalent".

## 9. Out of scope — RESOLVED 2026-07-21

The map-type/water-table "mismatch" was a **false alarm**. `RandomMapGenerator__Generate`
(`0x00598960`) gates the region/bridge block on exactly `map_type == 3 || map_type == 4` —
identical to `build.rs:170`. The confusion was the stage **name**: `IslandPasses` is a
misnomer; 3/4 are *inland* and *mountainous* (the block calls
`MapClass__MarkBridgesForRepair_Low`), while archipelago is map_type **0** and gets its
75–100% water from the *normal* water path. Renaming the stage is cosmetic.

Three genuinely open backend items surfaced by that decompile:
1. **Water gate** — native applies `water_amount != 0` **only** on the 3/4 branch.
2. **Tech buildings** — run only when `map_type != 0`.
3. **Rocks** — gated on theater (`+0x38 == 0`), not map type; the port already matches.
