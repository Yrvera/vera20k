# BRIDGE_REPAIR_AND_HUT_DEATH — Verify-Doc Amendments

**Date:** 2026-05-18
**Target doc:** `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` (2143 lines, Phase 1 + Phase 2 + Phase 3 ext.)
**Method:** Confirmed the two /re-swarm contradictions, then spot-checked
~10 load-bearing addresses/offsets from §§3, 11, 12, 13, 15, 16 against the
live Ghidra MCP decompilation.

**Tally:** 8 VERIFIED · 4 WRONG (must amend) · 2 STALE (Phase-2 deferral now resolved) · 1 UNVERIFIABLE.

---

## VERIFIED — sampled load-bearing claims that hold up

Spot-checked against `gamemd.exe` this session; doc wording matches binary:

- `RepairBridge_Low @ 0x57F200` body: direction-detect via overlay
  `[0x4A..0x52]` / `[0x53..0x5B]` / `==0x64` / `==0x65`, dispatch to
  `RepairBridgeWalker_NS_Low @ 0x57F6A0` / `RepairBridgeWalker_EW_Low @ 0x57FBC0`. (§§3, 11–12)
- `RepairBridgeSegment @ 0x575EE0` body: fires
  `TechnoClass::ProcessCellAction(0x1F, 0, DAT_00abd480, 0, 0)` per cell
  with `cell+0x3C != 0`; EW axis uses `DAT_0089f698`, NS axis uses
  `DAT_0089f690` / `DAT_0089f6a0`; no object clearing. (§1 Conflict C / §11)
- `ApplyDamageToCell @ 0x587180`: reads `cell+0x44`; `[0x4A..0x63]` →
  `DestroyBridge_Low`; `[0xCD..0xE6]` → `DestroyBridge_High`; ramp →
  `ProcessBridgeDamageStateMachine_*`. (§11 / §13.3)
- `UpdateAdjacentBridges_High @ 0x576770`: walks 8 neighbors checking
  `cell+0x140 & 0x500`, calls `UpdateBridgeEdgeTiles_High`; no `_Low`
  variant exists in the binary (`get_function_by_address` confirms
  only `_High`). (§11 / §13.4 "copy-paste bug")
- `vtable[0x160]` at `BuildingClass vtable+0x160` (`0x7E401C`) =
  `0x0041BF40`; decompile of `0x41BF40` is
  `TechnoClass::IsIronCurtainActive` reading instance fields `+0x18C`
  and `+0x194`; no read of `Type[+0xC4D]` (Immune). (§15)
- `FUN_00598030 @ 0x598030`: `Random__Next + Math__ftol` rejection-loop;
  no LAT table. (§12.5)
- `BombClass::Detonate @ 0x438720`, `BuildingClass::Update @ 0x43FB20`,
  `InfantryClass::PerCellProcess @ 0x519630` — addresses resolve to the
  named functions. (§§3.1, 3.2, 3.7)
- `RepairBridgeWalker_NS_Low @ 0x57F6A0` resolves; body matches doc §12.

(Full enumeration of 2143-line doc not performed — sampled the
addresses/offsets that the doc's conclusions hang on.)

---

## WRONG — must amend

### 1. §4 INI Keys table, row `DestroyableBridges` (~line 844) — section wrong

**Claim (doc):**
> `DestroyableBridges=yes` · `[CombatDamage]` · "Suspected master gate;
> Phase 1 did NOT find an explicit check in #1–#6 — needs Phase 2 to find
> the gate" · Read at `(deferred)`.

**Binary evidence (this session, confirming `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`):**
- String `"DestroyableBridges"` @ `0x00840248` is read only by
  `FUN_006B8CA0` (the `[SpecialFlags]` reader), called from
  `ScenarioClass::Read_INI_Basic @ 0x00689E90`.
- Stored as **bit 0xF (0x8000) of the `uint32` at `ScenarioClass + 0x000`**
  (SpecialFlags bitfield).
- Source INI: **map/scenario INI under `[SpecialFlags]`**, NOT
  `rulesmd.ini` `[CombatDamage]`. The `[CombatDamage] DestroyableBridges=`
  line in stock `rulesmd.ini` is decorative / no-op (no xref).
- Sole AoE gate: `Apply_area_damage @ 0x00489280`. Live this session
  at `LAB_00489f11`-area: `if (((*g_ScenarioClass_Instance & 0x8000) == 0)
  || (*(char *)(param_4 + 0x144) == '\0')) goto LAB_0048a2c4;` — skips
  all bridge-destruction sub-blocks (high/low ramp dispatch, overlay-range
  `DestroyBridge_Low/High`, `TacticalClass::DirtyScreenRect` for bridge
  tiles).
- **Asymmetry verified this session:** `DestroyBridge_High_OnHutDeath
  @ 0x574000` and `DestroyBridge_Low_OnHutDeath @ 0x574C20` decompiled —
  neither function reads `g_ScenarioClass_Instance` or `0x8000`. C4 /
  demo-truck on CABHUT collapses the bridge regardless of
  `DestroyableBridges=no`.

**Corrected wording:**

| Key | Section | Default | Effect (verified) | Read at |
|---|---|---|---|---|
| `DestroyableBridges=yes` | **`[SpecialFlags]` (map/scenario INI, NOT `rulesmd.ini`)** | yes (constructor default, bit 0xF of `ScenarioClass+0x000`) | Stored at `ScenarioClass+0x000` bit 0xF (0x8000). Gates AoE bridge destruction at `Apply_area_damage @ 0x489280`: when off, skips `DestroyBridge_Low/High`, ramp state-machine dispatch, and bridge-tile dirty-rect. **Does NOT gate the hut-death path** (`DestroyBridge_*_OnHutDeath` have no SpecialFlags read). | `FUN_006B8CA0 @ 0x6B8E1F` (reader); gate at `0x489280`. Asymmetry: hut-death paths `0x574000` / `0x574C20` are ungated. |

The doc's separate statement "Phase 1 did NOT find an explicit check"
should be marked **RESOLVED** with cite to `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`.

---

### 2. §6 row "Hut registry at `MapClass+0x1160`" (~line 920) — registry identity is wrong

**Claim (doc):**
> "Hut registry at `MapClass+0x1160` (`DAT_008B41A8`) — Not started.
> Already documented in `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`.
> Phase-1 did NOT touch this; Phase 2 needs to decompile
> `UnregisterBridgeRepairHut` (#15)."

**Binary evidence (this session):**
- `UnregisterBridgeRepairHut @ 0x577920` decompiled — function-entry is
  gated by `param_2->vtable[0x2C]() == 0x2C` (WhatAmI()==TagClass).
  BuildingClass (WhatAmI=6) never enters phase A's body.
- Phase A (per-cell loop): `*(int *)(param_1 + 0x116c)` is the **count**,
  `*(int *)(param_1 + 0x1160)` is the **data pointer** — i.e., the
  DynVec's vtable lives at `+0x115C`, not `+0x1160`. Each entry is a
  packed `(short x, short y)` cell coord; the loop scans cells whose
  `+0x3C` (attached object) matches the TagClass being unregistered.
- Phase B (global list): `DAT_008B41A8` is the **vtable** of a
  `DynamicVectorClass<TagClass*>`; data at `DAT_008B41AC`, count at
  `DAT_008B41B8`. This is `g_DestroyedEventTagList` (tags with
  Destroyed-type event bit set), **not** a hut/building registry.
- Single caller is `Detach_From_All_Lists @ 0x7258D0` (case `0x2C`
  dispatch). The case-`0x0C` (FactoryClass) call site fails the inner
  RTTI gate and is a no-op.

**Corrected wording:**

| Subsystem | Status | File(s) / notes |
|---|---|---|
| TagClass per-cell registry at `MapClass+0x115C` (DynVec; `+0x1160`=data, `+0x116C`=count) and global `g_DestroyedEventTagList @ DAT_008B41A8` | Not applicable to hut destruction | `MapClass::UnregisterBridgeRepairHut @ 0x577920` is misnamed — it is a TagClass-detach helper gated on `WhatAmI()==0x2C`, called only from `Detach_From_All_Lists @ 0x7258D0`. Buildings (WhatAmI=6) never enter the body. Not invoked from any hut-destruction path. See `UNREGISTERBRIDGEREPAIRHUT_AND_HUT_REGISTRY_GHIDRA_REPORT.md`. |

Doc §16 already discloses most of this correctly. The remaining wording
to fix is in §6 row "Hut registry at `MapClass+0x1160` (DAT_008B41A8)" —
strip "Phase 2 needs to decompile" (it has been), and replace
"hut registry" with "TagClass per-cell registry + g_DestroyedEventTagList".

---

### 3. §16 line ~1713 — "`MapClass+0x1160` is NOT a list of bridge-repair-hut buildings" is right but address is loose

**Claim (doc §16):**
> "It removes the tag from both the per-cell registry at `MapClass+0x1160`
> and the global tag list at `DAT_008B41A8` / `DAT_008B41AC`."

**Binary evidence:** The DynVec's vtable pointer lives at `MapClass+0x115C`
(`*(int *)(param_1 + 0x115c) + 0x10` is the vtable[4] = Find call); the
**data pointer** is at `+0x1160`; **count** at `+0x116C`; **capacity** at
`+0x1164`. Saying "per-cell registry at `MapClass+0x1160`" is
half-right — `+0x1160` is the data pointer field within the DynVec, not
the DynVec base.

**Corrected wording:** "per-cell registry whose DynVec lives at
`MapClass+0x115C` (vtable), with data pointer at `+0x1160` and count at
`+0x116C`." (Same correction applies to §6 row.)

---

### 4. §11 inventory table — `_MapInit` suffix is now corrected in Ghidra; doc still uses it throughout

**Claim (doc):** §0/§11 routinely write
`MapClass::DestroyBridge_High_MapInit (0x574000)` /
`MapClass::DestroyBridge_Low_MapInit (0x574C20)`, with a parenthetical
"misleading Ghidra label."

**Binary evidence (this session):**
- `get_function_by_address(0x574000)` returns
  `MapClass__DestroyBridge_High_OnHutDeath`.
- `get_function_by_address(0x574C20)` returns
  `MapClass__DestroyBridge_Low_OnHutDeath`.

The "`_MapInit` is a misleading Ghidra label" note in the doc is now
stale — Ghidra has been corrected to `_OnHutDeath`. All `_MapInit`
references in §§0, 11, 13.2, 18A.4, etc. should be globally replaced
with `_OnHutDeath` (and the "misleading label" footnote dropped or
turned into "previously labelled `_MapInit`").

---

## STALE — Phase-2 deferral resolved

These were flagged as open in the doc; the recent slot-1 / slot-5 swarm
reports resolved them. Mark resolved on next sweep:

- §4 (~line 844) `DestroyableBridges=` "needs Phase 2 to find the gate" →
  resolved (`Apply_area_damage @ 0x489280`, SpecialFlags bit 0xF). See
  WRONG #1 above for the correction; the open-question line itself should
  be marked **RESOLVED 2026-05-18** with cite.
- §6 (~line 920) "Hut registry at `MapClass+0x1160` — Phase 2 needs to
  decompile `UnregisterBridgeRepairHut`" → resolved by §16 itself (and
  fully decompiled by slot-5). See WRONG #2 above.

---

## UNVERIFIABLE — flag for user

- §13.4 "vanilla copy-paste bug — `UpdateAdjacentBridges_High` called from
  both Low and High `OnHutDeath`." The doc requests an in-game test to
  confirm whether this manifests as a visible glitch on low-bridge CABHUT
  destruction. The binary fact (both call `_High`, no `_Low` variant
  exists) is confirmed this session. Whether the visible output diverges
  cannot be answered from the binary alone — it requires running gamemd
  with a low-bridge map. **Flag for user.**

---

## Notes

- Did not re-verify every walker overlay-transition entry in §12.4
  (28 + 28 rows). Sampled the band boundaries and the dispatcher
  identity (`RepairBridge_Low → RepairBridgeWalker_NS_Low/EW_Low`); all
  sampled rows match. Full row-by-row audit is out of scope per the
  contract.
- Did not re-verify the §3.6 InfantryClass::PerCellProcess line-by-line
  decompilation (entry-point and identity confirmed; body shape
  consistent with described 5×5 scan + mission-0x11 branch, but not
  re-walked end-to-end).
- §15.1 vtable-slot computation: doc cites `vtable_BuildingClass @
  0x7E3EBC`, slot `+0x160` at `0x7E401C` (= base + 0x160). Read of
  `0x7E401C` returns `40 bf 41 00` → `0x0041BF40` = `IsIronCurtainActive`.
  Confirmed.
