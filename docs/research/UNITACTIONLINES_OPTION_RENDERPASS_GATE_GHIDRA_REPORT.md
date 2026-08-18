# UnitActionLines Option and Render-Pass Gate - Ghidra Research Report

**Address(es):** `0x005FA350`, `0x005FA620`, `0x005FAD10`, `0x004E1DE0`, `0x0055FAA0`, `0x0070D180`, `0x006D3D10`, `0x006DBE20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `UnitActionLines` default/read/write/apply flow, `DAT_00843108` sync, and `TacticalClass_Draw` pass-2 call order around selected action lines, radar action lines, CaptureManager links, and service/tether lines.
**Non-Scope:** endpoint math inside `TechnoClass::DrawActionLines`, low-level line rasterization, radar-line visual style, CaptureManager link geometry, service/tether line geometry, and UI text ownership beyond control IDs that read/write this option.
**Confidence:** High for the claimed slice.
**Active in YR:** Yes. The option is initialized by `OptionsClass__SetDefaults`, read/written by the live OptionsClass INI path, applied by both launcher and in-game options handlers, and consumed by the live tactical draw pass.

## 1. Overview

`UnitActionLines` is the player-facing target/action-line option. Binary evidence shows the option byte lives at `OptionsClass + 0x1E` (`DAT_00A8EB7E` in the global instance), defaults to enabled, is read from `[Options] UnitActionLines`, is written back to the INI, and is copied into `DAT_00843108` through `TechnoClass__SetDrawHealthBarsFlag`.

Selected-unit action lines are drawn in `TacticalClass_Draw` during pass `param_3 == 2` or `3`, after `Tactical__DrawUnitActionVisuals`, garrison/bandbox/placement/radar overlay setup, and before CaptureManager link drawing and service/tether lines for the same techno. Enemy/non-human radar action lines are in the same techno loop but use a different branch and do not read `DAT_00843108`.

## 2. Key Offsets and Globals

| Address / offset | Type | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| `OptionsClass + 0x1E` / `DAT_00A8EB7E` | byte bool | `UnitActionLines` option value | Yes | `OptionsClass__SetDefaults @ 0x005FA350`; `OptionsClass__ReadFromINI @ 0x005FA620`; apply handlers `0x004E1DE0`, `0x0055FAA0` |
| `0x008331C8` | string | `"UnitActionLines"` key name | Yes | `search_strings`: one match; xrefs from `0x005FA80E` and `0x005FAE08` |
| `0x00843108` | byte bool | draw-action-lines gate mirrored from `UnitActionLines` | Yes | xrefs: one write from `0x0070D180`, one read from `0x006D473F` |
| techno `+0x83` | byte bool | selected-state gate for selected action lines | Yes | read at `0x006D4735` before vtable `+0x438` call |
| techno `+0x81` | byte bool | cloaked/hidden-style gate for radar action lines | Conditional | read at `0x006D4782`; nonzero skips `DrawRadarActionLines` |
| techno `+0x2BC` | pointer | CaptureManager pointer | Conditional | read at `0x006D47A6` and `0x006D47B9` before `CaptureManagerClass__DrawLinks` |
| techno `+0x294` | pointer | service/tether state owner checked after CaptureManager links | Conditional | read at `0x006D47FB`; line draw ends at `FUN_00705860 @ 0x006D48F1` |

## 3. Option Flow

### 3.1 Default

`OptionsClass__SetDefaults @ 0x005FA350` writes `1` to direct byte offset `0x1E`.

Active in YR: Yes. This is the OptionsClass default path and the same object is later read by the INI/apply flows.

Evidence: decompile `0x005FA350` shows `*(undefined1 *)((int)param_1 + 0x1e) = 1`.

### 3.2 INI Read

`OptionsClass__ReadFromINI @ 0x005FA620` calls `CCINIClass__ReadBool` with section string `[Options]`, key string `UnitActionLines`, and current byte `param_1 + 0x1E` as the default. It stores the result back to direct offset `0x1E`.

At function end, it explicitly syncs the draw gate: `MOV CL, byte ptr [ESI + 0x1E]` at `0x005FACFA`, then `CALL 0x0070D180` at `0x005FACFD`.

Active in YR: Yes. This is the live OptionsClass read function and has a direct key-string xref.

Evidence: decompile `0x005FA620`; assembly context at `0x005FACFA-0x005FACFD`; `UnitActionLines` string at `0x008331C8`.

### 3.3 INI Write

`OptionsClass__WriteToINI @ 0x005FAD10` writes the byte at direct offset `0x1E` back through the bool-write helper with the same `[Options] UnitActionLines` strings.

Active in YR: Yes. It is the paired OptionsClass write path.

Evidence: decompile `0x005FAD10`; string xref at `0x005FAE08`.

### 3.4 Apply From Dialogs

Both option apply handlers use control `0x601` as the `UnitActionLines` checkbox:

| Function | Control | State write | Gate sync | Active in YR |
|---|---:|---|---|---|
| `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` | `0x601` | `DAT_00A8EB7E = SendMessage(..., 0xF0) == 1` | calls `0x0070D180` at `0x004E1F41` | Yes |
| `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0` | `0x601` | `DAT_00A8EB7E = SendMessage(..., 0xF0) == 1` | calls `0x0070D180` at `0x0055FB2B` | Yes |

Dialog initialization also pushes `0x601`, reads `DAT_00A8EB7E`, converts nonzero to checked state, and sends checkbox message `0xF1`. Verified at launcher init `0x005602B4-0x005602D6` and in-game init `0x004E2195-0x004E21B7`.

Active in YR: Yes. These handlers are the visible launcher and in-game options apply paths.

Evidence: decompile `0x004E1DE0`, `0x0055FAA0`; assembly context at `0x004E1F3B`, `0x0055FB25`, `0x005602B4`, `0x004E2195`.

### 3.5 `DAT_00843108` Sync

`TechnoClass__SetDrawHealthBarsFlag @ 0x0070D180` contains only one behavioral write: `DAT_00843108 = param_1`.

All discovered xrefs to `0x00843108` in this slice:

| Access | Address | Meaning | Active in YR |
|---|---:|---|---|
| write | `0x0070D180` | mirror input byte to draw gate | Yes |
| read | `0x006D473F` | selected action-line draw gate | Yes, during tactical pass 2/3 |

All discovered xrefs to `0x0070D180` feed `CL` from `OptionsClass + 0x1E` before the call/jump (`0x005FACFD`, `0x005FB2CC`, `0x004E1F41`, `0x0055FB2B`).

Active in YR: Yes. This is not TS-only: the read is inside `TacticalClass_Draw`, and the writes are live options paths.

Evidence: decompile `0x0070D180`; `get_bulk_xrefs` for `0x00843108` and `0x0070D180`; assembly contexts at the four caller sites.

## 4. Tactical Draw Pass Order

`TacticalClass_Draw @ 0x006D3D10` has the relevant UI/action-line slice only when `param_3 == 2` or `param_3 == 3`. A caller at `0x004F4515` pushes `0x2` before calling `0x006D3D10`; two prior calls push `0x0` and `0x1`.

Active in YR: Yes. This is the live tactical renderer path.

Evidence: xrefs to `0x006D3D10`; caller assembly at `0x004F44DF`, `0x004F44F4`, `0x004F4515`.

Within the pass-2/3 block, the verified order is:

| Order | Address | Call / branch | Player-visible role | Active in YR |
|---:|---:|---|---|---|
| 1 | `0x006D461A` | `FUN_006D9CE0` | viewport/clip prep for overlay pass | Yes |
| 2 | `0x006D4629` | surface vtable `+0x5C` | surface lock/prepare | Yes |
| 3 | `0x006D463F` | `FUN_006DAD60(0)` | first planning/queued waypoint path overlay pass | Yes / Conditional |
| 4 | `0x006D4648` | `FUN_006DA9D0(0)` | first selected factory rally-line overlay pass | Yes / Conditional |
| 5 | `0x006D4651` | `BuildingPlacement_OverlayRenderer` | building placement overlay | Conditional, when placement active |
| 6 | `0x006D4656` | `FUN_0053D850` | pre-object overlay work | Yes |
| 7 | `0x006D465F` | `Tactical_ObjectRenderingLoop` | main object rendering loop | Yes |
| 8 | `0x006D4664` | `FUN_005FFFA0` | post-object overlay work | Yes |
| 9 | `0x006D4669` | `LaserDrawClass__DrawAll` | lasers | Conditional, if active lasers exist |
| 10 | `0x006D466E` | `EBoltMgr__UpdateAndDrawAll` | electric bolts | Conditional |
| 11 | `0x006D4673` | `LineTrail__UpdateAndDrawAll` | projectile line trails | Conditional |
| 12 | `0x006D4678` | `RadBeam__DrawAndTickAll` | radiation beams | Conditional |
| 13 | `0x006D467F` | `Tactical__DrawUnitActionVisuals` | vtable `+0x130` visuals; also option-gated range/sensor/superweapon rings | Yes / Conditional per selected context |
| 14 | `0x006D46B6` | `FUN_00430AC0` | garrison/pip-style overlay from existing reports | Conditional |
| 15 | `0x006D46BD` | `FUN_006DA180` | bandbox rectangle | Conditional, while bandbox active |
| 16 | `0x006D46C6` | `FUN_006DAD60(1)` | second planning/queued waypoint path overlay pass | Yes / Conditional |
| 17 | `0x006D46CF` | `FUN_006DA9D0(1)` | second selected factory rally-line overlay pass | Yes / Conditional |
| 18 | `0x006D46D8` | `BuildingPlacement_OverlayRenderer(1)` | placement overlay second call | Conditional |
| 19 | `0x006D46DD` | `FUN_00637AA0` | reads `DAT_00AC4CF4`, gates radar overlay branch | Conditional |
| 20 | `0x006D46EA`, `0x006D46EF` | `DrawRadarOverlays_Normal`, `DrawRadarOverlays_Fog` | radar overlays when gate is nonzero | Conditional |
| 21 | `0x006D4711` loop | iterate `g_TechnoClass_Array` | per-techno lines/links/tethers | Yes |
| 21a | `0x006D473F-0x006D4750` | selected human branch: selected byte + `DAT_00843108`, then vtable `+0x438` | selected action lines | Conditional: human player techno, selected, option enabled, radar overlay gate false |
| 21b | `0x006D478E` | non-human FootClass/Psychic Sensor branch: `TechnoClass__DrawRadarActionLines` | tactical Psychic Sensor action lines | Conditional: caller gates pass and endpoint is inside local PsychicDetectionRadius coverage |
| 21c | `0x006D47B0-0x006D47BF` | `CaptureManagerClass__ShouldDrawLinks`, then `DrawLinks` | mind-control links | Conditional: CaptureManager exists and says draw |
| 21d | `0x006D47DF-0x006D47F6` | second CaptureManager owner lookup and `DrawLinks` | reverse/secondary mind-control links | Conditional |
| 21e | `0x006D47FB-0x006D48F1` | service/tether branch ending in `FUN_00705860` | service/tether line to building | Conditional: tether state exists and target object `WhatAmI == 6` |
| 22 | `0x006D491A` | `FUN_0063B2F0` | radar overlay cleanup/finalization | Conditional |
| 23 | `0x006D492B` | `DrawPixelFXSparkles` | pixel sparkles | Conditional |

Important ordering details:

- Selected action lines draw before CaptureManager links and before service/tether lines for the same techno.
- Non-human radar action lines draw before the same CaptureManager/service/tether section when `DAT_00AC4CF4 == 0`; when `DAT_00AC4CF4 != 0`, the branch skips the common link/tether block after radar-line consideration.
- `Tactical__DrawUnitActionVisuals` happens before both bandbox and selected action lines. It reads `DAT_00A8EB7E` directly for range/sensor/superweapon-ring style visuals; it is not the selected action-line call site.
- The selected action-line virtual call pushes two zero arguments (`PUSH 0`, `PUSH 0`) before calling vtable `+0x438`, so the forced/dashed argument path is not enabled from this pass.

## 5. INI and Retail Data

| Key | Section | Default/source | Active in YR | Evidence |
|---|---|---|---|---|
| `UnitActionLines` | `[Options]` | default `yes` from `OptionsClass__SetDefaults`; user INI may override | Yes | Ghidra `0x005FA350`, `0x005FA620`; retail install `RA2.INI:12` has `UnitActionLines=yes` |

The repo `ini/rules*.ini` and `ini/art*.ini` do not define this key; it is an Options/user INI key, not rules/art data.

## 6. Current Rust Implementation Status

The repo already has an app-layer target-line implementation:

| Rust area | Status vs this slice | Evidence |
|---|---|---|
| `TargetLineState` timer | implemented as 25 ticks | `src/app_target_lines.rs:18`, `src/app_target_lines.rs:148` |
| command trigger | implemented before queuing sim commands | `src/app_context_order.rs:731` |
| selected/mobile filtering | implemented | `src/app_target_lines.rs:158` |
| render order | Rust draws `target_lines` before `selection_brackets_front` in UI step 10 | `src/app_render/draw_passes.rs:298` |
| option gate | not observed in this slice | no `UnitActionLines` / user option read found in `src/` scan |
| radar action lines | not observed in this slice | no Rust analogue found in `src/` scan |
| CaptureManager/service/tether relative order | not mirrored by target-line pass | Rust has target lines in final UI step, not in the same per-techno loop |

This status is descriptive only; no Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OptionsClass__SetDefaults @ 0x005FA350` | verified | decompile shows `+0x1E = 1` | none |
| `OptionsClass__ReadFromINI @ 0x005FA620` | verified | decompile and assembly `0x005FACFA-0x005FACFD` | none |
| `OptionsClass__WriteToINI @ 0x005FAD10` | verified | decompile writes `[Options] UnitActionLines` | none |
| `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0` | verified | decompile and assembly `0x004E1F3B-0x004E1F41` | none |
| `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0` | verified | decompile and assembly `0x0055FB25-0x0055FB2B` | none |
| dialog init checkbox `0x601` | verified | assembly `0x005602B4-0x005602D6`, `0x004E2195-0x004E21B7` | none |
| `DAT_00843108` xrefs | verified | `get_bulk_xrefs`: one write, one read | none |
| `TechnoClass__SetDrawHealthBarsFlag @ 0x0070D180` | verified | decompile writes only `DAT_00843108` | none |
| `TacticalClass_Draw @ 0x006D3D10` pass order | verified | decompile and full disassembly | none for claimed slice |
| selected action-line call gate | verified | assembly `0x006D4735-0x006D4750` | none |
| non-human radar action-line placement | verified | assembly `0x006D4764-0x006D478E` | radar line style out of scope |
| CaptureManager link placement | verified | assembly `0x006D479F-0x006D47F6`; decompile `0x00472160`, `0x00472640` spot-check | link geometry out of scope |
| service/tether placement | verified | assembly `0x006D47FB-0x006D48F1` | geometry and field naming out of scope |
| `FUN_00637AA0` radar overlay gate | touched-not-exhausted | decompile returns `DAT_00AC4CF4` | exact user-visible mode represented by `DAT_00AC4CF4` deferred |
| current Rust option/read gate | touched-not-exhausted | `rg`/Codegraph/Rust reads | broader settings implementation out of scope |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - What is the binary default for `UnitActionLines`? Answer: enabled (`1`) at `OptionsClass + 0x1E`. Evidence: `0x005FA350`. Active in YR: Yes.

[RESOLVED] OQ2 - Is the key read from `[Options] UnitActionLines` or only a UI checkbox? Answer: read from `[Options] UnitActionLines` via `CCINIClass__ReadBool`, with current byte as default. Evidence: `0x005FA620`, string `0x008331C8`. Active in YR: Yes.

[RESOLVED] OQ3 - Is `DAT_00843108` independently configured? Answer: no independent key or writer found in this slice; it is written only by `0x0070D180`, whose discovered callers pass `OptionsClass + 0x1E`. Evidence: xrefs to `0x00843108`, `0x0070D180`. Active in YR: Yes.

[RESOLVED] OQ4 - Which visible checkbox applies the option? Answer: control `0x601` reads/writes `DAT_00A8EB7E` and syncs `DAT_00843108`; control `0x602` writes another option byte (`DAT_00A8EB80`), not this target. Evidence: `0x004E1DE0`, `0x0055FAA0`, `0x005602B4`, `0x004E2195`. Active in YR: Yes.

[RESOLVED] OQ5 - Is the selected action-line call inside `Tactical__DrawUnitActionVisuals`? Answer: no. `Tactical__DrawUnitActionVisuals @ 0x006DBE20` is called earlier at `0x006D467F`; selected action lines are called later from the parent `TacticalClass_Draw` loop at `0x006D4750`. Evidence: `0x006D3D10` disassembly. Active in YR: Yes.

[RESOLVED] OQ6 - What gates selected action lines in the tactical loop? Answer: human-player branch, radar overlay gate false, selected byte `+0x83 != 0`, and `DAT_00843108 != 0`; the virtual call receives two zero arguments. Evidence: `0x006D4725-0x006D4750`. Active in YR: Conditional on selected own unit and option enabled.

[RESOLVED] OQ7 - Are radar action lines gated by `DAT_00843108`? Answer: no such read occurs on the non-human radar action-line branch; it gates on visibility/function checks, techno flag bit from `+0x14`, and `+0x81 == 0`. Evidence: `0x006D4764-0x006D478E`; xrefs show only one read of `DAT_00843108` at `0x006D473F`. Active in YR: Conditional.

[RESOLVED] OQ8 - Do selected action lines draw before or after CaptureManager links and service/tether lines? Answer: before both, within the same per-techno loop. Evidence: `0x006D4750` precedes `0x006D47BF`, `0x006D47F6`, and `0x006D48F1`. Active in YR: Yes/Conditional per object state.

[DEFERRED] OQ9 - What exact user-visible mode does `DAT_00AC4CF4` represent? Reason: this slot only needed the branch value to place action lines relative to radar overlays. Category: requires-different-system-context. Next step: focused radar overlay investigation.

[DEFERRED] OQ10 - What exact pixels are produced by CaptureManager and service/tether line helpers? Reason: ordering was in scope; geometry was not. Category: out-of-scope. Next step: dedicated link/tether visual parity slice.

## Sources

- Ghidra decompiled: `0x005FA350`, `0x005FA620`, `0x005FAD10`, `0x004E1DE0`, `0x0055FAA0`, `0x0070D180`, `0x006D3D10`, `0x006DBE20`, `0x00472160`, `0x00472640`, `0x00637AA0`.
- Ghidra assembly/disassembly: full `TacticalClass_Draw @ 0x006D3D10`; focused contexts at `0x005FACFD`, `0x004E1F3B`, `0x0055FB25`, `0x005602B4`, `0x004E2195`, `0x006D473F`, `0x006D478E`, `0x006D47BF`, `0x006D48F1`, caller sites `0x004F44DF`, `0x004F44F4`, `0x004F4515`.
- Ghidra xrefs: `0x008331C8`, `0x00843108`, `0x00A8EB7E`, `0x0070D180`, `0x006D3D10`, `0x004DC340`.
- Starting docs checked but not trusted as ground truth: `C:/Users/enok/Documents/ra2-rust-game-docs/TARGET_LINES_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md`.
- Retail/user INI checked: `C:/Users/enok/Documents/Command and Conquer Red Alert II/RA2.INI:12`.
- Rust scan/context: `src/app_target_lines.rs`, `src/app_context_order.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`.
