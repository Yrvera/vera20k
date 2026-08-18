# Skirmish BuildOffAlly First Reader - Ghidra Research Report

**Target:** `SKIRMISH_BUILD_OFF_ALLY_FIRST_READER`  
**Investigation Mode:** exhaustive-slice  
**Scope:** first verified runtime reader of offline Skirmish Start-branch `DAT_00A8B264` / `BuildOffAlly` after shell packing.  
**Non-Scope:** full building placement formulas, full sidebar production lifecycle, online packet protocol beyond excluding non-gameplay xrefs.  
**Confidence:** High for the first gameplay reader and its effect; Medium for chronological "first" phrasing because static xrefs include lobby/network/save readers that are not placement gameplay.

## 1. Summary

`BuildOffAlly` is a building placement/base-adjacency option. The first verified gameplay reader is `FUN_004A8EB0 @ 0x004A8EB0`, with the actual byte read at `0x004A8FFA`.

When a player is placing a building, `FUN_004A8EB0` scans the ring around the candidate foundation. Owned allied-base providers already allow placement through an owner/self path. If `DAT_00A8B264 != 0`, the function also checks buildings owned by allied houses and accepts those allied buildings as build-area providers only when the provider building type allows the allied-adjacency flag.

Active in YR: Yes. Evidence: `DisplayClass__BandBox_LeftUp @ 0x004AB280` calls `FUN_004A8EB0` on final placement release; `FUN_004A91B0 @ 0x004A91B0` calls the same helper during placement cursor/preview movement; both are ordinary tactical display paths and are not TS-only.

## 2. Verified Writer

Active in YR: Yes. In the offline Skirmish dialog handler `FUN_006ACEE0 @ 0x006ACEE0`, the Start Game branch reads checkbox control `0x69D` with `BM_GETCHECK (0xF0)`. If the message result equals exactly `1`, it writes `DAT_00A8B264 = 1`; otherwise the byte is false. The same branch mirrors the byte into `DAT_00A8B3DA`.

Evidence: decompile of `FUN_006ACEE0` around the `GetDlgItem(param_1, 0x69D)` block and the later `DAT_00A8B3DA = DAT_00A8B264` write.

## 3. First Gameplay Reader

Active in YR: Yes. `FUN_004A8EB0 @ 0x004A8EB0` is the first verified runtime/gameplay consumer found in the data xrefs for `DAT_00A8B264`. The direct read is:

- `0x004A8FFA`: read `DAT_00A8B264`.
- If zero, skip the allied-building provider test.
- If nonzero, call `HouseClass__IsAlliedWith @ 0x004F9A50` between the candidate provider building's owner and the placing house.
- If allied and the provider building type byte at `BuildingType + 0x1550` is nonzero, set the placement/build-area result true.

The same helper also has a self-owned provider path: if the provider building's owner index equals the placing house index and the provider type byte at `BuildingType + 0x154F` is nonzero, the helper accepts it without consulting `DAT_00A8B264`.

Active in YR: Yes. Evidence: `FUN_004A91B0 @ 0x004A9480` calls `FUN_004A8EB0` for building placement preview validity, and `DisplayClass__BandBox_LeftUp @ 0x004ABA59` calls it before queuing the build placement command.

## 4. Reader Conditions And Small Details

Active in YR: Yes/Conditional. `FUN_004A8EB0` only performs the ring scan when all of these gates pass:

- The placing house index equals `g_PlayerPtr + 0x30`.
- `g_IsMapEditor == 0`.
- The placement object argument is non-null.
- The target cell is not the invalid sentinel `DAT_008A03F8`.
- The object vtable `+0x2C` returns kind `7`, matching building placement.

Active in YR: Conditional. The scan uses the building type foundation dimensions plus `param_1[0x3AD] + 1` as the expansion radius. The helper deliberately skips cells inside the candidate building foundation and tests only the surrounding ring.

Active in YR: Conditional. For each ring cell, the helper calls `MapClass__Get_CellClass` then `Look_up_building_in_cell`. Empty cells do not matter. Only actual buildings around the candidate footprint can satisfy the build-area result.

Active in YR: Yes. The allied path has two separate gates: the session option byte `DAT_00A8B264` and the provider building type byte at `+0x1550`. Do not implement `BuildOffAlly` as "all allied buildings always provide build area"; the provider type must opt in through the type field verified by this helper.

## 5. Non-Gameplay Xrefs Excluded

Active in YR: Yes, but not the scoped gameplay consumer. Static xrefs to `DAT_00A8B264` include shell/network/session readers that pack, mirror, compare, serialize, or refresh options:

- `FUN_005E32D0 @ 0x005E32D0` compares and snapshots option bytes into `DAT_00AC11BD..C1`, then emits an options string through `FUN_005DBB60` or `FUN_0077E430`.
- `FUN_005DBB60 @ 0x005DBB60` formats `DAT_00A8B264` into an options/status string.
- `0x005B5448`, `0x005B8A4C`, and `0x006AEE2E` update or sync checkbox/control state.
- `0x005ED57B` and `0x005C3A95` copy option bytes into persisted/default structs.

These are active support paths, but they do not answer the scoped gameplay question because they do not affect building placement/base adjacency.

## 6. Current Rust Status

Current Rust implements the scoped consumer. `GameOptions::build_off_ally`
defaults to `true` and parses an explicit option override. The object-type
parser defaults `Adjacent` to `3` and parses `BaseNormal` plus typo-preserved
`EligibileForAllyBuilding`.

`src/sim/production/production_placement.rs::is_within_build_area` preserves
the separate native gates: own providers require `BaseNormal`; allied
providers require the option, a friendly relationship, and
`EligibileForAllyBuilding`; the ring uses placed `Adjacent + 1`. Focused tests
cover all four branches. The scoped implementation landed in `a35f1cd4`; the
later session-state move in `a74be6430` preserved the behavior.

## 7. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `DAT_00A8B264 != 0` lets allied buildings count as build-area providers, after `HouseClass__IsAlliedWith` and provider type byte `+0x1550` pass | implemented; preserve | `src/sim/production/production_placement.rs`, alliance/house relationship model | Player A can place near allied Player B's valid base provider when option enabled, but not when disabled | `build_off_ally_enabled_accepts_allied_eligible_provider`, `build_off_ally_disabled_rejects_allied_eligible_provider` | Keep alliance iteration and option state deterministic |
| Self-owned providers do not depend on `DAT_00A8B264` | implemented; preserve | same | Player can still place near own ConYard with BuildOffAlly off | `build_off_ally_off_keeps_own_base_provider` | Avoid regressing normal production placement |
| Allied path also requires provider `EligibileForAllyBuilding` (`+0x1550`); self-owned path uses `BaseNormal` (`+0x154F`) | implemented; preserve | rules/object type parsing and placement | A building that is not valid for allied build area must not provide allied placement even when allied and option on | `build_off_ally_requires_eligibile_for_ally_building` | Preserve the typo-spelled retail INI key and do not reuse `BaseNormal` |

## 8. Negative Facts / Do Not Do

- Do not route `BuildOffAlly` through `House+0x1605C` team setup. Active in YR: Yes; `House+0x1605C` was verified separately as team/alliance adjunct, while `DAT_00A8B264` is read directly by `FUN_004A8EB0`.
- Do not treat BuildOffAlly as a scenario-init option. Active in YR: Yes; the first gameplay consumer is tactical placement preview/click logic, not `ScenarioClass__Create_Houses` or `Post_Map_Init`.
- Do not make every allied building a provider. Active in YR: Conditional; the helper also checks provider building type byte `+0x1550`.
- Do not let map editor placement inherit this path. Active in YR: Conditional; `FUN_004A8EB0` returns early to permissive success when `g_IsMapEditor != 0`.

## 9. Remaining Uncertainty

- Exact semantic names and INI sources for BuildingType bytes `+0x154F` and `+0x1550` were not traced in this slot. They should be mapped before implementation if Rust does not already parse equivalent fields.
- This slot did not trace the full placement formula beyond the first consumer, by request.

## 10. Stale Docs / Replacement Wording

- `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`: replace the deferred BuildOffAlly row with: "`BuildOffAlly` first verified gameplay reader is `FUN_004A8EB0 @ 0x004A8EB0`, read at `0x004A8FFA`; it gates whether allied buildings can count as build-area providers during building placement preview/click, subject to `HouseClass__IsAlliedWith` and provider BuildingType byte `+0x1550`."

## 11. Sources

- Ghidra read-only decompile: `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_004A8EB0 @ 0x004A8EB0`, `FUN_004A91B0 @ 0x004A91B0`, `DisplayClass__BandBox_LeftUp @ 0x004AB280`, `FUN_005E32D0 @ 0x005E32D0`, `FUN_005DBB60 @ 0x005DBB60`.
- Ghidra xrefs to data address `0x00A8B264`.
- Prior docs: `skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`, `SESSIONCLASS_GHIDRA_REPORT.md`, `SPECIAL_FLAGS_SYSTEM.md`, `skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/game_options.rs`, `src/sim/production/production_placement.rs`.
