# Behind Hidden Object Visual Path - Ghidra Research Report

**Address(es):** `0x006FA2AE..0x006FA2D3` in `TechnoClass__AI_Update`, `FUN_0070F1D0`, `FUN_00487E00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** What happens after `CellClass+0x100` hidden occupancy makes `FUN_00487E00` report hidden and `TechnoClass__AI_Update` enters the `CanBeHidden` / `[General] Behind` visual path.  
**Non-Scope:** Hidden-occupancy writer discovery, full `CellClass+0x100` reader inventory, passability, targeting, selection, and unrelated anim systems.  
**Confidence:** High for the binary path and marker ownership/lifetime; Medium for the fallback primitive line drawing shape because the decompiler mangles several stack temporaries, but its non-object nature and trigger condition are clear.  
**Active in YR:** Yes. This path is reached from standard `TechnoClass__AI_Update`, is gated by `TechnoTypeClass+0x724 CanBeHidden`, and retail YR has `[General] Behind=BEHIND` plus `[BEHIND]` art data. No TS-only `SpecialFlags` gate was found in this slice.

## 1. Overview

When a non-building techno with `CanBeHidden=true` stands in a cell where `FUN_00487E00` returns hidden, retail YR creates or maintains a "behind building" marker. In normal retail data that marker is an `AnimClass` using the `[General] Behind` anim type, attached to the hidden techno and stored on the techno at `+0x12C`.

The visible player effect is not making the unit itself disappear. It is a looping top-layer `BEHIND` highlight/marker over the obscured object while it remains in the hidden-occupancy condition.

## 2. Key Offsets / Fields

| Offset / field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x100` | hidden-occupancy counter entry context | `FUN_00487E00` reads it | Yes, via prior writer/reader report and current entry verification |
| `TechnoTypeClass+0x724` | `CanBeHidden` gate | `TechnoClass__AI_Update`; `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; parser evidence in prior report | Yes, default true and parsed from `CanBeHidden=` |
| `TechnoClass+0x12C` | pointer to the active behind marker anim/object | `TechnoClass__AI_Update` destroys/clears it; `FUN_0070F1D0` writes it | Yes |
| `RulesClass+0xB8` | `[General] Behind` `AnimTypeClass*` | `FUN_0070F1D0`; `RULESCLASS_FIELDS.csv:42`; `rulesmd.ini:562` | Yes |
| `AnimClass+0x33*4 = +0xCC` | owner object pointer | `AnimClass__SetOwnerObject` | Yes |
| `AnimClass+0x19D` | invisible/suppress-draw flag | `FUN_0070F1D0` writes it; `AnimClass__DrawIt` returns when nonzero | Conditional, set only when `DAT_00A8EB7F == 0` |

## 3. Core Logic

Entry context from `TechnoClass__AI_Update`:

1. The object gets its current coordinates via vtable `+0x48`.
2. X/Y are converted to a map coordinate by adding the signed `>> 31 & 0xFF` bias and shifting right by 8, matching the usual lepton-to-cell truncation pattern.
3. `MapClass__Get_CellClass` is called for that map coordinate.
4. The type gate at `TechnoTypeClass+0x724` must be true.
5. `FUN_00487E00(cell)` must return true.
6. `WhatAmI() == 2` is rejected after the helper returns true, so building subjects do not receive this marker even though buildings can write the hidden counter.
7. On failure of any gate, an existing `TechnoClass+0x12C` marker is destroyed through its vtable `+0xF8` and the field is cleared to zero.
8. On success, `FUN_0070F1D0` runs.

`FUN_0070F1D0`:

1. Calls the techno visual-state virtual at `vtable+0x68(0,0)` and returns unless the result is zero. This blocks marker creation for non-normal visual states such as cloak/warp states handled by `TechnoClass_GetVisualState @ 0x00703860`.
2. Returns immediately if `TechnoClass+0x12C` is already nonzero. It does not recreate, reposition, or retime the marker each tick once attached.
3. If `RulesClass+0xB8` is nonzero, allocates `0x1C8` bytes and constructs an `AnimClass` using that `AnimTypeClass*`.
4. The constructor call uses the techno's coordinate virtual `+0x48` with arguments that produce an owner-relative coordinate, passes loop multiplier `1`, draw flags `0x600`, and zero delay/z-adjust/reverse arguments.
5. The returned `AnimClass*` is stored in `TechnoClass+0x12C`.
6. If construction succeeded, `AnimClass__SetOwnerObject(anim, techno)` attaches the marker to the hidden techno and submits it to the display list.
7. If `DAT_00A8EB7F == 0`, `FUN_0070F1D0` sets `AnimClass+0x19D = 1`, and `AnimClass__DrawIt` later returns before drawing when this byte is nonzero.
8. If `RulesClass+0xB8` is zero, no anim object is created. Instead, the function draws a small flashing primitive line marker directly to `g_PrimarySurface` through surface vtable `+0x30`. This fallback has no `TechnoClass+0x12C` lifetime or owner attachment.

Active in YR: Yes for the normal anim path because retail YR sets `[General] Behind=BEHIND`; Conditional for the direct-line fallback because it requires the rules pointer at `+0xB8` to be null, which is not standard retail YR data.

## 4. INI Keys / Art Identity

| INI path | Value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `rulesmd.ini [General] Behind` | `BEHIND` | resolves into `RulesClass+0xB8` `AnimTypeClass*` | `rulesmd.ini:562`, `RULESCLASS_FIELDS.csv:42`, `FUN_0070F1D0` | Yes |
| `artmd.ini [BEHIND] Rate` | `200` | anim rate | `artmd.ini:14973` | Yes |
| `artmd.ini [BEHIND] Start` | `1` | start frame | `artmd.ini:14974` | Yes |
| `artmd.ini [BEHIND] LoopStart/LoopEnd` | `1` / `18` | looping frame range | `artmd.ini:14976..14977` | Yes |
| `artmd.ini [BEHIND] LoopCount` | `-1` | infinite loop, mapped to byte loop count behavior by anim system | `artmd.ini:14978`, `AnimClass__Constructor` loop-count field | Yes |
| `artmd.ini [BEHIND] Layer` | `top` | top-layer anim rendering | `artmd.ini:14979` | Yes |
| `artmd.ini [BEHIND] UseNormalLight` | `yes` | normal lighting behavior | `artmd.ini:14980` | Yes |
| `artmd.ini [BEHIND] ZAdjust` | `-512` | per-anim depth adjustment copied into `AnimClass+0x100` when constructor z-adjust argument is zero | `artmd.ini:14981`, `AnimClass__Constructor` | Yes |
| `artmd.ini [BEHIND] YSortAdjust` | `2000` | sort bias from art data | `artmd.ini:14982` | Yes |

## 5. Integration Points

| Area | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Caller | Only xref to `FUN_0070F1D0` is from `TechnoClass__AI_Update @ 0x006FA2D3` | Ghidra xrefs | Yes |
| Hidden test | `FUN_00487E00` returns hidden from `CellClass+0x100` with the exact building/counter==1 carve-out from prior report | `FUN_00487E00`; prior `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md` | Yes |
| Marker destruction | Failed gate destroys existing `TechnoClass+0x12C` through anim vtable `+0xF8`, then clears field | `TechnoClass__AI_Update`, `AnimClass__Destroy` | Yes |
| Marker ownership | New anim is attached to the techno via `AnimClass__SetOwnerObject`; the anim stores owner at `+0xCC` and owner gets a byte at object `+0x84` set | `AnimClass__SetOwnerObject` | Yes |
| Marker rendering | `AnimClass__DrawIt` suppresses draw if `AnimClass+0x19D != 0`; otherwise normal anim rendering applies | `AnimClass__DrawIt` | Conditional on `+0x19D` |

## 6. Current Rust Implementation Status

The Rust code has data and tests around building base foundation cells vs AddOccupy/RemoveOccupy hidden-occupancy semantics, but no discovered `BEHIND` marker/render lifecycle.

| Rust area | Status | Evidence |
|---|---|---|
| Hidden occupancy concept documented for footprint separation | implemented/scaffolded for movement-facing footprint decisions, not visual marker | `src/sim/production/production_tech.rs:566..570`; `src/sim/pathfinding/core_tests.rs:856..864` |
| `CanHideThings` and Add/Remove parsing | implemented in art data | `src/rules/art_data.rs:120..123`, `src/rules/art_data.rs:213`, `src/rules/art_data.rs:376..398` |
| `CanBeHidden` marker consumer | not found by symbol/search pass | Codegraph search for `CanBeHidden` and `BEHIND`; `rg` over `src/` |
| `BEHIND` anim visual | not found as a normal rendered effect path | `rg "BEHIND|Behind=" src` found no marker lifecycle |

Rust-facing visible effect required: when a non-building techno with `CanBeHidden=true` is hidden by the hidden-occupancy counter and is in normal visual state, render the `[General] Behind` anim (`BEHIND` in retail YR) as an owner-attached looping top-layer marker until the hidden condition fails; then destroy/remove the marker. The marker must be driven from INI-resolved `Behind`, not hardcoded to `BEHIND`, and the no-anim fallback is only needed if supporting rules with no `[General] Behind` anim pointer.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass__AI_Update` entry/exit gates | verified | `0x006FA2AE..0x006FA2D3` | none |
| `FUN_00487E00` entry behavior | verified as context only | `FUN_00487E00`; prior report | writer paths intentionally not re-covered |
| `FUN_0070F1D0` normal anim path | verified | `FUN_0070F1D0`, xref from `0x006FA2D3` | none |
| `FUN_0070F1D0` fallback line path | touched-not-exhausted | `FUN_0070F1D0`, `RulesClass+0xB8 == 0` branch | exact primitive geometry not fully reconstructed; nonstandard retail condition |
| `AnimClass__SetOwnerObject` ownership | verified | `AnimClass__SetOwnerObject`, xref from `0x0070F266` | none |
| `AnimClass__Destroy` marker removal | verified | `AnimClass__Destroy`, vtable `+0xF8` per decompile comment/xref | none |
| `AnimClass__DrawIt` visibility suppression | verified | early return on `AnimClass+0x19D` | none |
| `[General] Behind` / `[BEHIND]` identity | verified | `rulesmd.ini:562`, `artmd.ini:14972..14982`, `RULESCLASS_FIELDS.csv:42` | none |
| TS-legacy gate check | verified for this slice | no `SpecialFlags` / FogOfWar gate in decompiled path; standard AI and retail INI data | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-BH-001 - Does `FUN_0070F1D0` spawn an object or draw immediately?` It spawns an `AnimClass` when `RulesClass+0xB8` is nonzero; only the null-rules fallback draws immediate surface lines. Evidence: `FUN_0070F1D0`. Active in YR: Yes for anim path; Conditional for fallback.
- `[RESOLVED] OQ-BH-002 - What art is used?` The anim pointer is `[General] Behind`, which retail YR sets to `BEHIND`; `[BEHIND]` has top layer, loop 1..18, rate 200, `ZAdjust=-512`, `YSortAdjust=2000`. Evidence: `rulesmd.ini:562`, `artmd.ini:14972..14982`, `RULESCLASS_FIELDS.csv:42`. Active in YR: Yes.
- `[RESOLVED] OQ-BH-003 - Who owns marker lifetime?` The hidden techno owns the marker pointer at `TechnoClass+0x12C`; creation writes it, failure gates destroy it via `AnimClass__Destroy` and clear the field. Evidence: `FUN_0070F1D0`, `TechnoClass__AI_Update`, `AnimClass__Destroy`. Active in YR: Yes.
- `[RESOLVED] OQ-BH-004 - Does the marker follow the hidden object?` Yes; `AnimClass__SetOwnerObject` attaches the anim to the techno, storing owner at anim `+0xCC`, recomputing relative coords, and resubmitting display state. Evidence: `AnimClass__SetOwnerObject`. Active in YR: Yes.
- `[RESOLVED] OQ-BH-005 - Can the created marker be invisible?` Yes, if `DAT_00A8EB7F == 0`, `FUN_0070F1D0` sets `AnimClass+0x19D=1`, and `AnimClass__DrawIt` returns when that byte is nonzero. Evidence: `FUN_0070F1D0`, `AnimClass__DrawIt`. Active in YR: Conditional; exact global semantics not investigated.
- `[RESOLVED] OQ-BH-006 - Is this TS legacy?` No TS-only gate was found; path is standard `TechnoClass__AI_Update` with retail YR `Behind=BEHIND`. Evidence: Ghidra xrefs and `rulesmd.ini`. Active in YR: Yes.
- `[DEFERRED] OQ-BH-007 - What does `DAT_00A8EB7F` semantically represent?` Category: out-of-scope. Reason: this slice only needed to verify that it gates the marker's draw visibility after creation; tracing the global's writers/readers is a separate visual/options investigation.
- `[DEFERRED] OQ-BH-008 - Exact fallback line-marker geometry when `[General] Behind` is null.` Category: out-of-scope. Reason: retail YR data makes the anim path active; fallback is nonstandard and the decompiler mangles stack temporaries, though it clearly draws direct surface lines and owns no object lifetime.

## Sources

- Ghidra read-only decompiled: `TechnoClass__AI_Update`, `FUN_00487e00`, `FUN_0070f1d0`, `AnimClass__Constructor`, `AnimClass__SetOwnerObject`, `AnimClass__Destroy`, `AnimClass__DrawIt`, `TechnoClass_GetVisualState @ 0x00703860`.
- Ghidra xrefs: `FUN_0070F1D0` called only from `0x006FA2D3`; `FUN_00487E00` called from `0x006FA2AE`; `AnimClass__SetOwnerObject` xref from `0x0070F266`.
- INI: `ini/rulesmd.ini:562`; `ini/artmd.ini:14972..14982`; base RA2 corroboration `ini/rules.ini:594`, `ini/art.ini:10491..10500`.
- Docs: `CELLCLASS_0X100_HIDDEN_OCCUPANCY_READERS_GHIDRA_REPORT.md`; `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; `RULESCLASS_FIELDS.csv:42`; `RULESCLASS_CONSTRUCTOR_DEFAULTS.csv:44`.
- Rust scan: `src/sim/production/production_tech.rs:566..570`; `src/rules/art_data.rs:120..123`, `src/rules/art_data.rs:213`, `src/rules/art_data.rs:376..398`; `src/sim/pathfinding/core_tests.rs:856..864`.
