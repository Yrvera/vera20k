# GACNST Undeploy Unlimbo-Fail Refund - Ghidra Research Report

**Address(es):** `0x00449C30` (`BuildingClass__Sell`, state 2 branch), `0x00449BC0` (`BuildingClass__CanUndeployMCV`), `0x005F5C60` (`ObjectClass__GetHealthRatio`), `0x0070ADA0` (`TechnoClass::GetRefundValue` wrapper), `0x004F9950` (`HouseClass__Add_Credits`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** failed `UnitClass::Unlimbo` after stock YR `GACNST` reverse-converts through `UndeploysInto=AMCV`, including refund value provenance and visible object/money result.  
**Non-Scope:** full sell survivor/garrison flow, normal successful AMCV placement beyond health-transfer comparison, AMCV forward deploy validation, and all non-GACNST `UndeploysInto` cases except where needed for contrast.  
**Confidence:** High for refund provenance and same-function lifecycle; Medium for deeper limbo-object internals after failure because this slice verifies no same-function delete but does not exhaust all later global-array cleanup paths.  
**Active in YR:** Yes. Stock `rulesmd.ini` has `[GACNST] UndeploysInto=AMCV`, `[AMCV] DeploysInto=GACNST`, both cost/strength 3000/1000, and `[MultiplayerDialogSettings] MCVRedeploys=yes`.

## 1. Overview

The ambiguous `uStack_b4._4_4_` decompiler value in `BuildingClass__Sell` is not a high dword of the saved health-ratio double. Disassembly shows the saved health ratio is written to `[ESP+0x24]`, then `vtable+0x2BC` writes an integer refund value to `[ESP+0x30]`; the failed-AMCV-unlimbo branch pushes `[ESP+0x30]` into `HouseClass__Add_Credits`.

Player-visible result when the AMCV cannot be placed during redeploy: no AMCV appears, the GACNST is removed/uninitialized at the end of the sell/undeploy completion branch, and the owner receives the standard sell-back refund value from `vtable+0x2BC`.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass+0xBC` | `BuildingClass::Sell` internal phase counter (not the general `MissionClass` mission field); state `2` is completion/payout/undeploy conversion (corrected 2026-05-29: was labeled `MissionState`; binary shows it is a sell-phase counter local to `BuildingClass__Sell`, not the general MissionClass mission field — via `decompile_function 0x00449C30` — MISLEADING) | `0x00449C47..0x00449C68` dispatches states 0/1/2 | Yes |
| `BuildingClass+0x6DD` | animation-complete gate for state 2 | `0x00449C99..0x00449CA1` exits unless nonzero | Yes |
| `TechnoTypeClass+0x408` | `UndeploysInto` UnitType pointer | `0x00449CEA..0x00449CF8` tests it; INI `[GACNST] UndeploysInto=AMCV` | Yes |
| `TechnoTypeClass+0x16B9` | ConYard special restriction flag (`ConstructionYard=yes`) | `0x00449CFE..0x00449D5E`; `0x00449BC0` | Yes for GACNST |
| `BuildingClass+0x218` / `+0x2C0` | deployed-link / gate fields in ConYard redeploy legality chain | `0x00449D15..0x00449D58`; `0x00449BC0` | Conditional; live for MCV redeploy |
| `DAT_00A8B320` | runtime MCVRedeploys/session option | `0x00449D43..0x00449D4A`; `0x00449BC0`; INI default yes | Yes |
| vtable `+0x2BC` | refund-value getter; vtable slot resolves to `0x0070ADA0` (`FUN_0070ada0` — Ghidra has no named label; the "TechnoClass::GetRefundValue wrapper" description is an inference from calling context, not a confirmed Ghidra label; corrected 2026-05-29: was "`TechnoClass::GetRefundValue wrapper (0x0070ADA0)`"; binary shows `FUN_0070ada0` with no matching named label — via `get_function_by_address 0x0070ADA0` + `read_memory 0x007e4178` — MISLEADING) | `0x00449E74`, `0x0044A1A3`, `0x0044A215`; vtable base `0x007E3EBC`+`0x2BC`=`0x007E4178` reads `0x0070ADA0` | Yes |
| stack `[ESP+0x24]` | saved `ObjectClass__GetHealthRatio` double | `0x00449E66` call, `0x00449E70 FSTP double ptr [ESP+0x24]` | Yes |
| stack `[ESP+0x30]` | saved integer refund value from `vtable+0x2BC` | `0x00449E74` call, `0x00449E80 MOV [ESP+0x30],EAX`, `0x0044A16B MOV EAX,[ESP+0x30]` | Yes |
| Unit vtable `+0xD8` | `UnitClass__Unlimbo` placement attempt | `0x00449FF8..0x0044A008` | Yes |

## 3. Core Logic

State 2 begins only after the reverse/opening animation marks `+0x6DD != 0`. It first dirties the owner, clears target state via vtable `+0x3C8`, and skips normal "Structure sold" EVA when the type has `UndeploysInto`.

For a stock GACNST, the branch is active only if:

1. `Type+0x408 != 0` (`UndeploysInto=AMCV`).
2. `Type+0x16B9 == 0` or the ConYard-special chain passes.
3. For GACNST, `Type+0x16B9 != 0`, so the chain must pass: `g_GameMode != 0`, `building+0x218 != 0`, `HouseClass__IsPlayerControl() != 0`, `DAT_00A8B320 != 0`, and `building+0x2C0 == 0`.

The redeploy conversion sequence is:

1. Optional TS artillery/tick-tank rate-timer delay at `0x00449D5E..0x00449E0F`; stock GACNST does not use this because those flags are false.
2. Increment `g_MapEditorMode`, allocate `0x8E8`, call `UnitClass__Constructor(Type+0x408, Owner)`, then decrement `g_MapEditorMode` (`0x00449E10..0x00449E58`).
3. If allocation/constructor returns null, call `vtable+0x2BC` and `HouseClass__Add_Credits` (`0x0044A19E..0x0044A1B0`).
4. If the unit exists, call `ObjectClass__GetHealthRatio @ 0x005F5C60`, store the returned double to `[ESP+0x24]`, then call `vtable+0x2BC` and store `EAX` to `[ESP+0x30]` (`0x00449E64..0x00449E80`).
5. Compute the AMCV spawn coordinate. For foundations larger than 2x2, use `Location + DAT_0089F6F0/DAT_0089F6F4`, cell-align with signed `+0xFF` correction, and add `0x80` center offset (`0x00449EC0..0x00449F0E`). GACNST is 4x4, so this branch is used.
6. Remove/detach the building from the cell via vtable `+0xD4` before attempting unit unlimbo (`0x00449FE2..0x00449FE7`).
7. Get facing from `Deploy_facing_calculator @ 0x00465D70`, which directly returns `TechnoTypeClass+0xEDC`, then call the new unit's vtable `+0xD8` (`0x00449FED..0x0044A008`).
8. If unlimbo fails, load `[ESP+0x30]`, add credits to owner, skip all success-only transfer work, then remove/uninit the building (`0x0044A16B..0x0044A1D2`).
9. If unlimbo succeeds, set new AMCV health to `floor(saved_health_ratio * UnitType.Strength)`, clamp minimum to 1, copy selected/group/audio fields, optionally transfer slave/powerup manager state, then remove/uninit the building.

The critical disassembly sequence:

```text
00449E66 CALL 0x005F5C60                 ; ObjectClass__GetHealthRatio
00449E70 FSTP double ptr [ESP + 0x24]    ; health-ratio double
00449E74 CALL dword ptr [EDX + 0x2BC]    ; refund getter
00449E80 MOV dword ptr [ESP + 0x30],EAX  ; integer refund slot
...
0044A002 CALL dword ptr [EDX + 0xD8]     ; new AMCV Unlimbo
0044A008 TEST AL,AL
0044A00A JZ 0x0044A16B
...
0044A16B MOV EAX,dword ptr [ESP + 0x30]
0044A16F MOV ECX,dword ptr [EBP + 0x21C]
0044A175 PUSH EAX
0044A176 CALL 0x004F9950                 ; Owner->Credits += refund
```

This proves the failed-unlimbo refund is the integer returned by `vtable+0x2BC`, not the double's high 32 bits.

## 4. INI Keys

| Section/key | Stock value | Use in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `[MultiplayerDialogSettings] MCVRedeploys` | `yes` | Sets the session option mirrored by `DAT_00A8B320`; ConYard redeploy gate requires it | `ini/rulesmd.ini:3041`; `0x00449BC0`; `0x00449D43..0x00449D4A` | Yes |
| `[GACNST] ConstructionYard` | `yes` | Makes GACNST use the ConYard-special `MCVRedeploys` chain instead of unconditional non-ConYard branch | `ini/rulesmd.ini:11622`; `0x00449CFE..0x00449D5E` | Yes |
| `[GACNST] UndeploysInto` | `AMCV` | UnitType constructed by reverse conversion | `ini/rulesmd.ini:11631`; `0x00449E34..0x00449E44` | Yes |
| `[GACNST] Cost` | `3000` | Input to refund getter through object/type cost path | `ini/rulesmd.ini:11633`; `0x0070ADA0` | Yes |
| `[GACNST] Strength` | `1000` | Source health ratio denominator via `ObjectClass__GetHealthRatio` | `ini/rulesmd.ini:11624`; `0x005F5C60` | Yes |
| `[AMCV] DeploysInto` | `GACNST` | Confirms bidirectional MCV/ConYard pair, but not read by this reverse branch | `ini/rulesmd.ini:6977` | Yes, outside this failure branch |
| `[AMCV] Strength` | `1000` | Successful path health transfer multiplier; not used for failure refund | `ini/rulesmd.ini:6971`; `0x0044A014..0x0044A039` | Yes only on success |
| `[AMCV] Cost` | `3000` | Not used by failed-unlimbo refund in this branch; refund comes from source building `vtable+0x2BC` | `ini/rulesmd.ini:6979`; `0x00449E74`, `0x0044A16B` | Negative fact |

## 5. Integration Points

`BuildingClass__CanUndeployMCV @ 0x00449BC0` exposes/accepts the redeploy action only when `Type+0x408` is present and, for `ConstructionYard=yes`, the multiplayer/player/session chain passes. This code is active in standard YR because stock GACNST has `ConstructionYard=yes` and `UndeploysInto=AMCV`, and the default `MCVRedeploys` dialog setting is `yes`.

`BuildingClass__Sell @ 0x00449C30` owns the actual state machine. The failed-unlimbo branch happens after the sell animation completes, after the building has detached from cell occupancy, and before the final building cleanup (`vtable+0xDC(1)`, `SoundEvent__Release`, `vtable+0xF8`).

`UnitClass__Constructor @ 0x007353C0` is called before the unlimbo attempt. In this function, the failed path has no visible AMCV because `UnitClass__Unlimbo` returned false. The same-function branch does not call the new unit's delete/destructor/uninit vtable slot before continuing to building cleanup; deeper limbo-object cleanup outside this function was not exhausted.

## 6. Current Rust Implementation Status

Current Rust command plumbing reaches `src/sim/world/world_spawn.rs::undeploy_building`, which records a `BuildingDown` component with target unit type, owner, center cell, height, and selection state. `src/sim/world/mod.rs::tick_building_down` later despawns the building first, calls `spawn_object_at_height`, and if spawn fails, simply leaves the building gone with no refund.

This differs from the verified binary on the failure branch: gamemd pays `vtable+0x2BC` sell-back refund on failed AMCV unlimbo. The Rust code also uses `origin + width/2,height/2` for the stored undeploy center, while the binary's large-foundation branch uses the source building location plus the global centering constants and signed cell-align math; exact coordinate parity belongs to the separate AMCV origin/facing investigations, but the failure refund must be tied to the actual failed spawn/unlimbo result.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__Sell` state 2 entry and `+0x6DD` gate | verified | `0x00449C47..0x00449CA1` | none |
| GACNST ConYard redeploy gate | verified | `0x00449CFE..0x00449D5E`; `0x00449BC0`; INI `rulesmd.ini:3041,11622,11631` | none |
| TS artillery/tick-tank timer delay | verified for non-use by stock GACNST | `0x00449D5E..0x00449E0F`; stock GACNST lacks those flags | exact custom-mod behavior out-of-scope |
| Unit allocation/constructor null branch | verified | `0x00449E10..0x00449E5E`; `0x0044A19E..0x0044A1B0` | none |
| Health-ratio stack storage | verified | `0x00449E66..0x00449E70`; `0x005F5C60` | none |
| Refund stack storage | verified | `0x00449E74..0x00449E80`; `0x0070ADA0` | none |
| Failed-unlimbo refund source | verified | `0x0044A16B..0x0044A176`; `0x004F9950` | none |
| Successful AMCV health transfer | verified only for contrast | `0x0044A014..0x0044A039` | full success lifecycle out-of-scope |
| New unit deletion after failed unlimbo in same function | verified absent in same branch | `0x0044A16B..0x0044A1D2` branch contains refund, dynamic-array cleanup, building cleanup only | deeper global limbo cleanup deferred |
| Current Rust failure behavior | verified from source | `src/sim/world/mod.rs::tick_building_down`; `src/sim/world/world_spawn.rs::undeploy_building` | implementer should test failure path |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is the GACNST reverse path live in stock YR? -> Yes; GACNST has `UndeploysInto=AMCV` and `ConstructionYard=yes`, and MCVRedeploys defaults yes.` (evidence: `ini/rulesmd.ini:3041,11622,11631`; `0x00449BC0`)
- `[RESOLVED] OQ2 - Which state owns the unlimbo-fail branch? -> `BuildingClass__Sell` state 2 after `+0x6DD` is set.` (evidence: `0x00449C47..0x00449CA1`)
- `[RESOLVED] OQ3 - Is `uStack_b4._4_4_` cost, health ratio, computed refund, or something else? -> It is a decompiler stack-overlap artifact for the separate integer refund slot `[ESP+0x30]`, populated by `vtable+0x2BC`; it is not part of the health-ratio double.` (evidence: `0x00449E66..0x00449E80`; `0x0044A16B..0x0044A176`)
- `[RESOLVED] OQ4 - Is the failed-unlimbo refund based on AMCV cost/strength? -> No; failure uses the source building's `vtable+0x2BC` refund value, stored before unlimbo.` (evidence: `0x00449E74`; `0x0044A16B`)
- `[RESOLVED] OQ5 - Does failed unlimbo run the successful AMCV health transfer? -> No; `JZ 0x0044A16B` skips the `floor(HealthRatio * UnitType.Strength)` block.` (evidence: `0x0044A008..0x0044A039`)
- `[RESOLVED] OQ6 - Does the GACNST remain on failed unlimbo? -> No in this function; after refund, the branch reaches `vtable+0xDC(1)`, sound release, and `vtable+0xF8` cleanup for the building.` (evidence: `0x0044A1B5..0x0044A1D2`)
- `[RESOLVED] OQ7 - Does a visible AMCV appear on failed unlimbo? -> No; the visible/success setup is guarded by `TEST AL,AL` after vtable `+0xD8`, and the failure branch skips it.` (evidence: `0x0044A002..0x0044A010`)
- `[RESOLVED] OQ8 - Is the refund added directly to cash credits? -> Yes; `HouseClass__Add_Credits @ 0x004F9950` adds the integer to `House+0x30C`.` (evidence: `0x004F9950`)
- `[RESOLVED] OQ9 - Is there an alternate nearby-cell search before failing? -> No alternate search appears in this branch; it computes one coordinate then calls unit `Unlimbo` once.` (evidence: `0x00449EC0..0x0044A008`)
- `[RESOLVED] OQ10 - What does current Rust do if final unit spawn fails? -> It despawns the building first, calls `spawn_object_at_height`, and pays no refund on `None`.` (evidence: `src/sim/world/mod.rs::tick_building_down`)
- `[DEFERRED] OQ11 - Is the constructed but failed-unlimbo UnitClass later swept from global limbo arrays?` (category: `requires-different-system-context`; reason: same-function branch has no explicit delete/uninit call for the new unit, but global limbo/object-array lifetime is outside this narrow money/object-result slice; next-step-if-pursued: trace UnitClass constructor registration plus object-array cleanup/sweep readers after failed `Unlimbo`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Failed AMCV unlimbo refunds the source building's `vtable+0x2BC` sell-back value from `[ESP+0x30]` | `0x00449E74..0x00449E80`; `0x0044A16B..0x0044A176`; `0x004F9950` | missing | `src/sim/world/mod.rs::tick_building_down`; economy helper equivalent to sell refund | If final AMCV spawn/unlimbo fails after GACNST pack-up completion, remove the building and add the standard sell-back refund to owner credits | Block AMCV spawn cell at pack-up completion; after completion, no AMCV exists, GACNST is gone, owner credits increase by sell-back amount | Do not use the saved health ratio double or AMCV cost as the refund source |
| Successful AMCV placement uses saved health ratio only for new-unit health, not failure refund | `0x00449E66..0x00449E70`; `0x0044A014..0x0044A039` | partially mismatched/unchecked for health parity in this slice | `src/sim/world/world_spawn.rs::undeploy_building`; `tick_building_down` | Keep refund and health transfer as separate values; health ratio should not influence failed-placement credits | Damaged GACNST successful redeploy yields damaged AMCV; failed redeploy still pays sell-back refund value, not a health-ratio fragment | Do not multiply refund by health unless a separate verified refund formula report requires it |
| Failure branch tries exactly one computed AMCV coordinate and does not preserve the GACNST for retry | `0x00449EC0..0x0044A008`; `0x0044A1B5..0x0044A1D2` | Rust already removes building on completion; failure refund missing | `src/sim/world/mod.rs::tick_building_down`; placement/spawn failure handling | Treat final spawn failure as terminal for the building, with refund, not as command rejection/retry | Force spawn failure; player sees no ConYard and no AMCV, but credits increase | Do not keep the packed-up GACNST on the map after final failure |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` lines 208-210 should be replaced with: `If the new AMCV's Unlimbo call fails, the refund is the integer returned by the source building's vtable+0x2BC refund getter, saved at stack slot [ESP+0x30] immediately after ObjectClass::GetHealthRatio saved its double at [ESP+0x24]. It is not the high dword of the HealthRatio double. The branch adds that refund to Owner credits, skips successful AMCV health/field transfer, then removes/uninitializes the source building; no visible AMCV is placed.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` follow-up #4 should be replaced with: `Resolved by GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md: the ambiguous uStack_b4._4_4_ is a decompiler stack-overlap artifact for the separate [ESP+0x30] refund integer, not a HealthRatio fragment.`

## 10. Negative Facts / Do Not Do

- Do not treat `uStack_b4._4_4_` as the high 32 bits of a `double`; the disassembly has separate stack slots for health ratio and refund.
- Do not use `[AMCV] Strength` or `[AMCV] Cost` to compute the failed-placement refund; the failure branch uses the source building's `vtable+0x2BC`.
- Do not preserve the GACNST for retry after final unlimbo failure; the branch removes/uninitializes the building after refund.
- Do not run successful AMCV health transfer, selection transfer, destination mission setup, or contact retargeting when unlimbo returns false.
- Do not add alternate-cell search to this branch without separate evidence; this slice found one computed coordinate and one `Unlimbo` call.

## 11. Remaining Uncertainty

- Whether the constructed but failed-unlimbo UnitClass is later swept from global limbo/object arrays remains outside this slice. Same-function evidence shows no explicit delete/uninit call for the new unit on the failure branch, but the player-visible result remains no placed AMCV.

## Sources

- Ghidra decompiled/read: `0x00449C30`, `0x00449BC0`, `0x005F5C60`, `0x0070ADA0`, `0x004F9950`, `0x00465D70`, `UnitClass__Constructor`, `UnitClass__Unlimbo`, `FootClass__Unlimbo`, `TechnoClass__Unlimbo`.
- Disassembly contexts read: `0x00449C41`, `0x00449C6E`, `0x00449E00`, `0x00449F12`, `0x0044A020`, `0x0044A19E`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/YAREFN_UNDEPLOY_TO_SMIN_SLAVEMANAGER_PATH_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini` `[MultiplayerDialogSettings]`, `[GACNST]`, `[AMCV]`; `ini/rules.ini` fallback sections.
- Rust checked: `src/sim/world/world_spawn.rs::undeploy_building`, `src/sim/world/mod.rs::tick_building_down`, `src/sim/production/production_sell.rs` refund helper.
