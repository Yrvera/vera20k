# Loading ProgressClass Value / Max Mapping - Ghidra Research Report

**Address(es):** `0x00642A60`, `0x00643E90`, `0x00643C50`, `0x006433D0`, `0x00643720`, `0x00643400`, `0x0069AE90`, `0x00684620`, `0x007C5F00`, `0x007C5EE4`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** numeric ProgressClass value/max/lane semantics for standard offline Skirmish `PROGBARM.SHP`: max value, lane count, lane storage, `FUN_0069AE90` milestone mapping, duplicate/lower suppression, draw fill-width conversion, and edge behavior relevant to Rust.  
**Non-Scope:** caller milestone enumeration after first renderer, loading background composition, `mmpb.shp`, localized text, and campaign/multiplayer visual variants except where needed as negative boundaries.  
**Confidence:** High for standard Skirmish value/max/lane behavior; Medium for the exact CPU rounding mode behind `Math__ftol` because the binary uses the startup FPU control word rather than an inline constant.  
**Active in YR:** Yes for standard offline Skirmish (`g_GameMode == 5`) through `ScenarioClass__Read_Scenario @ 0x00684620`.

## 1. Target Question

Verify `ProgressClass` value/max/lane mapping for standard offline Skirmish `PROGBARM.SHP`: max value, current/lane fields, `FUN_0069AE90` argument-to-visible-fill mapping, duplicate/lower suppression, edge cases, and clamping/rounding behavior relevant to a Rust implementation.

## 2. Non-Goals

- Do not enumerate every later milestone caller; slot 1/2 own the milestone ledger.
- Do not re-investigate first loading background or progress geometry beyond value-to-width math.
- Do not implement Rust or patch in-repo docs.
- Do not mutate the Ghidra database.

## 3. Evidence Needed To Mark COMPLETE

- Prove standard Skirmish `Read_Scenario` initializes ProgressClass max and lane count.
- Prove exact fields for max, enabled flag, lane count, lane values, HWND, manager pointer, and width source.
- Prove `FUN_0069AE90` milestone gate and random-map halving boundary.
- Prove `FUN_00643C50` scale, upper clamp, duplicate-value gate, and repaint/direct-draw dispatch.
- Prove draw helper width formula and the actual rounding routine used.
- Record negative edge facts: no lower clamp, no zero-max guard, no row-index guard in setter.

## 4. Stop Conditions

- Stop after the above value/max/lane path is closed for standard offline Skirmish.
- Stop before follow-up caller enumeration, asset-frame dumping, or runtime screenshot capture.

## 5. Class Layout / Key Offsets

| Offset | Type / interpretation | Verified writer / reader | Standard Skirmish value |
|---:|---|---|---|
| `+0x04` | manager callback pointer | written by `0x00643E80`, read by `0x00643C50` | `LoadProgressMgr` pointer while loading |
| `+0x08 + lane*8` | lane current value as `double` | zeroed by `0x00642A60`, written by `0x00643C50`, read by `0x00643E90`/draw | lane `0`, starts `0.0` |
| `+0x48/+0x4C` | max value as `double` | written by `0x00642A60` | `100.0` (`0x40590000_00000000`) |
| `+0x60` | enabled byte | written `1` by `0x00642A60`, cleared by `0x00642AD0` | `1` during load |
| `+0x61` | signed byte lane count | written by `0x00642A60` | `1` for campaign and offline Skirmish |
| `+0x64` | HWND paint switch | written by `0x00642A60` | `0`, so direct-draw path |
| `+0x54` | progress SHP pointer | written by `0x00642C20` | `PROGBARM.SHP` for non-campaign |
| `+0x68/+0x6C` | stored draw origin | written by `0x00642C80` | explicit Skirmish origin from `0x00552BE0` |
| `+0x70/+0x71` | non-campaign row/color-fill flags | written by `0x00642C80` | both true for Skirmish |
| `+0x78` | row width override | written by `0x00642DF0` | `0x146` or `0x196`; not the fill width |

## 6. Core Logic

### 6.1 Standard Skirmish Initialization

`ScenarioClass__Read_Scenario @ 0x00684620` initializes the global `ProgressClass` object at `0x00AC4F58` before reading/full-init work. For `g_GameMode == 5`, the lane count argument is forced to `1`, not `DAT_00A8DA84`; the max is the double `100.0`; the HWND argument is `0`.

Assembly around `0x006846F9..0x00684706` pushes, in order, HWND `0`, lane count, max high word `0x40590000`, max low word `0`, then calls `0x00642A60` with `ECX=0x00AC4F58`. `0x00642A60` stores max at `+0x48/+0x4C`, writes lane count to signed byte `+0x61`, zeros `+0x08..` lane doubles for each lane, sets enabled byte `+0x60=1`, stores HWND to `+0x64` only if nonzero, otherwise keeps it zero and resets the DirectDraw surface path.

Active in YR: Yes, for standard offline Skirmish (`g_GameMode == 5`, map editor off).

### 6.2 Milestone Callback Gate

`FUN_0069AE90(param_2)` is the live loading milestone callback. If `ScenarioClass+0x34BD` is nonzero, the input milestone is halved first. The halving uses the signed `CDQ; SUB EAX,EDX; SAR EAX,1` sequence, i.e. signed divide-by-two semantics chosen by the compiler, before any progress comparison.

The callback then reads current lane `0` with `FUN_00643E90(0)`, multiplies that fraction by `100.0`, and only calls `FUN_00643C50(row=0, percent=param_2, x=-1, y=-1)` when:

```text
current_fraction * 100.0 < requested_milestone
```

The comparison is strict. Equal and lower milestones do not reach the ProgressClass setter.

Active in YR: Yes. Random-map halving is Conditional on `ScenarioClass+0x34BD`; normal stock map Skirmish does not take the halving branch.

### 6.3 Current Percent Helper

`FUN_00643E90(row)` returns:

- if `row < lane_count`: `lane_value[row] / max`
- otherwise: `max`

There is no zero-max guard. Standard Skirmish uses row `0`, lane count `1`, max `100.0`, so the active return is `lane0 / 100.0`.

Active in YR: Yes for row `0`; the out-of-range fallback is not active in the standard callback path.

### 6.4 Setter, Clamp, and Duplicate Gate

`FUN_00643C50(row, percent, x, y)` first computes an integer old average percent for the optional manager message. Then it writes:

```text
new_lane = max * 0.01 * percent
```

For standard Skirmish:

```text
new_lane = percent
```

because max is `100.0`.

The setter clamps only above max. If `new_lane > max`, it copies the exact max double into the lane. It does not clamp negative values upward to zero. After the possible upper clamp, it compares the old lane double with the stored lane double. If the double is equal, the function returns without manager notification, without `WM_PAINT`, and without direct draw.

If the lane changed, it computes the new integer average percent, optionally sends manager message `0x11AE` with the old/new integer average pair, then repaints. Standard Skirmish has `+0x64 == 0`, so it calls direct draw `FUN_00643AE0(-1,-1)` instead of sending HWND `WM_PAINT`.

Active in YR: Yes. The negative-input no-lower-clamp edge is not reached by ordinary `FUN_0069AE90` from a zero-or-positive current value, but it is the verified setter behavior.

### 6.5 Average Percent

Both `FUN_00643C50` and `FUN_006433D0` compute average progress by summing all lane doubles, dividing by signed lane count, then dividing by max. The value sent to the manager is `Math__ftol((sum / lane_count / max) * 100.0)`.

For standard Skirmish, lane count is `1`, so average percent is the same as lane `0` percent. The code has no lane-count-zero guard, but standard Skirmish initialization prevents zero lanes.

Active in YR: Yes for one-lane average; zero-lane behavior is not active in standard Skirmish.

### 6.6 Fill Width Mapping

The row draw helper `0x00643720` computes the active row fraction as `lane_value / max` for valid rows and passes that fraction to `0x00643400`. The filled `PROGBARM.SHP` span is:

```text
fill_width = Math__ftol(frame0_width * fraction)
fill_height = frame0_height
```

The helper then draws `PROGBARM.SHP` frame `0` with a source/clip rect whose width is the computed `fill_width`. The native path clips the frame draw; it does not scale the SHP.

There is no explicit draw-helper clamp to `0..=frame0_width`. In standard callback use, the upper clamp in `FUN_00643C50` keeps `fraction <= 1.0`, and the callback gate prevents ordinary negative progress, so the active standard path naturally stays within the frame width.

Active in YR: Yes for standard non-campaign Skirmish. Out-of-range row and negative setter cases are verified but not standard-path active.

### 6.7 Rounding / `Math__ftol`

`Math__ftol @ 0x007C5F00` converts the current x87 `ST0` value with `FISTP qword ptr [...]`. If the live FPU control word differs from global `DAT_00822D80`, it loads `DAT_00822D80` first, then performs the same `FISTP`. `DAT_00822D80` is initialized from the startup FPU control word by `0x007C5EE4`, called from `WinMain`.

Verified binary behavior: progress average percent and fill width use this native `Math__ftol` routine. This report does not replace that with a generic Rust `floor`, `round`, or truncation claim.

Active in YR: Yes. Exact runtime rounding direction depends on the startup FPU control word value stored in `DAT_00822D80`; the load-bearing implementation fact is that gamemd uses `Math__ftol/FISTP`, not a hand-written floor/truncate expression.

## 7. INI Keys

No INI key controls this numeric mapping. Assets and side selection are configured elsewhere, but max `100.0`, lane count branching, `0.01`, upper clamp, and draw fill math are hardcoded in the binary path.

## 8. Integration Points

| Point | Role | Evidence | Active for standard Skirmish? |
|---|---|---|---|
| `ScenarioClass__Read_Scenario @ 0x00684620` | initializes ProgressClass max/lane/HWND, selects `PROGBARM.SHP` | decompile + assembly `0x006846F9..0x00684706` | yes |
| `FUN_0069AE90 @ 0x0069AE90` | public load milestone callback, monotonic gate | decompile + assembly `0x0069AECB..0x0069AEE?` | yes |
| `FUN_00643E90 @ 0x00643E90` | row current fraction helper | decompile | yes, row `0` |
| `FUN_00643C50 @ 0x00643C50` | lane setter, upper clamp, duplicate gate, repaint dispatch | decompile + assembly `0x00643C94..0x00643D4B` | yes |
| `FUN_00643720 @ 0x00643720` | row draw helper, valid-row fraction selection | decompile | yes |
| `FUN_00643400 @ 0x00643400` | fill width conversion and clipped frame-0 draw | decompile + assembly `0x0064352C..0x00643594` | yes |
| `Math__ftol @ 0x007C5F00` | integer conversion for old/new average and fill width | decompile + assembly | yes |

## 9. Current Rust Implementation Status

Current Rust has no native loading ProgressClass model. The app presents one egui loading frame and then synchronously transitions through map load; no `PROGBARM.SHP` lane/max state, milestone setter, duplicate suppression, or native `Math__ftol` equivalent exists in the loading path.

Relevant current surfaces:

- `src/app.rs`: `GameScreen::Loading` and one-frame transition after present.
- `src/app_transitions.rs`: synchronous `transition_to_in_game`.
- `src/ui/main_menu.rs`: egui loading text/status; no native progress model.
- Future loading renderer/progress model: must own this behavior above `sim/`.

## 10. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00642A60` max/lane/HWND initialization | verified | decompile + assembly | none for standard Skirmish |
| `0x00684620` standard Skirmish caller args | verified | decompile + assembly | none for max/lane mapping |
| `0x0069AE90` strict monotonic callback gate | verified | decompile | caller milestone ledger is out-of-scope |
| random-map halving branch | verified | decompile | random-map UI flow outside standard stock map scope |
| `0x00643E90` current fraction helper | verified | decompile | out-of-range row not active in standard path |
| `0x00643C50` setter/upper clamp/duplicate gate | verified | decompile + assembly | none for standard path |
| `0x006433D0` average helper | verified | decompile | multiplayer multi-lane wait behavior out-of-scope |
| `0x00643720` valid-row fraction to draw helper | verified | decompile | full visual layout already covered by sibling report |
| `0x00643400` fill-width conversion | verified | decompile + assembly | exact retail frame dimensions remain asset-data work, not numeric mapping |
| `0x007C5F00` / `0x007C5EE4` ftol routine | verified | decompile + assembly | exact live FPU control word value would require runtime/startup observation |

## 11. Open Questions - Final State

- `[RESOLVED] OQ-01 - What max does standard Skirmish use? -> `100.0` double, stored at ProgressClass `+0x48/+0x4C`.` (evidence: `0x006846F9..0x00684706`, `0x00642A60`)
- `[RESOLVED] OQ-02 - How many lanes does standard offline Skirmish use? -> One lane; `g_GameMode == 5` takes the same one-lane branch as campaign, not the multiplayer player-count branch.` (evidence: `0x006846E4..0x00684706`)
- `[RESOLVED] OQ-03 - Where is current value stored? -> Lane `N` is a double at `+0x08 + N*8`, zeroed during initialization and written by `0x00643C50`.` (evidence: `0x00642A60`, `0x00643C50`)
- `[RESOLVED] OQ-04 - How does `FUN_0069AE90` map a milestone? -> For normal maps it compares `lane0/max*100 < milestone` and calls `0x00643C50(0, milestone, -1, -1)` only on strict advance.` (evidence: `0x0069AE90`)
- `[RESOLVED] OQ-05 - How does random-map mode alter milestones? -> It halves the integer input first when `ScenarioClass+0x34BD != 0`; standard non-random Skirmish does not use this branch.` (evidence: `0x0069AE90`, `0x00684620`)
- `[RESOLVED] OQ-06 - What does the setter store? -> `max * 0.01 * percent`; with max `100.0`, stored lane equals requested percent for standard Skirmish.` (evidence: `0x00643C94..0x00643CA4`)
- `[RESOLVED] OQ-07 - Is there upper clamping? -> Yes, if new lane exceeds max it copies max into the lane before duplicate comparison.` (evidence: `0x00643CA8..0x00643CBC`)
- `[RESOLVED] OQ-08 - Is there lower clamping? -> No setter-side lower clamp exists; ordinary callback gating prevents negative/lower standard milestones from reaching the setter.` (evidence: `0x00643C94..0x00643CC9`, `0x0069AE90`)
- `[RESOLVED] OQ-09 - Do duplicate or lower milestones redraw? -> No; lower/equal are rejected by `0x0069AE90`, and same stored double returns from `0x00643C50` before notification/repaint.` (evidence: `0x0069AE90`, `0x00643CC0..0x00643D50`)
- `[RESOLVED] OQ-10 - How does stored lane become visible fill? -> Valid row fraction is `lane/max`; fill width is `Math__ftol(frame0_width * fraction)`; height is frame0 height; frame 0 is clipped/drawn, not scaled.` (evidence: `0x00643720`, `0x00643400`)
- `[RESOLVED] OQ-11 - Does draw helper clamp the fill width? -> No explicit draw-helper clamp was found; standard path relies on setter upper clamp and monotonic non-negative callback inputs.` (evidence: `0x00643400`, `0x00643C50`)
- `[RESOLVED] OQ-12 - What rounding primitive is used? -> `Math__ftol @ 0x007C5F00` uses x87 `FISTP` under startup FPU control word `DAT_00822D80`.` (evidence: `0x007C5F00`, `0x007C5EE4`)
- `[RESOLVED] OQ-13 - What happens if max is zero? -> No zero-max guard exists in the helpers; standard Skirmish initializes max to `100.0`, so zero-max is not active.` (evidence: `0x00642A60`, `0x00643E90`, `0x00643C50`)
- `[RESOLVED] OQ-14 - Does `FUN_00643C50` bounds-check row? -> No explicit row-index bound check in the setter; standard callers pass row `0`.` (evidence: `0x00643C50`, `0x0069AE90`)
- `[DEFERRED] OQ-15 - What is the exact live FPU rounding-control value in a retail run?` (category: needs-runtime-debugger; reason: binary stores startup control word and uses it for `FISTP`, but static analysis did not observe the live word value; next-step-if-pursued: log or debug `DAT_00822D80` after `WinMain` calls `0x007C5EE4`)

## 12. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x00684620 -> 0x00642A60` | `g_IsMapEditor == 0`, `g_GameMode == 5` | none | max/lane setup | n/a | yes | numeric progress init |
| 2 | `0x00684620 -> 0x00642C20` | non-campaign branch | `PROGBARM.SHP` | stored `+0x54` | no override convert for Skirmish | yes | progress asset selection |
| 3 | `0x0069AE90` | strict greater-than-current milestone | none | row `0`, `(-1,-1)` draw point | n/a | yes | milestone gate |
| 4 | `0x00643C50` | stored lane value changes | none | `(-1,-1)` resolves to stored origin in direct draw | n/a | yes | value update / repaint dispatch |
| 5 | `0x00643720 -> 0x00643400` | row `0 < lane_count` | `PROGBARM.SHP` frame `0` | fill width `Math__ftol(frame0_width * lane/max)` | player/session ColorScheme path per sibling report | yes | visible clipped progress fill |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `PROGBARM.SHP` | yes | yes | yes | no | yes | progress fill | no | no | `0x00684620`, `0x00642C20`, `0x00643720`, `0x00643400` |
| `SPLDBR.SHP` | branch-only | no for Skirmish | no for Skirmish | campaign | campaign | campaign | no | yes for Skirmish | `0x00684620` campaign branch |

## 13. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard offline Skirmish initializes one ProgressClass lane with max `100.0`, lane0 `0.0`, enabled `1`, and HWND `0`. | `0x006846F9..0x00684706`, `0x00642A60` | missing | future app-level loading progress model / renderer | Create a one-lane native loading progress state for standard Skirmish, using direct-draw-equivalent presentation. | Starting a stock offline Skirmish begins with progress lane0 at `0/100`, then advances by native milestones. Proposed test: `loading_progress_standard_skirmish_initializes_one_lane_max_100`. | Do not derive lane count from player count for offline Skirmish. |
| `FUN_0069AE90` and `0x00643C50` together suppress lower/equal/unchanged values; the setter only clamps above max. | `0x0069AE90`, `0x00643C50` | missing | loading milestone bridge and progress event tests | Accept only strict native advances from the callback path; cap over-100 to 100; emit no redraw on duplicates. | Feed milestones `3,3,2,100,200` and observe redraws only at `3` and `100`; `200` caps to unchanged `100`. Proposed test: `loading_progress_suppresses_non_advancing_and_caps_above_max`. | Do not smooth, continuously repaint, or lower-clamp in a way that hides native edge behavior from tests. |
| Visible fill width is `Math__ftol(frame0_width * lane/max)` using gamemd's `Math__ftol/FISTP`, not a generic floor/truncate expression. | `0x00643720`, `0x00643400`, `0x007C5F00`, `0x007C5EE4` | missing | future `PROGBARM.SHP` renderer/test math | Use a single native-compatible conversion helper for progress average and fill width; test fractional widths against the selected compatibility rule. | With a fixture frame width and lane/max ratio, rendered clip width matches native `Math__ftol` behavior. Proposed test: `loading_progress_fill_width_uses_gamemd_ftol`. | Do not document or implement this as plain floor, truncation, or scale-to-rect. |

## 14. Negative Facts / Do Not Do

- Do not use player count as the lane count for standard offline Skirmish; `g_GameMode == 5` initializes one lane.
- Do not redraw duplicate or lower milestones; the native callback has a strict greater-than-current gate and the setter has a second unchanged-double gate.
- Do not add a draw-helper clamp as a claimed native behavior. Standard inputs stay in range, but the verified draw helper itself does not clamp `fill_width` to `0..=frame_width`.
- Do not describe the fill conversion as generic Rust `floor`, truncation, or scaling. The verified primitive is gamemd `Math__ftol/FISTP`.
- Do not implement standard loading progress through an HWND-only path; standard Skirmish initializes `+0x64` null and direct-draws.

## 15. Remaining Uncertainty

- Exact live FPU control-word value used by `Math__ftol` was not runtime-observed. Static binary evidence proves the conversion primitive and control-word source, but a debugger/runtime trace would pin the final tie/fraction rounding direction.
- Exact numeric `PROGBARM.SHP` retail frame dimensions are asset data, not embedded in the value/max logic; sibling geometry report already defers asset dumping.

## 16. Stale Docs / Follow-up Docs

`docs/plans/2026-05-23-standard-offline-skirmish-loading-plan.md` should replace any wording that says progress clip math is "truncate-toward-zero, clamp to `0..=frame_width`, and `max_value == 0` behavior" with:

> Progress value mapping must model gamemd `ProgressClass`: standard offline Skirmish initializes one lane with max `100.0`; `FUN_0069AE90` admits only strict advancing milestones; `FUN_00643C50` stores `max * 0.01 * percent`, clamps only above max, and skips unchanged stored values. The visible `PROGBARM.SHP` frame-0 clip width is `Math__ftol(frame0_width * lane/max)` through gamemd's `Math__ftol/FISTP` helper. Do not claim a native lower clamp, draw-helper `0..=frame_width` clamp, or zero-max fallback; standard path remains in range by initialization and callback gating.

`docs/research/PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md` can keep its formula, but future summaries should expand "`ftol`" to "`Math__ftol @ 0x007C5F00` using x87 `FISTP` under startup FPU control word `DAT_00822D80`".

## Sources

- Ghidra decompile/disassembly: `0x00642A60`, `0x00684620`, `0x0069AE90`, `0x00643E90`, `0x00643C50`, `0x006433D0`, `0x00643720`, `0x00643400`, `0x007C5F00`, `0x007C5EE4`.
- Prior docs checked: `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`, `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`, `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs`, `src/app_transitions.rs`, `src/ui/main_menu.rs`.

## Status

PARTIAL only for exact live FPU control-word value / final rounding direction; COMPLETE for the standard offline Skirmish max/lane/storage/gate/clamp/direct-draw value mapping.
