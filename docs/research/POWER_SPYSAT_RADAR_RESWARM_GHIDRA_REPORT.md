# Power / SpySat / Radar Availability — /re-swarm Reconciliation Ghidra Report

**Slot:** /re-swarm batch, slot 2 (session 20260712-reswarm-phase2)
**Target:** Settle RED items flagged 2026-07-10 in `POWER_SYSTEM_GHIDRA_REPORT.md`: true roles of
`0x00508F60` and `0x00508DF0`, and the meaning of `BuildingTypeClass+0x16A5`.
**Investigation Mode:** targeted re-verification — decompile + raw disassembly of both functions,
independent re-derivation of the `+0x16A5` INI reader, and a fresh trace of the `+0x2B0/+0x2B8`
timer and `ScenarioClass+0x34A4` flag that `0x00508DF0` reads.
**Scope:** `0x00508F60`, `0x00508DF0`, `BuildingTypeClass+0x16A5`, and the `HouseClass` fields
they read/write (`+0x5779` RecheckRadar, `+0x2B0/+0x2B8` timer, `ScenarioClass+0x34A4`). Does
**not** re-audit the power-total/formula system (`AI_AssessPower`, `PowerRatio`, production speed).
**Confidence:** High for all six load-bearing findings below — each is backed by raw disassembly
I read myself this session (not inherited from either prior doc).
**Active in YR:** See per-finding table.

## Summary

Both RED items are confirmed as the 2026-07-10 audit claimed, plus one finding neither prior doc
had right: `ScenarioClass+0x34A4` — the flag `0x00508DF0` checks before its power/building scan —
is `FreeRadar=` (a map `[Basic]` key), not "SpySat active" (`POWER_SYSTEM_GHIDRA_REPORT.md`'s
guess) and not "FogOfWar disabled" (`HOUSECLASS_GHIDRA_REPORT.md`'s guess). Its effect is also
inverted from what `POWER_SYSTEM_GHIDRA_REPORT.md` states: `FreeRadar=yes` **enables** the
tactical map unconditionally, it does not disable it, and it has nothing to do with SpySat.

`0x00508F60` (Ghidra label `HouseClass__CheckLowPower`) is a SpySat-family reveal/restore scan:
its entire building filter is `TypeClass+0x16A5 != 0` (SpySat), nothing else. It has no relation to
the `PoweredSpecial` flag (`+0x1574`) used by `IsOperational`, and is not a generic "any building
loses power → shroud" handler.

`0x00508DF0` (Ghidra label `HouseClass__CheckSuperweaponReady`) is a confirmed **mislabel**. The
function never reads a superweapon-related field (no `SuperWeapon` type index, no charge/ready
timer). It scans for `TypeClass+0x16A4` (`Radar=yes`) buildings, and its two callees write
`RadarClass+0x14D8` and log the literal debug string `"Radar/TacticalMap availability is %s"`. It
is the tactical-map/radar-availability gate, exactly as `POWER_SYSTEM_GHIDRA_REPORT.md`'s own prose
already treated it — the doc just never flagged that the Ghidra symbol name itself is wrong, and
got several details of the function body wrong (see below).

## Verified Binary Findings

### 1. `BuildingTypeClass+0x16A5` is `SpySat=`, not `PoweredSpecialShroud`

Active in YR: Yes.

Re-derived independently from raw disassembly (not copied from the sibling doc). `search_strings`
for `SpySat` returns exactly one hit, `0x0081AE58`. `get_xrefs_to 0x0081ae58` resolves to
`BuildingTypeClass__ReadINI` at instruction `0x0045ff72`, inside the large INI-reader function
whose entry is `0x0045FE50`. `get_assembly_context` on that xref shows:

```
0045ff65: MOV byte ptr [EBP + 0x16a4],AL     ; store previous ReadBool result -> +0x16A4 (Radar)
0045ff6b: MOV CL,byte ptr [EBP + 0x16a5]     ; load current +0x16A5 value as ReadBool's default arg
0045ff72: PUSH 0x81ae58                       ; push string "SpySat"
0045ff7a: CALL 0x005295f0                     ; CCINIClass::ReadBool(section, "SpySat", default)
0045ff7f: MOV byte ptr [EBP + 0x16a5],AL     ; store result -> +0x16A5
```

`grep ini/rulesmd.ini` confirms `[GASPYSAT]` (line 12187) has `Radar=yes` (12194), `SpySat=yes`
(12195), `Power=-100` (12204), `Powered=true` (12205) — the standard buildable Allied Spy
Satellite Uplink. A second `SpySat=yes` exists at line 14568 (civilian/campaign structure).
This matches `SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`'s independent finding
at the same address.

### 2. `0x00508F60` (`HouseClass__CheckLowPower`) is a SpySat-only reveal/restore scan

Active in YR: Yes, whenever the house owns an eligible `SpySat=yes` building.

Full disassembly read (not just decompile). At entry: `MOV byte ptr [ECX+0x5779],0x0` — clears
`RecheckRadar`. The building filter, read directly from machine code, is:

```
piVar1 = building              ; from house->field_0x6c (building list) [+0x78 = count]
TypeClass = piVar1[0x148]      ; = *(building + 0x520)
CMP byte ptr [TypeClass+0x16a5],0   ; SpySat flag — the ONLY type-level gate
building+0x81 == 0             ; not limbo/being-sold
building+0x74 != 0             ; alive/placed
(campaign single-player gate on +0x41b, then) building+0xAC/+0xB4 != 0x13 (not Selling)
vtable+0x1D4() == 0            ; not cloaked/warped
```

On the first building that passes all gates: if `SpySatActive` (`+0x577A`, confirmed by raw
`MOV byte ptr [ESI+0x577a],0x1`/`,0x0`) is false, calls `MapClass__BlackoutShroud (0x00577D90)`,
sets `SpySatActive=1`, plays the SpySat-activation sound (`CALL 0x00750920` with sound index from
`[…+0x220]`) only if `this == g_PlayerPtr`. If the building scan finds nothing eligible and
`SpySatActive` was true, calls `MapClass__RestoreShroud (0x00577AB0)`, clears `SpySatActive`, plays
the deactivation sound (index from `[…+0x224]`). These addresses, offsets, and behavior are
independently identical to `SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`'s
`HouseClass__CheckLowPower` findings — re-verified here directly from assembly, not copied.

**Negative finding:** the function never reads `PoweredSpecial` (`+0x1574`) or any other
`Powered=`-family flag. It is not a generic "building lost power → apply shroud" handler; it is
scoped exclusively to the `SpySat` flag family.

### 3. `0x00508DF0` (Ghidra-labeled `HouseClass__CheckSuperweaponReady`) is the radar/tactical-map availability check — the label is wrong

Active in YR: Yes — this is the mechanism behind "radar goes offline during low power."

At entry: `MOV byte ptr [ECX+0x5779],0x0` (clears RecheckRadar), then `if (this != g_PlayerPtr) return` —
local player only. The building filter (raw disasm) is `TypeClass+0x16A4 != 0` (`Radar=yes`, same
struct field the earlier `0x0045ff65` write targets), `building+0x660 != 0` (online), `+0x81==0`,
`+0x74!=0`, campaign gate, mission `+0xAC/+0xB4 != 0x13`, then `building+0x504 == 0`
(`EMPLockRemaining`, confirmed via `MOV ECX,[EAX+0x504]; TEST ECX,ECX`) AND `vtable+0x1D4()==0`
(not cloaked/warped). No `SuperWeapon` type field, no charge-ready timer, and no
`PsychicDetectionRadius` reference exists anywhere in the function body.

The result byte is compared against `RadarClass__IsTacticalMapAvailable (0x00656DE0)` and, if
different, passed to `FUN_00656DF0`, which I decompiled directly: it writes `RadarClass+0x14D8`,
logs the string `Radar/TacticalMap availability is %s` (`s_Radar__TacticalMap_availability_i_008394bc`),
and calls `RadarClass__ActivateDeactivate` or `RadarClass__SetRadarMode`. This is unambiguously a
radar/minimap-availability toggle, not a superweapon-readiness toggle. `FUN_00656DF0`'s only other
caller is `HouseClass__MPlayer_Defeated (0x004FC0B0)` — forcing radar off on defeat, consistent
with radar semantics.

**Behavior detail also missed by both prior docs:** like `CheckLowPower`, the building scan stops
at the *first* building passing the coarse filters (`Radar=yes`, online, alive, not selling); if
that one building fails the final EMP/cloak gate, the loop `break`s instead of continuing to check
other radar buildings. A second working `Radar=yes` building later in the list does not rescue
radar availability if the first one in iteration order is EMPed/cloaked/warped.

### 4. `ScenarioClass+0x34A4` is `FreeRadar=` (map `[Basic]` key) — not SpySat, not FogOfWar

Active in YR: Conditional — only when a map's `[Basic]` section sets `FreeRadar=yes`; this is a
per-scenario/campaign key, not a `rulesmd.ini` rule.

`0x00508DF0` reads this flag (`MOV EAX,[0x00a8b230]; MOV DL,[EAX+0x34a4]; TEST DL,DL; JNZ 0x00508f2a`)
**after** the timer check and **before** the power/building scan: if set, the function jumps
straight to "tactical map available = TRUE" (`cVar2=1`), skipping the power ratio and building
checks entirely. I independently decompiled `ScenarioClass::Read_INI_Basic (0x00689E90)` and found:

```c
uVar2 = CCINIClass__ReadBool(/* section */ "Basic", /* key */ "FreeRadar", /* default */ ...);
*(undefined1 *)(param_1 + 0x34a4) = uVar2;
```

with the literal key string `s_FreeRadar_0083dff8`. This is the same function
`OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md` already used to identify the adjacent `+0x34A6`
(`TiberiumGrowthEnabled`) — cross-confirmed independently in this session.

**Both prior docs are wrong on this field**, in different ways:
- `POWER_SYSTEM_GHIDRA_REPORT.md` calls it "SpySat active" and claims the effect is "radar
  DISABLED" — wrong identity AND wrong polarity (`FreeRadar=yes` enables the tactical map, and has
  no relationship to SpySat at all).
- `HOUSECLASS_GHIDRA_REPORT.md` calls it "Fog-of-War disabled" and inverts the same branch
  (`if (!fog_of_war_disabled)`) — wrong identity, and the described control flow is backwards from
  what the raw JNZ target does (JNZ on nonzero jumps to the "tactical map ready" path, it does not
  gate away from a check).

### 5. `+0x2B0`(start, `-1`=inactive)`/+0x2B8`(duration) is a distinct timer from the `+0x2A4/+0x2AC` SpyPowerBlackout pair

Active in YR: Timer-read mechanism confirmed active every tick `0x00508DF0` runs; genuine
multi-frame arming by anything other than `HouseClass::Update`'s own bootstrap was **not found**
in this pass (see Remaining Uncertainty).

Raw disassembly of `0x00508DF0` (`MOV EAX,[ECX+0x2b8]; MOV ESI,[ECX+0x2b0]; CMP ESI,-1; …`) shows a
`start==-1 / elapsed<duration` countdown identical in shape to the SpyPowerBlackout timer, but at
different literal byte offsets. Confirmed disjoint: `HouseClass::SpyPowerSabotage (0x0050BC90)` —
the function `BuildingClass::OnSpyInfiltrate` and `SuperClass::Launch` call to arm the real spy
power blackout — writes only `+0x5778`, `+0x2A4`, `+0x2A8`, `+0x2AC`; it never touches
`+0x2B0/+0x2B8`. While the `+0x2B0/+0x2B8` timer has not fully counted down, `0x00508DF0` skips
straight to the tactical-map-availability apply step with the result left at its default `0`
(not-ready), i.e. it forces radar unavailable for the remaining duration without evaluating power
or buildings that tick.

`HouseClass::Update (0x004F8440)` decrements both `+0x2A4/+0x2AC` and `+0x2B0/+0x2B8` every frame
using the same idiom: when a timer's start is `-1` and its stored "duration" equals the sentinel
value `1`, it arms the timer (`start=now`, `duration=0`) and sets the corresponding flag
(`RecheckPower`/`+0x5778` for the first pair, `RecheckRadar`/`+0x5779` for the second) true. For the
`+0x2A4` pair this matches `POWER_SYSTEM_GHIDRA_REPORT.md`'s existing "0x4F846C — spy blackout
timer expires — sets NeedsPowerRecalc" entry (re-derived independently here, same conclusion). The
`+0x2B0` pair's only observed writer in this pass is this same `Update()` bootstrap, and it only
ever arms a zero-length duration — meaning, absent an as-yet-unidentified other setter, the block
window inside `0x00508DF0` is a same-tick no-op in ordinary play (elapsed 0 < duration 0 is false,
so it falls through immediately to the real check). No other function was found writing a
non-sentinel duration to `+0x2B8`.

### 6. `+0x53A4`/`+0x53A8` (PowerOutput/PowerDrain) are correct in `POWER_SYSTEM_GHIDRA_REPORT.md`; a later "correction" in `HOUSECLASS_GHIDRA_REPORT.md` is wrong

Active in YR: Yes (core power totals, read every tick).

`HOUSECLASS_GHIDRA_REPORT.md`'s `CheckPoweredRadar` section carries a "corrected 2026-05-28" note
claiming the power output/drain reads inside `0x00508DF0` are at `+0x5384`/`+0x5388`, not
`+0x53A4`/`+0x53A8`. Raw disassembly read this session directly contradicts that:

```
00508e4a: MOV EDX,dword ptr [ECX + 0x53a4]
00508e50: MOV EAX,dword ptr [ECX + 0x53a8]
```

`+0x53A4`/`+0x53A8` is correct (matches `POWER_SYSTEM_GHIDRA_REPORT.md`'s existing field table and
`AI_AssessPower`'s own writes to the same offsets). The 2026-05-28 "correction" in
`HOUSECLASS_GHIDRA_REPORT.md` is itself the error — `+0x5384`/`+0x5388` are unrelated factory-count
fields (Vehicle/Infantry(alt) factory counts per that same doc's own factory-count table), not
power totals. This doc is out of this slot's write scope; flagged here for the parent to route a
correction.

## Active in Standard YR?

| Finding | Active in standard YR? | Condition / default |
|---|---|---|
| `BuildingTypeClass+0x16A5` = `SpySat` | Yes | `[GASPYSAT] SpySat=yes`, stock buildable Allied structure |
| `0x00508F60` SpySat reveal/restore scan | Yes | Runs whenever `RecheckRadar` fires; filters strictly on `+0x16A5` |
| `0x00508DF0` radar/tactical-map availability | Yes | Local player only, gated by `RecheckRadar` and (separately) global `DAT_00a8b538==0` |
| `ScenarioClass+0x34A4` = `FreeRadar` short-circuit | Conditional | Only when map `[Basic] FreeRadar=yes`; not a `rulesmd.ini` key, stock-map coverage not exhaustively checked in this pass |
| `+0x2B0/+0x2B8` timer block inside `0x00508DF0` | Conditional/Unverified | Mechanism reads every tick; no genuine multi-frame arm source found this pass — see Remaining Uncertainty |

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance | Test name | Risk / do-not-do |
|---|---|---|---|---|---|
| `BuildingTypeClass+0x16A5` is `SpySat`, and the reveal/restore scan (`0x00508F60`) filters exclusively on it, independent of `PoweredSpecial`/`Powered`. | Confirm Rust's `spy_sat` parsing (`src/rules/object_type.rs`) is not conflated with any generic `PoweredSpecial`/low-power-shroud concept; keep it a standalone gate. | `src/rules/object_type.rs`, `src/sim/vision/mod.rs`, `src/sim/power_system.rs` | Building only a `Radar=yes`+`Powered=yes` structure with no `SpySat=yes` never triggers full-map reveal. | `test_radar_only_building_never_triggers_spysat_reveal` | Do not implement SpySat reveal as a side effect of the generic `Powered=yes`/low-power disable path. |
| `0x00508DF0` is radar/tactical-map availability, not superweapon readiness; the scan stops at the first `Radar=yes` building passing coarse filters, and that one building's EMP/cloak state alone decides the result. | If Rust's radar-availability logic scans all radar buildings and takes an "any works" OR, that is a parity gap — native takes only the first list-order match. | `src/sim/power_system.rs` (or wherever radar-on/off is computed), radar/minimap gating in `src/render/minimap.rs` | With two `Radar=yes` buildings where the first (in house building-list order) is EMPed and the second is healthy, radar still goes offline. | `test_radar_availability_uses_first_matching_building_only` | Do not implement "radar is on if ANY Radar=yes building is usable" — native is first-match, not any-match. |
| `ScenarioClass+0x34A4` (`FreeRadar=` map `[Basic]` key) forces tactical-map availability true, bypassing power/building checks entirely; it has no relationship to SpySat. | If Rust has any "SpySat forces radar" or "FogOfWar disables radar" logic modeled on the stale doc text, remove/re-derive it — the real gate is a distinct, rarely-set map key. | `src/map/scenario.rs` (or wherever map `[Basic]` keys are parsed), radar availability logic | A map with `[Basic] FreeRadar=yes` keeps the local player's radar online even with zero power and zero radar buildings. | `test_free_radar_ini_key_forces_tactical_map_available` | Do not gate this on `SpySatActive` or on any fog-of-war setting — both are wrong per verified disassembly. |

## Negative Facts / Do Not Do

- Do not label `BuildingTypeClass+0x16A5` as `PoweredSpecialShroud`. It is `SpySat=`, verified via the INI reader at `0x0045ff72..0x0045ff7f` this session.
- Do not treat `0x00508F60` as a generic "any `Powered=`/`PoweredSpecial` building loses power → apply shroud" handler. Its only type-level filter is `+0x16A5` (SpySat).
- Do not call `0x00508DF0` "superweapon readiness" in any doc or code comment. It never reads a superweapon type field; it writes `RadarClass+0x14D8` through callees whose own debug string says "Radar/TacticalMap availability."
- Do not describe `ScenarioClass+0x34A4` as SpySat-related or as disabling radar. It is `FreeRadar=` (map `[Basic]` key) and it **enables** the tactical map when set.
- Do not conflate `+0x2A4/+0x2AC` (SpyPowerBlackout, armed only by `HouseClass::SpyPowerSabotage`) with `+0x2B0/+0x2B8` (a separate timer read only inside `0x00508DF0`/`HouseClass::Update`). They are different storage, different owners, different purposes.
- Do not carry forward `HOUSECLASS_GHIDRA_REPORT.md`'s 2026-05-28 "correction" that PowerOutput/PowerDrain are at `+0x5384`/`+0x5388`. Raw disassembly of `0x00508DF0` this session shows `+0x53A4`/`+0x53A8`, matching `POWER_SYSTEM_GHIDRA_REPORT.md`'s original (and `AI_AssessPower`'s own) offsets.

## Remaining Uncertainty

- No function other than `HouseClass::Update`'s own bootstrap (`duration` sentinel `1` → arm with `duration=0`) was found writing to `+0x2B0/+0x2B8` in this pass. Whether some other system (a jamming superweapon, a scripted trigger action) ever arms this timer with a genuine multi-frame duration was not exhaustively searched — Ghidra MCP has no generic "grep raw struct-offset write across all functions" tool, and an exhaustive function-by-function sweep was out of this slot's time budget. If no such setter exists, the block branch inside `0x00508DF0` is a same-tick no-op in ordinary play.
- `DAT_00a8b538` (the global gate that skips calling `0x00508DF0` entirely from `HouseClass::Update`, while `CheckLowPower` still always runs) was read via xrefs only — it is written once from `Main_Game` and read broadly (SuperClass::Launch, LightningStorm::Start, RadarClass::SetRadarMode, CommandBar_Dispatch, HouseClass::MakeAlly). Its exact semantic identity (likely a "game session active"/"not in a menu" flag) was not resolved in this pass; out of this slot's narrow scope.
- Whether any stock skirmish (non-campcampaign) map ships `[Basic] FreeRadar=yes` was not checked — only `ini/rulesmd.ini` (a rules file, not a map file) was searched. Map `.map`/`.mmx` files are outside this pass's file access.

## Stale-Doc Replacement Wording

**File:** `docs/research/POWER_SYSTEM_GHIDRA_REPORT.md`

**1. BuildingTypeClass Power Fields table** (current line ~68):

- OLD: `| +0x16A5 | 1 | PoweredSpecialShroud | Used in low-power shroud check |`
- NEW: `| +0x16A5 | 1 | SpySat | `SpySat=yes` — scanned by `HouseClass::CheckLowPower` (0x508F60) to drive the passive full-map-reveal shroud toggle; unrelated to `PoweredSpecial` (+0x1574) (verified via `get_assembly_context 0x0045ff72`/`0x0045ff7f` and `disassemble_function 0x00508F60`, 2026-07-12) |`

**2. "Low Power Effects Summary" → "3. Radar / Shroud" section** (current lines ~479-490), the
`HouseClass::CheckLowPower (0x508F60)` bullet listing "PoweredSpecial flag (+0x16A5)":

- OLD: `Iterates buildings with PoweredSpecial flag (+0x16A5)`
- NEW: `Iterates buildings with the SpySat flag (BuildingTypeClass+0x16A5, INI key SpySat=); PoweredSpecial (+0x1574) is a separate flag consumed only by IsOperational, not by this function (verified via disassemble_function 0x00508F60, 2026-07-12)`

**3. "CheckSuperweaponReady (0x508DF0) — Radar Enable/Disable" section** (current lines ~419-459
and ~1251-1266), replace the whole numbered list and pseudocode with:

- OLD (pseudocode + numbered steps claiming "only when SpySat is NOT active," "If SpySat active
  (`ScenarioClass+0x34A4`) → radar DISABLED," "Buildings with `PsychicDetectionRadius > 0` provide
  radar regardless of power," "Normal radar buildings must pass `HasPower` check (+0x270)," and
  "`RadarClass::SetRadarDisabled` toggles minimap state")
- NEW: `The Ghidra symbol name "HouseClass__CheckSuperweaponReady" is a mislabel; the function has no superweapon-related logic. It is the local-player tactical-map/radar-availability gate. Order: (1) clear RecheckRadar (+0x5779); (2) if not local player, return; (3) check the +0x2B0/+0x2B8 timer (distinct storage from the +0x2A4/+0x2AC SpyPowerBlackout pair) — while active, skip straight to applying "not ready"; (4) if ScenarioClass+0x34A4 (FreeRadar=, a map [Basic] key, NOT SpySat-related) is set, short-circuit to "tactical map ready = true"; (5) otherwise require PowerOutput(+0x53A4)/PowerDrain(+0x53A8) ratio >= 1.0; (6) scan owned buildings for the FIRST one (in list order) with Radar=yes (+0x16A4), online, alive, not selling, EMPLockRemaining(+0x504)==0, and vtable+0x1D4()==0 (not cloaked/warped) — that one building's state alone decides the result, the scan does not continue past it; (7) compare against RadarClass::IsTacticalMapAvailable (0x00656DE0) and, if changed, call FUN_00656DF0, which writes RadarClass+0x14D8 and logs "Radar/TacticalMap availability is %s" (verified via decompile_function 0x00508DF0, disassemble_function 0x00508DF0, decompile_function 0x00656DF0, decompile_function 0x00689E90, 2026-07-12).`

**Corroborates or contradicts the sibling doc?** Corroborates
`SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md` on every claim it makes about
`0x00508F60` and `BuildingTypeClass+0x16A5` — independently re-verified from raw disassembly this
session, not just copied. That sibling doc does not cover `0x00508DF0`, `ScenarioClass+0x34A4`, or
`+0x2B0/+0x2B8` at all, so there is no overlap to contradict there; those three findings are new to
this report.

## Sources

- Ghidra decompile: `HouseClass__CheckLowPower @ 0x00508F60`, `HouseClass__CheckSuperweaponReady @ 0x00508DF0` (Ghidra-current symbol name; role is radar/tactical-map, see Finding 3), `HouseClass__Update @ 0x004F8440`, `FUN_00656df0 @ 0x00656DF0`, `HouseClass__SpyPowerSabotage @ 0x0050BC90`, `ScenarioClass__Read_INI_Basic @ 0x00689E90`.
- Ghidra disassembly: `disassemble_function 0x00508DF0`, `disassemble_function 0x00508F60`.
- Ghidra assembly context: `get_assembly_context` on xref `0x0045ff72` (SpySat string reference).
- Ghidra xrefs: `get_xrefs_to 0x0081ae58` (SpySat string), `get_xrefs_to 0x00a8b538`, `get_function_callers 0x00508DF0`, `get_function_callers 0x00508F60`, `get_function_callers 0x00656df0`, `get_function_callers 0x0050BC90`.
- Ghidra string search: `search_strings "^SpySat$"`.
- INI checked: `ini/rulesmd.ini` `[GASPYSAT]` (lines 12187-12205), grep for `SpySat=`/`Radar=yes`.
- Prior docs reconciled: `docs/research/POWER_SYSTEM_GHIDRA_REPORT.md`, `docs/research/SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`, `docs/research/HOUSECLASS_GHIDRA_REPORT.md`, `docs/research/OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`, `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`.

## Status

COMPLETE for the scoped RED items (`0x00508F60`, `0x00508DF0`, `BuildingTypeClass+0x16A5`) and the
`HouseClass`/`ScenarioClass` fields they read/write. Two additional cross-doc corrections
identified as a byproduct (`ScenarioClass+0x34A4` identity/polarity, `+0x53A4/+0x53A8` vs the stale
`HOUSECLASS_GHIDRA_REPORT.md` "correction") are documented above for the parent to route.
