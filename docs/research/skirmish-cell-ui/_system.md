# Skirmish Cell UI — System Synthesis

**System:** Offline Skirmish dialog `0x102` per-player row "cells" subsystem
**Status:** Decoded, proofed, and parity-compared as of 2026-05-24
**Output of:** `/decode-system skirmish-cell-ui` (team `decode-skirmish-cell-ui`)

## Summary

The skirmish-cell-ui subsystem renders and manages the 8 player-slot rows in the
offline Skirmish setup dialog (`0x102`). Slot 0 is the local human player; slots
1–7 are AI/opponent rows. Each row owns six interacting cell controls (player
name edit on slot 0 only, picture-flag static, country combo, color combo,
AI-type combo on slots 1–7 only, team combo, start-position combo) plus shared
infrastructure: tooltip dispatcher, owner-draw combo paint, session restore from
INI, and per-row enable state machine.

The observable behavior is: the player opens Skirmish from the main menu →
dialog `0x102` appears with all rows populated from prior-session state → player
clicks/types in cells to change selections → cells cascade enable/disable +
sentinel writes through their row → Start button validates and launches the
game.

This synthesis covers 63 decoded symbols (48 functions, 5 STT strings, 2
structs, 3 globals, 2 Phase 0b helpers) plus a parity report against the current
Rust skirmish shell.

## Symbol scope

| Kind | Count | Examples |
|---|---|---|
| Functions | 48 | dialog launcher, DlgProc, init/cmd/row-enable anchors, 30+ per-cell helpers, session writers, tooltip dispatcher, owner-draw WndProc |
| STT strings | 5 | `SkirmishComboAIPlayer`, `SkirmishPictureFlag`, `SkirmishComboCountry`, `SkirmishComboColor`, `SkirmishEditPlayer` |
| Structs | 2 | `SessionClass-Slots-Slice` (per-slot data), `ColorTableEntry` (9-row color table) |
| Globals | 3 | `DAT_00A8B274` (active_ai_count), `DAT_00A8B23C` (selected-mode pointer), color table base |
| Phase 0b additions | 2 | `FUN_005e2ef0` (control 0x6EC custom msg), `FUN_005e2f60` (control 0x5A8 custom msg) |
| **Total** | **63 in scope** | (filtered from initial 65 manifest + 2 Phase 0b additions) |

Excluded by TS-filter or relevance: 9 functions (CRT/IO shared utilities, low-relevance helpers) and 14 STT strings (global dialog settings outside per-row cell scope). 0 TS-legacy drops — dialog `0x102` is YR-introduced.

## Control flow — dialog 0x102 message routing

```
Main Menu → Skirmish button
    │
    ▼
FUN_006AE2C0  (Skirmish launcher)
    ├─ Calls FUN_006722F0 + FUN_00672440 + per-house vtable+0x64
    │   to populate house-type data  [Rust gap: missing pre-dialog setup]
    ├─ CreateDialogIndirectParamA(0x102, FUN_006AE3F0)
    └─ Runs message pump until Start (returns true) or Back (returns false)
        │
        ▼
    FUN_006AE3F0  (DlgProc — Win32 dialog callback)
        ├─ 0x497 (custom WM_INITDIALOG) → FUN_006AE6E0
        ├─ WM_COMMAND                    → FUN_006ACEE0
        ├─ WM_PAINT                      → DrawStartPositions (0x00640710)
        │                                   on control 0x468 (mini-map) [Rust gap]
        ├─ WM_NOTIFY (tooltip + hover)   → FUN_006040B0 (STT dispatcher)
        ├─ WM_NOTIFY (AI-type tooltip)   → strings 0x87/0x89/0x8B/0x8D
        └─ default                       → DefDlgProcA
            │
            ▼
        FUN_006AE6E0  (Custom init — 0x497)  [ANCHOR 1]
            ├─ Phase 1: FUN_0069B540 — clear session slot arrays (+0x4C, +0x6C → -1)
            ├─ Phase 2: Populate 7 AI-type combos (slots 1-7)
            │   CB_RESETCONTENT + 4× CB_SETITEMDATA per slot
            │   item-data: -1 (None), 0 (Hard), 1 (Normal), 2 (Easy)
            ├─ Phase 3: Per-cell combo population (per-row helpers)
            │   FUN_004E3B90 (country)  + FUN_004E3CE0 (country flag)
            │   FUN_004E43C0 (color labels) + FUN_004E4820 (color combo)
            │   FUN_004E4F50 (start labels) + FUN_004E5310 (start combo)
            │   FUN_004E5AC0 (team labels) + FUN_004E5D60 (team combo)
            ├─ Phase 4: Restore AI-type from DAT_00A8B3F0 (7×3-dword session array)
            │   type-code 1→-1 (closed), 4→0, 5→1, 6→2; other→-1 + EnableWindow(FALSE)
            ├─ Phase 5: Map-selection init (selected scenario / mode)
            ├─ Phase 6: Game-option checkboxes / sliders init
            ├─ Phase 7: Team-sentinel propagation across all 7 rows
            │   item-data 3 if AlliesAllowed=true, -2 if false
            ├─ Phase 8: Row visibility (calls FUN_006ADDF0 for map start count)
            └─ Phase 9: Selected-mode pointer setup (DAT_00A8B23C)
            │
        FUN_006ACEE0  (WM_COMMAND dispatcher)  [ANCHOR 2]
            ├─ Switch on control ID (LOWORD of wParam):
            │   ├─ 0x50B/0x50E/0x516/0x51A-0x51D (AI-type) → FUN_006ADC20
            │   ├─ 0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521 (country)
            │   │   → FUN_004E3690 (selection) → FUN_0069B760 (session write)
            │   ├─ 0x6A2/0x522-0x528 (color)
            │   │   → FUN_004E4C20 (ownership refresh) → FUN_0069B7E0 (write)
            │   ├─ 0x6A3-0x6A8/0x6AA/0x6AB (start position)
            │   │   → FUN_004E5700 (selection + claim)
            │   ├─ 0x76D-0x774 (team) → FUN_004E6030 / FUN_004E5ED0
            │   ├─ 0x5AA (map selector) → FUN_005E6520 + FUN_006ADDF0 [Rust gap]
            │   ├─ 0x617 (Start) → 4 validation gates + ProcessRandomAssignments
            │   └─ 0x5C0 (Back) → close dialog
            └─ After per-cell handler: refresh dependent state
                ├─ FUN_006ACD60 (team enable refresh per AlliesAllowed)
                └─ FUN_006ADDF0 (row show/hide on map change)
            │
        FUN_006ADC20  (Per-row enable state machine)  [ANCHOR 3]
            ├─ Reads AI-type item-data (CB_GETCURSEL + CB_GETITEMDATA)
            ├─ if item-data in {0, 1, 2} (active AI):
            │   EnableWindow(country, color, start, team controls, TRUE)
            └─ else (item-data == -1 or other):
                FUN_004E3F70(row, -2)  — country combo sentinel
                FUN_004E49A0(row, -2)  — color combo sentinel
                FUN_004E5480(row, -2)  — start-position sentinel
                FUN_004E5ED0(row, 3 or -2 per AlliesAllowed) — team sentinel
                EnableWindow(country, color, start, team, FALSE)
```

## Per-row state machine

Each of the 8 rows has these cell states:

```
                   ┌──────────────────────────────────────────────┐
                   │  ACTIVE                                       │
                   │  AI-type ∈ {Hard(0), Normal(1), Easy(2)}      │
                   │  Country, color, start, team: enabled,        │
                   │  populated with restored or default values    │
                   └──────────────────────────────────────────────┘
                              ▲             │
                              │             │
                              │ AI-type     │ AI-type changed
                              │ changed to  │ to None or row hidden
                              │ active      │
                              │             ▼
                   ┌──────────────────────────────────────────────┐
                   │  INACTIVE (sentinel -2 in all cells)          │
                   │  AI-type == -1 (None)                         │
                   │  Country, color, start: combos disabled       │
                   │   + item-data forced to -2                    │
                   │  Team: combo disabled + item-data forced to   │
                   │   3 (if AlliesAllowed) or -2 (otherwise)      │
                   └──────────────────────────────────────────────┘
                              ▲             │
                              │             │
                              │ Row revealed│ Row hidden via
                              │ via map     │ FUN_006AE080 on
                              │ change      │ map start-count
                              │ (006ADF00)  │ shrink
                              │             ▼
                   ┌──────────────────────────────────────────────┐
                   │  HIDDEN (ShowWindow(FALSE))                   │
                   │  Row controls fully hidden from dialog        │
                   │  Map's start-slot count < row index           │
                   │  (slot 0 never hidden)                        │
                   └──────────────────────────────────────────────┘
```

### Sentinel `-2` propagation

When a row goes INACTIVE (AI-type → None), four sentinel writers fire in
sequence to ensure cell state is consistent:

1. `FUN_004E3F70(row, -2)` — country combo: scan items, set CB_SETCURSEL to
   item with data == -2.
2. `FUN_004E49A0(row, -2)` — color combo: clear prior ownership in
   `DAT_008B4040` color table; do NOT set new ownership when value is -2.
   Refresh all 8 color combos (so freed color becomes available to others).
3. `FUN_004E5480(row, -2)` — start-position combo: clear prior claim in
   `DAT_008B3F38` start-pos table; do not set new claim when value is -2.
4. `FUN_004E5ED0(row, 3 or -2)` — team combo: 3 if `*(DAT_00A8B23C+0x3C) != 0`
   (AlliesAllowed), -2 otherwise.

The sentinel value -2 maps to a "Random / Unspecified" item with no row
ownership. INACTIVE rows are visible but disabled — they show the sentinel
selection.

### AlliesAllowed team gate

`DAT_00A8B23C + 0x3C` (a byte) is the selected-mode `AlliesAllowed` flag.
The team controls follow this rule:

- Local row (slot 0): team `0x76D` enabled only if `AlliesAllowed != 0`.
- AI rows: team enabled if `AlliesAllowed != 0` AND row is ACTIVE.

If `AlliesAllowed == 0`, all 8 team controls are disabled via `EnableWindow(FALSE)`. This is enforced at:
- Dialog init (Phase 7 of `FUN_006AE6E0`)
- AI-type change handler (`FUN_006ACD60` chain)
- Mode change (re-read of `DAT_00A8B23C`)

## INI surface

The Skirmish subsystem reads from and writes to `RA2MD.INI`:

| INI section | Keys | Effect |
|---|---|---|
| `[Skirmish]` | `GameMode`, `ScenIndex`, `GameSpeed`, `Credits`, `UnitCount` | Restore prior session globals |
| `[Skirmish]` | `IsCampaignSelected`, `IsBuildOffAlly`, `CratesAppear`, `ShortGame`, `Redeploys`, `SWAllowed` | Booleans for game-option checkboxes |
| `[Skirmish]` | `Slot00`..`Slot07` | Per-slot triple `type,country,color` (parsed by `FUN_00477440`) |

Reader: `SessionClass__ReadSkirmishSettings @ 0x00697F10`. Triples are parsed
via `FUN_00477440` (3-int tokenizer). Writer: `FUN_006ACEE0`'s Start path writes
back to globals which are persisted by code outside this subsystem's scope.

**Defaults** (when key absent in stock `RA2MD.INI`):
- `Slot01 = 4,-1,-1` (active AI, random country, random color) — Skirmish always has at least one AI even though `[MultiplayerDialogSettings] AIPlayers=0`.
- `Slot02`..`Slot07 = 1,-1,-1` (closed/None).
- All booleans default `false`; sliders default mid-range.

## Observable behaviors

What the player sees in the offline Skirmish dialog:

1. **Cell paint**: each combo uses `OwnerDraw_ComboBox_00617250` for collapsed-state painting (swatch + truncated text), and creates a `ComboDropWin` popup for the dropdown list. Player-name edit (slot 0) is a standard Win32 edit; flag static (`0x6DA..0x6E1`) is a static control owner-drawn with a PCX flag image.
2. **Row enable cascade**: changing AI-type for an AI row immediately enables/disables its country/color/start/team cells. Sentinel `-2` is written to disabled cells.
3. **Row visibility cascade**: changing the selected map immediately shows/hides AI rows beyond the new start-slot count. Slot 0 never hides.
4. **Color ownership**: color combos enforce uniqueness — a color chosen by one row is removed from other rows' combos. The 9-row color table at `0x008B4034` tracks ownership.
5. **Start-position ownership**: similar to color — a start position chosen by one row is removed from others.
6. **Validation on Start**: 4 gates (map capacity, ≥2 players, team conflict auto-repair, AlliesAllowed). Failure shows a StringTable error message.
7. **Random resolution on Start**: any `-2` (random) country/color is silently resolved to a concrete value via `SessionClass__ProcessRandomAssignments`.
8. **Tooltip on hover**: STT keys (`STT:SkirmishComboAIPlayer`, `STT:SkirmishPictureFlag`, etc.) dispatched by `FUN_006040B0` based on control ID.

## Edge cases / known parity hazards

1. **Sentinel `-2` propagation** — must fire across ALL four cell types when row goes inactive. Skipping any one leaves stale state. (Rust gap: sentinel writes are mostly absent; combo display happens to be correct because Rust drives from typed state.)
2. **AlliesAllowed team gate** — must read `DAT_00A8B23C + 0x3C` BYTE (not WORD or DWORD). Frame: byte offset on dereferenced selected-mode pointer.
3. **CDFileClass__Constructor mislabel** at `0x005E6520` — Ghidra's RTTI labeler mistakenly named the selected-map start-count function as `CDFileClass__Constructor`. The function actually opens a map file, counts `[Waypoints]` keys 0-7, falls back to `[RandomMap] NumPlayers`, default 8. Do NOT treat the label as authoritative.
4. **ColorTableEntry layout — corrected by proofer-2 audit (PROOFED-YELLOW 90)** — the original decode of `struct-colortableentry.md` (#66) claimed base `0x008B4034` with field order `{swatch_rgb, flags, label_ptr}`. The proofer's verification against `FUN_004E43C0` showed: **actual base is `0x008B4038`** and **actual field order is `{+0x00 label_ptr, +0x04 swatch_rgb, +0x08 flags}`** — the decoder had the field order inverted. Stride (12 bytes), entry count (9), and string IDs (`0x1DB`..`0x1E3`) are confirmed correct. **Use the corrected layout for any Rust port — do NOT use the per-symbol doc's verified-body table; the doc's YELLOW section correctly flags this ambiguity.**
5. **DlgProc registration** — `FUN_006AE3F0` has NO Ghidra-found callers because it's registered via `SetWindowLongA` from `FUN_006AE2C0` (normal for WndProc-style functions). Trace via callers OF `FUN_006AE2C0` to find the entry point.
6. **AI-type code mapping** — persisted slot type codes are NOT the same as combo item-data:
   - Persisted: 1=closed, 4=Hard, 5=Normal, 6=Easy, other=closed
   - Combo item-data: -1=None, 0=Hard, 1=Normal, 2=Easy
   Translation happens in `FUN_006AE6E0` Phase 4.
7. **Map-selector cascade** — changing the map MUST trigger `FUN_006ADDF0` to recompute row visibility AND propagate team sentinels across all 7 rows (in case AlliesAllowed changed). Rust gap: cascade absent.
8. **Spectator/observer mode** — gamemd has spectator/observer code paths in EVERY per-row helper (FUN_004E3B90/4770/53D0/5E20 etc.). In standard offline skirmish (`g_GameMode != 3 && != 4`), these paths are dormant. Rust does not implement them — fine for standard skirmish, gap if observer mode is added.
9. **Country/color random resolution timing** — gamemd resolves `-2` (random) at Start via `SessionClass__ProcessRandomAssignments`. Rust currently throws a `LaunchValidationError::RandomSelectionUnverified` if any random selection remains at Start. Player-visible drift.
10. **Coordinate frame note** — none of the cell-UI work involves game-world coords. All references are dialog control IDs (e.g. `0x6A1` = local country combo) or static memory addresses (e.g. `DAT_00A8B3F0` = persisted slot type array). No leptons-vs-cells confusion to worry about here.

## Parity headline — Rust gaps ranked by player-visibility

Per CLAUDE.md parity bar (observable output is the spec). Ranked by visibility×frequency.

### Most player-visible (MISSING / DRIFT, every match)

| Finding | Verdict | Visibility |
|---|---|---|
| **Map-selector cascade** — Rust does NOT show/hide AI rows when the map changes | MISSING | Every map change; rows for non-existent start slots stay visible |
| **Per-row ShowWindow visibility** — Rust always renders all 7 AI rows regardless of map capacity | MISSING | Compound of map-selector cascade |
| **Color combo ownership** — Rust lets two players pick the same color; gamemd enforces uniqueness via 9-row ownership table | DRIFT | Every game; player-visible color collision |
| **Random resolution on Start** — Rust rejects Start with `RandomSelectionUnverified` error; gamemd silently resolves random | DRIFT | Every "Random" picked → cannot Start in Rust |
| **AlliesAllowed team gate** — Rust keeps team combos enabled even when AlliesAllowed=false | DRIFT | Every match with non-allied mode selected |
| **Session restore from RA2MD.INI `[Skirmish]`** — Rust always starts from defaults; gamemd restores prior session | MISSING | Every dialog re-open; player loses settings |
| **Team conflict auto-repair on Start** — gamemd loops to repair conflicts silently; Rust shows error immediately | DRIFT | Frequent in 1v3+ skirmishes |
| **Gate 4 AlliesAllowed validation** — Rust never blocks Start on AlliesAllowed mode mismatch | MISSING | Every non-allies mode picked |
| **Flag-static tooltip** — Rust shows no tooltip when hovering country flag image | MISSING | Frequent hover, low pain |

### Less visible (MISSING but rarely triggered)

| Finding | Verdict | Visibility |
|---|---|---|
| **Country/color session-restore** — Rust resets to defaults per slot | MISSING | Compound of session-restore |
| **Start-position markers on map preview** — Rust doesn't draw numbered start dots on mini-map | MISSING | Frequent but minor visual |
| **House-type setup before dialog** — pre-dialog vtable+0x64 call missing in Rust | MISSING | Latent; effect depends on house-data dynamic loading |
| **Spectator/observer mode paths** — entire branch missing | MISSING | Standard skirmish doesn't trigger; spectator support gap |

### Internals identical / not a finding

50+ `INTERNAL-ONLY` rows where Rust achieves the same observable output via a different mechanism (state-driven render vs Win32 SendMessage, etc.). Per CLAUDE.md parity bar these are NOT findings — they're informational only.

### MATCH

`FUN_004E3560` (side item-data → flag PCX filename mapping) is a clean MATCH. Rust's `flag_pcx_for_side_item_data` mirrors the gamemd table exactly.

## Per-symbol doc index

Dialog-level (orchestration):
- [fn-fun-006ae2c0-launcher.md](fn-fun-006ae2c0-launcher.md) — Skirmish dialog launcher
- [fn-fun-006ae3f0-dlgproc.md](fn-fun-006ae3f0-dlgproc.md) — DlgProc message router
- [fn-006ae6e0-init.md](fn-006ae6e0-init.md) — Anchor 1: dialog init / row population
- [fn-006acee0-cmd.md](fn-006acee0-cmd.md) — Anchor 2: WM_COMMAND dispatcher
- [fn-006adc20-row-enable.md](fn-006adc20-row-enable.md) — Anchor 3: per-row enable state machine
- [fn-fun-006addf0-row-showhide.md](fn-fun-006addf0-row-showhide.md) — Row visibility on map change
- [fn-fun-006adf00-reveal-ai-rows.md](fn-fun-006adf00-reveal-ai-rows.md) — Reveal AI rows
- [fn-fun-006ae080-hide-ai-rows.md](fn-fun-006ae080-hide-ai-rows.md) — Hide AI rows
- [fn-fun-006acd60-team-enable.md](fn-fun-006acd60-team-enable.md) — Team enable refresh

Per-cell helpers (slot 0 flag + 30 cell helpers):
- [fn-fun-004e3320-slot0-flag.md](fn-fun-004e3320-slot0-flag.md) — Slot 0 flag control-ID lookup
- [fn-fun-004e3560-side-flag-lookup.md](fn-fun-004e3560-side-flag-lookup.md) — Side→flag PCX mapping (MATCH)
- [fn-fun-004e3690-cell-cmd-handler.md](fn-fun-004e3690-cell-cmd-handler.md) — Country CBN_SELCHANGE
- [fn-fun-004e37d0-row-helper.md](fn-fun-004e37d0-row-helper.md) — Country combo ID lookup
- [fn-fun-004e3830-cell-cmd-handler-b.md](fn-fun-004e3830-cell-cmd-handler-b.md) — Country ID reverse lookup
- [fn-fun-004e3b90-country-flag-helper.md](fn-fun-004e3b90-country-flag-helper.md) — Country combo loader
- [fn-fun-004e3ce0-country-flag-helper-b.md](fn-fun-004e3ce0-country-flag-helper-b.md) — Country session restore
- [fn-fun-004e3f70-country-sentinel.md](fn-fun-004e3f70-country-sentinel.md) — Country sentinel writer
- [fn-fun-004e4170-cell-cmd-handler-c.md](fn-fun-004e4170-cell-cmd-handler-c.md) — Country item-data reader
- [fn-fun-004e41d0-row-helper-b.md](fn-fun-004e41d0-row-helper-b.md) — Color combo ID lookup
- [fn-fun-004e43c0-color-label-loader.md](fn-fun-004e43c0-color-label-loader.md) — 9-row color label loader
- [fn-fun-004e45a0-color-helper.md](fn-fun-004e45a0-color-helper.md) — Color combo population (filtered)
- [fn-fun-004e4770-color-helper-b.md](fn-fun-004e4770-color-helper-b.md) — Color combo sentinel loader
- [fn-fun-004e4820-color-combo-helper.md](fn-fun-004e4820-color-combo-helper.md) — Color init loop
- [fn-fun-004e48e0-color-combo-helper-b.md](fn-fun-004e48e0-color-combo-helper-b.md) — Color session restore
- [fn-fun-004e49a0-color-sentinel.md](fn-fun-004e49a0-color-sentinel.md) — Color ownership + refresh
- [fn-fun-004e4c20-color-selection.md](fn-fun-004e4c20-color-selection.md) — Color CBN_SELCHANGE
- [fn-fun-004e4e20-cell-cmd-handler-d.md](fn-fun-004e4e20-cell-cmd-handler-d.md) — Color item-data reader
- [fn-fun-004e4e60-row-helper-c.md](fn-fun-004e4e60-row-helper-c.md) — Start-pos combo ID lookup
- [fn-fun-004e4f50-color-helper-c.md](fn-fun-004e4f50-color-helper-c.md) — Start-pos label table init
- [fn-fun-004e4fc0-color-helper-d.md](fn-fun-004e4fc0-color-helper-d.md) — Start-pos availability mask
- [fn-fun-004e5310-startpos-helper.md](fn-fun-004e5310-startpos-helper.md) — Start-pos population loop
- [fn-fun-004e53d0-startpos-helper-b.md](fn-fun-004e53d0-startpos-helper-b.md) — Start-pos spectator dispatcher
- [fn-fun-004e5480-startpos-sentinel.md](fn-fun-004e5480-startpos-sentinel.md) — Start-pos selection + claim
- [fn-fun-004e5700-start-handler.md](fn-fun-004e5700-start-handler.md) — Start-pos CBN_SELCHANGE
- [fn-fun-004e5900-start-handler-b.md](fn-fun-004e5900-start-handler-b.md) — Start-pos item-data reader
- [fn-fun-004e5940-row-helper-d.md](fn-fun-004e5940-row-helper-d.md) — Team combo ID lookup
- [fn-fun-004e5ac0-team-combo-helper.md](fn-fun-004e5ac0-team-combo-helper.md) — Team labels
- [fn-fun-004e5d60-team-combo-helper-b.md](fn-fun-004e5d60-team-combo-helper-b.md) — Team population loop
- [fn-fun-004e5e20-team-combo-helper-c.md](fn-fun-004e5e20-team-combo-helper-c.md) — Team sentinel dispatcher
- [fn-fun-004e5ed0-team-sentinel.md](fn-fun-004e5ed0-team-sentinel.md) — Team selection setter
- [fn-fun-004e6030-cell-cmd-handler-e.md](fn-fun-004e6030-cell-cmd-handler-e.md) — Team item-data reader

Shared infrastructure:
- [fn-ownerdraw-combobox-00617250.md](fn-ownerdraw-combobox-00617250.md) — Owner-draw combo WndProc
- [fn-fun-006040b0-tooltip-dispatcher.md](fn-fun-006040b0-tooltip-dispatcher.md) — STT tooltip dispatcher

Session persistence:
- [fn-sessionclass-readskirmishsettings.md](fn-sessionclass-readskirmishsettings.md) — INI restore
- [fn-fun-00477440-slot-string-parser.md](fn-fun-00477440-slot-string-parser.md) — 3-int slot tokenizer
- [fn-fun-0069adf0-session-helper-init.md](fn-fun-0069adf0-session-helper-init.md) — RandMap_Sed validator
- [fn-fun-0069b540-session-helper-init-b.md](fn-fun-0069b540-session-helper-init-b.md) — Slot array init
- [fn-fun-0069b760-session-writer-cmd.md](fn-fun-0069b760-session-writer-cmd.md) — Country writer
- [fn-fun-0069b7e0-session-writer-cmd-b.md](fn-fun-0069b7e0-session-writer-cmd-b.md) — Color writer
- [fn-sessionclass-processrandomassignments.md](fn-sessionclass-processrandomassignments.md) — Random resolution on Start

Map / scenario:
- [fn-fun-005e6520-map-start-count.md](fn-fun-005e6520-map-start-count.md) — Selected-map start count (Ghidra-mislabeled as `CDFileClass__Constructor`)

Dialog helpers (Phase 0b additions):
- [fn-fun-005e2ef0-dialog-helper.md](fn-fun-005e2ef0-dialog-helper.md) — Control 0x6EC custom msg sender
- [fn-fun-005e2f60-dialog-helper-b.md](fn-fun-005e2f60-dialog-helper-b.md) — Control 0x5A8 custom msg sender

STT strings:
- [string-stt-skirmishcomboaiplayer.md](string-stt-skirmishcomboaiplayer.md)
- [string-stt-skirmishpictureflag.md](string-stt-skirmishpictureflag.md)
- [string-stt-skirmishcombocountry.md](string-stt-skirmishcombocountry.md)
- [string-stt-skirmishcombocolor.md](string-stt-skirmishcombocolor.md)
- [string-stt-skirmisheditplayer.md](string-stt-skirmisheditplayer.md)

Structs and globals:
- [struct-sessionclass-slots-slice.md](struct-sessionclass-slots-slice.md) — Per-slot data layout
- [struct-colortableentry.md](struct-colortableentry.md) — 9-row color table (base address claim UNVERIFIED — YELLOW)
- [global-dat-00a8b274-active-ai-count.md](global-dat-00a8b274-active-ai-count.md) — Active AI count
- [global-selectedmode-alliesallowed-ptr.md](global-selectedmode-alliesallowed-ptr.md) — Selected-mode + AlliesAllowed
- [global-colortablebase.md](global-colortablebase.md) — Color table base reference

Plus: [_parity.md](_parity.md) — full 89-row Rust-vs-gamemd parity report.

## References

All per-symbol docs cite their Ghidra MCP calls inline. The top-level addresses
are:

- **Dialog `0x102` launcher**: `0x006AE2C0`
- **DlgProc**: `0x006AE3F0` (registered via `SetWindowLongA` from `0x006AE2C0`)
- **Init handler** (msg `0x497`): `0x006AE6E0` (anchor 1)
- **WM_COMMAND dispatcher**: `0x006ACEE0` (anchor 2)
- **Per-row enable**: `0x006ADC20` (anchor 3)
- **Owner-draw combo WndProc**: `0x00617250`
- **Tooltip dispatcher**: `0x006040B0`
- **Session INI restore**: `0x00697F10` (`SessionClass__ReadSkirmishSettings`)
- **Random resolution**: `0x0069B8C0` (`SessionClass__ProcessRandomAssignments`)
- **Color table base** (UNVERIFIED): `0x008B4034`
- **Slot-type persistence array**: `0x00A8B3F0`
- **Active AI count**: `0x00A8B274`
- **Selected-mode pointer**: `0x00A8B23C` (+0x3C byte = AlliesAllowed)

Existing related research in [`../skirmish-ui/`](../skirmish-ui/) covers the
broader skirmish UI shell (background paint, layout matrix, button assembly,
map preview, validation modal). This `_system.md` focuses specifically on the
per-row cell subsystem and complements that earlier work.

## For downstream `/brainstorm` / `/write-plan`

The headline gaps are concentrated in three areas:

1. **Session persistence** — entire `[Skirmish]` INI restore is missing. High player-visibility (settings reset every dialog open). Self-contained refactor: add a `SkirmishSessionRestore` step on shell open that reads `RA2MD.INI`.
2. **Map-selector cascade** — Rust does not propagate map change through row visibility / team-sentinel / start-pos availability. This is one focused chain: hook `SelectMap` action to call the equivalent of `FUN_006ADDF0`.
3. **Color combo ownership** — replace the 8-color cycle with a 9-row ownership table; color picks across rows mutually exclude. Player-visible every match.

The four DRIFT findings (team conflict auto-repair, color ownership, AlliesAllowed gate, random resolution timing) should all be on the immediate-attention list. The 50+ INTERNAL-ONLY rows are informational and require no Rust changes.

**Note on `_system.md` confidence:** all 63 per-symbol docs were independently verified by a proofer (PROOFED at score ≥90 in 61 cases, PROOFED-YELLOW in 1 case for the color table base address). The 1 manual override on `#82` (proof of `FUN_004e3ce0`) was a redundant re-proof after a decoder re-fix; the underlying claims were spot-checked by team-lead via live Ghidra MCP. `_parity.md` has 85 rows produced by the rust-comparer based on PROOFED decode docs.
