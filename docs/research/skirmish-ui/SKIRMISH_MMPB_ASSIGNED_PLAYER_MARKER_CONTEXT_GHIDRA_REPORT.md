# Skirmish mmpb Assigned-Player Marker Context - Ghidra Research Report

**Address(es):** `0x00640A40` primary, caller `0x00553687` in `0x00552D60`, parent caller `ScenarioClass__Full_Init @ 0x00687588`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** live caller context for `mmpb.shp` assigned-player/house markers and contrast with offline Skirmish dialog `0x102` `DrawStartPositions @ 0x00640710`  
**Non-Scope:** full loading-screen text/layout reconstruction, full spawn-assignment algorithm, retail screenshot capture, and Rust implementation changes  
**Confidence:** High for caller identity and offline-dialog separation; Medium for player-facing screen naming because no runtime screenshot/debugger was used  
**Active in YR:** Conditional. The code is active in YR scenario-start/loading context for non-campaign modes, including Skirmish (`DAT_00A8B238 == 5` per prior SessionClass docs), but it is not active in the standard offline Skirmish setup dialog `0x102` `WM_PAINT` path.

## 1. Overview

`FUN_00640A40` is the verified `mmpb.shp` assigned-player/house marker renderer. It is not the map-preview control renderer used while the player is still editing the offline Skirmish setup screen. The only recovered direct caller is the `CALL 0x00640A40` at `0x00553687`, inside a scenario-start/loading-screen renderer called from `ScenarioClass__Full_Init`.

The standard offline Skirmish setup dialog `0x102` uses `DrawStartPositions @ 0x00640710` from `FUN_006AE3F0` `WM_PAINT`. That path draws the decoded preview surface, then `STARTBUT.SHP`, then numeric labels. No direct call connects `0x006AE3F0` or `0x00640710` to `0x00640A40`.

## 2. Key Offsets / Globals

| Offset / global | Purpose in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass+0x1180` | assigned start/house slot table read by `FUN_00640A40` | `0x00640A40` loop begins at offset `0x1180`; `ScenarioClass__Full_Init` clears `0x1180..0x11C0` to `-1` before non-campaign assignment setup | Conditional: non-campaign scenario start |
| `ScenarioClass+0x1134/+0x1138` | preview source width/height divisors for marker projection in `FUN_00640A40` | decompile `0x00640A40` | Conditional: only when marker pass runs |
| `g_HouseClass_Array + assigned_slot*4` | assigned house lookup | `0x00640A40` color-scheme gate | Conditional: assigned slot must not be `-1` |
| house field `+0x16054` | color-scheme index used for marker tint/convert gate | `0x00640A40` reads color scheme through this field | Conditional: assigned house exists |
| color scheme field `+0x30C` | non-null gate before drawing marker | `0x00640A40` tests it before `CC_Draw_Shape` | Conditional: must be non-null |
| `DAT_00AC1154` | offline setup dialog preview wrapper for `DrawStartPositions`, not the `mmpb` caller | `0x006AE3F0` `WM_PAINT`, `0x00640710` | Yes for setup preview; No for `mmpb` path |

## 3. Core Logic

### `FUN_00640A40`

**Active in YR:** Conditional. The function is reached from scenario-start/loading context, not from the setup dialog first-paint path.

Verified behavior:

1. Early-outs unless the caller surface/wrapper pointer has a non-null first field.
2. Walks map cells with `MapClass__CellIterator_*`, keeps cells in the playfield, projects cell centers using `cell_x * 0x100 + 0x80` and `cell_y * 0x100 + 0x80`, then derives projected bounds using `/ 0x3C` and `/ 0x1E`.
3. Counts valid start waypoints by calling `FUN_0068BD80(i)` for `i = 0..7`.
4. Draws small filled rectangles for valid start locations into the caller-provided surface before the later assigned-player pass.
5. Chooses a loading-screen map preview rectangle by screen width:
   - width equal to `DAT_007F5BE4`: x/y/size constants include `499`, `0x17B`, `0xD8`, `0xA6`;
   - width equal to `DAT_007F5BE8`: constants include `0x23A`, `0x1A8`, `300`, `0x104`;
   - fallback: constants include `0x181`, `0x10E`, `200`, `200`.
6. Allocates a temporary `DSurface`, blits the caller source into it, loads `mmpb.shp` from string `0x00836DF4`, and iterates assigned slots from `ScenarioClass+0x1180`.
7. A `mmpb.shp` marker draws only when all gates pass: start waypoint is valid, assigned slot is not `-1`, `mmpb.shp` loaded, and the assigned house color scheme has non-null field `+0x30C`.
8. The marker draw uses SHP frame `0`, draw flags including `0x400`, scale `1000`, and projected offsets `-3` X and `-2` Y.
9. The temporary surface is blitted back to the caller surface and destroyed.

### Contrast: `DrawStartPositions @ 0x00640710`

**Active in YR:** Yes for standard offline Skirmish setup dialog `0x102`.

Verified behavior:

1. Called by `FUN_006AE3F0` `WM_PAINT` at `0x006AE47B` after `DAT_00AC1154 != 0`, `GetDlgItem(hwnd, 0x468)`, and a false return from `0x006067A0`.
2. Uses child control `0x468` and `0x00775690` to get preview child coordinates.
3. Blits the preview surface to `DAT_00887310`.
4. Lazily loads `STARTBUT.SHP` from string `0x00836DE4`.
5. Draws only when `ScenarioClass+0x113C` is `1..8`.
6. Reads marker coordinates from `ScenarioClass+0x1140 + i*8` and `ScenarioClass+0x1144 + i*8`.
7. Draws `STARTBUT.SHP` frame `0` with offsets `-9` X and `-6` Y, then draws numeric label `i + 1`.

## 4. INI Keys

No new INI key reader was traced for `mmpb.shp` in this slot. The relevant state is runtime scenario/session state:

| Data | Source / prior evidence | Effect | Active in YR |
|---|---|---|---|
| `DAT_00A8B238` game mode | `SESSIONCLASS_GHIDRA_REPORT.md`; `SPAWN_POINT_ASSIGNMENT_SYSTEM.md` lists `5 = Skirmish` | Nonzero game modes take the multiplayer/skirmish branch in `ScenarioClass__Full_Init` before `0x00552D60` is called | Yes |
| `ScenarioClass+0x1180` assigned start table | `ScenarioClass__Full_Init` clears it; prior spawn assignment docs cover population | Drives whether `mmpb.shp` markers draw | Conditional: only assigned entries draw |
| Offline setup start controls | prior Skirmish start-position docs | Feed selected starts before game launch, not `mmpb` setup-dialog paint directly | Yes, but different phase |

## 5. Integration Points

**Active in YR:** Conditional for `mmpb`; Yes for `DrawStartPositions` setup preview.

Verified caller chain for `mmpb`:

| Link | Evidence | Meaning |
|---|---|---|
| `ScenarioClass__Full_Init -> 0x00552D60` | `get_function_xrefs 0x00552D60`: caller `0x00687588`; decompile shows `LoadProgressMgr__Constructor(); 0x00552D60(); FUN_0069AE90(3)` | The renderer is invoked during scenario initialization/loading, after non-campaign house/start setup and before normal progress stages continue |
| `0x00552D60 -> FUN_00640A40` | `get_function_xrefs 0x00640A40`: only recovered caller is `0x00553687`; assembly context passes `[EBP+0x60]` surface pointer | `mmpb` is a loading-screen/preview renderer subpass |
| `0x00640A40 -> mmpb.shp load` | string `mmpb.shp` at `0x00836DF4`, xref `0x00640E44` | The asset is loaded locally for assigned-player markers |

Screen/action recovered:

- **Verified:** The call happens during `ScenarioClass__Full_Init`, after non-campaign setup creates houses, computes radar/map bounds, and assigns or imports starting positions. Evidence: decompile `0x00687502` around non-campaign branch and call `0x00687588`.
- **Inference:** Player-facing timing is the match loading screen after pressing Start/launching a Skirmish or multiplayer game, not the editable setup dialog. This inference is based on the `LoadProgressMgr__Constructor` adjacency and loading-screen asset/key usage in the same function family; no runtime screenshot was captured in this slot.

Offline setup dialog separation:

| Path | Caller evidence | Marker asset | Active in standard offline Skirmish dialog `0x102` |
|---|---|---|---|
| `FUN_006AE3F0 WM_PAINT -> DrawStartPositions @ 0x00640710` | direct call at `0x006AE47B` | `STARTBUT.SHP` plus text labels | Yes |
| `0x00552D60 -> FUN_00640A40` | direct call at `0x00553687`, parent caller `ScenarioClass__Full_Init` | `mmpb.shp` assigned-player/house markers | No; active later/elsewhere |

## 6. Current Rust Implementation Status

Not scanned in this slot because the user constrained this subagent to research output only and no repo/Rust modifications. Prior docs already warn that `mmpb.shp` must not be used as a setup-dialog preview placeholder.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00640A40` primary behavior | verified | decompile `0x00640A40`; string xref `0x00836DF4 -> 0x00640E44` | exact visual screenshot not captured |
| Direct callers of `FUN_00640A40` | verified | `get_function_xrefs 0x00640A40`: only `0x00553687` | xref recovery is static; runtime frequency not measured |
| Parent caller of `0x00552D60` | verified | `get_function_xrefs 0x00552D60`: `ScenarioClass__Full_Init @ 0x00687588` | exact UI name of loading screen remains inferred |
| Offline setup dialog `0x102` contrast | verified | `0x006AE3F0` decompile and assembly call `0x006AE47B -> 0x00640710` | none for this scope |
| Assignment table population | touched-not-exhausted | `ScenarioClass__Full_Init` clears `+0x1180..+0x11C0`; prior spawn docs cover assignment | full assigner algorithm intentionally out of scope |
| `mmpb.shp` palette/tint internals | touched-not-exhausted | color scheme `+0x30C` gate in `0x00640A40` | exact color transform not investigated |
| Runtime screenshot/action label | deferred | static Ghidra only | runtime debugger or screenshot capture |

## 8. Open Questions - Final State

- [RESOLVED] OQ-1 - Who directly calls `FUN_00640A40`? Exactly one recovered direct caller, `0x00553687`. Evidence: Ghidra `get_function_xrefs 0x00640A40`.
- [RESOLVED] OQ-2 - Is `0x00553687` inside the offline Skirmish dialog `0x102` proc? No. It is inside `0x00552D60`, whose recovered parent caller is `ScenarioClass__Full_Init @ 0x00687588`. Evidence: Ghidra xrefs and decompiles.
- [RESOLVED] OQ-3 - Does standard offline Skirmish setup `WM_PAINT` call `mmpb`? No. `FUN_006AE3F0` calls `DrawStartPositions @ 0x00640710` at `0x006AE47B`; no xref from `0x006AE3F0`/`0x00640710` to `0x00640A40` was found. Evidence: Ghidra decompile and xrefs.
- [RESOLVED] OQ-4 - What gates `mmpb.shp` drawing? Valid start index, assigned slot not `-1`, non-null SHP load, and non-null assigned house color scheme field `+0x30C`. Evidence: decompile `0x00640A40`.
- [RESOLVED] OQ-5 - Active in YR? Conditional. Non-campaign scenario initialization calls `0x00552D60`; Skirmish is game mode `5` per prior SessionClass docs. It is inactive for setup dialog `0x102` first-paint. Evidence: `ScenarioClass__Full_Init`, prior `SESSIONCLASS_GHIDRA_REPORT.md`.
- [DEFERRED] OQ-6 - Exact player-facing loading-screen name/screenshot. Category: needs-runtime-debugger. Static evidence shows scenario-start/loading context, but no screenshot was captured.

## Sources

- Ghidra read-only decompile/xref:
  - `FUN_00640A40`
  - `0x00552D60`
  - `ScenarioClass__Full_Init @ 0x00687502`
  - `FUN_006AE3F0`
  - `DrawStartPositions @ 0x00640710`
  - `get_function_xrefs 0x00640A40`
  - `get_function_xrefs 0x00552D60`
  - string xrefs `mmpb.shp @ 0x00836DF4`, `STARTBUT.SHP @ 0x00836DE4`
- Prior docs:
  - `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
  - `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`
  - `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
  - `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
  - `SESSIONCLASS_GHIDRA_REPORT.md`
  - `SPAWN_POINT_ASSIGNMENT_SYSTEM.md`
