# Superweapon Lighting Transition Controller 0x0053C280 - Ghidra Research Report

**Address(es):** `0x0053C280` primary controller; `0x0053AD00` propagation helper; `0x0053AB70` nuke flash direct writer; `0x0053A6C0` shared effect tick; `0x00539EB0` Lightning Storm start; `0x0053AE50` Psychic Dominator start; `0x0053AF40` Psychic Dominator process; `0x00689E90` scenario `[Lighting]` reader.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact `SuperWeaponEffects::UpdateLighting` semantics for dynamic scenario-light profile selection and state/timeline integration for Lightning Storm, Nuke flash, and Psychic Dominator.
**Non-Scope:** Lightning Storm strike RNG/damage, nuke missile/ground-zero damage, Psychic Dominator capture/damage, ordinary map-lighting formula, LightConvert internals beyond the controller call boundary, screen-white nuke overlay internals.
**Confidence:** High
**Active in YR:** Yes, conditional on standard superweapon effects being triggered.

## Target Question

Re-check exact `SuperWeaponEffects::UpdateLighting` controller semantics at `0x0053C280` for Lightning Storm, Nuke, and Psychic Dominator ambient transitions: priority, target fields, rate/profile fields, state gates, restore-to-normal behavior, and standard YR activity.

## Non-Goals

- Do not re-investigate ordinary `[Lighting]` defaults/formula except where the controller reads or writes ScenarioClass lighting fields.
- Do not re-investigate LS bolt RNG, bridge AoE, PD capture/damage, or nuke damage.
- Do not mutate Ghidra state, Rust, INI files, or in-repo docs.

## Evidence Needed To Mark COMPLETE

- Decompile `0x0053C280` and verify each branch against assembly context.
- Verify the state globals that feed the branch gates from live launch/tick/reset paths.
- Verify the ScenarioClass fields selected by each branch.
- Verify restore-to-normal write/call behavior.
- Verify direct nuke flash path and shared tick state transition.
- Scan current Rust surfaces for missing dynamic ambient bridge.
- Identify stale prior-doc wording where binary evidence contradicts it.

## Stop Conditions

- Stop at the `0x0053AD00` call boundary after verifying the exact arguments passed by the controller.
- Stop after proving controller state gates and writers; downstream palette interpolation is a separate LightConvert/render investigation.
- Stop without writing anything except this report and the shared claims file row.

## 1. Overview

`0x0053C280` chooses one dynamic lighting profile and propagates it through `0x0053AD00`. It writes `ScenarioClass+0x3530` to the selected ambient target immediately, scales two or three profile channel fields by `*1000/100`, then calls `0x0053AD00(..., 1)`. The normal branch writes `+0x3530` back to ordinary ambient and calls `0x0053AD00(-1, -1, -1, 0)`.

The controller priority verified from branch order is:

1. Nuke/flash-style profile when `DAT_00A9FABC == 1` OR `DAT_00A9FAB0 != 0`.
2. Lightning/Ion-style profile when `DAT_00A9FAB4 != 0`.
3. Normal restore when `DAT_00A9FAC0 == 0` OR `DAT_00A9FAC0 == 5`.
4. Psychic Dominator profile when `DAT_00A9FAC0` is any other live state.

This corrects stale docs that swapped the Lightning Storm and Nuke/Ion field groups.

## 2. Class Layout / Key Offsets

All offsets are byte offsets from `ScenarioClass` (`g_ScenarioClass_Instance` / `DAT_00A8B230`).

| Offset / global | Verified role in this controller | Evidence | Active in YR |
|---:|---|---|---|
| `+0x3528` | ordinary/base ambient target used for restore | `0x0053C38E..0x0053C3A9`, `0x00689E90` | Yes |
| `+0x3530` | mutable current/dynamic ambient target written by controller | `0x0053C2B3`, `0x0053C32E`, `0x0053C3BE`, `0x0053C3A0` | Yes/conditional |
| `+0x3548` | Lightning/Ion ambient target | `0x0053C2A6..0x0053C2B3`, `0x00689E90` reads `IonAmbient` | Conditional |
| `+0x354C/+0x3550/+0x3554` | Lightning/Ion red/green/blue profile channels passed scaled to `0x0053AD00` | `0x0053C2B9..0x0053C30E`, `0x00689E90` reads `IonRed/IonGreen/IonBlue` | Conditional |
| `+0x3560` | nuke/flash ambient target | `0x0053C3B1..0x0053C3BE`, `0x0053AB70..0x0053ABBD` | Conditional |
| `+0x3564/+0x3568/+0x356C` | nuke/flash red/green/blue profile channels passed scaled to `0x0053AD00` | `0x0053C3C4..0x0053C43A`; reset defaults from `SCENARIO_LIGHTING_DEFAULT_RESET_PATH` | Conditional |
| `+0x3578` | `NukeAmbientChangeRate` parsed/reset field; not read by `0x0053C280` | `0x00689E90`; absent from controller assembly | Conditional, outside this controller |
| `+0x357C` | Dominator ambient target | `0x0053C321..0x0053C32E`, `0x00689E90` | Conditional |
| `+0x3580/+0x3584/+0x3588` | Dominator red/green/blue profile channels passed scaled to `0x0053AD00` | `0x0053C334..0x0053C389`, `0x00689E90` | Conditional |
| `+0x3594` | `DominatorAmbientChangeRate` parsed/reset field; not read by `0x0053C280` | `0x00689E90`; absent from controller assembly | Conditional, outside this controller |
| `DAT_00A9FABC` | nuke/flash lighting phase: 0 inactive, 1 flash profile, 2 post-flash delay | `0x0053AB70`, `0x0053A6C0`, `0x0053A110` | Conditional |
| `DAT_00A9FAB0` | nuke screen-overlay state; also forces nuke/flash lighting branch while nonzero | `0x0053C290..0x0053C297`, reset `0x00539760` | Conditional |
| `DAT_00A9FAB4` | Lightning Storm active flag | `0x00539EB0`, `0x0053A6C0`, `0x0053A100` | Conditional |
| `DAT_00A9FAC0` | Psychic Dominator state 0..5 | `0x0053AE50`, `0x0053AF40`, `0x0053B400` | Conditional |

## 3. Core Logic

### 3.1 Controller branch order

Verified pseudocode:

```text
if flash_state == 1 or nuke_screen_overlay_state != 0:
    current_ambient = scenario.nuke_flash_ambient
    apply_profile(nuke_red, nuke_green, nuke_blue, enabled=1)
elif lightning_storm_active:
    current_ambient = scenario.ion_lightning_ambient
    apply_profile(ion_red, ion_green, ion_blue, enabled=1)
elif pd_state == 0 or pd_state == 5:
    current_ambient = scenario.normal_ambient
    apply_profile(-1, -1, -1, enabled=0)
else:
    current_ambient = scenario.dominator_ambient
    apply_profile(dominator_red, dominator_green, dominator_blue, enabled=1)
```

Evidence: decompile `0x0053C280`; assembly context `0x0053C280..0x0053C43A`.

### 3.2 Scaling and signedness

Each profile channel passed to `0x0053AD00` is scaled as signed integer `value * 1000 / 100`. Assembly implements this with LEA multiplication to `value * 1000`, `0x51EB851F` magic signed divide-by-100, `SAR 5`, and sign correction. Evidence: repeated blocks at `0x0053C2BF..0x0053C30C`, `0x0053C33A..0x0053C387`, and `0x0053C3CA..0x0053C436`.

No clamp is performed in this controller. Any clamping/normalization is downstream of `0x0053AD00` and outside this slice.

### 3.3 `0x0053AD00` argument shape

The controller calls `0x0053AD00` with four effective arguments:

| Branch | ECX | EDX | stack arg | final stack arg |
|---|---|---|---|---|
| Lightning/Ion | scaled `+0x354C` | scaled `+0x3550` | scaled `+0x3554` | `1` |
| Nuke/flash | scaled `+0x3564` | scaled `+0x3568` | scaled `+0x356C` | `1` |
| Psychic Dominator | scaled `+0x3580` | scaled `+0x3584` | scaled `+0x3588` | `1` |
| Normal restore | `-1` | `-1` | `-1` | `0` |

`0x0053AD00` forwards those four values to LightConvert/color scheme collections, calls `0x004AE4C0`, then requests redraw via `0x004F42F0(1)`. Evidence: decompile `0x0053AD00`; controller assembly `0x0053C38E..0x0053C43A`.

## 4. INI Keys

| `[Lighting]` key | Scenario field | Controller use | Evidence | Active in YR |
|---|---:|---|---|---|
| `IonAmbient` | `+0x3548` | Lightning Storm ambient target | `0x00689E90`, `0x0053C2AD` | Conditional |
| `IonRed/IonGreen/IonBlue` | `+0x354C/+0x3550/+0x3554` | Lightning Storm profile channels | `0x00689E90`, `0x0053C2BF..0x0053C30E` | Conditional |
| `IonGround/IonLevel` | `+0x3558/+0x355C` | not read by controller; used by cell-lighting consumer branch | `0x00689E90`; `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT` | Conditional |
| `NukeAmbientChangeRate` | `+0x3578` | not read by controller | `0x00689E90`; no `+0x3578` access in `0x0053C280` | Conditional |
| `DominatorAmbient` | `+0x357C` | PD ambient target | `0x00689E90`, `0x0053C328` | Conditional |
| `DominatorRed/DominatorGreen/DominatorBlue` | `+0x3580/+0x3584/+0x3588` | PD profile channels | `0x00689E90`, `0x0053C33A..0x0053C389` | Conditional |
| `DominatorGround/DominatorLevel` | `+0x358C/+0x3590` | not read by controller; used by cell-lighting consumer branch | `0x00689E90`; `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT` | Conditional |
| `DominatorAmbientChangeRate` | `+0x3594` | not read by controller | `0x00689E90`; no `+0x3594` access in `0x0053C280` | Conditional |

Nuke/flash ambient and RGB fields `+0x3560/+0x3564/+0x3568/+0x356C` are reset by `FUN_00683610` but not map-key parsed by `0x00689E90`; only `+0x3578` is parsed as `NukeAmbientChangeRate`. Evidence: `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT`; decompile `0x00689E90`.

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Lightning Storm start | `0x00539EB0` sets `DAT_00A9FAB4=1`, stores owner/target/duration, then calls `0x0053C280`. | decompile `0x00539EB0` | Yes when Weather Control super fires |
| Lightning Storm cleanup | when cloud list is empty and ending flag is set, `0x0053A6C0` clears `DAT_00A9FAB4`, clears owner/target, calls `0x0053C280`, then clears ending flag. | decompile `0x0053A6C0` | Yes |
| Nuke direct flash writer | `0x0053AB70` sets `DAT_00A9FABC=1`, timer `0x1E`, start frame, writes `+0x3530=+0x3560`, and invokes flash profile propagation. | decompile + assembly `0x0053AB70..0x0053ABDD` | Yes on NUKE warhead impact |
| Shared flash tick | `0x0053A6C0` changes `DAT_00A9FABC: 1 -> 2` after timer expiry, sets timer `0x0F`, calls `0x0053C280`, then changes `2 -> 0` after the second timer. | decompile + assembly `0x0053A6C0..0x0053A742` | Yes |
| PD start | `0x0053AE50` requires both Dominator anim types, sets `DAT_00A9FAC0=1`, scenario timing fields `+0x1248/+0x124C/+0x1250`, then calls `0x0053C280`. | decompile `0x0053AE50`; assembly `0x0053AF0C..0x0053AF27` | Yes when Psychic Dominator super fires |
| PD process | state `1 -> 2` happens with no lighting call; states `2/3/4` remain in PD profile branch; state `4 -> 5` clears target/anim and calls `0x0053C280`; state `5 -> 0` waits until `+0x3530 == +0x352C`. | decompile `0x0053AF40` | Yes |
| Reset | `0x00539760` clears all effect globals, writes `+0x3530=+0x3528`, clears `DAT_00A9FAB0`, and calls `0x0053AD00(-1, -1, -1, 0)`. | decompile + assembly `0x00539760` | Yes on scenario reset/new scenario |

## 6. Current Rust Implementation Status

Rust currently has ordinary map lighting and building point-light rebuild surfaces, but no dynamic scenario ambient profile bridge for superweapon effects.

| Surface | Current status | Evidence |
|---|---|---|
| `src/map/lighting.rs` | parses ordinary `[Lighting]` into `LightingConfig`; computes per-cell tint/grid; tests ordinary defaults/formula | `LightingConfig` at line 44, `parse_lighting` at line 291, `cell_tint` at line 307 |
| `src/app_init.rs` | rebuilds app lighting from base map lighting plus live building point lights | `rebuild_lighting_grid_from_sim` at line 167 |
| `src/sim/superweapon/lightning_storm.rs` | has `LightningStormState` and bolt generation/damage, but no ScenarioClass-style dynamic ambient state/profile output | `LightningStormState` at line 36, `process` at line 111 |
| PD/Nuke dynamic ambient | no corresponding Rust state bridge observed in Codegraph context or source scan | Codegraph context for lighting/superweapon surfaces |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x0053C280` controller | verified | decompile + assembly `0x0053C280..0x0053C43A` | none |
| Branch priority | verified | `0x0053C280..0x0053C321` | none |
| Lightning/Ion branch fields | verified | `0x0053C2A6..0x0053C30E`, `0x00689E90` | none |
| Nuke/flash branch fields | verified | `0x0053C3B1..0x0053C43A`, `0x0053AB70` | none |
| PD branch fields | verified | `0x0053C321..0x0053C389`, `0x00689E90` | none |
| Normal restore branch | verified | `0x0053C38E..0x0053C3A9`, reset `0x00539760` | none |
| `0x0053AD00` propagation boundary | verified to call boundary | decompile `0x0053AD00` | downstream interpolation internals out-of-scope |
| LS start/cleanup writers | verified | `0x00539EB0`, `0x0053A6C0` | none for lighting controller |
| Nuke flash writer/timeline | verified | `0x0053AB70`, `0x0053A6C0` | screen overlay internals out-of-scope |
| PD start/process timeline | verified | `0x0053AE50`, `0x0053AF40` | PD capture/damage out-of-scope |
| Current Rust dynamic ambient bridge | verified missing | Codegraph + source scan | implement later |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What is the controller branch priority? -> Flash/nuke-style, Lightning/Ion, normal restore, PD profile.` (evidence: `0x0053C280..0x0053C321`)
- `[RESOLVED] OQ-02 - Which fields does Lightning Storm select? -> `+0x3548` ambient and `+0x354C/+0x3550/+0x3554` profile channels.` (evidence: `0x0053C2A6..0x0053C30E`)
- `[RESOLVED] OQ-03 - Which fields does Nuke/flash select? -> `+0x3560` ambient and `+0x3564/+0x3568/+0x356C` profile channels.` (evidence: `0x0053C3B1..0x0053C43A`, `0x0053AB70`)
- `[RESOLVED] OQ-04 - Which fields does PD select? -> `+0x357C` ambient and `+0x3580/+0x3584/+0x3588` profile channels.` (evidence: `0x0053C321..0x0053C389`)
- `[RESOLVED] OQ-05 - Does controller read `NukeAmbientChangeRate +0x3578`? -> No, no controller access; reader exists in `0x00689E90`.` (evidence: `0x0053C280..0x0053C43A`, `0x00689E90`)
- `[RESOLVED] OQ-06 - Does controller read `DominatorAmbientChangeRate +0x3594`? -> No, no controller access; reader exists in `0x00689E90`.` (evidence: `0x0053C280..0x0053C43A`, `0x00689E90`)
- `[RESOLVED] OQ-07 - How does normal restore work? -> writes `+0x3530=+0x3528`, calls `0x0053AD00(-1,-1,-1,0)`, returns.` (evidence: `0x0053C38E..0x0053C3B0`)
- `[RESOLVED] OQ-08 - What scales are used? -> profile channels are signed `*1000/100`; no clamp in controller.` (evidence: `0x0053C2BF..0x0053C436`)
- `[RESOLVED] OQ-09 - What starts LS lighting? -> `LightningStorm__Start` sets `DAT_00A9FAB4=1` then calls controller.` (evidence: `0x00539EB0`)
- `[RESOLVED] OQ-10 - What restores after LS? -> process cleanup clears `DAT_00A9FAB4` then calls controller.` (evidence: `0x0053A6C0`)
- `[RESOLVED] OQ-11 - What starts nuke flash lighting? -> `ScreenNukeFlash` sets `DAT_00A9FABC=1`, timer 30, writes nuke ambient, calls propagation.` (evidence: `0x0053AB70`)
- `[RESOLVED] OQ-12 - How does nuke/flash leave state 1? -> shared tick changes `1 -> 2` after 30 frames, calls controller, then `2 -> 0` after 15 frames without a controller call at that transition.` (evidence: `0x0053A6C0`)
- `[RESOLVED] OQ-13 - What starts PD lighting? -> PD start sets state 1 and calls controller after anim/scenario timing setup.` (evidence: `0x0053AE50`)
- `[RESOLVED] OQ-14 - What restores after PD? -> state `4 -> 5` calls controller, which restores normal because state 5 is restore branch; state `5 -> 0` waits on `+0x3530 == +0x352C`.` (evidence: `0x0053AF40`, `0x0053C313..0x0053C3A9`)
- `[RESOLVED] OQ-15 - Is current Rust wired for dynamic superweapon ambient? -> no bridge observed; ordinary map lighting and LS state exist separately.` (evidence: Codegraph context; `src/map/lighting.rs`, `src/app_init.rs`, `src/sim/superweapon/lightning_storm.rs`)
- `[DEFERRED] OQ-16 - What exact interpolation/fade math lives inside downstream LightConvert/color-scheme methods?` (category: out-of-scope; reason: this target stops at controller arguments; next-step-if-pursued: investigate `0x0053AD00` callees and LightConvert method slot `+4`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Dynamic lighting profile priority is flash/nuke first, then Lightning/Ion, then normal/PD by PD state. | `0x0053C280..0x0053C321` | missing | needed sim-to-render/app lighting bridge; `src/sim/superweapon/lightning_storm.rs`; future nuke/PD state | Resolve one active profile deterministically with that priority. | Nuke flash during active Lightning Storm uses flash profile until flash state no longer forces branch, then returns to LS profile if LS remains active. | Do not let LS override nuke flash; do not merge profiles additively. |
| Lightning Storm uses Ion profile fields `IonAmbient/IonRed/IonGreen/IonBlue`, not Nuke fields. | `0x0053C2A6..0x0053C30E`, `0x00689E90` | missing | map lighting config should retain dynamic profiles; render rebuild should accept active profile | During active storm, set dynamic ambient/profile from `+0x3548/+0x354C/+0x3550/+0x3554`; restore after storm cleanup. | `superweapon_lighting_lightning_storm_uses_ion_profile_until_cleanup` | Do not use `NukeAmbient`/`+0x3560` for Lightning Storm. |
| PD states 1-4 use Dominator profile; state 5 enters normal restore branch, then state 0 is set only after `+0x3530 == +0x352C`. | `0x0053AE50`, `0x0053AF40`, `0x0053C313..0x0053C389` | missing | future PD state machine + lighting bridge | Preserve PD state gates and call restore on 4->5. | `superweapon_lighting_psychic_dominator_state5_restores_normal_before_done` | Do not keep Dominator profile active in state 5. |
| Controller does not read `NukeAmbientChangeRate +0x3578` or `DominatorAmbientChangeRate +0x3594`; it passes RGB channels and a constant enable flag to `0x0053AD00`. | `0x0053C280..0x0053C43A`, `0x00689E90` | missing/unchecked | profile parser/storage naming | Store these rate fields separately until their actual consumer is implemented; do not feed them as controller RGB arguments. | `superweapon_lighting_controller_ignores_ambient_change_rate_fields` | Do not name `+0x356C` or `+0x3588` as ambient change rate in controller code; they are blue channels here. |

Concrete proposed Rust test names:

- `superweapon_lighting_priority_nuke_flash_over_lightning_storm`
- `superweapon_lighting_lightning_storm_uses_ion_profile_until_cleanup`
- `superweapon_lighting_psychic_dominator_states_1_to_4_use_dominator_profile`
- `superweapon_lighting_psychic_dominator_state5_restores_normal_before_done`
- `superweapon_lighting_controller_ignores_ambient_change_rate_fields`

## 10. Negative Facts / Do Not Do

- Do not use `+0x3560/+0x3564/+0x3568/+0x356C` as Lightning Storm fields. Evidence: LS branch is `DAT_00A9FAB4 != 0` at `0x0053C29D..0x0053C30E`, using `+0x3548/+0x354C/+0x3550/+0x3554`.
- Do not treat `+0x3554`, `+0x356C`, or `+0x3588` as ambient change-rate fields in `0x0053C280`; they are the third profile channel argument passed to `0x0053AD00`. Evidence: controller assembly and parser keys for IonBlue/DominatorBlue.
- Do not implement controller profile blending. The binary selects exactly one branch and returns. Evidence: explicit branch order and single `0x0053AD00` call path in `0x0053C280`.
- Do not keep PD lighting active in state 5. State 5 is a normal-restore branch in the controller. Evidence: `0x0053C313..0x0053C38E`.
- Do not expect `DAT_00A9FABC: 2 -> 0` to call `0x0053C280`. The controller call happens on `1 -> 2`; `2 -> 0` only clears the state. Evidence: `0x0053A6C0..0x0053A742`.

## Remaining Uncertainty

- None inside the claimed controller slice. Downstream interpolation inside `0x0053AD00` callees remains intentionally out-of-scope.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md` section "Shared Lighting System" replacement wording:
  - Replace "PD_State == 1 OR NukeFlash active selects Flash ambient `+0x3560`; else if LS_Active selects LS ambient `+0x3548`" with "If `DAT_00A9FABC == 1` or `DAT_00A9FAB0 != 0`, the controller selects nuke/flash ambient `+0x3560` and profile channels `+0x3564/+0x3568/+0x356C`. Else, if `DAT_00A9FAB4 != 0`, Lightning Storm selects the Ion/Lightning profile `+0x3548/+0x354C/+0x3550/+0x3554`."
  - Replace "intensity = `+0x3554/+0x356C/+0x3588`" with "the controller scales RGB/profile channel fields by `*1000/100` and passes them as three arguments to `0x0053AD00`; `+0x3554`, `+0x356C`, and `+0x3588` are the third profile channels in their respective branches."

- `C:/Users/enok/Documents/ra2-rust-game-docs/PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md` section "ScenarioClass Lighting Offsets" replacement wording:
  - Replace `+0x3554 NukeAmbientChangeRate`, `+0x3560 LightningStorm ambient`, `+0x356C LightningStormChangeRate`, and `+0x3588 DominatorBlue / PD blue tint` ambiguity with "Lightning/Ion branch uses `+0x3548/+0x354C/+0x3550/+0x3554`; nuke/flash branch uses `+0x3560/+0x3564/+0x3568/+0x356C`; `+0x3578` is the parsed `NukeAmbientChangeRate` but is not read by `0x0053C280`; PD branch uses `+0x357C/+0x3580/+0x3584/+0x3588`; `+0x3594` is parsed `DominatorAmbientChangeRate` but is not read by `0x0053C280`."

- `C:/Users/enok/Documents/ra2-rust-game-docs/SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md` section "Shared Lighting System" replacement wording:
  - Replace priority wording with "Priority is nuke/flash profile when `DAT_00A9FABC == 1` or `DAT_00A9FAB0 != 0`, then Lightning/Ion profile when `DAT_00A9FAB4 != 0`, then normal restore for PD state 0 or 5, else Dominator profile for PD states 1-4."

## Sources

- Ghidra decompile: `0x0053C280`, `0x0053AD00`, `0x00539760`, `0x00539EB0`, `0x0053A6C0`, `0x0053AB70`, `0x0053AE50`, `0x0053AF40`, `0x0053A100`, `0x0053A110`, `0x0053B400`, `0x00689E90`, `0x006CC390`.
- Ghidra assembly context: `0x0053C280..0x0053C43A`, `0x0053AB70..0x0053ABDD`, `0x0053A6C0..0x0053A742`, `0x00539760`.
- Existing docs checked: `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`, `PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT.md`, `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md`, `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md`, `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`, `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, `SUPERWEAPON_GAPS_INVESTIGATION_REPORT.md`, `NUKE_SUPERWEAPON_GHIDRA_REPORT.md`.
- Rust scan: Codegraph context plus `src/map/lighting.rs`, `src/app_init.rs`, `src/sim/superweapon/lightning_storm.rs`.
