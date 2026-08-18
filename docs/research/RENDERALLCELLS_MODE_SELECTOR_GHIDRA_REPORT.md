# RenderAllCells Mode Selector -- Ghidra Research Report

**Address(es):** `0x00657CE0` (`RadarClass::RefreshRadar`), `0x00656150` (`RadarClass::RenderAllCells`), `0x00656EC0` (`RadarClass::Update`), `0x00655C50` (`RadarClass::RenderCellPixel`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** selector conditions that choose native full-surface `RenderCellPixel` loops versus `RenderAllCells` in live YR radar refresh/update paths.  
**Non-Scope:** object-dot gates/colors, dirty queue primitive internals, spy satellite geometry, gap/shroud effect geometry, terrain color source inventory, viewport rectangle raster details.  
**Confidence:** High for direct-call selector behavior; Medium for runtime frequency of the null-window fallback because no runtime breakpoint sampled `g_hWnd`.  
**Active in YR:** Yes for the `RenderCellPixel` path in standard windowed YR. Conditional for `RenderAllCells`: only selected when `g_hWnd` / `DAT_00B73550` is null.

## 0. Working Contract

- **Target question:** exactly when does native `RadarClass::RenderAllCells @ 0x00656150` run instead of dirty/full `RenderCellPixel @ 0x00655C50`?
- **Non-goals:** do not re-open object gates, dirty-list storage, spy satellite, gap generator, terrain colors, or chrome.
- **Evidence needed to mark COMPLETE:** direct caller census for `RenderAllCells`; decompile plus assembly branch context for `RefreshRadar`; decompile plus call context for `Update`; proof that `DAT_00B73550` is not a shroud/FogOfWar selector.
- **Stop conditions:** stop after all direct `RenderAllCells` callers and all `RenderCellPixel` callers in the radar selector surface are categorized, and after stale doc replacement wording is written.

## 1. Overview

`RenderAllCells` is not selected by rules shroud, `Shroud=yes/no`, or `FogOfWar=no`. The selector is the global main window handle at `0x00B73550`: `RefreshRadar` takes a fast terrain-blit plus `RenderAllCells` overlay only when that handle is null. With the normal window present, `RefreshRadar` full-refreshes every primary-surface pixel through `RenderCellPixel`, so shroud/fog/object gates remain active for full refreshes.

`RadarClass::Update` never calls `RenderAllCells`. Its active radar path drains terrain dirty, rerenders the generated terrain dirty rect through `RenderCellPixel`, rerenders explicit pixel dirty entries through `RenderCellPixel`, draws late overlays, and blits the accumulated primary-surface rect into the sidebar.

## 2. Key Fields / Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `0x00B73550` | `g_hWnd`, main game window handle; null selects `RenderAllCells` fallback in `RefreshRadar` | `RefreshRadar @ 0x00657CE0`; prior correction in `BSURFACE_CIRCBUF_ABUFFER_REPORT.md` | Yes, normally non-null after window creation |
| `RadarClass+0x121C` | primary radar surface; destination for `RenderCellPixel` and `RenderAllCells` | `0x00655C50`, `0x00656150`, `0x00657CE0` | Yes |
| `RadarClass+0x1220` | generated secondary terrain surface; copied to primary in the null-window fast path and terrain dirty path | `0x00657D0D..0x00657D2B`, `0x00656EC0` | Yes |
| `RadarClass+0x1274` | visited bitfield used by `RenderAllCells`, `MarkCellDirty`, and reset after `Update` | `0x00656150`, `0x006562D0`, `0x00657872` | Yes |
| `RadarClass+0x14B0/+0x14AC` | radar mode/state; `Update` treats mode `1` plus state `1` as active/open for sidebar blit | `0x00656EF7..0x00656F0D`, `0x00656E50` | Yes |
| `RadarClass+0x14D9` | radar dirty flag; gates `Update` work but does not choose `RenderAllCells` | `0x006571C1..0x006571DA` | Yes |

## 3. Selector Logic

### `RefreshRadar @ 0x00657CE0`

Active in YR: Yes, called by shroud reset/reveal/blackout, scenario init/load, map size changes, and gap/cloak shroud updates. Evidence: direct call scan found callers at `0x00577B91`, `0x00577C76`, `0x00577D79`, `0x00577F0D`, `0x00578069`, `0x0067E6BD`, `0x00687C84`, `0x006E2257`, `0x006FB44B`, `0x006FB728`.

Selector:

1. Read `g_hWnd` at `0x00B73550`.
2. If nonzero, get primary surface width and height and nested-loop every `(x,y)`:
   `for y in 0..height { for x in 0..width { RenderCellPixel(x,y); } }`.
   Evidence: decompile `0x00657CE0`; assembly `0x00657CF0 JNZ 0x00657D3D`, loop call `0x00657D77 CALL 0x00655C50`.
   Active in YR: Yes for standard windowed gameplay.
3. If zero, copy the full secondary terrain rect into the primary surface, then call `RenderAllCells`.
   Evidence: decompile `0x00657CE0`; assembly `0x00657D0D..0x00657D2B` virtual rect/blit calls, then `0x00657D30 CALL 0x00656150`.
   Active in YR: Conditional, only when the main window handle is null.

Read-only PE direct-call scan found exactly one direct `CALL 0x00656150`, at `0x00657D30`, and no literal function-pointer reference to `0x00656150`. This makes `RefreshRadar` the only direct selector for `RenderAllCells` in the executable image scanned.

### `Update @ 0x00656EC0`

Active in YR: Yes. `RadarClass::Draw @ 0x00653100` calls `Update` every radar draw pass after `FUN_0065FDD0`; assembly `0x0065336D CALL 0x0065FDD0`, `0x00653374 CALL 0x00656EC0`.

`Update` does not call `RenderAllCells`. It calls:

- `ClearBackground @ 0x00655250` when any update condition fires, after clearing `+0x14D9` if radar is active/open. Evidence: assembly `0x006571C9..0x006571DA`.
- `RenderCellPixel @ 0x00655C50` for each pixel in the terrain-generated rect `+0x1248..+0x1254`. Evidence: assembly `0x006574A4..0x006574C3`.
- `RenderCellPixel @ 0x00655C50` for each explicit pixel dirty vector entry, back-to-front. Evidence: assembly `0x0065750B..0x00657526`.
- late radar events and spy satellite overlay after pixel rerendering. Evidence: decompile `0x00656EC0`; assembly after `0x00657537`.

`Update` work condition is broad: dirty flag, viewport rect movement/size change, object-moved/dirty-list state, terrain dirty count, explicit pixel dirty count, or spy-satellite update. The branch at `0x006571C1 JZ 0x00657872` skips the rendering body if none of these conditions requires work. Active in YR: Yes.

### `RenderAllCells @ 0x00656150`

Active in YR: Conditional. It is compiled and internally verified, but selected only by the null-window `RefreshRadar` branch.

It clears `+0x1274`, scans 256 tracker buckets (`0x1800 / 0x18`), draws at most one object per visited pixel, and writes directly to the primary surface. It does not evaluate `IsShrouded`, `IsFogged`, `RadarInvisible`, or `Insignificant` gates. Those omissions are safe only under the selector's null-window fallback, not as a normal gameplay shortcut.

## 4. INI Keys

| Key | Default / stock value | Selector effect | Active in YR |
|---|---|---|---|
| `[MultiplayerDialogSettings] Shroud` | `rulesmd.ini:3031` = `yes`; comment says not yet supported | Does not select `RenderAllCells`; stale docs conflated this with `g_hWnd` | No direct selector effect |
| `[MultiplayerDialogSettings] FogOfWar` | `rulesmd.ini:3040` = `no` | Does not select `RenderAllCells`; standard YR still uses `RenderCellPixel` with `g_hWnd != 0` | Conditional TS/fog behavior elsewhere, not this selector |
| `[General] FogOfWar` | `rulesmd.ini:205` = `no` | No direct selector effect | Conditional elsewhere |
| `[General] ShroudGrow` | `rulesmd.ini:677` = `no` | No direct selector effect | TS legacy elsewhere |

## 5. Integration Points

`RefreshRadar` is the full-refresh entry used after global shroud/reveal state changes and load/init events. Its direct callers include local-player shroud reset/restore/blackout/reveal paths (`0x00577B60`, `0x00577C40`, `0x00577D40`, `0x00577EE0`, `0x00578030`), save/load/scenario init paths (`0x0067E440`, `0x00687C40`), map-size or terrain reinitialization (`0x006E21E0`), and gap/cloak shroud apply/remove paths (`0x006FB410`, `0x006FB700`).

`Update` is the ordinary per-draw retained-surface updater. It is reached from `Draw`, not from `RefreshRadar`, and it uses only `RenderCellPixel` for final pixel recomposition.

## 6. Current Rust Implementation Status

Rust currently has a single RGBA texture rebuild path in `src/render/minimap.rs::update_unit_dots` and a square `MINIMAP_SIZE = 200` in `src/render/minimap_helpers.rs`. It does not model native `RefreshRadar` as a full `RenderCellPixel` sweep when shroud/global reveal changes, and it does not need to use `RenderAllCells` for standard windowed gameplay parity unless the engine intentionally models the native null-window fallback.

Rust also has one visibility/no-visibility branch around an optional `FogState`. That branch is not equivalent to native selector behavior: native `FogOfWar=no` or no fog does not choose `RenderAllCells`; the presence of the window handle does.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct caller census for `RenderAllCells` | verified | PE direct `CALL rel32` scan; assembly `0x00657D30` | indirect runtime dispatch not expected; no literal function pointer found |
| `RefreshRadar` nonzero-window branch | verified | decompile `0x00657CE0`; assembly `0x00657CF0`, `0x00657D3D..0x00657D77` | runtime sample of `g_hWnd` not taken |
| `RefreshRadar` null-window branch | verified | decompile `0x00657CE0`; assembly `0x00657D0D..0x00657D30` | exact teardown/startup frequency not sampled |
| `Update` render calls | verified | decompile `0x00656EC0`; assembly `0x006571DA`, `0x006574C3`, `0x00657520` | none for this selector slice |
| `Draw -> Update` liveness | verified | decompile `0x00653100`; assembly `0x00653374` | none |
| `DAT_00B73550` stale shroud label | verified | `RefreshRadar`; `BSURFACE_CIRCBUF_ABUFFER_REPORT.md:308..314` | ADDRESS_MAP still stale |
| Full object gate/color behavior inside `RenderCellPixel` and `RenderAllCells` | deferred | prior object-dot report | out-of-scope; already owned by sibling report |
| Spy satellite and gap special geometry | deferred | sibling reports | out-of-scope |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- What directly calls RenderAllCells? -> only `RefreshRadar` direct call at `0x00657D30`; no literal function-pointer reference found.` (evidence: read-only PE direct-call/literal scan; assembly `0x00657D30`)
- `[RESOLVED] OQ-02 -- What branch selects RenderAllCells? -> `RefreshRadar` takes it only when `g_hWnd` / `DAT_00B73550` is zero.` (evidence: decompile `0x00657CE0`; assembly `0x00657CE8..0x00657D30`)
- `[RESOLVED] OQ-03 -- What happens when the window handle is nonzero? -> full primary-surface nested loop through `RenderCellPixel`.` (evidence: `0x00657D3D..0x00657D77`)
- `[RESOLVED] OQ-04 -- Is `DAT_00B73550` a shroud-enabled flag? -> no, it is the main window handle.` (evidence: `BSURFACE_CIRCBUF_ABUFFER_REPORT.md:308..314`; `RefreshRadar` behavior)
- `[RESOLVED] OQ-05 -- Do `Shroud=yes` or `FogOfWar=no` choose RenderAllCells? -> no, no INI-read selector reaches this branch; the selector reads `g_hWnd`.` (evidence: `0x00657CE0`; `rulesmd.ini:3031`, `rulesmd.ini:3040`)
- `[RESOLVED] OQ-06 -- Does ordinary `Update` ever use RenderAllCells? -> no; its only final-pixel rerender calls are `RenderCellPixel`.` (evidence: direct-call scan; `0x006574C3`, `0x00657520`)
- `[RESOLVED] OQ-07 -- When does `Update` run? -> `RadarClass::Draw` calls it after radar chrome/state prep.` (evidence: `0x00653374`)
- `[RESOLVED] OQ-08 -- Does `Update` skip work when no dirty/viewport/object/satellite condition changed? -> yes, branch jumps to visited-bitfield cleanup/exit when the broad condition is false.` (evidence: `0x006571A1..0x006571C3`)
- `[RESOLVED] OQ-09 -- Does `RefreshRadar` use dirty lists? -> no, nonzero-window branch brute-forces width*height through `RenderCellPixel`; null-window branch blits full secondary then overlays all cells.` (evidence: `0x00657CE0`)
- `[RESOLVED] OQ-10 -- Is `RenderAllCells` safe as a gameplay fast path under standard YR FogOfWar=no? -> no; it bypasses shroud/fog/object gates and native does not select it for that reason.` (evidence: `0x00656150`, `0x00657CE0`)
- `[RESOLVED] OQ-11 -- What should Rust compare against for standard windowed gameplay? -> retained primary-surface `RenderCellPixel` composition for full refresh and dirty paths, not `RenderAllCells`.` (evidence: `0x00657D77`, `0x006574C3`, `0x00657520`)
- `[DEFERRED] OQ-12 -- What exact startup/teardown moments call `RefreshRadar` while `g_hWnd == 0`?` (category: needs-runtime-debugger; reason: static evidence proves the selector but not runtime frequency; next-step-if-pursued: breakpoint on `0x00657CE0` and sample `0x00B73550` during startup/load/exit)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `RefreshRadar @ 0x00657CE0` | caller-triggered full refresh | none | full primary surface | see chosen path | Yes | full-refresh owner |
| 2A | `RenderCellPixel @ 0x00655C50` loop | `g_hWnd != 0` at `0x00657CF0` | none | every `(x,y)` primary pixel | secondary terrain + shroud/fog/object packing | Yes in standard windowed YR | full recomposition |
| 2B | secondary-to-primary blit | `g_hWnd == 0` | none | full secondary rect to primary | surface copy | Conditional | fallback terrain copy |
| 3B | `RenderAllCells @ 0x00656150` | only after 2B | none | tracker object pixels | color-scheme prepacked palette entry | Conditional | fallback object overlay |
| 4 | `Update @ 0x00656EC0` | draw-time dirty/viewport/object/satellite condition | events/satellite later | accumulated dirty rect | `RenderCellPixel` + late overlays | Yes | retained radar updater |

Asset role matrix:

| Asset / surface | Loaded | Drawn | Visible in target | Content | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| primary radar surface `+0x121C` | Yes | Yes | Yes | Yes | No | Yes | No | No | `0x00655C50`, `0x00656150`, `0x00656EC0`, `0x00657CE0` |
| secondary terrain surface `+0x1220` | Yes | copied/read | indirect | Yes | No | No | No | No | `0x00657D0D..0x00657D2B`, `0x00656EC0` |
| object tracker `+0x1258` | runtime | pixels drawn | Yes/Conditional | No | No | object overlay | No | No | `0x00656150`, `0x00655C50` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard windowed `RefreshRadar` full refresh rerenders every primary-surface pixel through `RenderCellPixel`; `RenderAllCells` is not the normal FogOfWar=no shortcut. | `0x00657CE0`; assembly `0x00657CF0`, `0x00657D77`; `BSURFACE_CIRCBUF_ABUFFER_REPORT.md:308..314` | Mismatch: Rust has one `visibility: Option<FogState>` RGBA rebuild branch and no native full `RenderCellPixel` retained-surface sweep. | `src/render/minimap.rs::update_unit_dots`, future retained radar surface path | Model global reveal/reshroud/full refresh as per-pixel composition over the native primary surface when standard windowed gameplay is targeted. | Trigger `MapClass::RevealEntireMap`/spy-sat reveal equivalent; every primary radar pixel is recomposed through the same gate path as dirty pixels. Proposed test: `minimap_refresh_radar_windowed_uses_render_cell_pixel_full_sweep`. | Do not replace full refresh with `RenderAllCells` just because `FogOfWar=no`; that bypasses native gates and colors. |
| `RadarClass::Update` never selects `RenderAllCells`; terrain dirty rect pixels and explicit dirty pixels both call `RenderCellPixel`. | `0x00656EC0`; assembly `0x006574C3`, `0x00657520`; direct-call scan | Mismatch: Rust rebuilds/reuploads a full RGBA texture and lacks a retained primary pixel dirty sweep. | `src/render/minimap.rs`, future dirty rect/sidebar surface model | Keep terrain-dirty refresh and pixel-dirty recomposition on the per-pixel path, with late event/satellite overlays after final pixel writes. | Dirty terrain rect plus two object dirty entries in one frame call the pixel compositor for terrain rect pixels first, explicit dirty pixels second. Proposed test: `minimap_update_dirty_paths_never_use_render_all_cells`. | Do not unify `Update` with the null-window `RefreshRadar` fallback. |
| `DAT_00B73550` is a window handle selector, not a shroud/FogOfWar rules flag. | `0x00657CE0`; prior correction `BSURFACE_CIRCBUF_ABUFFER_REPORT.md:308..314`; INI defaults `rulesmd.ini:3031`, `rulesmd.ini:3040` | Mismatch risk: Rust/no-doc consumers may treat no FogState as native `RenderAllCells` eligibility. | minimap mode-switch docs/tests and any future renderer selector | Select standard parity behavior from game/window mode, not rules shroud/fog defaults; if the engine does not model native null-window drawing, omit `RenderAllCells` from standard gameplay. | With `FogOfWar=no` and `Shroud=yes` defaults, a full radar refresh still uses per-pixel shroud/fog-aware composition. Proposed test: `minimap_fogofwar_off_does_not_select_render_all_cells`. | Do not call `0x00B73550` `shroud_enabled`; that stale label inverts the selector. |

### Negative Facts / Do Not Do

- Do not implement `RenderAllCells` as the standard YR no-shroud/FogOfWar-off minimap path. Native selects it only when `g_hWnd == 0` (`0x00657CE0`).
- Do not label `DAT_00B73550` / `0x00B73550` as `shroud_enabled`. It is the main window handle; prior `RADAR_MINIMAP_RENDERING.md` and `ADDRESS_MAP.md` wording is stale.
- Do not make `RadarClass::Update` call a whole-tracker object overlay path. Native `Update` uses `RenderCellPixel` for generated terrain rect pixels and explicit pixel dirty entries (`0x006574C3`, `0x00657520`).
- Do not use `RenderAllCells` to avoid shroud/fog checks in standard gameplay. Its missing gates are a consequence of the null-window fallback, not proof that those gates are disabled when `FogOfWar=no`.
- Do not assume `RefreshRadar` is incremental. With `g_hWnd != 0`, it loops the entire primary surface; dirty queues belong to `Update`.

### Stale Docs / Follow-up Docs

- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace section "Shroud flag: DAT_00B73550" with: "`DAT_00B73550` is `g_hWnd`, the main game window handle, not a shroud-enabled flag. `RefreshRadar @ 0x00657CE0` selects the full `RenderCellPixel` sweep when `g_hWnd != 0`; only the null-window fallback copies the secondary surface and calls `RenderAllCells @ 0x00656150`. `Shroud=`, `FogOfWar=`, and standard YR `FogOfWar=no` do not select `RenderAllCells`."
- `docs/research/RADAR_MINIMAP_RENDERING.md`: replace "With shroud (`DAT_00b73550 != 0`)" with "With a live main window (`g_hWnd != 0`)"; replace "Without shroud" with "With no main window handle (`g_hWnd == 0`) fallback."
- `docs/research/ADDRESS_MAP.md`: replace "`0x00B73550 | int | Shroud enabled flag | RADAR_MINIMAP`" with "`0x00B73550 | HWND/int | main game window handle (`g_hWnd`); `RefreshRadar` null-window fallback selector | RADAR_MINIMAP`".
- `docs/research/RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md`: replace "`RenderAllCells` is a no-shroud fast overlay path" with "`RenderAllCells` is the null-window `RefreshRadar` fallback overlay path; it is not selected by standard YR `FogOfWar=no` or `Shroud=` settings."

## Remaining Uncertainty

- Exact startup/load/teardown frequency of `g_hWnd == 0` calls to `RefreshRadar` was not runtime-sampled. Static evidence proves the selector and its branch behavior; a debugger trace would quantify how often the fallback runs outside normal windowed gameplay.

## Sources

- Ghidra decompile: `RadarClass::RefreshRadar @ 0x00657CE0`.
- Ghidra assembly context: `0x00657CE8..0x00657D30` null-window fallback and `RenderAllCells` call.
- Ghidra assembly context: `0x00657D3D..0x00657D77` nonzero-window full `RenderCellPixel` loop.
- Ghidra decompile: `RadarClass::RenderAllCells @ 0x00656150`.
- Ghidra decompile: `RadarClass::Update @ 0x00656EC0`; assembly contexts `0x006571DA`, `0x006574C3`, `0x00657520`.
- Ghidra decompile: `RadarClass::Draw @ 0x00653100`; assembly `0x00653374`.
- Ghidra decompile: `MapClass::RestoreShroud @ 0x00577B60`, `MapClass::ResetShroud @ 0x00577C40`, `MapClass::ResetShroudWithReveal @ 0x00577D40`, `MapClass::BlackoutShroud @ 0x00577EE0`, `MapClass::RevealEntireMap @ 0x00578030`, `ScenarioClass::Full_Init @ 0x00687C40`, `TechnoClass::UpdateCloakShroud @ 0x006FB410`, `TechnoClass::RemoveCloakShroud @ 0x006FB700`.
- Read-only local PE scan of `gamemd.exe` direct `CALL rel32` sites for `0x00656150`, `0x00655C50`, `0x00656EC0`, `0x00657CE0`.
- Prior doc correction: `docs/research/BSURFACE_CIRCBUF_ABUFFER_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
